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

    assert!(
        lean_source.contains("runtime_pipeline_binding_required_external_source_sound")
            && lean_source.contains("runtime_trace_constraint_required_external_source_pcs_sound")
            && lean_source.contains("runtime_opening_required_external_source_sound")
            && lean_source
                .contains("runtime_pipeline_binding_checked_acceptance_query_opening_evidence")
            && lean_source
                .contains("runtime_pipeline_binding_checked_acceptance_query_opening_contract")
            && lean_source
                .contains("runtime_pipeline_binding_checked_acceptance_full_soundness_contract")
            && lean_source
                .contains("runtime_pipeline_binding_checked_acceptance_verifier_sound_witness")
            && lean_source
                .contains("runtime_pipeline_binding_checked_acceptance_verifier_core_contract")
            && lean_source.contains("system.accepts publicInput proof")
            && lean_source.contains("RuntimeArtifactSoundnessObligations")
            && lean_source.contains("runtime_pipeline_binding_checked_acceptance_execution_obligations")
            && lean_source.contains("SoundWitness system publicInput proof"),
        "Lean pipeline binding should expose query-plan, opening, execution, and full runtime soundness evidence"
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
