#!/usr/bin/env python3
import argparse
import sys
from pathlib import Path


ROOT_COUNT_KEY = "timing_guest_stage_tree_commit_root_count"
ROOT_GROUPS_KEY = "timing_guest_stage_tree_commit_root_materialization_groups"
ROOT_MAX_GROUP_KEY = "timing_guest_stage_tree_commit_root_materialization_max_group_size"
TOTAL_MS_KEY = "timing_total_ms"

HEADER = (
    "profile,total_ms,root_count,materialization_groups,"
    "materialization_max_group_size,roots_per_group,needs_cross_segment_root_pipeline"
)


def parse_timing_log(text: str) -> dict[str, int]:
    values: dict[str, int] = {}
    for line in text.splitlines():
        if "=" not in line:
            continue
        key, value = line.split("=", 1)
        if key not in {TOTAL_MS_KEY, ROOT_COUNT_KEY, ROOT_GROUPS_KEY, ROOT_MAX_GROUP_KEY}:
            continue
        try:
            values[key] = int(value.strip())
        except ValueError:
            continue
    return values


def summarize_profile(label: str, text: str) -> str:
    values = parse_timing_log(text)
    missing = [
        key
        for key in [ROOT_COUNT_KEY, ROOT_GROUPS_KEY, ROOT_MAX_GROUP_KEY]
        if key not in values
    ]
    if missing:
        raise SystemExit(f"{label}: missing timing fields: {', '.join(missing)}")

    total_ms = values.get(TOTAL_MS_KEY, 0)
    root_count = values[ROOT_COUNT_KEY]
    groups = values[ROOT_GROUPS_KEY]
    max_group_size = values[ROOT_MAX_GROUP_KEY]
    roots_per_group = root_count / groups if groups else 0.0
    needs_cross_segment_root_pipeline = (
        "yes" if root_count > 1 and groups >= root_count and max_group_size <= 1 else "no"
    )
    return (
        f"{label},{total_ms},{root_count},{groups},{max_group_size},"
        f"{roots_per_group:.3f},{needs_cross_segment_root_pipeline}"
    )


def print_summary(inputs: list[tuple[str, str]]) -> None:
    print(HEADER)
    for label, text in inputs:
        print(summarize_profile(label, text))


def self_test() -> None:
    print_summary(
        [
            (
                "single-root-groups",
                "\n".join(
                    [
                        f"{TOTAL_MS_KEY}=9050",
                        f"{ROOT_COUNT_KEY}=23",
                        f"{ROOT_GROUPS_KEY}=23",
                        f"{ROOT_MAX_GROUP_KEY}=1",
                    ]
                ),
            ),
            (
                "batched-roots",
                "\n".join(
                    [
                        f"{TOTAL_MS_KEY}=9050",
                        f"{ROOT_COUNT_KEY}=23",
                        f"{ROOT_GROUPS_KEY}=1",
                        f"{ROOT_MAX_GROUP_KEY}=23",
                    ]
                ),
            ),
        ]
    )


def read_input(path: str | None) -> tuple[str, str]:
    if path is None or path == "-":
        return ("stdin", sys.stdin.read())
    input_path = Path(path)
    return (str(input_path), input_path.read_text(encoding="utf-8"))


def main() -> None:
    parser = argparse.ArgumentParser(description="Summarize prove timing root materialization shape.")
    parser.add_argument("logs", nargs="*", help="prove --timings log paths, or '-' for stdin")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()

    if args.self_test:
        self_test()
        return
    if not args.logs:
        raise SystemExit("at least one log path is required unless --self-test is used")
    print_summary([read_input(path) for path in args.logs])


if __name__ == "__main__":
    main()
