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
    let prove_witness_path = crate_root.join("../lzvm-cli/src/prove_witness.rs");
    let prove_witness_source =
        std::fs::read_to_string(&prove_witness_path).expect("prove witness source should read");
    let proof_timing_path = crate_root.join("../lzvm-cli/src/prove_witness/proof_timing.rs");
    let proof_timing_source =
        std::fs::read_to_string(&proof_timing_path).expect("proof timing source should read");
    let cli_timing_path = crate_root.join("../lzvm-cli/src/prove_witness/timing.rs");
    let cli_timing_source =
        std::fs::read_to_string(&cli_timing_path).expect("CLI timing source should read");
    let fri_fold_path = crate_root.join("src/pcs_fri/fold.rs");
    let fri_fold_source =
        std::fs::read_to_string(&fri_fold_path).expect("FRI fold source should read");
    let fri_polynomial_path = crate_root.join("src/prove_fri_polynomial.rs");
    let fri_polynomial_source =
        std::fs::read_to_string(&fri_polynomial_path).expect("FRI polynomial source should read");
    let fri_opening_path = crate_root.join("src/prove_fri_opening.rs");
    let fri_opening_source =
        std::fs::read_to_string(&fri_opening_path).expect("FRI opening source should read");
    let merkle_path = crate_root.join("src/merkle_hash.rs");
    let merkle_source =
        std::fs::read_to_string(&merkle_path).expect("Merkle hash source should read");
    let cuda_field_test_path = crate_root.join("../lzvm-accel/tests/cuda_field.rs");
    let cuda_field_test_source =
        std::fs::read_to_string(&cuda_field_test_path).expect("CUDA field tests should read");
    let source_hot_paths_path = crate_root.join("tests/source_hot_paths.rs");
    let source_hot_paths =
        std::fs::read_to_string(&source_hot_paths_path).expect("source hot paths should read");

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
            && lean_source.contains("ConstantMaterialValidationTimingSummary")
            && lean_source.contains("ConstantMaterialValidationTimingObservedAcceptance")
            && lean_source.contains("ProverGpuModeSummary")
            && lean_source.contains("ProverGpuModeObservedAcceptance")
            && lean_source.contains("GpuSetupCheckedAcceptance")
            && lean_source.contains("GpuAllocationCheckedAcceptance")
            && lean_source.contains("GpuHostDeviceCopyRoundTripValidation")
            && lean_source.contains("GpuHostDeviceCopyRoundTripCheckedAcceptance")
            && lean_source.contains("FriFixedColumnCacheValidation")
            && lean_source.contains("FriFixedColumnCacheCheckedAcceptance")
            && lean_source.contains("GpuCosetExtensionValidation")
            && lean_source.contains("GpuCosetExtensionCheckedAcceptance")
            && lean_source.contains("GpuFriFoldInterpolationValidation")
            && lean_source.contains("GpuFriFoldInterpolationCheckedAcceptance")
            && lean_source.contains("GpuMerkleDigestPrefixBatchValidation")
            && lean_source.contains("GpuMerkleDigestPrefixBatchCheckedAcceptance")
            && lean_source.contains("RuntimeVerifierCoreContract system publicInput proof")
            && lean_source.contains("SoundWitness system publicInput proof"),
        "Lean auxiliary checks should expose checked acceptance structures and verifier core clauses"
    );
    assert!(
        lean_source.contains("uploadedBytesRoundTrip")
            && lean_source.contains("roundTripImpliesWrittenContents"),
        "Lean auxiliary checks should bind GPU copy roundtrip evidence to written contents"
    );
    assert!(
        lean_source.contains("cosetExtensionMatchesHost")
            && lean_source.contains("cosetExtensionImpliesCanonicalLeafBytes"),
        "Lean auxiliary checks should bind GPU coset extension evidence to canonical leaf bytes"
    );
    assert!(
        lean_source.contains("gpuFriInterpolationMatchesHost")
            && lean_source.contains("gpuFriInterpolationImpliesFriFoldsValid"),
        "Lean auxiliary checks should bind GPU FRI interpolation evidence to FRI fold validity"
    );
    assert!(
        lean_source.contains("gpuMerkleDigestPrefixBatchMatchesSinglePaths")
            && lean_source.contains("gpuMerkleDigestPrefixBatchImpliesLowerPrefixesBound"),
        "Lean auxiliary checks should bind GPU Merkle prefix batches to lower-prefix path evidence"
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
    for field in [
        "constantMaterialValidationElapsedMilliseconds",
        "constantMaterialValidationUnitCount",
        "constantMaterialValidationByteCount",
    ] {
        assert!(
            lean_source.contains(field),
            "Lean constant material validation timing summary should expose {field}"
        );
    }
    for line_name in [
        "\"constant_material_validation_elapsed\"",
        "\"constant_material_validation_units\"",
        "\"constant_material_validation_bytes\"",
    ] {
        assert!(
            prove_witness_source.contains(line_name),
            "CLI constant material validation timing output should include {line_name}"
        );
    }
    assert!(
        cli_timing_source.contains("\"prover_gpu_mode={}\"")
            && cli_timing_source.contains("prover_gpu_mode()"),
        "CLI timing output should include the prover GPU mode summary"
    );
    assert!(
        lean_source.contains("proverGpuModeName"),
        "Lean prover GPU mode summary should expose the reported mode name"
    );
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
    assert!(
        cuda_field_test_source.contains("cuda_device_buffer_round_trips_large_bytes")
            && cuda_field_test_source.contains("large host bytes should copy to device"),
        "CUDA tests should cover large host-device byte roundtrips"
    );
    assert!(
        cuda_field_test_source.contains("cuda_extends_evaluations_over_shifted_cosets")
            && cuda_field_test_source.contains("cuda_extends_row_major_columns_from_device_memory")
            && cuda_field_test_source
                .contains("cuda_extends_row_major_coset_row_range_from_device_memory")
            && cuda_field_test_source
                .contains("cuda_extends_selected_strided_row_major_coset_rows_from_device_memory"),
        "CUDA tests should cover GPU coset extension outputs and selected row ranges"
    );
    assert!(
        fri_fold_source.contains("cuda_goldilocks_intt(&raw, bits)")
            && fri_fold_source.contains("interpolate_fold_column")
            && cuda_field_test_source.contains("cuda_computes_inverse_ntt"),
        "runtime FRI fold interpolation should use CUDA INTT with CUDA correctness coverage"
    );
    assert!(
        fri_polynomial_source.contains("struct PcsFriFixedColumnsCache")
            && fri_polynomial_source.contains("struct PcsFriFixedColumnsCacheKey")
            && fri_polynomial_source.contains("source_device: Option<CudaDeviceBuffer>")
            && fri_polynomial_source.contains("entry.key == key")
            && fri_polynomial_source.contains("Some(&mut entry.source_device)")
            && fri_polynomial_source.contains("path: plan_unit.fixed_columns.clone()")
            && fri_polynomial_source.contains("fixed_column_count: plan_unit.fixed_column_count")
            && fri_polynomial_source.contains("source_rows")
            && fri_polynomial_source.contains("source_bits")
            && fri_polynomial_source.contains("target_bits")
            && fri_polynomial_source.contains("digest: plan_unit.pcs_material_fixed_column_digest")
            && fri_opening_source
                .contains("build_pcs_fri_transcript_values_from_trace_refs_with_fixed_cache")
            && fri_opening_source
                .contains("let mut fixed_columns_cache = PcsFriFixedColumnsCache::default()")
            && source_hot_paths.contains("cuda_fri_polynomial_reuses_fixed_source_device_cache"),
        "runtime FRI fixed-column cache should reuse CUDA source buffers only for matching fixed-column requests"
    );
    assert!(
        merkle_source.contains("opening_path_prefix_batch_for_source_rows")
            && merkle_source
                .contains("cuda_poseidon2_width8_merkle_digest_opening_prefix_batch_device")
            && merkle_source
                .contains("cuda_poseidon2_width16_merkle_digest_opening_prefix_batch_device")
            && cuda_field_test_source.contains(
                "cuda_poseidon2_width8_merkle_digest_opening_prefix_batch_device_matches_path_prefixes"
            )
            && cuda_field_test_source.contains(
                "cuda_poseidon2_width16_merkle_digest_opening_prefix_batch_device_matches_path_prefixes"
            ),
        "runtime retained checkpoint lower-prefix openings should use CUDA prefix batches with CUDA correctness coverage"
    );
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
            "constant_material_validation_timing_acceptance_sound",
            "constant_material_validation_timing_acceptance_verifier_core_contract",
            "prover_gpu_mode_acceptance_sound",
            "prover_gpu_mode_acceptance_verifier_core_contract",
            "gpu_setup_checked_acceptance_sound",
            "gpu_setup_checked_acceptance_verifier_core_contract",
            "gpu_allocation_checked_acceptance_sound",
            "gpu_allocation_checked_acceptance_verifier_core_contract",
            "gpu_host_device_copy_round_trip_implies_written_contents",
            "gpu_host_device_copy_round_trip_checked_acceptance_sound",
            "gpu_host_device_copy_round_trip_checked_acceptance_verifier_core_contract",
            "fri_fixed_column_cache_same_request_implies_cached_contents_bound",
            "fri_fixed_column_cache_checked_acceptance_sound",
            "fri_fixed_column_cache_checked_acceptance_verifier_core_contract",
            "gpu_coset_extension_matches_host_implies_leaf_bytes",
            "gpu_coset_extension_checked_acceptance_sound",
            "gpu_coset_extension_checked_acceptance_verifier_core_contract",
            "gpu_fri_interpolation_matches_host_implies_fri_folds_valid",
            "gpu_fri_fold_interpolation_checked_acceptance_sound",
            "gpu_fri_fold_interpolation_checked_acceptance_verifier_core_contract",
            "gpu_merkle_digest_prefix_batch_matches_single_paths_implies_lower_prefixes_bound",
            "gpu_merkle_digest_prefix_batch_checked_acceptance_sound",
            "gpu_merkle_digest_prefix_batch_checked_acceptance_verifier_core_contract",
        ],
    );
}
