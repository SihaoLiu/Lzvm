use std::path::Path;

#[path = "support/lean_binding.rs"]
mod lean_binding;

#[test]
fn lean_external_source_binding_exports_core_contract_projection() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_path = crate_root.join("../../lean/Lzvm/ExternalSource.lean");
    let lean_source =
        std::fs::read_to_string(&lean_path).expect("Lean external source should read");
    let top_level_path = crate_root.join("../../lean/Lzvm.lean");
    let top_level_source =
        std::fs::read_to_string(&top_level_path).expect("Lean top-level source should read");

    assert!(
        lean_binding::contains_import(&top_level_source, "Lzvm.ExternalSource"),
        "top-level Lean module should import external source"
    );
    assert!(
        lean_source.contains("ExternalSourceOpeningValidation")
            && lean_source.contains("ExternalSourceOpeningSoundnessObligations")
            && lean_source.contains("RuntimeVerifierCoreContract system publicInput proof")
            && lean_source.contains("SoundWitness system publicInput proof"),
        "Lean external source binding should expose checked soundness and verifier core projection"
    );
    lean_binding::assert_theorem_declarations(
        &lean_source,
        &[
            "external_source_opening_evidence_provider_transcript_bound",
            "external_source_opening_evidence_provider_matches_committed_trace",
            "external_source_opening_evidence_provider_openings_root_bound",
            "external_source_opening_evidence_implies_pcs_openings",
            "external_source_opening_requirement_from_evidence",
            "external_source_opening_requirement_not_required",
            "external_source_opening_requirement_implies_evidence",
            "external_source_opening_checked_acceptance_implies_pcs_openings",
            "external_source_opening_checked_acceptance_obligations",
            "external_source_opening_checked_acceptance_sound",
            "external_source_opening_checked_acceptance_verifier_core_contract",
            "external_source_opening_checked_acceptance_evidence_core_and_sound",
            "external_source_opening_checked_acceptance_accepts_evidence_core_and_sound",
            "external_source_opening_checked_acceptance_audited_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "external_source_opening_evidence_provider_transcript_bound",
        &["exact evidence.left"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "external_source_opening_evidence_provider_matches_committed_trace",
        &["exact evidence.right.left"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "external_source_opening_evidence_provider_openings_root_bound",
        &["exact evidence.right.right"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "external_source_opening_evidence_implies_pcs_openings",
        &[
            "ExternalSourceOpeningEvidence system validation publicInput proof",
            "system.pcsOpeningsValid publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "external_source_opening_evidence_implies_pcs_openings",
        &["validation.providerEvidenceImpliesPcsOpenings"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "external_source_opening_requirement_from_evidence",
        &["exact evidence"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "external_source_opening_requirement_not_required",
        &["False.elim (notRequired required)"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "external_source_opening_requirement_implies_evidence",
        &["exact requirement required"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "external_source_opening_checked_acceptance_implies_pcs_openings",
        &["external_source_opening_evidence_implies_pcs_openings"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "external_source_opening_checked_acceptance_obligations",
        &[
            "assumption_bundle_fiat_shamir_transcript_binding",
            "assumption_bundle_public_input_binding",
            "assumption_bundle_fri_query_soundness",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "external_source_opening_checked_acceptance_obligations",
        &[
            "assumptions.crypto.transcript_binding",
            "assumptions.semantic.public_input_binding",
            "assumptions.crypto.fri_query_sound",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "external_source_opening_checked_acceptance_sound",
        &["abstract_verifier_sound_with_semantic_evidence"],
    );
    lean_binding::assert_theorem_body_omits_identifier(
        &lean_source,
        "external_source_opening_checked_acceptance_sound",
        "abstract_verifier_sound",
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "external_source_opening_checked_acceptance_evidence_core_and_sound",
        &[
            "ExternalSourceOpeningCheckedAcceptance system validation publicInput proof",
            "ExternalSourceOpeningEvidence system validation publicInput proof",
            "system.pcsOpeningsValid publicInput proof",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "external_source_opening_checked_acceptance_evidence_core_and_sound",
        &[
            "external_source_opening_checked_acceptance_sound",
            "external_source_opening_checked_acceptance_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "external_source_opening_checked_acceptance_evidence_core_and_sound",
        &["sound_witness_implies_verifier_core_contract"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "external_source_opening_checked_acceptance_accepts_evidence_core_and_sound",
        &[
            "ExternalSourceOpeningCheckedAcceptance system validation publicInput proof",
            "system.accepts publicInput proof",
            "ExternalSourceOpeningEvidence system validation publicInput proof",
            "system.pcsOpeningsValid publicInput proof",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "external_source_opening_checked_acceptance_accepts_evidence_core_and_sound",
        &[
            "acceptedWithExternalSource.left",
            "external_source_opening_checked_acceptance_evidence_core_and_sound",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "external_source_opening_checked_acceptance_accepts_evidence_core_and_sound",
        &[
            "abstract_verifier_sound_with_semantic_evidence",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "external_source_opening_checked_acceptance_audited_core_contract",
        &[
            "RequiredCryptographicAssumptionStatements assumptions.crypto",
            "RequiredSemanticAssumptionStatements assumptions.semantic",
            "ExternalSourceOpeningEvidence system validation publicInput proof",
            "system.pcsOpeningsValid publicInput proof",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "external_source_opening_checked_acceptance_audited_core_contract",
        &[
            "assumption_bundle_carries_required_crypto_evidence",
            "assumption_bundle_carries_required_semantic_evidence",
            "external_source_opening_checked_acceptance_sound",
            "external_source_opening_checked_acceptance_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "external_source_opening_checked_acceptance_audited_core_contract",
        &[
            "assumption_bundle_carries_required_evidence",
            "external_source_opening_checked_acceptance_evidence_core_and_sound",
        ],
    );
}
