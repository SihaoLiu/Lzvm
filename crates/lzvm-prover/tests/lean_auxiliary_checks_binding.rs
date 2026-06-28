use std::path::Path;

#[path = "support/lean_binding.rs"]
mod lean_binding;

fn quoted_name(raw: &str) -> Option<&str> {
    raw.strip_prefix('"')?.strip_suffix('"')
}

fn compact_source(source: &str) -> String {
    source.chars().filter(|c| !c.is_whitespace()).collect()
}

fn compact_source_contains(source: &str, needle: &str) -> bool {
    compact_source(source).contains(&compact_source(needle))
}

fn guest_pc_timing_source_contains(source: &str, line_name: &str, accessor: &str) -> bool {
    let accessor_name = accessor.strip_suffix("()").unwrap_or(accessor);
    if source.contains(line_name) && (source.contains(accessor) || source.contains(accessor_name)) {
        return true;
    }

    let Some(required_name) = quoted_name(line_name) else {
        return false;
    };
    let Some(method_name) = accessor.strip_suffix("()") else {
        return false;
    };
    let Some(derived_name) = method_name.strip_suffix("_duration") else {
        return false;
    };
    required_name == derived_name
        && (source.contains(&format!("record_duration!({method_name}"))
            || source.contains(&format!("record_sampled_duration_count!({method_name}")))
}

#[test]
fn lean_auxiliary_checks_binding_exports_core_contract_projections() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let auxiliary_wrapper_path = crate_root.join("../../lean/Lzvm/AuxiliaryChecks.lean");
    let auxiliary_wrapper_source = std::fs::read_to_string(&auxiliary_wrapper_path)
        .expect("Lean auxiliary checks wrapper should read");
    let auxiliary_core_path = crate_root.join("../../lean/Lzvm/AuxiliaryChecks/Core.lean");
    let auxiliary_core_source =
        std::fs::read_to_string(&auxiliary_core_path).expect("Lean auxiliary core should read");
    let auxiliary_leaf_path = crate_root.join("../../lean/Lzvm/AuxiliaryChecks/LeafDigest.lean");
    let auxiliary_leaf_source =
        std::fs::read_to_string(&auxiliary_leaf_path).expect("Lean auxiliary leaf checks read");
    let auxiliary_source = [
        auxiliary_wrapper_source.as_str(),
        auxiliary_core_source.as_str(),
        auxiliary_leaf_source.as_str(),
    ]
    .join("\n");
    let auxiliary_all_path = crate_root.join("../../lean/Lzvm/AuxiliaryChecks/All.lean");
    let auxiliary_all_source = std::fs::read_to_string(&auxiliary_all_path)
        .expect("Lean auxiliary checks aggregate should read");
    let gpu_runtime_wrapper_path =
        crate_root.join("../../lean/Lzvm/AuxiliaryChecks/GpuRuntime.lean");
    let gpu_runtime_wrapper_source = std::fs::read_to_string(&gpu_runtime_wrapper_path)
        .expect("Lean GPU runtime checks wrapper should read");
    let gpu_runtime_common_path =
        crate_root.join("../../lean/Lzvm/AuxiliaryChecks/GpuRuntime/Common.lean");
    let gpu_runtime_common_source = std::fs::read_to_string(&gpu_runtime_common_path)
        .expect("Lean GPU runtime common checks should read");
    let gpu_runtime_core_path =
        crate_root.join("../../lean/Lzvm/AuxiliaryChecks/GpuRuntime/Core.lean");
    let gpu_runtime_core_source = std::fs::read_to_string(&gpu_runtime_core_path)
        .expect("Lean GPU runtime core checks should read");
    let gpu_runtime_trace_path =
        crate_root.join("../../lean/Lzvm/AuxiliaryChecks/GpuRuntime/Trace.lean");
    let gpu_runtime_trace_source = std::fs::read_to_string(&gpu_runtime_trace_path)
        .expect("Lean GPU runtime trace checks should read");
    let gpu_runtime_source = [
        gpu_runtime_wrapper_source.as_str(),
        gpu_runtime_common_source.as_str(),
        gpu_runtime_core_source.as_str(),
        gpu_runtime_trace_source.as_str(),
    ]
    .join("\n");
    let timing_core_path = crate_root.join("../../lean/Lzvm/AuxiliaryChecks/TimingCore.lean");
    let timing_core_source =
        std::fs::read_to_string(&timing_core_path).expect("Lean timing core checks should read");
    let timing_source = lean_binding::read_lean_sources(
        crate_root,
        &[
            "../../lean/Lzvm/AuxiliaryChecks/Timing.lean",
            "../../lean/Lzvm/AuxiliaryChecks/Timing/Trace.lean",
            "../../lean/Lzvm/AuxiliaryChecks/Timing/Stage.lean",
        ],
    );
    let timing_projected_path =
        crate_root.join("../../lean/Lzvm/AuxiliaryChecks/TimingProjected.lean");
    let timing_projected_source = std::fs::read_to_string(&timing_projected_path)
        .expect("Lean projected timing checks should read");
    let auxiliary_projected_path =
        crate_root.join("../../lean/Lzvm/AuxiliaryChecks/Projected.lean");
    let auxiliary_projected_source = std::fs::read_to_string(&auxiliary_projected_path)
        .expect("Lean auxiliary projected checks should read");
    let proof_timing_path = crate_root.join("../../lean/Lzvm/AuxiliaryChecks/ProofTiming.lean");
    let proof_timing_aggregate_source =
        std::fs::read_to_string(&proof_timing_path).expect("Lean proof timing checks should read");
    assert!(
        lean_binding::contains_import(
            &proof_timing_aggregate_source,
            "Lzvm.AuxiliaryChecks.ProofTiming.Core",
        ) && lean_binding::contains_import(
            &proof_timing_aggregate_source,
            "Lzvm.AuxiliaryChecks.ProofTiming.Finish",
        ),
        "Lean proof timing wrapper should re-export split proof timing modules"
    );
    let proof_timing_core_path =
        crate_root.join("../../lean/Lzvm/AuxiliaryChecks/ProofTiming/Core.lean");
    let proof_timing_core_source = std::fs::read_to_string(&proof_timing_core_path)
        .expect("Lean proof timing core checks should read");
    let proof_timing_finish_path =
        crate_root.join("../../lean/Lzvm/AuxiliaryChecks/ProofTiming/Finish.lean");
    let proof_timing_finish_source = std::fs::read_to_string(&proof_timing_finish_path)
        .expect("Lean proof timing finish checks should read");
    let lean_proof_timing_source = [
        proof_timing_aggregate_source.as_str(),
        proof_timing_core_source.as_str(),
        proof_timing_finish_source.as_str(),
    ]
    .join("\n");
    let proof_timing_projected_path =
        crate_root.join("../../lean/Lzvm/AuxiliaryChecks/ProofTimingProjected.lean");
    let proof_timing_projected_source = std::fs::read_to_string(&proof_timing_projected_path)
        .expect("Lean projected proof timing checks should read");
    let proof_timing_verifier_path =
        crate_root.join("../../lean/Lzvm/AuxiliaryChecks/ProofTimingVerifier.lean");
    let proof_timing_verifier_source = std::fs::read_to_string(&proof_timing_verifier_path)
        .expect("Lean proof timing verifier checks should read");
    let runtime_performance_path =
        crate_root.join("../../lean/Lzvm/AuxiliaryChecks/RuntimePerformance.lean");
    let runtime_performance_source = std::fs::read_to_string(&runtime_performance_path)
        .expect("Lean runtime performance checks should read");
    let proof_batch_runner_path = crate_root.join("../../scripts/run-proof-timing-batch.py");
    let proof_batch_runner_source =
        std::fs::read_to_string(&proof_batch_runner_path).expect("proof batch runner should read");
    let proof_timing_keys_path = crate_root.join("../../scripts/proof_timing_keys.py");
    let proof_timing_keys_source =
        std::fs::read_to_string(&proof_timing_keys_path).expect("proof timing keys should read");
    let lean_source = [
        auxiliary_source.as_str(),
        auxiliary_all_source.as_str(),
        gpu_runtime_source.as_str(),
        timing_core_source.as_str(),
        timing_source.as_str(),
        timing_projected_source.as_str(),
        auxiliary_projected_source.as_str(),
        lean_proof_timing_source.as_str(),
        proof_timing_projected_source.as_str(),
        proof_timing_verifier_source.as_str(),
        runtime_performance_source.as_str(),
    ]
    .join("\n");
    let top_level_path = crate_root.join("../../lean/Lzvm.lean");
    let top_level_source =
        std::fs::read_to_string(&top_level_path).expect("Lean top-level source should read");
    let guest_pc_timing_path = crate_root.join("../lzvm-cli/src/prove_witness/guest_pc_trace.rs");
    let guest_pc_timing_source =
        std::fs::read_to_string(&guest_pc_timing_path).expect("guest PC timing source should read");
    let prove_witness_path = crate_root.join("../lzvm-cli/src/prove_witness.rs");
    let prove_witness_source =
        std::fs::read_to_string(&prove_witness_path).expect("prove witness source should read");
    let gpu_preflight_path = crate_root.join("../lzvm-cli/src/prove_witness/gpu_preflight.rs");
    let gpu_preflight_source =
        std::fs::read_to_string(&gpu_preflight_path).expect("GPU preflight source should read");
    let constant_material_path =
        crate_root.join("../lzvm-cli/src/prove_witness/constant_material.rs");
    let constant_material_source = std::fs::read_to_string(&constant_material_path)
        .expect("constant material source should read");
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
    let witness_execution_path = crate_root.join("src/witness_execution.rs");
    let witness_execution_source = std::fs::read_to_string(&witness_execution_path)
        .expect("witness execution source should read");
    let guest_backend_path = crate_root.join("src/guest_pc_trace_backend.rs");
    let guest_backend_source =
        std::fs::read_to_string(&guest_backend_path).expect("guest backend source should read");

    assert!(
        lean_binding::contains_import(&auxiliary_wrapper_source, "Lzvm.AuxiliaryChecks.Core")
            && lean_binding::contains_import(
                &auxiliary_wrapper_source,
                "Lzvm.AuxiliaryChecks.LeafDigest",
            )
            && lean_binding::contains_import(
                &gpu_runtime_wrapper_source,
                "Lzvm.AuxiliaryChecks.GpuRuntime.Core",
            )
            && lean_binding::contains_import(
                &gpu_runtime_wrapper_source,
                "Lzvm.AuxiliaryChecks.GpuRuntime.Trace",
            )
            && lean_binding::contains_import(
                &gpu_runtime_core_source,
                "Lzvm.AuxiliaryChecks.GpuRuntime.Common",
            ),
        "Lean auxiliary aggregate wrappers should re-export split core and runtime modules"
    );
    assert!(
        lean_binding::contains_import(&top_level_source, "Lzvm.AuxiliaryChecks.All"),
        "top-level Lean module should import the auxiliary checks aggregate"
    );
    for obsolete_import in [
        "Lzvm.AuxiliaryChecks",
        "Lzvm.AuxiliaryChecks.GpuRuntime",
        "Lzvm.AuxiliaryChecks.TimingCore",
        "Lzvm.AuxiliaryChecks.Timing",
        "Lzvm.AuxiliaryChecks.TimingProjected",
        "Lzvm.AuxiliaryChecks.Projected",
        "Lzvm.AuxiliaryChecks.ProofTiming",
        "Lzvm.AuxiliaryChecks.ProofTimingProjected",
        "Lzvm.AuxiliaryChecks.ProofTimingVerifier",
        "Lzvm.AuxiliaryChecks.RuntimePerformance",
    ] {
        assert!(
            !lean_binding::contains_import(&top_level_source, obsolete_import),
            "top-level Lean module should rely on the auxiliary checks aggregate"
        );
    }
    for required_import in [
        "Lzvm.AuxiliaryChecks",
        "Lzvm.AuxiliaryChecks.GpuRuntime",
        "Lzvm.AuxiliaryChecks.Timing",
        "Lzvm.AuxiliaryChecks.TimingProjected",
        "Lzvm.AuxiliaryChecks.Projected",
        "Lzvm.AuxiliaryChecks.ProofTiming",
        "Lzvm.AuxiliaryChecks.ProofTimingProjected",
        "Lzvm.AuxiliaryChecks.ProofTimingVerifier",
        "Lzvm.AuxiliaryChecks.RuntimePerformance",
    ] {
        assert!(
            lean_binding::contains_import(&auxiliary_all_source, required_import),
            "auxiliary checks aggregate should import every auxiliary checks module"
        );
    }
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
            && lean_source.contains("GpuRunOptionsSummary")
            && lean_source.contains("GpuRunOptionsObservedAcceptance")
            && lean_source.contains("CudaBackendSummary")
            && lean_source.contains("CudaBackendObservedAcceptance")
            && lean_source.contains("CudaAllocatorTimingSummary")
            && lean_source.contains("CudaAllocatorTimingObservedAcceptance")
            && lean_source.contains("ProofArtifactFinishTimingSummary")
            && lean_source.contains("ProofArtifactFinishTimingObservedAcceptance")
            && lean_source.contains("ProofTimingBatchSummary")
            && lean_source.contains("ProofTimingBatchObservedAcceptance")
            && lean_source.contains("RuntimePerformanceObservationSummary")
            && lean_source.contains("RuntimePerformanceObservedAcceptance")
            && lean_source.contains("GpuSetupCheckedAcceptance")
            && lean_source.contains("GpuAllocationCheckedAcceptance")
            && lean_source.contains("GpuHostDeviceCopyRoundTripValidation")
            && lean_source.contains("GpuHostDeviceCopyRoundTripCheckedAcceptance")
            && lean_source.contains("GpuTemporaryBufferReuseValidation")
            && lean_source.contains("GpuTemporaryBufferReuseCheckedAcceptance")
            && lean_source.contains("GpuLeafOutputBufferReuseValidation")
            && lean_source.contains("GpuLeafOutputBufferReuseCheckedAcceptance")
            && lean_source.contains("GpuAllocatorNoWaitBypassValidation")
            && lean_source.contains("GpuAllocatorNoWaitBypassCheckedAcceptance")
            && lean_source.contains("GpuAllocatorNoWaitLimitConfig")
            && lean_source.contains("GpuAllocatorNoWaitLimitValidation")
            && lean_source.contains("GpuAllocatorNoWaitLimitCheckedAcceptance")
            && lean_source.contains("GuestPcTraceSegmentQueueConfig")
            && lean_source.contains("GuestPcTraceSegmentQueueValidation")
            && lean_source.contains("GuestPcTraceSegmentQueueCheckedAcceptance")
            && lean_source.contains("GuestPcTraceLargeGpuGateConfig")
            && lean_source.contains("GuestPcTraceLargeGpuGateValidation")
            && lean_source.contains("GuestPcTraceLargeGpuGateCheckedAcceptance")
            && lean_source.contains("GpuRetainedLeafDigestLimitConfig")
            && lean_source.contains("GpuRetainedLeafDigestLimitValidation")
            && lean_source.contains("GpuRetainedLeafDigestLimitCheckedAcceptance")
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
        lean_source.contains(
            "gpu_setup_checked_acceptance_projects_constants_sound\n        validation\n        request\n        publicInput\n        proof\n        acceptedWithSetup"
        ),
        "Lean GPU setup soundness should reuse the checked-acceptance constants projector"
    );
    assert!(
        lean_source.contains(
            "gpu_allocation_checked_acceptance_projects_written_contents\n        validation\n        allocation\n        publicInput\n        proof\n        acceptedWithAllocation"
        ),
        "Lean GPU allocation soundness should reuse the checked-acceptance written-contents projector"
    );
    assert!(
        lean_source.contains(
            "auxiliary_checked_acceptance_sound_witness\n        assumptions\n        publicInput\n        proof\n        acceptedWithLookupChecks"
        ),
        "Lean source lookup soundness should reuse the checked-acceptance SoundWitness helper"
    );
    assert!(
        lean_source.contains(
            "auxiliary_checked_acceptance_sound_witness\n        assumptions\n        publicInput\n        proof\n        acceptedWithLeafDigestChecks"
        ),
        "Lean witness leaf digest soundness should reuse the checked-acceptance SoundWitness helper"
    );
    assert!(
        lean_source.contains(
            "gpu_temporary_buffer_reuse_checked_acceptance_projects_same_request\n      validation\n      previous\n      next\n      publicInput\n      proof\n      checked"
        ) && lean_source.contains(
            "gpu_temporary_buffer_reuse_checked_acceptance_projects_pending_reads_complete\n      validation\n      previous\n      next\n      publicInput\n      proof\n      checked"
        ),
        "Lean temporary buffer reuse soundness should reuse checked-acceptance projectors"
    );
    assert!(
        lean_source.contains(
            "fri_fixed_column_cache_checked_acceptance_projects_request_bound\n        validation\n        cached\n        fresh\n        publicInput\n        proof\n        checked"
        ) && lean_source.contains(
            "fri_fixed_column_cache_checked_acceptance_projects_fresh_contents_bound\n        validation\n        cached\n        fresh\n        publicInput\n        proof\n        checked"
        ),
        "Lean fixed-column cache cached-contents projector should reuse checked-acceptance projectors"
    );
    assert!(
        lean_source.contains("IgnoredMetadataObservedAcceptance")
            && lean_source
                .contains("ignored_metadata_observed_acceptance_projects_verifier_acceptance")
            && lean_source.contains("ignored_metadata_acceptance_sound"),
        "Lean auxiliary timing metadata should use one generic ignored-metadata acceptance wrapper"
    );
    assert!(
        !lean_binding::contains_theorem_declaration(
            &auxiliary_source,
            "ignored_metadata_acceptance_verifier_core_contract_via_soundness",
        ),
        "generic ignored-metadata verifier-core contracts should not keep a SoundWitness projection shortcut"
    );
    lean_binding::assert_theorem_body_contains(
        &auxiliary_source,
        "ignored_metadata_acceptance_sound",
        &[
            "abstract_verifier_sound",
            "ignored_metadata_observed_acceptance_projects_verifier_acceptance",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &auxiliary_source,
        "ignored_metadata_acceptance_sound",
        &[
            "sound_witness_implies_verifier_core_contract",
            "ignored_metadata_acceptance_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &auxiliary_source,
        "ignored_metadata_acceptance_verifier_core_contract",
        &[
            "ignored_metadata_observed_acceptance_projects_verifier_acceptance",
            "assumption_bundle_fiat_shamir_transcript_binding",
            "assumption_bundle_public_input_binding",
            "assumption_bundle_pcs_opening_soundness",
            "assumption_bundle_fri_query_soundness",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &auxiliary_source,
        "ignored_metadata_acceptance_verifier_core_contract",
        &[
            "ignored_metadata_acceptance_verifier_core_contract_via_soundness",
            "ignored_metadata_acceptance_sound",
            "assumptions.crypto.transcript_binding",
            "assumptions.semantic.public_input_binding",
            "assumptions.crypto.pcs_opening_sound",
            "assumptions.crypto.fri_query_sound",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &auxiliary_source,
        "auxiliary_checked_acceptance_verifier_core_contract",
        &[
            "assumption_bundle_fiat_shamir_transcript_binding",
            "assumption_bundle_public_input_binding",
            "assumption_bundle_pcs_opening_soundness",
            "assumption_bundle_fri_query_soundness",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &auxiliary_source,
        "auxiliary_checked_acceptance_verifier_core_contract",
        &[
            "sound_witness_implies_verifier_core_contract",
            "abstract_verifier_sound",
            "assumptions.crypto.transcript_binding",
            "assumptions.semantic.public_input_binding",
            "assumptions.crypto.pcs_opening_sound",
            "assumptions.crypto.fri_query_sound",
        ],
    );
    for obligation in [
        "assumption_bundle_fiat_shamir_transcript_binding",
        "assumption_bundle_public_input_binding",
        "assumption_bundle_pcs_opening_soundness",
        "assumption_bundle_fri_query_soundness",
    ] {
        lean_binding::assert_theorem_body_contains_identifier(
            &auxiliary_source,
            "auxiliary_checked_acceptance_verifier_core_contract",
            obligation,
        );
    }
    for shortcut in [
        "sound_witness_implies_verifier_core_contract",
        "abstract_verifier_sound",
    ] {
        lean_binding::assert_theorem_body_omits_identifier(
            &auxiliary_source,
            "auxiliary_checked_acceptance_verifier_core_contract",
            shortcut,
        );
    }
    lean_binding::assert_theorem_declarations(
        &auxiliary_source,
        &["auxiliary_checked_acceptance_sound_witness"],
    );
    lean_binding::assert_theorem_body_contains(
        &auxiliary_source,
        "auxiliary_checked_acceptance_sound_witness",
        &["abstract_verifier_sound"],
    );
    lean_binding::assert_theorem_body_omits(
        &auxiliary_source,
        "auxiliary_checked_acceptance_sound_witness",
        &["sound_witness_implies_verifier_core_contract"],
    );
    lean_binding::assert_theorem_body_contains_identifier(
        &auxiliary_source,
        "auxiliary_checked_acceptance_sound_witness",
        "abstract_verifier_sound",
    );
    lean_binding::assert_theorem_body_omits_identifier(
        &auxiliary_source,
        "auxiliary_checked_acceptance_sound_witness",
        "sound_witness_implies_verifier_core_contract",
    );
    assert_eq!(
        lean_binding::visible_identifier_occurrence_count(
            &auxiliary_source,
            "abstract_verifier_sound"
        ),
        2,
        "Lean auxiliary checks should keep direct abstract verifier soundness calls limited to ignored-metadata acceptance and auxiliary checked acceptance"
    );
    for theorem_name in [
        "source_lookup_auxiliary_acceptance_sound",
        "witness_leaf_digest_acceptance_sound",
        "gpu_canonical_leaf_checked_acceptance_sound",
        "gpu_leaf_output_buffer_reuse_checked_acceptance_sound",
        "gpu_coset_extension_checked_acceptance_sound",
        "gpu_fri_fold_interpolation_checked_acceptance_sound",
        "gpu_merkle_digest_prefix_batch_checked_acceptance_sound",
    ] {
        lean_binding::assert_theorem_body_contains(
            &auxiliary_source,
            theorem_name,
            &["auxiliary_checked_acceptance_sound_witness"],
        );
        lean_binding::assert_theorem_body_omits(
            &auxiliary_source,
            theorem_name,
            &[
                "abstract_verifier_sound",
                "sound_witness_implies_verifier_core_contract",
            ],
        );
    }
    assert!(
        timing_core_source.contains("IgnoredMetadataObservedAcceptance system observations")
            && lean_source
                .matches("IgnoredMetadataObservedAcceptance system summary")
                .count()
                >= 8
            && runtime_performance_source
                .contains("IgnoredMetadataObservedAcceptance system summary"),
        "Lean timing modules should instantiate the generic ignored-metadata wrapper"
    );
    for (module_name, module_source) in [
        ("TimingCore", timing_core_source.as_str()),
        ("ProofTiming", lean_proof_timing_source.as_str()),
        ("RuntimePerformance", runtime_performance_source.as_str()),
    ] {
        assert!(
            module_source.contains("ignored_metadata_acceptance_sound\n      assumptions")
                && module_source.contains(
                    "ignored_metadata_acceptance_verifier_core_contract\n      assumptions"
                ),
            "{module_name} should prove ignored metadata neutrality through the generic wrapper"
        );
        assert!(
            !module_source.contains("abstract_verifier_sound"),
            "{module_name} should not duplicate abstract verifier soundness for ignored metadata"
        );
    }
    lean_binding::assert_theorem_declarations(
        &runtime_performance_source,
        &[
            "runtime_performance_observation_projects_metadata",
            "runtime_performance_observation_projected_metadata_acceptance_sound",
            "runtime_performance_observation_projected_metadata_acceptance_verifier_core_contract",
        ],
    );
    assert_eq!(
        lean_binding::visible_identifier_occurrence_count(
            &gpu_runtime_source,
            "abstract_verifier_sound"
        ),
        0,
        "Lean GPU runtime wrappers should route checked acceptance through the auxiliary SoundWitness chokepoint"
    );
    for (theorem_name, projector, wrapper) in [
        (
            "runtime_performance_observation_timing_observations_acceptance_sound",
            "runtime_performance_observation_projects_timing_observations",
            "timing_observation_acceptance_sound",
        ),
        (
            "runtime_performance_observation_guest_pc_trace_timing_acceptance_sound",
            "runtime_performance_observation_projects_guest_pc_trace_timing",
            "guest_pc_trace_timing_acceptance_sound",
        ),
        (
            "runtime_performance_observation_row_value_timing_acceptance_sound",
            "runtime_performance_observation_projects_witness_opening_row_value_timing",
            "witness_opening_row_value_timing_acceptance_sound",
        ),
        (
            "runtime_performance_observation_constant_material_timing_acceptance_sound",
            "runtime_performance_observation_projects_constant_material_validation_timing",
            "constant_material_validation_timing_acceptance_sound",
        ),
        (
            "runtime_performance_observation_prover_gpu_mode_acceptance_sound",
            "runtime_performance_observation_projects_prover_gpu_mode",
            "prover_gpu_mode_acceptance_sound",
        ),
        (
            "runtime_performance_observation_gpu_run_options_acceptance_sound",
            "runtime_performance_observation_projects_gpu_run_options",
            "gpu_run_options_acceptance_sound",
        ),
        (
            "runtime_performance_observation_cuda_backend_acceptance_sound",
            "runtime_performance_observation_projects_cuda_backend",
            "cuda_backend_acceptance_sound",
        ),
        (
            "runtime_performance_observation_cuda_allocator_timing_acceptance_sound",
            "runtime_performance_observation_projects_cuda_allocator_timing",
            "cuda_allocator_timing_acceptance_sound",
        ),
        (
            "runtime_performance_observation_finish_timing_acceptance_sound",
            "runtime_performance_observation_projects_proof_artifact_finish_timing",
            "proof_artifact_finish_timing_acceptance_sound",
        ),
        (
            "runtime_performance_observation_proof_timing_batch_acceptance_sound",
            "runtime_performance_observation_projects_proof_timing_batch",
            "proof_timing_batch_acceptance_sound",
        ),
    ] {
        lean_binding::assert_theorem_body_contains(
            &runtime_performance_source,
            theorem_name,
            &[
                projector,
                "runtime_performance_observation_projected_metadata_acceptance_sound",
            ],
        );
        lean_binding::assert_theorem_body_omits(
            &runtime_performance_source,
            theorem_name,
            &["runtime_performance_observation_acceptance_sound", wrapper],
        );
    }
    for (theorem_name, projector, wrapper) in [
        (
            "runtime_performance_observation_timing_observations_acceptance_verifier_core_contract",
            "runtime_performance_observation_projects_timing_observations",
            "timing_observation_acceptance_verifier_core_contract",
        ),
        (
            "runtime_performance_observation_guest_pc_trace_timing_acceptance_verifier_core_contract",
            "runtime_performance_observation_projects_guest_pc_trace_timing",
            "guest_pc_trace_timing_acceptance_verifier_core_contract",
        ),
        (
            "runtime_performance_observation_row_value_timing_acceptance_verifier_core_contract",
            "runtime_performance_observation_projects_witness_opening_row_value_timing",
            "witness_opening_row_value_timing_acceptance_verifier_core_contract",
        ),
        (
            "runtime_performance_observation_constant_material_timing_acceptance_verifier_core_contract",
            "runtime_performance_observation_projects_constant_material_validation_timing",
            "constant_material_validation_timing_acceptance_verifier_core_contract",
        ),
        (
            "runtime_performance_observation_prover_gpu_mode_acceptance_verifier_core_contract",
            "runtime_performance_observation_projects_prover_gpu_mode",
            "prover_gpu_mode_acceptance_verifier_core_contract",
        ),
        (
            "runtime_performance_observation_gpu_run_options_acceptance_verifier_core_contract",
            "runtime_performance_observation_projects_gpu_run_options",
            "gpu_run_options_acceptance_verifier_core_contract",
        ),
        (
            "runtime_performance_observation_cuda_backend_acceptance_verifier_core_contract",
            "runtime_performance_observation_projects_cuda_backend",
            "cuda_backend_acceptance_verifier_core_contract",
        ),
        (
            "runtime_performance_observation_cuda_allocator_timing_acceptance_verifier_core_contract",
            "runtime_performance_observation_projects_cuda_allocator_timing",
            "cuda_allocator_timing_acceptance_verifier_core_contract",
        ),
        (
            "runtime_performance_observation_finish_timing_acceptance_verifier_core_contract",
            "runtime_performance_observation_projects_proof_artifact_finish_timing",
            "proof_artifact_finish_timing_acceptance_verifier_core_contract",
        ),
        (
            "runtime_performance_observation_proof_timing_batch_acceptance_verifier_core_contract",
            "runtime_performance_observation_projects_proof_timing_batch",
            "proof_timing_batch_acceptance_verifier_core_contract",
        ),
    ] {
        lean_binding::assert_theorem_body_contains(
            &runtime_performance_source,
            theorem_name,
            &[
                projector,
                concat!(
                    "runtime_performance_observation_projected_metadata_",
                    "acceptance_verifier_core_contract"
                ),
            ],
        );
        lean_binding::assert_theorem_body_omits(
            &runtime_performance_source,
            theorem_name,
            &[
                "runtime_performance_observation_acceptance_verifier_core_contract",
                "sound_witness_implies_verifier_core_contract",
                wrapper,
            ],
        );
    }
    assert!(
        runtime_performance_source
            .contains("structure RuntimePerformanceObservationProjectedCoreContracts")
            && runtime_performance_source.contains("timingObservations :")
            && runtime_performance_source.contains("proofArtifactFinishTiming :")
            && runtime_performance_source.contains("proofTimingBatch :"),
        "Lean runtime performance checks should batch projected wrapper core contracts"
    );
    lean_binding::assert_theorem_body_contains(
        &runtime_performance_source,
        "runtime_performance_observation_projected_core_contracts",
        &[
            "runtime_performance_observation_timing_observations_acceptance_verifier_core_contract",
            "runtime_performance_observation_guest_pc_trace_timing_acceptance_verifier_core_contract",
            "runtime_performance_observation_row_value_timing_acceptance_verifier_core_contract",
            "runtime_performance_observation_constant_material_timing_acceptance_verifier_core_contract",
            "runtime_performance_observation_prover_gpu_mode_acceptance_verifier_core_contract",
            "runtime_performance_observation_gpu_run_options_acceptance_verifier_core_contract",
            "runtime_performance_observation_cuda_backend_acceptance_verifier_core_contract",
            concat!(
                "runtime_performance_observation_cuda_allocator_timing_",
                "acceptance_verifier_core_contract"
            ),
            "runtime_performance_observation_finish_timing_acceptance_verifier_core_contract",
            concat!(
                "runtime_performance_observation_proof_timing_batch_",
                "acceptance_verifier_core_contract"
            ),
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &runtime_performance_source,
        "runtime_performance_observation_projected_core_contracts",
        &[
            "ignored_metadata_acceptance_verifier_core_contract",
            "abstract_verifier_sound",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    assert!(
        timing_projected_source.contains("structure TimingProjectedCoreContracts")
            && timing_projected_source.contains("timingObservations :")
            && timing_projected_source.contains("guestPcTraceTiming :"),
        "Lean timing checks should batch top-level timing wrapper core contracts"
    );
    lean_binding::assert_theorem_declarations(
        &timing_projected_source,
        &["timing_projected_metadata_acceptance_verifier_core_contract"],
    );
    lean_binding::assert_theorem_body_contains(
        &timing_projected_source,
        "timing_projected_core_contracts",
        &["timing_projected_metadata_acceptance_verifier_core_contract"],
    );
    lean_binding::assert_theorem_body_omits(
        &timing_projected_source,
        "timing_projected_core_contracts",
        &[
            "timing_observation_acceptance_verifier_core_contract",
            "guest_pc_trace_timing_acceptance_verifier_core_contract",
            "ignored_metadata_acceptance_verifier_core_contract",
            "abstract_verifier_sound",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    assert!(
        proof_timing_projected_source.contains("structure ProofTimingProjectedCoreContracts")
            && proof_timing_projected_source.contains("witnessOpeningRowValueTiming :")
            && proof_timing_projected_source.contains("proofArtifactFinishTiming :")
            && proof_timing_projected_source.contains("proofTimingBatch :"),
        "Lean proof timing checks should batch top-level proof timing wrapper core contracts"
    );
    lean_binding::assert_theorem_declarations(
        &proof_timing_projected_source,
        &[
            "proof_timing_projected_metadata_acceptance_verifier_core_contract",
            "proof_timing_projected_finish_summary_required_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &proof_timing_projected_source,
        "proof_timing_projected_core_contracts",
        &[
            "batchTiming : Option ProofTimingBatchSummary",
            "ProofTimingBatchObservedAcceptance",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &proof_timing_projected_source,
        "proof_timing_projected_finish_summary_required_verifier_core_contract",
        &["proof_artifact_finish_timing_some_summary_acceptance_verifier_core_contract"],
    );
    lean_binding::assert_theorem_body_omits(
        &proof_timing_projected_source,
        "proof_timing_projected_finish_summary_required_verifier_core_contract",
        &[
            "proof_artifact_finish_timing_some_summary_acceptance_sound",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &proof_timing_projected_source,
        "proof_timing_projected_core_contracts",
        &[
            "proof_timing_projected_metadata_acceptance_verifier_core_contract",
            "proof_timing_batch_acceptance_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &proof_timing_projected_source,
        "proof_timing_projected_core_contracts",
        &[
            "witness_opening_row_value_timing_acceptance_verifier_core_contract",
            "constant_material_validation_timing_acceptance_verifier_core_contract",
            "prover_gpu_mode_acceptance_verifier_core_contract",
            "gpu_run_options_acceptance_verifier_core_contract",
            "cuda_backend_acceptance_verifier_core_contract",
            "cuda_allocator_timing_acceptance_verifier_core_contract",
            "proof_artifact_finish_timing_acceptance_verifier_core_contract",
            "ignored_metadata_acceptance_verifier_core_contract",
            "abstract_verifier_sound",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    assert!(
        auxiliary_projected_source.contains("structure AuxiliaryProjectedCoreContracts")
            && auxiliary_projected_source.contains("timing :")
            && auxiliary_projected_source.contains("proofTiming :")
            && auxiliary_projected_source.contains("runtimePerformance :"),
        "Lean auxiliary projected checks should batch timing, proof timing, and runtime performance contracts"
    );
    lean_binding::assert_theorem_body_contains(
        &auxiliary_projected_source,
        "runtime_performance_observation_auxiliary_projected_core_contracts",
        &[
            "timing_projected_core_contracts",
            "proof_timing_projected_core_contracts",
            "runtime_performance_observation_projected_core_contracts",
            "runtime_performance_observation_projects_timing_observations",
            "runtime_performance_observation_projects_guest_pc_trace_timing",
            "runtime_performance_observation_projects_witness_opening_row_value_timing",
            "runtime_performance_observation_projects_constant_material_validation_timing",
            "runtime_performance_observation_projects_prover_gpu_mode",
            "runtime_performance_observation_projects_gpu_run_options",
            "runtime_performance_observation_projects_cuda_backend",
            "runtime_performance_observation_projects_cuda_allocator_timing",
            "runtime_performance_observation_projects_proof_artifact_finish_timing",
            "runtime_performance_observation_projects_proof_timing_batch",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &auxiliary_projected_source,
        "runtime_performance_observation_auxiliary_projected_core_contracts",
        &[
            "ignored_metadata_acceptance_verifier_core_contract",
            "abstract_verifier_sound",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_declarations(
        &gpu_runtime_source,
        &[
            "checked_acceptance_sound_witness",
            "checked_acceptance_verifier_core_contract",
        ],
    );
    assert_eq!(
        gpu_runtime_source
            .matches("theorem checked_acceptance_sound_witness")
            .count(),
        1,
        "Lean GPU runtime internal checks should centralize checked-acceptance SoundWitness projection"
    );
    lean_binding::assert_theorem_body_contains(
        &gpu_runtime_source,
        "checked_acceptance_sound_witness",
        &["auxiliary_checked_acceptance_sound_witness"],
    );
    lean_binding::assert_theorem_body_omits(
        &gpu_runtime_source,
        "checked_acceptance_sound_witness",
        &[
            "abstract_verifier_sound",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_contains_identifier(
        &gpu_runtime_source,
        "checked_acceptance_sound_witness",
        "auxiliary_checked_acceptance_sound_witness",
    );
    for shortcut in [
        "abstract_verifier_sound",
        "sound_witness_implies_verifier_core_contract",
    ] {
        lean_binding::assert_theorem_body_omits_identifier(
            &gpu_runtime_source,
            "checked_acceptance_sound_witness",
            shortcut,
        );
    }
    lean_binding::assert_theorem_body_contains(
        &gpu_runtime_source,
        "checked_acceptance_verifier_core_contract",
        &["auxiliary_checked_acceptance_verifier_core_contract"],
    );
    lean_binding::assert_theorem_body_omits(
        &gpu_runtime_source,
        "checked_acceptance_verifier_core_contract",
        &[
            "checked_acceptance_sound_witness",
            "assumptions.crypto.transcript_binding",
            "assumptions.semantic.public_input_binding",
            "assumptions.crypto.pcs_opening_sound",
            "assumptions.crypto.fri_query_sound",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_contains_identifier(
        &gpu_runtime_source,
        "checked_acceptance_verifier_core_contract",
        "auxiliary_checked_acceptance_verifier_core_contract",
    );
    for shortcut in [
        "checked_acceptance_sound_witness",
        "assumptions.crypto.transcript_binding",
        "assumptions.semantic.public_input_binding",
        "assumptions.crypto.pcs_opening_sound",
        "assumptions.crypto.fri_query_sound",
        "sound_witness_implies_verifier_core_contract",
    ] {
        lean_binding::assert_theorem_body_omits_identifier(
            &gpu_runtime_source,
            "checked_acceptance_verifier_core_contract",
            shortcut,
        );
    }
    for theorem_name in [
        "guest_pc_trace_large_gpu_gate_checked_acceptance_sound",
        "guest_pc_trace_traceless_commitment_input_checked_acceptance_sound",
        "guest_pc_trace_traceless_segment_output_checked_acceptance_sound",
        "guest_pc_trace_cross_root_materialization_checked_acceptance_sound",
        "guest_pc_trace_commit_mode_checked_acceptance_sound",
        "guest_pc_trace_device_trace_source_checked_acceptance_sound",
        "guest_pc_trace_sparse_source_checked_acceptance_sound",
        "guest_pc_trace_terminal_sparse_source_checked_acceptance_sound",
        "fri_retained_stage_source_checked_acceptance_sound",
        "guest_pc_trace_cuda_run_checked_acceptance_sound",
    ] {
        let theorem_body = lean_binding::theorem_body(&gpu_runtime_source, theorem_name);
        assert!(
            theorem_body.contains("GpuRuntimeInternal.checked_acceptance_sound_witness"),
            "{theorem_name} should reuse the centralized checked-acceptance SoundWitness projector"
        );
    }
    let finish_summary_wrapper_uses = lean_source
        .matches(
            "proof_artifact_finish_timing_some_summary_acceptance_sound\n      assumptions\n      { summary with",
        )
        .count();
    assert!(
        finish_summary_wrapper_uses >= 10,
        "Lean proof finish timing specializations should reuse the some-summary soundness wrapper"
    );
    assert!(
        lean_source.contains("guestTraceDescriptorCompactRowCount")
            && lean_source.contains("guestTraceDescriptorWideRowCount"),
        "Lean guest PC timing summary should expose descriptor width row counts"
    );
    assert!(
        lean_source.contains("guest_pc_trace_descriptor_width_counts_acceptance_sound")
            && lean_source.contains(
                "guest_pc_trace_descriptor_width_counts_acceptance_verifier_core_contract"
            ),
        "Lean guest PC timing summary should prove descriptor width row counts are verifier-core-neutral"
    );
    assert!(
        lean_source.contains("guestDeviceSourceDescriptorUploadWordCount")
            && lean_source.contains(
                "guest_pc_trace_descriptor_upload_word_count_acceptance_sound"
            )
            && lean_source.contains(
                "guest_pc_trace_descriptor_upload_word_count_acceptance_verifier_core_contract"
            ),
        "Lean guest PC timing summary should prove descriptor upload word counts are verifier-core-neutral"
    );
    assert!(
        lean_source.contains("guestDeviceSourceDescriptorUploadByteCount")
            && lean_source.contains("guestDeviceSourceDescriptorUploadRowCount")
            && lean_source.contains("guest_pc_trace_descriptor_upload_shape_acceptance_sound")
            && lean_source
                .contains("guest_pc_trace_descriptor_upload_shape_acceptance_verifier_core_contract"),
        "Lean guest PC timing summary should prove descriptor upload byte and row counts are verifier-core-neutral"
    );
    lean_binding::assert_theorem_declarations(
        &timing_core_source,
        &["guest_pc_trace_timing_some_summary_acceptance_verifier_core_contract"],
    );
    for theorem_name in [
        "guest_pc_trace_stream_elapsed_timing_acceptance_verifier_core_contract",
        "guest_pc_trace_descriptor_width_counts_acceptance_verifier_core_contract",
        "guest_pc_trace_report_timing_acceptance_verifier_core_contract",
        "guest_pc_trace_report_subtiming_acceptance_verifier_core_contract",
        "guest_pc_trace_report_lower_subtiming_acceptance_verifier_core_contract",
        "guest_pc_trace_emit_descriptor_wait_timing_acceptance_verifier_core_contract",
        "guest_pc_trace_device_source_timing_acceptance_verifier_core_contract",
        "guest_pc_trace_regular_stage_timing_acceptance_verifier_core_contract",
        "guest_pc_trace_shape_counts_acceptance_verifier_core_contract",
        "guest_pc_trace_memory_access_shape_acceptance_verifier_core_contract",
        "guest_pc_trace_report_buffer_capacity_acceptance_verifier_core_contract",
        "guest_pc_trace_descriptor_upload_word_count_acceptance_verifier_core_contract",
        "guest_pc_trace_descriptor_upload_shape_acceptance_verifier_core_contract",
        "guest_pc_trace_source_retention_byte_counts_acceptance_verifier_core_contract",
        "guest_pc_trace_source_retention_counts_acceptance_verifier_core_contract",
        "guest_pc_trace_descriptor_buffer_retention_byte_counts_acceptance_verifier_core_contract",
        "guest_pc_trace_descriptor_buffer_retention_counts_acceptance_verifier_core_contract",
        "guest_pc_trace_leaf_output_cache_counts_acceptance_verifier_core_contract",
        "guest_pc_trace_leaf_extend_timing_acceptance_verifier_core_contract",
        "guest_pc_trace_leaf_setup_timing_acceptance_verifier_core_contract",
        "guest_pc_trace_leaf_work_timing_acceptance_verifier_core_contract",
        "guest_pc_trace_leaf_coset_timing_acceptance_verifier_core_contract",
        "guest_pc_trace_tree_commit_timing_acceptance_verifier_core_contract",
        "guest_pc_trace_segment_commit_worker_timing_acceptance_verifier_core_contract",
        "guest_pc_trace_stage_timing_acceptance_verifier_core_contract",
    ] {
        lean_binding::assert_theorem_body_contains(
            &timing_source,
            theorem_name,
            &["guest_pc_trace_timing_some_summary_acceptance_verifier_core_contract"],
        );
        lean_binding::assert_theorem_body_omits(
            &timing_source,
            theorem_name,
            &["sound_witness_implies_verifier_core_contract"],
        );
    }
    assert!(
        lean_source.contains("guestStageSourceRetentionRetainedByteCount")
            && lean_source.contains(
                "guest_pc_trace_source_retention_byte_counts_acceptance_sound"
            )
            && lean_source.contains(
                "guest_pc_trace_source_retention_byte_counts_acceptance_verifier_core_contract"
            ),
        "Lean guest PC timing summary should prove source retention byte counts are verifier-core-neutral"
    );
    assert!(
        lean_source.contains("guestStageSourceRetentionAttemptCount")
            && lean_source.contains("guestStageSourceRetentionRetainedCount")
            && lean_source.contains("guestStageSourceRetentionRejectedCount")
            && lean_source.contains("guest_pc_trace_source_retention_counts_acceptance_sound")
            && lean_source
                .contains("guest_pc_trace_source_retention_counts_acceptance_verifier_core_contract"),
        "Lean guest PC timing summary should prove source retention attempt counts are verifier-core-neutral"
    );
    assert!(
        lean_source.contains("guestDescriptorBufferRetentionRetainedByteCount")
            && lean_source.contains(
                "guest_pc_trace_descriptor_buffer_retention_byte_counts_acceptance_sound"
            )
            && lean_source.contains(
                "guest_pc_trace_descriptor_buffer_retention_byte_counts_acceptance_verifier_core_contract"
            ),
        "Lean guest PC timing summary should prove descriptor buffer retention byte counts are verifier-core-neutral"
    );
    assert!(
        lean_source.contains("guestDescriptorBufferRetentionAttemptCount")
            && lean_source.contains("guestDescriptorBufferRetentionRetainedCount")
            && lean_source.contains("guestDescriptorBufferRetentionRejectedCount")
            && lean_source
                .contains("guest_pc_trace_descriptor_buffer_retention_counts_acceptance_sound")
            && lean_source.contains(
                "guest_pc_trace_descriptor_buffer_retention_counts_acceptance_verifier_core_contract"
            ),
        "Lean guest PC timing summary should prove descriptor buffer retention attempt counts are verifier-core-neutral"
    );
    assert!(
        lean_source.contains("guestTraceStreamElapsedMilliseconds")
            && lean_source.contains("guest_pc_trace_stream_elapsed_timing_acceptance_sound")
            && lean_source.contains(
                "guest_pc_trace_stream_elapsed_timing_acceptance_verifier_core_contract"
            ),
        "Lean guest PC timing summary should prove stream elapsed timing metadata is verifier-core-neutral"
    );
    assert!(
        lean_source.contains("guest_pc_trace_report_timing_acceptance_sound")
            && lean_source
                .contains("guest_pc_trace_report_timing_acceptance_verifier_core_contract"),
        "Lean guest PC timing summary should prove report-loop timing metadata is verifier-core-neutral"
    );
    assert!(
        lean_source.contains("guestTraceReportRowValidationMilliseconds")
            && lean_source.contains("guestTraceReportSourceValuesMilliseconds")
            && lean_source.contains("guestTraceReportRegisterAccessMilliseconds")
            && lean_source.contains("guestTraceReportMemoryAccessMilliseconds")
            && lean_source.contains("guestTraceReportStoreApplyMilliseconds")
            && lean_source.contains("guestTraceReportVisitMilliseconds")
            && lean_source.contains("guest_pc_trace_report_subtiming_acceptance_sound")
            && lean_source
                .contains("guest_pc_trace_report_subtiming_acceptance_verifier_core_contract"),
        "Lean guest PC timing summary should prove report subtiming metadata is verifier-core-neutral"
    );
    assert!(
        lean_source.contains("guestTraceSingleRowReportLowerMilliseconds")
            && lean_source.contains("guestTraceMultiRowReportLowerMilliseconds")
            && lean_source.contains("guestTracePendingDmaReportLowerMilliseconds")
            && lean_source.contains("guestTraceAmoReportLowerMilliseconds")
            && lean_source.contains("guestTraceStoreConditionalReportLowerMilliseconds")
            && lean_source.contains("guest_pc_trace_report_lower_subtiming_acceptance_sound")
            && lean_source.contains(
                "guest_pc_trace_report_lower_subtiming_acceptance_verifier_core_contract"
            ),
        "Lean guest PC timing summary should prove report-lower subtiming metadata is verifier-core-neutral"
    );
    assert!(
        lean_source.contains("guestTraceEmitMilliseconds")
            && lean_source.contains("guestTraceDescriptorMilliseconds")
            && lean_source.contains("guestTraceDescriptorRowCount")
            && lean_source.contains("guestTracePendingSendWaitMilliseconds")
            && lean_source.contains("guestTracePendingReceiveWaitMilliseconds")
            && lean_source.contains("guestTraceSegmentSendWaitMilliseconds")
            && lean_source.contains("guestTraceSegmentReceiveWaitMilliseconds")
            && lean_source.contains("guestTraceParallelLowerWorkerCount")
            && lean_source.contains("guestTraceParallelLowerDispatchedCount")
            && lean_source.contains("guestTraceParallelLowerReceivedCount")
            && lean_source.contains("guestTraceParallelLowerEmittedCount")
            && lean_source.contains("guestTraceParallelLowerMaxReorderCount")
            && lean_source.contains("guestTraceOwnedStreamingLowerSegmentCount")
            && lean_source.contains("guestTraceParallelLowerStreamStartDispatchWaitMilliseconds")
            && lean_source.contains("guestTraceParallelLowerStreamChunkDispatchWaitMilliseconds")
            && lean_source.contains("guestTraceParallelLowerStreamSegmentDispatchWaitMilliseconds")
            && lean_source.contains("guestTraceParallelLowerStreamFinishDispatchWaitMilliseconds")
            && lean_source.contains("guest_pc_trace_emit_descriptor_wait_timing_acceptance_sound")
            && lean_source.contains(
                "guest_pc_trace_emit_descriptor_wait_timing_acceptance_verifier_core_contract"
            ),
        "Lean guest PC timing summary should prove emit, descriptor, and channel-wait timing metadata is verifier-core-neutral"
    );
    assert!(
        lean_source.contains("guestDeviceSourceBuildMilliseconds")
            && lean_source.contains("guestDeviceSourceDescriptorUploadMilliseconds")
            && lean_source.contains("guestDeviceSourceTraceExpandMilliseconds")
            && lean_source.contains("guest_pc_trace_device_source_timing_acceptance_sound")
            && lean_source
                .contains("guest_pc_trace_device_source_timing_acceptance_verifier_core_contract"),
        "Lean guest PC timing summary should prove device-source timing metadata is verifier-core-neutral"
    );
    assert!(
        lean_source.contains("guestRegularConstraintsMilliseconds")
            && lean_source.contains("guestRegularHintsMilliseconds")
            && lean_source.contains("guestStageCommitMilliseconds")
            && lean_source.contains("guestStageTraceExtractMilliseconds")
            && lean_source.contains("guest_pc_trace_regular_stage_timing_acceptance_sound")
            && lean_source
                .contains("guest_pc_trace_regular_stage_timing_acceptance_verifier_core_contract"),
        "Lean guest PC timing summary should prove regular and stage timing metadata is verifier-core-neutral"
    );
    assert!(
        lean_source.contains("guest_pc_trace_stage_timing_acceptance_sound")
            && lean_source
                .contains("guest_pc_trace_stage_timing_acceptance_verifier_core_contract"),
        "Lean guest PC timing summary should prove per-stage timing metadata is verifier-core-neutral"
    );
    assert!(
        lean_source.contains("guestStageLeafExtendWorkMilliseconds")
            && lean_source.contains("guest_pc_trace_leaf_extend_timing_acceptance_sound")
            && lean_source
                .contains("guest_pc_trace_leaf_extend_timing_acceptance_verifier_core_contract"),
        "Lean guest PC timing summary should prove leaf extend timing metadata is verifier-core-neutral"
    );
    assert!(
        lean_source.contains("temporaryBufferReuseAllowed")
            && lean_source.contains("pendingDeviceReadsComplete")
            && lean_source.contains("temporaryBufferReuseImpliesSameRequest")
            && lean_source.contains("temporaryBufferReuseImpliesPendingReadsComplete"),
        "Lean auxiliary checks should bind temporary GPU buffer reuse to same requests and completed pending reads"
    );
    assert!(
        lean_source.contains("leafOutputBufferFullyOverwritten")
            && lean_source.contains("leafOutputBufferLengthMatches")
            && lean_source.contains("leafOutputBufferReuseImpliesCanonicalLeafBytes")
            && lean_source.contains(
                "gpu_leaf_output_buffer_reuse_checked_acceptance_projects_length_match"
            )
            && source_hot_paths.contains("source_device_leaf_extension_reuses_only_narrow_output_cache")
            && source_hot_paths.contains("should_cache_leaf_output(view.column_count)"),
        "Lean auxiliary checks should bind leaf output buffer reuse to exact-size fully overwritten canonical leaf bytes"
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
        lean_source.contains("pendingNoWaitLimitBytes")
            && lean_source.contains("pendingAllocationBytes")
            && lean_source.contains("freshAllocationBytes")
            && lean_source.contains("bypassSelected")
            && lean_source.contains("GpuAllocatorNoWaitLimitDecisionMatches")
            && lean_source.contains("noWaitLimitConfigAccepted")
            && lean_source.contains("noWaitLimitConfigImpliesDecisionMatches"),
        "Lean auxiliary checks should bind allocator no-wait runtime limits to the bypass decision"
    );
    assert!(
        lean_source.contains("defaultSegmentQueueCapacity")
            && lean_source.contains("configuredSegmentQueueCapacity")
            && lean_source.contains("effectiveSegmentQueueCapacity")
            && lean_source.contains("GuestPcTraceSegmentQueueDecisionMatches")
            && lean_source.contains("segmentQueueConfigAccepted")
            && lean_source.contains("segmentQueueConfigImpliesDecisionMatches")
            && guest_backend_source.contains("LZVM_GUEST_PC_TRACE_SEGMENT_QUEUE")
            && guest_backend_source.contains("fn guest_pc_trace_segment_queue_capacity")
            && guest_backend_source.contains("mpsc::sync_channel(guest_pc_trace_segment_queue_capacity())"),
        "Lean auxiliary checks should bind guest trace segment queue capacity selection to the Rust runtime knob"
    );
    assert!(
        lean_source.contains("GuestPcTraceLargeGpuGateInstructionThreshold")
            && lean_source.contains(": Nat := 1000000")
            && lean_source.contains("defaultLargeTraceInstructionThreshold")
            && lean_source.contains(
                "config.defaultLargeTraceInstructionThreshold =\n    GuestPcTraceLargeGpuGateInstructionThreshold"
            )
            && lean_source.contains("requestedInstructionLimit")
            && lean_source.contains("gpuBackendAvailable")
            && lean_source.contains("largeTraceAllowed")
            && lean_source.contains("GuestPcTraceLargeGpuGateDecisionMatches")
            && lean_source.contains("largeGpuGateConfigAccepted")
            && lean_source.contains("largeGpuGateConfigImpliesDecisionMatches")
            && lean_source.contains(
                "guest_pc_trace_large_gpu_gate_checked_acceptance_requires_runtime_memory_for_large_allowed"
            )
            && lean_source.contains(
                "guest_pc_trace_large_gpu_gate_decision_projects_observed_memory_floor_for_large_allowed"
            )
            && lean_source.contains("guest_pc_trace_large_gpu_gate_checked_acceptance_sound")
            && lean_source.contains(
                "guest_pc_trace_large_gpu_gate_checked_acceptance_verifier_core_contract"
            )
            && prove_witness_source.contains("validate_large_guest_pc_runtime_gpu")
            && gpu_preflight_source.contains("fn validate_large_guest_pc_gpu")
            && gpu_preflight_source.contains("fn validate_large_guest_pc_runtime_gpu")
            && gpu_preflight_source.contains("const GUEST_PC_TRACE_GPU_SIZE_THRESHOLD: u64 = 1_000_000")
            && gpu_preflight_source
                .contains("instruction_limit.unwrap_or(0) >= GUEST_PC_TRACE_GPU_SIZE_THRESHOLD")
            && gpu_preflight_source.contains("lzvm_prover::gpu_setup_available()")
            && gpu_preflight_source.contains("lzvm_prover::gpu_memory_info()")
            && gpu_preflight_source.contains("validate_large_guest_pc_gpu_memory"),
        "Lean auxiliary checks should bind the large guest trace GPU gate to the Rust runtime guard and memory preflight"
    );
    assert!(
        lean_source.contains("GuestPcTraceTracelessCommitmentInputConfig")
            && lean_source.contains("configuredTracelessCommitmentInput")
            && lean_source.contains("effectiveTracelessCommitmentInput")
            && lean_source.contains("GuestPcTraceTracelessCommitmentInputDecisionMatches")
            && lean_source.contains("tracelessCommitmentInputConfigAccepted")
            && lean_source.contains("tracelessCommitmentInputConfigImpliesDecisionMatches")
            && lean_source
                .contains("guest_pc_trace_traceless_commitment_input_checked_acceptance_sound")
            && lean_source.contains(
                "guest_pc_trace_traceless_commitment_input_checked_acceptance_projects_default_enabled"
            )
            && lean_source.contains(
                "guest_pc_trace_traceless_commitment_input_checked_acceptance_verifier_core_contract"
            )
            && witness_execution_source
                .contains("fn guest_pc_trace_less_commitment_input_enabled")
            && witness_execution_source.contains("LZVM_CUDA_GUEST_PC_TRACELESS_COMMITMENT_INPUT")
            && witness_execution_source.contains("unwrap_or(true)")
            && witness_execution_source.contains("Ok((None, device_segment_material))"),
        "Lean auxiliary checks should bind traceless guest trace commitment input selection to the Rust runtime guard"
    );
    assert!(
        lean_source.contains("GuestPcTraceTracelessSegmentOutputConfig")
            && lean_source.contains("configuredTracelessSegmentOutput")
            && lean_source.contains("effectiveTracelessSegmentOutput")
            && lean_source.contains("GuestPcTraceTracelessSegmentOutputDecisionMatches")
            && lean_source.contains("tracelessSegmentOutputConfigAccepted")
            && lean_source.contains("tracelessSegmentOutputConfigImpliesDecisionMatches")
            && lean_source.contains("guest_pc_trace_traceless_segment_output_checked_acceptance_sound")
            && lean_source.contains(
                "guest_pc_trace_traceless_segment_output_checked_acceptance_projects_default_enabled"
            )
            && lean_source.contains(
                "guest_pc_trace_traceless_segment_output_checked_acceptance_verifier_core_contract"
            )
            && guest_backend_source.contains("fn guest_pc_trace_less_segment_output_enabled")
            && guest_backend_source.contains("LZVM_CUDA_GUEST_PC_TRACELESS_SEGMENT_OUTPUT")
            && guest_backend_source.contains(
                "env_flag_enabled(\"LZVM_CUDA_GUEST_PC_TRACELESS_SEGMENT_OUTPUT\", true)"
        ),
        "Lean auxiliary checks should bind traceless guest trace segment output selection to the Rust runtime guard"
    );
    assert!(
        lean_source.contains("GuestPcTraceCrossSegmentRootMaterializationConfig")
            && lean_source.contains("configuredCrossSegmentRootMaterialization")
            && lean_source.contains("effectiveCrossSegmentRootMaterialization")
            && lean_source.contains("supportedInputByteLimit")
            && lean_source.contains("GuestPcTraceCrossSegmentRootMaterializationDecisionMatches")
            && lean_source.contains("crossSegmentRootMaterializationConfigAccepted")
            && lean_source.contains(
                "crossSegmentRootMaterializationConfigImpliesDecisionMatches"
            )
            && lean_source
                .contains("GuestPcTraceCrossSegmentRootMaterializationCheckedAcceptance")
            && lean_source.contains(
                "GuestPcTraceCrossSegmentRootMaterializationDecisionMatches config"
            )
            && gpu_runtime_source.contains(
                "guest_pc_trace_cross_root_materialization_checked_acceptance_sound"
            )
            && gpu_runtime_source.contains(
                "guest_pc_trace_cross_root_materialization_checked_acceptance_verifier_core_contract"
            )
            && witness_execution_source
                .contains("fn guest_pc_cross_segment_root_materialization_enabled")
            && witness_execution_source.contains("LZVM_CUDA_GUEST_PC_CROSS_SEGMENT_ROOTS")
            && witness_execution_source
                .contains("fn guest_pc_cross_segment_root_materialization_selected")
            && witness_execution_source.contains(
                "guest_pc_cross_segment_root_materialization_supported_for_input(input_byte_count)"
            )
            && witness_execution_source.contains(
                "const SUPPORTED_INPUT_BYTE_LIMIT: usize = 8 * 1024 * 1024"
            ),
        "Lean auxiliary checks should bind cross-segment root materialization to the Rust runtime guard and input-size limit"
    );
    assert!(
        lean_source.contains("GuestPcTraceSegmentCommitModeConfig")
            && lean_source.contains("configuredWorkerCount")
            && lean_source.contains("effectiveWorkerCount")
            && lean_source.contains("configuredAsyncSingleWorker")
            && lean_source.contains("effectiveAsyncSingleWorker")
            && lean_source.contains("tracelessCommitmentInputConfig")
            && lean_source.contains("crossSegmentRootMaterializationConfig")
            && lean_source.contains("descriptorBufferRetentionConfig")
            && lean_source.contains("selectedDescriptorBufferRetention")
            && lean_source.contains("effectivePendingRootMaterializationWindow")
            && lean_source.contains("GuestPcTraceSegmentCommitModeDecisionMatches")
            && lean_source.contains("segmentCommitModeConfigAccepted")
            && lean_source.contains("segmentCommitModeConfigImpliesDecisionMatches")
            && lean_source.contains("GuestPcTraceSegmentCommitModeCheckedAcceptance")
            && lean_source.contains("GuestPcTraceDescriptorBufferRetentionDecisionMatches")
            && gpu_runtime_source.contains("guest_pc_trace_commit_mode_checked_acceptance_sound")
            && gpu_runtime_source
                .contains("guest_pc_trace_commit_mode_checked_acceptance_verifier_core_contract")
            && gpu_runtime_source
                .contains("guest_pc_trace_commit_mode_async_requires_single_worker")
            && gpu_runtime_source.contains(
                "guest_pc_trace_commit_mode_checked_acceptance_projects_descriptor_retention"
            )
            && gpu_runtime_source.contains(
                "guest_pc_trace_commit_mode_checked_acceptance_projects_disabled_root_window"
            )
            && witness_execution_source.contains("struct GuestPcTraceSegmentCommitMode")
            && witness_execution_source.contains("fn from_input(")
            && witness_execution_source
                .contains("guest_pc_trace_segment_commit_worker_count_for_input_with_override")
            && witness_execution_source
                .contains("guest_pc_trace_segment_commit_async_single_worker_enabled")
            && witness_execution_source
                .contains("guest_pc_trace_traceless_commitment_input_selected")
            && witness_execution_source
                .contains("guest_pc_cross_segment_root_materialization_selected")
            && compact_source_contains(
                &witness_execution_source,
                "guest_pc_descriptor_buffer_retention_enabled(input_byte_count,)",
            )
            && witness_execution_source
                .contains("WitnessTraceCudaRunConfig::from_input(input_byte_count)")
            && witness_execution_source
                .contains("trace_cuda_run_config: Some(trace_cuda_run_config)")
            && witness_execution_source.contains(
                "let pending_root_materialization_window = if cross_segment_root_materialization"
            )
            && witness_execution_source
                .contains("GuestPcTraceSegmentCommitWorkerPool::new(scope, segment_commit_mode)"),
        "Lean auxiliary checks should bind cached segment commit mode to the Rust runtime snapshot"
    );
    for (theorem_name, projector) in [
        (
            "guest_pc_trace_traceless_commitment_input_checked_acceptance_projects_default_enabled",
            "guest_pc_trace_traceless_commitment_input_checked_acceptance_projects_decision",
        ),
        (
            "guest_pc_trace_traceless_segment_output_checked_acceptance_projects_default_enabled",
            "guest_pc_trace_traceless_segment_output_checked_acceptance_projects_decision",
        ),
        (
            concat!(
                "guest_pc_trace_cross_root_materialization_checked_acceptance_",
                "projects_default_enabled"
            ),
            "guest_pc_trace_cross_root_materialization_checked_acceptance_projects_decision",
        ),
        (
            concat!(
                "guest_pc_trace_cross_root_materialization_checked_acceptance_",
                "projects_disabled"
            ),
            "guest_pc_trace_cross_root_materialization_checked_acceptance_projects_decision",
        ),
    ] {
        lean_binding::assert_theorem_body_contains(&gpu_runtime_source, theorem_name, &[projector]);
        lean_binding::assert_theorem_body_omits(
            &gpu_runtime_source,
            theorem_name,
            &["GpuRuntimeInternal.checked_acceptance_sound_witness"],
        );
    }
    for (theorem_name, projector) in [
        (
            "guest_pc_trace_commit_mode_async_requires_single_worker",
            "rw [asyncFalse] at asyncSelected",
        ),
        (
            "guest_pc_trace_commit_mode_checked_acceptance_projects_disabled_root_window",
            "guest_pc_trace_commit_mode_checked_acceptance_projects_decision",
        ),
    ] {
        lean_binding::assert_theorem_body_contains(&gpu_runtime_source, theorem_name, &[projector]);
        lean_binding::assert_theorem_body_omits(
            &gpu_runtime_source,
            theorem_name,
            &["GpuRuntimeInternal.checked_acceptance_sound_witness"],
        );
    }
    assert!(
        lean_source.contains("GuestPcTraceDeviceTraceSourceConfig")
            && lean_source.contains("configuredDeviceTraceSourceEnabled")
            && lean_source.contains("effectiveDeviceTraceSourceEnabled")
            && lean_source.contains("configuredDeviceTraceSourceDeepValidation")
            && lean_source.contains("effectiveDeviceTraceSourceDeepValidation")
            && lean_source.contains("GuestPcTraceDeviceTraceSourceDecisionMatches")
            && lean_source.contains("deviceTraceSourceConfigAccepted")
            && lean_source.contains("deviceTraceSourceConfigImpliesDecisionMatches")
            && lean_source.contains("guest_pc_trace_device_trace_source_checked_acceptance_sound")
            && lean_source.contains(
                "guest_pc_trace_device_trace_source_checked_acceptance_verifier_core_contract"
            )
            && guest_backend_source.contains("fn guest_pc_device_trace_source_enabled")
            && guest_backend_source.contains(
                "env_flag_enabled(\"LZVM_CUDA_GUEST_PC_DEVICE_TRACE_SOURCE\", true)"
            )
            && guest_backend_source
                .contains("fn guest_pc_device_trace_source_deep_validation_enabled")
            && guest_backend_source.contains(
                "env_flag_enabled(\"LZVM_CUDA_VALIDATE_GUEST_PC_DEVICE_TRACE_SOURCE\", false)"
            )
            && guest_backend_source.contains("if !guest_pc_device_trace_source_enabled()")
            && guest_backend_source
                .contains("if guest_pc_device_trace_source_deep_validation_enabled()"),
        "Lean auxiliary checks should bind guest PC device trace source selection to the Rust runtime guard"
    );
    assert!(
        lean_source.contains("GuestPcTraceSparseSourceConfig")
            && lean_source.contains("configuredSparseSourceEnabled")
            && lean_source.contains("effectiveSparseSourceSelected")
            && lean_source.contains("defaultSparseSourceMaxPercent")
            && lean_source.contains("configuredSparseSourceMaxPercent")
            && lean_source.contains("traceWordCount")
            && lean_source.contains("nonzeroWordCount")
            && lean_source.contains("maxNonzeroWordCount")
            && lean_source.contains("GuestPcTraceSparseSourceWordLimitMatches")
            && lean_source.contains(
                "config.maxNonzeroWordCount =\n    config.traceWordCount * config.effectiveSparseSourceMaxPercent / 100"
            )
            && lean_source.contains("GuestPcTraceSparseSourceDecisionMatches")
            && lean_source.contains(
                "config.nonzeroWordCount <=\n                  config.maxNonzeroWordCount"
            )
            && lean_source.contains("sparseSourceConfigAccepted")
            && lean_source.contains("sparseSourceConfigImpliesDecisionMatches")
            && lean_source.contains("guest_pc_trace_sparse_source_checked_acceptance_sound")
            && lean_source
                .contains("guest_pc_trace_sparse_source_checked_acceptance_verifier_core_contract")
            && witness_execution_source.contains("fn sparse_trace_source_enabled")
            && witness_execution_source.contains("LZVM_CUDA_SPARSE_TRACE_SOURCE")
            && witness_execution_source.contains("fn sparse_trace_source_max_percent")
            && witness_execution_source.contains("LZVM_CUDA_SPARSE_TRACE_SOURCE_MAX_PERCENT")
            && witness_execution_source.contains("unwrap_or(45)")
            && witness_execution_source.contains("struct WitnessStageSourceUploadConfig")
            && witness_execution_source.contains("struct WitnessTraceCudaRunConfig")
            && witness_execution_source.contains("selected_trace_cuda_run_config")
            && compact_source_contains(
                &witness_execution_source,
                "sparse_trace_source: sparse_trace_source_enabled()",
            )
            && compact_source_contains(
                &witness_execution_source,
                "sparse_trace_source_max_percent: sparse_trace_source_max_percent()",
            )
            && witness_execution_source.contains("upload_config.sparse_trace_source")
            && witness_execution_source
                .contains("trace_words.len().saturating_mul(max_percent) / 100")
            && witness_execution_source.contains("nonzero_count > max_nonzero_words"),
        "Lean auxiliary checks should bind sparse CUDA source selection to the Rust runtime guard"
    );
    assert!(
        lean_source.contains("GuestPcTraceTerminalSparseSourceConfig")
            && lean_source.contains("configuredTerminalSparseSourceEnabled")
            && lean_source.contains("effectiveTerminalSparseSourceSelected")
            && lean_source.contains("terminalTraceSourcePrefixRows")
            && lean_source.contains("terminalTraceLayoutRows")
            && lean_source.contains("GuestPcTraceTerminalSparseSourceDecisionMatches")
            && lean_source.contains("terminalSparseSourceConfigAccepted")
            && lean_source.contains("terminalSparseSourceConfigImpliesDecisionMatches")
            && lean_source
                .contains("guest_pc_trace_terminal_sparse_source_checked_acceptance_sound")
            && lean_source.contains(
                "guest_pc_trace_terminal_sparse_source_checked_acceptance_verifier_core_contract"
            )
            && witness_execution_source.contains("fn terminal_sparse_trace_source_enabled")
            && witness_execution_source.contains("LZVM_CUDA_TERMINAL_SPARSE_TRACE_SOURCE")
            && witness_execution_source.contains("unwrap_or(false)")
            && witness_execution_source.contains("terminal_sparse_trace_source_enabled()")
            && compact_source_contains(
                &witness_execution_source,
                "terminal_sparse_trace_source: terminal_sparse_trace_source_enabled()",
            )
            && witness_execution_source.contains("upload_config.terminal_sparse_trace_source")
            && witness_execution_source.contains("if prefix_rows < layout.row_count()")
            && witness_execution_source
                .contains("upload_from_trace_prefix_and_terminal_fill_if_empty"),
        "Lean auxiliary checks should bind terminal sparse CUDA source selection to the Rust runtime guard"
    );
    assert!(
        lean_source.contains("FriRetainedStageSourceConfig")
            && lean_source.contains("configuredRetainedStageSourceEnabled")
            && lean_source.contains("effectiveRetainedStageSourceEnabled")
            && lean_source.contains("FriRetainedStageSourceDecisionMatches")
            && lean_source.contains("retainedStageSourceConfigAccepted")
            && lean_source.contains("retainedStageSourceConfigImpliesDecisionMatches")
            && lean_source.contains("fri_retained_stage_source_checked_acceptance_sound")
            && lean_source
                .contains("fri_retained_stage_source_checked_acceptance_verifier_core_contract")
            && witness_execution_source.contains("fn retain_fri_stage_source_devices")
            && witness_execution_source.contains("LZVM_CUDA_RETAIN_FRI_STAGE_SOURCES")
            && witness_execution_source.contains("Ok(\"0\") | Ok(\"false\") | Ok(\"no\")")
            && witness_execution_source.contains("stage_source_retention: bool")
            && witness_execution_source
                .contains("let stage_source_retention = retain_fri_stage_source_devices();")
            && witness_execution_source.contains("selected_trace_cuda_run_config")
            && witness_execution_source.contains("stage_source_retention_debug: bool")
            && compact_source_contains(
                &witness_execution_source,
                "trace_cuda_run_config: Some(trace_cuda_run_config)",
            )
            && compact_source_contains(
                &witness_execution_source,
                "trace_cuda_run_config.stage_source_retention_debug",
            )
            && witness_execution_source.contains("let retained_stage_source_devices = if retain_stage_sources")
            && witness_execution_source.contains("stage_source_device_cache.retained_descriptors")
            && witness_execution_source.contains("Vec::new()"),
        "Lean auxiliary checks should bind retained stage source selection to the Rust runtime guard"
    );
    assert!(
        lean_source.contains("GuestPcTraceSparseSourceDebugConfig")
            && lean_source.contains("configuredSparseSourceDebug")
            && lean_source.contains("effectiveSparseSourceDebug")
            && lean_source.contains("GuestPcTraceSparseSourceDebugDecisionMatches")
            && compact_source_contains(
                &lean_source,
                "config.effectiveSparseSourceDebug = false",
            )
            && lean_source.contains("FriRetainedStageSourceDebugConfig")
            && lean_source.contains("configuredRetainedStageSourceDebug")
            && lean_source.contains("selectedRetainedStageSource")
            && lean_source.contains("effectiveRetainedStageSourceDebug")
            && lean_source.contains("FriRetainedStageSourceDebugDecisionMatches")
            && compact_source_contains(
                &lean_source,
                "config.effectiveRetainedStageSourceDebug = (config.selectedRetainedStageSource && configured)",
            )
            && compact_source_contains(
                &lean_source,
                "config.effectiveRetainedStageSourceDebug = false",
            )
            && lean_source.contains("GuestPcTraceCudaRunConfig")
            && lean_source.contains("sparseSourceConfig")
            && lean_source.contains("selectedSparseSource")
            && lean_source.contains("sparseSourceDebugConfig")
            && lean_source.contains("selectedSparseSourceDebug")
            && lean_source.contains("terminalSparseSourceConfig")
            && lean_source.contains("selectedTerminalSparseSource")
            && lean_source.contains("retainedStageSourceConfig")
            && lean_source.contains("selectedRetainedStageSource")
            && lean_source.contains("retainedStageSourceDebugConfig")
            && lean_source.contains("selectedRetainedStageSourceDebug")
            && lean_source.contains("descriptorBufferRetentionConfig")
            && lean_source.contains("selectedDescriptorBufferRetention")
            && lean_source.contains("GuestPcTraceCudaRunDecisionEvidence")
            && lean_source.contains("sparseSourceDecision")
            && lean_source.contains("sparseSourceSelected")
            && lean_source.contains("sparseSourceDebugDecision")
            && lean_source.contains("sparseSourceDebugSelected")
            && lean_source.contains("terminalSparseSourceDecision")
            && lean_source.contains("terminalSparseSourceSelected")
            && lean_source.contains("retainedStageSourceDecision")
            && lean_source.contains("retainedStageSourceSelected")
            && lean_source.contains("retainedStageSourceDebugUsesSelectedSource")
            && lean_source.contains("retainedStageSourceDebugDecision")
            && lean_source.contains("retainedStageSourceDebugSelected")
            && lean_source.contains("descriptorBufferRetentionDecision")
            && lean_source.contains("descriptorBufferRetentionSelected")
            && lean_source.contains("GuestPcTraceCudaRunDecisionMatches")
            && lean_source.contains("traceCudaRunConfigAccepted")
            && lean_source.contains("traceCudaRunConfigImpliesDecisionMatches")
            && lean_source.contains("GuestPcTraceCudaRunCheckedAcceptance")
            && lean_source.contains("guest_pc_trace_cuda_run_sparse_source_matches")
            && lean_source.contains("guest_pc_trace_cuda_run_sparse_source_debug_matches")
            && lean_source.contains("guest_pc_trace_cuda_run_terminal_sparse_source_matches")
            && lean_source.contains("guest_pc_trace_cuda_run_retained_stage_source_matches")
            && lean_source.contains(
                "guest_pc_trace_cuda_run_retained_stage_source_debug_uses_selected_source",
            )
            && lean_source.contains(
                "guest_pc_trace_cuda_run_retained_stage_source_debug_decision_matches",
            )
            && lean_source.contains("guest_pc_trace_cuda_run_retained_stage_source_debug_matches")
            && lean_source.contains("fri_retained_stage_source_debug_requires_retention")
            && lean_source.contains(
                "guest_pc_trace_cuda_run_retained_stage_source_debug_requires_retention",
            )
            && lean_source.contains("guest_pc_trace_cuda_run_descriptor_retention_matches")
            && lean_source.contains(
                "guest_pc_trace_cuda_run_checked_acceptance_projects_sparse_source",
            )
            && lean_source.contains(
                "guest_pc_trace_cuda_run_checked_acceptance_projects_sparse_source_debug",
            )
            && lean_source.contains(
                "guest_pc_trace_cuda_run_checked_acceptance_projects_terminal_sparse_source",
            )
            && lean_source.contains(
                "guest_pc_trace_cuda_run_checked_acceptance_projects_retained_stage_source",
            )
            && lean_source.contains(
                "guest_pc_trace_cuda_run_checked_acceptance_projects_retained_source_debug",
            )
            && lean_source.contains(
                "guest_pc_trace_cuda_run_checked_acceptance_projects_retained_debug_requires_retention",
            )
            && lean_source.contains(
                "guest_pc_trace_cuda_run_checked_acceptance_projects_descriptor_retention",
            )
            && lean_source.contains("guest_pc_trace_cuda_run_checked_acceptance_sound")
            && lean_source
                .contains("guest_pc_trace_cuda_run_checked_acceptance_verifier_core_contract")
            && witness_execution_source.contains("struct WitnessTraceCudaRunConfig")
            && witness_execution_source.contains("selected_trace_cuda_run_config")
            && witness_execution_source.contains(
                "trace_cuda_run_config: Option<WitnessTraceCudaRunConfig>",
            )
            && compact_source_contains(
                &witness_execution_source,
                "trace_cuda_run_config: Some(trace_cuda_run_config)",
            )
            && witness_execution_source
                .contains("stage_source_upload: WitnessStageSourceUploadConfig")
            && witness_execution_source.contains("debug_sparse_trace_source: bool")
            && witness_execution_source.contains("fn debug_sparse_trace_source_enabled")
            && witness_execution_source.contains("LZVM_CUDA_SPARSE_TRACE_SOURCE_DEBUG")
            && compact_source_contains(
                &witness_execution_source,
                "debug_sparse_trace_source: debug_sparse_trace_source_enabled()",
            )
            && witness_execution_source.contains("stage_source_retention: bool")
            && witness_execution_source.contains("stage_source_retention_debug: bool")
            && witness_execution_source.contains("fn debug_fri_stage_source_devices")
            && witness_execution_source.contains("LZVM_CUDA_FRI_STAGE_SOURCE_DEBUG")
            && compact_source_contains(
                &witness_execution_source,
                "stage_source_retention_debug: stage_source_retention && debug_fri_stage_source_devices()",
            )
            && witness_execution_source.contains("descriptor_buffer_retention: bool"),
        "Lean auxiliary checks should bind the grouped CUDA trace runtime config to the Rust runtime guard"
    );
    for (theorem_name, projector) in [
        (
            "guest_pc_trace_cuda_run_checked_acceptance_projects_sparse_source",
            "guest_pc_trace_cuda_run_sparse_source_matches",
        ),
        (
            "guest_pc_trace_cuda_run_checked_acceptance_projects_sparse_source_debug",
            "guest_pc_trace_cuda_run_sparse_source_debug_matches",
        ),
        (
            "guest_pc_trace_cuda_run_checked_acceptance_projects_terminal_sparse_source",
            "guest_pc_trace_cuda_run_terminal_sparse_source_matches",
        ),
        (
            "guest_pc_trace_cuda_run_checked_acceptance_projects_retained_stage_source",
            "guest_pc_trace_cuda_run_retained_stage_source_matches",
        ),
        (
            "guest_pc_trace_cuda_run_checked_acceptance_projects_retained_source_debug",
            "guest_pc_trace_cuda_run_retained_stage_source_debug_matches",
        ),
        (
            "guest_pc_trace_cuda_run_checked_acceptance_projects_descriptor_retention",
            "guest_pc_trace_cuda_run_descriptor_retention_matches",
        ),
    ] {
        lean_binding::assert_theorem_body_contains(
            &gpu_runtime_source,
            theorem_name,
            &[
                projector,
                "guest_pc_trace_cuda_run_checked_acceptance_projects_decision",
            ],
        );
    }
    for (theorem_name, field) in [
        (
            "guest_pc_trace_cuda_run_sparse_source_matches",
            "decision.sparseSourceSelected",
        ),
        (
            "guest_pc_trace_cuda_run_sparse_source_debug_matches",
            "decision.sparseSourceDebugSelected",
        ),
        (
            "guest_pc_trace_cuda_run_terminal_sparse_source_matches",
            "decision.terminalSparseSourceSelected",
        ),
        (
            "guest_pc_trace_cuda_run_retained_stage_source_matches",
            "decision.retainedStageSourceSelected",
        ),
        (
            "guest_pc_trace_cuda_run_retained_stage_source_debug_uses_selected_source",
            "decision.retainedStageSourceDebugUsesSelectedSource",
        ),
        (
            "guest_pc_trace_cuda_run_retained_stage_source_debug_decision_matches",
            "decision.retainedStageSourceDebugDecision",
        ),
        (
            "guest_pc_trace_cuda_run_retained_stage_source_debug_matches",
            "decision.retainedStageSourceDebugSelected",
        ),
        (
            "guest_pc_trace_cuda_run_descriptor_retention_matches",
            "decision.descriptorBufferRetentionSelected",
        ),
    ] {
        lean_binding::assert_theorem_body_contains(&gpu_runtime_source, theorem_name, &[field]);
        lean_binding::assert_theorem_body_omits(
            &gpu_runtime_source,
            theorem_name,
            &["rcases decision"],
        );
    }
    lean_binding::assert_theorem_body_contains(
        &gpu_runtime_source,
        "guest_pc_trace_cuda_run_retained_stage_source_debug_requires_retention",
        &[
            "fri_retained_stage_source_debug_requires_retention",
            "guest_pc_trace_cuda_run_retained_stage_source_debug_decision_matches",
            "guest_pc_trace_cuda_run_retained_stage_source_debug_matches",
            "guest_pc_trace_cuda_run_retained_stage_source_debug_uses_selected_source",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &gpu_runtime_source,
        "guest_pc_trace_cuda_run_checked_acceptance_projects_retained_debug_requires_retention",
        &[
            "guest_pc_trace_cuda_run_retained_stage_source_debug_requires_retention",
            "guest_pc_trace_cuda_run_checked_acceptance_projects_decision",
        ],
    );
    assert!(
        lean_source.contains("GpuRetainedDeviceCacheBudget")
            && lean_source.contains("sourceBytes")
            && lean_source.contains("descriptorBytes")
            && lean_source.contains("leafDigestBytes")
            && lean_source.contains("sourceLimit")
            && lean_source.contains("descriptorLimit")
            && lean_source.contains("leafDigestLimit")
            && lean_source.contains("combinedLimit")
            && lean_source.contains("GpuRetainedDeviceCacheBudgetWithinLimits")
            && lean_source.contains("retainedDeviceCacheBudgetAccepted")
            && lean_source.contains("retainedDeviceCacheBudgetImpliesWithinLimits")
            && witness_values_source.contains("retained_combined_device_cache_allows")
            && witness_values_source.contains("reserve_retained_device_bytes")
            && witness_values_source.contains("reserve_retained_descriptor_buffer_bytes")
            && witness_values_source.contains("reserve_retained_leaf_digest_bytes"),
        "Lean auxiliary checks should bind retained source, descriptor, and leaf digest cache retention to runtime budget limits"
    );
    assert!(
        lean_source.contains("defaultLeafDigestLimitBytes")
            && lean_source.contains("configuredLeafDigestLimitBytes")
            && lean_source.contains("effectiveLeafDigestLimitBytes")
            && lean_source.contains("GpuRetainedLeafDigestLimitDecisionMatches")
            && lean_source.contains("retainedLeafDigestLimitConfigAccepted")
            && lean_source.contains("retainedLeafDigestLimitConfigImpliesDecisionMatches")
            && witness_values_source.contains("DEFAULT_RETAINED_LEAF_DIGEST_BYTES")
            && witness_values_source.contains("LZVM_CUDA_RETAINED_LEAF_DIGEST_BYTES")
            && witness_values_source.contains("retained_combined_device_cache_allows"),
        "Lean auxiliary checks should bind retained leaf digest runtime limit selection to the Rust cache cap"
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
        "leafOutputCacheHitCount",
        "leafOutputCacheMissCount",
        "leafSetupWorkspaceAllocCount",
        "leafUploadWorkMilliseconds",
        "leafKernelWorkMilliseconds",
        "leafDownloadWorkMilliseconds",
        "leafValidateWorkMilliseconds",
        "leafHashWorkMilliseconds",
        "leafHashRowCount",
        "leafHashByteCount",
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
        "deviceDownloadBatchCount",
        "deviceSingleDownloadCount",
        "rowValueSourceExtendMilliseconds",
        "rowValueSourceDownloadMilliseconds",
        "rowValueDeviceDownloadMilliseconds",
        "deviceRowCount",
        "guestTraceStreamElapsedMilliseconds",
        "guestTraceProofValuePrerunMilliseconds",
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
        "guestTraceReportBufferCapacity",
        "guestTraceReportBufferMaxCapacity",
        "guestTraceReportBufferExcessCapacity",
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
        "guestTraceDescriptorCompactRowCount",
        "guestTraceDescriptorWideRowCount",
        "guestTracePendingSendWaitMilliseconds",
        "guestTracePendingReceiveWaitMilliseconds",
        "guestTraceSegmentSendWaitMilliseconds",
        "guestTraceSegmentReceiveWaitMilliseconds",
        "guestTraceParallelLowerWorkerCount",
        "guestTraceParallelLowerDispatchedCount",
        "guestTraceParallelLowerReceivedCount",
        "guestTraceParallelLowerEmittedCount",
        "guestTraceParallelLowerMaxReorderCount",
        "guestTraceOwnedStreamingLowerSegmentCount",
        "guestTraceParallelLowerStreamStartDispatchWaitMilliseconds",
        "guestTraceParallelLowerStreamChunkDispatchWaitMilliseconds",
        "guestTraceParallelLowerStreamSegmentDispatchWaitMilliseconds",
        "guestTraceParallelLowerStreamFinishDispatchWaitMilliseconds",
        "guestSegmentCommitInitialWorkerCount",
        "guestSegmentCommitEffectiveWorkerCount",
        "guestSegmentCommitOomRetryCount",
        "guestSegmentCommitCudaMemoryTotalByteCount",
        "guestSegmentCommitCudaMemoryInitialFreeByteCount",
        "guestSegmentCommitCudaMemoryEffectiveFreeByteCount",
        "guestSegmentCommitCudaMemoryMinFreeByteCount",
        "guestSegmentCommitCudaAllocatorInitialCachedByteCount",
        "guestSegmentCommitCudaAllocatorEffectiveCachedByteCount",
        "guestDeviceSourceBuildMilliseconds",
        "guestDeviceSourceDescriptorUploadMilliseconds",
        "guestDeviceSourceDescriptorUploadByteCount",
        "guestDeviceSourceDescriptorUploadWordCount",
        "guestDeviceSourceDescriptorUploadRowCount",
        "guestDeviceSourceTraceExpandMilliseconds",
        "guestStageSourceRetentionAttemptCount",
        "guestStageSourceRetentionRetainedCount",
        "guestStageSourceRetentionRejectedCount",
        "guestStageSourceRetentionRetainedByteCount",
        "guestStageSourceRetentionRejectedByteCount",
        "guestStageSourceRetentionLimitByteCount",
        "guestDescriptorBufferRetentionAttemptCount",
        "guestDescriptorBufferRetentionRetainedCount",
        "guestDescriptorBufferRetentionRejectedCount",
        "guestDescriptorBufferRetentionRetainedByteCount",
        "guestDescriptorBufferRetentionRejectedByteCount",
        "guestDescriptorBufferRetentionLimitByteCount",
        "guestStageLeafExtendWorkMilliseconds",
        "guestStageLeafSetupWorkMilliseconds",
        "guestStageLeafSetupPrepareMilliseconds",
        "guestStageLeafSetupOutputAllocMilliseconds",
        "guestStageLeafSetupWorkspaceAllocMilliseconds",
        "guestStageLeafSetupOutputAllocByteCount",
        "guestStageLeafSetupWorkspaceAllocByteCount",
        "guestStageLeafSetupOutputAllocCount",
        "guestStageLeafOutputCacheHitCount",
        "guestStageLeafOutputCacheMissCount",
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
        "guestStageTreeCommitRootMaterializationGroupCount",
        "guestStageTreeCommitRootMaterializationMaxGroupSize",
        "guestStageTreeCommitRetainWorkMilliseconds",
    ] {
        assert!(
            lean_source.contains(field),
            "Lean guest PC trace timing summary should expose {field}"
        );
    }
    for (line_name, accessor) in [
        (
            "\"guest_trace_stream_elapsed\"",
            "guest_trace_stream_elapsed_duration()",
        ),
        (
            "\"guest_trace_proof_value_prerun\"",
            "guest_trace_proof_value_prerun_duration()",
        ),
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
            "\"guest_trace_report_memory_columns\"",
            "guest_trace_report_memory_columns_duration()",
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
        (
            "\"guest_trace_external_op_row_lower\"",
            "guest_trace_external_op_row_duration()",
        ),
        (
            "\"guest_trace_copy_row_lower\"",
            "guest_trace_copy_row_duration()",
        ),
        ("\"guest_trace_reports\"", "guest_trace_report_count()"),
        (
            "\"guest_trace_report_rows\"",
            "guest_trace_report_row_count()",
        ),
        (
            "\"guest_trace_report_buffer_capacity\"",
            "guest_trace_report_buffer_capacity()",
        ),
        (
            "\"guest_trace_report_buffer_max_capacity\"",
            "guest_trace_report_buffer_max_capacity()",
        ),
        (
            "\"guest_trace_report_buffer_excess_capacity\"",
            "guest_trace_report_buffer_excess_capacity()",
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
        (
            "\"guest_trace_row_shape_top_1_pattern\"",
            "guest_trace_row_shape_top_patterns()",
        ),
        (
            "\"guest_trace_row_shape_top_1_count\"",
            "guest_trace_row_shape_top_patterns()",
        ),
        (
            "\"guest_trace_row_shape_top_2_pattern\"",
            "guest_trace_row_shape_top_patterns()",
        ),
        (
            "\"guest_trace_row_shape_top_2_count\"",
            "guest_trace_row_shape_top_patterns()",
        ),
        (
            "\"guest_trace_row_shape_top_3_pattern\"",
            "guest_trace_row_shape_top_patterns()",
        ),
        (
            "\"guest_trace_row_shape_top_3_count\"",
            "guest_trace_row_shape_top_patterns()",
        ),
        (
            "\"guest_trace_row_shape_top_4_pattern\"",
            "guest_trace_row_shape_top_patterns()",
        ),
        (
            "\"guest_trace_row_shape_top_4_count\"",
            "guest_trace_row_shape_top_patterns()",
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
            "\"guest_trace_descriptor_compact_rows\"",
            "guest_trace_descriptor_compact_row_count()",
        ),
        (
            "\"guest_trace_descriptor_wide_rows\"",
            "guest_trace_descriptor_wide_row_count()",
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
            "\"guest_trace_parallel_lower_workers\"",
            "guest_trace_parallel_lower_worker_count()",
        ),
        (
            "\"guest_trace_parallel_lower_dispatched\"",
            "guest_trace_parallel_lower_dispatched_count()",
        ),
        (
            "\"guest_trace_parallel_lower_received\"",
            "guest_trace_parallel_lower_received_count()",
        ),
        (
            "\"guest_trace_parallel_lower_emitted\"",
            "guest_trace_parallel_lower_emitted_count()",
        ),
        (
            "\"guest_trace_parallel_lower_max_reorder\"",
            "guest_trace_parallel_lower_max_reorder_count()",
        ),
        (
            "\"guest_trace_parallel_lower_snapshot_replay_count\"",
            "guest_trace_parallel_lower_snapshot_replay_count()",
        ),
        (
            "\"guest_trace_parallel_lower_snapshot_replay\"",
            "guest_trace_parallel_lower_snapshot_replay_duration()",
        ),
        (
            "\"guest_trace_parallel_lower_report_elided_count\"",
            "guest_trace_parallel_lower_report_elided_count()",
        ),
        (
            "\"guest_trace_owned_streaming_lower_segments\"",
            "guest_trace_owned_streaming_lower_segment_count()",
        ),
        (
            "\"guest_trace_parallel_lower_dispatch_wait\"",
            "guest_trace_parallel_lower_dispatch_wait_duration()",
        ),
        (
            "\"guest_trace_parallel_lower_stream_start_dispatch_wait\"",
            "guest_trace_parallel_lower_stream_start_dispatch_wait_duration()",
        ),
        (
            "\"guest_trace_parallel_lower_stream_chunk_dispatch_wait\"",
            "guest_trace_parallel_lower_stream_chunk_dispatch_wait_duration()",
        ),
        (
            "\"guest_trace_parallel_lower_stream_segment_dispatch_wait\"",
            "guest_trace_parallel_lower_stream_segment_dispatch_wait_duration()",
        ),
        (
            "\"guest_trace_parallel_lower_stream_finish_dispatch_wait\"",
            "guest_trace_parallel_lower_stream_finish_dispatch_wait_duration()",
        ),
        (
            "\"guest_trace_parallel_lower_dispatch_blocked_count\"",
            "guest_trace_parallel_lower_dispatch_blocked_count()",
        ),
        (
            "\"guest_segment_commit_initial_workers\"",
            "guest_segment_commit_initial_worker_count()",
        ),
        (
            "\"guest_segment_commit_effective_workers\"",
            "guest_segment_commit_effective_worker_count()",
        ),
        (
            "\"guest_segment_commit_oom_retries\"",
            "guest_segment_commit_oom_retry_count()",
        ),
        (
            "\"guest_segment_commit_attempt\"",
            "guest_segment_commit_attempt_duration()",
        ),
        (
            "\"guest_segment_commit_oom_retry\"",
            "guest_segment_commit_oom_retry_duration()",
        ),
        (
            "\"guest_segment_commit_cuda_memory_total_bytes\"",
            "guest_segment_commit_cuda_memory_total_byte_count()",
        ),
        (
            "\"guest_segment_commit_cuda_memory_initial_free_bytes\"",
            "guest_segment_commit_cuda_memory_initial_free_byte_count()",
        ),
        (
            "\"guest_segment_commit_cuda_memory_effective_free_bytes\"",
            "guest_segment_commit_cuda_memory_effective_free_byte_count()",
        ),
        (
            "\"guest_segment_commit_cuda_memory_min_free_bytes\"",
            "guest_segment_commit_cuda_memory_min_free_byte_count()",
        ),
        (
            "\"guest_segment_commit_cuda_allocator_initial_cached_bytes\"",
            "guest_segment_commit_cuda_allocator_initial_cached_byte_count()",
        ),
        (
            "\"guest_segment_commit_cuda_allocator_effective_cached_bytes\"",
            "guest_segment_commit_cuda_allocator_effective_cached_byte_count()",
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
            "\"guest_device_source_descriptor_upload_words\"",
            "guest_device_source_descriptor_upload_word_count()",
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
            "\"guest_stage_leaf_output_cache_hits\"",
            "guest_stage_leaf_output_cache_hit_count()",
        ),
        (
            "\"guest_stage_leaf_output_cache_misses\"",
            "guest_stage_leaf_output_cache_miss_count()",
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
            "\"guest_stage_source_retention_retained_bytes\"",
            "guest_stage_source_retention_retained_byte_count()",
        ),
        (
            "\"guest_stage_source_retention_rejected_bytes\"",
            "guest_stage_source_retention_rejected_byte_count()",
        ),
        (
            "\"guest_stage_source_retention_max_retained_bytes\"",
            "guest_stage_source_retention_max_retained_byte_count()",
        ),
        (
            "\"guest_stage_source_retention_max_rejected_bytes\"",
            "guest_stage_source_retention_max_rejected_byte_count()",
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
            "\"guest_descriptor_buffer_retention_limit_bytes\"",
            "guest_descriptor_buffer_retention_limit_byte_count()",
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
            "\"guest_stage_tree_commit_root_materialization_groups\"",
            "guest_stage_tree_commit_root_materialization_group_count()",
        ),
        (
            "\"guest_stage_tree_commit_root_materialization_max_group_size\"",
            "guest_stage_tree_commit_root_materialization_max_group_size()",
        ),
        (
            "\"guest_stage_tree_commit_retain_work\"",
            "guest_stage_tree_commit_retain_work_duration()",
        ),
    ] {
        assert!(
            guest_pc_timing_source_contains(&guest_pc_timing_source, line_name, accessor),
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
            "guest_stage_{stage_index}_leaf_output_cache_hits",
            "leaf_output_cache_hit_count()",
        ),
        (
            "guest_stage_{stage_index}_leaf_output_cache_misses",
            "leaf_output_cache_miss_count()",
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
            "guest_stage_{stage_index}_leaf_hash_rows",
            "leaf_hash_row_count()",
        ),
        (
            "guest_stage_{stage_index}_leaf_hash_bytes",
            "leaf_hash_byte_count()",
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
            constant_material_source.contains(line_name),
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
    for field in [
        "gpuPreallocateRequested",
        "gpuStreamLimit",
        "witnessThreadPoolCount",
        "storedWitnessLimit",
        "packTraceEnabled",
    ] {
        assert!(
            lean_source.contains(field),
            "Lean GPU run options summary should expose {field}"
        );
    }
    for line_name in [
        "\"gpu_preallocate={}\"",
        "\"gpu_streams={}\"",
        "\"witness_thread_pools={}\"",
        "\"stored_witnesses={}\"",
        "\"pack_trace={}\"",
    ] {
        assert!(
            prove_plan_source.contains(line_name),
            "CLI run plan summary should include {line_name}"
        );
    }
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
        "cudaAllocatorMallocWaitNanoseconds",
        "cudaAllocatorMallocMaxWaitNanoseconds",
        "cudaAllocatorHostRegisterCallCount",
        "cudaAllocatorHostRegisterByteCount",
        "cudaAllocatorHostRegisterWaitNanoseconds",
        "cudaAllocatorHostRegisterMaxWaitNanoseconds",
        "cudaAllocatorHostUnregisterCallCount",
        "cudaAllocatorHostUnregisterWaitNanoseconds",
        "cudaAllocatorHostUnregisterMaxWaitNanoseconds",
        "cudaAllocatorCopyH2DCallCount",
        "cudaAllocatorCopyH2DByteCount",
        "cudaAllocatorCopyH2DWaitNanoseconds",
        "cudaAllocatorCopyH2DMaxWaitNanoseconds",
        "cudaAllocatorCopyH2DHotByteCount",
        "cudaAllocatorCopyH2DHotCount",
        "cudaAllocatorCopyH2DHotWaitNanoseconds",
        "cudaAllocatorCopyH2DSecondHotByteCount",
        "cudaAllocatorCopyH2DSecondHotCount",
        "cudaAllocatorCopyH2DSecondHotWaitNanoseconds",
        "cudaAllocatorCopyD2HCallCount",
        "cudaAllocatorCopyD2HByteCount",
        "cudaAllocatorCopyD2HWaitNanoseconds",
        "cudaAllocatorCopyD2HMaxWaitNanoseconds",
        "cudaAllocatorCopyD2DCallCount",
        "cudaAllocatorCopyD2DByteCount",
        "cudaAllocatorCopyD2DWaitNanoseconds",
        "cudaAllocatorCopyD2DMaxWaitNanoseconds",
        "cudaAllocatorDeviceSynchronizeCallCount",
        "cudaAllocatorDeviceSynchronizeWaitNanoseconds",
        "cudaAllocatorDeviceSynchronizeMaxWaitNanoseconds",
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
        "gpuRunOptions : Option GpuRunOptionsSummary",
        "cudaBackend : Option CudaBackendSummary",
        "cudaAllocatorTiming : Option CudaAllocatorTimingSummary",
        "proofArtifactFinishTiming : Option ProofArtifactFinishTimingSummary",
        "proofTimingBatch : Option ProofTimingBatchSummary",
    ] {
        assert!(
            lean_source.contains(field),
            "Lean runtime performance observation summary should expose {field}"
        );
    }
    for field in [
        "smallRunCount : Nat",
        "largeRunCount : Nat",
        "smallStableRunCount : Nat",
        "largeStableRunCount : Nat",
        "smallStableAverageMilliseconds : Nat",
        "largeStableAverageMilliseconds : Nat",
        "smallStableSpreadMilliseconds : Nat",
        "largeStableSpreadMilliseconds : Nat",
        "smallTimingParseFailedCount : Nat",
        "largeTimingParseFailedCount : Nat",
    ] {
        assert!(
            lean_source.contains(field),
            "Lean proof timing batch summary should expose {field}"
        );
    }
    lean_binding::assert_theorem_declarations(
        &lean_proof_timing_source,
        &[
            "proof_timing_batch_observed_acceptance_projects_verifier_acceptance",
            "proof_timing_batch_acceptance_sound",
            "proof_timing_batch_acceptance_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_proof_timing_source,
        "proof_timing_batch_acceptance_sound",
        &["ignored_metadata_acceptance_sound"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_proof_timing_source,
        "proof_timing_batch_acceptance_verifier_core_contract",
        &["ignored_metadata_acceptance_verifier_core_contract"],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_proof_timing_source,
        "proof_timing_batch_acceptance_verifier_core_contract",
        &[
            "proof_timing_batch_acceptance_sound",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &runtime_performance_source,
        "runtime_performance_observation_projects_proof_timing_batch",
        &[
            "ProofTimingBatchObservedAcceptance",
            "summary.proofTimingBatch",
        ],
    );
    assert!(
        proof_batch_runner_source
            .contains("from proof_timing_keys import TIMING_SUMMARY_REQUIRED_KEYS")
            && proof_batch_runner_source.contains("key for key in TIMING_SUMMARY_REQUIRED_KEYS"),
        "proof timing batch runner should gate summaries through the shared required-key source"
    );
    for (lean_field, runner_key) in [
        ("smallRunCount", "\"small_run_count\""),
        ("largeRunCount", "\"large_run_count\""),
        ("smallStableRunCount", "\"small_stable_run_count\""),
        ("largeStableRunCount", "\"large_stable_run_count\""),
        ("smallStableAverageMilliseconds", "\"small_stable_avg_ms\""),
        ("largeStableAverageMilliseconds", "\"large_stable_avg_ms\""),
        (
            "smallStableSpreadMilliseconds",
            "\"small_stable_spread_ms\"",
        ),
        (
            "largeStableSpreadMilliseconds",
            "\"large_stable_spread_ms\"",
        ),
        (
            "smallTimingParseFailedCount",
            "\"small_timing_parse_failed_count\"",
        ),
        (
            "largeTimingParseFailedCount",
            "\"large_timing_parse_failed_count\"",
        ),
    ] {
        assert!(
            lean_source.contains(lean_field) && proof_batch_runner_source.contains(runner_key),
            "Lean proof timing batch field {lean_field} should align with runner key {runner_key}"
        );
    }
    for (lean_field, timing_key) in [
        (
            "finishWitnessOpeningRowDedupInputRowCount",
            "\"timing_finish_witness_opening_row_dedup_input_rows\"",
        ),
        (
            "finishWitnessOpeningRowDedupUniqueRowCount",
            "\"timing_finish_witness_opening_row_dedup_unique_rows\"",
        ),
        (
            "finishWitnessOpeningRowDedupElidedRowCount",
            "\"timing_finish_witness_opening_row_dedup_elided_rows\"",
        ),
        (
            "finishFriOpeningMilliseconds",
            "\"timing_finish_fri_opening_ms\"",
        ),
        (
            "finishFriOpeningUnitBuildMilliseconds",
            "\"timing_finish_fri_opening_unit_build_ms\"",
        ),
        (
            "finishFriOpeningLayerTreeMilliseconds",
            "\"timing_finish_fri_opening_layer_tree_ms\"",
        ),
        (
            "finishFriOpeningQueryMilliseconds",
            "\"timing_finish_fri_opening_query_ms\"",
        ),
        (
            "finishFriOpeningFoldMilliseconds",
            "\"timing_finish_fri_opening_fold_ms\"",
        ),
        (
            "finishFriOpeningUnitCount",
            "\"timing_finish_fri_opening_unit_count\"",
        ),
        (
            "finishFriOpeningLayerCount",
            "\"timing_finish_fri_opening_layer_count\"",
        ),
        (
            "finishFriOpeningQueryCount",
            "\"timing_finish_fri_opening_query_count\"",
        ),
        (
            "finishFriTranscriptUnitBuildMilliseconds",
            "\"timing_finish_fri_transcript_unit_build_ms\"",
        ),
        (
            "finishFriTranscriptLayerTreeMilliseconds",
            "\"timing_finish_fri_transcript_layer_tree_ms\"",
        ),
        (
            "finishFriTranscriptFoldMilliseconds",
            "\"timing_finish_fri_transcript_fold_ms\"",
        ),
        (
            "finishFriTranscriptUnitCount",
            "\"timing_finish_fri_transcript_unit_count\"",
        ),
        (
            "finishFriTranscriptLayerCount",
            "\"timing_finish_fri_transcript_layer_count\"",
        ),
        (
            "finishContributionSegmentMilliseconds",
            "\"timing_finish_contribution_segment_ms\"",
        ),
        (
            "finishContributionVerifyMilliseconds",
            "\"timing_finish_contribution_verify_ms\"",
        ),
        (
            "finishContributionChallengeMilliseconds",
            "\"timing_finish_contribution_challenge_ms\"",
        ),
    ] {
        assert!(
            lean_source.contains(lean_field) && proof_timing_keys_source.contains(timing_key),
            "Lean proof finish timing field {lean_field} should align with required key {timing_key}"
        );
    }
    for line_name in [
        "\"cuda_allocator_malloc_calls\"",
        "\"cuda_allocator_malloc_bytes\"",
        "\"cuda_allocator_malloc_wait_ns\"",
        "\"cuda_allocator_malloc_max_wait_ns\"",
        "\"cuda_allocator_host_register_calls\"",
        "\"cuda_allocator_host_register_bytes\"",
        "\"cuda_allocator_host_register_wait_ns\"",
        "\"cuda_allocator_host_register_max_wait_ns\"",
        "\"cuda_allocator_host_unregister_calls\"",
        "\"cuda_allocator_host_unregister_wait_ns\"",
        "\"cuda_allocator_host_unregister_max_wait_ns\"",
        "\"cuda_allocator_copy_h2d_calls\"",
        "\"cuda_allocator_copy_h2d_bytes\"",
        "\"cuda_allocator_copy_h2d_wait_ns\"",
        "\"cuda_allocator_copy_h2d_max_wait_ns\"",
        "\"cuda_allocator_copy_h2d_hot_bytes\"",
        "\"cuda_allocator_copy_h2d_hot_count\"",
        "\"cuda_allocator_copy_h2d_hot_wait_ns\"",
        "\"cuda_allocator_copy_h2d_hot_avg_wait_per_call_ns\"",
        "\"cuda_allocator_copy_h2d_second_hot_bytes\"",
        "\"cuda_allocator_copy_h2d_second_hot_count\"",
        "\"cuda_allocator_copy_h2d_second_hot_wait_ns\"",
        "\"cuda_allocator_copy_h2d_second_hot_avg_wait_per_call_ns\"",
        "\"cuda_allocator_copy_d2h_calls\"",
        "\"cuda_allocator_copy_d2h_bytes\"",
        "\"cuda_allocator_copy_d2h_wait_ns\"",
        "\"cuda_allocator_copy_d2h_max_wait_ns\"",
        "\"cuda_allocator_copy_d2d_calls\"",
        "\"cuda_allocator_copy_d2d_bytes\"",
        "\"cuda_allocator_copy_d2d_wait_ns\"",
        "\"cuda_allocator_copy_d2d_max_wait_ns\"",
        "\"cuda_allocator_device_synchronize_calls\"",
        "\"cuda_allocator_device_synchronize_wait_ns\"",
        "\"cuda_allocator_device_synchronize_max_wait_ns\"",
        "\"cuda_allocator_device_synchronize_avg_wait_per_call_ns\"",
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
    for field in [
        "finishQueryPlanMilliseconds",
        "finishConstantOpeningMilliseconds",
        "finishWitnessOpeningMilliseconds",
        "finishWitnessOpeningQueryCount",
        "finishWitnessOpeningQueryUnitCount",
        "finishWitnessOpeningSingleQueryUnitCount",
        "finishWitnessOpeningMaxQueriesPerUnit",
        "finishWitnessOpeningStageCount",
        "finishWitnessOpeningRetainedSourceCount",
        "finishWitnessOpeningExternalSourceCount",
        "finishWitnessOpeningEmbeddedSourceCount",
        "finishWitnessOpeningMissingSourceCount",
        "finishWitnessOpeningRetainedLeafDigestOpeningCount",
        "finishWitnessOpeningRetainedLeafDigestOpeningRowCount",
        "finishWitnessOpeningRetainedParentCheckpointOpeningCount",
        "finishWitnessOpeningRetainedParentCheckpointOpeningRowCount",
        "finishWitnessOpeningRowDedupInputRowCount",
        "finishWitnessOpeningRowDedupUniqueRowCount",
        "finishWitnessOpeningRowDedupElidedRowCount",
        "finishWitnessExternalSourceMilliseconds",
        "finishWitnessExternalSourceDescriptorUploadMilliseconds",
        "finishWitnessExternalSourceDescriptorUploadByteCount",
        "finishWitnessExternalSourceDescriptorUploadWordCount",
        "finishWitnessExternalSourceDescriptorUploadRowCount",
        "finishWitnessExternalSourceTraceExpandMilliseconds",
        "finishWitnessOpeningSetupMilliseconds",
        "finishWitnessOpeningLeafExtendMilliseconds",
        "finishWitnessOpeningLeafHashMilliseconds",
        "finishWitnessOpeningLeafHashRowCount",
        "finishWitnessOpeningLeafHashByteCount",
        "finishWitnessOpeningLeafHashArity2RowCount",
        "finishWitnessOpeningLeafHashArity2ByteCount",
        "finishWitnessOpeningLeafHashArity4RowCount",
        "finishWitnessOpeningLeafHashArity4ByteCount",
        "finishWitnessOpeningLeafCosetExtendCallCount",
        "finishWitnessOpeningLeafCosetExtendOutputByteCount",
        "finishWitnessOpeningLeafCosetExtendColumnCount",
        "finishWitnessOpeningLeafCosetExtendMaxColumnCount",
        "finishWitnessOpeningLeafCosetExtendNttLaunchCount",
        "finishWitnessOpeningLeafCosetExtendBitReverseLaunchCount",
        "finishWitnessOpeningLeafCosetExtendNttStageLaunchCount",
        "finishWitnessOpeningLeafCosetExtendNttBlockTwiddleLaunchCount",
        "finishWitnessOpeningLeafCosetExtendNormalizeLaunchCount",
        "finishWitnessOpeningLeafCosetExtendPackLaunchCount",
        "finishWitnessOpeningLeafCosetExtendUnpackLaunchCount",
        "finishWitnessOpeningPathParentHashRowCount",
        "finishWitnessOpeningPathParentHashByteCount",
        "finishWitnessOpeningPathParentHashLaunchCount",
        "finishWitnessOpeningPathParentHashRecomputedRowCount",
        "finishWitnessOpeningPathParentHashRecomputedByteCount",
        "finishWitnessOpeningPathParentHashRecomputedLaunchCount",
        "finishWitnessOpeningPathParentHashRetainedLeafDigestRowCount",
        "finishWitnessOpeningPathParentHashRetainedLeafDigestByteCount",
        "finishWitnessOpeningPathParentHashRetainedLeafDigestLaunchCount",
        "finishWitnessOpeningPathParentHashRetainedParentCheckpointPrefixRowCount",
        "finishWitnessOpeningPathParentHashRetainedParentCheckpointPrefixByteCount",
        "finishWitnessOpeningPathParentHashRetainedParentCheckpointPrefixLaunchCount",
        "finishWitnessOpeningPathParentHashRetainedParentCheckpointSuffixRowCount",
        "finishWitnessOpeningPathParentHashRetainedParentCheckpointSuffixByteCount",
        "finishWitnessOpeningPathParentHashRetainedParentCheckpointSuffixLaunchCount",
        "finishWitnessOpeningPathParentHashRowsPerQuery",
        "finishWitnessOpeningPathParentHashRowsPerStage",
        "finishWitnessOpeningPathParentHashLaunchesPerStage",
        "finishWitnessOpeningRowValuesMilliseconds",
        "finishWitnessOpeningRowValueSourceExtendMilliseconds",
        "finishWitnessOpeningRowValueSourceDownloadMilliseconds",
        "finishWitnessOpeningRowValueDeviceDownloadMilliseconds",
        "finishWitnessOpeningRowValuesDeviceRowCount",
        "finishWitnessOpeningRowValuesDeviceDownloadBatchCount",
        "finishWitnessOpeningRowValuesDeviceSingleDownloadCount",
        "finishWitnessOpeningRowValuesSourceRowCount",
        "finishWitnessOpeningRowValuesWordCount",
        "finishWitnessOpeningRowValuesByteCount",
        "finishWitnessOpeningPathMilliseconds",
        "finishFriOpeningMilliseconds",
        "finishFriOpeningUnitBuildMilliseconds",
        "finishFriOpeningLayerTreeMilliseconds",
        "finishFriOpeningQueryMilliseconds",
        "finishFriOpeningFoldMilliseconds",
        "finishFriOpeningUnitCount",
        "finishFriOpeningLayerCount",
        "finishFriOpeningQueryCount",
        "finishFriTranscriptUnitBuildMilliseconds",
        "finishFriTranscriptLayerTreeMilliseconds",
        "finishFriTranscriptFoldMilliseconds",
        "finishFriTranscriptUnitCount",
        "finishFriTranscriptLayerCount",
        "finishProofEncodeMilliseconds",
        "finishContributionSegmentMilliseconds",
        "finishContributionVerifyMilliseconds",
        "finishContributionChallengeMilliseconds",
    ] {
        assert!(
            lean_source.contains(field),
            "Lean proof artifact finish timing summary should expose {field}"
        );
    }
    for line_name in [
        "\"finish_query_plan\"",
        "\"finish_constant_opening\"",
        "\"finish_witness_opening\"",
        "\"finish_fri_opening\"",
        "\"finish_fri_transcript_unit_build\"",
        "\"finish_fri_transcript_layer_tree\"",
        "\"finish_fri_transcript_fold\"",
        "\"finish_fri_transcript_unit_count\"",
        "\"finish_fri_transcript_layer_count\"",
        "\"finish_contribution_segment\"",
        "\"finish_contribution_verify\"",
        "\"finish_contribution_challenge\"",
    ] {
        assert!(
            proof_timing_source.contains(line_name),
            "CLI proof artifact timing output should include {line_name}"
        );
    }
    assert!(
        prove_witness_source.contains("\"finish_proof_encode\""),
        "CLI prove witness output should include proof encode timing"
    );
    for (line_name, field) in [
        (
            "\"finish_witness_opening_query_count\"",
            "witness_opening_query_count",
        ),
        (
            "\"finish_witness_opening_query_unit_count\"",
            "witness_opening_query_unit_count",
        ),
        (
            "\"finish_witness_opening_single_query_unit_count\"",
            "witness_opening_single_query_unit_count",
        ),
        (
            "\"finish_witness_opening_max_queries_per_unit\"",
            "witness_opening_max_queries_per_unit",
        ),
        (
            "\"finish_witness_opening_stage_count\"",
            "witness_opening_stage_count",
        ),
        (
            "\"finish_witness_opening_retained_source_count\"",
            "witness_opening_retained_source_count",
        ),
        (
            "\"finish_witness_opening_external_source_count\"",
            "witness_opening_external_source_count",
        ),
        (
            "\"finish_witness_opening_embedded_source_count\"",
            "witness_opening_embedded_source_count",
        ),
        (
            "\"finish_witness_opening_missing_source_count\"",
            "witness_opening_missing_source_count",
        ),
        (
            "\"finish_witness_opening_retained_leaf_digest_openings\"",
            "witness_opening_retained_leaf_digest_opening_count",
        ),
        (
            "\"finish_witness_opening_retained_leaf_digest_rows\"",
            "witness_opening_retained_leaf_digest_opening_row_count",
        ),
        (
            "\"finish_witness_opening_retained_parent_checkpoint_openings\"",
            "witness_opening_retained_parent_checkpoint_opening_count",
        ),
        (
            "\"finish_witness_opening_retained_parent_checkpoint_rows\"",
            "witness_opening_retained_parent_checkpoint_opening_row_count",
        ),
        (
            "\"finish_witness_opening_row_dedup_input_rows\"",
            "witness_opening_row_dedup_input_row_count",
        ),
        (
            "\"finish_witness_opening_row_dedup_unique_rows\"",
            "witness_opening_row_dedup_unique_row_count",
        ),
        (
            "\"finish_witness_opening_row_dedup_elided_rows\"",
            "witness_opening_row_dedup_elided_row_count",
        ),
    ] {
        assert!(
            proof_timing_source.contains(line_name) && proof_timing_source.contains(field),
            "CLI proof timing output should include {line_name}"
        );
    }
    for (line_name, field) in [
        (
            "\"finish_witness_external_source\"",
            "witness_external_source",
        ),
        (
            "\"finish_witness_external_source_descriptor_upload\"",
            "witness_external_source_descriptor_upload",
        ),
        (
            "\"finish_witness_opening_row_values\"",
            "witness_opening_row_values",
        ),
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
        (
            "\"finish_witness_opening_row_values_device_rows\"",
            "witness_opening_row_values_device_row_count",
        ),
        (
            "\"finish_witness_opening_row_values_device_download_batches\"",
            "witness_opening_row_values_device_download_batch_count",
        ),
        (
            "\"finish_witness_opening_row_values_device_single_downloads\"",
            "witness_opening_row_values_device_single_download_count",
        ),
        (
            "\"finish_witness_opening_row_values_source_rows\"",
            "witness_opening_row_values_source_row_count",
        ),
        (
            "\"finish_witness_opening_row_values_words\"",
            "witness_opening_row_values_word_count",
        ),
        (
            "\"finish_witness_opening_row_values_bytes\"",
            "witness_opening_row_values_byte_count",
        ),
        (
            "\"finish_witness_external_source_descriptor_upload_bytes\"",
            "witness_external_source_descriptor_upload_byte_count",
        ),
        (
            "\"finish_witness_external_source_descriptor_upload_words\"",
            "witness_external_source_descriptor_upload_word_count",
        ),
        (
            "\"finish_witness_external_source_descriptor_upload_rows\"",
            "witness_external_source_descriptor_upload_row_count",
        ),
        (
            "\"finish_witness_external_source_trace_expand\"",
            "witness_external_source_trace_expand",
        ),
        ("\"finish_witness_opening_setup\"", "witness_opening_setup"),
        (
            "\"finish_witness_opening_leaf_extend\"",
            "witness_opening_leaf_extend",
        ),
        (
            "\"finish_witness_opening_leaf_hash\"",
            "witness_opening_leaf_hash",
        ),
        ("\"finish_witness_opening_path\"", "witness_opening_path"),
    ] {
        assert!(
            proof_timing_source.contains(line_name) && proof_timing_source.contains(field),
            "CLI proof timing output should include {line_name}"
        );
    }
    for (line_name, field) in [
        (
            "\"finish_witness_opening_leaf_hash_rows\"",
            "witness_opening_leaf_hash_row_count",
        ),
        (
            "\"finish_witness_opening_leaf_hash_bytes\"",
            "witness_opening_leaf_hash_byte_count",
        ),
        (
            "\"finish_witness_opening_leaf_hash_arity2_rows\"",
            "witness_opening_leaf_hash_arity2_row_count",
        ),
        (
            "\"finish_witness_opening_leaf_hash_arity2_bytes\"",
            "witness_opening_leaf_hash_arity2_byte_count",
        ),
        (
            "\"finish_witness_opening_leaf_hash_arity4_rows\"",
            "witness_opening_leaf_hash_arity4_row_count",
        ),
        (
            "\"finish_witness_opening_leaf_hash_arity4_bytes\"",
            "witness_opening_leaf_hash_arity4_byte_count",
        ),
        (
            "\"finish_witness_opening_leaf_coset_extend_calls\"",
            "witness_opening_leaf_coset_extend_call_count",
        ),
        (
            "\"finish_witness_opening_leaf_coset_extend_output_bytes\"",
            "witness_opening_leaf_coset_extend_output_byte_count",
        ),
        (
            "\"finish_witness_opening_leaf_coset_extend_columns\"",
            "witness_opening_leaf_coset_extend_column_count",
        ),
        (
            "\"finish_witness_opening_leaf_coset_extend_max_columns\"",
            "witness_opening_leaf_coset_extend_max_column_count",
        ),
        (
            "\"finish_witness_opening_leaf_coset_extend_ntt_launches\"",
            "witness_opening_leaf_coset_extend_ntt_launch_count",
        ),
        (
            "\"finish_witness_opening_leaf_coset_extend_bit_reverse_launches\"",
            "witness_opening_leaf_coset_extend_bit_reverse_launch_count",
        ),
        (
            "\"finish_witness_opening_leaf_coset_extend_ntt_stage_launches\"",
            "witness_opening_leaf_coset_extend_ntt_stage_launch_count",
        ),
        (
            "\"finish_witness_opening_leaf_coset_extend_ntt_block_twiddle_launches\"",
            "witness_opening_leaf_coset_extend_ntt_block_twiddle_launch_count",
        ),
        (
            "\"finish_witness_opening_leaf_coset_extend_normalize_launches\"",
            "witness_opening_leaf_coset_extend_normalize_launch_count",
        ),
        (
            "\"finish_witness_opening_leaf_coset_extend_pack_launches\"",
            "witness_opening_leaf_coset_extend_pack_launch_count",
        ),
        (
            "\"finish_witness_opening_leaf_coset_extend_unpack_launches\"",
            "witness_opening_leaf_coset_extend_unpack_launch_count",
        ),
    ] {
        assert!(
            proof_timing_source.contains(line_name) && proof_timing_source.contains(field),
            "CLI proof timing output should include {line_name}"
        );
    }
    for (line_name, field) in [
        (
            "\"finish_witness_opening_path_parent_hash_rows\"",
            "witness_opening_path_parent_hash_row_count",
        ),
        (
            "\"finish_witness_opening_path_parent_hash_bytes\"",
            "witness_opening_path_parent_hash_byte_count",
        ),
        (
            "\"finish_witness_opening_path_parent_hash_launches\"",
            "witness_opening_path_parent_hash_launch_count",
        ),
        (
            "\"finish_witness_opening_path_parent_hash_rows_per_query\"",
            "witness_opening_query_count",
        ),
        (
            "\"finish_witness_opening_path_parent_hash_rows_per_stage\"",
            "witness_opening_stage_count",
        ),
        (
            "\"finish_witness_opening_path_parent_hash_launches_per_stage\"",
            "witness_opening_stage_count",
        ),
    ] {
        assert!(
            proof_timing_source.contains(line_name) && proof_timing_source.contains(field),
            "CLI proof timing output should include {line_name}"
        );
    }
    assert!(
        proof_timing_source.contains("\"finish_witness_opening_path_parent_hash\""),
        "CLI proof timing output should include path parent hash split prefix"
    );
    for (suffix, field) in [
        (
            "\"recomputed\"",
            "witness_opening_path_parent_hash_recomputed_row_count",
        ),
        (
            "\"recomputed\"",
            "witness_opening_path_parent_hash_recomputed_byte_count",
        ),
        (
            "\"recomputed\"",
            "witness_opening_path_parent_hash_recomputed_launch_count",
        ),
        (
            "\"retained_leaf_digest\"",
            "witness_opening_path_parent_hash_retained_leaf_digest_row_count",
        ),
        (
            "\"retained_leaf_digest\"",
            "witness_opening_path_parent_hash_retained_leaf_digest_byte_count",
        ),
        (
            "\"retained_leaf_digest\"",
            "witness_opening_path_parent_hash_retained_leaf_digest_launch_count",
        ),
        (
            "\"retained_parent_checkpoint_prefix\"",
            "witness_opening_path_parent_hash_retained_parent_checkpoint_prefix_row_count",
        ),
        (
            "\"retained_parent_checkpoint_prefix\"",
            "witness_opening_path_parent_hash_retained_parent_checkpoint_prefix_byte_count",
        ),
        (
            "\"retained_parent_checkpoint_prefix\"",
            "witness_opening_path_parent_hash_retained_parent_checkpoint_prefix_launch_count",
        ),
        (
            "\"retained_parent_checkpoint_suffix\"",
            "witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_row_count",
        ),
        (
            "\"retained_parent_checkpoint_suffix\"",
            "witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_byte_count",
        ),
        (
            "\"retained_parent_checkpoint_suffix\"",
            "witness_opening_path_parent_hash_retained_parent_checkpoint_suffix_launch_count",
        ),
    ] {
        assert!(
            proof_timing_source.contains(suffix) && proof_timing_source.contains(field),
            "CLI proof timing output should bind path parent hash split {suffix}"
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
            "source_lookup_checked_acceptance_projects_verifier_acceptance",
            "source_lookup_checked_acceptance_projects_auxiliary_evidence",
            "source_lookup_auxiliary_acceptance_sound",
            "source_lookup_checked_acceptance_verifier_core_contract",
            "witness_leaf_digest_checked_acceptance_projects_verifier_acceptance",
            "witness_leaf_digest_checked_acceptance_projects_evidence",
            "witness_leaf_digest_checked_acceptance_projects_canonical_leaf_bytes",
            "witness_leaf_digest_checked_acceptance_projects_narrow_padded_digest_rows",
            "witness_leaf_digest_checked_acceptance_projects_wide_linear_digest_rows",
            "witness_leaf_digest_acceptance_sound",
            "witness_leaf_digest_checked_acceptance_verifier_core_contract",
            "gpu_canonical_leaf_checked_acceptance_projects_verifier_acceptance",
            "gpu_canonical_leaf_checked_acceptance_projects_flag_clear",
            "gpu_canonical_leaf_checked_acceptance_projects_leaf_bytes",
            "gpu_canonical_leaf_checked_acceptance_sound",
            "gpu_canonical_leaf_checked_acceptance_verifier_core_contract",
            "timing_observed_acceptance_projects_verifier_acceptance",
            "timing_observation_acceptance_sound",
            "timing_observation_acceptance_verifier_core_contract",
            "guest_pc_trace_timing_observed_acceptance_projects_verifier_acceptance",
            "guest_pc_trace_timing_acceptance_sound",
            "guest_pc_trace_timing_acceptance_verifier_core_contract",
            "guest_pc_trace_shape_counts_acceptance_sound",
            "guest_pc_trace_shape_counts_acceptance_verifier_core_contract",
            "guest_pc_trace_memory_access_shape_acceptance_sound",
            "guest_pc_trace_memory_access_shape_acceptance_verifier_core_contract",
            "guest_pc_trace_report_buffer_capacity_acceptance_sound",
            "guest_pc_trace_report_buffer_capacity_acceptance_verifier_core_contract",
            "guest_pc_trace_descriptor_upload_word_count_acceptance_sound",
            "guest_pc_trace_descriptor_upload_word_count_acceptance_verifier_core_contract",
            "guest_pc_trace_descriptor_upload_shape_acceptance_sound",
            "guest_pc_trace_descriptor_upload_shape_acceptance_verifier_core_contract",
            "guest_pc_trace_source_retention_byte_counts_acceptance_sound",
            "guest_pc_trace_source_retention_byte_counts_acceptance_verifier_core_contract",
            "guest_pc_trace_descriptor_buffer_retention_byte_counts_acceptance_sound",
            "guest_pc_trace_descriptor_buffer_retention_byte_counts_acceptance_verifier_core_contract",
            "guest_pc_trace_leaf_output_cache_counts_acceptance_sound",
            "guest_pc_trace_leaf_output_cache_counts_acceptance_verifier_core_contract",
            "guest_pc_trace_leaf_setup_timing_acceptance_sound",
            "guest_pc_trace_leaf_setup_timing_acceptance_verifier_core_contract",
            "guest_pc_trace_leaf_work_timing_acceptance_sound",
            "guest_pc_trace_leaf_work_timing_acceptance_verifier_core_contract",
            "guest_pc_trace_leaf_coset_timing_acceptance_sound",
            "guest_pc_trace_leaf_coset_timing_acceptance_verifier_core_contract",
            "guest_pc_trace_tree_commit_timing_acceptance_sound",
            "guest_pc_trace_tree_commit_timing_acceptance_verifier_core_contract",
            "guest_pc_trace_segment_commit_worker_timing_acceptance_sound",
            "guest_pc_trace_segment_commit_worker_timing_acceptance_verifier_core_contract",
            "timing_projected_core_contracts",
            "witness_opening_row_value_timing_observed_acceptance_projects_verifier_acceptance",
            "witness_opening_row_value_timing_acceptance_sound",
            "witness_opening_row_value_timing_acceptance_verifier_core_contract",
            "witness_opening_row_value_aggregate_timing_acceptance_sound",
            "witness_opening_row_value_aggregate_timing_acceptance_verifier_core_contract",
            "constant_material_validation_timing_observed_acceptance_projects_verifier_acceptance",
            "constant_material_validation_timing_acceptance_sound",
            "constant_material_validation_timing_acceptance_verifier_core_contract",
            "constant_material_validation_aggregate_timing_acceptance_sound",
            "constant_material_validation_aggregate_timing_acceptance_verifier_core_contract",
            "prover_gpu_mode_observed_acceptance_projects_verifier_acceptance",
            "prover_gpu_mode_acceptance_sound",
            "prover_gpu_mode_acceptance_verifier_core_contract",
            "gpu_run_options_observed_acceptance_projects_verifier_acceptance",
            "gpu_run_options_acceptance_sound",
            "gpu_run_options_acceptance_verifier_core_contract",
            "cuda_backend_observed_acceptance_projects_verifier_acceptance",
            "cuda_backend_acceptance_sound",
            "cuda_backend_acceptance_verifier_core_contract",
            "cuda_allocator_timing_observed_acceptance_projects_verifier_acceptance",
            "cuda_allocator_timing_acceptance_sound",
            "cuda_allocator_timing_acceptance_verifier_core_contract",
            "cuda_allocator_aggregate_timing_acceptance_sound",
            "cuda_allocator_aggregate_timing_acceptance_verifier_core_contract",
            "cuda_allocator_host_registration_timing_acceptance_sound",
            "cuda_allocator_host_registration_timing_acceptance_verifier_core_contract",
            "proof_artifact_finish_timing_observed_acceptance_projects_verifier_acceptance",
            "proof_artifact_finish_timing_acceptance_sound",
            "proof_artifact_finish_timing_acceptance_verifier_core_contract",
            "proof_artifact_finish_timing_some_summary_acceptance_sound",
            "proof_artifact_finish_timing_some_summary_acceptance_verifier_core_contract",
            "proof_artifact_finish_top_level_timing_acceptance_sound",
            "proof_artifact_finish_top_level_timing_acceptance_verifier_core_contract",
            "proof_artifact_finish_witness_opening_shape_acceptance_sound",
            "proof_artifact_finish_witness_opening_shape_acceptance_verifier_core_contract",
            "proof_artifact_finish_leaf_work_shape_acceptance_sound",
            "proof_artifact_finish_leaf_work_shape_acceptance_verifier_core_contract",
            "proof_artifact_finish_path_parent_hash_shape_acceptance_sound",
            "proof_artifact_finish_path_parent_hash_shape_acceptance_verifier_core_contract",
            "proof_artifact_finish_path_parent_hash_per_unit_shape_acceptance_sound",
            "proof_artifact_finish_path_parent_hash_per_unit_shape_acceptance_verifier_core_contract",
            "proof_artifact_finish_row_values_shape_acceptance_sound",
            "proof_artifact_finish_row_values_shape_acceptance_verifier_core_contract",
            "proof_artifact_finish_external_source_timing_acceptance_sound",
            "proof_artifact_finish_external_source_timing_acceptance_verifier_core_contract",
            "proof_artifact_finish_witness_opening_subtiming_acceptance_sound",
            "proof_artifact_finish_witness_opening_subtiming_acceptance_verifier_core_contract",
            "proof_artifact_finish_descriptor_upload_word_count_acceptance_sound",
            "proof_artifact_finish_descriptor_upload_word_count_acceptance_verifier_core_contract",
            "proof_artifact_finish_descriptor_upload_shape_acceptance_sound",
            "proof_artifact_finish_descriptor_upload_shape_acceptance_verifier_core_contract",
            "proof_artifact_finish_aggregate_timing_acceptance_sound",
            "proof_artifact_finish_aggregate_timing_acceptance_verifier_core_contract",
            "proof_timing_projected_core_contracts",
            "proof_timing_batch_observed_acceptance_projects_verifier_acceptance",
            "proof_timing_batch_acceptance_sound",
            "proof_timing_batch_acceptance_verifier_core_contract",
            "runtime_performance_observed_acceptance_projects_verifier_acceptance",
            "runtime_performance_observation_acceptance_sound",
            "runtime_performance_observation_acceptance_verifier_core_contract",
            "runtime_performance_observation_projects_timing_observations",
            "runtime_performance_observation_timing_observations_acceptance_sound",
            "runtime_performance_observation_timing_observations_acceptance_verifier_core_contract",
            "runtime_performance_observation_projects_guest_pc_trace_timing",
            "runtime_performance_observation_guest_pc_trace_timing_acceptance_sound",
            "runtime_performance_observation_guest_pc_trace_timing_acceptance_verifier_core_contract",
            "runtime_performance_observation_projects_witness_opening_row_value_timing",
            "runtime_performance_observation_row_value_timing_acceptance_sound",
            "runtime_performance_observation_row_value_timing_acceptance_verifier_core_contract",
            "runtime_performance_observation_projects_constant_material_validation_timing",
            "runtime_performance_observation_constant_material_timing_acceptance_sound",
            "runtime_performance_observation_constant_material_timing_acceptance_verifier_core_contract",
            "runtime_performance_observation_projects_prover_gpu_mode",
            "runtime_performance_observation_prover_gpu_mode_acceptance_sound",
            "runtime_performance_observation_prover_gpu_mode_acceptance_verifier_core_contract",
            "runtime_performance_observation_projects_gpu_run_options",
            "runtime_performance_observation_gpu_run_options_acceptance_sound",
            "runtime_performance_observation_gpu_run_options_acceptance_verifier_core_contract",
            "runtime_performance_observation_projects_cuda_backend",
            "runtime_performance_observation_cuda_backend_acceptance_sound",
            "runtime_performance_observation_cuda_backend_acceptance_verifier_core_contract",
            "runtime_performance_observation_projects_cuda_allocator_timing",
            "runtime_performance_observation_cuda_allocator_timing_acceptance_sound",
            concat!(
                "runtime_performance_observation_cuda_allocator_timing_",
                "acceptance_verifier_core_contract"
            ),
            "runtime_performance_observation_projects_proof_artifact_finish_timing",
            "runtime_performance_observation_finish_timing_acceptance_sound",
            "runtime_performance_observation_finish_timing_acceptance_verifier_core_contract",
            "runtime_performance_observation_projects_proof_timing_batch",
            "runtime_performance_observation_proof_timing_batch_acceptance_sound",
            "runtime_performance_observation_proof_timing_batch_acceptance_verifier_core_contract",
            "runtime_performance_observation_projected_core_contracts",
            "gpu_setup_checked_acceptance_projects_constants_sound",
            "gpu_setup_checked_acceptance_sound",
            "gpu_setup_checked_acceptance_verifier_core_contract",
            "gpu_allocation_checked_acceptance_projects_written_contents",
            "gpu_allocation_checked_acceptance_sound",
            "gpu_allocation_checked_acceptance_verifier_core_contract",
            "gpu_host_device_copy_round_trip_implies_written_contents",
            "gpu_host_device_copy_round_trip_checked_acceptance_projects_round_trip",
            "gpu_host_device_copy_round_trip_checked_acceptance_projects_written_contents",
            "gpu_host_device_copy_round_trip_checked_acceptance_sound",
            "gpu_host_device_copy_round_trip_checked_acceptance_verifier_core_contract",
            "gpu_temporary_buffer_reuse_implies_same_request",
            "gpu_temporary_buffer_reuse_implies_pending_reads_complete",
            "gpu_temporary_buffer_reuse_checked_acceptance_projects_same_request",
            "gpu_temporary_buffer_reuse_checked_acceptance_projects_pending_reads_complete",
            "gpu_temporary_buffer_reuse_checked_acceptance_sound",
            "gpu_temporary_buffer_reuse_checked_acceptance_verifier_core_contract",
            "gpu_leaf_output_buffer_reuse_implies_canonical_leaf_bytes",
            "gpu_leaf_output_buffer_reuse_checked_acceptance_projects_verifier_acceptance",
            "gpu_leaf_output_buffer_reuse_checked_acceptance_projects_length_match",
            "gpu_leaf_output_buffer_reuse_checked_acceptance_projects_fully_overwritten",
            "gpu_leaf_output_buffer_reuse_checked_acceptance_projects_leaf_bytes",
            "gpu_leaf_output_buffer_reuse_checked_acceptance_sound",
            "gpu_leaf_output_buffer_reuse_checked_acceptance_verifier_core_contract",
            "gpu_allocator_no_wait_bypass_implies_same_request",
            "gpu_allocator_no_wait_bypass_implies_pending_not_reused",
            "gpu_allocator_no_wait_bypass_implies_fresh_allocation",
            "gpu_allocator_no_wait_bypass_checked_acceptance_projects_same_request",
            "gpu_allocator_no_wait_bypass_checked_acceptance_projects_pending_not_reused",
            "gpu_allocator_no_wait_bypass_checked_acceptance_projects_fresh_allocation",
            "gpu_allocator_no_wait_bypass_checked_acceptance_sound",
            "gpu_allocator_no_wait_bypass_checked_acceptance_verifier_core_contract",
            "gpu_allocator_no_wait_limit_checked_acceptance_projects_decision",
            "gpu_allocator_no_wait_limit_checked_acceptance_sound",
            "gpu_allocator_no_wait_limit_checked_acceptance_verifier_core_contract",
            "guest_pc_trace_segment_queue_checked_acceptance_projects_decision",
            "guest_pc_trace_segment_queue_checked_acceptance_sound",
            "guest_pc_trace_segment_queue_checked_acceptance_verifier_core_contract",
            "guest_pc_trace_cross_root_materialization_checked_acceptance_projects_decision",
            concat!(
                "guest_pc_trace_cross_root_materialization_decision_",
                "default_enabled_when_supported"
            ),
            concat!(
                "guest_pc_trace_cross_root_materialization_decision_",
                "disabled_when_unsupported"
            ),
            concat!(
                "guest_pc_trace_cross_root_materialization_checked_acceptance_",
                "projects_default_enabled"
            ),
            concat!(
                "guest_pc_trace_cross_root_materialization_checked_acceptance_",
                "projects_disabled"
            ),
            "guest_pc_trace_cross_root_materialization_checked_acceptance_sound",
            concat!(
                "guest_pc_trace_cross_root_materialization_checked_acceptance_",
                "verifier_core_contract"
            ),
            "guest_pc_trace_commit_mode_checked_acceptance_projects_decision",
            "guest_pc_trace_commit_mode_effective_worker_positive",
            "guest_pc_trace_commit_mode_async_requires_single_worker",
            concat!(
                "guest_pc_trace_descriptor_buffer_retention_",
                "default_disabled_for_parallel_lower"
            ),
            "guest_pc_trace_commit_mode_descriptor_retention_matches",
            "guest_pc_trace_commit_mode_disabled_root_window_is_one",
            concat!(
                "guest_pc_trace_commit_mode_checked_acceptance_",
                "projects_descriptor_retention"
            ),
            concat!(
                "guest_pc_trace_commit_mode_checked_acceptance_",
                "projects_disabled_root_window"
            ),
            "guest_pc_trace_commit_mode_checked_acceptance_sound",
            "guest_pc_trace_commit_mode_checked_acceptance_verifier_core_contract",
            "guest_pc_trace_cuda_run_checked_acceptance_projects_decision",
            "guest_pc_trace_cuda_run_sparse_source_matches",
            "guest_pc_trace_cuda_run_sparse_source_debug_matches",
            "guest_pc_trace_cuda_run_terminal_sparse_source_matches",
            "guest_pc_trace_cuda_run_retained_stage_source_matches",
            concat!(
                "guest_pc_trace_cuda_run_retained_stage_source_debug_",
                "uses_selected_source"
            ),
            concat!(
                "guest_pc_trace_cuda_run_retained_stage_source_debug_",
                "decision_matches"
            ),
            "guest_pc_trace_cuda_run_retained_stage_source_debug_matches",
            "fri_retained_stage_source_debug_requires_retention",
            concat!(
                "guest_pc_trace_cuda_run_retained_stage_source_debug_",
                "requires_retention"
            ),
            "guest_pc_trace_cuda_run_descriptor_retention_matches",
            "guest_pc_trace_cuda_run_checked_acceptance_projects_sparse_source",
            concat!(
                "guest_pc_trace_cuda_run_checked_acceptance_",
                "projects_sparse_source_debug"
            ),
            concat!(
                "guest_pc_trace_cuda_run_checked_acceptance_",
                "projects_terminal_sparse_source"
            ),
            concat!(
                "guest_pc_trace_cuda_run_checked_acceptance_",
                "projects_retained_stage_source"
            ),
            concat!(
                "guest_pc_trace_cuda_run_checked_acceptance_",
                "projects_retained_source_debug"
            ),
            concat!(
                "guest_pc_trace_cuda_run_checked_acceptance_",
                "projects_retained_debug_requires_retention"
            ),
            concat!(
                "guest_pc_trace_cuda_run_checked_acceptance_",
                "projects_descriptor_retention"
            ),
            "guest_pc_trace_cuda_run_checked_acceptance_sound",
            "guest_pc_trace_cuda_run_checked_acceptance_verifier_core_contract",
            "gpu_retained_leaf_digest_limit_checked_acceptance_projects_decision",
            "gpu_retained_leaf_digest_limit_checked_acceptance_sound",
            "gpu_retained_leaf_digest_limit_checked_acceptance_verifier_core_contract",
            "gpu_retained_device_cache_budget_checked_acceptance_projects_within_limits",
            "gpu_retained_device_cache_budget_checked_acceptance_sound",
            "gpu_retained_device_cache_budget_checked_acceptance_verifier_core_contract",
            "fri_fixed_column_cache_same_request_implies_cached_contents_bound",
            "fri_fixed_column_cache_checked_acceptance_projects_request_bound",
            "fri_fixed_column_cache_checked_acceptance_projects_fresh_contents_bound",
            "fri_fixed_column_cache_checked_acceptance_projects_cached_contents_bound",
            "fri_fixed_column_cache_checked_acceptance_sound",
            "fri_fixed_column_cache_checked_acceptance_verifier_core_contract",
            "gpu_coset_extension_matches_host_implies_leaf_bytes",
            "gpu_coset_extension_checked_acceptance_projects_verifier_acceptance",
            "gpu_coset_extension_checked_acceptance_projects_matches_host",
            "gpu_coset_extension_checked_acceptance_projects_leaf_bytes",
            "gpu_coset_extension_checked_acceptance_sound",
            "gpu_coset_extension_checked_acceptance_verifier_core_contract",
            "gpu_fri_interpolation_matches_host_implies_fri_folds_valid",
            "gpu_fri_fold_interpolation_checked_acceptance_projects_verifier_acceptance",
            "gpu_fri_fold_interpolation_checked_acceptance_projects_matches_host",
            "gpu_fri_fold_interpolation_checked_acceptance_projects_fri_folds_valid",
            "gpu_fri_fold_interpolation_checked_acceptance_sound",
            "gpu_fri_fold_interpolation_checked_acceptance_verifier_core_contract",
            "gpu_merkle_digest_prefix_batch_matches_single_paths_implies_lower_prefixes_bound",
            "gpu_merkle_digest_prefix_batch_checked_acceptance_projects_verifier_acceptance",
            "gpu_merkle_digest_prefix_batch_checked_acceptance_projects_matches_single_paths",
            "gpu_merkle_digest_prefix_batch_checked_acceptance_projects_lower_prefixes_bound",
            "gpu_merkle_digest_prefix_batch_checked_acceptance_sound",
            "gpu_merkle_digest_prefix_batch_checked_acceptance_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &gpu_runtime_source,
        "gpu_setup_checked_acceptance_verifier_core_contract",
        &["GpuRuntimeInternal.checked_acceptance_verifier_core_contract"],
    );
    lean_binding::assert_theorem_body_omits(
        &gpu_runtime_source,
        "gpu_setup_checked_acceptance_verifier_core_contract",
        &[
            "gpu_setup_checked_acceptance_sound",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &gpu_runtime_source,
        "gpu_allocation_checked_acceptance_verifier_core_contract",
        &["GpuRuntimeInternal.checked_acceptance_verifier_core_contract"],
    );
    lean_binding::assert_theorem_body_omits(
        &gpu_runtime_source,
        "gpu_allocation_checked_acceptance_verifier_core_contract",
        &[
            "gpu_allocation_checked_acceptance_sound",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &gpu_runtime_source,
        "gpu_host_device_copy_round_trip_checked_acceptance_verifier_core_contract",
        &["GpuRuntimeInternal.checked_acceptance_verifier_core_contract"],
    );
    lean_binding::assert_theorem_body_omits(
        &gpu_runtime_source,
        "gpu_host_device_copy_round_trip_checked_acceptance_verifier_core_contract",
        &[
            "gpu_host_device_copy_round_trip_checked_acceptance_sound",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &gpu_runtime_source,
        "gpu_temporary_buffer_reuse_checked_acceptance_verifier_core_contract",
        &["GpuRuntimeInternal.checked_acceptance_verifier_core_contract"],
    );
    lean_binding::assert_theorem_body_omits(
        &gpu_runtime_source,
        "gpu_temporary_buffer_reuse_checked_acceptance_verifier_core_contract",
        &[
            "gpu_temporary_buffer_reuse_checked_acceptance_sound",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &gpu_runtime_source,
        "gpu_allocator_no_wait_bypass_checked_acceptance_verifier_core_contract",
        &["GpuRuntimeInternal.checked_acceptance_verifier_core_contract"],
    );
    lean_binding::assert_theorem_body_omits(
        &gpu_runtime_source,
        "gpu_allocator_no_wait_bypass_checked_acceptance_verifier_core_contract",
        &[
            "gpu_allocator_no_wait_bypass_checked_acceptance_sound",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &gpu_runtime_source,
        "gpu_allocator_no_wait_limit_checked_acceptance_verifier_core_contract",
        &["GpuRuntimeInternal.checked_acceptance_verifier_core_contract"],
    );
    lean_binding::assert_theorem_body_omits(
        &gpu_runtime_source,
        "gpu_allocator_no_wait_limit_checked_acceptance_verifier_core_contract",
        &[
            "gpu_allocator_no_wait_limit_checked_acceptance_sound",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &gpu_runtime_source,
        "guest_pc_trace_segment_queue_checked_acceptance_verifier_core_contract",
        &["GpuRuntimeInternal.checked_acceptance_verifier_core_contract"],
    );
    lean_binding::assert_theorem_body_omits(
        &gpu_runtime_source,
        "guest_pc_trace_segment_queue_checked_acceptance_verifier_core_contract",
        &[
            "guest_pc_trace_segment_queue_checked_acceptance_sound",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &gpu_runtime_source,
        "guest_pc_trace_cuda_run_checked_acceptance_verifier_core_contract",
        &["GpuRuntimeInternal.checked_acceptance_verifier_core_contract"],
    );
    lean_binding::assert_theorem_body_omits(
        &gpu_runtime_source,
        "guest_pc_trace_cuda_run_checked_acceptance_verifier_core_contract",
        &[
            "guest_pc_trace_cuda_run_checked_acceptance_sound",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &gpu_runtime_source,
        "gpu_retained_leaf_digest_limit_checked_acceptance_verifier_core_contract",
        &["GpuRuntimeInternal.checked_acceptance_verifier_core_contract"],
    );
    lean_binding::assert_theorem_body_omits(
        &gpu_runtime_source,
        "gpu_retained_leaf_digest_limit_checked_acceptance_verifier_core_contract",
        &[
            "gpu_retained_leaf_digest_limit_checked_acceptance_sound",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &gpu_runtime_source,
        "gpu_retained_device_cache_budget_checked_acceptance_verifier_core_contract",
        &["GpuRuntimeInternal.checked_acceptance_verifier_core_contract"],
    );
    lean_binding::assert_theorem_body_omits(
        &gpu_runtime_source,
        "gpu_retained_device_cache_budget_checked_acceptance_verifier_core_contract",
        &[
            "gpu_retained_device_cache_budget_checked_acceptance_sound",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &gpu_runtime_source,
        "fri_fixed_column_cache_checked_acceptance_verifier_core_contract",
        &["GpuRuntimeInternal.checked_acceptance_verifier_core_contract"],
    );
    lean_binding::assert_theorem_body_omits(
        &gpu_runtime_source,
        "fri_fixed_column_cache_checked_acceptance_verifier_core_contract",
        &[
            "fri_fixed_column_cache_checked_acceptance_sound",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_proof_timing_source,
        "witness_opening_row_value_aggregate_timing_acceptance_verifier_core_contract",
        &["witness_opening_row_value_timing_acceptance_verifier_core_contract"],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_proof_timing_source,
        "witness_opening_row_value_aggregate_timing_acceptance_verifier_core_contract",
        &[
            "witness_opening_row_value_aggregate_timing_acceptance_sound",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_proof_timing_source,
        "constant_material_validation_aggregate_timing_acceptance_verifier_core_contract",
        &["constant_material_validation_timing_acceptance_verifier_core_contract"],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_proof_timing_source,
        "constant_material_validation_aggregate_timing_acceptance_verifier_core_contract",
        &[
            "constant_material_validation_aggregate_timing_acceptance_sound",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_proof_timing_source,
        "cuda_allocator_aggregate_timing_acceptance_verifier_core_contract",
        &["cuda_allocator_timing_acceptance_verifier_core_contract"],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_proof_timing_source,
        "cuda_allocator_aggregate_timing_acceptance_verifier_core_contract",
        &[
            "cuda_allocator_aggregate_timing_acceptance_sound",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_proof_timing_source,
        "cuda_allocator_host_registration_timing_acceptance_verifier_core_contract",
        &["cuda_allocator_timing_acceptance_verifier_core_contract"],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_proof_timing_source,
        "cuda_allocator_host_registration_timing_acceptance_verifier_core_contract",
        &[
            "cuda_allocator_host_registration_timing_acceptance_sound",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_proof_timing_source,
        "proof_artifact_finish_timing_some_summary_acceptance_verifier_core_contract",
        &["proof_artifact_finish_timing_acceptance_verifier_core_contract"],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_proof_timing_source,
        "proof_artifact_finish_timing_some_summary_acceptance_verifier_core_contract",
        &[
            "proof_artifact_finish_timing_some_summary_acceptance_sound",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_proof_timing_source,
        "proof_artifact_finish_top_level_timing_acceptance_verifier_core_contract",
        &["proof_artifact_finish_timing_some_summary_acceptance_verifier_core_contract"],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_proof_timing_source,
        "proof_artifact_finish_top_level_timing_acceptance_verifier_core_contract",
        &[
            "proof_artifact_finish_top_level_timing_acceptance_sound",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_proof_timing_source,
        "proof_artifact_finish_witness_opening_shape_acceptance_verifier_core_contract",
        &["proof_artifact_finish_timing_some_summary_acceptance_verifier_core_contract"],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_proof_timing_source,
        "proof_artifact_finish_witness_opening_shape_acceptance_verifier_core_contract",
        &[
            "proof_artifact_finish_witness_opening_shape_acceptance_sound",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_proof_timing_source,
        "proof_artifact_finish_leaf_work_shape_acceptance_verifier_core_contract",
        &["proof_artifact_finish_timing_some_summary_acceptance_verifier_core_contract"],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_proof_timing_source,
        "proof_artifact_finish_leaf_work_shape_acceptance_verifier_core_contract",
        &[
            "proof_artifact_finish_leaf_work_shape_acceptance_sound",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    for (theorem, sound_theorem) in [
        (
            "proof_artifact_finish_path_parent_hash_shape_acceptance_verifier_core_contract",
            "proof_artifact_finish_path_parent_hash_shape_acceptance_sound",
        ),
        (
            "proof_artifact_finish_path_parent_hash_per_unit_shape_acceptance_verifier_core_contract",
            "proof_artifact_finish_path_parent_hash_per_unit_shape_acceptance_sound",
        ),
        (
            "proof_artifact_finish_row_values_shape_acceptance_verifier_core_contract",
            "proof_artifact_finish_row_values_shape_acceptance_sound",
        ),
        (
            "proof_artifact_finish_external_source_timing_acceptance_verifier_core_contract",
            "proof_artifact_finish_external_source_timing_acceptance_sound",
        ),
        (
            "proof_artifact_finish_witness_opening_subtiming_acceptance_verifier_core_contract",
            "proof_artifact_finish_witness_opening_subtiming_acceptance_sound",
        ),
        (
            "proof_artifact_finish_descriptor_upload_word_count_acceptance_verifier_core_contract",
            "proof_artifact_finish_descriptor_upload_word_count_acceptance_sound",
        ),
        (
            "proof_artifact_finish_descriptor_upload_shape_acceptance_verifier_core_contract",
            "proof_artifact_finish_descriptor_upload_shape_acceptance_sound",
        ),
    ] {
        lean_binding::assert_theorem_body_contains(
            &lean_proof_timing_source,
            theorem,
            &["proof_artifact_finish_timing_some_summary_acceptance_verifier_core_contract"],
        );
        lean_binding::assert_theorem_body_omits(
            &lean_proof_timing_source,
            theorem,
            &[sound_theorem, "sound_witness_implies_verifier_core_contract"],
        );
    }
    lean_binding::assert_theorem_body_contains(
        &proof_timing_verifier_source,
        "proof_artifact_finish_aggregate_timing_acceptance_verifier_core_contract",
        &["proof_artifact_finish_timing_some_summary_acceptance_verifier_core_contract"],
    );
    lean_binding::assert_theorem_body_omits(
        &proof_timing_verifier_source,
        "proof_artifact_finish_aggregate_timing_acceptance_verifier_core_contract",
        &[
            "proof_artifact_finish_aggregate_timing_acceptance_sound",
            "proof_artifact_finish_timing_observed_acceptance_projects_verifier_acceptance",
            "assumptions.crypto.transcript_binding",
            "assumptions.semantic.public_input_binding",
            "assumptions.crypto.pcs_opening_sound",
            "assumptions.crypto.fri_query_sound",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &auxiliary_source,
        "source_lookup_checked_acceptance_verifier_core_contract",
        &["auxiliary_checked_acceptance_verifier_core_contract"],
    );
    lean_binding::assert_theorem_body_omits(
        &auxiliary_source,
        "source_lookup_checked_acceptance_verifier_core_contract",
        &[
            "source_lookup_auxiliary_acceptance_sound",
            "assumptions.crypto.transcript_binding",
            "assumptions.semantic.public_input_binding",
            "assumptions.crypto.pcs_opening_sound",
            "assumptions.crypto.fri_query_sound",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &auxiliary_source,
        "witness_leaf_digest_checked_acceptance_verifier_core_contract",
        &["auxiliary_checked_acceptance_verifier_core_contract"],
    );
    lean_binding::assert_theorem_body_omits(
        &auxiliary_source,
        "witness_leaf_digest_checked_acceptance_verifier_core_contract",
        &[
            "witness_leaf_digest_acceptance_sound",
            "assumptions.crypto.transcript_binding",
            "assumptions.semantic.public_input_binding",
            "assumptions.crypto.pcs_opening_sound",
            "assumptions.crypto.fri_query_sound",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &auxiliary_source,
        "gpu_canonical_leaf_checked_acceptance_verifier_core_contract",
        &["auxiliary_checked_acceptance_verifier_core_contract"],
    );
    lean_binding::assert_theorem_body_omits(
        &auxiliary_source,
        "gpu_canonical_leaf_checked_acceptance_verifier_core_contract",
        &[
            "gpu_canonical_leaf_checked_acceptance_sound",
            "assumptions.crypto.transcript_binding",
            "assumptions.semantic.public_input_binding",
            "assumptions.crypto.pcs_opening_sound",
            "assumptions.crypto.fri_query_sound",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &auxiliary_source,
        "gpu_leaf_output_buffer_reuse_checked_acceptance_verifier_core_contract",
        &["auxiliary_checked_acceptance_verifier_core_contract"],
    );
    lean_binding::assert_theorem_body_omits(
        &auxiliary_source,
        "gpu_leaf_output_buffer_reuse_checked_acceptance_verifier_core_contract",
        &[
            "gpu_leaf_output_buffer_reuse_checked_acceptance_sound",
            "assumptions.crypto.transcript_binding",
            "assumptions.semantic.public_input_binding",
            "assumptions.crypto.pcs_opening_sound",
            "assumptions.crypto.fri_query_sound",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &auxiliary_source,
        "gpu_coset_extension_checked_acceptance_verifier_core_contract",
        &["auxiliary_checked_acceptance_verifier_core_contract"],
    );
    lean_binding::assert_theorem_body_omits(
        &auxiliary_source,
        "gpu_coset_extension_checked_acceptance_verifier_core_contract",
        &[
            "gpu_coset_extension_checked_acceptance_sound",
            "assumptions.crypto.transcript_binding",
            "assumptions.semantic.public_input_binding",
            "assumptions.crypto.pcs_opening_sound",
            "assumptions.crypto.fri_query_sound",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &auxiliary_source,
        "gpu_fri_fold_interpolation_checked_acceptance_verifier_core_contract",
        &["auxiliary_checked_acceptance_verifier_core_contract"],
    );
    lean_binding::assert_theorem_body_omits(
        &auxiliary_source,
        "gpu_fri_fold_interpolation_checked_acceptance_verifier_core_contract",
        &[
            "gpu_fri_fold_interpolation_checked_acceptance_sound",
            "assumptions.crypto.transcript_binding",
            "assumptions.semantic.public_input_binding",
            "assumptions.crypto.pcs_opening_sound",
            "assumptions.crypto.fri_query_sound",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &auxiliary_source,
        "gpu_merkle_digest_prefix_batch_checked_acceptance_verifier_core_contract",
        &["auxiliary_checked_acceptance_verifier_core_contract"],
    );
    lean_binding::assert_theorem_body_omits(
        &auxiliary_source,
        "gpu_merkle_digest_prefix_batch_checked_acceptance_verifier_core_contract",
        &[
            "gpu_merkle_digest_prefix_batch_checked_acceptance_sound",
            "assumptions.crypto.transcript_binding",
            "assumptions.semantic.public_input_binding",
            "assumptions.crypto.pcs_opening_sound",
            "assumptions.crypto.fri_query_sound",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
}
