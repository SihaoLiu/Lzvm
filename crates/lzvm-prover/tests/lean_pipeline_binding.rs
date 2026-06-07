use std::path::Path;

#[test]
fn lean_pipeline_binding_exports_required_external_source_soundness() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_path = crate_root.join("../../lean/Lzvm/PipelineBinding.lean");
    let lean_source =
        std::fs::read_to_string(&lean_path).expect("Lean pipeline binding source should read");
    let setup_preflight_path = crate_root.join("src/setup_preflight.rs");
    let setup_preflight_source =
        std::fs::read_to_string(&setup_preflight_path).expect("setup preflight source should read");
    let proof_artifact_path = crate_root.join("src/proof_artifact.rs");
    let proof_artifact_source =
        std::fs::read_to_string(&proof_artifact_path).expect("proof artifact source should read");
    let model_path = crate_root.join("../../lean/Lzvm/Model.lean");
    let model_source = std::fs::read_to_string(&model_path).expect("Lean model source should read");
    let conformance_path = crate_root.join("../../lean/Lzvm/Conformance.lean");
    let conformance_source =
        std::fs::read_to_string(&conformance_path).expect("Lean conformance source should read");
    let external_source_path = crate_root.join("../../lean/Lzvm/ExternalSource.lean");
    let external_source_source = std::fs::read_to_string(&external_source_path)
        .expect("Lean external source source should read");
    let trace_constraint_path = crate_root.join("../../lean/Lzvm/TraceConstraintValidation.lean");
    let trace_constraint_source = std::fs::read_to_string(&trace_constraint_path)
        .expect("Lean trace constraint source should read");
    let runtime_soundness_path = crate_root.join("../../lean/Lzvm/RuntimeSoundness.lean");
    let runtime_soundness_source = std::fs::read_to_string(&runtime_soundness_path)
        .expect("Lean runtime soundness source should read");

    assert!(
        lean_source.contains("runtime_pipeline_binding_required_external_source_sound")
            && lean_source.contains("runtime_trace_constraint_required_external_source_pcs_sound")
            && lean_source.contains("runtime_opening_required_external_source_sound")
            && lean_source
                .contains("runtime_pipeline_binding_checked_acceptance_query_opening_evidence")
            && lean_source
                .contains("runtime_pipeline_binding_checked_acceptance_query_opening_contract")
            && lean_source.contains(
                "runtime_pipeline_binding_checked_acceptance_opening_segment_bound_contract"
            )
            && lean_source.contains("RuntimeOpeningSegmentBindingBoundContract")
            && lean_source
                .contains("runtime_pipeline_binding_checked_acceptance_challenge_query_opening_contract")
            && lean_source
                .contains("runtime_pipeline_binding_checked_acceptance_full_soundness_contract")
            && lean_source
                .contains("runtime_pipeline_binding_checked_acceptance_trace_conformance_contract")
            && lean_source.contains("runtime_pipeline_compact_digest_merkle_observation_eq_full_state")
            && lean_source
                .contains("runtime_pipeline_binding_checked_acceptance_compact_digest_merkle_contract")
            && lean_source.contains("RowMajorDigestPrefixEvidence")
            && lean_source.contains("wideLinearDigestsBindRows")
            && lean_source
                .contains("runtime_pipeline_binding_checked_acceptance_verifier_sound_witness")
            && lean_source
                .contains("runtime_pipeline_binding_checked_acceptance_verifier_core_contract")
            && lean_source.contains("constraintBackendConformant")
            && lean_source.contains("system.accepts publicInput proof")
            && lean_source.contains("RuntimeArtifactSoundnessObligations")
            && lean_source.contains("runtime_pipeline_binding_checked_acceptance_execution_obligations")
            && lean_source.contains("RuntimeVerifierCoreContract")
            && lean_source.contains("SoundWitness system publicInput proof"),
        "Lean pipeline binding should expose query-plan, opening, execution, and full runtime soundness evidence"
    );
    assert!(
        model_source.contains("def RuntimeVerifierCoreContract")
            && model_source.contains("sound_witness_implies_verifier_core_contract")
            && model_source.contains("SoundWitness system publicInput proof")
            && model_source.contains("RuntimeVerifierCoreContract system publicInput proof")
            && runtime_soundness_source.contains("RuntimeVerifierCoreContract")
            && runtime_soundness_source.contains("runtime_soundness_checked_acceptance_core_obligations"),
        "Lean runtime soundness should name the verifier core obligations shared by pipeline soundness"
    );
    assert!(
        conformance_source.contains("RuntimeArtifactSoundnessObligations")
            && conformance_source.contains("RuntimeVerifierCoreContract")
            && external_source_source.contains("ExternalSourceOpeningSoundnessObligations")
            && external_source_source.contains("RuntimeVerifierCoreContract")
            && trace_constraint_source.contains("RuntimeTraceConstraintSoundnessObligations")
            && trace_constraint_source.contains("RuntimeVerifierCoreContract"),
        "Lean soundness obligations should share the model-level verifier core contract"
    );
    assert!(
        setup_preflight_source.contains("validate_global_source_lookup_hints")
            && setup_preflight_source.contains("SourceLookupBalance::default")
            && setup_preflight_source.contains("validate_all_units"),
        "setup preflight should keep source lookup balance validation wired into proof checks"
    );
    assert!(
        proof_artifact_source.contains("build_pcs_query_plan_segment")
            && proof_artifact_source.contains("build_witness_opening_segment_batch_from_trace_outputs")
            && proof_artifact_source
                .contains("build_pcs_fri_transcript_values_from_trace_segment_refs")
            && proof_artifact_source
                .contains("build_pcs_fri_opening_segment_from_transcript_values"),
        "proof artifact building should keep query-plan, transcript, and opening builders wired together"
    );
}
