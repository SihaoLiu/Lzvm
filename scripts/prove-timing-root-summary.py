#!/usr/bin/env python3
import argparse
import re
import sys
from pathlib import Path


INPUT_BYTES_KEY = "input_bytes"
ROOT_COUNT_KEY = "timing_guest_stage_tree_commit_root_count"
ROOT_GROUPS_KEY = "timing_guest_stage_tree_commit_root_materialization_groups"
ROOT_MAX_GROUP_KEY = "timing_guest_stage_tree_commit_root_materialization_max_group_size"
TOTAL_MS_KEY = "timing_total_ms"
RUNNER_MS_KEY = "timing_guest_trace_runner_ms"
LOWERER_MS_KEY = "timing_guest_trace_lowerer_ms"
STREAM_ELAPSED_MS_KEY = "timing_guest_trace_stream_elapsed_ms"
STREAM_WORKER_MS_KEY = "timing_guest_trace_stream_ms"
SEGMENT_COMMIT_MS_KEY = "timing_guest_segment_commit_ms"
SEGMENT_RECEIVE_WAIT_MS_KEY = "timing_guest_trace_segment_receive_wait_ms"
PENDING_RECEIVE_WAIT_MS_KEY = "timing_guest_trace_pending_receive_wait_ms"
PENDING_SEND_WAIT_MS_KEY = "timing_guest_trace_pending_send_wait_ms"
PARALLEL_LOWER_WORKERS_KEY = "timing_guest_trace_parallel_lower_workers"
PARALLEL_LOWER_DISPATCHED_KEY = "timing_guest_trace_parallel_lower_dispatched"
PARALLEL_LOWER_RECEIVED_KEY = "timing_guest_trace_parallel_lower_received"
PARALLEL_LOWER_EMITTED_KEY = "timing_guest_trace_parallel_lower_emitted"
PARALLEL_LOWER_MAX_REORDER_KEY = "timing_guest_trace_parallel_lower_max_reorder"
SEED_DIRECT_LIFT_ATTEMPTS_KEY = "timing_guest_trace_seed_direct_lift_attempts"
SEED_DIRECT_LIFT_SUCCESSES_KEY = "timing_guest_trace_seed_direct_lift_successes"
SEED_FULL_ADVANCES_KEY = "timing_guest_trace_seed_full_advances"
FINISH_OPENING_MS_KEY = "timing_finish_witness_opening_ms"
LEAF_KERNEL_MS_KEY = "timing_guest_stage_leaf_kernel_work_ms"
LEAF_COSET_CALLS_KEY = "timing_guest_stage_leaf_coset_extend_calls"
LEAF_COSET_COLUMNS_KEY = "timing_guest_stage_leaf_coset_extend_columns"
LEAF_NTT_LAUNCHES_KEY = "timing_guest_stage_leaf_coset_extend_ntt_launches"
LEAF_NTT_STAGE_LAUNCHES_KEY = "timing_guest_stage_leaf_coset_extend_ntt_stage_launches"
LEAF_NTT_BLOCK_TWIDDLE_LAUNCHES_KEY = (
    "timing_guest_stage_leaf_coset_extend_ntt_block_twiddle_launches"
)
DIRECT_D2H_WAIT_NS_KEY = "timing_cuda_direct_copy_d2h_wait_ns"
PERF_LOWERED_REPORT_ROW_SELF_PCT_KEY = "perf_lowered_report_row_self_pct"
PERF_MEMMOVE_SELF_PCT_KEY = "perf_memmove_self_pct"
PERF_MEMMOVE_GUEST_MACHINE_PCT_KEY = "perf_memmove_guest_machine_pct"
PERF_MEMMOVE_TRACE_SLICE_PCT_KEY = "perf_memmove_trace_slice_pct"
PERF_PENDING_SEGMENT_DROP_SELF_PCT_KEY = "perf_pending_segment_drop_self_pct"
ROOT_PIPELINE_INPUT_BYTE_LIMIT = 8 * 1024 * 1024
PERF_SELF_PERCENT_RE = re.compile(r"^\s*(\d+(?:\.\d+)?)%\s+(.*)$")
PERF_CALLCHAIN_PERCENT_RE = re.compile(r"(\d+(?:\.\d+)?)%--(.*)$")

HEADER = (
    "profile,input_bytes,total_ms,runner_ms,lowerer_ms,stream_elapsed_ms,stream_worker_ms,"
    "segment_commit_ms,stream_commit_residual_ms,segment_receive_wait_ms,"
    "pending_receive_wait_ms,pending_send_wait_ms,parallel_lower_workers,"
    "parallel_lower_dispatched,parallel_lower_received,parallel_lower_emitted,"
    "parallel_lower_max_reorder,seed_direct_lift_attempts,"
    "seed_direct_lift_successes,seed_full_advances,"
    "finish_opening_ms,root_count,materialization_groups,"
    "materialization_max_group_size,roots_per_group,needs_cross_segment_root_pipeline,"
    "root_pipeline_policy_hint,leaf_kernel_ms,leaf_coset_calls,leaf_coset_columns,leaf_ntt_launches,"
    "leaf_ntt_stage_launches,leaf_ntt_block_twiddle_launches,"
    "leaf_ntt_launches_per_call,direct_d2h_wait_ms,leaf_launch_pressure,"
    "trace_to_leaf_ratio,primary_bottleneck,perf_lowered_report_row_self_pct,"
    "perf_memmove_self_pct,perf_memmove_guest_machine_pct,"
    "perf_memmove_trace_slice_pct,perf_memmove_source_hint,"
    "perf_pending_segment_drop_self_pct,cpu_trace_hotspot_hint"
)
AGGREGATE_HEADER = (
    "aggregate,total_count,valid_total_count,total_min_ms,total_mean_ms,"
    "total_median_ms,total_max_ms,sample_spread_pct,close_samples,max_outlier"
)
CLOSE_SAMPLE_SPREAD_PCT = 5.0
OUTLIER_RATIO_THRESHOLD = 1.5

TIMING_KEYS = {
    INPUT_BYTES_KEY,
    TOTAL_MS_KEY,
    RUNNER_MS_KEY,
    LOWERER_MS_KEY,
    STREAM_ELAPSED_MS_KEY,
    STREAM_WORKER_MS_KEY,
    SEGMENT_COMMIT_MS_KEY,
    SEGMENT_RECEIVE_WAIT_MS_KEY,
    PENDING_RECEIVE_WAIT_MS_KEY,
    PENDING_SEND_WAIT_MS_KEY,
    PARALLEL_LOWER_WORKERS_KEY,
    PARALLEL_LOWER_DISPATCHED_KEY,
    PARALLEL_LOWER_RECEIVED_KEY,
    PARALLEL_LOWER_EMITTED_KEY,
    PARALLEL_LOWER_MAX_REORDER_KEY,
    SEED_DIRECT_LIFT_ATTEMPTS_KEY,
    SEED_DIRECT_LIFT_SUCCESSES_KEY,
    SEED_FULL_ADVANCES_KEY,
    FINISH_OPENING_MS_KEY,
    ROOT_COUNT_KEY,
    ROOT_GROUPS_KEY,
    ROOT_MAX_GROUP_KEY,
    LEAF_KERNEL_MS_KEY,
    LEAF_COSET_CALLS_KEY,
    LEAF_COSET_COLUMNS_KEY,
    LEAF_NTT_LAUNCHES_KEY,
    LEAF_NTT_STAGE_LAUNCHES_KEY,
    LEAF_NTT_BLOCK_TWIDDLE_LAUNCHES_KEY,
    DIRECT_D2H_WAIT_NS_KEY,
}


def parse_timing_log(text: str) -> dict[str, int]:
    values: dict[str, int] = {}
    for line in text.splitlines():
        if "=" not in line:
            continue
        key, value = line.split("=", 1)
        if key not in TIMING_KEYS:
            continue
        try:
            values[key] = int(value.strip())
        except ValueError:
            continue
    return values


def parse_perf_self_hotspots(text: str) -> dict[str, float]:
    hotspots = {
        PERF_LOWERED_REPORT_ROW_SELF_PCT_KEY: 0.0,
        PERF_MEMMOVE_SELF_PCT_KEY: 0.0,
        PERF_MEMMOVE_GUEST_MACHINE_PCT_KEY: 0.0,
        PERF_MEMMOVE_TRACE_SLICE_PCT_KEY: 0.0,
        PERF_PENDING_SEGMENT_DROP_SELF_PCT_KEY: 0.0,
    }
    in_memmove_callchain = False
    for line in text.splitlines():
        match = PERF_SELF_PERCENT_RE.match(line)
        if match:
            try:
                pct = float(match.group(1))
            except ValueError:
                continue
            symbol_text = match.group(2)
            in_memmove_callchain = "memmove" in symbol_text
            if "lowered_report_row" in symbol_text:
                key = PERF_LOWERED_REPORT_ROW_SELF_PCT_KEY
            elif "memmove" in symbol_text:
                key = PERF_MEMMOVE_SELF_PCT_KEY
            elif (
                "GuestPcTracePendingSegmentSlice" in symbol_text
                and "drop_in_place" in symbol_text
            ):
                key = PERF_PENDING_SEGMENT_DROP_SELF_PCT_KEY
            else:
                continue
            hotspots[key] = max(hotspots[key], pct)
            continue

        if not in_memmove_callchain:
            continue
        callchain_match = PERF_CALLCHAIN_PERCENT_RE.search(line)
        if not callchain_match:
            continue
        try:
            pct = float(callchain_match.group(1))
        except ValueError:
            continue
        symbol_text = callchain_match.group(2)
        if "advance_guest_machine_prepared_inner" in symbol_text:
            key = PERF_MEMMOVE_GUEST_MACHINE_PCT_KEY
        elif "run_guest_pc_trace_segment_slice" in symbol_text:
            key = PERF_MEMMOVE_TRACE_SLICE_PCT_KEY
        else:
            continue
        hotspots[key] = max(hotspots[key], pct)
    return hotspots


def primary_bottleneck(
    total_ms: int,
    runner_ms: int,
    lowerer_ms: int,
    stream_elapsed_ms: int,
    stream_worker_ms: int,
    segment_commit_ms: int,
    segment_receive_wait_ms: int,
    finish_opening_ms: int,
    leaf_kernel_ms: int,
    direct_d2h_wait_ms: float,
) -> str:
    candidates = [
        ("trace_runner", float(runner_ms)),
        ("trace_lowerer", float(lowerer_ms)),
        ("stream_elapsed", float(stream_elapsed_ms)),
        ("stream_worker", float(stream_worker_ms)),
        ("segment_commit", float(segment_commit_ms)),
        ("segment_receive_wait", float(segment_receive_wait_ms)),
        ("finish_opening", float(finish_opening_ms)),
        ("leaf_kernel", float(leaf_kernel_ms)),
        ("direct_d2h_wait", direct_d2h_wait_ms),
    ]
    name, value = max(candidates, key=lambda item: item[1])
    return name if value > 0.0 else "total" if total_ms > 0 else "unknown"


def cpu_trace_hotspot_hint(perf_hotspots: dict[str, float]) -> str:
    lowered_report_row_pct = perf_hotspots.get(
        PERF_LOWERED_REPORT_ROW_SELF_PCT_KEY, 0.0
    )
    memmove_pct = perf_hotspots.get(PERF_MEMMOVE_SELF_PCT_KEY, 0.0)
    pending_drop_pct = perf_hotspots.get(
        PERF_PENDING_SEGMENT_DROP_SELF_PCT_KEY, 0.0
    )
    if lowered_report_row_pct >= 20.0 and memmove_pct >= 15.0:
        return "report_lifetime_and_data_movement"
    if lowered_report_row_pct >= 20.0:
        return "lowered_report_rows"
    if memmove_pct >= 15.0:
        return "guest_state_copies"
    if pending_drop_pct >= 5.0:
        return "pending_segment_lifetime"
    return "none"


def memmove_source_hint(perf_hotspots: dict[str, float]) -> str:
    guest_machine_pct = perf_hotspots.get(PERF_MEMMOVE_GUEST_MACHINE_PCT_KEY, 0.0)
    trace_slice_pct = perf_hotspots.get(PERF_MEMMOVE_TRACE_SLICE_PCT_KEY, 0.0)
    if guest_machine_pct > 0.0 and trace_slice_pct > 0.0:
        if guest_machine_pct >= trace_slice_pct * 1.25:
            return "guest_machine_dominant"
        if trace_slice_pct >= guest_machine_pct * 1.25:
            return "trace_slice_dominant"
        return "guest_machine_and_trace_slice"
    if guest_machine_pct > 0.0:
        return "guest_machine"
    if trace_slice_pct > 0.0:
        return "trace_slice"
    return "none"


def root_pipeline_policy_hint(
    input_bytes: int,
    root_count: int,
    groups: int,
    max_group_size: int,
) -> str:
    if root_count <= 1:
        return "none"
    if groups < root_count or max_group_size > 1:
        return "root_batches_already_grouped"
    if input_bytes >= ROOT_PIPELINE_INPUT_BYTE_LIMIT:
        return "large_input_root_pipeline_gated"
    return "enable_cross_segment_root_pipeline"


def summarize_profile_values(
    label: str,
    values: dict[str, int],
    perf_hotspots: dict[str, float] | None = None,
) -> str:
    missing = [
        key
        for key in [ROOT_COUNT_KEY, ROOT_GROUPS_KEY, ROOT_MAX_GROUP_KEY]
        if key not in values
    ]
    if missing:
        raise SystemExit(f"{label}: missing timing fields: {', '.join(missing)}")

    input_bytes = values.get(INPUT_BYTES_KEY, 0)
    total_ms = values.get(TOTAL_MS_KEY, 0)
    runner_ms = values.get(RUNNER_MS_KEY, 0)
    lowerer_ms = values.get(LOWERER_MS_KEY, 0)
    stream_elapsed_ms = values.get(STREAM_ELAPSED_MS_KEY, 0)
    stream_worker_ms = values.get(STREAM_WORKER_MS_KEY, 0)
    segment_commit_ms = values.get(SEGMENT_COMMIT_MS_KEY, 0)
    stream_commit_residual_ms = (
        stream_elapsed_ms - stream_worker_ms - segment_commit_ms
    )
    segment_receive_wait_ms = values.get(SEGMENT_RECEIVE_WAIT_MS_KEY, 0)
    pending_receive_wait_ms = values.get(PENDING_RECEIVE_WAIT_MS_KEY, 0)
    pending_send_wait_ms = values.get(PENDING_SEND_WAIT_MS_KEY, 0)
    parallel_lower_workers = values.get(PARALLEL_LOWER_WORKERS_KEY, 0)
    parallel_lower_dispatched = values.get(PARALLEL_LOWER_DISPATCHED_KEY, 0)
    parallel_lower_received = values.get(PARALLEL_LOWER_RECEIVED_KEY, 0)
    parallel_lower_emitted = values.get(PARALLEL_LOWER_EMITTED_KEY, 0)
    parallel_lower_max_reorder = values.get(PARALLEL_LOWER_MAX_REORDER_KEY, 0)
    seed_direct_lift_attempts = values.get(SEED_DIRECT_LIFT_ATTEMPTS_KEY, 0)
    seed_direct_lift_successes = values.get(SEED_DIRECT_LIFT_SUCCESSES_KEY, 0)
    seed_full_advances = values.get(SEED_FULL_ADVANCES_KEY, 0)
    finish_opening_ms = values.get(FINISH_OPENING_MS_KEY, 0)
    root_count = values[ROOT_COUNT_KEY]
    groups = values[ROOT_GROUPS_KEY]
    max_group_size = values[ROOT_MAX_GROUP_KEY]
    roots_per_group = root_count / groups if groups else 0.0
    needs_cross_segment_root_pipeline = (
        "yes" if root_count > 1 and groups >= root_count and max_group_size <= 1 else "no"
    )
    policy_hint = root_pipeline_policy_hint(
        input_bytes, root_count, groups, max_group_size
    )
    leaf_kernel_ms = values.get(LEAF_KERNEL_MS_KEY, 0)
    leaf_coset_calls = values.get(LEAF_COSET_CALLS_KEY, 0)
    leaf_coset_columns = values.get(LEAF_COSET_COLUMNS_KEY, 0)
    leaf_ntt_launches = values.get(LEAF_NTT_LAUNCHES_KEY, 0)
    leaf_ntt_stage_launches = values.get(LEAF_NTT_STAGE_LAUNCHES_KEY, 0)
    leaf_ntt_block_twiddle_launches = values.get(LEAF_NTT_BLOCK_TWIDDLE_LAUNCHES_KEY, 0)
    ntt_launches_per_call = leaf_ntt_launches / leaf_coset_calls if leaf_coset_calls else 0.0
    direct_d2h_wait_ms = values.get(DIRECT_D2H_WAIT_NS_KEY, 0) / 1_000_000.0
    leaf_launch_pressure = "yes" if leaf_ntt_launches >= 10_000 else "no"
    trace_to_leaf_ratio = (
        max(runner_ms, lowerer_ms) / leaf_kernel_ms if leaf_kernel_ms else 0.0
    )
    bottleneck = primary_bottleneck(
        total_ms,
        runner_ms,
        lowerer_ms,
        stream_elapsed_ms,
        stream_worker_ms,
        segment_commit_ms,
        segment_receive_wait_ms,
        finish_opening_ms,
        leaf_kernel_ms,
        direct_d2h_wait_ms,
    )
    if perf_hotspots is None:
        perf_hotspots = parse_perf_self_hotspots("")
    lowered_report_row_pct = perf_hotspots.get(
        PERF_LOWERED_REPORT_ROW_SELF_PCT_KEY, 0.0
    )
    memmove_pct = perf_hotspots.get(PERF_MEMMOVE_SELF_PCT_KEY, 0.0)
    memmove_guest_machine_pct = perf_hotspots.get(
        PERF_MEMMOVE_GUEST_MACHINE_PCT_KEY, 0.0
    )
    memmove_trace_slice_pct = perf_hotspots.get(
        PERF_MEMMOVE_TRACE_SLICE_PCT_KEY, 0.0
    )
    memmove_hint = memmove_source_hint(perf_hotspots)
    pending_drop_pct = perf_hotspots.get(
        PERF_PENDING_SEGMENT_DROP_SELF_PCT_KEY, 0.0
    )
    cpu_hint = cpu_trace_hotspot_hint(perf_hotspots)
    return (
        f"{label},{input_bytes},{total_ms},{runner_ms},{lowerer_ms},"
        f"{stream_elapsed_ms},{stream_worker_ms},{segment_commit_ms},"
        f"{stream_commit_residual_ms},{segment_receive_wait_ms},"
        f"{pending_receive_wait_ms},{pending_send_wait_ms},"
        f"{parallel_lower_workers},{parallel_lower_dispatched},"
        f"{parallel_lower_received},{parallel_lower_emitted},"
        f"{parallel_lower_max_reorder},{seed_direct_lift_attempts},"
        f"{seed_direct_lift_successes},{seed_full_advances},"
        f"{finish_opening_ms},"
        f"{root_count},{groups},{max_group_size},"
        f"{roots_per_group:.3f},{needs_cross_segment_root_pipeline},{policy_hint},"
        f"{leaf_kernel_ms},{leaf_coset_calls},{leaf_coset_columns},{leaf_ntt_launches},"
        f"{leaf_ntt_stage_launches},{leaf_ntt_block_twiddle_launches},"
        f"{ntt_launches_per_call:.3f},{direct_d2h_wait_ms:.3f},{leaf_launch_pressure},"
        f"{trace_to_leaf_ratio:.3f},{bottleneck},{lowered_report_row_pct:.3f},"
        f"{memmove_pct:.3f},{memmove_guest_machine_pct:.3f},"
        f"{memmove_trace_slice_pct:.3f},{memmove_hint},"
        f"{pending_drop_pct:.3f},{cpu_hint}"
    )


def summarize_profile(label: str, text: str) -> str:
    return summarize_profile_values(
        label, parse_timing_log(text), parse_perf_self_hotspots(text)
    )


def median_int(values: list[int]) -> float:
    ordered = sorted(values)
    midpoint = len(ordered) // 2
    if len(ordered) % 2 == 1:
        return float(ordered[midpoint])
    return (ordered[midpoint - 1] + ordered[midpoint]) / 2.0


def summarize_total_samples(parsed_inputs: list[tuple[str, dict[str, int]]]) -> str:
    total_count = len(parsed_inputs)
    totals = [
        values[TOTAL_MS_KEY]
        for _, values in parsed_inputs
        if values.get(TOTAL_MS_KEY, 0) > 0
    ]
    valid_total_count = len(totals)
    if not totals:
        return f"aggregate,{total_count},0,0,0.000,0.000,0,0.000,no,no"

    total_min_ms = min(totals)
    total_mean_ms = sum(totals) / valid_total_count
    total_median_ms = median_int(totals)
    total_max_ms = max(totals)
    sample_spread_pct = (
        (total_max_ms - total_min_ms) * 100.0 / total_median_ms
        if total_median_ms
        else 0.0
    )
    close_samples = (
        "yes"
        if valid_total_count >= 3 and sample_spread_pct <= CLOSE_SAMPLE_SPREAD_PCT
        else "no"
    )
    max_outlier = (
        "yes"
        if valid_total_count >= 3 and total_max_ms > total_median_ms * OUTLIER_RATIO_THRESHOLD
        else "no"
    )
    return (
        f"aggregate,{total_count},{valid_total_count},{total_min_ms},"
        f"{total_mean_ms:.3f},{total_median_ms:.3f},{total_max_ms},"
        f"{sample_spread_pct:.3f},{close_samples},{max_outlier}"
    )


def print_summary(inputs: list[tuple[str, str]]) -> None:
    parsed_inputs = [
        (label, parse_timing_log(text), parse_perf_self_hotspots(text))
        for label, text in inputs
    ]
    print(HEADER)
    for label, values, perf_hotspots in parsed_inputs:
        print(summarize_profile_values(label, values, perf_hotspots))
    if len(parsed_inputs) > 1:
        print(AGGREGATE_HEADER)
        print(
            summarize_total_samples(
                [(label, values) for label, values, _ in parsed_inputs]
            )
        )


def self_test() -> None:
    print_summary(
        [
            (
                "single-root-groups",
                "\n".join(
                    [
                        f"{TOTAL_MS_KEY}=9050",
                        f"{RUNNER_MS_KEY}=7800",
                        f"{LOWERER_MS_KEY}=7812",
                        f"{STREAM_ELAPSED_MS_KEY}=9912",
                        f"{STREAM_WORKER_MS_KEY}=7812",
                        f"{SEGMENT_COMMIT_MS_KEY}=2100",
                        f"{SEGMENT_RECEIVE_WAIT_MS_KEY}=6000",
                        f"{PENDING_RECEIVE_WAIT_MS_KEY}=1200",
                        f"{PENDING_SEND_WAIT_MS_KEY}=345",
                        f"{PARALLEL_LOWER_WORKERS_KEY}=2",
                        f"{PARALLEL_LOWER_DISPATCHED_KEY}=23",
                        f"{PARALLEL_LOWER_RECEIVED_KEY}=23",
                        f"{PARALLEL_LOWER_EMITTED_KEY}=23",
                        f"{PARALLEL_LOWER_MAX_REORDER_KEY}=1",
                        f"{SEED_DIRECT_LIFT_ATTEMPTS_KEY}=22",
                        f"{SEED_DIRECT_LIFT_SUCCESSES_KEY}=22",
                        f"{SEED_FULL_ADVANCES_KEY}=1",
                        f"{FINISH_OPENING_MS_KEY}=476",
                        f"{INPUT_BYTES_KEY}=2758032",
                        f"{ROOT_COUNT_KEY}=23",
                        f"{ROOT_GROUPS_KEY}=23",
                        f"{ROOT_MAX_GROUP_KEY}=1",
                        f"{LEAF_KERNEL_MS_KEY}=858",
                        f"{LEAF_COSET_CALLS_KEY}=23",
                        f"{LEAF_COSET_COLUMNS_KEY}=874",
                        f"{LEAF_NTT_LAUNCHES_KEY}=41078",
                        f"{LEAF_NTT_STAGE_LAUNCHES_KEY}=15732",
                        f"{LEAF_NTT_BLOCK_TWIDDLE_LAUNCHES_KEY}=23598",
                        f"{DIRECT_D2H_WAIT_NS_KEY}=192973857",
                        "    26.35%  [.] lzvm_prover::guest_pc_trace_backend::apply_main_lowered_report_row",
                        "    20.94%  [.] __memmove_avx512_unaligned_erms",
                        "            |--10.61%--lzvm_prover::guest_machine::advance_guest_machine_prepared_inner",
                        "             --8.67%--lzvm_prover::guest_pc_trace_backend::run_guest_pc_trace_segment_slice",
                        "     7.41%  [.] core::ptr::drop_in_place<lzvm_prover::guest_pc_trace_backend::GuestPcTracePendingSegmentSlice>",
                    ]
                ),
            ),
            (
                "batched-roots",
                "\n".join(
                    [
                        f"{TOTAL_MS_KEY}=9050",
                        f"{INPUT_BYTES_KEY}=2758032",
                        f"{ROOT_COUNT_KEY}=23",
                        f"{ROOT_GROUPS_KEY}=1",
                        f"{ROOT_MAX_GROUP_KEY}=23",
                    ]
                ),
            ),
            (
                "slow-sample",
                "\n".join(
                    [
                        f"{TOTAL_MS_KEY}=18100",
                        f"{INPUT_BYTES_KEY}=12447640",
                        f"{ROOT_COUNT_KEY}=120",
                        f"{ROOT_GROUPS_KEY}=120",
                        f"{ROOT_MAX_GROUP_KEY}=1",
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
