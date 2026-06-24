#!/usr/bin/env python3
import argparse
import os
import re
import shlex
import shutil
import subprocess
import sys
from datetime import datetime
from pathlib import Path


SAFE_NAME_RE = re.compile(r"^[A-Za-z0-9._-]+$")


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


def prefixed_path(output_dir: Path, name: str, suffix: str) -> Path:
    return Path(str(output_dir / name) + suffix)


def nsys_outputs(output_dir: Path, name: str) -> dict[str, Path]:
    return {
        "prefix": output_dir / name,
        "report": prefixed_path(output_dir, name, ".nsys-rep"),
        "sqlite": prefixed_path(output_dir, name, ".sqlite"),
    }


def ncu_outputs(output_dir: Path, name: str) -> dict[str, Path]:
    return {
        "report": prefixed_path(output_dir, name, ".ncu-rep"),
        "csv": prefixed_path(output_dir, name, ".ncu.csv"),
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
        *command,
    ]
    return profile, outputs


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
        *command,
    ]
    return profile, outputs


def print_nsys_outputs(outputs: dict[str, Path], root: Path) -> None:
    report = display_path_for_shell(outputs["report"], root)
    sqlite = display_path_for_shell(outputs["sqlite"], root)
    print(f"nsys_report={report}")
    print(f"nsys_sqlite={sqlite}")
    print(
        "nsys_export_command="
        + shell_join(
            [
                "nsys",
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
    for script in [
        "scripts/nsys-cuda-kernel-summary.py",
        "scripts/nsys-cuda-sync-summary.py",
        "scripts/nsys-cuda-copy-summary.py",
    ]:
        name = Path(script).stem.replace("-", "_")
        print(f"{name}_command=" + shell_join([script, sqlite]))


def print_ncu_outputs(outputs: dict[str, Path], root: Path) -> None:
    report = display_path_for_shell(outputs["report"], root)
    csv_path = display_path_for_shell(outputs["csv"], root)
    print(f"ncu_report={report}")
    print(f"ncu_csv={csv_path}")
    print(
        "ncu_cuda_kernel_summary_command="
        + shell_join(["scripts/ncu-cuda-kernel-summary.py", csv_path])
    )


def run_profile(args: argparse.Namespace) -> int:
    root = workspace_root()
    command = strip_separator(args.command)
    if not command:
        raise SystemExit("profiled command is required after --")
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
    output_dir.mkdir(parents=True, exist_ok=True)

    if args.tool == "nsys":
        profile_command, outputs = build_nsys_command(args, output_dir, command)
        print("profile_command=" + shell_join(profile_command))
        print_nsys_outputs(outputs, root)
    else:
        profile_command, outputs = build_ncu_command(args, output_dir, command)
        print("profile_command=" + shell_join(profile_command))
        print_ncu_outputs(outputs, root)

    if args.dry_run:
        return 0
    return subprocess.run(profile_command, cwd=cwd).returncode


def write_fake_profiler(path: Path, tool: str, log_path: Path) -> None:
    path.write_text(
        "\n".join(
            [
                "#!/usr/bin/env python3",
                "import pathlib",
                "import subprocess",
                "import sys",
                f"log = pathlib.Path({str(log_path)!r})",
                "log.parent.mkdir(parents=True, exist_ok=True)",
                f"log.write_text({tool!r} + '\\n' + '\\n'.join(sys.argv[1:]) + '\\n', encoding='utf-8')",
                "args = sys.argv[1:]",
                "if 'profile' in args and '--output' in args:",
                "    prefix = pathlib.Path(args[args.index('--output') + 1])",
                "    pathlib.Path(str(prefix) + '.nsys-rep').write_text('report\\n', encoding='utf-8')",
                "if '--log-file' in args:",
                "    pathlib.Path(args[args.index('--log-file') + 1]).write_text('Kernel Name,gpu__time_duration.sum\\nself,1\\n', encoding='utf-8')",
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


def self_test() -> None:
    root = workspace_root()
    work_dir = root / "temp" / f"proof-profile-self-test-{os.getpid()}"
    shutil.rmtree(work_dir, ignore_errors=True)
    work_dir.mkdir(parents=True)
    fake_nsys = work_dir / "fake-nsys.py"
    fake_ncu = work_dir / "fake-ncu.py"
    write_fake_profiler(fake_nsys, "nsys", work_dir / "fake-nsys.argv")
    write_fake_profiler(fake_ncu, "ncu", work_dir / "fake-ncu.argv")
    try:
        base = {
            "command": [
                sys.executable,
                "-c",
                "print('timing_total_ms=1000')",
            ],
            "cwd": ".",
            "dry_run": False,
            "name": "self-test",
            "output_dir": str(work_dir / "profiles"),
            "profile_arg": [],
            "ncu_set": "basic",
            "ncu_target_processes": "all",
            "nsys_trace": "cuda,nvtx,osrt",
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

        ncu_base = dict(base)
        ncu_base["name"] = "self-test-ncu"
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
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    args.name = profile_name(args.name)
    return args


def main() -> int:
    args = parse_args()
    if args.self_test:
        self_test()
        return 0
    return run_profile(args)


if __name__ == "__main__":
    sys.exit(main())
