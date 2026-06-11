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
    let prove_plan_path = crate_root.join("../lzvm-cli/src/prove_plan.rs");
    let prove_plan_source =
        std::fs::read_to_string(&prove_plan_path).expect("prove plan source should read");
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
    let witness_values_path = crate_root.join("src/witness_commitment/values.rs");
    let witness_values_source =
        std::fs::read_to_string(&witness_values_path).expect("witness values source should read");

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
            && lean_source.contains("CudaBackendSummary")
            && lean_source.contains("CudaBackendObservedAcceptance")
            && lean_source.contains("CudaAllocatorTimingSummary")
            && lean_source.contains("CudaAllocatorTimingObservedAcceptance")
            && lean_source.contains("RuntimePerformanceObservationSummary")
            && lean_source.contains("RuntimePerformanceObservedAcceptance")
            && lean_source.contains("GpuSetupCheckedAcceptance")
            && lean_source.contains("GpuAllocationCheckedAcceptance")
            && lean_source.contains("GpuHostDeviceCopyRoundTripValidation")
            && lean_source.contains("GpuHostDeviceCopyRoundTripCheckedAcceptance")
            && lean_source.contains("GpuTemporaryBufferReuseValidation")
            && lean_source.contains("GpuTemporaryBufferReuseCheckedAcceptance")
            && lean_source.contains("GpuAllocatorNoWaitBypassValidation")
            && lean_source.contains("GpuAllocatorNoWaitBypassCheckedAcceptance")
            && lean_source.contains("GpuRetainedDeviceCacheBudgetValidation")
            && lean_source.contains("GpuRetainedDeviceCacheBudgetCheckedAcceptance")
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
        lean_source.contains("temporaryBufferReuseAllowed")
            && lean_source.contains("pendingDeviceReadsComplete")
            && lean_source.contains("temporaryBufferReuseImpliesSameRequest")
            && lean_source.contains("temporaryBufferReuseImpliesPendingReadsComplete"),
        "Lean auxiliary checks should bind temporary GPU buffer reuse to same requests and completed pending reads"
    );
    assert!(
        lean_source.contains("noWaitBypassAllowed")
            && lean_source.contains("pendingAllocationNotReused")
            && lean_source.contains("freshAllocationIssued")
            && lean_source.contains("noWaitBypassImpliesSameRequest")
            && lean_source.contains("noWaitBypassImpliesPendingNotReused")
            && lean_source.contains("noWaitBypassImpliesFreshAllocation"),
        "Lean auxiliary checks should bind allocator no-wait bypass to skipped pending allocations and fresh requests"
    );
    assert!(
        lean_source.contains("GpuRetainedDeviceCacheBudget")
            && lean_source.contains("sourceBytes")
            && lean_source.contains("leafDigestBytes")
            && lean_source.contains("sourceLimit")
            && lean_source.contains("leafDigestLimit")
            && lean_source.contains("combinedLimit")
            && lean_source.contains("GpuRetainedDeviceCacheBudgetWithinLimits")
            && lean_source.contains("retainedDeviceCacheBudgetAccepted")
            && lean_source.contains("retainedDeviceCacheBudgetImpliesWithinLimits")
            && witness_values_source.contains("retained_combined_device_cache_allows")
            && witness_values_source.contains("reserve_retained_device_bytes")
            && witness_values_source.contains("reserve_retained_leaf_digest_bytes"),
        "Lean auxiliary checks should bind retained source and leaf digest cache retention to runtime budget limits"
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
        "GuestPcTraceStageTimingSummary",
        "stageTimings : List GuestPcTraceStageTimingSummary",
        "stageIndex",
        "leafExtendWorkMilliseconds",
        "leafSetupWorkMilliseconds",
        "leafSetupPrepareMilliseconds",
        "leafSetupOutputAllocMilliseconds",
        "leafSetupWorkspaceAllocMilliseconds",
        "leafSetupOutputAllocByteCount",
        "leafSetupWorkspaceAllocByteCount",
        "leafSetupOutputAllocCount",
        "leafSetupWorkspaceAllocCount",
        "leafUploadWorkMilliseconds",
        "leafKernelWorkMilliseconds",
        "leafDownloadWorkMilliseconds",
        "leafValidateWorkMilliseconds",
        "leafHashWorkMilliseconds",
        "leafHashArity2RowCount",
        "leafHashArity2ByteCount",
        "leafHashArity4RowCount",
        "leafHashArity4ByteCount",
        "leafCosetExtendCallCount",
        "leafCosetExtendOutputByteCount",
        "leafCosetExtendColumnCount",
        "leafCosetExtendMaxColumnCount",
        "leafCosetExtendNttLaunchCount",
        "leafCosetExtendBitReverseLaunchCount",
        "leafCosetExtendNttStageLaunchCount",
        "leafCosetExtendNttBlockTwiddleLaunchCount",
        "leafCosetExtendNormalizeLaunchCount",
        "leafCosetExtendPackLaunchCount",
        "leafCosetExtendUnpackLaunchCount",
        "treeCommitWorkMilliseconds",
        "treeCommitCheckpointWorkMilliseconds",
        "treeCommitRootWorkMilliseconds",
        "treeCommitRetainWorkMilliseconds",
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
        "guestTraceReportValidationMilliseconds",
        "guestTraceReportLoweringMilliseconds",
        "guestTraceReportRowValidationMilliseconds",
        "guestTraceReportSourceValuesMilliseconds",
        "guestTraceReportPrecompileMemoryMilliseconds",
        "guestTraceReportInstructionResultMilliseconds",
        "guestTraceReportNextPcMilliseconds",
        "guestTraceReportRegisterAccessMilliseconds",
        "guestTraceReportMemoryAccessMilliseconds",
        "guestTraceReportStoreApplyMilliseconds",
        "guestTraceReportVisitMilliseconds",
        "guestTraceSingleRowReportLowerMilliseconds",
        "guestTraceMultiRowReportLowerMilliseconds",
        "guestTracePendingDmaReportLowerMilliseconds",
        "guestTraceAmoReportLowerMilliseconds",
        "guestTraceStoreConditionalReportLowerMilliseconds",
        "guestTraceReportCount",
        "guestTraceReportRowCount",
        "guestTraceSingleRowReportCount",
        "guestTraceMultiRowReportCount",
        "guestTracePendingDmaReportCount",
        "guestTraceAmoReportCount",
        "guestTraceStoreConditionalReportCount",
        "guestTraceExternalOpRowCount",
        "guestTraceCopyRowCount",
        "guestTraceFlagRowCount",
        "guestTracePrecompileRowCount",
        "guestTraceIndirectMemoryRowCount",
        "guestTraceRegisterSourceReadCount",
        "guestTraceMemorySourceReadCount",
        "guestTraceRegisterStoreRowCount",
        "guestTraceMemoryStoreRowCount",
        "guestTraceNoStoreRowCount",
        "guestTraceEmitMilliseconds",
        "guestTraceDescriptorMilliseconds",
        "guestTraceDescriptorRowCount",
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
        "guestStageLeafExtendWorkMilliseconds",
        "guestStageLeafSetupWorkMilliseconds",
        "guestStageLeafSetupPrepareMilliseconds",
        "guestStageLeafSetupOutputAllocMilliseconds",
        "guestStageLeafSetupWorkspaceAllocMilliseconds",
        "guestStageLeafSetupOutputAllocByteCount",
        "guestStageLeafSetupWorkspaceAllocByteCount",
        "guestStageLeafSetupOutputAllocCount",
        "guestStageLeafSetupWorkspaceAllocCount",
        "guestStageLeafUploadWorkMilliseconds",
        "guestStageLeafKernelWorkMilliseconds",
        "guestStageLeafDownloadWorkMilliseconds",
        "guestStageLeafValidateWorkMilliseconds",
        "guestStageLeafHashWorkMilliseconds",
        "guestStageLeafHashRowCount",
        "guestStageLeafHashByteCount",
        "guestStageLeafHashArity2RowCount",
        "guestStageLeafHashArity2ByteCount",
        "guestStageLeafHashArity4RowCount",
        "guestStageLeafHashArity4ByteCount",
        "guestStageLeafCosetExtendCallCount",
        "guestStageLeafCosetExtendOutputByteCount",
        "guestStageLeafCosetExtendColumnCount",
        "guestStageLeafCosetExtendMaxColumnCount",
        "guestStageLeafCosetExtendNttLaunchCount",
        "guestStageLeafCosetExtendBitReverseLaunchCount",
        "guestStageLeafCosetExtendNttStageLaunchCount",
        "guestStageLeafCosetExtendNttBlockTwiddleLaunchCount",
        "guestStageLeafCosetExtendNormalizeLaunchCount",
        "guestStageLeafCosetExtendPackLaunchCount",
        "guestStageLeafCosetExtendUnpackLaunchCount",
        "guestStageTreeCommitWorkMilliseconds",
        "guestStageTreeCommitCheckpointWorkMilliseconds",
        "guestStageTreeCommitRootWorkMilliseconds",
        "guestStageTreeCommitRootCount",
        "guestStageTreeCommitRootByteCount",
        "guestStageTreeCommitRetainWorkMilliseconds",
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
        (
            "\"guest_trace_report_validation\"",
            "guest_trace_report_validation_duration()",
        ),
        (
            "\"guest_trace_report_lowering\"",
            "guest_trace_report_lowering_duration()",
        ),
        (
            "\"guest_trace_report_row_validation\"",
            "guest_trace_report_row_validation_duration()",
        ),
        (
            "\"guest_trace_report_source_values\"",
            "guest_trace_report_source_values_duration()",
        ),
        (
            "\"guest_trace_report_precompile_memory\"",
            "guest_trace_report_precompile_memory_duration()",
        ),
        (
            "\"guest_trace_report_instruction_result\"",
            "guest_trace_report_instruction_result_duration()",
        ),
        (
            "\"guest_trace_report_next_pc\"",
            "guest_trace_report_next_pc_duration()",
        ),
        (
            "\"guest_trace_report_register_access\"",
            "guest_trace_report_register_access_duration()",
        ),
        (
            "\"guest_trace_report_memory_access\"",
            "guest_trace_report_memory_access_duration()",
        ),
        (
            "\"guest_trace_report_store_apply\"",
            "guest_trace_report_store_apply_duration()",
        ),
        (
            "\"guest_trace_report_visit\"",
            "guest_trace_report_visit_duration()",
        ),
        (
            "\"guest_trace_single_row_report_lower\"",
            "guest_trace_single_row_report_duration()",
        ),
        (
            "\"guest_trace_multi_row_report_lower\"",
            "guest_trace_multi_row_report_duration()",
        ),
        (
            "\"guest_trace_pending_dma_report_lower\"",
            "guest_trace_pending_dma_report_duration()",
        ),
        (
            "\"guest_trace_amo_report_lower\"",
            "guest_trace_amo_report_duration()",
        ),
        (
            "\"guest_trace_store_conditional_report_lower\"",
            "guest_trace_store_conditional_report_duration()",
        ),
        ("\"guest_trace_reports\"", "guest_trace_report_count()"),
        (
            "\"guest_trace_report_rows\"",
            "guest_trace_report_row_count()",
        ),
        (
            "\"guest_trace_single_row_reports\"",
            "guest_trace_single_row_report_count()",
        ),
        (
            "\"guest_trace_multi_row_reports\"",
            "guest_trace_multi_row_report_count()",
        ),
        (
            "\"guest_trace_pending_dma_reports\"",
            "guest_trace_pending_dma_report_count()",
        ),
        (
            "\"guest_trace_amo_reports\"",
            "guest_trace_amo_report_count()",
        ),
        (
            "\"guest_trace_store_conditional_reports\"",
            "guest_trace_store_conditional_report_count()",
        ),
        (
            "\"guest_trace_external_op_rows\"",
            "guest_trace_external_op_row_count()",
        ),
        ("\"guest_trace_copy_rows\"", "guest_trace_copy_row_count()"),
        ("\"guest_trace_flag_rows\"", "guest_trace_flag_row_count()"),
        (
            "\"guest_trace_precompile_rows\"",
            "guest_trace_precompile_row_count()",
        ),
        (
            "\"guest_trace_indirect_memory_rows\"",
            "guest_trace_indirect_memory_row_count()",
        ),
        (
            "\"guest_trace_register_source_reads\"",
            "guest_trace_register_source_read_count()",
        ),
        (
            "\"guest_trace_memory_source_reads\"",
            "guest_trace_memory_source_read_count()",
        ),
        (
            "\"guest_trace_register_store_rows\"",
            "guest_trace_register_store_row_count()",
        ),
        (
            "\"guest_trace_memory_store_rows\"",
            "guest_trace_memory_store_row_count()",
        ),
        (
            "\"guest_trace_no_store_rows\"",
            "guest_trace_no_store_row_count()",
        ),
        ("\"guest_trace_emit\"", "guest_trace_emit_duration()"),
        (
            "\"guest_trace_descriptor\"",
            "guest_trace_descriptor_duration()",
        ),
        (
            "\"guest_trace_descriptor_rows\"",
            "guest_trace_descriptor_row_count()",
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
            "\"guest_stage_leaf_setup_prepare\"",
            "guest_stage_leaf_setup_prepare_duration()",
        ),
        (
            "\"guest_stage_leaf_setup_output_alloc\"",
            "guest_stage_leaf_setup_output_alloc_duration()",
        ),
        (
            "\"guest_stage_leaf_setup_workspace_alloc\"",
            "guest_stage_leaf_setup_workspace_alloc_duration()",
        ),
        (
            "\"guest_stage_leaf_setup_output_alloc_bytes\"",
            "guest_stage_leaf_setup_output_alloc_byte_count()",
        ),
        (
            "\"guest_stage_leaf_setup_workspace_alloc_bytes\"",
            "guest_stage_leaf_setup_workspace_alloc_byte_count()",
        ),
        (
            "\"guest_stage_leaf_setup_output_alloc_count\"",
            "guest_stage_leaf_setup_output_alloc_count()",
        ),
        (
            "\"guest_stage_leaf_setup_workspace_alloc_count\"",
            "guest_stage_leaf_setup_workspace_alloc_count()",
        ),
        (
            "\"guest_stage_leaf_hash_rows\"",
            "guest_stage_leaf_hash_row_count()",
        ),
        (
            "\"guest_stage_leaf_hash_bytes\"",
            "guest_stage_leaf_hash_byte_count()",
        ),
        (
            "\"guest_stage_leaf_hash_arity2_rows\"",
            "guest_stage_leaf_hash_arity2_row_count()",
        ),
        (
            "\"guest_stage_leaf_hash_arity2_bytes\"",
            "guest_stage_leaf_hash_arity2_byte_count()",
        ),
        (
            "\"guest_stage_leaf_hash_arity4_rows\"",
            "guest_stage_leaf_hash_arity4_row_count()",
        ),
        (
            "\"guest_stage_leaf_hash_arity4_bytes\"",
            "guest_stage_leaf_hash_arity4_byte_count()",
        ),
        (
            "\"guest_stage_leaf_coset_extend_calls\"",
            "guest_stage_leaf_coset_extend_call_count()",
        ),
        (
            "\"guest_stage_leaf_coset_extend_output_bytes\"",
            "guest_stage_leaf_coset_extend_output_byte_count()",
        ),
        (
            "\"guest_stage_leaf_coset_extend_columns\"",
            "guest_stage_leaf_coset_extend_column_count()",
        ),
        (
            "\"guest_stage_leaf_coset_extend_max_columns\"",
            "guest_stage_leaf_coset_extend_max_column_count()",
        ),
        (
            "\"guest_stage_leaf_coset_extend_ntt_launches\"",
            "guest_stage_leaf_coset_extend_ntt_launch_count()",
        ),
        (
            "\"guest_stage_leaf_coset_extend_bit_reverse_launches\"",
            "guest_stage_leaf_coset_extend_bit_reverse_launch_count()",
        ),
        (
            "\"guest_stage_leaf_coset_extend_ntt_stage_launches\"",
            "guest_stage_leaf_coset_extend_ntt_stage_launch_count()",
        ),
        (
            "\"guest_stage_leaf_coset_extend_ntt_block_twiddle_launches\"",
            "guest_stage_leaf_coset_extend_ntt_block_twiddle_launch_count()",
        ),
        (
            "\"guest_stage_leaf_coset_extend_normalize_launches\"",
            "guest_stage_leaf_coset_extend_normalize_launch_count()",
        ),
        (
            "\"guest_stage_leaf_coset_extend_pack_launches\"",
            "guest_stage_leaf_coset_extend_pack_launch_count()",
        ),
        (
            "\"guest_stage_leaf_coset_extend_unpack_launches\"",
            "guest_stage_leaf_coset_extend_unpack_launch_count()",
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
        (
            "\"guest_stage_tree_commit_work\"",
            "guest_stage_tree_commit_work_duration()",
        ),
        (
            "\"guest_stage_tree_commit_checkpoint_work\"",
            "guest_stage_tree_commit_checkpoint_work_duration()",
        ),
        (
            "\"guest_stage_tree_commit_root_work\"",
            "guest_stage_tree_commit_root_work_duration()",
        ),
        (
            "\"guest_stage_tree_commit_root_count\"",
            "guest_stage_tree_commit_root_count()",
        ),
        (
            "\"guest_stage_tree_commit_root_bytes\"",
            "guest_stage_tree_commit_root_byte_count()",
        ),
        (
            "\"guest_stage_tree_commit_retain_work\"",
            "guest_stage_tree_commit_retain_work_duration()",
        ),
    ] {
        assert!(
            guest_pc_timing_source.contains(line_name) && guest_pc_timing_source.contains(accessor),
            "CLI guest PC timing output should include {line_name}"
        );
    }
    for (line_name, accessor) in [
        (
            "guest_stage_{stage_index}_leaf_extend_work",
            "leaf_extend_work_duration()",
        ),
        (
            "guest_stage_{stage_index}_leaf_setup_work",
            "leaf_setup_work_duration()",
        ),
        (
            "guest_stage_{stage_index}_leaf_setup_prepare",
            "leaf_setup_prepare_duration()",
        ),
        (
            "guest_stage_{stage_index}_leaf_setup_output_alloc",
            "leaf_setup_output_alloc_duration()",
        ),
        (
            "guest_stage_{stage_index}_leaf_setup_workspace_alloc",
            "leaf_setup_workspace_alloc_duration()",
        ),
        (
            "guest_stage_{stage_index}_leaf_setup_output_alloc_bytes",
            "leaf_setup_output_alloc_byte_count()",
        ),
        (
            "guest_stage_{stage_index}_leaf_setup_workspace_alloc_bytes",
            "leaf_setup_workspace_alloc_byte_count()",
        ),
        (
            "guest_stage_{stage_index}_leaf_setup_output_alloc_count",
            "leaf_setup_output_alloc_count()",
        ),
        (
            "guest_stage_{stage_index}_leaf_setup_workspace_alloc_count",
            "leaf_setup_workspace_alloc_count()",
        ),
        (
            "guest_stage_{stage_index}_leaf_upload_work",
            "leaf_upload_work_duration()",
        ),
        (
            "guest_stage_{stage_index}_leaf_kernel_work",
            "leaf_kernel_work_duration()",
        ),
        (
            "guest_stage_{stage_index}_leaf_download_work",
            "leaf_download_work_duration()",
        ),
        (
            "guest_stage_{stage_index}_leaf_validate_work",
            "leaf_validate_work_duration()",
        ),
        (
            "guest_stage_{stage_index}_leaf_hash_work",
            "leaf_hash_work_duration()",
        ),
        (
            "guest_stage_{stage_index}_leaf_hash_arity2_rows",
            "leaf_hash_arity2_row_count()",
        ),
        (
            "guest_stage_{stage_index}_leaf_hash_arity2_bytes",
            "leaf_hash_arity2_byte_count()",
        ),
        (
            "guest_stage_{stage_index}_leaf_hash_arity4_rows",
            "leaf_hash_arity4_row_count()",
        ),
        (
            "guest_stage_{stage_index}_leaf_hash_arity4_bytes",
            "leaf_hash_arity4_byte_count()",
        ),
        (
            "guest_stage_{stage_index}_leaf_coset_extend_calls",
            "leaf_coset_extend_call_count()",
        ),
        (
            "guest_stage_{stage_index}_leaf_coset_extend_output_bytes",
            "leaf_coset_extend_output_byte_count()",
        ),
        (
            "guest_stage_{stage_index}_leaf_coset_extend_columns",
            "leaf_coset_extend_column_count()",
        ),
        (
            "guest_stage_{stage_index}_leaf_coset_extend_max_columns",
            "leaf_coset_extend_max_column_count()",
        ),
        (
            "guest_stage_{stage_index}_leaf_coset_extend_ntt_launches",
            "leaf_coset_extend_ntt_launch_count()",
        ),
        (
            "guest_stage_{stage_index}_leaf_coset_extend_bit_reverse_launches",
            "leaf_coset_extend_bit_reverse_launch_count()",
        ),
        (
            "guest_stage_{stage_index}_leaf_coset_extend_ntt_stage_launches",
            "leaf_coset_extend_ntt_stage_launch_count()",
        ),
        (
            "guest_stage_{stage_index}_leaf_coset_extend_ntt_block_twiddle_launches",
            "leaf_coset_extend_ntt_block_twiddle_launch_count()",
        ),
        (
            "guest_stage_{stage_index}_leaf_coset_extend_normalize_launches",
            "leaf_coset_extend_normalize_launch_count()",
        ),
        (
            "guest_stage_{stage_index}_leaf_coset_extend_pack_launches",
            "leaf_coset_extend_pack_launch_count()",
        ),
        (
            "guest_stage_{stage_index}_leaf_coset_extend_unpack_launches",
            "leaf_coset_extend_unpack_launch_count()",
        ),
        (
            "guest_stage_{stage_index}_tree_commit_work",
            "tree_commit_work_duration()",
        ),
        (
            "guest_stage_{stage_index}_tree_commit_checkpoint_work",
            "tree_commit_checkpoint_work_duration()",
        ),
        (
            "guest_stage_{stage_index}_tree_commit_root_work",
            "tree_commit_root_work_duration()",
        ),
        (
            "guest_stage_{stage_index}_tree_commit_retain_work",
            "tree_commit_retain_work_duration()",
        ),
    ] {
        assert!(
            guest_pc_timing_source.contains(line_name) && guest_pc_timing_source.contains(accessor),
            "CLI guest PC stage timing output should include {line_name}"
        );
    }
    for field in [
        "constantMaterialValidationElapsedMilliseconds",
        "constantMaterialValidationJoinWaitMilliseconds",
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
        "\"constant_material_validation_join_wait\"",
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
    assert!(
        prove_plan_source.contains("\"cuda_backend={}\"")
            && prove_plan_source.contains("cuda_backend_status()"),
        "CLI run plan summary should include the CUDA backend capability summary"
    );
    assert!(
        lean_source.contains("cudaBackendEnabled"),
        "Lean CUDA backend summary should expose the compile-time backend capability"
    );
    for field in [
        "cudaAllocatorMallocCallCount",
        "cudaAllocatorMallocByteCount",
        "cudaAllocatorCachedBlockCount",
        "cudaAllocatorCachedByteCount",
        "cudaAllocatorEventQueryCallCount",
        "cudaAllocatorEventQueryReadyCount",
        "cudaAllocatorEventQueryNotReadyCount",
        "cudaAllocatorEventSynchronizeCallCount",
        "cudaAllocatorEventSynchronizeByteCount",
        "cudaAllocatorEventSynchronizeMaxByteCount",
        "cudaAllocatorEventSynchronizeWaitNanoseconds",
        "cudaAllocatorEventSynchronizeMaxWaitNanoseconds",
        "cudaAllocatorEventSynchronizeHotByteCount",
        "cudaAllocatorEventSynchronizeHotCount",
        "cudaAllocatorEventSynchronizeHotWaitNanoseconds",
        "cudaAllocatorCachedReuseCount",
        "cudaAllocatorPendingReuseCount",
        "cudaAllocatorNoWaitBypassCount",
        "cudaAllocatorNoWaitBypassByteCount",
    ] {
        assert!(
            lean_source.contains(field),
            "Lean CUDA allocator timing summary should expose {field}"
        );
    }
    for field in [
        "timingObservations : List TimingObservation",
        "guestPcTraceTiming : Option GuestPcTraceTimingSummary",
        "witnessOpeningRowValueTiming : Option WitnessOpeningRowValueTimingSummary",
        "constantMaterialValidationTiming : Option ConstantMaterialValidationTimingSummary",
        "proverGpuMode : Option ProverGpuModeSummary",
        "cudaBackend : Option CudaBackendSummary",
        "cudaAllocatorTiming : Option CudaAllocatorTimingSummary",
    ] {
        assert!(
            lean_source.contains(field),
            "Lean runtime performance observation summary should expose {field}"
        );
    }
    for line_name in [
        "\"cuda_allocator_malloc_calls\"",
        "\"cuda_allocator_malloc_bytes\"",
        "\"cuda_allocator_cached_blocks\"",
        "\"cuda_allocator_cached_bytes\"",
        "\"cuda_allocator_event_query_calls\"",
        "\"cuda_allocator_event_query_ready\"",
        "\"cuda_allocator_event_query_not_ready\"",
        "\"cuda_allocator_event_synchronize_calls\"",
        "\"cuda_allocator_event_synchronize_bytes\"",
        "\"cuda_allocator_event_synchronize_max_bytes\"",
        "\"cuda_allocator_event_synchronize_wait_ns\"",
        "\"cuda_allocator_event_synchronize_max_wait_ns\"",
        "\"cuda_allocator_event_synchronize_hot_bytes\"",
        "\"cuda_allocator_event_synchronize_hot_count\"",
        "\"cuda_allocator_event_synchronize_hot_wait_ns\"",
        "\"cuda_allocator_cached_reuse_count\"",
        "\"cuda_allocator_pending_reuse_count\"",
        "\"cuda_allocator_no_wait_bypass_count\"",
        "\"cuda_allocator_no_wait_bypass_bytes\"",
    ] {
        assert!(
            cli_timing_source.contains(line_name),
            "CLI CUDA allocator timing output should include {line_name}"
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
            "cuda_backend_acceptance_sound",
            "cuda_backend_acceptance_verifier_core_contract",
            "cuda_allocator_timing_acceptance_sound",
            "cuda_allocator_timing_acceptance_verifier_core_contract",
            "runtime_performance_observation_acceptance_sound",
            "runtime_performance_observation_acceptance_verifier_core_contract",
            "gpu_setup_checked_acceptance_sound",
            "gpu_setup_checked_acceptance_verifier_core_contract",
            "gpu_allocation_checked_acceptance_sound",
            "gpu_allocation_checked_acceptance_verifier_core_contract",
            "gpu_host_device_copy_round_trip_implies_written_contents",
            "gpu_host_device_copy_round_trip_checked_acceptance_sound",
            "gpu_host_device_copy_round_trip_checked_acceptance_verifier_core_contract",
            "gpu_temporary_buffer_reuse_implies_same_request",
            "gpu_temporary_buffer_reuse_implies_pending_reads_complete",
            "gpu_temporary_buffer_reuse_checked_acceptance_sound",
            "gpu_temporary_buffer_reuse_checked_acceptance_verifier_core_contract",
            "gpu_allocator_no_wait_bypass_implies_same_request",
            "gpu_allocator_no_wait_bypass_implies_pending_not_reused",
            "gpu_allocator_no_wait_bypass_implies_fresh_allocation",
            "gpu_allocator_no_wait_bypass_checked_acceptance_sound",
            "gpu_allocator_no_wait_bypass_checked_acceptance_verifier_core_contract",
            "gpu_retained_device_cache_budget_checked_acceptance_sound",
            "gpu_retained_device_cache_budget_checked_acceptance_verifier_core_contract",
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
