use std::path::Path;

#[path = "support/lean_binding.rs"]
mod lean_binding;

#[test]
fn lean_conformance_binding_exports_core_contract_projection() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_path = crate_root.join("../../lean/Lzvm/Conformance.lean");
    let lean_source =
        std::fs::read_to_string(&lean_path).expect("Lean conformance source should read");
    let top_level_path = crate_root.join("../../lean/Lzvm.lean");
    let top_level_source =
        std::fs::read_to_string(&top_level_path).expect("Lean top-level source should read");

    assert!(
        lean_binding::contains_import(&top_level_source, "Lzvm.Conformance"),
        "top-level Lean module should import conformance"
    );
    assert!(
        lean_source.contains("RuntimeConformanceValidation")
            && lean_source.contains("RuntimeArtifactSoundnessObligations")
            && lean_source.contains("RuntimeVerifierCoreContract system publicInput proof")
            && lean_source.contains("SoundWitness system publicInput proof"),
        "Lean runtime conformance should expose checked soundness and verifier core projection"
    );
    lean_binding::assert_theorem_declarations(
        &lean_source,
        &[
            "runtime_artifact_checked_acceptance_sound",
            "runtime_artifact_checked_acceptance_verifier_core_contract",
            "runtime_artifact_checked_acceptance_audited_sound",
            "runtime_conformance_agreement_checked_acceptance_audited_sound",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_artifact_checked_acceptance_obligations",
        &[
            "assumption_bundle_fiat_shamir_transcript_binding",
            "assumption_bundle_pcs_opening_soundness",
            "assumption_bundle_fri_query_soundness",
            "assumption_bundle_public_input_binding",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_artifact_checked_acceptance_obligations",
        &[
            "assumptions.crypto.transcript_binding",
            "assumptions.crypto.pcs_opening_sound",
            "assumptions.crypto.fri_query_sound",
            "assumptions.semantic.public_input_binding",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_artifact_checked_acceptance_sound",
        &[
            "runtime_artifact_checked_acceptance_obligations",
            "abstract_verifier_sound",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_artifact_checked_acceptance_sound",
        &["sound_witness_implies_verifier_core_contract"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_artifact_checked_acceptance_verifier_core_contract",
        &["runtime_artifact_checked_acceptance_obligations"],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_artifact_checked_acceptance_verifier_core_contract",
        &[
            "runtime_artifact_checked_acceptance_sound",
            "sound_witness_implies_verifier_core_contract",
            "abstract_verifier_sound",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_artifact_checked_acceptance_audited_sound",
        &[
            "RuntimeArtifactCheckedAcceptance system validation artifact publicInput proof",
            "RequiredCryptographicAssumptionStatements assumptions.crypto",
            "RequiredSemanticAssumptionStatements assumptions.semantic",
            "RuntimeArtifactEvidence system validation artifact publicInput proof",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_artifact_checked_acceptance_audited_sound",
        &[
            "runtime_artifact_checked_acceptance_implies_verifier_accepts",
            "runtime_artifact_checked_acceptance_evidence",
            "accepted_proof_audited_core_and_sound_witness",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_conformance_agreement_checked_acceptance_audited_sound",
        &[
            "RuntimeConformanceValidationAgreement left right",
            "RuntimeArtifactCheckedAcceptance system left artifact publicInput proof",
            "RequiredCryptographicAssumptionStatements assumptions.crypto",
            "RequiredSemanticAssumptionStatements assumptions.semantic",
            "RuntimeArtifactEvidence system right artifact publicInput proof",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_conformance_agreement_checked_acceptance_audited_sound",
        &[
            "runtime_conformance_agreement_checked_acceptance_iff",
            "runtime_artifact_checked_acceptance_audited_sound",
        ],
    );
}
