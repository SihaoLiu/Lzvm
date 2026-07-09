#!/usr/bin/env python3
import argparse
import importlib.util
import json
import math
import os
import re
import shlex
import shutil
import signal
import stat
import subprocess
import sys
import time
from datetime import datetime
from pathlib import Path


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

TIMING_TOTAL_RE = re.compile(r"^timing_total_ms=(\d+)\s*$", re.MULTILINE)
INHERITED_RUNTIME_ENV_NAMES = (
    "LZVM_CUDA_RETAINED_SOURCE_BYTES",
    "LZVM_CUDA_RETAIN_FRI_STAGE_SOURCES",
    "LZVM_CUDA_FRI_STAGE_SOURCE_DEBUG",
    "LZVM_CUDA_RETAINED_DESCRIPTOR_BYTES",
    "LZVM_CUDA_RETAINED_LEAF_DIGEST_BYTES",
    "LZVM_CUDA_RETAINED_PARENT_CHECKPOINT_BYTES",
    "LZVM_CUDA_RETAINED_PARENT_CHECKPOINT_MAX_STATES",
    "LZVM_GUEST_PC_TRACE_DESCRIPTOR_HIGH32_STATS",
    "LZVM_CUDA_GUEST_PC_SPARSE_HIGH32_DESCRIPTORS",
    "LZVM_CUDA_GUEST_PC_DESCRIPTOR_STREAM_INGRESS",
)


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


def current_commit(root: Path) -> str:
    result = subprocess.run(
        ["git", "rev-parse", "--short=8", "HEAD"],
        cwd=root,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if result.returncode != 0:
        raise SystemExit(
            "failed to resolve current git commit: " + result.stderr.strip()
        )
    return result.stdout.strip()


def tracked_worktree_status(root: Path) -> list[str]:
    result = subprocess.run(
        ["git", "status", "--short", "--untracked-files=no"],
        cwd=root,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
    )
    if result.returncode != 0:
        return []
    return [line for line in result.stdout.splitlines() if line]


def host_load_average() -> list[float] | None:
    try:
        return [round(value, 3) for value in os.getloadavg()]
    except (AttributeError, OSError):
        return None


def host_gpu_status() -> list[dict[str, int]] | None:
    try:
        result = subprocess.run(
            [
                "nvidia-smi",
                "--query-gpu=index,memory.total,memory.used,utilization.gpu",
                "--format=csv,noheader,nounits",
            ],
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            timeout=2.0,
        )
    except (OSError, subprocess.TimeoutExpired):
        return None
    if result.returncode != 0:
        return None
    devices = []
    for line in result.stdout.splitlines():
        parts = [part.strip() for part in line.split(",")]
        if len(parts) != 4:
            return None
        try:
            index, total_mib, used_mib, utilization_pct = map(int, parts)
        except ValueError:
            return None
        devices.append(
            {
                "index": index,
                "memory_total_mib": total_mib,
                "memory_used_mib": used_mib,
                "utilization_gpu_pct": utilization_pct,
            }
        )
    return devices or None


def positive_run_count(raw: str) -> int:
    try:
        value = int(raw)
    except ValueError as error:
        raise argparse.ArgumentTypeError(f"invalid run count: {raw!r}") from error
    if value < 3:
        raise argparse.ArgumentTypeError("run count must be at least 3")
    return value


def positive_timeout(raw: str) -> float:
    value = finite_float(raw, "timeout")
    if value <= 0.0:
        raise argparse.ArgumentTypeError("timeout must be positive")
    return value


def finite_float(raw: str, label: str) -> float:
    try:
        value = float(raw)
    except ValueError as error:
        raise argparse.ArgumentTypeError(f"invalid {label}: {raw!r}") from error
    if not math.isfinite(value):
        raise argparse.ArgumentTypeError(f"{label} must be finite")
    return value


def nonnegative_float(raw: str) -> float:
    value = finite_float(raw, "float")
    if value < 0.0:
        raise argparse.ArgumentTypeError("value must be nonnegative")
    return value


def open_text_no_follow(
    path: Path,
    mode: int = 0o600,
    *,
    readback: bool = False,
    errors: str | None = None,
):
    flags = (os.O_RDWR if readback else os.O_WRONLY) | os.O_CREAT | os.O_TRUNC
    if hasattr(os, "O_NOFOLLOW"):
        flags |= os.O_NOFOLLOW
    try:
        descriptor = os.open(path, flags, mode)
    except OSError as error:
        if path.is_symlink():
            raise SystemExit(f"output path must not be a symlink: {path}") from error
        raise
    if readback:
        return os.fdopen(
            descriptor,
            "w+",
            encoding="utf-8",
            errors=errors or "strict",
        )
    return os.fdopen(descriptor, "w", encoding="utf-8")


def open_read_text_no_follow(path: Path, label: str = "input path"):
    flags = os.O_RDONLY
    if hasattr(os, "O_NOFOLLOW"):
        flags |= os.O_NOFOLLOW
    try:
        descriptor = os.open(path, flags)
    except OSError as error:
        if path.is_symlink():
            raise SystemExit(f"{label} must not be a symlink: {path}") from error
        raise
    return os.fdopen(descriptor, "r", encoding="utf-8", errors="replace")


def read_text(path: Path) -> str:
    with open_read_text_no_follow(path) as source:
        return source.read()


def write_text_no_follow(path: Path, text: str) -> None:
    with open_text_no_follow(path) as output:
        output.write(text)


def write_combined_log(path: Path, stdout: str, stderr: str) -> None:
    combined = ["[stdout]\n", stdout]
    if stdout and not stdout.endswith("\n"):
        combined.append("\n")
    combined.extend(["[stderr]\n", stderr])
    if stderr and not stderr.endswith("\n"):
        combined.append("\n")
    write_text_no_follow(path, "".join(combined))


def inherited_runtime_env() -> dict[str, str]:
    values = {}
    for name in INHERITED_RUNTIME_ENV_NAMES:
        value = os.environ.get(name)
        if value is not None:
            values[name] = value
    return values


def first_diagnostic_line(text: str) -> str:
    lines = [line.strip() for line in text.splitlines() if line.strip()]
    return lines[0][:200] if lines else ""


def remove_summary_outputs(output_path: Path) -> None:
    stderr_path = prefixed_path(output_path.parent, output_path.name, ".stderr")
    for path in [output_path, stderr_path]:
        try:
            path.unlink()
        except FileNotFoundError:
            pass


def write_status(path: Path, lines: list[str]) -> None:
    write_text_no_follow(path, "\n".join(lines) + "\n")


def path_is_regular_file_no_follow(path: Path) -> bool:
    try:
        return stat.S_ISREG(path.lstat().st_mode)
    except FileNotFoundError:
        return False


def prepare_run_tmp_dir(path: Path) -> None:
    if path.exists() or path.is_symlink():
        raise SystemExit(f"run tmp dir must not already exist: {path}")
    path.mkdir(mode=0o700)


def line_key_present(text: str, key: str) -> bool:
    return any(line.startswith(f"{key}=") for line in text.splitlines())


def missing_timing_summary_keys(text: str) -> list[str]:
    return [key for key in TIMING_SUMMARY_REQUIRED_KEYS if not line_key_present(text, key)]


def timing_total_ms_from_text(text: str, path: Path) -> int:
    matches = [int(match.group(1)) for match in TIMING_TOTAL_RE.finditer(text)]
    if len(matches) != 1:
        raise SystemExit(
            f"{path}: expected exactly one timing_total_ms line, found {len(matches)}"
        )
    return matches[0]


def timing_total_seconds_from_log(path: Path) -> float:
    return timing_total_ms_from_text(read_text(path), path) / 1000.0


def stable_sample_values(
    samples: list[float],
    max_relative_spread: float,
    min_stable_count: int = 3,
) -> list[float] | None:
    if len(samples) < min_stable_count:
        return None

    ordered = sorted(samples)
    window = stable_sample_window(ordered, max_relative_spread, min_stable_count)
    if window is None:
        return None
    start, end = window
    return ordered[start:end]


def stable_sample_window(
    ordered_samples: list[float],
    max_relative_spread: float,
    min_stable_count: int = 3,
) -> tuple[int, int] | None:
    best: list[float] | None = None
    best_window: tuple[int, int] | None = None
    for start in range(len(ordered_samples)):
        for end in range(start + min_stable_count, len(ordered_samples) + 1):
            candidate = ordered_samples[start:end]
            median = candidate[len(candidate) // 2]
            if median == 0.0:
                relative_spread = 0.0 if candidate[0] == candidate[-1] else math.inf
            else:
                relative_spread = (candidate[-1] - candidate[0]) / median
            if relative_spread <= max_relative_spread:
                if best is None or len(candidate) > len(best):
                    best = candidate
                    best_window = (start, end)
                elif best is not None and len(candidate) == len(best):
                    best_median = best[len(best) // 2]
                    best_spread = (
                        0.0
                        if best_median == 0.0 and best[0] == best[-1]
                        else (
                            math.inf
                            if best_median == 0.0
                            else (best[-1] - best[0]) / best_median
                        )
                    )
                    if relative_spread < best_spread:
                        best = candidate
                        best_window = (start, end)
    return best_window


def stable_timing_group(
    logs: list[Path],
    max_relative_spread: float,
    min_stable_count: int = 3,
) -> list[Path] | None:
    if len(logs) < min_stable_count:
        return None
    samples = [(timing_total_seconds_from_log(path), path) for path in logs]
    ordered = sorted(samples, key=lambda sample: sample[0])
    window = stable_sample_window(
        [seconds for seconds, _path in ordered],
        max_relative_spread,
        min_stable_count,
    )
    if window is None:
        return None
    start, end = window
    selected = {path for _seconds, path in ordered[start:end]}
    return [path for path in logs if path in selected]


def parseable_timing_samples(logs: list[Path]) -> tuple[list[tuple[float, Path]], int]:
    samples = []
    failed = 0
    for path in logs:
        try:
            samples.append((timing_total_seconds_from_log(path), path))
        except (OSError, SystemExit):
            failed += 1
    return samples, failed


def stable_parseable_timing_group(
    logs: list[Path],
    max_relative_spread: float,
    min_stable_count: int = 3,
) -> list[Path] | None:
    samples, _failed = parseable_timing_samples(logs)
    if len(samples) < min_stable_count:
        return None
    ordered = sorted(samples, key=lambda sample: sample[0])
    window = stable_sample_window(
        [seconds for seconds, _path in ordered],
        max_relative_spread,
        min_stable_count,
    )
    if window is None:
        return None
    start, end = window
    selected = {path for _seconds, path in ordered[start:end]}
    return [path for path in logs if path in selected]


def has_stable_timing_group(
    logs: list[Path],
    max_relative_spread: float,
    min_stable_count: int = 3,
) -> bool:
    return (
        stable_parseable_timing_group(logs, max_relative_spread, min_stable_count)
        is not None
    )



def append_artifact_paths(batch_dir: Path) -> tuple[Path, Path, Path]:
    return (
        batch_dir / "append.stdout",
        batch_dir / "append.stderr",
        batch_dir / "append.status",
    )


def prepare_append_artifact_paths(batch_dir: Path) -> tuple[Path, Path, Path]:
    paths = append_artifact_paths(batch_dir)
    for path in paths:
        if path.exists() or path.is_symlink():
            raise SystemExit(f"append artifact path must not already exist: {path}")
    for path in paths:
        with open_text_no_follow(path):
            pass
    return paths


def require_texts_in_log(text: str, path: Path, required_texts: list[str]) -> None:
    for required in required_texts:
        if required not in text:
            raise SystemExit(f"{path}: missing required text {required!r}")


def retryable_run_failure_reason(text: str) -> str | None:
    if "GPU memory preflight failed" in text:
        return "gpu_memory_preflight"
    if "cuda backend out of memory" in text:
        return "cuda_memory_exhausted"
    return None


def write_timing_summary(
    summary_script: Path,
    input_log: Path,
    output_path: Path,
    text: str,
    root: Path,
) -> str:
    missing_keys = missing_timing_summary_keys(text)
    if missing_keys:
        remove_summary_outputs(output_path)
        return "skipped_missing_keys=" + ";".join(missing_keys)
    stderr_path = prefixed_path(output_path.parent, output_path.name, ".stderr")
    result = subprocess.run(
        [sys.executable, str(summary_script), str(input_log)],
        cwd=root,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    write_text_no_follow(output_path, result.stdout)
    write_text_no_follow(stderr_path, result.stderr)
    if result.returncode != 0:
        diagnostic = first_diagnostic_line(result.stderr)
        diagnostic_suffix = f"; first stderr line: {diagnostic}" if diagnostic else ""
        remove_summary_outputs(output_path)
        raise SystemExit(
            "timing summary failed with status "
            f"{result.returncode}; stderr: {stderr_path}"
            f"{diagnostic_suffix}"
        )
    return str(output_path)


def write_group_timing_summary(
    summary_script: Path,
    logs: list[Path],
    output_path: Path,
    root: Path,
) -> Path | None:
    if not logs:
        return None
    for path in logs:
        if missing_timing_summary_keys(read_text(path)):
            return None
    stderr_path = prefixed_path(output_path.parent, output_path.name, ".stderr")
    result = subprocess.run(
        [sys.executable, str(summary_script), *[str(path) for path in logs]],
        cwd=root,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    write_text_no_follow(output_path, result.stdout)
    write_text_no_follow(stderr_path, result.stderr)
    if result.returncode != 0:
        diagnostic = first_diagnostic_line(result.stderr)
        diagnostic_suffix = f"; first stderr line: {diagnostic}" if diagnostic else ""
        remove_summary_outputs(output_path)
        raise SystemExit(
            "group timing summary failed with status "
            f"{result.returncode}; stderr: {stderr_path}"
            f"{diagnostic_suffix}"
        )
    return output_path


def prefixed_path(output_dir: Path, name: str, suffix: str) -> Path:
    return Path(str(output_dir / name) + suffix)


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
    max_run_count: int,
    batch_dir: Path,
    tmp_dir: Path,
    cwd: Path,
) -> str:
    replacements = {
        "{label}": label,
        "{run}": str(run_index),
        "{run_padded}": f"{run_index:03d}",
        "{runs}": str(run_count),
        "{max_runs}": str(max_run_count),
        "{batch_dir}": shlex.quote(str(batch_dir)),
        "{tmp_dir}": shlex.quote(str(tmp_dir)),
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
    max_run_count: int,
    timeout: float,
    batch_dir: Path,
    cwd: Path,
    root: Path,
    timing_summary_script: Path,
    required_texts: list[str],
) -> Path:
    stem = f"{label}-{run_index:03d}"
    tmp_dir = batch_dir / f"{stem}.tmp"
    command = expand_command_template(
        command_template,
        label,
        run_index,
        run_count,
        max_run_count,
        batch_dir,
        tmp_dir,
        cwd,
    )
    stdout_path = batch_dir / f"{stem}.stdout"
    stderr_path = batch_dir / f"{stem}.stderr"
    combined_path = batch_dir / f"{stem}.log"
    timing_summary_path = batch_dir / f"{stem}.proof-timing-summary.csv"
    status_path = batch_dir / f"{stem}.status"
    status_lines = [
        f"label={label}",
        f"run={run_index}",
        f"command={command}",
        f"cwd={cwd}",
        f"tmp_dir={tmp_dir}",
        f"timeout_s={timeout:.3f}",
    ]

    try:
        prepare_run_tmp_dir(tmp_dir)
    except SystemExit as error:
        status_lines.append(f"validation_error={error}")
        write_status(status_path, status_lines)
        raise

    env = os.environ.copy()
    env["LZVM_TIMING_BATCH_LABEL"] = label
    env["LZVM_TIMING_BATCH_RUN"] = str(run_index)
    env["LZVM_TIMING_BATCH_RUNS"] = str(run_count)
    env["LZVM_TIMING_BATCH_MAX_RUNS"] = str(max_run_count)
    env["TMPDIR"] = str(tmp_dir)

    start = time.monotonic()
    timed_out = False
    with open_text_no_follow(
        stdout_path,
        readback=True,
        errors="replace",
    ) as stdout_file:
        with open_text_no_follow(
            stderr_path,
            readback=True,
            errors="replace",
        ) as stderr_file:
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
            stdout_file.flush()
            stderr_file.flush()
            stdout_file.seek(0)
            stderr_file.seek(0)
            stdout = stdout_file.read()
            stderr = stderr_file.read()
    elapsed_s = time.monotonic() - start
    status_lines.extend(
        [
            f"elapsed_s={elapsed_s:.3f}",
            f"exit_code={exit_code}",
            f"timed_out={str(timed_out).lower()}",
            f"combined_log={combined_path}",
        ]
    )
    try:
        write_combined_log(combined_path, stdout, stderr)
    except (OSError, SystemExit) as error:
        status_lines.append(f"validation_error={error}")
        write_status(status_path, status_lines)
        raise

    if timed_out:
        write_status(status_path, status_lines)
        raise SystemExit(
            f"{label} run {run_index} timed out after {timeout:.3f}s; log: {combined_path}"
        )
    if exit_code != 0:
        retryable_failure = retryable_run_failure_reason(read_text(combined_path))
        if retryable_failure is not None:
            status_lines.append(f"retryable_failure={retryable_failure}")
            write_status(status_path, status_lines)
            return combined_path
        write_status(status_path, status_lines)
        raise SystemExit(
            f"{label} run {run_index} exited with status {exit_code}; log: {combined_path}"
        )

    combined_text = read_text(combined_path)
    try:
        require_texts_in_log(combined_text, combined_path, required_texts)
        total_ms = timing_total_ms_from_text(combined_text, combined_path)
        timing_summary = write_timing_summary(
            timing_summary_script,
            combined_path,
            timing_summary_path,
            combined_text,
            root,
        )
    except SystemExit as error:
        status_lines.append(f"validation_error={error}")
        write_status(status_path, status_lines)
        raise
    status_lines.append(f"timing_total_ms={total_ms}")
    status_lines.append(f"proof_timing_summary={timing_summary}")
    write_status(status_path, status_lines)
    return combined_path


def run_group(
    label: str,
    command: str | None,
    run_count: int,
    max_run_count: int,
    timeout: float,
    batch_dir: Path,
    cwd: Path,
    root: Path,
    timing_summary_script: Path,
    required_texts: list[str],
    max_relative_spread: float,
    retryable_failure_wait_s: float,
) -> list[Path]:
    if command is None:
        return []
    logs = []
    for run_index in range(1, max_run_count + 1):
        log = run_once(
            label,
            command,
            run_index,
            run_count,
            max_run_count,
            timeout,
            batch_dir,
            cwd,
            root,
            timing_summary_script,
            required_texts,
        )
        logs.append(log)
        if len(logs) >= run_count:
            if max_run_count == run_count:
                break
            if has_stable_timing_group(logs, max_relative_spread):
                break
        if retryable_failure_wait_s <= 0 or run_index >= max_run_count:
            continue
        retryable_failure = retryable_run_failure_reason(read_text(log))
        if retryable_failure is not None:
            print(
                f"{label}_retryable_failure_wait="
                f"run={run_index} reason={retryable_failure} "
                f"seconds={retryable_failure_wait_s:.3f}",
                flush=True,
            )
            time.sleep(retryable_failure_wait_s)
    return logs


def append_improve_log(
    append_script: Path,
    improve_log_path: Path,
    commit: str | None,
    summary: str,
    small_logs: list[Path],
    large_logs: list[Path],
    small_field: str | None,
    large_field: str | None,
    max_relative_spread: float,
    small_max_avg_s: float | None,
    large_max_avg_s: float | None,
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
    if small_max_avg_s is not None:
        command.extend(["--small-max-avg-s", str(small_max_avg_s)])
    if large_max_avg_s is not None:
        command.extend(["--large-max-avg-s", str(large_max_avg_s)])
    if small_field is not None:
        command.extend(["--small", small_field])
    else:
        small_parse_failed_field = stable_average_field_for_parse_failed_logs(
            small_logs,
            max_relative_spread,
        )
        if small_parse_failed_field is not None:
            command.extend(["--small", small_parse_failed_field])
        else:
            for path in small_logs:
                command.extend(["--small-log", str(path)])
    if large_field is not None:
        command.extend(["--large", large_field])
    else:
        large_parse_failed_field = stable_average_field_for_parse_failed_logs(
            large_logs,
            max_relative_spread,
        )
        if large_parse_failed_field is not None:
            command.extend(["--large", large_parse_failed_field])
        else:
            for path in large_logs:
                command.extend(["--large-log", str(path)])

    append_stdout_path, append_stderr_path, append_status_path = prepare_append_artifact_paths(
        batch_dir
    )
    output = subprocess.run(
        command,
        cwd=root,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    write_text_no_follow(append_stdout_path, output.stdout)
    write_text_no_follow(append_stderr_path, output.stderr)
    write_status(
        append_status_path,
        [
            f"append_script={append_script}",
            f"improve_log={improve_log_path}",
            f"exit_code={output.returncode}",
            f"append_stdout={append_stdout_path}",
            f"append_stderr={append_stderr_path}",
        ],
    )
    if output.returncode != 0:
        raise SystemExit(
            "append-improve-log failed with status "
            f"{output.returncode}; stderr: {output.stderr.strip()}"
        )


def path_texts(paths: list[Path]) -> list[str]:
    return [str(path) for path in paths]


def timing_seconds_values(paths: list[Path]) -> tuple[list[float], int]:
    values = []
    failed = 0
    for path in paths:
        try:
            values.append(round(timing_total_seconds_from_log(path), 3))
        except (OSError, SystemExit):
            failed += 1
    return values, failed


def excluded_log_paths(all_logs: list[Path], stable_logs: list[Path]) -> list[Path]:
    stable_log_set = set(stable_logs)
    return [path for path in all_logs if path not in stable_log_set]


def timing_average_seconds(values: list[float]) -> float | None:
    if not values:
        return None
    return round(sum(values) / len(values), 3)


def timing_spread_seconds(values: list[float]) -> float | None:
    if not values:
        return None
    return round(max(values) - min(values), 3)


def max_average_rejection_message(
    label: str,
    stable_logs: list[Path],
    max_average_s: float | None,
) -> str | None:
    if max_average_s is None:
        return None
    stable_timing_s, failed = timing_seconds_values(stable_logs)
    if failed or not stable_timing_s:
        return None
    average = timing_average_seconds(stable_timing_s)
    if average is None or average <= max_average_s:
        return None
    option = "--small-max-avg-s" if label == "small" else "--large-max-avg-s"
    return f"{label} proof time: average {average:.3f}s exceeds {option} {max_average_s:.3f}s"


def max_average_rejection_messages(
    small_stable_logs: list[Path],
    large_stable_logs: list[Path],
    small_max_average_s: float | None,
    large_max_average_s: float | None,
) -> list[str]:
    messages = []
    small_message = max_average_rejection_message(
        "small",
        small_stable_logs,
        small_max_average_s,
    )
    if small_message is not None:
        messages.append(small_message)
    large_message = max_average_rejection_message(
        "large",
        large_stable_logs,
        large_max_average_s,
    )
    if large_message is not None:
        messages.append(large_message)
    return messages


def max_average_rejection_field(
    stable_logs: list[Path],
    max_average_s: float | None,
) -> str | None:
    if max_average_s is None:
        return None
    stable_timing_s, failed = timing_seconds_values(stable_logs)
    if failed or not stable_timing_s:
        return None
    average = timing_average_seconds(stable_timing_s)
    if average is None or average <= max_average_s:
        return None
    samples = ";".join(f"{value:.3f}" for value in sorted(stable_timing_s))
    return (
        f"avg={average:.3f} samples={samples} "
        f"used={len(stable_timing_s)}/{len(stable_logs)} "
        f"rejected baseline={max_average_s:.3f}"
    )


def stable_average_field_for_parse_failed_logs(
    logs: list[Path],
    max_relative_spread: float,
) -> str | None:
    if not logs:
        return None
    _samples, failed = parseable_timing_samples(logs)
    if failed == 0:
        return None
    stable_logs = stable_parseable_timing_group(logs, max_relative_spread)
    if not stable_logs:
        return None
    stable_timing_s = [timing_total_seconds_from_log(path) for path in stable_logs]
    average = timing_average_seconds(stable_timing_s)
    if average is None:
        return None
    samples = ";".join(f"{value:.3f}" for value in sorted(stable_timing_s))
    return (
        f"avg={average:.3f} samples={samples} "
        f"used={len(stable_timing_s)}/{len(logs)}"
    )


def retryable_failure_counts(logs: list[Path]) -> dict[str, int]:
    counts: dict[str, int] = {}
    for path in logs:
        try:
            reason = retryable_run_failure_reason(read_text(path))
        except OSError:
            continue
        if reason is not None:
            counts[reason] = counts.get(reason, 0) + 1
    return counts


def unstable_parse_failure_message(
    label: str,
    logs: list[Path],
    max_relative_spread: float,
) -> str | None:
    if not logs:
        return None
    samples, failed = parseable_timing_samples(logs)
    if failed == 0:
        return None
    if stable_parseable_timing_group(logs, max_relative_spread) is not None:
        return None
    retryable_counts = retryable_failure_counts(logs)
    retryable_text = (
        ",".join(
            f"{reason}={count}"
            for reason, count in sorted(retryable_counts.items())
        )
        if retryable_counts
        else "none"
    )
    return (
        f"{label} stable timing group unavailable after {len(logs)} runs; "
        f"parseable_timing_logs={len(samples)}; parse_failed_logs={failed}; "
        f"retryable_failures={retryable_text}"
    )


def unstable_parse_failure_messages(
    small_logs: list[Path],
    large_logs: list[Path],
    max_relative_spread: float,
) -> list[str]:
    messages = []
    for label, logs in [("small", small_logs), ("large", large_logs)]:
        message = unstable_parse_failure_message(label, logs, max_relative_spread)
        if message is not None:
            messages.append(message)
    return messages


def rejected_average_summary(
    summary: str,
    small_field: str | None,
    large_field: str | None,
) -> str:
    labels = []
    if small_field is not None:
        labels.append("small")
    if large_field is not None:
        labels.append("large")
    if not labels:
        return summary
    return f"{summary}; rejected {'/'.join(labels)} baseline"


def requested_summary_output_path(summary: str, root: Path) -> Path | None:
    candidate = Path(summary)
    if candidate.suffix != ".csv":
        return None
    if not candidate.is_absolute() and len(candidate.parts) <= 1:
        return None
    return require_workspace_temp_path(
        resolve_workspace_path(summary, root),
        root,
        "--summary",
    )


def materialize_requested_summary(
    summary: str,
    root: Path,
    stable_summaries: dict[str, Path | None],
    rejected_labels: list[str],
    improve_log_path: Path,
) -> Path | None:
    output_path = requested_summary_output_path(summary, root)
    if output_path is None:
        return None
    if output_path.resolve(strict=False) == improve_log_path.resolve(strict=False):
        raise SystemExit("--summary CSV output must not match --path improvement log")
    sources = [
        (label, stable_summaries[label])
        for label in rejected_labels
        if stable_summaries.get(label) is not None
    ]
    if not sources:
        return None
    source_path: Path | None = None
    if len(sources) == 1:
        source_path = sources[0][1]
    else:
        output_name = output_path.name.lower()
        for label, source in sources:
            if label in output_name:
                source_path = source
                break
    if source_path is None:
        return None
    output_path.parent.mkdir(parents=True, exist_ok=True)
    write_text_no_follow(output_path, read_text(source_path))
    stderr_source = prefixed_path(source_path.parent, source_path.name, ".stderr")
    if path_is_regular_file_no_follow(stderr_source):
        write_text_no_follow(
            prefixed_path(output_path.parent, output_path.name, ".stderr"),
            read_text(stderr_source),
        )
    return output_path


def timing_milliseconds(value: float | None) -> int:
    if value is None:
        return 0
    return int(round(value * 1000))


def timing_relative_spread(values: list[float]) -> float | None:
    if not values:
        return None
    ordered = sorted(values)
    median = ordered[len(ordered) // 2]
    if median == 0.0:
        return None
    return round((ordered[-1] - ordered[0]) / median, 6)


def print_stable_timing(label: str, logs: list[Path]) -> None:
    values, failed = timing_seconds_values(logs)
    average = timing_average_seconds(values)
    spread = timing_spread_seconds(values)
    relative_spread = timing_relative_spread(values)
    if average is not None:
        print(f"{label}_stable_avg_s={average:.3f}")
    if spread is not None:
        print(f"{label}_stable_spread_s={spread:.3f}")
    if relative_spread is not None:
        print(f"{label}_stable_relative_spread={relative_spread:.6f}")
    if failed:
        print(f"{label}_stable_timing_parse_failed_count={failed}")


def print_excluded_timing(label: str, all_logs: list[Path], stable_logs: list[Path]) -> None:
    excluded_logs = excluded_log_paths(all_logs, stable_logs)
    excluded, failed = timing_seconds_values(excluded_logs)
    print(f"{label}_excluded_runs={len(excluded_logs)}")
    if excluded:
        joined = ";".join(f"{value:.3f}" for value in excluded)
        print(f"{label}_excluded_timing_s={joined}")
    if failed:
        print(f"{label}_excluded_timing_parse_failed_count={failed}")


def safe_stable_timing_group(
    logs: list[Path],
    max_relative_spread: float,
) -> list[Path]:
    return stable_parseable_timing_group(logs, max_relative_spread) or []


def discovered_run_paths(batch_dir: Path, label: str, suffix: str) -> list[Path]:
    run_path_re = re.compile(rf"{re.escape(label)}-(\d+){re.escape(suffix)}\Z")
    matches: list[tuple[int, str, Path]] = []
    for path in batch_dir.glob(f"{label}-*{suffix}"):
        match = run_path_re.fullmatch(path.name)
        if match:
            matches.append((int(match.group(1)), path.name, path))
    return [path for _run, _name, path in sorted(matches)]


def write_batch_json(
    path: Path,
    args: argparse.Namespace,
    max_runs: int,
    commit: str,
    batch_started_at: str,
    batch_start_load_average: list[float] | None,
    batch_start_gpu_status: list[dict[str, int]] | None,
    root: Path,
    batch_dir: Path,
    cwd: Path,
    improve_log_path: Path,
    small_logs: list[Path] | None,
    large_logs: list[Path] | None,
    small_statuses: list[Path] | None,
    large_statuses: list[Path] | None,
    small_stable_logs: list[Path] | None,
    large_stable_logs: list[Path] | None,
    small_timing_summaries: list[Path] | None,
    large_timing_summaries: list[Path] | None,
    small_stable_timing_summary: Path | None,
    large_stable_timing_summary: Path | None,
    appended: bool,
) -> None:
    small_timing_s, small_timing_parse_failed_count = timing_seconds_values(small_logs or [])
    large_timing_s, large_timing_parse_failed_count = timing_seconds_values(large_logs or [])
    small_stable_timing_s, small_stable_timing_parse_failed_count = timing_seconds_values(
        small_stable_logs or []
    )
    large_stable_timing_s, large_stable_timing_parse_failed_count = timing_seconds_values(
        large_stable_logs or []
    )
    small_excluded_logs = excluded_log_paths(small_logs or [], small_stable_logs or [])
    large_excluded_logs = excluded_log_paths(large_logs or [], large_stable_logs or [])
    small_excluded_timing_s, _ = timing_seconds_values(small_excluded_logs)
    large_excluded_timing_s, _ = timing_seconds_values(large_excluded_logs)
    small_stable_avg_s = timing_average_seconds(small_stable_timing_s)
    large_stable_avg_s = timing_average_seconds(large_stable_timing_s)
    small_stable_spread_s = timing_spread_seconds(small_stable_timing_s)
    large_stable_spread_s = timing_spread_seconds(large_stable_timing_s)
    append_stdout_path, append_stderr_path, append_status_path = append_artifact_paths(batch_dir)
    worktree_status = tracked_worktree_status(root)
    load_average = host_load_average()
    gpu_status = host_gpu_status()
    payload = {
        "created_at": datetime.now().astimezone().strftime("%Y-%m-%dT%H:%M:%S%z"),
        "batch_started_at": batch_started_at,
        "workspace": str(root),
        "batch_dir": str(batch_dir),
        "cwd": str(cwd),
        "improve_log": str(improve_log_path),
        "appended": appended,
        "append_script": args.append_script,
        "append_status": (
            str(append_status_path)
            if path_is_regular_file_no_follow(append_status_path)
            else None
        ),
        "append_stdout": (
            str(append_stdout_path)
            if path_is_regular_file_no_follow(append_stdout_path)
            else None
        ),
        "append_stderr": (
            str(append_stderr_path)
            if path_is_regular_file_no_follow(append_stderr_path)
            else None
        ),
        "runs": args.runs,
        "max_runs": max_runs,
        "small_timeout_s": args.small_timeout,
        "large_timeout_s": args.large_timeout,
        "max_relative_spread": args.max_relative_spread,
        "small_max_avg_s": args.small_max_avg_s,
        "large_max_avg_s": args.large_max_avg_s,
        "retryable_failure_wait_s": args.retryable_failure_wait_s,
        "inherited_runtime_env": inherited_runtime_env(),
        "append_max_average_rejections": args.append_max_average_rejections,
        "commit": commit,
        "tracked_worktree_dirty": bool(worktree_status),
        "tracked_worktree_status": worktree_status,
        "batch_start_host_load_average": batch_start_load_average,
        "batch_start_gpu_status": batch_start_gpu_status,
        "host_load_average": load_average,
        "host_gpu_status": gpu_status,
        "host_cpu_count": os.cpu_count(),
        "summary": args.summary,
        "metadata_lines": list(args.metadata_line or []),
        "small_command": args.small_command,
        "large_command": args.large_command,
        "timing_summary_script": args.timing_summary_script,
        "require_text": list(args.require_text or []),
        "small_require_text": list(args.small_require_text or []),
        "large_require_text": list(args.large_require_text or []),
        "require_proof_output": args.require_proof_output,
        "small_run_count": len(small_logs or []),
        "large_run_count": len(large_logs or []),
        "small_logs": path_texts(small_logs or []),
        "large_logs": path_texts(large_logs or []),
        "small_timing_s": small_timing_s,
        "large_timing_s": large_timing_s,
        "small_timing_parse_failed_count": small_timing_parse_failed_count,
        "large_timing_parse_failed_count": large_timing_parse_failed_count,
        "small_stable_run_count": len(small_stable_logs or []),
        "large_stable_run_count": len(large_stable_logs or []),
        "small_stable_logs": path_texts(small_stable_logs or []),
        "large_stable_logs": path_texts(large_stable_logs or []),
        "small_excluded_logs": path_texts(small_excluded_logs),
        "large_excluded_logs": path_texts(large_excluded_logs),
        "small_excluded_log_count": len(small_excluded_logs),
        "large_excluded_log_count": len(large_excluded_logs),
        "small_stable_timing_s": small_stable_timing_s,
        "large_stable_timing_s": large_stable_timing_s,
        "small_excluded_timing_s": small_excluded_timing_s,
        "large_excluded_timing_s": large_excluded_timing_s,
        "small_excluded_run_count": len(small_excluded_timing_s),
        "large_excluded_run_count": len(large_excluded_timing_s),
        "small_stable_avg_s": small_stable_avg_s,
        "large_stable_avg_s": large_stable_avg_s,
        "small_stable_avg_ms": timing_milliseconds(small_stable_avg_s),
        "large_stable_avg_ms": timing_milliseconds(large_stable_avg_s),
        "small_stable_spread_s": small_stable_spread_s,
        "large_stable_spread_s": large_stable_spread_s,
        "small_stable_spread_ms": timing_milliseconds(small_stable_spread_s),
        "large_stable_spread_ms": timing_milliseconds(large_stable_spread_s),
        "small_stable_relative_spread": timing_relative_spread(small_stable_timing_s),
        "large_stable_relative_spread": timing_relative_spread(large_stable_timing_s),
        "small_stable_timing_parse_failed_count": small_stable_timing_parse_failed_count,
        "large_stable_timing_parse_failed_count": large_stable_timing_parse_failed_count,
        "small_statuses": path_texts(small_statuses or []),
        "large_statuses": path_texts(large_statuses or []),
        "small_timing_summaries": path_texts(small_timing_summaries or []),
        "large_timing_summaries": path_texts(large_timing_summaries or []),
        "small_stable_timing_summary": (
            str(small_stable_timing_summary) if small_stable_timing_summary else None
        ),
        "large_stable_timing_summary": (
            str(large_stable_timing_summary) if large_stable_timing_summary else None
        ),
    }
    write_text_no_follow(path, json.dumps(payload, indent=2, sort_keys=True) + "\n")


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
    batch_started_at = datetime.now().astimezone().strftime("%Y-%m-%dT%H:%M:%S%z")
    batch_start_load_average = host_load_average()
    batch_start_gpu_status = host_gpu_status()
    if args.small_command is None and args.large_command is None:
        raise SystemExit("provide --small-command and/or --large-command")
    if args.summary is None:
        raise SystemExit("--summary is required")
    max_runs = args.max_runs if args.max_runs is not None else args.runs
    if max_runs < args.runs:
        raise SystemExit("--max-runs must be at least --runs")
    commit = args.commit or current_commit(root)

    append_script = resolve_workspace_path(args.append_script, root)
    if not append_script.exists():
        raise SystemExit(f"{append_script}: append script does not exist")
    timing_summary_script = resolve_workspace_path(args.timing_summary_script, root)
    if not timing_summary_script.exists():
        raise SystemExit(f"{timing_summary_script}: timing summary script does not exist")
    work_dir = require_workspace_temp_path(
        resolve_workspace_path(args.work_dir, root),
        root,
        "--work-dir",
    )
    cwd = resolve_workspace_path(args.cwd, root)
    if not cwd.exists():
        raise SystemExit(f"{cwd}: command working directory does not exist")
    if not cwd.is_dir():
        raise SystemExit(f"{cwd}: command working directory is not a directory")
    improve_log_path = require_workspace_temp_path(
        resolve_workspace_path(args.path, root),
        root,
        "--path",
    )
    summary_output_path = requested_summary_output_path(args.summary, root)
    if (
        summary_output_path is not None
        and summary_output_path.resolve(strict=False)
        == improve_log_path.resolve(strict=False)
    ):
        raise SystemExit("--summary CSV output must not match --path improvement log")
    batch_dir = work_dir / batch_dir_name()
    batch_dir.mkdir(parents=True, exist_ok=False)
    batch_json_path = batch_dir / "batch.json"
    stable_timing_summary_paths: dict[str, Path | None] = {
        "small": None,
        "large": None,
    }

    def record_batch_json(
        small_logs: list[Path] | None = None,
        large_logs: list[Path] | None = None,
        appended: bool = False,
    ) -> None:
        small_statuses = discovered_run_paths(batch_dir, "small", ".status")
        large_statuses = discovered_run_paths(batch_dir, "large", ".status")
        small_stable_logs = safe_stable_timing_group(
            small_logs or [],
            args.max_relative_spread,
        )
        large_stable_logs = safe_stable_timing_group(
            large_logs or [],
            args.max_relative_spread,
        )
        small_timing_summaries = discovered_run_paths(
            batch_dir,
            "small",
            ".proof-timing-summary.csv",
        )
        large_timing_summaries = discovered_run_paths(
            batch_dir,
            "large",
            ".proof-timing-summary.csv",
        )
        write_batch_json(
            batch_json_path,
            args,
            max_runs,
            commit,
            batch_started_at,
            batch_start_load_average,
            batch_start_gpu_status,
            root,
            batch_dir,
            cwd,
            improve_log_path,
            small_logs,
            large_logs,
            small_statuses,
            large_statuses,
            small_stable_logs,
            large_stable_logs,
            small_timing_summaries,
            large_timing_summaries,
            stable_timing_summary_paths["small"],
            stable_timing_summary_paths["large"],
            appended,
        )

    record_batch_json()

    small_logs: list[Path] = []
    large_logs: list[Path] = []
    try:
        small_logs = run_group(
            "small",
            args.small_command,
            args.runs,
            max_runs,
            args.small_timeout,
            batch_dir,
            cwd,
            root,
            timing_summary_script,
            required_texts_for_label(args, "small"),
            args.max_relative_spread,
            args.retryable_failure_wait_s,
        )
        large_logs = run_group(
            "large",
            args.large_command,
            args.runs,
            max_runs,
            args.large_timeout,
            batch_dir,
            cwd,
            root,
            timing_summary_script,
            required_texts_for_label(args, "large"),
            args.max_relative_spread,
            args.retryable_failure_wait_s,
        )
    except SystemExit:
        record_batch_json(
            discovered_run_paths(batch_dir, "small", ".log"),
            discovered_run_paths(batch_dir, "large", ".log"),
            appended=False,
        )
        raise
    small_stable_logs = safe_stable_timing_group(small_logs, args.max_relative_spread)
    large_stable_logs = safe_stable_timing_group(large_logs, args.max_relative_spread)
    try:
        stable_timing_summary_paths["small"] = write_group_timing_summary(
            timing_summary_script,
            small_stable_logs,
            batch_dir / "small-stable.proof-timing-summary.csv",
            root,
        )
        stable_timing_summary_paths["large"] = write_group_timing_summary(
            timing_summary_script,
            large_stable_logs,
            batch_dir / "large-stable.proof-timing-summary.csv",
            root,
        )
    except SystemExit:
        record_batch_json(small_logs, large_logs, appended=False)
        raise
    record_batch_json(small_logs, large_logs, appended=False)
    parse_failure_messages = unstable_parse_failure_messages(
        small_logs,
        large_logs,
        args.max_relative_spread,
    )
    if parse_failure_messages:
        raise SystemExit("; ".join(parse_failure_messages))
    rejection_messages = max_average_rejection_messages(
        small_stable_logs,
        large_stable_logs,
        args.small_max_avg_s,
        args.large_max_avg_s,
    )
    small_rejection_field = max_average_rejection_field(
        small_stable_logs,
        args.small_max_avg_s,
    )
    large_rejection_field = max_average_rejection_field(
        large_stable_logs,
        args.large_max_avg_s,
    )
    rejected_labels = []
    if small_rejection_field is not None:
        rejected_labels.append("small")
    if large_rejection_field is not None:
        rejected_labels.append("large")
    if rejection_messages:
        materialize_requested_summary(
            args.summary,
            root,
            stable_timing_summary_paths,
            rejected_labels,
            improve_log_path,
        )
    if args.append_max_average_rejections and rejection_messages:
        try:
            append_improve_log(
                append_script,
                improve_log_path,
                commit,
                rejected_average_summary(
                    args.summary,
                    small_rejection_field,
                    large_rejection_field,
                ),
                small_logs,
                large_logs,
                small_rejection_field,
                large_rejection_field,
                args.max_relative_spread,
                None,
                None,
                root,
                batch_dir,
            )
        except SystemExit:
            record_batch_json(small_logs, large_logs, appended=False)
            raise
        record_batch_json(small_logs, large_logs, appended=True)
        raise SystemExit("; ".join(rejection_messages))
    try:
        append_improve_log(
            append_script,
            improve_log_path,
            commit,
            args.summary,
            small_logs,
            large_logs,
            None,
            None,
            args.max_relative_spread,
            args.small_max_avg_s,
            args.large_max_avg_s,
            root,
            batch_dir,
        )
    except SystemExit:
        record_batch_json(small_logs, large_logs, appended=False)
        raise
    record_batch_json(small_logs, large_logs, appended=True)

    print(f"batch_dir={batch_dir}")
    print(f"batch_json={batch_json_path}")
    if small_logs:
        print(f"small_runs={len(small_logs)}")
        print(f"small_stable_runs={len(small_stable_logs)}")
        print_stable_timing("small", small_stable_logs)
        print_excluded_timing("small", small_logs, small_stable_logs)
        print(
            "small_timing_summaries="
            f"{len(discovered_run_paths(batch_dir, 'small', '.proof-timing-summary.csv'))}"
        )
        if stable_timing_summary_paths["small"] is not None:
            print(f"small_stable_timing_summary={stable_timing_summary_paths['small']}")
    if large_logs:
        print(f"large_runs={len(large_logs)}")
        print(f"large_stable_runs={len(large_stable_logs)}")
        print_stable_timing("large", large_stable_logs)
        print_excluded_timing("large", large_logs, large_stable_logs)
        print(
            "large_timing_summaries="
            f"{len(discovered_run_paths(batch_dir, 'large', '.proof-timing-summary.csv'))}"
        )
        if stable_timing_summary_paths["large"] is not None:
            print(f"large_stable_timing_summary={stable_timing_summary_paths['large']}")
    print(f"improve_log={improve_log_path}")
    return batch_dir


def self_test() -> None:
    root = workspace_root()
    work_dir = root / "temp" / f"proof-timing-batch-self-test-{os.getpid()}"
    shutil.rmtree(work_dir, ignore_errors=True)
    command = shlex.join(
        [
            sys.executable,
            "-c",
            (
                "print('timing_total_ms=1000'); "
                "print('timing_guest_stage_tree_commit_root_count=1'); "
                "print('timing_guest_stage_tree_commit_root_materialization_groups=1'); "
                "print('timing_guest_stage_tree_commit_root_materialization_max_group_size=1'); "
                "print('timing_finish_witness_opening_row_values_ms=0'); "
                "print('timing_finish_witness_opening_row_value_source_extend_ms=0'); "
                "print('timing_finish_witness_opening_row_value_source_download_ms=0'); "
                "print('timing_finish_witness_opening_row_value_device_download_ms=0'); "
                "print('timing_finish_witness_opening_row_values_device_rows=0'); "
                "print('timing_finish_witness_opening_row_values_device_download_batches=0'); "
                "print('timing_finish_witness_opening_row_values_device_single_downloads=0'); "
                "print('timing_finish_witness_opening_row_value_source_extend_calls=0'); "
                "print('timing_finish_witness_opening_row_value_source_extend_max_rows=0'); "
                "print('timing_finish_witness_opening_row_values_source_rows=0'); "
                "print('timing_finish_witness_opening_row_values_words=0'); "
                "print('timing_finish_witness_opening_row_values_bytes=0'); "
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
    )
    args = argparse.Namespace(
        append_script="scripts/append-improve-log.py",
        commit="selftest",
        cwd=".",
        large_command=command,
        large_timeout=10.0,
        timing_summary_script="scripts/prove-timing-root-summary.py",
        max_relative_spread=0.10,
        max_runs=None,
        metadata_line=["wrapper=proof-self-test"],
        small_max_avg_s=None,
        large_max_avg_s=None,
        retryable_failure_wait_s=0.0,
        append_max_average_rejections=False,
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
    previous_runtime_env = {
        name: os.environ.get(name) for name in INHERITED_RUNTIME_ENV_NAMES
    }
    for name in INHERITED_RUNTIME_ENV_NAMES:
        os.environ.pop(name, None)
    os.environ["LZVM_CUDA_RETAINED_SOURCE_BYTES"] = "123456"
    os.environ["LZVM_GUEST_PC_TRACE_DESCRIPTOR_HIGH32_STATS"] = "1"
    try:
        work_dir.mkdir(parents=True, exist_ok=True)
        improve_log = work_dir / "improve-log.csv"
        improve_log.write_text(
            "\n".join(
                [
                    "timestamp,commit,small_proof_time_s,large_proof_time_s,summary",
                    '"2026-01-01T00:00:00+0000","selftest","","timeout=10s run1 after retry","timeout note"',
                ]
            )
            + "\n",
            encoding="utf-8",
        )
        batch_dir = run_batch(args)
        contents = improve_log.read_text(encoding="utf-8")
        expected = '"avg=1.000 samples=1.000;1.000;1.000 used=3/3"'
        if expected not in contents:
            raise SystemExit("self-test improve log did not contain the expected average")
        if "timeout=10s run1 after retry" not in contents:
            raise SystemExit("self-test improve log should preserve timeout note")
        summary = batch_dir / "small-001.proof-timing-summary.csv"
        if not summary.exists():
            raise SystemExit("self-test timing summary missing")
        if not summary.read_text(encoding="utf-8").startswith("profile,"):
            raise SystemExit("self-test timing summary is not CSV")
        stable_summary = batch_dir / "small-stable.proof-timing-summary.csv"
        if not stable_summary.exists():
            raise SystemExit("self-test stable timing summary missing")
        stable_summary_text = stable_summary.read_text(encoding="utf-8")
        if "aggregate,total_count,valid_total_count" not in stable_summary_text:
            raise SystemExit("self-test stable timing summary missing aggregate row")
        batch_payload = json.loads((batch_dir / "batch.json").read_text(encoding="utf-8"))
        expected_runtime_env = {
            "LZVM_CUDA_RETAINED_SOURCE_BYTES": "123456",
            "LZVM_GUEST_PC_TRACE_DESCRIPTOR_HIGH32_STATS": "1",
        }
        if batch_payload.get("inherited_runtime_env") != expected_runtime_env:
            raise SystemExit("self-test batch json should record inherited runtime env")
        if batch_payload.get("metadata_lines") != ["wrapper=proof-self-test"]:
            raise SystemExit("self-test batch json should record metadata lines")
        if not isinstance(batch_payload.get("tracked_worktree_dirty"), bool):
            raise SystemExit("self-test batch json should record dirty status")
        if not isinstance(batch_payload.get("tracked_worktree_status"), list):
            raise SystemExit("self-test batch json should record tracked status lines")
        if not isinstance(batch_payload.get("batch_started_at"), str):
            raise SystemExit("self-test batch json should record batch start time")
        batch_start_load = batch_payload.get("batch_start_host_load_average")
        if batch_start_load is not None and not (
            isinstance(batch_start_load, list) and len(batch_start_load) == 3
        ):
            raise SystemExit("self-test batch json should record batch start load average")
        host_load = batch_payload.get("host_load_average")
        if host_load is not None and not (
            isinstance(host_load, list) and len(host_load) == 3
        ):
            raise SystemExit("self-test batch json should record host load average")
        for key in ["batch_start_gpu_status", "host_gpu_status"]:
            gpu_status = batch_payload.get(key)
            if gpu_status is not None and not isinstance(gpu_status, list):
                raise SystemExit(f"self-test batch json should record {key}")
        if not isinstance(batch_payload.get("host_cpu_count"), int):
            raise SystemExit("self-test batch json should record host CPU count")
        if (
            retryable_run_failure_reason("cuda backend out of memory: error code 2")
            != "cuda_memory_exhausted"
        ):
            raise SystemExit("self-test should classify cuda backend memory exhaustion")
        for key in [
            "small_stable_spread_s",
            "large_stable_spread_s",
            "small_stable_relative_spread",
            "large_stable_relative_spread",
            "small_stable_spread_ms",
            "large_stable_spread_ms",
        ]:
            if batch_payload.get(key) != 0.0:
                raise SystemExit(f"self-test batch json {key} should be zero")
        for key in ["small_stable_avg_ms", "large_stable_avg_ms"]:
            if batch_payload.get(key) != 1000:
                raise SystemExit(f"self-test batch json {key} should record milliseconds")
        expected_counts = {
            "small_run_count": 3,
            "large_run_count": 3,
            "small_stable_run_count": 3,
            "large_stable_run_count": 3,
        }
        for key, value in expected_counts.items():
            if batch_payload.get(key) != value:
                raise SystemExit(f"self-test batch json {key} should record count")
        for key in ["small_stable_timing_s", "large_stable_timing_s"]:
            if batch_payload.get(key) != [1.0, 1.0, 1.0]:
                raise SystemExit(f"self-test batch json {key} should record samples")
        retry_command = shlex.join(
            [
                sys.executable,
                "-c",
                (
                    "import os, sys; "
                    "fail = os.environ.get('LZVM_TIMING_BATCH_RUN') == '1'; "
                    "sys.stderr.write('prove witness failed: large --guest-pc-trace "
                    "GPU memory preflight failed\\n') if fail else None; "
                    "sys.exit(1) if fail else None; "
                    "print('timing_total_ms=1000'); "
                    "print('timing_guest_stage_tree_commit_root_count=1'); "
                    "print('timing_guest_stage_tree_commit_root_materialization_groups=1'); "
                    "print('timing_guest_stage_tree_commit_root_materialization_max_group_size=1'); "
                    "print('timing_finish_witness_opening_row_values_ms=0'); "
                    "print('timing_finish_witness_opening_row_value_source_extend_ms=0'); "
                    "print('timing_finish_witness_opening_row_value_source_download_ms=0'); "
                    "print('timing_finish_witness_opening_row_value_device_download_ms=0'); "
                    "print('timing_finish_witness_opening_row_values_device_rows=0'); "
                    "print('timing_finish_witness_opening_row_values_device_download_batches=0'); "
                    "print('timing_finish_witness_opening_row_values_device_single_downloads=0'); "
                    "print('timing_finish_witness_opening_row_value_source_extend_calls=0'); "
                    "print('timing_finish_witness_opening_row_value_source_extend_max_rows=0'); "
                    "print('timing_finish_witness_opening_row_values_source_rows=0'); "
                    "print('timing_finish_witness_opening_row_values_words=0'); "
                    "print('timing_finish_witness_opening_row_values_bytes=0'); "
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
        )
        retry_args = argparse.Namespace(**vars(args))
        retry_args.small_command = None
        retry_args.large_command = retry_command
        retry_args.max_runs = 4
        retry_args.path = str(work_dir / "retry-log.csv")
        retry_args.retryable_failure_wait_s = 0.001
        retry_args.summary = "retry self test"
        retry_args.work_dir = str(work_dir / "retry-runs")
        retry_batch_dir = run_batch(retry_args)
        retry_payload = json.loads(
            (retry_batch_dir / "batch.json").read_text(encoding="utf-8")
        )
        if retry_payload.get("large_run_count") != 4:
            raise SystemExit("self-test retry batch should keep retryable failed log")
        if retry_payload.get("large_excluded_log_count") != 1:
            raise SystemExit("self-test retry batch should exclude failed log")
        if retry_payload.get("large_stable_run_count") != 3:
            raise SystemExit("self-test retry batch should find stable retries")
        retry_status = (retry_batch_dir / "large-001.status").read_text(encoding="utf-8")
        if "retryable_failure=gpu_memory_preflight" not in retry_status:
            raise SystemExit("self-test retry batch should mark retryable failure")
        retry_log = (work_dir / "retry-log.csv").read_text(encoding="utf-8")
        if "used=3/4" not in retry_log:
            raise SystemExit("self-test retry log should append stable retry count")
        guarded_log = work_dir / "guard-log.csv"
        reject_args = argparse.Namespace(**vars(args))
        reject_args.path = str(guarded_log)
        reject_args.summary = str(guarded_log)
        reject_args.work_dir = str(work_dir / "reject-runs")
        reject_args.small_max_avg_s = 0.5
        reject_args.large_max_avg_s = 0.5
        try:
            run_batch(reject_args)
        except SystemExit as error:
            expected_error = "--summary CSV output must not match --path improvement log"
            if expected_error not in str(error):
                raise SystemExit("self-test summary path guard reported the wrong error")
        else:
            raise SystemExit("self-test summary path guard should fail")
        if guarded_log.exists():
            raise SystemExit("self-test guarded improve log should not be written")
    finally:
        for name, value in previous_runtime_env.items():
            if value is None:
                os.environ.pop(name, None)
            else:
                os.environ[name] = value
        shutil.rmtree(work_dir, ignore_errors=True)


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Run proof timing commands repeatedly and append stable CSV fields."
    )
    parser.add_argument("--small-command")
    parser.add_argument("--large-command")
    parser.add_argument("--runs", type=positive_run_count, default=3)
    parser.add_argument("--max-runs", type=positive_run_count, default=None)
    parser.add_argument("--small-timeout", type=positive_timeout, default=60.0)
    parser.add_argument("--large-timeout", type=positive_timeout, default=180.0)
    parser.add_argument("--work-dir", default="temp/proof-timing-batch")
    parser.add_argument("--cwd", default=".")
    parser.add_argument("--path", default="temp/improve-log.csv")
    parser.add_argument("--commit", default=None)
    parser.add_argument("--summary")
    parser.add_argument("--max-relative-spread", type=nonnegative_float, default=0.10)
    parser.add_argument("--small-max-avg-s", type=positive_timeout, default=None)
    parser.add_argument("--large-max-avg-s", type=positive_timeout, default=None)
    parser.add_argument("--retryable-failure-wait-s", type=nonnegative_float, default=0.0)
    parser.add_argument("--append-max-average-rejections", action="store_true")
    parser.add_argument("--append-script", default="scripts/append-improve-log.py")
    parser.add_argument("--timing-summary-script", default="scripts/prove-timing-root-summary.py")
    parser.add_argument("--metadata-line", action="append", default=[])
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
