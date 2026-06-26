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
TIMING_SUMMARY_REQUIRED_KEYS = [
    "timing_total_ms",
    "timing_guest_stage_tree_commit_root_count",
    "timing_guest_stage_tree_commit_root_materialization_groups",
    "timing_guest_stage_tree_commit_root_materialization_max_group_size",
    "timing_finish_witness_opening_row_dedup_input_rows",
    "timing_finish_witness_opening_row_dedup_unique_rows",
    "timing_finish_witness_opening_row_dedup_elided_rows",
]


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


def write_combined_log(path: Path, stdout: str, stderr: str) -> None:
    combined = ["[stdout]\n", stdout]
    if stdout and not stdout.endswith("\n"):
        combined.append("\n")
    combined.extend(["[stderr]\n", stderr])
    if stderr and not stderr.endswith("\n"):
        combined.append("\n")
    write_text_no_follow(path, "".join(combined))


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
    if matches[0] <= 0:
        raise SystemExit(f"{path}: timing_total_ms must be positive")
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
                continue
            relative_spread = (candidate[-1] - candidate[0]) / median
            if relative_spread <= max_relative_spread:
                if best is None or len(candidate) > len(best):
                    best = candidate
                    best_window = (start, end)
                elif best is not None and len(candidate) == len(best):
                    best_spread = (best[-1] - best[0]) / best[len(best) // 2]
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


def has_stable_timing_group(
    logs: list[Path],
    max_relative_spread: float,
    min_stable_count: int = 3,
) -> bool:
    return stable_timing_group(logs, max_relative_spread, min_stable_count) is not None


def require_texts_in_log(text: str, path: Path, required_texts: list[str]) -> None:
    for required in required_texts:
        if required not in text:
            raise SystemExit(f"{path}: missing required text {required!r}")


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
    tmp_dir.mkdir(parents=True, exist_ok=True)
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
    env = os.environ.copy()
    env["LZVM_TIMING_BATCH_LABEL"] = label
    env["LZVM_TIMING_BATCH_RUN"] = str(run_index)
    env["LZVM_TIMING_BATCH_RUNS"] = str(run_count)
    env["LZVM_TIMING_BATCH_MAX_RUNS"] = str(max_run_count)
    env["TMPDIR"] = str(tmp_dir)

    start = time.monotonic()
    timed_out = False
    with open_text_no_follow(stdout_path) as stdout_file:
        with open_text_no_follow(stderr_path) as stderr_file:
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
        f"tmp_dir={tmp_dir}",
        f"elapsed_s={elapsed_s:.3f}",
        f"timeout_s={timeout:.3f}",
        f"exit_code={exit_code}",
        f"timed_out={str(timed_out).lower()}",
        f"combined_log={combined_path}",
    ]
    if timed_out:
        write_status(status_path, status_lines)
        raise SystemExit(
            f"{label} run {run_index} timed out after {timeout:.3f}s; log: {combined_path}"
        )
    if exit_code != 0:
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
) -> list[Path]:
    if command is None:
        return []
    logs = []
    for run_index in range(1, max_run_count + 1):
        logs.append(
            run_once(
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
        )
        if len(logs) >= run_count:
            if max_run_count == run_count:
                break
            if has_stable_timing_group(logs, max_relative_spread):
                break
    return logs


def append_improve_log(
    append_script: Path,
    improve_log_path: Path,
    commit: str | None,
    summary: str,
    small_logs: list[Path],
    large_logs: list[Path],
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
    write_text_no_follow(batch_dir / "append.stdout", output.stdout)
    write_text_no_follow(batch_dir / "append.stderr", output.stderr)
    if output.returncode != 0:
        raise SystemExit(
            "append-improve-log failed with status "
            f"{output.returncode}; stderr: {output.stderr.strip()}"
        )


def path_texts(paths: list[Path]) -> list[str]:
    return [str(path) for path in paths]


def average_timing_seconds(paths: list[Path]) -> float | None:
    if not paths:
        return None
    average = sum(timing_total_seconds_from_log(path) for path in paths) / len(paths)
    return round(average, 3)


def safe_stable_timing_group(
    logs: list[Path],
    max_relative_spread: float,
) -> list[Path]:
    try:
        return stable_timing_group(logs, max_relative_spread) or []
    except SystemExit:
        return []


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
    payload = {
        "created_at": datetime.now().astimezone().strftime("%Y-%m-%dT%H:%M:%S%z"),
        "workspace": str(root),
        "batch_dir": str(batch_dir),
        "cwd": str(cwd),
        "improve_log": str(improve_log_path),
        "appended": appended,
        "runs": args.runs,
        "max_runs": max_runs,
        "small_timeout_s": args.small_timeout,
        "large_timeout_s": args.large_timeout,
        "max_relative_spread": args.max_relative_spread,
        "small_max_avg_s": args.small_max_avg_s,
        "large_max_avg_s": args.large_max_avg_s,
        "commit": commit,
        "summary": args.summary,
        "small_command": args.small_command,
        "large_command": args.large_command,
        "timing_summary_script": args.timing_summary_script,
        "require_text": list(args.require_text or []),
        "small_require_text": list(args.small_require_text or []),
        "large_require_text": list(args.large_require_text or []),
        "require_proof_output": args.require_proof_output,
        "small_logs": path_texts(small_logs or []),
        "large_logs": path_texts(large_logs or []),
        "small_stable_logs": path_texts(small_stable_logs or []),
        "large_stable_logs": path_texts(large_stable_logs or []),
        "small_stable_avg_s": average_timing_seconds(small_stable_logs or []),
        "large_stable_avg_s": average_timing_seconds(large_stable_logs or []),
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
    try:
        append_improve_log(
            append_script,
            improve_log_path,
            commit,
            args.summary,
            small_logs,
            large_logs,
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
        print(
            "small_timing_summaries="
            f"{len(discovered_run_paths(batch_dir, 'small', '.proof-timing-summary.csv'))}"
        )
        if stable_timing_summary_paths["small"] is not None:
            print(f"small_stable_timing_summary={stable_timing_summary_paths['small']}")
    if large_logs:
        print(f"large_runs={len(large_logs)}")
        print(f"large_stable_runs={len(large_stable_logs)}")
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
                "print('timing_finish_witness_opening_row_dedup_input_rows=0'); "
                "print('timing_finish_witness_opening_row_dedup_unique_rows=0'); "
                "print('timing_finish_witness_opening_row_dedup_elided_rows=0')"
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
        small_max_avg_s=None,
        large_max_avg_s=None,
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
        batch_dir = run_batch(args)
        contents = (work_dir / "improve-log.csv").read_text(encoding="utf-8")
        expected = '"avg=1.000 samples=1.000;1.000;1.000 used=3/3"'
        if expected not in contents:
            raise SystemExit("self-test improve log did not contain the expected average")
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
    finally:
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
    parser.add_argument("--append-script", default="scripts/append-improve-log.py")
    parser.add_argument("--timing-summary-script", default="scripts/prove-timing-root-summary.py")
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
