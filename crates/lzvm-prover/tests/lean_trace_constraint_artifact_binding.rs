use std::path::Path;

#[path = "support/lean_binding.rs"]
mod lean_binding;

#[test]
fn lean_trace_constraint_artifact_binding_exports_core_contract_projection() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_path = crate_root.join("../../lean/Lzvm/TraceConstraintArtifactBinding.lean");
    let lean_source = std::fs::read_to_string(&lean_path)
        .expect("Lean trace constraint artifact binding source should read");
    let top_level_path = crate_root.join("../../lean/Lzvm.lean");
    let top_level_source =
        std::fs::read_to_string(&top_level_path).expect("Lean top-level source should read");

    assert!(
        lean_binding::contains_import(&top_level_source, "Lzvm.TraceConstraintArtifactBinding"),
        "top-level Lean module should import trace constraint artifact binding"
    );
    assert!(
        lean_source.contains("RuntimeTraceConstraintArtifactBindingValidation")
            && lean_source.contains("RuntimeTraceConstraintPreflightBindingEvidence")
            && lean_source.contains("RuntimeTraceConstraintSemanticEvidenceComplete")
            && lean_source.contains("RuntimeTraceConstraintEvidence")
            && lean_source.contains("RuntimeTraceConstraintSoundnessObligations")
            && lean_source.contains("RuntimeTraceConstraintCheckedAcceptance")
            && lean_source.contains("RuntimeVerifierCoreContract system publicInput proof")
            && lean_source.contains("SoundWitness system publicInput proof"),
        "Lean trace constraint artifact binding should expose preflight evidence and verifier core clauses"
    );
    lean_binding::assert_theorem_declarations(
        &lean_source,
        &[
            "runtime_trace_constraint_artifact_binding_checked_acceptance_pcs_fri_backend_contract",
            "runtime_trace_constraint_artifact_binding_checked_acceptance_semantic_evidence_complete",
            "runtime_trace_constraint_preflight_binding_evidence_implies_payload_valid",
            "runtime_trace_constraint_preflight_binding_evidence_implies_witness_segments_match",
            "runtime_trace_constraint_preflight_binding_evidence_implies_constraint_catalog_matches",
            "runtime_trace_constraint_preflight_binding_evidence_implies_artifact_binding_evidence",
            "runtime_trace_constraint_artifact_binding_checked_acceptance_artifact_binding_evidence",
            "runtime_trace_constraint_artifact_binding_checked_acceptance_sound",
            "runtime_trace_constraint_artifact_binding_checked_acceptance_soundness_obligations",
            "runtime_trace_constraint_artifact_binding_checked_acceptance_verifier_core_contract",
            "runtime_trace_constraint_artifact_binding_checked_acceptance_evidence_core_and_sound",
            "runtime_trace_constraint_artifact_binding_checked_acceptance_accepts_evidence_core_and_sound",
            "runtime_trace_constraint_artifact_binding_checked_acceptance_audited_core_contract",
            "runtime_trace_constraint_artifact_binding_required_external_source_evidence_core_and_sound",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_trace_constraint_artifact_binding_checked_acceptance_pcs_fri_backend_contract",
        &[
            "RuntimeTraceConstraintPreflightBindingEvidence",
            "system.pcsOpeningsValid publicInput proof",
            "system.friQueriesValid publicInput proof",
            "RuntimeTraceConstraintArtifactBindingEvidence",
            "RuntimeTraceConstraintSemanticEvidenceComplete",
            "RuntimeTraceConstraintBackendContract",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_trace_constraint_artifact_binding_checked_acceptance_artifact_binding_evidence",
        &[
            "runtime_trace_constraint_artifact_binding_checked_acceptance_evidence",
            "runtime_trace_constraint_preflight_binding_evidence_implies_artifact_binding_evidence",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_trace_constraint_artifact_binding_checked_acceptance_semantic_evidence_complete",
        &["runtime_trace_constraint_checked_acceptance_semantic_evidence_complete"],
    );
    lean_binding::assert_theorem_prefix_omits(
        &lean_source,
        "runtime_trace_constraint_artifact_binding_checked_acceptance_pcs_fri_backend_contract",
        &["AssumptionBundle"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_trace_constraint_artifact_binding_checked_acceptance_verifier_core_contract",
        &[
            "runtime_trace_constraint_artifact_binding_checked_acceptance_trace_constraint",
            "runtime_trace_constraint_checked_acceptance_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_trace_constraint_artifact_binding_checked_acceptance_verifier_core_contract",
        &[
            "runtime_trace_constraint_artifact_binding_checked_acceptance_sound",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_trace_constraint_artifact_binding_checked_acceptance_evidence_core_and_sound",
        &[
            "RuntimeTraceConstraintArtifactBindingCheckedAcceptance",
            "RuntimeTraceConstraintPreflightBindingEvidence",
            "RuntimeTraceConstraintEvidence",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
            "requiresExternalSource",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_trace_constraint_artifact_binding_checked_acceptance_evidence_core_and_sound",
        &[
            "runtime_trace_constraint_artifact_binding_checked_acceptance_sound",
            "runtime_trace_constraint_artifact_binding_checked_acceptance_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_trace_constraint_artifact_binding_checked_acceptance_evidence_core_and_sound",
        &["sound_witness_implies_verifier_core_contract"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_trace_constraint_artifact_binding_checked_acceptance_accepts_evidence_core_and_sound",
        &[
            "RuntimeTraceConstraintArtifactBindingCheckedAcceptance",
            "system.accepts publicInput proof",
            "RuntimeTraceConstraintPreflightBindingEvidence",
            "RuntimeTraceConstraintEvidence",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
            "requiresExternalSource",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_trace_constraint_artifact_binding_checked_acceptance_accepts_evidence_core_and_sound",
        &[
            "runtime_trace_constraint_artifact_binding_checked_acceptance_trace_constraint",
            "runtime_trace_constraint_checked_acceptance_accepts_evidence_core_and_sound",
            "runtime_trace_constraint_artifact_binding_checked_acceptance_evidence_core_and_sound",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_trace_constraint_artifact_binding_checked_acceptance_accepts_evidence_core_and_sound",
        &[
            "runtime_trace_constraint_artifact_binding_checked_acceptance_sound",
            "runtime_trace_constraint_checked_acceptance_evidence_core_and_sound",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_trace_constraint_artifact_binding_checked_acceptance_audited_core_contract",
        &[
            "RequiredCryptographicAssumptionStatements assumptions.crypto",
            "RequiredSemanticAssumptionStatements assumptions.semantic",
            "RuntimeTraceConstraintPreflightBindingEvidence",
            "RuntimeTraceConstraintEvidence",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
            "requiresExternalSource",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_trace_constraint_artifact_binding_checked_acceptance_audited_core_contract",
        &[
            "assumption_bundle_carries_required_crypto_evidence",
            "assumption_bundle_carries_required_semantic_evidence",
            "runtime_trace_constraint_artifact_binding_checked_acceptance_sound",
            "runtime_trace_constraint_artifact_binding_checked_acceptance_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_trace_constraint_artifact_binding_checked_acceptance_audited_core_contract",
        &[
            "assumption_bundle_carries_required_evidence",
            "runtime_trace_constraint_artifact_binding_checked_acceptance_evidence_core_and_sound",
            "sound_witness_implies_verifier_core_contract",
            "abstract_verifier_sound",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_trace_constraint_artifact_binding_required_external_source_evidence_core_and_sound",
        &[
            "RuntimeTraceConstraintArtifactBindingCheckedAcceptance",
            "requiresExternalSource ->",
            "RuntimeTraceConstraintPreflightBindingEvidence",
            "RuntimeTraceConstraintEvidence",
            "ExternalSourceOpeningEvidence",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_trace_constraint_artifact_binding_required_external_source_evidence_core_and_sound",
        &[
            "runtime_trace_constraint_artifact_binding_checked_acceptance_evidence",
            "runtime_trace_constraint_artifact_binding_checked_acceptance_trace_constraint",
            "runtime_trace_constraint_required_external_source_evidence_core_and_sound",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_trace_constraint_artifact_binding_required_external_source_evidence_core_and_sound",
        &["sound_witness_implies_verifier_core_contract"],
    );
}
