#!/usr/bin/env python3
import argparse
import os
import shlex
import shutil
import subprocess
import sys
from pathlib import Path

SMALL_PREFIX = "LZVM_REAL_SMALL_PARITY"
LARGE_PREFIX = "LZVM_REAL_LARGE_PARITY"

REQUIRED_SUFFIXES = [
    "SETUP",
    "BLOCK_INPUT",
    "PROGRAM_IMAGE_CACHE",
    "INPUT_DATA",
    "GUEST_IMAGE",
]

PIPELINE_ENV_TO_CLEAR = [
    "LZVM_GUEST_PC_TRACE_PARALLEL_LOWER",
    "LZVM_GUEST_PC_TRACE_COMMIT_PIPELINE",
    "LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_WORKERS",
    "LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_JOB_QUEUE",
    "LZVM_GUEST_PC_TRACE_SEGMENT_COMMIT_WORKERS",
    "LZVM_GUEST_PC_TRACE_SEGMENT_COMMIT_ASYNC_SINGLE",
    "LZVM_GUEST_PC_TRACE_SEGMENT_REPLAY",
    "LZVM_GUEST_PC_TRACE_SEGMENT_REPLAY_SNAPSHOT",
    "LZVM_GUEST_PC_TRACE_RUNNER_SEED_SNAPSHOT",
    "LZVM_GUEST_PC_TRACE_RUNNER_SEED_SNAPSHOT_TRUSTED",
    "LZVM_GUEST_PC_TRACE_RUNNER_SEED_SNAPSHOT_VALIDATE",
    "LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_REPLAY",
    "LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_REPLAY_ONLY",
    "LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_REPLAY_SNAPSHOT",
    "LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_WORK_UNITS",
    "LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_WORKER_REPLAY",
    "LZVM_GUEST_PC_TRACE_LIVE_REPORT_CHUNKS",
    "LZVM_GUEST_PC_TRACE_LIVE_STREAM_START",
    "LZVM_GUEST_PC_TRACE_PARALLEL_STREAM_CHUNKS",
    "LZVM_GUEST_PC_TRACE_REPORT_CHUNKS",
    "LZVM_GUEST_PC_TRACE_REPORT_CHUNK_CAPACITY",
    "LZVM_CUDA_GUEST_PC_OWNED_STREAMING_LOWER",
]

MODE_ENV = {
    "default": {},
    "pipeline": {
        "LZVM_GUEST_PC_TRACE_PARALLEL_LOWER": "1",
        "LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_WORKERS": "2",
        "LZVM_GUEST_PC_TRACE_SEGMENT_COMMIT_WORKERS": "1",
    },
    "work-units": {
        "LZVM_GUEST_PC_TRACE_PARALLEL_LOWER": "1",
        "LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_WORKERS": "2",
        "LZVM_GUEST_PC_TRACE_SEGMENT_COMMIT_WORKERS": "1",
        "LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_WORK_UNITS": "1",
    },
    "stream-pipeline": {
        "LZVM_GUEST_PC_TRACE_LIVE_REPORT_CHUNKS": "1",
        "LZVM_GUEST_PC_TRACE_LIVE_STREAM_START": "1",
        "LZVM_GUEST_PC_TRACE_PARALLEL_STREAM_CHUNKS": "1",
        "LZVM_GUEST_PC_TRACE_PARALLEL_LOWER": "1",
        "LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_WORKERS": "2",
    },
    "combined": {
        "LZVM_GUEST_PC_TRACE_PARALLEL_LOWER": "1",
        "LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_WORKERS": "2",
        "LZVM_GUEST_PC_TRACE_COMMIT_PIPELINE": "1",
    },
}


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


def shell_assign(name: str, value: str | Path) -> str:
    return f"{name}={shlex.quote(str(value))}"


def shell_arg(value: str | Path) -> str:
    return shlex.quote(str(value))


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


class ProofEnv:
    def __init__(self, prefix: str, label: str, default_trace_limit: str, root: Path):
        self.prefix = prefix
        self.label = label
        self.default_trace_limit = default_trace_limit
        self.root = root

    def var(self, suffix: str) -> str:
        return f"{self.prefix}_{suffix}"

    def available(self) -> bool:
        return all(os.environ.get(self.var(suffix)) for suffix in REQUIRED_SUFFIXES)

    def missing(self) -> list[str]:
        return [
            self.var(suffix)
            for suffix in REQUIRED_SUFFIXES
            if not os.environ.get(self.var(suffix))
        ]

    def path(self, suffix: str) -> Path:
        value = os.environ.get(self.var(suffix))
        if value is None:
            raise SystemExit(f"{self.var(suffix)} is required")
        path = resolve_workspace_path(value, self.root)
        if not path.exists():
            raise SystemExit(f"{self.var(suffix)} path does not exist: {path}")
        return path

    def optional_path(self, suffix: str, default: Path) -> Path:
        value = os.environ.get(self.var(suffix))
        path = resolve_workspace_path(value, self.root) if value else default
        return path

    def trace_limit(self) -> str:
        return os.environ.get(self.var("TRACE_LIMIT"), self.default_trace_limit)


def configured_paths(config: ProofEnv) -> dict[str, Path]:
    paths = {suffix.lower(): config.path(suffix) for suffix in REQUIRED_SUFFIXES}
    bin_path = config.optional_path("BIN", config.root / "target" / "release" / "lzvm")
    if not bin_path.exists():
        raise SystemExit(f"{config.var('BIN')} path does not exist: {bin_path}")
    paths["bin"] = bin_path
    paths["tmp_dir"] = require_workspace_temp_path(
        config.optional_path("TMP_DIR", config.root / "temp" / "tmp"),
        config.root,
        config.var("TMP_DIR"),
    )
    return paths


def command_for_env(config: ProofEnv, mode: str) -> str:
    paths = configured_paths(config)
    bin_path = paths["bin"]
    output_dir = f"{{batch_dir}}/{config.label}-{{run_padded}}.proof"

    parts = ["env"]
    for name in PIPELINE_ENV_TO_CLEAR:
        parts.extend(["-u", name])
    parts.append("TMPDIR={tmp_dir}")
    for name, value in MODE_ENV[mode].items():
        parts.append(shell_assign(name, value))
    parts.extend(
        [
            shell_arg(bin_path),
            "prove",
            "witness",
            "--guest-pc-trace",
            shell_arg(config.trace_limit()),
            "--timings",
            "--eth-block-input",
            shell_arg(paths["block_input"]),
            "--program-image-cache",
            shell_arg(paths["program_image_cache"]),
            "--input-data",
            shell_arg(paths["input_data"]),
            shell_arg(paths["setup"]),
            output_dir,
            shell_arg(paths["guest_image"]),
        ]
    )
    return " ".join(parts)


def selected_envs(args: argparse.Namespace, root: Path) -> list[tuple[ProofEnv, str]]:
    small = ProofEnv(SMALL_PREFIX, "small", "120000000", root)
    large = ProofEnv(LARGE_PREFIX, "large", "600000000", root)
    requested = {
        "small": [(small, args.small_mode)],
        "large": [(large, args.large_mode)],
        "both": [(small, args.small_mode), (large, args.large_mode)],
        "available": [],
    }
    if args.suite == "available":
        if small.available():
            requested["available"].append((small, args.small_mode))
        if large.available():
            requested["available"].append((large, args.large_mode))
    selected = requested[args.suite]
    if not selected:
        missing = small.missing() + large.missing()
        raise SystemExit("no proof environments available; missing " + ", ".join(missing))
    for config, _mode in selected:
        missing = config.missing()
        if missing:
            raise SystemExit(f"{config.label} proof environment is incomplete: {', '.join(missing)}")
    return selected


def runner_command(args: argparse.Namespace, root: Path) -> list[str]:
    runner = resolve_workspace_path(args.runner, root)
    if not runner.exists():
        raise SystemExit(f"runner script does not exist: {runner}")
    selected = selected_envs(args, root)
    work_dir = require_workspace_temp_path(
        resolve_workspace_path(args.work_dir, root),
        root,
        "--work-dir",
    )
    improve_log_path = require_workspace_temp_path(
        resolve_workspace_path(args.path, root),
        root,
        "--path",
    )
    command = [
        sys.executable,
        str(runner),
        "--runs",
        str(args.runs),
        "--small-timeout",
        str(args.small_timeout),
        "--large-timeout",
        str(args.large_timeout),
        "--work-dir",
        str(work_dir),
        "--path",
        str(improve_log_path),
        "--summary",
        args.summary,
        "--require-proof-output",
        "--max-relative-spread",
        str(args.max_relative_spread),
    ]
    if args.commit is not None:
        command.extend(["--commit", args.commit])
    for config, mode in selected:
        option = "--small-command" if config.label == "small" else "--large-command"
        command.extend([option, command_for_env(config, mode)])
    return command


def ensure_runtime_dirs(args: argparse.Namespace, root: Path) -> None:
    for config, _mode in selected_envs(args, root):
        configured_paths(config)["tmp_dir"].mkdir(parents=True, exist_ok=True)


def run(args: argparse.Namespace) -> int:
    root = workspace_root()
    if args.check_env:
        check_env(args, root)
        return 0
    if args.summary is None:
        raise SystemExit("--summary is required")
    command = runner_command(args, root)
    if args.dry_run:
        print("runner_command=" + shlex.join(command))
        for index, part in enumerate(command):
            if part in ("--small-command", "--large-command") and index + 1 < len(command):
                print(f"{part[2:].replace('-', '_')}={command[index + 1]}")
        return 0
    ensure_runtime_dirs(args, root)
    return subprocess.run(command, cwd=root).returncode


def check_env(args: argparse.Namespace, root: Path) -> None:
    selected = selected_envs(args, root)
    print("status=ok")
    for config, mode in selected:
        paths = configured_paths(config)
        print(f"{config.label}=ready")
        print(f"{config.label}_mode={mode}")
        print(f"{config.label}_trace_limit={config.trace_limit()}")
        for key in [
            "bin",
            "setup",
            "block_input",
            "program_image_cache",
            "input_data",
            "guest_image",
            "tmp_dir",
        ]:
            print(f"{config.label}_{key}={paths[key]}")


def self_test() -> None:
    root = workspace_root()
    work_dir = root / "temp" / f"eth-proof-timing-batch-self-test-{os.getpid()}"
    shutil.rmtree(work_dir, ignore_errors=True)
    work_dir.mkdir(parents=True)
    fake_bin = work_dir / "fake-prover.py"
    fake_bin.write_text(
        "\n".join(
            [
                "#!/usr/bin/env python3",
                "import os",
                "import sys",
                "tmp = os.environ.get('TMPDIR')",
                "if not tmp or not os.path.isdir(tmp):",
                "    sys.stderr.write('missing TMPDIR\\n')",
                "    sys.exit(9)",
                "label = os.environ.get('LZVM_TIMING_BATCH_LABEL', 'small')",
                "run = int(os.environ.get('LZVM_TIMING_BATCH_RUN', '1'))",
                "base = 1000 if label == 'small' else 2000",
                "print('status=ok')",
                "print('verify_outputs=true')",
                "print(f'timing_total_ms={base + run}')",
            ]
        )
        + "\n",
        encoding="utf-8",
    )
    fake_bin.chmod(0o755)
    for prefix in [SMALL_PREFIX, LARGE_PREFIX]:
        for suffix in REQUIRED_SUFFIXES:
            path = work_dir / f"{prefix.lower()}-{suffix.lower()}"
            if suffix == "SETUP":
                path.mkdir()
            else:
                path.write_bytes(b"fixture")
            os.environ[f"{prefix}_{suffix}"] = str(path)
        os.environ[f"{prefix}_BIN"] = str(fake_bin)
        os.environ[f"{prefix}_TMP_DIR"] = str(work_dir / "tmp")
    args = argparse.Namespace(
        commit="selftest",
        dry_run=False,
        large_mode="combined",
        large_timeout=10.0,
        max_relative_spread=0.10,
        path=str(work_dir / "improve-log.csv"),
        runner="scripts/run-proof-timing-batch.py",
        runs=3,
        small_mode="combined",
        small_timeout=10.0,
        suite="both",
        summary="self test",
        work_dir=str(work_dir / "runs"),
        check_env=False,
    )
    try:
        code = run(args)
        if code != 0:
            raise SystemExit(code)
        contents = (work_dir / "improve-log.csv").read_text(encoding="utf-8")
        if '"avg=1.002 samples=1.001;1.002;1.003 used=3/3"' not in contents:
            raise SystemExit("self-test small average missing")
        if '"avg=2.002 samples=2.001;2.002;2.003 used=3/3"' not in contents:
            raise SystemExit("self-test large average missing")
    finally:
        shutil.rmtree(work_dir, ignore_errors=True)


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Build ETH proof timing commands from real proof environment variables."
    )
    parser.add_argument("--suite", choices=["small", "large", "both", "available"], default="both")
    parser.add_argument("--small-mode", choices=sorted(MODE_ENV), default="combined")
    parser.add_argument("--large-mode", choices=sorted(MODE_ENV), default="combined")
    parser.add_argument("--runs", type=positive_run_count, default=3)
    parser.add_argument("--small-timeout", type=positive_timeout, default=60.0)
    parser.add_argument("--large-timeout", type=positive_timeout, default=180.0)
    parser.add_argument("--work-dir", default="temp/proof-timing-batch")
    parser.add_argument("--path", default="temp/improve-log.csv")
    parser.add_argument("--summary")
    parser.add_argument("--commit")
    parser.add_argument("--max-relative-spread", type=nonnegative_float, default=0.10)
    parser.add_argument("--runner", default="scripts/run-proof-timing-batch.py")
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("--check-env", action="store_true")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()

    if args.self_test:
        self_test()
    else:
        raise SystemExit(run(args))


if __name__ == "__main__":
    main()
