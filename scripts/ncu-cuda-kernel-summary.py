#!/usr/bin/env python3
import argparse
import csv
import io
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


def metric_value(row: dict[str, str], metric: str) -> str | None:
    if metric in row:
        return row[metric]
    suffix = f".{metric}"
    for name, value in row.items():
        if name.endswith(suffix):
            return value
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
    with path.open(newline="") as handle:
        reader = csv.DictReader(handle)
        if not reader.fieldnames or "Kernel Name" not in reader.fieldnames:
            raise SystemExit(f"not an Nsight Compute CSV export: {path}")
        return list(reader)


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
        description="Summarize CUDA kernel metrics from an Nsight Compute CSV export."
    )
    parser.add_argument("csv", nargs="?", help="path to an ncu CSV export")
    parser.add_argument("--top", type=int, default=16, help="rows to print per summary")
    parser.add_argument("--self-test", action="store_true", help="run against an in-memory sample")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if args.self_test:
        summarize(build_self_test_rows(), "self-test", max(args.top, 1))
        return 0
    if not args.csv:
        raise SystemExit("CSV path is required unless --self-test is used")
    path = Path(args.csv)
    if not path.exists():
        raise SystemExit(f"NCU CSV export does not exist: {path}")
    summarize(read_ncu_csv(path), str(path), max(args.top, 1))
    return 0


if __name__ == "__main__":
    sys.exit(main())
