use std::path::Path;

#[path = "support/lean_binding.rs"]
mod lean_binding;

#[test]
fn lean_pipeline_binding_exports_required_external_source_soundness() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_path = crate_root.join("../../lean/Lzvm/PipelineBinding.lean");
    let pipeline_source =
        std::fs::read_to_string(&lean_path).expect("Lean pipeline binding source should read");
    let core_path = crate_root.join("../../lean/Lzvm/PipelineBinding/Core.lean");
    let core_source =
        std::fs::read_to_string(&core_path).expect("Lean pipeline core binding source should read");
    let lean_source = format!("{core_source}\n{pipeline_source}");
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

    lean_binding::assert_theorem_declarations(
        &lean_source,
        &[
            "runtime_pipeline_binding_required_external_source_sound",
            "runtime_pipeline_binding_checked_acceptance_pipeline_evidence",
            "runtime_pipeline_binding_checked_acceptance_sound",
            "runtime_pipeline_binding_evidence_implies_transcript_bound",
            "runtime_pipeline_binding_evidence_implies_public_input_bound",
            "runtime_pipeline_binding_checked_acceptance_transcript_bound",
            "runtime_pipeline_binding_checked_acceptance_public_input_bound",
            "runtime_pipeline_binding_evidence_implies_pcs_and_fri",
            "runtime_pipeline_binding_checked_acceptance_pcs_and_fri",
            "runtime_pipeline_binding_evidence_implies_core_obligations",
            "runtime_pipeline_binding_evidence_implies_execution_obligations",
            "runtime_pipeline_binding_evidence_implies_runtime_soundness_evidence",
            "runtime_pipeline_binding_evidence_implies_runtime_artifact_evidence",
            "runtime_pipeline_binding_checked_acceptance_query_opening_evidence",
            "runtime_pipeline_binding_checked_acceptance_query_opening_contract",
            "runtime_pipeline_binding_checked_acceptance_opening_segment_checked_acceptance",
            "runtime_pipeline_binding_checked_acceptance_opening_segment_evidence",
            "runtime_pipeline_binding_checked_acceptance_opening_segment_bound_contract",
            "runtime_pipeline_binding_checked_acceptance_challenge_query_opening_contract",
            "runtime_pipeline_binding_checked_acceptance_full_soundness_contract",
            "runtime_pipeline_binding_checked_acceptance_trace_conformance_contract",
            "runtime_pipeline_compact_digest_merkle_observation_eq_full_state",
            "runtime_pipeline_binding_checked_acceptance_compact_digest_merkle_contract",
            "runtime_pipeline_binding_checked_acceptance_audited_assumptions",
            "runtime_pipeline_binding_checked_acceptance_verifier_sound_witness",
            "runtime_pipeline_binding_checked_acceptance_verifier_core_contract",
            "runtime_pipeline_binding_checked_acceptance_execution_obligations",
            "runtime_pipeline_binding_checked_acceptance_runtime_soundness_contract",
            "runtime_pipeline_binding_checked_acceptance_runtime_soundness_accepts_contract",
            "runtime_pipeline_binding_checked_acceptance_runtime_artifact_evidence",
            "runtime_pipeline_binding_checked_acceptance_runtime_artifact_soundness_obligations",
            "runtime_pipeline_binding_checked_acceptance_trace_artifact_soundness_obligations",
            "runtime_pipeline_binding_checked_acceptance_accepts_full_soundness_contract",
            "runtime_pipeline_binding_checked_acceptance_proof_system_sound",
            "runtime_pipeline_binding_checked_acceptance_proof_system_full_soundness_contract",
            "runtime_pipeline_binding_required_external_source_verifier_core_contract",
            "runtime_pipeline_binding_required_external_source_full_soundness_contract",
        ],
    );
    assert!(
        pipeline_source.contains("import Lzvm.PipelineBinding.Core"),
        "Lean pipeline binding module should import the core pipeline binding module"
    );
    assert!(theorem_prefix(
        &lean_source,
        "runtime_pipeline_binding_checked_acceptance_runtime_artifact_evidence"
    )
    .contains("(validation : RuntimePipelineBindingValidation system)"));
    assert!(theorem_prefix(
        &lean_source,
        "runtime_pipeline_binding_checked_acceptance_pipeline_evidence"
    )
    .contains("(validation : RuntimePipelineBindingValidation system)"));
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_pipeline_binding_checked_acceptance_pipeline_evidence"
        )
        .contains("(assumptions : AssumptionBundle system)"),
        "pipeline evidence projection should require the audited assumption bundle"
    );
    assert!(
        !theorem_prefix(
            &lean_source,
            "runtime_pipeline_binding_checked_acceptance_runtime_artifact_evidence"
        )
        .contains("AssumptionBundle"),
        "runtime artifact evidence projection should not require cryptographic assumptions"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_pipeline_binding_checked_acceptance_runtime_soundness_accepts_contract"
        )
        .contains("system.accepts publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_checked_acceptance_runtime_soundness_accepts_contract"
            )
            .contains("RuntimeSoundnessEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_checked_acceptance_runtime_soundness_accepts_contract"
            )
            .contains("RuntimeVerifierCoreContract system publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_checked_acceptance_runtime_soundness_accepts_contract"
            )
            .contains("SoundWitness system publicInput proof"),
        "runtime soundness accepts contract should expose verifier acceptance and runtime evidence"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_pipeline_binding_checked_acceptance_runtime_artifact_soundness_obligations"
        )
        .contains("(assumptions : AssumptionBundle system)"),
        "runtime artifact soundness obligations should require the audited assumption bundle"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_pipeline_binding_checked_acceptance_trace_artifact_soundness_obligations"
        )
        .contains("(assumptions : AssumptionBundle system)")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_checked_acceptance_trace_artifact_soundness_obligations"
            )
            .contains("requiresExternalSource")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_checked_acceptance_trace_artifact_soundness_obligations"
            )
            .contains("RuntimeTraceConstraintSoundnessObligations"),
        "trace artifact soundness obligations should expose the trace contract with audited assumptions"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_pipeline_binding_checked_acceptance_accepts_full_soundness_contract"
        )
        .contains("(assumptions : AssumptionBundle system)"),
        "accepts plus full soundness contract should require the audited assumption bundle"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_pipeline_binding_checked_acceptance_accepts_full_soundness_contract"
        )
        .contains("system.accepts publicInput proof"),
        "accepts plus full soundness contract should expose verifier acceptance"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_pipeline_binding_checked_acceptance_proof_system_sound"
        )
        .contains("ProofSystemSound system")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_checked_acceptance_proof_system_sound"
            )
            .contains("system.accepts publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_checked_acceptance_proof_system_sound"
            )
            .contains("SoundWitness system publicInput proof"),
        "pipeline checked acceptance should expose model-wide proof-system soundness and the accepted proof witness"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_pipeline_binding_checked_acceptance_proof_system_full_soundness_contract"
        )
        .contains("ProofSystemSound system")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_checked_acceptance_proof_system_full_soundness_contract"
            )
            .contains("system.accepts publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_checked_acceptance_proof_system_full_soundness_contract"
            )
            .contains("RuntimePipelineBindingEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_checked_acceptance_proof_system_full_soundness_contract"
            )
            .contains("RuntimeArtifactSoundnessObligations")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_checked_acceptance_proof_system_full_soundness_contract"
            )
            .contains("RuntimeVerifierCoreContract system publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_checked_acceptance_proof_system_full_soundness_contract"
            )
            .contains("SoundWitness system publicInput proof"),
        "pipeline checked acceptance should package model soundness with the accepted full soundness contract"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_pipeline_binding_required_external_source_verifier_core_contract"
        )
        .contains("ExternalSourceOpeningEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_verifier_core_contract"
            )
            .contains("RuntimeVerifierCoreContract system publicInput proof"),
        "pipeline required external-source projection should expose external evidence and verifier core contract"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_pipeline_binding_required_external_source_full_soundness_contract"
        )
        .contains("system.accepts publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_full_soundness_contract"
            )
            .contains("ExternalSourceOpeningEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_full_soundness_contract"
            )
            .contains("RuntimePipelineBindingEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_full_soundness_contract"
            )
            .contains("RuntimeArtifactSoundnessObligations")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_full_soundness_contract"
            )
            .contains("RuntimeVerifierCoreContract system publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_full_soundness_contract"
            )
            .contains("system.traceConsistent publicInput proof trace")
            && theorem_prefix(
                &lean_source,
                "runtime_pipeline_binding_required_external_source_full_soundness_contract"
            )
            .contains("SoundWitness system publicInput proof"),
        "pipeline required external-source projection should expose the accepted full soundness contract"
    );
    assert!(
        lean_source.contains("runtime_trace_constraint_required_external_source_pcs_sound")
            && lean_source.contains("runtime_opening_required_external_source_sound")
            && lean_source.contains("RuntimeOpeningSegmentBindingBoundContract")
            && lean_source.contains("RowMajorDigestPrefixEvidence")
            && lean_source.contains("wideLinearDigestsBindRows")
            && lean_source.contains("constraintBackendConformant")
            && lean_source.contains("system.accepts publicInput proof")
            && lean_source.contains("RuntimeArtifactSoundnessObligations")
            && lean_source.contains("RuntimeVerifierCoreContract")
            && lean_source.contains("RequiredCryptographicAssumptionStatements assumptions.crypto")
            && lean_source.contains("assumption_bundle_carries_required_crypto_evidence")
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

fn theorem_prefix(source: &str, name: &str) -> String {
    let theorem_start = source
        .find(&format!("theorem {name}"))
        .unwrap_or_else(|| panic!("Lean source should contain theorem {name}"));
    let proof_start = source[theorem_start..]
        .find(" := by")
        .unwrap_or_else(|| panic!("Lean theorem {name} should have a proof body"));
    source[theorem_start..theorem_start + proof_start].to_owned()
}
