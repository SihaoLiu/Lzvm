use super::timing::TimingRecorder;

pub(super) fn record_proof_artifact_timing(
    timings: &mut TimingRecorder,
    timing: &lzvm_prover::WitnessProofArtifactTiming,
) {
    timings.record("finish_query_plan", timing.query_plan);
    timings.record("finish_constant_opening", timing.constant_opening);
    timings.record("finish_witness_opening", timing.witness_opening);
    timings.record_count(
        "finish_witness_opening_query_count",
        timing.witness_opening_query_count,
    );
    timings.record_count(
        "finish_witness_opening_stage_count",
        timing.witness_opening_stage_count,
    );
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
    timings.record_count(
        "finish_witness_opening_leaf_hash_rows",
        timing.witness_opening_leaf_hash_row_count,
    );
    timings.record_count(
        "finish_witness_opening_leaf_hash_bytes",
        timing.witness_opening_leaf_hash_byte_count,
    );
    timings.record_count(
        "finish_witness_opening_leaf_hash_arity2_rows",
        timing.witness_opening_leaf_hash_arity2_row_count,
    );
    timings.record_count(
        "finish_witness_opening_leaf_hash_arity2_bytes",
        timing.witness_opening_leaf_hash_arity2_byte_count,
    );
    timings.record_count(
        "finish_witness_opening_leaf_hash_arity4_rows",
        timing.witness_opening_leaf_hash_arity4_row_count,
    );
    timings.record_count(
        "finish_witness_opening_leaf_hash_arity4_bytes",
        timing.witness_opening_leaf_hash_arity4_byte_count,
    );
    timings.record_count(
        "finish_witness_opening_leaf_coset_extend_calls",
        timing.witness_opening_leaf_coset_extend_call_count,
    );
    timings.record_count(
        "finish_witness_opening_leaf_coset_extend_output_bytes",
        timing.witness_opening_leaf_coset_extend_output_byte_count,
    );
    timings.record_count(
        "finish_witness_opening_leaf_coset_extend_columns",
        timing.witness_opening_leaf_coset_extend_column_count,
    );
    timings.record_count(
        "finish_witness_opening_leaf_coset_extend_max_columns",
        timing.witness_opening_leaf_coset_extend_max_column_count,
    );
    timings.record_count(
        "finish_witness_opening_leaf_coset_extend_ntt_launches",
        timing.witness_opening_leaf_coset_extend_ntt_launch_count,
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
    for stage_work in &timing.witness_stage_opening_work {
        timings.record_count_dynamic(
            format!(
                "finish_witness_stage_{}_opening_leaf_hash_rows",
                stage_work.stage_index
            ),
            stage_work.leaf_hash_row_count,
        );
        timings.record_count_dynamic(
            format!(
                "finish_witness_stage_{}_opening_leaf_hash_bytes",
                stage_work.stage_index
            ),
            stage_work.leaf_hash_byte_count,
        );
        timings.record_count_dynamic(
            format!(
                "finish_witness_stage_{}_opening_leaf_coset_extend_calls",
                stage_work.stage_index
            ),
            stage_work.leaf_coset_extend_call_count,
        );
        timings.record_count_dynamic(
            format!(
                "finish_witness_stage_{}_opening_leaf_coset_extend_output_bytes",
                stage_work.stage_index
            ),
            stage_work.leaf_coset_extend_output_byte_count,
        );
        timings.record_count_dynamic(
            format!(
                "finish_witness_stage_{}_opening_leaf_coset_extend_columns",
                stage_work.stage_index
            ),
            stage_work.leaf_coset_extend_column_count,
        );
        timings.record_count_dynamic(
            format!(
                "finish_witness_stage_{}_opening_leaf_coset_extend_max_columns",
                stage_work.stage_index
            ),
            stage_work.leaf_coset_extend_max_column_count,
        );
        timings.record_count_dynamic(
            format!(
                "finish_witness_stage_{}_opening_leaf_coset_extend_ntt_launches",
                stage_work.stage_index
            ),
            stage_work.leaf_coset_extend_ntt_launch_count,
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
    timings.record(
        "finish_fri_opening_unit_build",
        timing.fri_opening_unit_build,
    );
    timings.record(
        "finish_fri_opening_layer_tree",
        timing.fri_opening_layer_tree,
    );
    timings.record("finish_fri_opening_query", timing.fri_opening_query);
    timings.record("finish_fri_opening_fold", timing.fri_opening_fold);
    timings.record_count(
        "finish_fri_opening_unit_count",
        timing.fri_opening_unit_count,
    );
    timings.record_count(
        "finish_fri_opening_layer_count",
        timing.fri_opening_layer_count,
    );
    timings.record_count(
        "finish_fri_opening_query_count",
        timing.fri_opening_query_count,
    );
}
