#!/usr/bin/env python3
import argparse
import json
import os
import re
import shlex
import shutil
import subprocess
import sys
from pathlib import Path
from typing import NamedTuple

SMALL_PREFIX = "LZVM_REAL_SMALL_PARITY"
LARGE_PREFIX = "LZVM_REAL_LARGE_PARITY"
DEFAULT_BIN_RELATIVE = Path("target/release/lzvm")
DEFAULT_BIN_BUILD_COMMAND = "cargo build --release -p lzvm-cli --bin lzvm --features cuda"
DEFAULT_RUNNER = "scripts/run-proof-timing-batch.py"
DEFAULT_ENV_TEMPLATE_PATH = "temp/real-proof.env"
DEFAULT_PROFILE_OUTPUT_DIR = "temp/proof-profiles"
DEFAULT_PROFILE_TOOL = "nsys"
DEFAULT_NSYS_TRACE = "cuda,nvtx,osrt"
DEFAULT_NCU_SET = "basic"
DEFAULT_NCU_TARGET_PROCESSES = "all"
DEFAULT_NVIDIA_SMI_COMMAND = "nvidia-smi"
DEFAULT_MIN_GPU_FREE_MIB = 1024
DEFAULT_EXTRA_RUN_BUDGET = 2
ENV_NAME_RE = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*$")

REQUIRED_PATHS = [
    ("SETUP", "dir"),
    ("BLOCK_INPUT", "file"),
    ("PROGRAM_IMAGE_CACHE", "file"),
    ("INPUT_DATA", "file"),
    ("GUEST_IMAGE", "file"),
]
REQUIRED_SUFFIXES = [suffix for suffix, _kind in REQUIRED_PATHS]
REQUIRED_PATH_KINDS = dict(REQUIRED_PATHS)
ALLOWED_ENV_FILE_NAMES = {
    *(
        f"{prefix}_{suffix}"
        for prefix in (SMALL_PREFIX, LARGE_PREFIX)
        for suffix in (*REQUIRED_SUFFIXES, "BIN", "TRACE_LIMIT")
    ),
    "LZVM_NSYS_COMMAND",
    "LZVM_NCU_COMMAND",
    "LZVM_NVIDIA_SMI_COMMAND",
    "CUDA_VISIBLE_DEVICES",
}
VERIFY_REQUIRED_TEXTS = [
    "verify_proof_status=ok",
    "artifact_public_input_match=ok",
    "artifact_proof_match=ok",
    "eth_block_input_match=ok",
    "program_image_cache_match=ok",
    "framed_guest_input_match=ok",
    "pipeline_input_bindings=ok",
]
ARTIFACT_HELP_ITEMS = [
    (
        "setup",
        "Use an existing setup key directory, or run: target/release/lzvm setup generate-key [--backend cpu|cuda] <setup-dir>",
    ),
    (
        "block_input",
        "Write BLOCK_INPUT from block RLP, hex, RPC JSON, and optional receipts: target/release/lzvm eth write-block-input [--hex|--rpc-json] [--receipts <receipts-rlp>|--receipts-rpc-json <receipts-json>] <block> <out>",
    ),
    (
        "public_input_block",
        "Write BLOCK_INPUT from ETH public input: target/release/lzvm eth write-public-block-input [--allow-trailing] [--receipts-rpc-json <receipts-json>] <public-input> <out>",
    ),
    (
        "program_image_cache",
        "Write PROGRAM_IMAGE_CACHE: target/release/lzvm setup write-program-image-cache [--backend cpu|cuda] --setup-dir <setup-dir> <program-bin> <guest-image> <root-bin> <trace-rows> <trace-columns> <blowup-factor> <arity> <out-cache>",
    ),
    (
        "input_data",
        "INPUT_DATA must be framed guest stdin consumed by the guest image.",
    ),
    (
        "guest_image",
        "GUEST_IMAGE must be the guest executable used to produce the matching framed input.",
    ),
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
    "LZVM_GUEST_PC_TRACE_SEED_DISCOVERY",
    "LZVM_GUEST_PC_TRACE_SEED_DISCOVERY_STREAMING_DEVICE_LOWER",
    "LZVM_CUDA_GUEST_PC_OWNED_STREAMING_LOWER",
    "LZVM_GUEST_TRACE_DETAIL_TIMING",
    "LZVM_GUEST_TRACE_DETAIL_TIMING_SAMPLE_STRIDE",
    "LZVM_GUEST_TRACE_SHAPE_TIMING",
]

MODE_ENV = {
    "default": {},
    "pipeline": {
        "LZVM_GUEST_PC_TRACE_PARALLEL_LOWER": "1",
    },
    "work-units": {
        "LZVM_GUEST_PC_TRACE_PARALLEL_LOWER": "1",
        "LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_WORK_UNITS": "1",
    },
    "stream-pipeline": {
        "LZVM_GUEST_PC_TRACE_LIVE_REPORT_CHUNKS": "1",
        "LZVM_GUEST_PC_TRACE_LIVE_STREAM_START": "1",
        "LZVM_GUEST_PC_TRACE_PARALLEL_STREAM_CHUNKS": "1",
        "LZVM_GUEST_PC_TRACE_PARALLEL_LOWER": "1",
    },
    "combined": {
        "LZVM_GUEST_PC_TRACE_PARALLEL_LOWER": "1",
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


def reject_symlinked_output_path(path: Path, label: str) -> None:
    if path.is_symlink():
        raise SystemExit(f"{label} must not be a symlink: {path}")


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


def parse_env_file_assignment(line: str, path: Path, line_number: int) -> tuple[str, str] | None:
    try:
        parts = shlex.split(line, comments=True, posix=True)
    except ValueError as error:
        raise SystemExit(f"{path}:{line_number}: invalid env line: {error}") from error
    if not parts:
        return None
    if parts[0] == "export":
        parts = parts[1:]
    if len(parts) != 1 or "=" not in parts[0]:
        raise SystemExit(f"{path}:{line_number}: expected NAME=value or export NAME=value")
    name, value = parts[0].split("=", 1)
    if not ENV_NAME_RE.match(name):
        raise SystemExit(f"{path}:{line_number}: invalid env name: {name!r}")
    if name not in ALLOWED_ENV_FILE_NAMES:
        raise SystemExit(f"{path}:{line_number}: env name is not allowed in --env-file: {name}")
    return (name, value)


def clear_env_file_controlled_names() -> None:
    for name in ALLOWED_ENV_FILE_NAMES:
        os.environ.pop(name, None)


def load_env_file(path: Path, root: Path) -> None:
    path = require_workspace_temp_path(path, root, "--env-file")
    reject_symlinked_output_path(path, "--env-file")
    if not path.exists():
        command = shell_join(
            [
                "scripts/run-eth-proof-timing-batch.py",
                "--write-env-template",
                display_path_for_shell(path, root),
            ]
        )
        raise SystemExit(
            f"--env-file path does not exist: {path}; create a template with: {command}"
        )
    if not path.is_file():
        raise SystemExit(f"--env-file must be a file: {path}")
    clear_env_file_controlled_names()
    seen_names: dict[str, int] = {}
    with path.open(encoding="utf-8") as source:
        for line_number, line in enumerate(source, start=1):
            assignment = parse_env_file_assignment(line, path, line_number)
            if assignment is not None:
                name, value = assignment
                if name in seen_names:
                    raise SystemExit(
                        f"{path}:{line_number}: duplicate env name in --env-file: "
                        f"{name} first set on line {seen_names[name]}"
                    )
                seen_names[name] = line_number
                os.environ[name] = value


def shell_assign(name: str, value: str | Path) -> str:
    return f"{name}={shlex.quote(str(value))}"


def shell_arg(value: str | Path) -> str:
    return shlex.quote(str(value))


def shell_export(name: str, value: str | Path) -> str:
    if str(value) == "":
        return f"export {name}="
    return "export " + shell_assign(name, value)


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


def positive_mib(raw: str) -> int:
    try:
        value = int(raw)
    except ValueError as error:
        raise argparse.ArgumentTypeError(f"invalid MiB value: {raw!r}") from error
    if value <= 0:
        raise argparse.ArgumentTypeError("MiB value must be positive")
    return value


def positive_integer(raw: str) -> int:
    try:
        value = int(raw)
    except ValueError as error:
        raise argparse.ArgumentTypeError(f"invalid integer: {raw!r}") from error
    if value <= 0:
        raise argparse.ArgumentTypeError("value must be positive")
    return value


def nonnegative_float(raw: str) -> float:
    try:
        value = float(raw)
    except ValueError as error:
        raise argparse.ArgumentTypeError(f"invalid float: {raw!r}") from error
    if value < 0.0:
        raise argparse.ArgumentTypeError("value must be nonnegative")
    return value


def positive_integer_env(raw: str, name: str) -> str:
    try:
        value = int(raw)
    except ValueError as error:
        raise SystemExit(f"{name} must be a positive integer: {raw!r}") from error
    if value <= 0:
        raise SystemExit(f"{name} must be a positive integer: {raw!r}")
    return str(value)


def validate_framed_input_data(path: Path, name: str) -> None:
    total_size = path.stat().st_size
    if total_size == 0:
        raise SystemExit(f"{name} framed input is invalid: empty input")
    with path.open("rb") as source:
        offset = 0
        while offset < total_size:
            header = source.read(8)
            if len(header) < 8:
                raise SystemExit(
                    f"{name} framed input is invalid: truncated chunk length at offset "
                    f"{offset}: expected 8 bytes, found {len(header)}"
                )
            payload_len = int.from_bytes(header, "little")
            payload_offset = offset + 8
            payload_end = payload_offset + payload_len
            if payload_end > total_size:
                raise SystemExit(
                    f"{name} framed input is invalid: truncated chunk at offset {offset}: "
                    f"expected {payload_len} bytes, found {total_size - payload_offset}"
                )
            source.seek(payload_len, os.SEEK_CUR)
            next_offset = (payload_end + 7) // 8 * 8
            if next_offset > total_size:
                raise SystemExit(
                    f"{name} framed input is invalid: truncated chunk padding at offset "
                    f"{payload_end}: expected {next_offset - payload_end} bytes, "
                    f"found {total_size - payload_end}"
                )
            padding = source.read(next_offset - payload_end)
            if len(padding) < next_offset - payload_end:
                raise SystemExit(
                    f"{name} framed input is invalid: truncated chunk padding at offset "
                    f"{payload_end}: expected {next_offset - payload_end} bytes, "
                    f"found {len(padding)}"
                )
            if any(padding):
                raise SystemExit(
                    f"{name} framed input is invalid: nonzero chunk padding at offset "
                    f"{payload_end}"
                )
            offset = next_offset


def subprocess_diagnostic(result: subprocess.CompletedProcess[str]) -> str:
    text = "\n".join(
        part.strip() for part in [result.stderr, result.stdout] if part.strip()
    )
    if not text:
        return f"exit code {result.returncode}"
    lines = text.splitlines()
    return "; ".join(lines[:8])


def validate_artifact_with_cli_summary(
    bin_path: Path,
    artifact_path: Path,
    name: str,
    command_args: list[str],
    root: Path,
) -> None:
    result = subprocess.run(
        [str(bin_path), *command_args, str(artifact_path)],
        cwd=root,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if result.returncode != 0:
        diagnostic = subprocess_diagnostic(result)
        if "usage: lzvm <group> <command> [args]" in diagnostic:
            diagnostic += f"; rebuild binary with: {DEFAULT_BIN_BUILD_COMMAND}"
        raise SystemExit(
            f"{name} artifact is invalid: semantic summary failed: "
            f"{diagnostic}"
        )
    if "status=ok" not in result.stdout.splitlines():
        raise SystemExit(
            f"{name} artifact is invalid: semantic summary did not report status=ok"
        )


def framed_input_data(payload: bytes) -> bytes:
    data = len(payload).to_bytes(8, "little") + payload
    return data + b"\0" * ((8 - (len(data) % 8)) % 8)


def target_max_avg_s(args: argparse.Namespace, label: str) -> float | None:
    explicit = args.small_max_avg_s if label == "small" else args.large_max_avg_s
    if explicit is not None:
        return explicit
    if args.skip_targets:
        return None
    return 10.0 if label == "small" else 30.0


class ProofEnv:
    def __init__(
        self,
        prefix: str,
        label: str,
        default_trace_limit: str,
        root: Path,
        bin_override: str | None = None,
    ):
        self.prefix = prefix
        self.label = label
        self.default_trace_limit = default_trace_limit
        self.root = root
        self.bin_override = bin_override

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
        kind = REQUIRED_PATH_KINDS[suffix]
        if kind == "dir" and not path.is_dir():
            raise SystemExit(f"{self.var(suffix)} must be a directory: {path}")
        if kind == "file" and not path.is_file():
            raise SystemExit(f"{self.var(suffix)} must be a file: {path}")
        return path

    def trace_limit(self) -> str:
        return positive_integer_env(
            os.environ.get(self.var("TRACE_LIMIT"), self.default_trace_limit),
            self.var("TRACE_LIMIT"),
        )

    def bin_value(self) -> str | Path:
        if self.bin_override:
            return self.bin_override
        return os.environ.get(self.var("BIN"), DEFAULT_BIN_RELATIVE)


class GpuMemoryRow(NamedTuple):
    index: int
    uuid: str | None
    total: int
    used: int
    free: int


def configured_paths(config: ProofEnv) -> dict[str, Path]:
    paths = {suffix.lower(): config.path(suffix) for suffix in REQUIRED_SUFFIXES}
    validate_framed_input_data(paths["input_data"], config.var("INPUT_DATA"))
    bin_value = config.bin_override or os.environ.get(config.var("BIN"))
    bin_path = (
        resolve_workspace_path(bin_value, config.root)
        if bin_value
        else config.root / DEFAULT_BIN_RELATIVE
    )
    if not bin_path.exists():
        message = f"{config.var('BIN')} path does not exist: {bin_path}"
        if not bin_value:
            message += f"; build default binary with: {DEFAULT_BIN_BUILD_COMMAND}"
        raise SystemExit(message)
    if not bin_path.is_file():
        raise SystemExit(f"{config.var('BIN')} must be a file: {bin_path}")
    if not os.access(bin_path, os.X_OK):
        raise SystemExit(f"{config.var('BIN')} must be executable: {bin_path}")
    paths["bin"] = bin_path
    validate_artifact_with_cli_summary(
        bin_path,
        paths["block_input"],
        config.var("BLOCK_INPUT"),
        ["eth", "block-input-summary"],
        config.root,
    )
    validate_artifact_with_cli_summary(
        bin_path,
        paths["program_image_cache"],
        config.var("PROGRAM_IMAGE_CACHE"),
        ["setup", "program-image-cache-summary"],
        config.root,
    )
    return paths


def proof_envs(args: argparse.Namespace, root: Path) -> tuple[ProofEnv, ProofEnv]:
    return (
        ProofEnv(SMALL_PREFIX, "small", "120000000", root, args.small_bin),
        ProofEnv(LARGE_PREFIX, "large", "600000000", root, args.large_bin),
    )


def template_envs(args: argparse.Namespace, root: Path) -> list[tuple[ProofEnv, str]]:
    small, large = proof_envs(args, root)
    requested = {
        "small": [(small, args.small_mode)],
        "large": [(large, args.large_mode)],
        "both": [(small, args.small_mode), (large, args.large_mode)],
        "available": [(small, args.small_mode), (large, args.large_mode)],
    }
    return requested[args.suite]


def env_template_text(args: argparse.Namespace, root: Path) -> str:
    visible_devices = os.environ.get("CUDA_VISIBLE_DEVICES")
    lines = [
        "# build default binary first:",
        f"# {DEFAULT_BIN_BUILD_COMMAND}",
        "",
        *artifact_template_help_lines(),
    ]
    if visible_devices:
        lines.extend(
            [
                "# GPU selection captured from the current environment",
                shell_export("CUDA_VISIBLE_DEVICES", visible_devices),
                "",
            ]
        )
    else:
        lines.extend(
            [
                "# optional GPU selection for reproducible timing and profiling",
                "# export CUDA_VISIBLE_DEVICES=0",
                "",
            ]
        )
    for index, (config, mode) in enumerate(template_envs(args, root)):
        if index:
            lines.append("")
        lines.append(f"# {config.label} suite")
        lines.append(f"# run with --{config.label}-mode {mode}")
        lines.append(
            shell_export(
                config.var("BIN"),
                config.bin_value(),
            )
        )
        for suffix in REQUIRED_SUFFIXES:
            if suffix == "INPUT_DATA":
                lines.append("# INPUT_DATA must be framed guest stdin")
            lines.append(
                shell_export(config.var(suffix), os.environ.get(config.var(suffix), ""))
            )
        lines.append(
            shell_export(
                config.var("TRACE_LIMIT"),
                os.environ.get(config.var("TRACE_LIMIT"), config.default_trace_limit),
            )
        )
    return "\n".join(lines) + "\n"


def print_env_template(args: argparse.Namespace, root: Path) -> None:
    sys.stdout.write(env_template_text(args, root))


def display_path_for_shell(path: Path, root: Path) -> str:
    resolved = path.resolve(strict=False)
    try:
        return str(resolved.relative_to(root.resolve(strict=False)))
    except ValueError:
        return str(resolved)


def mode_args(args: argparse.Namespace) -> list[str]:
    result = ["--small-mode", args.small_mode, "--large-mode", args.large_mode]
    if args.skip_verify_proof:
        result.append("--skip-verify-proof")
    if args.seed_discovery:
        result.append("--seed-discovery")
    if args.seed_discovery_streaming_device_lower:
        result.append("--seed-discovery-streaming-device-lower")
    if args.owned_streaming_lower:
        result.append("--owned-streaming-lower")
    if args.trace_shape_timing:
        result.append("--trace-shape-timing")
    if trace_detail_timing_enabled(args):
        result.append("--trace-detail-timing")
    if args.trace_detail_timing_sample_stride is not None:
        result.extend(
            [
                "--trace-detail-timing-sample-stride",
                str(args.trace_detail_timing_sample_stride),
            ]
        )
    return result


def trace_detail_timing_enabled(args: argparse.Namespace) -> bool:
    return bool(args.trace_detail_timing or args.trace_detail_timing_sample_stride is not None)


def trace_timing_env_for_args(args: argparse.Namespace) -> dict[str, str]:
    env: dict[str, str] = {}
    if args.trace_shape_timing:
        env["LZVM_GUEST_TRACE_SHAPE_TIMING"] = "1"
    if trace_detail_timing_enabled(args):
        env["LZVM_GUEST_TRACE_DETAIL_TIMING"] = "1"
    if args.trace_detail_timing_sample_stride is not None:
        env["LZVM_GUEST_TRACE_DETAIL_TIMING_SAMPLE_STRIDE"] = str(
            args.trace_detail_timing_sample_stride
        )
    return env


def mode_env_for_args(args: argparse.Namespace, mode: str) -> dict[str, str]:
    mode_env = dict(MODE_ENV[mode])
    if args.seed_discovery:
        mode_env["LZVM_GUEST_PC_TRACE_SEED_DISCOVERY"] = "1"
    if args.seed_discovery_streaming_device_lower:
        mode_env["LZVM_GUEST_PC_TRACE_SEED_DISCOVERY_STREAMING_DEVICE_LOWER"] = "1"
    if args.owned_streaming_lower:
        mode_env["LZVM_CUDA_GUEST_PC_OWNED_STREAMING_LOWER"] = "1"
    mode_env.update(trace_timing_env_for_args(args))
    if args.parallel_lower_workers is not None:
        mode_env["LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_WORKERS"] = str(
            args.parallel_lower_workers
        )
    if args.parallel_lower_job_queue is not None:
        mode_env["LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_JOB_QUEUE"] = str(
            args.parallel_lower_job_queue
        )
    if args.segment_commit_workers is not None:
        mode_env["LZVM_GUEST_PC_TRACE_SEGMENT_COMMIT_WORKERS"] = str(
            args.segment_commit_workers
        )
    return mode_env


def proof_tuning_args(args: argparse.Namespace) -> list[str]:
    parts: list[str] = []
    if args.gpu_preallocate:
        parts.append("--gpu-preallocate")
    if args.minimal_memory:
        parts.append("--minimal-memory")
    if args.no_pack_trace:
        parts.append("--no-pack-trace")
    if args.gpu_streams is not None:
        parts.extend(["--gpu-streams", str(args.gpu_streams)])
    if args.witness_thread_pools is not None:
        parts.extend(["--witness-thread-pools", str(args.witness_thread_pools)])
    if args.stored_witnesses is not None:
        parts.extend(["--stored-witnesses", str(args.stored_witnesses)])
    return parts


def effective_max_runs(args: argparse.Namespace) -> int:
    if args.max_runs is not None:
        return args.max_runs
    return args.runs + DEFAULT_EXTRA_RUN_BUDGET


def next_command_parts(
    args: argparse.Namespace,
    root: Path,
    env_file: Path | str | None = None,
) -> list[str]:
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
    parts = [
        "scripts/run-eth-proof-timing-batch.py",
        "--suite",
        args.suite,
        *mode_args(args),
        "--runs",
        str(args.runs),
    ]
    env_file_value = env_file if env_file is not None else args.env_file
    if env_file_value is not None:
        env_file_path = require_workspace_temp_path(
            resolve_workspace_path(str(env_file_value), root),
            root,
            "--env-file",
        )
        parts.extend(["--env-file", display_path_for_shell(env_file_path, root)])
    if args.small_bin is not None:
        small_bin = display_path_for_shell(resolve_workspace_path(args.small_bin, root), root)
        parts.extend(["--small-bin", small_bin])
    if args.large_bin is not None:
        large_bin = display_path_for_shell(resolve_workspace_path(args.large_bin, root), root)
        parts.extend(["--large-bin", large_bin])
    parts.extend(["--max-runs", str(effective_max_runs(args))])
    parts.extend(
        [
            "--small-timeout",
            str(args.small_timeout),
            "--large-timeout",
            str(args.large_timeout),
            "--max-relative-spread",
            str(args.max_relative_spread),
            "--work-dir",
            display_path_for_shell(work_dir, root),
            "--path",
            display_path_for_shell(improve_log_path, root),
        ]
    )
    if args.runner != DEFAULT_RUNNER:
        runner = display_path_for_shell(resolve_workspace_path(args.runner, root), root)
        parts.extend(["--runner", runner])
    parts.extend(profile_tool_cli_parts(args, root, "nsys"))
    parts.extend(profile_tool_cli_parts(args, root, "ncu"))
    if args.nsys_trace != DEFAULT_NSYS_TRACE:
        parts.extend(["--nsys-trace", args.nsys_trace])
    if args.ncu_set != DEFAULT_NCU_SET:
        parts.extend(["--ncu-set", args.ncu_set])
    if args.ncu_target_processes != DEFAULT_NCU_TARGET_PROCESSES:
        parts.extend(["--ncu-target-processes", args.ncu_target_processes])
    if args.skip_nsys_export:
        parts.append("--skip-nsys-export")
    if args.profile_output_dir != DEFAULT_PROFILE_OUTPUT_DIR:
        profile_output_dir = require_workspace_temp_path(
            resolve_workspace_path(args.profile_output_dir, root),
            root,
            "--profile-output-dir",
        )
        parts.extend(["--profile-output-dir", display_path_for_shell(profile_output_dir, root)])
    if args.profile_tool != DEFAULT_PROFILE_TOOL:
        parts.extend(["--profile-tool", args.profile_tool])
    parts.extend(profile_arg_cli_parts(args))
    if args.check_gpu_memory:
        parts.append("--check-gpu-memory")
    if args.min_gpu_free_mib != DEFAULT_MIN_GPU_FREE_MIB:
        parts.extend(["--min-gpu-free-mib", str(args.min_gpu_free_mib)])
    parts.extend(gpu_memory_cli_parts(args, root))
    if args.enforce_targets:
        parts.append("--enforce-targets")
    if args.skip_targets:
        parts.append("--skip-targets")
    if args.small_max_avg_s is not None:
        parts.extend(["--small-max-avg-s", str(args.small_max_avg_s)])
    if args.large_max_avg_s is not None:
        parts.extend(["--large-max-avg-s", str(args.large_max_avg_s)])
    if args.append_max_average_rejections:
        parts.append("--append-max-average-rejections")
    if args.parallel_lower_workers is not None:
        parts.extend(["--parallel-lower-workers", str(args.parallel_lower_workers)])
    if args.parallel_lower_job_queue is not None:
        parts.extend(["--parallel-lower-job-queue", str(args.parallel_lower_job_queue)])
    if args.segment_commit_workers is not None:
        parts.extend(["--segment-commit-workers", str(args.segment_commit_workers)])
    parts.extend(proof_tuning_args(args))
    if args.commit is not None:
        parts.extend(["--commit", args.commit])
    return parts


def shell_join(parts: list[str | Path]) -> str:
    return " ".join(shell_arg(part) for part in parts)


def next_followup_commands(
    args: argparse.Namespace,
    root: Path,
    env_file: Path | str | None = None,
) -> dict[str, str]:
    base_parts = next_command_parts(args, root, env_file=env_file)
    return {
        "next_check_command": shell_join([*base_parts, "--check-env"]),
        "next_profile_tool_check_command": shell_join(
            [*base_parts, "--check-profile-tools"]
        ),
        "next_preflight_command": shell_join(
            [*base_parts, "--check-env", "--check-profile-tools"]
        ),
        "next_profile_command": shell_join([*base_parts, "--print-profile-commands"]),
        "next_run_command": shell_join([*base_parts, "--summary", "real proof timing"]),
    }


def write_env_template(args: argparse.Namespace, root: Path) -> None:
    path = require_workspace_temp_path(
        resolve_workspace_path(args.write_env_template, root),
        root,
        "--write-env-template",
    )
    env_path = display_path_for_shell(path, root)
    followup_commands = next_followup_commands(args, root, env_file=path)
    path.parent.mkdir(parents=True, exist_ok=True)
    reject_symlinked_output_path(path, "--write-env-template")
    write_text_no_follow(path, env_template_text(args, root))

    print(f"env_template={env_path}")
    for key in [
        "next_check_command",
        "next_profile_tool_check_command",
        "next_preflight_command",
        "next_profile_command",
        "next_run_command",
    ]:
        print(f"{key}={followup_commands[key]}")


def append_cleared_pipeline_env(parts: list[str]) -> None:
    parts.append("env")
    for name in PIPELINE_ENV_TO_CLEAR:
        parts.extend(["-u", name])


def append_cuda_visible_devices_assignment(parts: list[str]) -> None:
    visible_devices = os.environ.get("CUDA_VISIBLE_DEVICES")
    if visible_devices:
        parts.append(shell_assign("CUDA_VISIBLE_DEVICES", visible_devices))


def command_for_env(
    config: ProofEnv,
    mode: str,
    verify_proof: bool,
    mode_env: dict[str, str] | None = None,
    proof_args: list[str] | None = None,
) -> str:
    paths = configured_paths(config)
    bin_path = paths["bin"]
    output_dir = f"{{batch_dir}}/{config.label}-{{run_padded}}.proof"
    proof_path = f"{output_dir}/proof.bin"
    public_values_path = f"{output_dir}/eth-block-public-values.bin"

    parts: list[str] = []
    append_cleared_pipeline_env(parts)
    parts.append("TMPDIR={tmp_dir}")
    append_cuda_visible_devices_assignment(parts)
    for name, value in (mode_env or MODE_ENV[mode]).items():
        parts.append(shell_assign(name, value))
    parts.extend(
        [
            shell_arg(bin_path),
            "prove",
            "witness",
            "--guest-pc-trace",
            shell_arg(config.trace_limit()),
            "--timings",
            *(proof_args or []),
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
    if verify_proof:
        parts.append("&&")
        append_cleared_pipeline_env(parts)
        parts.append("TMPDIR={tmp_dir}")
        append_cuda_visible_devices_assignment(parts)
        parts.extend(
            [
                shell_arg(bin_path),
                "verify",
                "proof",
                "--eth-block-input",
                shell_arg(paths["block_input"]),
                "--program-image-cache",
                shell_arg(paths["program_image_cache"]),
                "--input-data",
                shell_arg(paths["input_data"]),
                shell_arg(paths["setup"]),
                proof_path,
                public_values_path,
                "&&",
                "printf",
                shell_arg("verify_proof_status=ok\n"),
            ]
        )
    return " ".join(parts)


def expand_profile_command_template(
    command: str,
    label: str,
    batch_dir: Path,
    tmp_dir: Path,
    cwd: Path,
) -> str:
    replacements = {
        "{label}": label,
        "{run}": "1",
        "{run_padded}": "profile",
        "{runs}": "1",
        "{max_runs}": "1",
        "{batch_dir}": shell_arg(batch_dir),
        "{tmp_dir}": shell_arg(tmp_dir),
        "{cwd}": shell_arg(cwd),
    }
    expanded = command
    for needle, value in replacements.items():
        expanded = expanded.replace(needle, value)
    return expanded


def selected_profile_tools(args: argparse.Namespace) -> list[str]:
    if args.profile_tool == "both":
        return ["nsys", "ncu"]
    return [args.profile_tool]


def profile_tool_spec(args: argparse.Namespace, tool: str) -> tuple[str, str]:
    if tool == "nsys":
        if args.nsys_command is not None:
            return ("arg", args.nsys_command)
        env_value = os.environ.get("LZVM_NSYS_COMMAND")
        if env_value:
            return ("env", env_value)
        return ("path", "nsys")
    if args.ncu_command is not None:
        return ("arg", args.ncu_command)
    env_value = os.environ.get("LZVM_NCU_COMMAND")
    if env_value:
        return ("env", env_value)
    return ("path", "ncu")


def profile_tool_command_arg(raw: str, root: Path) -> str:
    path = Path(raw)
    if path.is_absolute() or len(path.parts) > 1:
        return display_path_for_shell(resolve_workspace_path(raw, root), root)
    return raw


def profile_tool_cli_parts(
    args: argparse.Namespace, root: Path, tool: str
) -> list[str]:
    source, raw = profile_tool_spec(args, tool)
    if source == "path":
        return []
    option = "--nsys-command" if tool == "nsys" else "--ncu-command"
    return [option, profile_tool_command_arg(raw, root)]


def profile_arg_cli_parts(args: argparse.Namespace) -> list[str]:
    return [f"--profile-arg={value}" for value in args.profile_arg]


def resolve_profile_tool(raw: str, root: Path) -> Path | None:
    path = Path(raw)
    candidates: list[Path] = []
    if path.is_absolute():
        candidates.append(path)
    elif len(path.parts) > 1:
        candidates.append(root / path)
    else:
        found = shutil.which(raw)
        if found is not None:
            candidates.append(Path(found))
    for candidate in candidates:
        if candidate.is_file() and os.access(candidate, os.X_OK):
            return candidate
    return None


def gpu_memory_tool_spec(args: argparse.Namespace) -> tuple[str, str]:
    if args.nvidia_smi_command is not None:
        return ("arg", args.nvidia_smi_command)
    env_value = os.environ.get("LZVM_NVIDIA_SMI_COMMAND")
    if env_value:
        return ("env", env_value)
    return ("path", DEFAULT_NVIDIA_SMI_COMMAND)


def gpu_memory_cli_parts(args: argparse.Namespace, root: Path) -> list[str]:
    source, raw = gpu_memory_tool_spec(args)
    if source == "path":
        return []
    return ["--nvidia-smi-command", profile_tool_command_arg(raw, root)]


def cuda_visible_devices_command_prefix() -> list[str]:
    visible_devices = os.environ.get("CUDA_VISIBLE_DEVICES")
    if not visible_devices:
        return []
    return ["env", f"CUDA_VISIBLE_DEVICES={visible_devices}"]


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


def print_gpu_memory_check(args: argparse.Namespace, root: Path) -> bool:
    source, raw = gpu_memory_tool_spec(args)
    resolved = resolve_profile_tool(raw, root)
    print(f"gpu_memory_source={source}")
    print(f"gpu_memory_command={raw}")
    print(f"gpu_memory_min_free_mib={args.min_gpu_free_mib}")
    visible_devices = os.environ.get("CUDA_VISIBLE_DEVICES")
    if visible_devices is not None:
        print(f"gpu_memory_cuda_visible_devices={visible_devices}")
    if resolved is None:
        print("gpu_memory_status=missing")
        return False
    print(f"gpu_memory_resolved={display_path_for_shell(resolved, root)}")
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
        print("gpu_memory_status=query_failed")
        print(f"gpu_memory_error={error}")
        return False
    if result.returncode != 0:
        print("gpu_memory_status=query_failed")
        print(f"gpu_memory_exit_code={result.returncode}")
        diagnostic = first_diagnostic_line(result.stderr)
        if diagnostic:
            print(f"gpu_memory_error={diagnostic}")
        return False
    try:
        rows = parse_gpu_memory_rows(result.stdout)
    except ValueError as error:
        print("gpu_memory_status=parse_failed")
        print(f"gpu_memory_error={error}")
        return False
    try:
        selected = select_default_cuda_gpu_memory_row(rows)
    except ValueError as error:
        print("gpu_memory_status=device_unavailable")
        print(f"gpu_memory_error={error}")
        return False
    print(f"gpu_memory_device_count={len(rows)}")
    print(f"gpu_memory_selected_index={selected.index}")
    if selected.uuid is not None:
        print(f"gpu_memory_selected_uuid={selected.uuid}")
    print(f"gpu_memory_total_mib={selected.total}")
    print(f"gpu_memory_used_mib={selected.used}")
    print(f"gpu_memory_free_mib={selected.free}")
    ready = selected.free >= args.min_gpu_free_mib
    print(f"gpu_memory_status={'ready' if ready else 'low'}")
    return ready


def print_profile_tool_checks(args: argparse.Namespace, root: Path) -> bool:
    profile_output_dir = require_workspace_temp_path(
        resolve_workspace_path(args.profile_output_dir, root),
        root,
        "--profile-output-dir",
    )
    print(f"profile_output_dir={display_path_for_shell(profile_output_dir, root)}")
    print(f"profile_tool={args.profile_tool}")
    all_ready = True
    for tool in selected_profile_tools(args):
        source, raw = profile_tool_spec(args, tool)
        resolved = resolve_profile_tool(raw, root)
        ready = resolved is not None
        all_ready = all_ready and ready
        print(f"{tool}_profiler_source={source}")
        print(f"{tool}_profiler_command={raw}")
        print(f"{tool}_profiler_status={'ready' if ready else 'missing'}")
        if resolved is not None:
            print(f"{tool}_profiler_resolved={display_path_for_shell(resolved, root)}")
    return all_ready


def profile_command_for_env(
    args: argparse.Namespace,
    root: Path,
    config: ProofEnv,
    mode: str,
    tool: str,
) -> list[str]:
    profile_root = require_workspace_temp_path(
        resolve_workspace_path(args.profile_output_dir, root),
        root,
        "--profile-output-dir",
    )
    profile_name = f"{config.label}-{mode}"
    profile_dir = profile_root / profile_name
    batch_dir = profile_dir / "run"
    tmp_dir = profile_dir / "tmp"
    profile_output_dir = profile_dir / tool
    proof_command = expand_profile_command_template(
        command_for_env(
            config,
            mode,
            not args.skip_verify_proof,
            mode_env_for_args(args, mode),
            proof_tuning_args(args),
        ),
        config.label,
        batch_dir,
        tmp_dir,
        root,
    )
    summary_args = (
        ["--proof-timing-summary", "--require-proof-timing-summary"]
        if tool == "nsys" and args.skip_nsys_export
        else ["--summarize", "--require-proof-timing-summary"]
    )
    profiler_command: list[str] = []
    profiler_command.extend(profile_tool_cli_parts(args, root, tool))
    if tool == "nsys" and args.nsys_trace != DEFAULT_NSYS_TRACE:
        profiler_command.extend(["--nsys-trace", args.nsys_trace])
    if tool == "nsys" and args.skip_nsys_export:
        profiler_command.append("--skip-nsys-export")
    if tool == "ncu" and args.ncu_set != DEFAULT_NCU_SET:
        profiler_command.extend(["--ncu-set", args.ncu_set])
    if tool == "ncu" and args.ncu_target_processes != DEFAULT_NCU_TARGET_PROCESSES:
        profiler_command.extend(["--ncu-target-processes", args.ncu_target_processes])
    if args.check_gpu_memory:
        profiler_command.append("--check-gpu-memory")
    if args.min_gpu_free_mib != DEFAULT_MIN_GPU_FREE_MIB:
        profiler_command.extend(["--min-gpu-free-mib", str(args.min_gpu_free_mib)])
    profiler_command.extend(gpu_memory_cli_parts(args, root))
    return [
        *cuda_visible_devices_command_prefix(),
        "scripts/run-proof-profile.py",
        "--tool",
        tool,
        "--output-dir",
        display_path_for_shell(profile_output_dir, root),
        "--name",
        profile_name,
        *summary_args,
        "--cwd",
        ".",
        *profiler_command,
        *profile_arg_cli_parts(args),
        "--",
        "sh",
        "-lc",
        proof_command,
    ]


def print_profile_commands(args: argparse.Namespace, root: Path) -> None:
    selected = selected_envs(args, root)
    for config, mode in selected:
        for tool in selected_profile_tools(args):
            key = f"{config.label}_{tool}_profile_command"
            command = profile_command_for_env(args, root, config, mode, tool)
            print(f"{key}={shell_join(command)}")


def env_template_command_for_missing_config(args: argparse.Namespace, root: Path) -> str:
    if args.env_file is None:
        template_value = DEFAULT_ENV_TEMPLATE_PATH
    else:
        env_path = resolve_workspace_path(args.env_file, root)
        if env_path.suffix:
            template_value = env_path.with_name(
                f"{env_path.stem}.template{env_path.suffix}"
            )
        else:
            template_value = env_path.with_name(env_path.name + ".template.env")
    template_path = require_workspace_temp_path(
        resolve_workspace_path(template_value, root),
        root,
        "--write-env-template",
    )
    return shell_join(
        [
            *next_command_parts(args, root),
            "--write-env-template",
            display_path_for_shell(template_path, root),
        ]
    )


def artifact_help_text() -> str:
    return "\n".join(
        f"artifact_help_{name}={text}" for name, text in ARTIFACT_HELP_ITEMS
    )


def artifact_template_help_lines() -> list[str]:
    return [
        "# artifact helpers:",
        *(f"# {text}" for _name, text in ARTIFACT_HELP_ITEMS),
        "",
    ]


def selected_envs(args: argparse.Namespace, root: Path) -> list[tuple[ProofEnv, str]]:
    small, large = proof_envs(args, root)
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
        raise SystemExit(
            "no proof environments available; missing "
            + ", ".join(missing)
            + "\n"
            + artifact_help_text()
            + "\nnext_env_template_command="
            + env_template_command_for_missing_config(args, root)
        )
    missing_configs = []
    for config, _mode in selected:
        missing = config.missing()
        if missing:
            missing_configs.append(
                f"{config.label} proof environment is incomplete: {', '.join(missing)}"
            )
    if missing_configs:
        raise SystemExit(
            "\n".join(missing_configs)
            + "\n"
            + artifact_help_text()
            + "\nnext_env_template_command="
            + env_template_command_for_missing_config(args, root)
        )
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
        "--max-runs",
        str(effective_max_runs(args)),
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
    if not args.skip_verify_proof:
        for required_text in VERIFY_REQUIRED_TEXTS:
            command.extend(["--require-text", required_text])
    if args.commit is not None:
        command.extend(["--commit", args.commit])
    selected_labels = {config.label for config, _mode in selected}
    if "small" in selected_labels:
        small_max_avg_s = target_max_avg_s(args, "small")
        if small_max_avg_s is not None:
            command.extend(["--small-max-avg-s", str(small_max_avg_s)])
    if "large" in selected_labels:
        large_max_avg_s = target_max_avg_s(args, "large")
        if large_max_avg_s is not None:
            command.extend(["--large-max-avg-s", str(large_max_avg_s)])
    if args.append_max_average_rejections:
        command.append("--append-max-average-rejections")
    for config, mode in selected:
        option = "--small-command" if config.label == "small" else "--large-command"
        command.extend(
            [
                option,
                command_for_env(
                    config,
                    mode,
                    not args.skip_verify_proof,
                    mode_env_for_args(args, mode),
                    proof_tuning_args(args),
                ),
            ]
        )
    return command


def dry_run_summary_lines(args: argparse.Namespace, root: Path) -> list[str]:
    selected = selected_envs(args, root)
    selected_labels = [config.label for config, _mode in selected]
    lines = [
        f"suite={args.suite}",
        f"selected={','.join(selected_labels)}",
        f"runs={args.runs}",
        f"max_runs={effective_max_runs(args)}",
        f"verify_proof={str(not args.skip_verify_proof).lower()}",
        f"parallel_lower_workers={args.parallel_lower_workers or ''}",
        f"parallel_lower_job_queue={args.parallel_lower_job_queue or ''}",
        f"segment_commit_workers={args.segment_commit_workers or ''}",
        f"seed_discovery={str(args.seed_discovery).lower()}",
        "seed_discovery_streaming_device_lower="
        f"{str(args.seed_discovery_streaming_device_lower).lower()}",
        f"owned_streaming_lower={str(args.owned_streaming_lower).lower()}",
        f"gpu_preallocate={str(args.gpu_preallocate).lower()}",
        f"minimal_memory={str(args.minimal_memory).lower()}",
        f"pack_trace={str(not args.no_pack_trace).lower()}",
        f"gpu_streams={args.gpu_streams or ''}",
        f"witness_thread_pools={args.witness_thread_pools or ''}",
        f"stored_witnesses={args.stored_witnesses or ''}",
        f"trace_shape_timing={str(args.trace_shape_timing).lower()}",
        f"trace_detail_timing={str(trace_detail_timing_enabled(args)).lower()}",
        f"trace_detail_timing_sample_stride={args.trace_detail_timing_sample_stride or ''}",
        f"append_max_average_rejections={str(args.append_max_average_rejections).lower()}",
    ]
    for config, mode in selected:
        max_avg_s = target_max_avg_s(args, config.label)
        lines.append(f"{config.label}_mode={mode}")
        lines.append(
            f"{config.label}_target_max_avg_s={max_avg_s if max_avg_s is not None else ''}"
        )
    return lines


def run(args: argparse.Namespace) -> int:
    root = workspace_root()
    if args.enforce_targets and args.skip_targets:
        raise SystemExit("--skip-targets conflicts with --enforce-targets")
    if effective_max_runs(args) < args.runs:
        raise SystemExit("--max-runs must be at least --runs")
    if (
        args.env_file is not None
        and (args.print_env_template or args.write_env_template is not None)
    ):
        env_file_path = resolve_workspace_path(args.env_file, root)
        if env_file_path.exists():
            load_env_file(env_file_path, root)
    if args.print_env_template:
        print_env_template(args, root)
    if args.write_env_template is not None:
        write_env_template(args, root)
    if args.print_env_template or args.write_env_template is not None:
        return 0
    if args.env_file is not None:
        load_env_file(resolve_workspace_path(args.env_file, root), root)
    if args.check_env:
        check_env(args, root, require_profile_tools=args.check_profile_tools)
        return 0
    if args.check_profile_tools:
        profile_ready = print_profile_tool_checks(args, root)
        gpu_ready = True
        if args.check_gpu_memory:
            gpu_ready = print_gpu_memory_check(args, root)
        return 0 if profile_ready and gpu_ready else 1
    if args.print_profile_commands:
        if args.check_gpu_memory and not print_gpu_memory_check(args, root):
            return 1
        print_profile_commands(args, root)
        return 0
    if args.summary is None:
        if args.check_gpu_memory:
            return 0 if print_gpu_memory_check(args, root) else 1
        raise SystemExit("--summary is required")
    if args.check_gpu_memory and not print_gpu_memory_check(args, root):
        return 1
    command = runner_command(args, root)
    if args.dry_run:
        print("runner_command=" + shlex.join(command))
        for line in dry_run_summary_lines(args, root):
            print(line)
        for index, part in enumerate(command):
            if part in ("--small-command", "--large-command") and index + 1 < len(command):
                print(f"{part[2:].replace('-', '_')}={command[index + 1]}")
        return 0
    return subprocess.run(command, cwd=root).returncode


def check_env(
    args: argparse.Namespace, root: Path, require_profile_tools: bool = False
) -> None:
    profile_ready = print_profile_tool_checks(args, root)
    gpu_ready = True
    if args.check_gpu_memory:
        gpu_ready = print_gpu_memory_check(args, root)
    selected = selected_envs(args, root)
    ready = [
        (config, mode, configured_paths(config), config.trace_limit())
        for config, mode in selected
    ]
    if require_profile_tools and not profile_ready:
        raise SystemExit("profile tool preflight failed")
    if not gpu_ready:
        raise SystemExit("GPU memory preflight failed")
    print("status=ok")
    for config, mode, paths, trace_limit in ready:
        print(f"{config.label}=ready")
        print(f"{config.label}_mode={mode}")
        print(f"{config.label}_verify_proof={str(not args.skip_verify_proof).lower()}")
        if not args.skip_verify_proof:
            for required_text in VERIFY_REQUIRED_TEXTS:
                print(f"{config.label}_verify_required_text={required_text}")
        print(f"{config.label}_trace_limit={trace_limit}")
        for key in [
            "bin",
            "setup",
            "block_input",
            "program_image_cache",
            "input_data",
            "guest_image",
        ]:
            print(f"{config.label}_{key}={paths[key]}")
    followup_commands = next_followup_commands(args, root)
    for key in [
        "next_preflight_command",
        "next_profile_command",
        "next_run_command",
    ]:
        print(f"{key}={followup_commands[key]}")


def self_test() -> None:
    root = workspace_root()
    work_dir = root / "temp" / f"eth-proof-timing-batch-self-test-{os.getpid()}"
    shutil.rmtree(work_dir, ignore_errors=True)
    work_dir.mkdir(parents=True)
    empty_env = work_dir / "empty.env"
    empty_env.write_text("", encoding="utf-8")
    inherited_env = {
        name: value
        for name, value in os.environ.items()
        if not name.startswith("LZVM_REAL_")
        and name
        not in {
            "LZVM_NSYS_COMMAND",
            "LZVM_NCU_COMMAND",
            "LZVM_NVIDIA_SMI_COMMAND",
        }
    }
    missing_result = subprocess.run(
        [
            sys.executable,
            str(Path(__file__).resolve()),
            "--suite",
            "both",
            "--env-file",
            str(empty_env),
            "--check-env",
        ],
        cwd=root,
        env=inherited_env,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if missing_result.returncode == 0:
        raise SystemExit("self-test missing environment preflight should fail")
    if "small proof environment is incomplete:" not in missing_result.stderr:
        raise SystemExit("self-test missing small environment diagnostic")
    if "large proof environment is incomplete:" not in missing_result.stderr:
        raise SystemExit("self-test missing large environment diagnostic")
    for artifact_name, _text in ARTIFACT_HELP_ITEMS:
        if f"artifact_help_{artifact_name}=" not in missing_result.stderr:
            raise SystemExit(f"self-test missing {artifact_name} artifact hint")
    template_result = subprocess.run(
        [
            sys.executable,
            str(Path(__file__).resolve()),
            "--suite",
            "small",
            "--env-file",
            str(empty_env),
            "--print-env-template",
        ],
        cwd=root,
        env=inherited_env,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if template_result.returncode != 0:
        raise SystemExit("self-test env template print should succeed")
    for line in artifact_template_help_lines():
        if line and line not in template_result.stdout:
            raise SystemExit(f"self-test env template missing artifact hint: {line}")
    fake_bin = work_dir / "fake-prover.py"
    fake_bin.write_text(
        "\n".join(
            [
                "#!/usr/bin/env python3",
                "import os",
                "import pathlib",
                "import sys",
                "args = sys.argv[1:]",
                "if args[:2] in (['eth', 'block-input-summary'], ['setup', 'program-image-cache-summary']):",
                "    print('status=ok')",
                "    sys.exit(0)",
                "if args[:2] == ['verify', 'proof']:",
                "    proof = pathlib.Path(args[-2])",
                "    public_values = pathlib.Path(args[-1])",
                "    if not proof.exists() or not public_values.exists():",
                "        sys.stderr.write('missing proof outputs for verify\\n')",
                "        sys.exit(8)",
                "    print('status=ok')",
                "    print('verify_proof_status=ok')",
                "    print('artifact_public_input_match=ok')",
                "    print('artifact_proof_match=ok')",
                "    print('eth_block_input_match=ok')",
                "    print('program_image_cache_match=ok')",
                "    print('framed_guest_input_match=ok')",
                "    print('pipeline_input_bindings=ok')",
                "    sys.exit(0)",
                "tmp = os.environ.get('TMPDIR')",
                "if not tmp or not os.path.isdir(tmp):",
                "    sys.stderr.write('missing TMPDIR\\n')",
                "    sys.exit(9)",
                "output_dir = pathlib.Path(args[-2])",
                "output_dir.mkdir(parents=True, exist_ok=True)",
                "(output_dir / 'proof.bin').write_bytes(b'proof')",
                "(output_dir / 'eth-block-public-values.bin').write_bytes(b'public')",
                "label = os.environ.get('LZVM_TIMING_BATCH_LABEL', 'small')",
                "run = int(os.environ.get('LZVM_TIMING_BATCH_RUN', '1'))",
                "base = 1000 if label == 'small' else 2000",
                "print('status=ok')",
                "print('verify_outputs=true')",
                "print(f'timing_total_ms={base + run}')",
                "print('timing_guest_stage_tree_commit_root_count=1')",
                "print('timing_guest_stage_tree_commit_root_materialization_groups=1')",
                "print('timing_guest_stage_tree_commit_root_materialization_max_group_size=1')",
                "print('timing_finish_witness_opening_row_dedup_input_rows=0')",
                "print('timing_finish_witness_opening_row_dedup_unique_rows=0')",
                "print('timing_finish_witness_opening_row_dedup_elided_rows=0')",
                "print('timing_finish_fri_opening_ms=10')",
                "print('timing_finish_fri_opening_unit_build_ms=8')",
                "print('timing_finish_fri_opening_layer_tree_ms=2')",
                "print('timing_finish_fri_opening_query_ms=3')",
                "print('timing_finish_fri_opening_fold_ms=1')",
                "print('timing_finish_fri_opening_unit_count=1')",
                "print('timing_finish_fri_opening_layer_count=2')",
                "print('timing_finish_fri_opening_query_count=3')",
                "print('timing_finish_fri_transcript_unit_build_ms=4')",
                "print('timing_finish_fri_transcript_layer_tree_ms=2')",
                "print('timing_finish_fri_transcript_fold_ms=1')",
                "print('timing_finish_fri_transcript_unit_count=1')",
                "print('timing_finish_fri_transcript_layer_count=2')",
                "print('timing_finish_contribution_segment_ms=5')",
                "print('timing_finish_contribution_verify_ms=6')",
                "print('timing_finish_contribution_challenge_ms=7')",
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
            elif suffix == "INPUT_DATA":
                path.write_bytes(framed_input_data(b"fixture"))
            else:
                path.write_bytes(b"fixture")
            os.environ[f"{prefix}_{suffix}"] = str(path)
        os.environ[f"{prefix}_BIN"] = str(fake_bin)
    args = argparse.Namespace(
        commit="selftest",
        dry_run=False,
        env_file=None,
        large_mode="combined",
        large_timeout=10.0,
        max_relative_spread=0.10,
        max_runs=None,
        path=str(work_dir / "improve-log.csv"),
        runner="scripts/run-proof-timing-batch.py",
        runs=3,
        small_mode="combined",
        small_timeout=10.0,
        skip_targets=False,
        small_bin=None,
        small_max_avg_s=None,
        append_max_average_rejections=False,
        suite="both",
        summary="self test",
        work_dir=str(work_dir / "runs"),
        check_env=False,
        enforce_targets=False,
        large_bin=None,
        large_max_avg_s=None,
        parallel_lower_job_queue=None,
        parallel_lower_workers=None,
        segment_commit_workers=None,
        seed_discovery=False,
        seed_discovery_streaming_device_lower=False,
        owned_streaming_lower=False,
        trace_shape_timing=False,
        trace_detail_timing=False,
        trace_detail_timing_sample_stride=None,
        gpu_preallocate=False,
        minimal_memory=False,
        no_pack_trace=False,
        gpu_streams=None,
        witness_thread_pools=None,
        stored_witnesses=None,
        print_env_template=False,
        check_profile_tools=False,
        check_gpu_memory=False,
        print_profile_commands=False,
        min_gpu_free_mib=DEFAULT_MIN_GPU_FREE_MIB,
        nvidia_smi_command=None,
        ncu_command=None,
        ncu_set=DEFAULT_NCU_SET,
        ncu_target_processes=DEFAULT_NCU_TARGET_PROCESSES,
        nsys_command=None,
        nsys_trace=DEFAULT_NSYS_TRACE,
        profile_output_dir=DEFAULT_PROFILE_OUTPUT_DIR,
        profile_arg=[],
        profile_tool=DEFAULT_PROFILE_TOOL,
        skip_nsys_export=False,
        write_env_template=None,
        skip_verify_proof=False,
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
        batch_root = work_dir / "runs"
        batch_dirs = [path for path in batch_root.iterdir() if path.is_dir()]
        if len(batch_dirs) != 1:
            raise SystemExit("self-test batch directory missing")
        batch_dir = batch_dirs[0]
        batch_json = json.loads((batch_dir / "batch.json").read_text(encoding="utf-8"))
        expected_spread = {
            "small_stable_spread_s": 0.002,
            "large_stable_spread_s": 0.002,
            "small_stable_spread_ms": 2,
            "large_stable_spread_ms": 2,
            "small_stable_relative_spread": 0.001996,
            "large_stable_relative_spread": 0.000999,
        }
        for key, value in expected_spread.items():
            if batch_json.get(key) != value:
                raise SystemExit(f"self-test {key} mismatch in batch json")
        expected_averages = {
            "small_stable_avg_ms": 1002,
            "large_stable_avg_ms": 2002,
        }
        for key, value in expected_averages.items():
            if batch_json.get(key) != value:
                raise SystemExit(f"self-test {key} mismatch in batch json")
        expected_counts = {
            "small_run_count": 3,
            "large_run_count": 3,
            "small_stable_run_count": 3,
            "large_stable_run_count": 3,
        }
        for key, value in expected_counts.items():
            if batch_json.get(key) != value:
                raise SystemExit(f"self-test {key} mismatch in batch json")
        expected_samples = {
            "small_stable_timing_s": [1.001, 1.002, 1.003],
            "large_stable_timing_s": [2.001, 2.002, 2.003],
        }
        for key, value in expected_samples.items():
            if batch_json.get(key) != value:
                raise SystemExit(f"self-test {key} mismatch in batch json")
        for key, name in [
            ("small_stable_timing_summary", "small-stable.proof-timing-summary.csv"),
            ("large_stable_timing_summary", "large-stable.proof-timing-summary.csv"),
        ]:
            if batch_json.get(key) is None:
                raise SystemExit(f"self-test {key} missing from batch json")
            summary = Path(batch_json[key])
            if summary.name != name or not summary.exists():
                raise SystemExit(f"self-test {key} artifact missing")
            summary_text = summary.read_text(encoding="utf-8")
            if "aggregate,total_count,valid_total_count" not in summary_text:
                raise SystemExit(f"self-test {key} aggregate row missing")
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
    parser.add_argument("--max-runs", type=positive_run_count, default=None)
    parser.add_argument("--small-timeout", type=positive_timeout, default=60.0)
    parser.add_argument("--large-timeout", type=positive_timeout, default=180.0)
    parser.add_argument("--small-max-avg-s", type=positive_timeout, default=None)
    parser.add_argument("--large-max-avg-s", type=positive_timeout, default=None)
    parser.add_argument("--append-max-average-rejections", action="store_true")
    parser.add_argument("--parallel-lower-workers", type=positive_integer, default=None)
    parser.add_argument("--parallel-lower-job-queue", type=positive_integer, default=None)
    parser.add_argument("--segment-commit-workers", type=positive_integer, default=None)
    parser.add_argument("--seed-discovery", action="store_true")
    parser.add_argument("--seed-discovery-streaming-device-lower", action="store_true")
    parser.add_argument("--owned-streaming-lower", action="store_true")
    parser.add_argument("--trace-shape-timing", action="store_true")
    parser.add_argument("--trace-detail-timing", action="store_true")
    parser.add_argument("--trace-detail-timing-sample-stride", type=positive_integer, default=None)
    parser.add_argument("--gpu-preallocate", action="store_true")
    parser.add_argument("--minimal-memory", action="store_true")
    parser.add_argument("--no-pack-trace", action="store_true")
    parser.add_argument("--gpu-streams", type=positive_integer, default=None)
    parser.add_argument("--witness-thread-pools", type=positive_integer, default=None)
    parser.add_argument("--stored-witnesses", type=positive_integer, default=None)
    parser.add_argument("--work-dir", default="temp/proof-timing-batch")
    parser.add_argument("--path", default="temp/improve-log.csv")
    parser.add_argument("--summary")
    parser.add_argument("--commit")
    parser.add_argument("--env-file")
    parser.add_argument("--small-bin")
    parser.add_argument("--large-bin")
    parser.add_argument("--max-relative-spread", type=nonnegative_float, default=0.10)
    parser.add_argument("--runner", default=DEFAULT_RUNNER)
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("--check-env", action="store_true")
    parser.add_argument("--enforce-targets", action="store_true")
    parser.add_argument("--skip-targets", action="store_true")
    parser.add_argument("--print-env-template", action="store_true")
    parser.add_argument("--write-env-template")
    parser.add_argument("--check-profile-tools", action="store_true")
    parser.add_argument("--print-profile-commands", action="store_true")
    parser.add_argument("--check-gpu-memory", action="store_true")
    parser.add_argument("--min-gpu-free-mib", type=positive_mib, default=DEFAULT_MIN_GPU_FREE_MIB)
    parser.add_argument("--nvidia-smi-command")
    parser.add_argument("--profile-output-dir", default=DEFAULT_PROFILE_OUTPUT_DIR)
    parser.add_argument("--nsys-command")
    parser.add_argument("--ncu-command")
    parser.add_argument("--nsys-trace", default=DEFAULT_NSYS_TRACE)
    parser.add_argument("--ncu-set", default=DEFAULT_NCU_SET)
    parser.add_argument("--ncu-target-processes", default=DEFAULT_NCU_TARGET_PROCESSES)
    parser.add_argument("--profile-arg", action="append", default=[])
    parser.add_argument("--skip-nsys-export", action="store_true")
    parser.add_argument(
        "--profile-tool",
        choices=["nsys", "ncu", "both"],
        default=DEFAULT_PROFILE_TOOL,
    )
    parser.add_argument("--skip-verify-proof", action="store_true")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()

    if args.self_test:
        self_test()
    else:
        raise SystemExit(run(args))


if __name__ == "__main__":
    main()
