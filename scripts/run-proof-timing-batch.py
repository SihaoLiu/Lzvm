#!/usr/bin/env python3
import argparse
import json
import os
import re
import shlex
import shutil
import signal
import subprocess
import sys
import time
from datetime import datetime
from pathlib import Path

TIMING_TOTAL_RE = re.compile(r"^timing_total_ms=(\d+)\s*$", re.MULTILINE)


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


def positive_run_count(raw: str) -> int:
    try:
        value = int(raw)
    except ValueError as error:
        raise argparse.ArgumentTypeError(f"invalid run count: {raw!r}") from error
    if value < 3:
        raise argparse.ArgumentTypeError("run count must be at least 3")
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


def read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8", errors="replace")


def write_combined_log(path: Path, stdout: str, stderr: str) -> None:
    combined = ["[stdout]\n", stdout]
    if stdout and not stdout.endswith("\n"):
        combined.append("\n")
    combined.extend(["[stderr]\n", stderr])
    if stderr and not stderr.endswith("\n"):
        combined.append("\n")
    path.write_text("".join(combined), encoding="utf-8")


def timing_total_ms_from_text(text: str, path: Path) -> int:
    matches = [int(match.group(1)) for match in TIMING_TOTAL_RE.finditer(text)]
    if len(matches) != 1:
        raise SystemExit(
            f"{path}: expected exactly one timing_total_ms line, found {len(matches)}"
        )
    if matches[0] <= 0:
        raise SystemExit(f"{path}: timing_total_ms must be positive")
    return matches[0]


def require_texts_in_log(text: str, path: Path, required_texts: list[str]) -> None:
    for required in required_texts:
        if required not in text:
            raise SystemExit(f"{path}: missing required text {required!r}")


def stop_process_group(process: subprocess.Popen[str]) -> None:
    if process.poll() is not None:
        return
    try:
        if hasattr(os, "killpg"):
            os.killpg(process.pid, signal.SIGKILL)
        else:
            process.kill()
    except ProcessLookupError:
        pass


def expand_command_template(
    command: str,
    label: str,
    run_index: int,
    run_count: int,
    batch_dir: Path,
    cwd: Path,
) -> str:
    replacements = {
        "{label}": label,
        "{run}": str(run_index),
        "{run_padded}": f"{run_index:03d}",
        "{runs}": str(run_count),
        "{batch_dir}": shlex.quote(str(batch_dir)),
        "{cwd}": shlex.quote(str(cwd)),
    }
    expanded = command
    for needle, value in replacements.items():
        expanded = expanded.replace(needle, value)
    return expanded


def run_once(
    label: str,
    command_template: str,
    run_index: int,
    run_count: int,
    timeout: float,
    batch_dir: Path,
    cwd: Path,
    required_texts: list[str],
) -> Path:
    command = expand_command_template(
        command_template,
        label,
        run_index,
        run_count,
        batch_dir,
        cwd,
    )
    stem = f"{label}-{run_index:03d}"
    stdout_path = batch_dir / f"{stem}.stdout"
    stderr_path = batch_dir / f"{stem}.stderr"
    combined_path = batch_dir / f"{stem}.log"
    status_path = batch_dir / f"{stem}.status"
    env = os.environ.copy()
    env["LZVM_TIMING_BATCH_LABEL"] = label
    env["LZVM_TIMING_BATCH_RUN"] = str(run_index)
    env["LZVM_TIMING_BATCH_RUNS"] = str(run_count)

    start = time.monotonic()
    timed_out = False
    with stdout_path.open("w", encoding="utf-8") as stdout_file:
        with stderr_path.open("w", encoding="utf-8") as stderr_file:
            process = subprocess.Popen(
                command,
                cwd=cwd,
                env=env,
                shell=True,
                start_new_session=True,
                stdout=stdout_file,
                stderr=stderr_file,
                text=True,
            )
            try:
                exit_code = process.wait(timeout=timeout)
            except subprocess.TimeoutExpired:
                timed_out = True
                stop_process_group(process)
                exit_code = process.wait()
    elapsed_s = time.monotonic() - start
    stdout = read_text(stdout_path)
    stderr = read_text(stderr_path)
    write_combined_log(combined_path, stdout, stderr)

    status_lines = [
        f"label={label}",
        f"run={run_index}",
        f"command={command}",
        f"cwd={cwd}",
        f"elapsed_s={elapsed_s:.3f}",
        f"timeout_s={timeout:.3f}",
        f"exit_code={exit_code}",
        f"timed_out={str(timed_out).lower()}",
        f"combined_log={combined_path}",
    ]
    if timed_out:
        status_path.write_text("\n".join(status_lines) + "\n", encoding="utf-8")
        raise SystemExit(
            f"{label} run {run_index} timed out after {timeout:.3f}s; log: {combined_path}"
        )
    if exit_code != 0:
        status_path.write_text("\n".join(status_lines) + "\n", encoding="utf-8")
        raise SystemExit(
            f"{label} run {run_index} exited with status {exit_code}; log: {combined_path}"
        )

    combined_text = read_text(combined_path)
    require_texts_in_log(combined_text, combined_path, required_texts)
    total_ms = timing_total_ms_from_text(combined_text, combined_path)
    status_lines.append(f"timing_total_ms={total_ms}")
    status_path.write_text("\n".join(status_lines) + "\n", encoding="utf-8")
    return combined_path


def run_group(
    label: str,
    command: str | None,
    run_count: int,
    timeout: float,
    batch_dir: Path,
    cwd: Path,
    required_texts: list[str],
) -> list[Path]:
    if command is None:
        return []
    logs = []
    for run_index in range(1, run_count + 1):
        logs.append(
            run_once(
                label,
                command,
                run_index,
                run_count,
                timeout,
                batch_dir,
                cwd,
                required_texts,
            )
        )
    return logs


def append_improve_log(
    append_script: Path,
    improve_log_path: Path,
    commit: str | None,
    summary: str,
    small_logs: list[Path],
    large_logs: list[Path],
    max_relative_spread: float,
    root: Path,
    batch_dir: Path,
) -> None:
    command = [
        sys.executable,
        str(append_script),
        "--path",
        str(improve_log_path),
        "--summary",
        summary,
        "--max-relative-spread",
        str(max_relative_spread),
    ]
    if commit is not None:
        command.extend(["--commit", commit])
    for path in small_logs:
        command.extend(["--small-log", str(path)])
    for path in large_logs:
        command.extend(["--large-log", str(path)])

    output = subprocess.run(
        command,
        cwd=root,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    (batch_dir / "append.stdout").write_text(output.stdout, encoding="utf-8")
    (batch_dir / "append.stderr").write_text(output.stderr, encoding="utf-8")
    if output.returncode != 0:
        raise SystemExit(
            "append-improve-log failed with status "
            f"{output.returncode}; stderr: {output.stderr.strip()}"
        )


def path_texts(paths: list[Path]) -> list[str]:
    return [str(path) for path in paths]


def write_batch_json(
    path: Path,
    args: argparse.Namespace,
    root: Path,
    batch_dir: Path,
    cwd: Path,
    improve_log_path: Path,
    small_logs: list[Path] | None,
    large_logs: list[Path] | None,
    appended: bool,
) -> None:
    payload = {
        "created_at": datetime.now().astimezone().strftime("%Y-%m-%dT%H:%M:%S%z"),
        "workspace": str(root),
        "batch_dir": str(batch_dir),
        "cwd": str(cwd),
        "improve_log": str(improve_log_path),
        "appended": appended,
        "runs": args.runs,
        "small_timeout_s": args.small_timeout,
        "large_timeout_s": args.large_timeout,
        "max_relative_spread": args.max_relative_spread,
        "commit": args.commit,
        "summary": args.summary,
        "small_command": args.small_command,
        "large_command": args.large_command,
        "require_text": list(args.require_text or []),
        "small_require_text": list(args.small_require_text or []),
        "large_require_text": list(args.large_require_text or []),
        "require_proof_output": args.require_proof_output,
        "small_logs": path_texts(small_logs or []),
        "large_logs": path_texts(large_logs or []),
    }
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def batch_dir_name() -> str:
    timestamp = datetime.now().astimezone().strftime("%Y%m%dT%H%M%S%z")
    return f"{timestamp}-{os.getpid()}"


def required_texts_for_label(args: argparse.Namespace, label: str) -> list[str]:
    required = list(args.require_text or [])
    if args.require_proof_output:
        required.extend(["status=ok", "verify_outputs=true"])
    if label == "small":
        required.extend(args.small_require_text or [])
    elif label == "large":
        required.extend(args.large_require_text or [])
    return required


def run_batch(args: argparse.Namespace) -> Path:
    root = workspace_root()
    if args.small_command is None and args.large_command is None:
        raise SystemExit("provide --small-command and/or --large-command")
    if args.summary is None:
        raise SystemExit("--summary is required")

    append_script = resolve_workspace_path(args.append_script, root)
    if not append_script.exists():
        raise SystemExit(f"{append_script}: append script does not exist")
    work_dir = require_workspace_temp_path(
        resolve_workspace_path(args.work_dir, root),
        root,
        "--work-dir",
    )
    cwd = resolve_workspace_path(args.cwd, root)
    if not cwd.exists():
        raise SystemExit(f"{cwd}: command working directory does not exist")
    improve_log_path = require_workspace_temp_path(
        resolve_workspace_path(args.path, root),
        root,
        "--path",
    )
    batch_dir = work_dir / batch_dir_name()
    batch_dir.mkdir(parents=True, exist_ok=False)
    batch_json_path = batch_dir / "batch.json"

    def record_batch_json(
        small_logs: list[Path] | None = None,
        large_logs: list[Path] | None = None,
        appended: bool = False,
    ) -> None:
        write_batch_json(
            batch_json_path,
            args,
            root,
            batch_dir,
            cwd,
            improve_log_path,
            small_logs,
            large_logs,
            appended,
        )

    record_batch_json()

    small_logs = run_group(
        "small",
        args.small_command,
        args.runs,
        args.small_timeout,
        batch_dir,
        cwd,
        required_texts_for_label(args, "small"),
    )
    large_logs = run_group(
        "large",
        args.large_command,
        args.runs,
        args.large_timeout,
        batch_dir,
        cwd,
        required_texts_for_label(args, "large"),
    )
    record_batch_json(small_logs, large_logs, appended=False)
    append_improve_log(
        append_script,
        improve_log_path,
        args.commit,
        args.summary,
        small_logs,
        large_logs,
        args.max_relative_spread,
        root,
        batch_dir,
    )
    record_batch_json(small_logs, large_logs, appended=True)

    print(f"batch_dir={batch_dir}")
    print(f"batch_json={batch_json_path}")
    if small_logs:
        print(f"small_runs={len(small_logs)}")
    if large_logs:
        print(f"large_runs={len(large_logs)}")
    print(f"improve_log={improve_log_path}")
    return batch_dir


def self_test() -> None:
    root = workspace_root()
    work_dir = root / "temp" / f"proof-timing-batch-self-test-{os.getpid()}"
    shutil.rmtree(work_dir, ignore_errors=True)
    command = shlex.join([sys.executable, "-c", "print('timing_total_ms=1000')"])
    args = argparse.Namespace(
        append_script="scripts/append-improve-log.py",
        commit="selftest",
        cwd=".",
        large_command=command,
        large_timeout=10.0,
        max_relative_spread=0.10,
        path=str(work_dir / "improve-log.csv"),
        require_proof_output=False,
        require_text=[],
        runs=3,
        small_command=command,
        small_require_text=[],
        small_timeout=10.0,
        summary="self test",
        work_dir=str(work_dir / "runs"),
        large_require_text=[],
    )
    try:
        run_batch(args)
        contents = (work_dir / "improve-log.csv").read_text(encoding="utf-8")
        expected = '"avg=1.000 samples=1.000;1.000;1.000 used=3/3"'
        if expected not in contents:
            raise SystemExit("self-test improve log did not contain the expected average")
    finally:
        shutil.rmtree(work_dir, ignore_errors=True)


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Run proof timing commands repeatedly and append stable CSV fields."
    )
    parser.add_argument("--small-command")
    parser.add_argument("--large-command")
    parser.add_argument("--runs", type=positive_run_count, default=3)
    parser.add_argument("--small-timeout", type=positive_timeout, default=60.0)
    parser.add_argument("--large-timeout", type=positive_timeout, default=180.0)
    parser.add_argument("--work-dir", default="temp/proof-timing-batch")
    parser.add_argument("--cwd", default=".")
    parser.add_argument("--path", default="temp/improve-log.csv")
    parser.add_argument("--commit", default=None)
    parser.add_argument("--summary")
    parser.add_argument("--max-relative-spread", type=nonnegative_float, default=0.10)
    parser.add_argument("--append-script", default="scripts/append-improve-log.py")
    parser.add_argument("--require-text", action="append", default=[])
    parser.add_argument("--small-require-text", action="append", default=[])
    parser.add_argument("--large-require-text", action="append", default=[])
    parser.add_argument("--require-proof-output", action="store_true")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()

    if args.self_test:
        self_test()
    else:
        run_batch(args)


if __name__ == "__main__":
    main()
