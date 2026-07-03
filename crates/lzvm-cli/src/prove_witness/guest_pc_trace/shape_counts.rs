use std::time::Duration;

use super::TimingRecorder;

pub(in crate::prove_witness) fn record_guest_stage_root_materialization_shape(
    timings: &mut TimingRecorder,
    root_count: usize,
    group_count: usize,
    max_group_size: usize,
) {
    let avg_group_size_milli = if group_count == 0 {
        0
    } else {
        root_count.saturating_mul(1000) / group_count
    };
    let needs_cross_segment_pipeline =
        usize::from(root_count > 1 && group_count == root_count && max_group_size <= 1);
    timings.record_count(
        "guest_stage_tree_commit_root_materialization_avg_group_size_milli",
        avg_group_size_milli,
    );
    timings.record_count(
        "guest_stage_tree_commit_root_materialization_needs_cross_segment_pipeline",
        needs_cross_segment_pipeline,
    );
}

pub(in crate::prove_witness) fn record_guest_trace_sampled_duration_counts(
    timings: &mut TimingRecorder,
    name: &'static str,
    duration: Duration,
    sample_count: usize,
) {
    if sample_count == 0 {
        return;
    }
    let sampled_ns = duration_ns(duration);
    timings.record_count_dynamic(format!("{name}_sampled_ns"), sampled_ns);
    timings.record_count_dynamic(format!("{name}_avg_sample_ns"), sampled_ns / sample_count);
}

fn duration_ns(duration: Duration) -> usize {
    usize::try_from(duration.as_nanos()).unwrap_or(usize::MAX)
}
