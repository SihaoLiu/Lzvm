#!/usr/bin/env python3
import argparse
import csv
import io
import os
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path


METRIC_DURATION_US = "gpu__time_duration.sum"
METRIC_SM_THROUGHPUT = "sm__throughput.avg.pct_of_peak_sustained_elapsed"
METRIC_DRAM_THROUGHPUT = "gpu__dram_throughput.avg.pct_of_peak_sustained_elapsed"
METRIC_MEMORY_THROUGHPUT = "gpu__compute_memory_throughput.avg.pct_of_peak_sustained_elapsed"
METRIC_ISSUE_ACTIVE = "sm__issue_active.avg.pct_of_peak_sustained_elapsed"
METRIC_ACTIVE_WARPS = "sm__warps_active.avg.pct_of_peak_sustained_active"
METRIC_REGISTER_LIMIT = "launch__occupancy_limit_registers"
METRIC_SHARED_MEM_LIMIT = "launch__occupancy_limit_shared_mem"
METRIC_WARP_LIMIT = "launch__occupancy_limit_warps"
METRIC_BLOCK_LIMIT = "launch__occupancy_limit_blocks"
METRIC_REGISTERS_PER_THREAD = "launch__registers_per_thread"
METRIC_SHARED_MEM_PER_BLOCK = "launch__shared_mem_per_block"

REQUIRED_METRIC_COLUMNS = [
    METRIC_DURATION_US,
    METRIC_SM_THROUGHPUT,
    METRIC_DRAM_THROUGHPUT,
    METRIC_MEMORY_THROUGHPUT,
    METRIC_ISSUE_ACTIVE,
    METRIC_ACTIVE_WARPS,
    METRIC_REGISTER_LIMIT,
    METRIC_SHARED_MEM_LIMIT,
    METRIC_WARP_LIMIT,
    METRIC_BLOCK_LIMIT,
    METRIC_REGISTERS_PER_THREAD,
    METRIC_SHARED_MEM_PER_BLOCK,
]


def metric_value(row: dict[str, str], metric: str) -> str | None:
    key = metric_key(row, metric)
    if key is not None:
        return row[key]
    return None


def metric_key(row: dict[str, str], metric: str) -> str | None:
    if metric in row:
        return metric
    suffix = f".{metric}"
    for name in row:
        if name.endswith(suffix):
            return name
    return None


def parse_float(value: str | None) -> float | None:
    if value is None:
        return None
    value = value.strip()
    if not value:
        return None
    try:
        return float(value.replace(",", ""))
    except ValueError:
        return None


def fmt(value: float | None) -> str:
    if value is None:
        return "na"
    return f"{value:.3f}"


@dataclass
class KernelMetrics:
    kernel: str
    profiles: int = 0
    duration_us: float = 0.0
    samples: dict[str, list[float]] = field(default_factory=dict)

    def add_sample(self, name: str, value: float | None) -> None:
        if value is not None:
            self.samples.setdefault(name, []).append(value)

    def avg(self, name: str) -> float | None:
        values = self.samples.get(name, [])
        if not values:
            return None
        return sum(values) / len(values)

    def add_row(self, row: dict[str, str]) -> None:
        self.profiles += 1
        self.duration_us += parse_float(metric_value(row, METRIC_DURATION_US)) or 0.0
        for name in [
            METRIC_SM_THROUGHPUT,
            METRIC_DRAM_THROUGHPUT,
            METRIC_MEMORY_THROUGHPUT,
            METRIC_ISSUE_ACTIVE,
            METRIC_ACTIVE_WARPS,
            METRIC_REGISTER_LIMIT,
            METRIC_SHARED_MEM_LIMIT,
            METRIC_WARP_LIMIT,
            METRIC_BLOCK_LIMIT,
            METRIC_REGISTERS_PER_THREAD,
            METRIC_SHARED_MEM_PER_BLOCK,
        ]:
            self.add_sample(name, parse_float(metric_value(row, name)))

    def limiting_factors(self) -> str:
        limits = [
            ("register_limited", self.avg(METRIC_REGISTER_LIMIT)),
            ("shared_mem_limited", self.avg(METRIC_SHARED_MEM_LIMIT)),
            ("warp_limited", self.avg(METRIC_WARP_LIMIT)),
            ("block_limited", self.avg(METRIC_BLOCK_LIMIT)),
        ]
        present = [(name, value) for name, value in limits if value is not None]
        if not present:
            return "unknown"
        minimum = min(value for _, value in present)
        winners = [name for name, value in present if abs(value - minimum) <= 0.001]
        return "|".join(winners)


def row_kernel_name(row: dict[str, str]) -> str:
    return (row.get("Kernel Name") or row.get("Kernel") or "").strip()


def fieldnames_contain_metric(fieldnames: list[str], metric: str) -> bool:
    suffix = f".{metric}"
    return any(name == metric or name.endswith(suffix) for name in fieldnames)


def scale_for_duration_unit(unit: str | None) -> float:
    normalized = (unit or "").strip().lower()
    if normalized in {"ns", "nanosecond", "nanoseconds"}:
        return 0.001
    if normalized in {"us", "usecond", "useconds", "microsecond", "microseconds"}:
        return 1.0
    if normalized in {"ms", "msecond", "mseconds", "millisecond", "milliseconds"}:
        return 1000.0
    if normalized in {"s", "sec", "second", "seconds"}:
        return 1_000_000.0
    return 1.0


def scale_for_shared_mem_unit(unit: str | None) -> float:
    normalized = (unit or "").strip().lower()
    if normalized in {"byte", "bytes", "byte/block", "bytes/block"}:
        return 1.0 / 1024.0
    if normalized in {"kbyte", "kbytes", "kbyte/block", "kbytes/block", "kb", "kb/block"}:
        return 1.0
    if normalized in {"mbyte", "mbytes", "mbyte/block", "mbytes/block", "mb", "mb/block"}:
        return 1024.0
    return 1.0


def scale_metric(row: dict[str, str], metric: str, scale: float) -> None:
    if scale == 1.0:
        return
    key = metric_key(row, metric)
    if key is None:
        return
    value = parse_float(row[key])
    if value is None:
        return
    row[key] = f"{value * scale:.12g}"


def normalize_ncu_metric_units(rows: list[dict[str, str]]) -> list[dict[str, str]]:
    unit_row = next(
        (
            row
            for row in rows
            if not row_kernel_name(row) and metric_value(row, METRIC_DURATION_US)
        ),
        {},
    )
    duration_scale = scale_for_duration_unit(metric_value(unit_row, METRIC_DURATION_US))
    shared_mem_scale = scale_for_shared_mem_unit(
        metric_value(unit_row, METRIC_SHARED_MEM_PER_BLOCK)
    )
    for row in rows:
        if not row_kernel_name(row):
            continue
        scale_metric(row, METRIC_DURATION_US, duration_scale)
        scale_metric(row, METRIC_SHARED_MEM_PER_BLOCK, shared_mem_scale)
    return rows


def find_ncu_csv_header_offset(lines: list[str], path: Path) -> int:
    for index, line in enumerate(lines):
        if "Kernel Name" not in line:
            continue
        try:
            fieldnames = next(csv.reader([line]))
        except csv.Error:
            continue
        if "Kernel Name" in fieldnames:
            return index
    raise SystemExit(f"not an Nsight Compute CSV export: {path}")


def missing_metric_message(path: Path, lines: list[str], missing: list[str]) -> str:
    hint = "collect with --set basic --page raw --csv"
    warning = next((line.strip() for line in lines if "No metrics to collect" in line), "")
    if warning:
        hint = f"{hint}; profiler warning: {warning}"
    return (
        f"NCU CSV export is missing required metric columns in {path}: "
        f"{', '.join(missing)}; {hint}"
    )


def summarize_rows(rows: list[dict[str, str]]) -> list[KernelMetrics]:
    by_kernel: dict[str, KernelMetrics] = {}
    for row in rows:
        kernel = row_kernel_name(row)
        if not kernel:
            continue
        metrics = by_kernel.setdefault(kernel, KernelMetrics(kernel))
        metrics.add_row(row)
    return sorted(
        by_kernel.values(),
        key=lambda metrics: (-metrics.duration_us, -metrics.profiles, metrics.kernel),
    )


def read_ncu_csv(path: Path) -> list[dict[str, str]]:
    lines = path.read_text().splitlines(keepends=True)
    return parse_ncu_csv_lines(path, lines)


def parse_ncu_csv_lines(path: Path, lines: list[str]) -> list[dict[str, str]]:
    header_offset = find_ncu_csv_header_offset(lines, path)
    reader = csv.DictReader(io.StringIO("".join(lines[header_offset:])))
    if not reader.fieldnames or "Kernel Name" not in reader.fieldnames:
        raise SystemExit(f"not an Nsight Compute CSV export: {path}")
    missing = [
        metric
        for metric in REQUIRED_METRIC_COLUMNS
        if not fieldnames_contain_metric(reader.fieldnames, metric)
    ]
    if missing:
        raise SystemExit(missing_metric_message(path, lines, missing))
    return normalize_ncu_metric_units(list(reader))


def read_ncu_report(path: Path, ncu_command: str) -> list[dict[str, str]]:
    command = [ncu_command, "--import", str(path), "--csv", "--page", "raw"]
    try:
        completed = subprocess.run(
            command,
            check=False,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
    except FileNotFoundError:
        raise SystemExit(
            f"failed to import NCU report {path}: command not found: {ncu_command}"
        )
    if completed.returncode != 0:
        detail = completed.stderr.strip() or completed.stdout.strip()
        raise SystemExit(
            f"failed to import NCU report {path} with {ncu_command}: {detail}"
        )
    return parse_ncu_csv_lines(path, completed.stdout.splitlines(keepends=True))


def read_ncu_profile(path: Path, ncu_command: str) -> list[dict[str, str]]:
    if path.suffix == ".ncu-rep":
        return read_ncu_report(path, ncu_command)
    try:
        return read_ncu_csv(path)
    except UnicodeDecodeError as error:
        raise SystemExit(
            f"not a UTF-8 Nsight Compute CSV export: {path}; "
            "pass a .ncu-rep report or export CSV with ncu --import <report> --csv --page raw"
        ) from error


def writerow(writer: csv.writer, values: list[object]) -> None:
    writer.writerow(values)


def print_kernel_metric_summary(writer: csv.writer, rows: list[KernelMetrics], limit: int) -> None:
    print("kernel_metric_summary")
    writerow(
        writer,
        [
            "kernel",
            "profiles",
            "duration_ms",
            "avg_duration_us",
            "sm_throughput_pct",
            "dram_throughput_pct",
            "compute_memory_pct",
            "issue_active_pct",
            "active_warps_pct",
            "registers_per_thread",
            "shared_mem_kb_per_block",
        ],
    )
    for metrics in rows[:limit]:
        avg_duration = metrics.duration_us / metrics.profiles if metrics.profiles else 0.0
        writerow(
            writer,
            [
                metrics.kernel,
                metrics.profiles,
                fmt(metrics.duration_us / 1000.0),
                fmt(avg_duration),
                fmt(metrics.avg(METRIC_SM_THROUGHPUT)),
                fmt(metrics.avg(METRIC_DRAM_THROUGHPUT)),
                fmt(metrics.avg(METRIC_MEMORY_THROUGHPUT)),
                fmt(metrics.avg(METRIC_ISSUE_ACTIVE)),
                fmt(metrics.avg(METRIC_ACTIVE_WARPS)),
                fmt(metrics.avg(METRIC_REGISTERS_PER_THREAD)),
                fmt(metrics.avg(METRIC_SHARED_MEM_PER_BLOCK)),
            ],
        )
    if not rows:
        writerow(writer, ["none", 0, "0.000", "0.000", "na", "na", "na", "na", "na", "na", "na"])


def print_occupancy_limits(writer: csv.writer, rows: list[KernelMetrics], limit: int) -> None:
    print()
    print("occupancy_limits")
    writerow(
        writer,
        [
            "kernel",
            "profiles",
            "register_limit_blocks",
            "shared_mem_limit_blocks",
            "warp_limit_blocks",
            "block_limit_blocks",
            "registers_per_thread",
            "shared_mem_kb_per_block",
            "limiting_factors",
        ],
    )
    ranked = sorted(
        rows,
        key=lambda metrics: (
            metrics.avg(METRIC_REGISTER_LIMIT) is None,
            metrics.avg(METRIC_REGISTER_LIMIT) or 0.0,
            -metrics.duration_us,
            metrics.kernel,
        ),
    )
    for metrics in ranked[:limit]:
        writerow(
            writer,
            [
                metrics.kernel,
                metrics.profiles,
                fmt(metrics.avg(METRIC_REGISTER_LIMIT)),
                fmt(metrics.avg(METRIC_SHARED_MEM_LIMIT)),
                fmt(metrics.avg(METRIC_WARP_LIMIT)),
                fmt(metrics.avg(METRIC_BLOCK_LIMIT)),
                fmt(metrics.avg(METRIC_REGISTERS_PER_THREAD)),
                fmt(metrics.avg(METRIC_SHARED_MEM_PER_BLOCK)),
                metrics.limiting_factors(),
            ],
        )
    if not rows:
        writerow(writer, ["none", 0, "na", "na", "na", "na", "na", "na", "unknown"])


def print_memory_bound_candidates(
    writer: csv.writer, rows: list[KernelMetrics], limit: int
) -> None:
    print()
    print("memory_bound_candidates")
    writerow(
        writer,
        [
            "kernel",
            "profiles",
            "duration_ms",
            "dram_throughput_pct",
            "compute_memory_pct",
            "sm_throughput_pct",
            "issue_active_pct",
        ],
    )
    ranked = sorted(
        rows,
        key=lambda metrics: (
            -(metrics.avg(METRIC_DRAM_THROUGHPUT) or 0.0),
            -metrics.duration_us,
            metrics.kernel,
        ),
    )
    for metrics in ranked[:limit]:
        writerow(
            writer,
            [
                metrics.kernel,
                metrics.profiles,
                fmt(metrics.duration_us / 1000.0),
                fmt(metrics.avg(METRIC_DRAM_THROUGHPUT)),
                fmt(metrics.avg(METRIC_MEMORY_THROUGHPUT)),
                fmt(metrics.avg(METRIC_SM_THROUGHPUT)),
                fmt(metrics.avg(METRIC_ISSUE_ACTIVE)),
            ],
        )
    if not rows:
        writerow(writer, ["none", 0, "0.000", "na", "na", "na", "na"])


def summarize(rows: list[dict[str, str]], label: str, limit: int) -> None:
    metrics = summarize_rows(rows)
    writer = csv.writer(sys.stdout, lineterminator="\n")
    print(f"profile={label}")
    print_kernel_metric_summary(writer, metrics, limit)
    print_occupancy_limits(writer, metrics, limit)
    print_memory_bound_candidates(writer, metrics, limit)


def build_self_test_rows() -> list[dict[str, str]]:
    text = io.StringIO()
    writer = csv.writer(text, lineterminator="\n")
    header = [
        "Kernel Name",
        METRIC_DURATION_US,
        METRIC_SM_THROUGHPUT,
        METRIC_DRAM_THROUGHPUT,
        METRIC_MEMORY_THROUGHPUT,
        METRIC_ISSUE_ACTIVE,
        METRIC_ACTIVE_WARPS,
        METRIC_REGISTER_LIMIT,
        METRIC_SHARED_MEM_LIMIT,
        METRIC_WARP_LIMIT,
        METRIC_BLOCK_LIMIT,
        METRIC_REGISTERS_PER_THREAD,
        METRIC_SHARED_MEM_PER_BLOCK,
    ]
    writer.writerow(header)
    writer.writerow(["", "us", "%", "%", "%", "%", "%", "block", "block", "block", "block", "register/thread", "Kbyte/block"])
    writer.writerow(
        [
            "ntt_stage_kernel",
            "40.0",
            "62.0",
            "58.0",
            "58.0",
            "54.0",
            "89.0",
            "6",
            "14",
            "6",
            "24",
            "38",
            "1.104",
        ]
    )
    writer.writerow(
        [
            "ntt_stage_kernel",
            "60.0",
            "64.0",
            "60.0",
            "60.0",
            "56.0",
            "91.0",
            "6",
            "14",
            "6",
            "24",
            "38",
            "1.104",
        ]
    )
    writer.writerow(
        [
            "poseidon2_width16_merkle_parent_kernel",
            "20.0",
            "35.0",
            "15.0",
            "18.0",
            "20.0",
            "42.0",
            "8",
            "12",
            "8",
            "24",
            "64",
            "2.000",
        ]
    )
    text.seek(0)
    return list(csv.DictReader(text))


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Summarize CUDA kernel metrics from an Nsight Compute CSV or .ncu-rep export."
    )
    parser.add_argument("profile", nargs="?", help="path to an ncu CSV or .ncu-rep export")
    parser.add_argument("--top", type=int, default=16, help="rows to print per summary")
    parser.add_argument(
        "--ncu-command",
        default=os.environ.get("LZVM_NCU_COMMAND", "ncu"),
        help="Nsight Compute command used to import .ncu-rep reports",
    )
    parser.add_argument("--self-test", action="store_true", help="run against an in-memory sample")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if args.self_test:
        summarize(build_self_test_rows(), "self-test", max(args.top, 1))
        return 0
    if not args.profile:
        raise SystemExit("NCU profile path is required unless --self-test is used")
    path = Path(args.profile)
    if not path.exists():
        raise SystemExit(f"NCU profile export does not exist: {path}")
    summarize(read_ncu_profile(path, args.ncu_command), str(path), max(args.top, 1))
    return 0


if __name__ == "__main__":
    sys.exit(main())
