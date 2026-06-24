#!/usr/bin/env python3
import argparse
import csv
import math
import subprocess
from datetime import datetime
from pathlib import Path

HEADER = [
    "timestamp",
    "commit",
    "small_proof_time_s",
    "large_proof_time_s",
    "summary",
]


def current_commit() -> str:
    return subprocess.check_output(
        ["git", "rev-parse", "--short=8", "HEAD"],
        text=True,
    ).strip()


def timestamp_now() -> str:
    return datetime.now().astimezone().strftime("%Y-%m-%dT%H:%M:%S%z")


def raw_csv_fields(record: str) -> list[str]:
    fields: list[str] = []
    start = 0
    in_quotes = False
    index = 0
    while index < len(record):
        char = record[index]
        if char == '"':
            if in_quotes and index + 1 < len(record) and record[index + 1] == '"':
                index += 2
                continue
            in_quotes = not in_quotes
        elif char == "," and not in_quotes:
            fields.append(record[start:index])
            start = index + 1
        index += 1
    fields.append(record[start:])
    return fields


def summary_field_is_double_quoted(record: str) -> bool:
    raw_fields = raw_csv_fields(record)
    if len(raw_fields) != len(HEADER):
        return False
    summary = raw_fields[-1]
    return len(summary) >= 2 and summary[0] == '"' and summary[-1] == '"'


def validate_improve_log(path: Path) -> None:
    if not path.exists():
        return
    with path.open(newline="") as source:
        lines = source.readlines()
        if not lines:
            raise SystemExit(f"{path}: empty improve log")
        header = next(csv.reader([lines[0]]))
        if header != HEADER:
            raise SystemExit(f"{path}: unexpected header: {header!r}")
        for index, line in enumerate(lines[1:], start=2):
            record = line.rstrip("\n")
            if record.endswith("\r"):
                record = record[:-1]
            row = next(csv.reader([record]))
            if len(row) != 5:
                raise SystemExit(
                    f"{path}:{index}: expected 5 CSV fields, found {len(row)}"
                )
            if not summary_field_is_double_quoted(record):
                raise SystemExit(f"{path}:{index}: summary field must be double-quoted")


def parse_run_times(raw: str, label: str) -> list[float]:
    values = []
    for index, part in enumerate(raw.split(","), start=1):
        value = part.strip()
        if not value:
            raise SystemExit(f"{label}: empty run time at position {index}")
        try:
            parsed = float(value)
        except ValueError as error:
            raise SystemExit(f"{label}: invalid run time {value!r}") from error
        if not math.isfinite(parsed) or parsed <= 0.0:
            raise SystemExit(f"{label}: run time must be a positive finite value: {value!r}")
        values.append(parsed)
    return values


def stable_average_field(
    samples: list[float],
    label: str,
    max_relative_spread: float,
) -> str:
    if len(samples) < 3:
        raise SystemExit(f"{label}: at least three runs are required")
    if max_relative_spread < 0.0:
        raise SystemExit("--max-relative-spread must be nonnegative")

    ordered = sorted(samples)
    best: list[float] | None = None
    for start in range(len(ordered)):
        for end in range(start + 3, len(ordered) + 1):
            candidate = ordered[start:end]
            median = candidate[len(candidate) // 2]
            if median == 0.0:
                continue
            relative_spread = (candidate[-1] - candidate[0]) / median
            if relative_spread <= max_relative_spread:
                if best is None or len(candidate) > len(best):
                    best = candidate
                elif best is not None and len(candidate) == len(best):
                    best_spread = (best[-1] - best[0]) / best[len(best) // 2]
                    if relative_spread < best_spread:
                        best = candidate

    if best is None:
        percent = max_relative_spread * 100.0
        raise SystemExit(
            f"{label}: no group of at least three runs is within {percent:.1f}% spread"
        )

    average = sum(best) / len(best)
    sample_text = ";".join(f"{value:.3f}" for value in best)
    return f"avg={average:.3f} samples={sample_text} used={len(best)}/{len(samples)}"


def resolve_timing_field(
    explicit: str,
    runs: str | None,
    label: str,
    max_relative_spread: float,
) -> str:
    if explicit and runs is not None:
        raise SystemExit(f"{label}: provide either explicit value or run samples, not both")
    if runs is None:
        return explicit
    return stable_average_field(parse_run_times(runs, label), label, max_relative_spread)


def append_row(
    path: Path,
    commit: str,
    small_proof_time_s: str,
    large_proof_time_s: str,
    summary: str,
) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    needs_header = not path.exists() or path.stat().st_size == 0
    with path.open("a", newline="") as output:
        writer = csv.writer(output, quoting=csv.QUOTE_ALL, lineterminator="\n")
        if needs_header:
            writer.writerow(HEADER)
        writer.writerow(
            [
                timestamp_now(),
                commit,
                small_proof_time_s,
                large_proof_time_s,
                summary,
            ]
        )


def main() -> None:
    parser = argparse.ArgumentParser(description="Append a quoted improve-log CSV row.")
    parser.add_argument("summary", nargs="?")
    parser.add_argument("--summary", dest="summary_flag")
    parser.add_argument("--path", default="temp/improve-log.csv")
    parser.add_argument("--commit", default=None)
    parser.add_argument("--small", default="")
    parser.add_argument("--large", default="")
    parser.add_argument("--small-runs", default=None)
    parser.add_argument("--large-runs", default=None)
    parser.add_argument("--max-relative-spread", type=float, default=0.10)
    parser.add_argument(
        "--check",
        action="store_true",
        help="validate the target log without appending",
    )
    args = parser.parse_args()

    path = Path(args.path)
    validate_improve_log(path)
    if not args.check:
        if args.summary is not None and args.summary_flag is not None:
            parser.error("summary must be provided either positionally or with --summary")
        summary = args.summary_flag if args.summary_flag is not None else args.summary
        if summary is None:
            parser.error("summary is required unless --check is used")
        small_proof_time_s = resolve_timing_field(
            args.small,
            args.small_runs,
            "small proof time",
            args.max_relative_spread,
        )
        large_proof_time_s = resolve_timing_field(
            args.large,
            args.large_runs,
            "large proof time",
            args.max_relative_spread,
        )
        append_row(
            path,
            args.commit or current_commit(),
            small_proof_time_s,
            large_proof_time_s,
            summary,
        )
        validate_improve_log(path)


if __name__ == "__main__":
    main()
