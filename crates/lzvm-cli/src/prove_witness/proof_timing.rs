use super::timing::TimingRecorder;

pub(super) fn record_proof_artifact_timing(
    timings: &mut TimingRecorder,
    timing: &lzvm_prover::WitnessProofArtifactTiming,
) {
    timings.record("finish_query_plan", timing.query_plan);
    timings.record("finish_constant_opening", timing.constant_opening);
    timings.record("finish_witness_opening", timing.witness_opening);
    timings.record(
        "finish_witness_external_source",
        timing.witness_external_source,
    );
    timings.record("finish_witness_opening_setup", timing.witness_opening_setup);
    timings.record(
        "finish_witness_opening_leaf_extend",
        timing.witness_opening_leaf_extend,
    );
    timings.record(
        "finish_witness_opening_leaf_hash",
        timing.witness_opening_leaf_hash,
    );
    timings.record("finish_witness_opening_path", timing.witness_opening_path);
    timings.record(
        "finish_witness_opening_row_values",
        timing.witness_opening_row_values,
    );
    for stage_timing in &timing.witness_stage_external_source {
        timings.record_dynamic(
            format!(
                "finish_witness_stage_{}_external_source",
                stage_timing.stage_index
            ),
            stage_timing.duration,
        );
    }
    for stage_timing in &timing.witness_stage_opening_setup {
        timings.record_dynamic(
            format!(
                "finish_witness_stage_{}_opening_setup",
                stage_timing.stage_index
            ),
            stage_timing.duration,
        );
    }
    for stage_timing in &timing.witness_stage_opening_leaf_extend {
        timings.record_dynamic(
            format!(
                "finish_witness_stage_{}_opening_leaf_extend",
                stage_timing.stage_index
            ),
            stage_timing.duration,
        );
    }
    for stage_timing in &timing.witness_stage_opening_leaf_hash {
        timings.record_dynamic(
            format!(
                "finish_witness_stage_{}_opening_leaf_hash",
                stage_timing.stage_index
            ),
            stage_timing.duration,
        );
    }
    for stage_timing in &timing.witness_stage_opening_path {
        timings.record_dynamic(
            format!(
                "finish_witness_stage_{}_opening_path",
                stage_timing.stage_index
            ),
            stage_timing.duration,
        );
    }
    for stage_timing in &timing.witness_stage_opening_row_values {
        timings.record_dynamic(
            format!(
                "finish_witness_stage_{}_opening_row_values",
                stage_timing.stage_index
            ),
            stage_timing.duration,
        );
    }
    for stage_timing in &timing.witness_stage_opening {
        timings.record_dynamic(
            format!("finish_witness_stage_{}_opening", stage_timing.stage_index),
            stage_timing.duration,
        );
    }
    timings.record("finish_fri_opening", timing.fri_opening);
}
