#!/usr/bin/env python3
import argparse
import csv
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
    parser.add_argument("--path", default="temp/improve-log.csv")
    parser.add_argument("--commit", default=None)
    parser.add_argument("--small", default="")
    parser.add_argument("--large", default="")
    parser.add_argument(
        "--check",
        action="store_true",
        help="validate the target log without appending",
    )
    args = parser.parse_args()

    path = Path(args.path)
    validate_improve_log(path)
    if not args.check:
        if args.summary is None:
            parser.error("summary is required unless --check is used")
        append_row(
            path,
            args.commit or current_commit(),
            args.small,
            args.large,
            args.summary,
        )
        validate_improve_log(path)


if __name__ == "__main__":
    main()
