#!/usr/bin/env python3
import argparse
import csv
import importlib.util
import json
import os
import re
import shlex
import shutil
import subprocess
import sys
import threading
import time
from datetime import datetime
from pathlib import Path
from typing import NamedTuple


def load_timing_summary_required_keys() -> tuple[str, ...]:
    module_path = Path(__file__).resolve().parent / "proof_timing_keys.py"
    spec = importlib.util.spec_from_file_location("proof_timing_keys", module_path)
    if spec is None or spec.loader is None:
        raise ImportError(f"failed to load proof timing keys from {module_path}")
    module = importlib.util.module_from_spec(spec)
    previous_dont_write_bytecode = sys.dont_write_bytecode
    sys.dont_write_bytecode = True
    try:
        spec.loader.exec_module(module)
    finally:
        sys.dont_write_bytecode = previous_dont_write_bytecode
    return tuple(module.TIMING_SUMMARY_REQUIRED_KEYS)


TIMING_SUMMARY_REQUIRED_KEYS = load_timing_summary_required_keys()


SAFE_NAME_RE = re.compile(r"^[A-Za-z0-9._-]+$")
TIMING_TOTAL_RE = re.compile(r"^timing_total_ms=", re.MULTILINE)
DEFAULT_NVIDIA_SMI_COMMAND = "nvidia-smi"
DEFAULT_MIN_GPU_FREE_MIB = 1024
DEFAULT_GPU_MEMORY_WAIT_TIMEOUT_S = 0.0
DEFAULT_GPU_MEMORY_WAIT_POLL_S = 5.0


class GpuMemoryRow(NamedTuple):
    index: int
    uuid: str | None
    total: int
    used: int
    free: int


class ProofTimingSummaryResult(NamedTuple):
    written: bool
    skip_reason: str | None
    missing_keys: list[str]


def workspace_root() -> Path:
    return Path(__file__).resolve().parent.parent


def resolve_workspace_path(path: str, root: Path) -> Path:
    candidate = Path(path)
    if candidate.is_absolute():
        return candidate
    return root / candidate


def require_workspace_temp_path(path: Path, root: Path, label: str) -> Path:
    temp_dir = (root / "temp").resolve(strict=False)
    resolved = path.resolve(strict=False)
    if resolved != temp_dir and temp_dir not in resolved.parents:
        raise SystemExit(f"{label} must be under {temp_dir}: {path}")
    return path


def profile_name(raw: str | None) -> str:
    if raw is None:
        timestamp = datetime.now().astimezone().strftime("%Y%m%dT%H%M%S%z")
        return f"{timestamp}-{os.getpid()}"
    if not SAFE_NAME_RE.fullmatch(raw):
        raise argparse.ArgumentTypeError(
            "--name may contain only letters, digits, '.', '_', and '-'"
        )
    if raw in {".", ".."}:
        raise argparse.ArgumentTypeError("--name must be a regular profile name")
    return raw


def strip_separator(command: list[str]) -> list[str]:
    if command and command[0] == "--":
        return command[1:]
    return command


def shell_join(command: list[str | Path]) -> str:
    return shlex.join([str(part) for part in command])


def display_path_for_shell(path: Path, root: Path) -> str:
    resolved = path.resolve(strict=False)
    try:
        return str(resolved.relative_to(root.resolve(strict=False)))
    except ValueError:
        return str(resolved)


def resolve_tool(explicit: str | None, env_name: str, default: str) -> str:
    if explicit:
        return explicit
    env_value = os.environ.get(env_name)
    if env_value:
        return env_value
    return shutil.which(default) or default


def positive_mib(raw: str) -> int:
    try:
        value = int(raw)
    except ValueError as error:
        raise argparse.ArgumentTypeError(f"invalid MiB value: {raw!r}") from error
    if value <= 0:
        raise argparse.ArgumentTypeError("MiB value must be positive")
    return value


def positive_timeout(raw: str) -> float:
    try:
        value = float(raw)
    except ValueError as error:
        raise argparse.ArgumentTypeError(f"invalid timeout: {raw!r}") from error
    if value <= 0.0:
        raise argparse.ArgumentTypeError("timeout must be positive")
    return value


def nonnegative_float(raw: str) -> float:
    try:
        value = float(raw)
    except ValueError as error:
        raise argparse.ArgumentTypeError(f"invalid float: {raw!r}") from error
    if value < 0.0:
        raise argparse.ArgumentTypeError("value must be nonnegative")
    return value


def tool_config(args: argparse.Namespace) -> tuple[str | None, str, str]:
    if args.tool == "nsys":
        return args.nsys_command, "LZVM_NSYS_COMMAND", "nsys"
    return args.ncu_command, "LZVM_NCU_COMMAND", "ncu"


def tool_source(args: argparse.Namespace) -> str:
    explicit, env_name, _default = tool_config(args)
    if explicit:
        return "arg"
    if os.environ.get(env_name):
        return "env"
    return "path"


def executable_path(command: str, cwd: Path) -> Path | None:
    path = Path(command)
    if path.is_absolute():
        candidates = [path]
    elif len(path.parts) > 1:
        candidates = [cwd / path]
    else:
        found = shutil.which(command)
        candidates = [Path(found)] if found is not None else []
    for candidate in candidates:
        if candidate.is_file() and os.access(candidate, os.X_OK):
            return candidate
    return None


def prefixed_path(output_dir: Path, name: str, suffix: str) -> Path:
    return Path(str(output_dir / name) + suffix)


def line_key_present(text: str, key: str) -> bool:
    return any(line.startswith(f"{key}=") for line in text.splitlines())


def missing_timing_summary_keys(text: str) -> list[str]:
    return [key for key in TIMING_SUMMARY_REQUIRED_KEYS if not line_key_present(text, key)]


def common_outputs(output_dir: Path, name: str) -> dict[str, Path]:
    return {
        "profile_stdout": prefixed_path(output_dir, name, ".profile.stdout"),
        "profile_stderr": prefixed_path(output_dir, name, ".profile.stderr"),
        "profile_log": prefixed_path(output_dir, name, ".profile.log"),
        "profile_json": prefixed_path(output_dir, name, ".profile.json"),
        "proof_timing_summary": prefixed_path(
            output_dir,
            name,
            ".proof-timing-summary.csv",
        ),
    }


def nsys_outputs(output_dir: Path, name: str) -> dict[str, Path]:
    return {
        **common_outputs(output_dir, name),
        "prefix": output_dir / name,
        "tmp_dir": prefixed_path(output_dir, name, ".tmp"),
        "target_tmp_dir": prefixed_path(output_dir, name, ".target.tmp"),
        "report": prefixed_path(output_dir, name, ".nsys-rep"),
        "sqlite": prefixed_path(output_dir, name, ".sqlite"),
        "export_stdout": prefixed_path(output_dir, name, ".nsys-export.stdout"),
        "export_stderr": prefixed_path(output_dir, name, ".nsys-export.stderr"),
        "kernel_summary": prefixed_path(output_dir, name, ".nsys-kernel-summary.txt"),
        "sync_summary": prefixed_path(output_dir, name, ".nsys-sync-summary.txt"),
        "copy_summary": prefixed_path(output_dir, name, ".nsys-copy-summary.txt"),
    }


def ncu_outputs(output_dir: Path, name: str) -> dict[str, Path]:
    return {
        **common_outputs(output_dir, name),
        "tmp_dir": prefixed_path(output_dir, name, ".tmp"),
        "target_tmp_dir": prefixed_path(output_dir, name, ".target.tmp"),
        "report": prefixed_path(output_dir, name, ".ncu-rep"),
        "csv": prefixed_path(output_dir, name, ".ncu.csv"),
        "kernel_summary": prefixed_path(output_dir, name, ".ncu-kernel-summary.txt"),
    }


def build_nsys_command(
    args: argparse.Namespace,
    output_dir: Path,
    command: list[str],
) -> tuple[list[str], dict[str, Path]]:
    outputs = nsys_outputs(output_dir, args.name)
    profile = [
        resolve_tool(args.nsys_command, "LZVM_NSYS_COMMAND", "nsys"),
        "profile",
        "--force-overwrite=true",
        "--stats=false",
        f"--trace={args.nsys_trace}",
        "--output",
        str(outputs["prefix"]),
        *args.profile_arg,
        "--",
        *target_command(outputs, command),
    ]
    return profile, outputs


def build_nsys_export_command(
    args: argparse.Namespace,
    outputs: dict[str, Path],
) -> list[str]:
    return [
        resolve_tool(args.nsys_command, "LZVM_NSYS_COMMAND", "nsys"),
        "export",
        "--type",
        "sqlite",
        "--force-overwrite=true",
        "--output",
        str(outputs["sqlite"]),
        str(outputs["report"]),
    ]


def build_ncu_command(
    args: argparse.Namespace,
    output_dir: Path,
    command: list[str],
) -> tuple[list[str], dict[str, Path]]:
    outputs = ncu_outputs(output_dir, args.name)
    profile = [
        resolve_tool(args.ncu_command, "LZVM_NCU_COMMAND", "ncu"),
        "--target-processes",
        args.ncu_target_processes,
        "--set",
        args.ncu_set,
        "--page",
        "raw",
        "--csv",
        "--log-file",
        str(outputs["csv"]),
        "--export",
        str(outputs["report"]),
        "--force-overwrite",
        *args.profile_arg,
        "--",
        *target_command(outputs, command),
    ]
    return profile, outputs


def print_common_outputs(outputs: dict[str, Path], root: Path) -> None:
    print(f"profile_stdout={display_path_for_shell(outputs['profile_stdout'], root)}")
    print(f"profile_stderr={display_path_for_shell(outputs['profile_stderr'], root)}")
    print(f"profile_log={display_path_for_shell(outputs['profile_log'], root)}")
    print(f"profile_json_output={display_path_for_shell(outputs['profile_json'], root)}")
    print(
        "proof_timing_summary_output="
        f"{display_path_for_shell(outputs['proof_timing_summary'], root)}"
    )


def print_nsys_outputs(args: argparse.Namespace, outputs: dict[str, Path], root: Path) -> None:
    tmp_dir = display_path_for_shell(outputs["tmp_dir"], root)
    target_tmp_dir = display_path_for_shell(outputs["target_tmp_dir"], root)
    report = display_path_for_shell(outputs["report"], root)
    sqlite = display_path_for_shell(outputs["sqlite"], root)
    kernel_summary = display_path_for_shell(outputs["kernel_summary"], root)
    sync_summary = display_path_for_shell(outputs["sync_summary"], root)
    copy_summary = display_path_for_shell(outputs["copy_summary"], root)
    print(f"profile_tmp_dir={tmp_dir}")
    print(f"profile_target_tmp_dir={target_tmp_dir}")
    print_common_outputs(outputs, root)
    print(f"nsys_report={report}")
    print(f"nsys_sqlite={sqlite}")
    print(
        "nsys_export_command="
        + shell_join(
            [
                resolve_tool(args.nsys_command, "LZVM_NSYS_COMMAND", "nsys"),
                "export",
                "--type",
                "sqlite",
                "--force-overwrite=true",
                "--output",
                sqlite,
                report,
            ]
        )
    )
    for script, summary in [
        ("scripts/nsys-cuda-kernel-summary.py", kernel_summary),
        ("scripts/nsys-cuda-sync-summary.py", sync_summary),
        ("scripts/nsys-cuda-copy-summary.py", copy_summary),
    ]:
        name = Path(script).stem.replace("-", "_")
        print(f"{name}_command=" + shell_join([script, sqlite]))
        print(f"{name}_output={summary}")


def print_ncu_outputs(outputs: dict[str, Path], root: Path) -> None:
    tmp_dir = display_path_for_shell(outputs["tmp_dir"], root)
    target_tmp_dir = display_path_for_shell(outputs["target_tmp_dir"], root)
    report = display_path_for_shell(outputs["report"], root)
    csv_path = display_path_for_shell(outputs["csv"], root)
    kernel_summary = display_path_for_shell(outputs["kernel_summary"], root)
    print(f"profile_tmp_dir={tmp_dir}")
    print(f"profile_target_tmp_dir={target_tmp_dir}")
    print_common_outputs(outputs, root)
    print(f"ncu_report={report}")
    print(f"ncu_csv={csv_path}")
    print(
        "ncu_cuda_kernel_summary_command="
        + shell_join(["scripts/ncu-cuda-kernel-summary.py", csv_path])
    )
    print(f"ncu_cuda_kernel_summary_output={kernel_summary}")


def profile_env(outputs: dict[str, Path]) -> dict[str, str]:
    tmp_dir = outputs["tmp_dir"]
    if tmp_dir.is_symlink() or not tmp_dir.is_dir():
        raise SystemExit(f"tmp_dir output path is not a prepared directory: {tmp_dir}")
    env = os.environ.copy()
    env["TMPDIR"] = str(tmp_dir)
    return env


def target_command(outputs: dict[str, Path], command: list[str]) -> list[str]:
    return ["env", f"TMPDIR={outputs['target_tmp_dir']}", *command]


def reject_symlinked_output_paths(outputs: dict[str, Path]) -> None:
    for key, path in sorted(outputs.items()):
        if isinstance(path, Path) and path.is_symlink():
            raise SystemExit(f"{key} output path must not be a symlink: {path}")


def reject_symlinked_output_keys(outputs: dict[str, Path], keys: list[str]) -> None:
    for key in keys:
        path = outputs[key]
        if path.is_symlink():
            raise SystemExit(f"{key} output path must not be a symlink: {path}")


def profile_tool_output_keys(args: argparse.Namespace) -> list[str]:
    if args.tool == "nsys":
        return ["report"]
    return ["report", "csv"]


def open_text_no_follow(path: Path, mode: int = 0o600):
    flags = os.O_WRONLY | os.O_CREAT | os.O_TRUNC
    if hasattr(os, "O_NOFOLLOW"):
        flags |= os.O_NOFOLLOW
    try:
        descriptor = os.open(path, flags, mode)
    except OSError as error:
        if path.is_symlink():
            raise SystemExit(f"output path must not be a symlink: {path}") from error
        raise
    return os.fdopen(descriptor, "w", encoding="utf-8")


def write_text_no_follow(path: Path, text: str) -> None:
    with open_text_no_follow(path) as output:
        output.write(text)


def prepare_managed_tmp_dir(path: Path, key: str) -> None:
    if path.exists() or path.is_symlink():
        raise SystemExit(f"{key} output path must not already exist: {path}")
    path.mkdir(mode=0o700)


def prepare_output_dirs(output_dir: Path, outputs: dict[str, Path]) -> None:
    output_dir.mkdir(parents=True, exist_ok=True)
    reject_symlinked_output_paths(outputs)
    prepare_managed_tmp_dir(outputs["tmp_dir"], "tmp_dir")
    prepare_managed_tmp_dir(outputs["target_tmp_dir"], "target_tmp_dir")
    reject_symlinked_output_paths(outputs)


def run_captured(
    command: list[str],
    cwd: Path,
    stdout_path: Path,
    stderr_path: Path,
    env: dict[str, str],
) -> int:
    completed = subprocess.run(
        command,
        cwd=cwd,
        env=env,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    write_text_no_follow(stdout_path, completed.stdout)
    write_text_no_follow(stderr_path, completed.stderr)
    return completed.returncode


def tee_pipe(pipe, output_path: Path, sink) -> None:
    with open_text_no_follow(output_path) as output:
        for chunk in iter(pipe.readline, ""):
            output.write(chunk)
            output.flush()
            sink.write(chunk)
            sink.flush()
    pipe.close()


def write_combined_profile_log(outputs: dict[str, Path]) -> None:
    reject_symlinked_output_keys(outputs, ["profile_stdout", "profile_stderr", "profile_log"])
    stdout = outputs["profile_stdout"].read_text(encoding="utf-8")
    stderr = outputs["profile_stderr"].read_text(encoding="utf-8")
    write_text_no_follow(
        outputs["profile_log"],
        "[stdout]\n"
        + stdout
        + ("" if not stdout or stdout.endswith("\n") else "\n")
        + "[stderr]\n"
        + stderr
        + ("" if not stderr or stderr.endswith("\n") else "\n"),
    )


def run_profile_command(
    command: list[str],
    cwd: Path,
    env: dict[str, str],
    outputs: dict[str, Path],
) -> int:
    process = subprocess.Popen(
        command,
        cwd=cwd,
        env=env,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        bufsize=1,
    )
    if process.stdout is None or process.stderr is None:
        raise SystemExit("failed to capture profile command output")
    stdout_thread = threading.Thread(
        target=tee_pipe,
        args=(process.stdout, outputs["profile_stdout"], sys.stdout),
    )
    stderr_thread = threading.Thread(
        target=tee_pipe,
        args=(process.stderr, outputs["profile_stderr"], sys.stderr),
    )
    stdout_thread.start()
    stderr_thread.start()
    code = process.wait()
    stdout_thread.join()
    stderr_thread.join()
    write_combined_profile_log(outputs)
    return code


def run_summary(
    command: list[str],
    cwd: Path,
    output_path: Path,
    env: dict[str, str],
) -> None:
    stderr_path = prefixed_path(output_path.parent, output_path.name, ".stderr")
    code = run_captured(command, cwd, output_path, stderr_path, env)
    if code != 0:
        diagnostic = first_diagnostic_line(stderr_path.read_text(encoding="utf-8"))
        diagnostic_suffix = f"; first stderr line: {diagnostic}" if diagnostic else ""
        raise SystemExit(
            f"summary command failed with status {code}: "
            f"{shell_join(command)}; stderr: {stderr_path}"
            f"{diagnostic_suffix}"
        )


def profile_json_outputs(outputs: dict[str, Path], root: Path) -> dict[str, str]:
    return {
        key: display_path_for_shell(path, root)
        for key, path in sorted(outputs.items())
        if isinstance(path, Path)
    }


def write_profile_json(
    args: argparse.Namespace,
    root: Path,
    cwd: Path,
    profile_command: list[str],
    command: list[str],
    outputs: dict[str, Path],
    status: str,
    profile_exit_code: int | None = None,
    proof_timing_summary_written: bool = False,
    proof_timing_summary_skip_reason: str | None = None,
    proof_timing_summary_missing_keys: list[str] | None = None,
    tool_summary_paths: list[Path] | None = None,
    gpu_memory_check: dict[str, object] | None = None,
    error: str | None = None,
) -> None:
    payload = {
        "created_at": datetime.now().astimezone().strftime("%Y-%m-%dT%H:%M:%S%z"),
        "tool": args.tool,
        "name": args.name,
        "status": status,
        "profile_exit_code": profile_exit_code,
        "cwd": display_path_for_shell(cwd, root),
        "command": command,
        "profile_command": profile_command,
        "summarize": args.summarize,
        "proof_timing_summary": args.proof_timing_summary,
        "require_proof_timing_summary": args.require_proof_timing_summary,
        "outputs": profile_json_outputs(outputs, root),
        "proof_timing_summary_written": proof_timing_summary_written,
        "tool_summaries": [
            display_path_for_shell(path, root) for path in (tool_summary_paths or [])
        ],
    }
    if proof_timing_summary_skip_reason is not None:
        payload["proof_timing_summary_skip_reason"] = proof_timing_summary_skip_reason
    if proof_timing_summary_missing_keys:
        payload["proof_timing_summary_missing_keys"] = proof_timing_summary_missing_keys
    if gpu_memory_check is not None:
        payload["gpu_memory_check"] = gpu_memory_check
    if error is not None:
        payload["error"] = error
    write_text_no_follow(
        outputs["profile_json"],
        json.dumps(payload, indent=2, sort_keys=True) + "\n",
    )


def export_nsys_sqlite(args: argparse.Namespace, root: Path, outputs: dict[str, Path]) -> None:
    command = build_nsys_export_command(args, outputs)
    code = run_captured(
        command,
        root,
        outputs["export_stdout"],
        outputs["export_stderr"],
        profile_env(outputs),
    )
    if code != 0:
        raise SystemExit(
            "nsys export failed with status "
            f"{code}: {outputs['export_stderr']}"
        )
    reject_symlinked_output_keys(outputs, ["sqlite"])


def summarize_nsys(root: Path, outputs: dict[str, Path]) -> None:
    env = profile_env(outputs)
    for script, output in [
        ("scripts/nsys-cuda-kernel-summary.py", outputs["kernel_summary"]),
        ("scripts/nsys-cuda-sync-summary.py", outputs["sync_summary"]),
        ("scripts/nsys-cuda-copy-summary.py", outputs["copy_summary"]),
    ]:
        run_summary([script, str(outputs["sqlite"])], root, output, env)


def summarize_ncu(root: Path, outputs: dict[str, Path]) -> None:
    env = profile_env(outputs)
    run_summary(
        ["scripts/ncu-cuda-kernel-summary.py", str(outputs["csv"])],
        root,
        outputs["kernel_summary"],
        env,
    )


def remove_stale_summary(output_path: Path) -> None:
    stderr_path = prefixed_path(output_path.parent, output_path.name, ".stderr")
    for path in [output_path, stderr_path]:
        try:
            path.unlink()
        except FileNotFoundError:
            pass


def summarize_proof_timing(
    root: Path,
    outputs: dict[str, Path],
    extra_args: list[str] | None = None,
) -> ProofTimingSummaryResult:
    profile_log = outputs["profile_log"]
    log_text = profile_log.read_text(encoding="utf-8")
    if TIMING_TOTAL_RE.search(log_text) is None:
        remove_stale_summary(outputs["proof_timing_summary"])
        print("proof_timing_summary=skipped_no_timing_total")
        return ProofTimingSummaryResult(False, "missing_timing_total", [])
    missing_keys = missing_timing_summary_keys(log_text)
    if missing_keys:
        remove_stale_summary(outputs["proof_timing_summary"])
        print("proof_timing_summary=skipped_missing_keys=" + ";".join(missing_keys))
        return ProofTimingSummaryResult(False, "missing_keys", missing_keys)
    command = ["scripts/prove-timing-root-summary.py"]
    if extra_args:
        command.extend(extra_args)
    command.append(str(profile_log))
    run_summary(
        command,
        root,
        outputs["proof_timing_summary"],
        profile_env(outputs),
    )
    print(
        "proof_timing_summary="
        f"{display_path_for_shell(outputs['proof_timing_summary'], root)}"
    )
    return ProofTimingSummaryResult(True, None, [])


def merge_proof_timing_result(
    result: ProofTimingSummaryResult,
    written: bool,
    skip_reason: str | None,
    missing_keys: list[str],
) -> ProofTimingSummaryResult:
    if result.written:
        return ProofTimingSummaryResult(True, None, [])
    if written:
        return ProofTimingSummaryResult(written, skip_reason, missing_keys)
    return result


def validate_static_profile_args(args: argparse.Namespace, root: Path) -> tuple[Path, Path]:
    if args.tool == "nsys" and args.summarize and args.skip_nsys_export:
        raise SystemExit("--summarize requires nsys SQLite export; remove --skip-nsys-export")
    if args.require_proof_timing_summary and not (
        args.summarize or args.proof_timing_summary
    ):
        raise SystemExit(
            "--require-proof-timing-summary requires --summarize or --proof-timing-summary"
        )
    output_dir = require_workspace_temp_path(
        resolve_workspace_path(args.output_dir, root),
        root,
        "--output-dir",
    )
    cwd = resolve_workspace_path(args.cwd, root)
    if not cwd.exists():
        raise SystemExit(f"{cwd}: command working directory does not exist")
    if not cwd.is_dir():
        raise SystemExit(f"{cwd}: command working directory is not a directory")
    return output_dir, cwd


def check_profile_tool(args: argparse.Namespace) -> int:
    root = workspace_root()
    output_dir, cwd = validate_static_profile_args(args, root)
    explicit, env_name, default = tool_config(args)
    command = resolve_tool(explicit, env_name, default)
    resolved = executable_path(command, cwd)
    print(f"tool={args.tool}")
    print(f"tool_source={tool_source(args)}")
    print(f"tool_command={command}")
    print(f"tool_status={'ready' if resolved is not None else 'missing'}")
    if resolved is not None:
        print(f"tool_resolved={display_path_for_shell(resolved, root)}")
    print(f"output_dir={display_path_for_shell(output_dir, root)}")
    print(f"cwd={display_path_for_shell(cwd, root)}")
    return 0 if resolved is not None else 1


def gpu_memory_tool_spec(args: argparse.Namespace) -> tuple[str, str]:
    if args.nvidia_smi_command is not None:
        return ("arg", args.nvidia_smi_command)
    env_value = os.environ.get("LZVM_NVIDIA_SMI_COMMAND")
    if env_value:
        return ("env", env_value)
    return ("path", DEFAULT_NVIDIA_SMI_COMMAND)


def parse_gpu_memory_rows(text: str) -> list[GpuMemoryRow]:
    rows: list[GpuMemoryRow] = []
    for line in text.splitlines():
        if not line.strip():
            continue
        parts = [part.strip() for part in line.split(",")]
        if len(parts) == 4:
            index_text, total_text, used_text, free_text = parts
            uuid = None
        elif len(parts) == 5:
            index_text, uuid, total_text, used_text, free_text = parts
            uuid = uuid or None
        else:
            raise ValueError(f"expected 4 or 5 columns, found {len(parts)}")
        index = int(index_text)
        total = int(total_text)
        used = int(used_text)
        free = int(free_text)
        if min(index, total, used, free) < 0:
            raise ValueError("memory values must be nonnegative")
        rows.append(GpuMemoryRow(index=index, uuid=uuid, total=total, used=used, free=free))
    if not rows:
        raise ValueError("no GPU rows returned")
    return rows


def default_cuda_visible_device() -> str | None:
    visible_devices = os.environ.get("CUDA_VISIBLE_DEVICES")
    if visible_devices is None:
        return "0"
    token = next((part.strip() for part in visible_devices.split(",") if part.strip()), "")
    if token.lower() in {"", "-1", "none", "nodevfiles", "void"}:
        return None
    return token


def select_default_cuda_gpu_memory_row(rows: list[GpuMemoryRow]) -> GpuMemoryRow:
    token = default_cuda_visible_device()
    if token is None:
        raise ValueError("CUDA_VISIBLE_DEVICES hides all CUDA devices")
    if token.isdecimal():
        requested_index = int(token)
        for row in rows:
            if row.index == requested_index:
                return row
        if os.environ.get("CUDA_VISIBLE_DEVICES") is None:
            return rows[0]
        raise ValueError(
            f"default CUDA device index {requested_index} was not returned by nvidia-smi"
        )
    for row in rows:
        if row.uuid == token:
            return row
    raise ValueError(f"default CUDA device {token!r} was not returned by nvidia-smi")


def first_diagnostic_line(text: str) -> str:
    lines = [line.strip() for line in text.splitlines() if line.strip()]
    return lines[0][:200] if lines else ""


def gpu_memory_wait_requested(args: argparse.Namespace) -> bool:
    return (
        args.gpu_memory_wait_timeout_s != DEFAULT_GPU_MEMORY_WAIT_TIMEOUT_S
        or args.gpu_memory_wait_poll_s != DEFAULT_GPU_MEMORY_WAIT_POLL_S
    )


def gpu_memory_check_payload_once(
    args: argparse.Namespace,
    root: Path,
) -> tuple[bool, dict[str, object]]:
    source, raw = gpu_memory_tool_spec(args)
    resolved = executable_path(raw, root)
    payload: dict[str, object] = {
        "source": source,
        "command": raw,
        "min_free_mib": args.min_gpu_free_mib,
    }
    visible_devices = os.environ.get("CUDA_VISIBLE_DEVICES")
    if visible_devices is not None:
        payload["cuda_visible_devices"] = visible_devices
    if resolved is None:
        payload["status"] = "missing"
        return False, payload
    payload["resolved"] = display_path_for_shell(resolved, root)
    query = [
        str(resolved),
        "--query-gpu=index,uuid,memory.total,memory.used,memory.free",
        "--format=csv,noheader,nounits",
    ]
    try:
        result = subprocess.run(
            query,
            cwd=root,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            timeout=10.0,
            check=False,
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        payload["status"] = "query_failed"
        payload["error"] = str(error)
        return False, payload
    if result.returncode != 0:
        payload["status"] = "query_failed"
        payload["exit_code"] = result.returncode
        diagnostic = first_diagnostic_line(result.stderr)
        if diagnostic:
            payload["error"] = diagnostic
        return False, payload
    try:
        rows = parse_gpu_memory_rows(result.stdout)
    except ValueError as error:
        payload["status"] = "parse_failed"
        payload["error"] = str(error)
        return False, payload
    try:
        selected = select_default_cuda_gpu_memory_row(rows)
    except ValueError as error:
        payload["status"] = "device_unavailable"
        payload["error"] = str(error)
        return False, payload
    payload["device_count"] = len(rows)
    payload["selected_index"] = selected.index
    if selected.uuid is not None:
        payload["selected_uuid"] = selected.uuid
    payload["total_mib"] = selected.total
    payload["used_mib"] = selected.used
    payload["free_mib"] = selected.free
    ready = selected.free >= args.min_gpu_free_mib
    payload["status"] = "ready" if ready else "low"
    return ready, payload


def gpu_memory_check_payload(
    args: argparse.Namespace,
    root: Path,
) -> tuple[bool, dict[str, object]]:
    wait_timeout_s = args.gpu_memory_wait_timeout_s
    if wait_timeout_s <= 0.0:
        return gpu_memory_check_payload_once(args, root)
    deadline = time.monotonic() + wait_timeout_s
    attempt = 1
    while True:
        ready, payload = gpu_memory_check_payload_once(args, root)
        payload["wait_timeout_s"] = wait_timeout_s
        payload["wait_poll_s"] = args.gpu_memory_wait_poll_s
        payload["wait_attempts"] = attempt
        if ready:
            payload["wait_status"] = "ready"
            return True, payload
        remaining_s = deadline - time.monotonic()
        if remaining_s <= 0.0:
            payload["wait_status"] = "timeout"
            return False, payload
        time.sleep(min(args.gpu_memory_wait_poll_s, remaining_s))
        attempt += 1


def print_gpu_memory_payload(payload: dict[str, object]) -> None:
    for key, output_name in [
        ("source", "gpu_memory_source"),
        ("command", "gpu_memory_command"),
        ("min_free_mib", "gpu_memory_min_free_mib"),
        ("cuda_visible_devices", "gpu_memory_cuda_visible_devices"),
        ("resolved", "gpu_memory_resolved"),
        ("device_count", "gpu_memory_device_count"),
        ("selected_index", "gpu_memory_selected_index"),
        ("selected_uuid", "gpu_memory_selected_uuid"),
        ("total_mib", "gpu_memory_total_mib"),
        ("used_mib", "gpu_memory_used_mib"),
        ("free_mib", "gpu_memory_free_mib"),
        ("status", "gpu_memory_status"),
        ("wait_timeout_s", "gpu_memory_wait_timeout_s"),
        ("wait_poll_s", "gpu_memory_wait_poll_s"),
        ("wait_attempts", "gpu_memory_wait_attempts"),
        ("wait_status", "gpu_memory_wait_status"),
        ("exit_code", "gpu_memory_exit_code"),
        ("error", "gpu_memory_error"),
    ]:
        if key in payload:
            print(f"{output_name}={payload[key]}")


def print_gpu_memory_check(args: argparse.Namespace, root: Path) -> bool:
    ready, payload = gpu_memory_check_payload(args, root)
    print_gpu_memory_payload(payload)
    return ready


def run_profile(args: argparse.Namespace) -> int:
    root = workspace_root()
    command = strip_separator(args.command)
    if not command:
        raise SystemExit("profiled command is required after --")
    output_dir, cwd = validate_static_profile_args(args, root)

    if args.tool == "nsys":
        profile_command, outputs = build_nsys_command(args, output_dir, command)
        print("profile_command=" + shell_join(profile_command))
        print_nsys_outputs(args, outputs, root)
    else:
        profile_command, outputs = build_ncu_command(args, output_dir, command)
        print("profile_command=" + shell_join(profile_command))
        print_ncu_outputs(outputs, root)

    gpu_memory_payload = None
    if args.check_gpu_memory:
        gpu_ready, gpu_memory_payload = gpu_memory_check_payload(args, root)
        print_gpu_memory_payload(gpu_memory_payload)
        if not gpu_ready:
            if not args.dry_run:
                output_dir.mkdir(parents=True, exist_ok=True)
                reject_symlinked_output_paths(outputs)
                write_profile_json(
                    args,
                    root,
                    cwd,
                    profile_command,
                    command,
                    outputs,
                    "gpu_memory_failed",
                    profile_exit_code=1,
                    gpu_memory_check=gpu_memory_payload,
                )
            return 1

    if args.dry_run:
        return 0
    prepare_output_dirs(output_dir, outputs)
    write_profile_json(
        args,
        root,
        cwd,
        profile_command,
        command,
        outputs,
        "running",
        gpu_memory_check=gpu_memory_payload,
    )
    profile_code = run_profile_command(
        profile_command,
        cwd=cwd,
        env=profile_env(outputs),
        outputs=outputs,
    )
    if profile_code != 0:
        write_profile_json(
            args,
            root,
            cwd,
            profile_command,
            command,
            outputs,
            "profile_failed",
            profile_exit_code=profile_code,
            gpu_memory_check=gpu_memory_payload,
        )
        return profile_code
    try:
        reject_symlinked_output_keys(outputs, profile_tool_output_keys(args))
    except SystemExit as error:
        write_profile_json(
            args,
            root,
            cwd,
            profile_command,
            command,
            outputs,
            "profile_output_failed",
            profile_exit_code=profile_code,
            gpu_memory_check=gpu_memory_payload,
            error=str(error),
        )
        raise
    proof_timing_summary_written = False
    proof_timing_summary_skip_reason = None
    proof_timing_summary_missing_keys: list[str] = []
    tool_summary_paths: list[Path] = []
    if args.proof_timing_summary:
        try:
            proof_timing_result = summarize_proof_timing(root, outputs)
        except SystemExit as error:
            remove_stale_summary(outputs["proof_timing_summary"])
            write_profile_json(
                args,
                root,
                cwd,
                profile_command,
                command,
                outputs,
                "summary_failed",
                profile_exit_code=profile_code,
                proof_timing_summary_written=False,
                tool_summary_paths=tool_summary_paths,
                gpu_memory_check=gpu_memory_payload,
                error=str(error),
            )
            raise
        proof_timing_summary_written = proof_timing_result.written
        proof_timing_summary_skip_reason = proof_timing_result.skip_reason
        proof_timing_summary_missing_keys = proof_timing_result.missing_keys
    if args.tool == "nsys" and not args.skip_nsys_export:
        try:
            export_nsys_sqlite(args, root, outputs)
            print(f"nsys_exported_sqlite={display_path_for_shell(outputs['sqlite'], root)}")
            if args.summarize:
                summarize_nsys(root, outputs)
                tool_summary_paths.extend(
                    [
                        outputs["kernel_summary"],
                        outputs["sync_summary"],
                        outputs["copy_summary"],
                    ]
                )
                print(
                    "nsys_kernel_summary="
                    f"{display_path_for_shell(outputs['kernel_summary'], root)}"
                )
                print(
                    f"nsys_sync_summary={display_path_for_shell(outputs['sync_summary'], root)}"
                )
                print(
                    f"nsys_copy_summary={display_path_for_shell(outputs['copy_summary'], root)}"
                )
                proof_timing_result = summarize_proof_timing(
                    root,
                    outputs,
                    [
                        "--nsys-kernel-summary",
                        str(outputs["kernel_summary"]),
                        "--nsys-copy-summary",
                        str(outputs["copy_summary"]),
                    ],
                )
                merged = merge_proof_timing_result(
                    proof_timing_result,
                    proof_timing_summary_written,
                    proof_timing_summary_skip_reason,
                    proof_timing_summary_missing_keys,
                )
                proof_timing_summary_written = merged.written
                proof_timing_summary_skip_reason = merged.skip_reason
                proof_timing_summary_missing_keys = merged.missing_keys
        except SystemExit as error:
            if args.summarize:
                remove_stale_summary(outputs["proof_timing_summary"])
            write_profile_json(
                args,
                root,
                cwd,
                profile_command,
                command,
                outputs,
                "summary_failed",
                profile_exit_code=profile_code,
                proof_timing_summary_written=False,
                tool_summary_paths=tool_summary_paths,
                gpu_memory_check=gpu_memory_payload,
                error=str(error),
            )
            raise
    elif args.tool == "ncu" and args.summarize:
        try:
            summarize_ncu(root, outputs)
            tool_summary_paths.append(outputs["kernel_summary"])
            print(
                f"ncu_kernel_summary={display_path_for_shell(outputs['kernel_summary'], root)}"
            )
            proof_timing_result = summarize_proof_timing(
                root,
                outputs,
                ["--ncu-kernel-summary", str(outputs["kernel_summary"])],
            )
            merged = merge_proof_timing_result(
                proof_timing_result,
                proof_timing_summary_written,
                proof_timing_summary_skip_reason,
                proof_timing_summary_missing_keys,
            )
            proof_timing_summary_written = merged.written
            proof_timing_summary_skip_reason = merged.skip_reason
            proof_timing_summary_missing_keys = merged.missing_keys
        except SystemExit as error:
            remove_stale_summary(outputs["proof_timing_summary"])
            write_profile_json(
                args,
                root,
                cwd,
                profile_command,
                command,
                outputs,
                "summary_failed",
                profile_exit_code=profile_code,
                proof_timing_summary_written=False,
                tool_summary_paths=tool_summary_paths,
                gpu_memory_check=gpu_memory_payload,
                error=str(error),
            )
            raise
    if args.require_proof_timing_summary and not proof_timing_summary_written:
        reason = proof_timing_summary_skip_reason or "not_written"
        if proof_timing_summary_missing_keys:
            reason += ": " + ",".join(proof_timing_summary_missing_keys)
        error = f"required proof timing summary was not written: {reason}"
        print(error, file=sys.stderr)
        write_profile_json(
            args,
            root,
            cwd,
            profile_command,
            command,
            outputs,
            "summary_failed",
            profile_exit_code=profile_code,
            proof_timing_summary_written=False,
            proof_timing_summary_skip_reason=proof_timing_summary_skip_reason,
            proof_timing_summary_missing_keys=proof_timing_summary_missing_keys,
            tool_summary_paths=tool_summary_paths,
            gpu_memory_check=gpu_memory_payload,
            error=error,
        )
        return 1
    write_profile_json(
        args,
        root,
        cwd,
        profile_command,
        command,
        outputs,
        "ok",
        profile_exit_code=profile_code,
        proof_timing_summary_written=proof_timing_summary_written,
        proof_timing_summary_skip_reason=proof_timing_summary_skip_reason,
        proof_timing_summary_missing_keys=proof_timing_summary_missing_keys,
        tool_summary_paths=tool_summary_paths,
        gpu_memory_check=gpu_memory_payload,
    )
    return 0


def write_fake_profiler(
    path: Path,
    tool: str,
    log_path: Path,
    ncu_csv_text: str = "Kernel Name,gpu__time_duration.sum\nself,1\n",
) -> None:
    path.write_text(
        "\n".join(
            [
                "#!/usr/bin/env python3",
                "import os",
                "import pathlib",
                "import subprocess",
                "import sys",
                f"log = pathlib.Path({str(log_path)!r})",
                "log.parent.mkdir(parents=True, exist_ok=True)",
                "lines = ["
                + repr(tool)
                + ", 'tmpdir=' + os.environ.get('TMPDIR', ''), *sys.argv[1:]]",
                "with log.open('a', encoding='utf-8') as output:",
                "    output.write('\\n'.join(lines) + '\\n')",
                "args = sys.argv[1:]",
                "if 'profile' in args and '--output' in args:",
                "    prefix = pathlib.Path(args[args.index('--output') + 1])",
                "    pathlib.Path(str(prefix) + '.nsys-rep').write_text('report\\n', encoding='utf-8')",
                "if 'export' in args and '--output' in args:",
                "    pathlib.Path(args[args.index('--output') + 1]).write_text('sqlite\\n', encoding='utf-8')",
                "if '--log-file' in args:",
                "    pathlib.Path(args[args.index('--log-file') + 1]).write_text("
                + repr(ncu_csv_text)
                + ", encoding='utf-8')",
                "if '--export' in args:",
                "    pathlib.Path(args[args.index('--export') + 1]).write_text('report\\n', encoding='utf-8')",
                "if '--' in args:",
                "    command = args[args.index('--') + 1:]",
                "    if command:",
                "        raise SystemExit(subprocess.run(command).returncode)",
            ]
        )
        + "\n",
        encoding="utf-8",
    )
    path.chmod(0o755)


def csv_value(text: str, column: str) -> str:
    rows = list(csv.reader(text.splitlines()))
    if len(rows) < 2:
        raise SystemExit("summary CSV missing data row")
    header = rows[0]
    try:
        index = header.index(column)
    except ValueError as error:
        raise SystemExit(f"summary CSV missing column {column}") from error
    try:
        return rows[1][index]
    except IndexError as error:
        raise SystemExit(f"summary CSV row missing column {column}") from error


def self_test() -> None:
    root = workspace_root()
    work_dir = root / "temp" / f"proof-profile-self-test-{os.getpid()}"
    shutil.rmtree(work_dir, ignore_errors=True)
    work_dir.mkdir(parents=True)
    fake_nsys = work_dir / "fake-nsys.py"
    fake_ncu = work_dir / "fake-ncu.py"
    fake_bad_ncu = work_dir / "fake-bad-ncu.py"
    write_fake_profiler(fake_nsys, "nsys", work_dir / "fake-nsys.argv")
    write_fake_profiler(fake_ncu, "ncu", work_dir / "fake-ncu.argv")
    write_fake_profiler(
        fake_bad_ncu,
        "bad-ncu",
        work_dir / "fake-bad-ncu.argv",
        "Kernel Name,unexpected_metric\nself,1\n",
    )
    try:
        base = {
            "command": [
                sys.executable,
                "-c",
                (
                    "print('timing_total_ms=1000'); "
                    "print('timing_guest_stage_tree_commit_root_count=1'); "
                    "print('timing_guest_stage_tree_commit_root_materialization_groups=1'); "
                    "print('timing_guest_stage_tree_commit_root_materialization_max_group_size=1'); "
                    "print('timing_finish_witness_opening_row_dedup_input_rows=0'); "
                    "print('timing_finish_witness_opening_row_dedup_unique_rows=0'); "
                    "print('timing_finish_witness_opening_row_dedup_elided_rows=0'); "
                    "print('timing_finish_fri_opening_ms=10'); "
                    "print('timing_finish_fri_opening_unit_build_ms=8'); "
                    "print('timing_finish_fri_opening_layer_tree_ms=2'); "
                    "print('timing_finish_fri_opening_query_ms=3'); "
                    "print('timing_finish_fri_opening_fold_ms=1'); "
                    "print('timing_finish_fri_opening_unit_count=1'); "
                    "print('timing_finish_fri_opening_layer_count=2'); "
                    "print('timing_finish_fri_opening_query_count=3'); "
                    "print('timing_finish_fri_transcript_unit_build_ms=4'); "
                    "print('timing_finish_fri_transcript_layer_tree_ms=2'); "
                    "print('timing_finish_fri_transcript_fold_ms=1'); "
                    "print('timing_finish_fri_transcript_unit_count=1'); "
                    "print('timing_finish_fri_transcript_layer_count=2'); "
                    "print('timing_finish_contribution_segment_ms=5'); "
                    "print('timing_finish_contribution_verify_ms=6'); "
                    "print('timing_finish_contribution_challenge_ms=7')"
                ),
            ],
            "cwd": ".",
            "dry_run": False,
            "name": "self-test",
            "output_dir": str(work_dir / "profiles"),
            "profile_arg": [],
            "ncu_set": "basic",
            "ncu_target_processes": "all",
            "nsys_trace": "cuda,nvtx,osrt",
            "skip_nsys_export": False,
            "summarize": False,
            "proof_timing_summary": False,
            "require_proof_timing_summary": False,
            "check_gpu_memory": False,
            "gpu_memory_wait_timeout_s": DEFAULT_GPU_MEMORY_WAIT_TIMEOUT_S,
            "gpu_memory_wait_poll_s": DEFAULT_GPU_MEMORY_WAIT_POLL_S,
            "min_gpu_free_mib": DEFAULT_MIN_GPU_FREE_MIB,
            "nvidia_smi_command": None,
        }
        nsys_args = argparse.Namespace(
            **base,
            tool="nsys",
            nsys_command=str(fake_nsys),
            ncu_command=None,
        )
        if run_profile(nsys_args) != 0:
            raise SystemExit("fake nsys profile failed")
        if not (work_dir / "profiles" / "self-test.nsys-rep").exists():
            raise SystemExit("fake nsys report missing")
        if not (work_dir / "profiles" / "self-test.sqlite").exists():
            raise SystemExit("fake nsys sqlite export missing")
        nsys_profile_log = work_dir / "profiles" / "self-test.profile.log"
        if not (work_dir / "profiles" / "self-test.profile.stdout").exists():
            raise SystemExit("fake nsys profile stdout missing")
        if not (work_dir / "profiles" / "self-test.profile.stderr").exists():
            raise SystemExit("fake nsys profile stderr missing")
        if "timing_total_ms=1000" not in nsys_profile_log.read_text(encoding="utf-8"):
            raise SystemExit("fake nsys profile log missing target output")
        nsys_argv = (work_dir / "fake-nsys.argv").read_text(encoding="utf-8")
        if f"tmpdir={work_dir / 'profiles' / 'self-test.tmp'}" not in nsys_argv:
            raise SystemExit("fake nsys did not receive managed TMPDIR")
        if f"TMPDIR={work_dir / 'profiles' / 'self-test.target.tmp'}" not in nsys_argv:
            raise SystemExit("fake nsys did not wrap target TMPDIR")
        nsys_json = json.loads(
            (work_dir / "profiles" / "self-test.profile.json").read_text(
                encoding="utf-8",
            )
        )
        if nsys_json.get("status") != "ok" or nsys_json.get("profile_exit_code") != 0:
            raise SystemExit("fake nsys profile json did not record success")
        if nsys_json.get("tool") != "nsys" or nsys_json.get("name") != "self-test":
            raise SystemExit("fake nsys profile json did not record identity")
        if nsys_json.get("proof_timing_summary_written") is not False:
            raise SystemExit("fake nsys profile json recorded an unexpected timing summary")
        if nsys_json.get("tool_summaries") != []:
            raise SystemExit("fake nsys profile json recorded unexpected tool summaries")
        if not nsys_json.get("outputs", {}).get("profile_json", "").endswith(
            "self-test.profile.json"
        ):
            raise SystemExit("fake nsys profile json did not record its own path")

        ncu_base = dict(base)
        ncu_base["name"] = "self-test-ncu"
        ncu_base["summarize"] = True
        ncu_args = argparse.Namespace(
            **ncu_base,
            tool="ncu",
            nsys_command=None,
            ncu_command=str(fake_ncu),
        )
        if run_profile(ncu_args) != 0:
            raise SystemExit("fake ncu profile failed")
        if not (work_dir / "profiles" / "self-test-ncu.ncu.csv").exists():
            raise SystemExit("fake ncu csv missing")
        if not (work_dir / "profiles" / "self-test-ncu.ncu-rep").exists():
            raise SystemExit("fake ncu report missing")
        if not (work_dir / "profiles" / "self-test-ncu.ncu-kernel-summary.txt").exists():
            raise SystemExit("fake ncu summary missing")
        ncu_profile_log = work_dir / "profiles" / "self-test-ncu.profile.log"
        if "timing_total_ms=1000" not in ncu_profile_log.read_text(encoding="utf-8"):
            raise SystemExit("fake ncu profile log missing target output")
        proof_summary = work_dir / "profiles" / "self-test-ncu.proof-timing-summary.csv"
        if not proof_summary.exists():
            raise SystemExit("fake ncu proof timing summary missing")
        proof_summary_text = proof_summary.read_text(encoding="utf-8")
        if not proof_summary_text.startswith("profile,"):
            raise SystemExit("fake ncu proof timing summary is not CSV")
        if csv_value(proof_summary_text, "ncu_top_kernel") != "self":
            raise SystemExit("fake ncu proof timing summary missing top kernel")
        if (
            csv_value(proof_summary_text, "ncu_metric_collection_hint")
            != "duration_only_missing_throughput"
        ):
            raise SystemExit("fake ncu proof timing summary missing NCU metric hint")
        ncu_json = json.loads(
            (work_dir / "profiles" / "self-test-ncu.profile.json").read_text(
                encoding="utf-8",
            )
        )
        if ncu_json.get("status") != "ok" or ncu_json.get("profile_exit_code") != 0:
            raise SystemExit("fake ncu profile json did not record success")
        if ncu_json.get("tool") != "ncu" or ncu_json.get("name") != "self-test-ncu":
            raise SystemExit("fake ncu profile json did not record identity")
        if ncu_json.get("proof_timing_summary_written") is not True:
            raise SystemExit("fake ncu profile json did not record the timing summary")
        if not any(
            path.endswith("self-test-ncu.ncu-kernel-summary.txt")
            for path in ncu_json.get("tool_summaries", [])
        ):
            raise SystemExit("fake ncu profile json missing kernel summary path")
        ncu_argv = (work_dir / "fake-ncu.argv").read_text(encoding="utf-8")
        if f"tmpdir={work_dir / 'profiles' / 'self-test-ncu.tmp'}" not in ncu_argv:
            raise SystemExit("fake ncu did not receive managed TMPDIR")
        if f"TMPDIR={work_dir / 'profiles' / 'self-test-ncu.target.tmp'}" not in ncu_argv:
            raise SystemExit("fake ncu did not wrap target TMPDIR")

        missing_base = dict(base)
        missing_base["command"] = [
            sys.executable,
            "-c",
            "print('timing_total_ms=1000')",
        ]
        missing_base["name"] = "self-test-ncu-missing-keys"
        missing_base["summarize"] = True
        stale_summary = (
            work_dir
            / "profiles"
            / "self-test-ncu-missing-keys.proof-timing-summary.csv"
        )
        stale_stderr = prefixed_path(stale_summary.parent, stale_summary.name, ".stderr")
        stale_summary.write_text("stale\n", encoding="utf-8")
        stale_stderr.write_text("stale\n", encoding="utf-8")
        missing_args = argparse.Namespace(
            **missing_base,
            tool="ncu",
            nsys_command=None,
            ncu_command=str(fake_ncu),
        )
        if run_profile(missing_args) != 0:
            raise SystemExit("fake ncu missing-key profile failed")
        if stale_summary.exists() or stale_stderr.exists():
            raise SystemExit("fake ncu missing-key proof timing summary was not removed")
        missing_json = json.loads(
            (
                work_dir
                / "profiles"
                / "self-test-ncu-missing-keys.profile.json"
            ).read_text(encoding="utf-8")
        )
        if missing_json.get("status") != "ok":
            raise SystemExit("fake ncu missing-key profile json did not record success")
        if missing_json.get("proof_timing_summary_written") is not False:
            raise SystemExit("fake ncu missing-key profile json recorded a summary")
        if missing_json.get("proof_timing_summary_skip_reason") != "missing_keys":
            raise SystemExit("fake ncu missing-key profile json missed the skip reason")
        missing_keys = missing_json.get("proof_timing_summary_missing_keys", [])
        if "timing_finish_witness_opening_row_dedup_input_rows" not in missing_keys:
            raise SystemExit("fake ncu missing-key profile json missed required keys")

        duplicate_base = dict(base)
        duplicate_base["command"] = [
            sys.executable,
            "-c",
            (
                "print('timing_total_ms=1000'); "
                "print('timing_total_ms=1001'); "
                "print('timing_guest_stage_tree_commit_root_count=1'); "
                "print('timing_guest_stage_tree_commit_root_materialization_groups=1'); "
                "print('timing_guest_stage_tree_commit_root_materialization_max_group_size=1'); "
                "print('timing_finish_witness_opening_row_dedup_input_rows=0'); "
                "print('timing_finish_witness_opening_row_dedup_unique_rows=0'); "
                "print('timing_finish_witness_opening_row_dedup_elided_rows=0'); "
                "print('timing_finish_fri_opening_ms=10'); "
                "print('timing_finish_fri_opening_unit_build_ms=8'); "
                "print('timing_finish_fri_opening_layer_tree_ms=2'); "
                "print('timing_finish_fri_opening_query_ms=3'); "
                "print('timing_finish_fri_opening_fold_ms=1'); "
                "print('timing_finish_fri_opening_unit_count=1'); "
                "print('timing_finish_fri_opening_layer_count=2'); "
                "print('timing_finish_fri_opening_query_count=3'); "
                "print('timing_finish_fri_transcript_unit_build_ms=4'); "
                "print('timing_finish_fri_transcript_layer_tree_ms=2'); "
                "print('timing_finish_fri_transcript_fold_ms=1'); "
                "print('timing_finish_fri_transcript_unit_count=1'); "
                "print('timing_finish_fri_transcript_layer_count=2'); "
                "print('timing_finish_contribution_segment_ms=5'); "
                "print('timing_finish_contribution_verify_ms=6'); "
                "print('timing_finish_contribution_challenge_ms=7')"
            ),
        ]
        duplicate_base["name"] = "self-test-ncu-duplicate-timing"
        duplicate_base["summarize"] = True
        duplicate_summary = (
            work_dir
            / "profiles"
            / "self-test-ncu-duplicate-timing.proof-timing-summary.csv"
        )
        duplicate_summary_stderr = prefixed_path(
            duplicate_summary.parent,
            duplicate_summary.name,
            ".stderr",
        )
        duplicate_summary.write_text("stale\n", encoding="utf-8")
        duplicate_summary_stderr.write_text("stale\n", encoding="utf-8")
        duplicate_args = argparse.Namespace(
            **duplicate_base,
            tool="ncu",
            nsys_command=None,
            ncu_command=str(fake_ncu),
        )
        try:
            run_profile(duplicate_args)
        except SystemExit:
            pass
        else:
            raise SystemExit("fake ncu duplicate timing summary unexpectedly passed")
        if duplicate_summary.exists() or duplicate_summary_stderr.exists():
            raise SystemExit("fake ncu duplicate proof timing summary was not removed")
        duplicate_json = json.loads(
            (
                work_dir
                / "profiles"
                / "self-test-ncu-duplicate-timing.profile.json"
            ).read_text(encoding="utf-8")
        )
        if duplicate_json.get("status") != "summary_failed":
            raise SystemExit("fake ncu duplicate profile json did not record failure")
        if duplicate_json.get("proof_timing_summary_written") is not False:
            raise SystemExit("fake ncu duplicate profile json recorded a summary")
        if "duplicate timing field" not in duplicate_json.get("error", ""):
            raise SystemExit("fake ncu duplicate profile json did not record the error")

        failed_summary_base = dict(base)
        failed_summary_base["name"] = "self-test-ncu-summary-failure"
        failed_summary_base["summarize"] = True
        failed_summary = (
            work_dir
            / "profiles"
            / "self-test-ncu-summary-failure.proof-timing-summary.csv"
        )
        failed_summary_stderr = prefixed_path(
            failed_summary.parent, failed_summary.name, ".stderr"
        )
        failed_args = argparse.Namespace(
            **failed_summary_base,
            tool="ncu",
            nsys_command=None,
            ncu_command=str(fake_bad_ncu),
        )
        try:
            run_profile(failed_args)
        except SystemExit:
            pass
        else:
            raise SystemExit("fake ncu summary failure unexpectedly passed")
        if failed_summary.exists() or failed_summary_stderr.exists():
            raise SystemExit("fake ncu failed proof timing summary was not removed")
        failed_json = json.loads(
            (
                work_dir
                / "profiles"
                / "self-test-ncu-summary-failure.profile.json"
            ).read_text(encoding="utf-8")
        )
        if failed_json.get("status") != "summary_failed":
            raise SystemExit("fake ncu failure profile json did not record failure")
        if "summary command failed" not in failed_json.get("error", ""):
            raise SystemExit("fake ncu failure profile json did not record the error")

        failed_profile_base = dict(base)
        failed_profile_base["command"] = [sys.executable, "-c", "raise SystemExit(7)"]
        failed_profile_base["name"] = "self-test-profile-failure"
        failed_profile_args = argparse.Namespace(
            **failed_profile_base,
            tool="ncu",
            nsys_command=None,
            ncu_command=str(fake_ncu),
        )
        if run_profile(failed_profile_args) != 7:
            raise SystemExit("fake profile failure did not return the target status")
        failed_profile_json = json.loads(
            (
                work_dir
                / "profiles"
                / "self-test-profile-failure.profile.json"
            ).read_text(encoding="utf-8")
        )
        if (
            failed_profile_json.get("status") != "profile_failed"
            or failed_profile_json.get("profile_exit_code") != 7
        ):
            raise SystemExit("fake profile failure json did not record the exit status")
    finally:
        shutil.rmtree(work_dir, ignore_errors=True)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Capture a proof command with Nsight profiling into workspace temp."
    )
    parser.add_argument("--tool", choices=["nsys", "ncu"], default="nsys")
    parser.add_argument("--output-dir", default="temp/proof-profiles")
    parser.add_argument("--name", type=profile_name, default=None)
    parser.add_argument("--cwd", default=".")
    parser.add_argument("--nsys-command")
    parser.add_argument("--ncu-command")
    parser.add_argument("--nsys-trace", default="cuda,nvtx,osrt")
    parser.add_argument("--ncu-set", default="basic")
    parser.add_argument("--ncu-target-processes", default="all")
    parser.add_argument("--profile-arg", action="append", default=[])
    parser.add_argument("--skip-nsys-export", action="store_true")
    parser.add_argument("--summarize", action="store_true")
    parser.add_argument("--proof-timing-summary", action="store_true")
    parser.add_argument("--require-proof-timing-summary", action="store_true")
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("--check-tool", action="store_true")
    parser.add_argument("--check-gpu-memory", action="store_true")
    parser.add_argument("--min-gpu-free-mib", type=positive_mib, default=DEFAULT_MIN_GPU_FREE_MIB)
    parser.add_argument(
        "--gpu-memory-wait-timeout-s",
        type=nonnegative_float,
        default=DEFAULT_GPU_MEMORY_WAIT_TIMEOUT_S,
    )
    parser.add_argument(
        "--gpu-memory-wait-poll-s",
        type=positive_timeout,
        default=DEFAULT_GPU_MEMORY_WAIT_POLL_S,
    )
    parser.add_argument("--nvidia-smi-command")
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    args.name = profile_name(args.name)
    return args


def main() -> int:
    args = parse_args()
    if not args.check_gpu_memory and gpu_memory_wait_requested(args):
        raise SystemExit("--gpu-memory-wait-* requires --check-gpu-memory")
    if args.self_test:
        self_test()
        return 0
    if args.check_tool:
        tool_ready = check_profile_tool(args) == 0
        gpu_ready = True
        if args.check_gpu_memory:
            gpu_ready = print_gpu_memory_check(args, workspace_root())
        return 0 if tool_ready and gpu_ready else 1
    if args.check_gpu_memory and not args.command:
        return 0 if print_gpu_memory_check(args, workspace_root()) else 1
    return run_profile(args)


if __name__ == "__main__":
    sys.exit(main())
