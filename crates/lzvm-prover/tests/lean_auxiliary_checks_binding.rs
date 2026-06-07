use std::path::Path;

#[path = "support/lean_binding.rs"]
mod lean_binding;

#[test]
fn lean_auxiliary_checks_binding_exports_core_contract_projections() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_path = crate_root.join("../../lean/Lzvm/AuxiliaryChecks.lean");
    let lean_source =
        std::fs::read_to_string(&lean_path).expect("Lean auxiliary checks should read");
    let top_level_path = crate_root.join("../../lean/Lzvm.lean");
    let top_level_source =
        std::fs::read_to_string(&top_level_path).expect("Lean top-level source should read");
    let guest_pc_timing_path = crate_root.join("../lzvm-cli/src/prove_witness/guest_pc_trace.rs");
    let guest_pc_timing_source =
        std::fs::read_to_string(&guest_pc_timing_path).expect("guest PC timing source should read");
    let proof_timing_path = crate_root.join("../lzvm-cli/src/prove_witness/proof_timing.rs");
    let proof_timing_source =
        std::fs::read_to_string(&proof_timing_path).expect("proof timing source should read");

    assert!(
        top_level_source.contains("import Lzvm.AuxiliaryChecks"),
        "top-level Lean module should import auxiliary checks"
    );
    assert!(
        lean_source.contains("SourceLookupCheckedAcceptance")
            && lean_source.contains("WitnessLeafDigestCheckedAcceptance")
            && lean_source.contains("GpuCanonicalLeafCheckedAcceptance")
            && lean_source.contains("TimingObservedAcceptance")
            && lean_source.contains("GuestPcTraceTimingObservedAcceptance")
            && lean_source.contains("WitnessOpeningRowValueTimingSummary")
            && lean_source.contains("WitnessOpeningRowValueTimingObservedAcceptance")
            && lean_source.contains("GpuSetupCheckedAcceptance")
            && lean_source.contains("GpuAllocationCheckedAcceptance")
            && lean_source.contains("RuntimeVerifierCoreContract system publicInput proof")
            && lean_source.contains("SoundWitness system publicInput proof"),
        "Lean auxiliary checks should expose checked acceptance structures and verifier core clauses"
    );
    for field in [
        "sourceExtendMilliseconds",
        "sourceDownloadMilliseconds",
        "deviceDownloadMilliseconds",
        "rowValueSourceExtendMilliseconds",
        "rowValueSourceDownloadMilliseconds",
        "rowValueDeviceDownloadMilliseconds",
        "guestTraceRunnerMilliseconds",
        "guestTraceLowererMilliseconds",
        "guestTraceLowerMilliseconds",
        "guestTraceReportMilliseconds",
        "guestTraceEmitMilliseconds",
        "guestTraceDescriptorMilliseconds",
        "guestTracePendingSendWaitMilliseconds",
        "guestTracePendingReceiveWaitMilliseconds",
        "guestTraceSegmentSendWaitMilliseconds",
        "guestTraceSegmentReceiveWaitMilliseconds",
        "guestDeviceSourceBuildMilliseconds",
        "guestDeviceSourceDescriptorUploadMilliseconds",
        "guestDeviceSourceDescriptorUploadByteCount",
        "guestDeviceSourceDescriptorUploadRowCount",
        "guestDeviceSourceTraceExpandMilliseconds",
        "guestStageSourceRetentionAttemptCount",
        "guestStageSourceRetentionRetainedCount",
        "guestStageSourceRetentionRejectedCount",
        "guestStageSourceRetentionRejectedByteCount",
        "guestStageSourceRetentionLimitByteCount",
        "guestDescriptorBufferRetentionAttemptCount",
        "guestDescriptorBufferRetentionRetainedCount",
        "guestDescriptorBufferRetentionRejectedCount",
        "guestDescriptorBufferRetentionRetainedByteCount",
        "guestDescriptorBufferRetentionRejectedByteCount",
    ] {
        assert!(
            lean_source.contains(field),
            "Lean guest PC trace timing summary should expose {field}"
        );
    }
    for (line_name, accessor) in [
        ("\"guest_trace_runner\"", "guest_trace_runner_duration()"),
        ("\"guest_trace_lowerer\"", "guest_trace_lowerer_duration()"),
        ("\"guest_trace_lower\"", "guest_trace_lower_duration()"),
        ("\"guest_trace_report\"", "guest_trace_report_duration()"),
        ("\"guest_trace_emit\"", "guest_trace_emit_duration()"),
        (
            "\"guest_trace_descriptor\"",
            "guest_trace_descriptor_duration()",
        ),
        (
            "\"guest_trace_pending_send_wait\"",
            "guest_trace_pending_send_wait_duration()",
        ),
        (
            "\"guest_trace_pending_receive_wait\"",
            "guest_trace_pending_receive_wait_duration()",
        ),
        (
            "\"guest_trace_segment_send_wait\"",
            "guest_trace_segment_send_wait_duration()",
        ),
        (
            "\"guest_trace_segment_receive_wait\"",
            "guest_trace_segment_receive_wait_duration()",
        ),
        (
            "\"guest_device_source_build\"",
            "guest_device_source_build_duration()",
        ),
        (
            "\"guest_device_source_descriptor_upload\"",
            "guest_device_source_descriptor_upload_duration()",
        ),
        (
            "\"guest_device_source_descriptor_upload_bytes\"",
            "guest_device_source_descriptor_upload_byte_count()",
        ),
        (
            "\"guest_device_source_descriptor_upload_rows\"",
            "guest_device_source_descriptor_upload_row_count()",
        ),
        (
            "\"guest_device_source_trace_expand\"",
            "guest_device_source_trace_expand_duration()",
        ),
        (
            "\"guest_stage_source_retention_attempts\"",
            "guest_stage_source_retention_attempt_count()",
        ),
        (
            "\"guest_stage_source_retention_retained\"",
            "guest_stage_source_retention_retained_count()",
        ),
        (
            "\"guest_stage_source_retention_rejected\"",
            "guest_stage_source_retention_rejected_count()",
        ),
        (
            "\"guest_stage_source_retention_rejected_bytes\"",
            "guest_stage_source_retention_rejected_byte_count()",
        ),
        (
            "\"guest_stage_source_retention_limit_bytes\"",
            "guest_stage_source_retention_limit_byte_count()",
        ),
        (
            "\"guest_descriptor_buffer_retention_attempts\"",
            "guest_descriptor_buffer_retention_attempt_count()",
        ),
        (
            "\"guest_descriptor_buffer_retention_retained\"",
            "guest_descriptor_buffer_retention_retained_count()",
        ),
        (
            "\"guest_descriptor_buffer_retention_rejected\"",
            "guest_descriptor_buffer_retention_rejected_count()",
        ),
        (
            "\"guest_descriptor_buffer_retention_retained_bytes\"",
            "guest_descriptor_buffer_retention_retained_byte_count()",
        ),
        (
            "\"guest_descriptor_buffer_retention_rejected_bytes\"",
            "guest_descriptor_buffer_retention_rejected_byte_count()",
        ),
    ] {
        assert!(
            guest_pc_timing_source.contains(line_name) && guest_pc_timing_source.contains(accessor),
            "CLI guest PC timing output should include {line_name}"
        );
    }
    for (line_name, field) in [
        (
            "\"finish_witness_opening_row_value_source_extend\"",
            "witness_opening_row_values_source_extend",
        ),
        (
            "\"finish_witness_opening_row_value_source_download\"",
            "witness_opening_row_values_source_download",
        ),
        (
            "\"finish_witness_opening_row_value_device_download\"",
            "witness_opening_row_values_device_download",
        ),
    ] {
        assert!(
            proof_timing_source.contains(line_name) && proof_timing_source.contains(field),
            "CLI proof timing output should include {line_name}"
        );
    }
    lean_binding::assert_theorem_declarations(
        &lean_source,
        &[
            "source_lookup_auxiliary_acceptance_sound",
            "source_lookup_checked_acceptance_verifier_core_contract",
            "witness_leaf_digest_acceptance_sound",
            "witness_leaf_digest_checked_acceptance_verifier_core_contract",
            "gpu_canonical_leaf_checked_acceptance_sound",
            "gpu_canonical_leaf_checked_acceptance_verifier_core_contract",
            "timing_observation_acceptance_sound",
            "timing_observation_acceptance_verifier_core_contract",
            "guest_pc_trace_timing_acceptance_sound",
            "guest_pc_trace_timing_acceptance_verifier_core_contract",
            "witness_opening_row_value_timing_acceptance_sound",
            "witness_opening_row_value_timing_acceptance_verifier_core_contract",
            "gpu_setup_checked_acceptance_sound",
            "gpu_setup_checked_acceptance_verifier_core_contract",
            "gpu_allocation_checked_acceptance_sound",
            "gpu_allocation_checked_acceptance_verifier_core_contract",
        ],
    );
}
