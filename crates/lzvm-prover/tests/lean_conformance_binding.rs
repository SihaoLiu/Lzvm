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
        lean_binding::contains_import(&lean_source, "Lzvm.GuestReportMemoryAccess")
            && lean_binding::contains_import(&lean_source, "Lzvm.GuestReportRegisterWrite"),
        "Lean runtime conformance should import the guest report storage models"
    );
    assert!(
        lean_source.contains("RuntimeConformanceValidation")
            && lean_source.contains("artifactPublicInputMatchesImpliesPublicInputBound")
            && lean_source.contains("RuntimeArtifactSoundnessObligations")
            && lean_source.contains("RuntimeGuestReportStorageEvidence")
            && lean_source.contains("RuntimeGuestReportStorageLogicalViews")
            && lean_source.contains("CompactGuestRegisterWriteCanonical")
            && lean_source.contains("FoldedGuestMemoryEffectsCanonical")
            && lean_source.contains("RuntimeVerifierCoreContract system publicInput proof")
            && lean_source.contains("SoundWitness system publicInput proof"),
        "Lean runtime conformance should expose report storage evidence, checked soundness, and verifier core projection"
    );
    lean_binding::assert_theorem_declarations(
        &lean_source,
        &[
            "runtime_guest_report_storage_evidence_register_writes",
            "runtime_guest_report_storage_evidence_register_write_length_le_one",
            "runtime_guest_report_storage_evidence_normal_memory_accesses",
            "runtime_guest_report_storage_evidence_precompile_memory_accesses",
            "runtime_guest_report_storage_evidence_precompile_result",
            "runtime_guest_report_storage_evidence_logical_views",
            "runtime_conformance_agreement_evidence_iff",
            "runtime_artifact_evidence_implies_public_input_bound",
            "runtime_artifact_checked_acceptance_crypto_obligations",
            "runtime_artifact_checked_acceptance_sound",
            "runtime_artifact_checked_acceptance_crypto_verifier_core_contract",
            "runtime_artifact_checked_acceptance_verifier_core_contract",
            "runtime_artifact_checked_acceptance_evidence_core_and_sound",
            "runtime_artifact_checked_acceptance_accepts_evidence_core_and_sound",
            "runtime_artifact_checked_acceptance_audited_sound",
            "runtime_artifact_checked_acceptance_audited_core_contract",
            "runtime_conformance_agreement_checked_acceptance_sound",
            "runtime_conformance_agreement_checked_acceptance_audited_sound",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_guest_report_storage_evidence_logical_views",
        &[
            "runtime_guest_report_storage_evidence_register_writes",
            "runtime_guest_report_storage_evidence_normal_memory_accesses",
            "runtime_guest_report_storage_evidence_precompile_memory_accesses",
            "runtime_guest_report_storage_evidence_precompile_result",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_guest_report_storage_evidence_register_write_length_le_one",
        &["compact_guest_register_write_canonical_length_le_one"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_artifact_checked_acceptance_crypto_obligations",
        &[
            "cryptographic_assumptions_carry_required_evidence",
            "required_crypto_assumptions_fiat_shamir_transcript_binding",
            "required_crypto_assumptions_pcs_opening_soundness",
            "required_crypto_assumptions_fri_query_soundness",
            "runtime_artifact_evidence_implies_public_input_bound",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_artifact_checked_acceptance_crypto_obligations",
        &[
            "assumption_bundle_public_input_binding",
            "SemanticAssumptions",
            "assumptions.semantic",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_artifact_checked_acceptance_obligations",
        &["runtime_artifact_checked_acceptance_crypto_obligations"],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_artifact_checked_acceptance_obligations",
        &["assumption_bundle_public_input_binding"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_artifact_checked_acceptance_sound",
        &[
            "runtime_artifact_checked_acceptance_obligations",
            "abstract_verifier_sound_with_semantic_evidence",
        ],
    );
    lean_binding::assert_theorem_body_omits_identifier(
        &lean_source,
        "runtime_artifact_checked_acceptance_sound",
        "abstract_verifier_sound",
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_artifact_checked_acceptance_sound",
        &["sound_witness_implies_verifier_core_contract"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_artifact_checked_acceptance_verifier_core_contract",
        &["runtime_artifact_checked_acceptance_crypto_verifier_core_contract"],
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
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_artifact_checked_acceptance_crypto_verifier_core_contract",
        &["runtime_artifact_checked_acceptance_crypto_obligations"],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_artifact_checked_acceptance_crypto_verifier_core_contract",
        &[
            "runtime_artifact_checked_acceptance_sound",
            "abstract_verifier_sound",
            "assumptions.semantic",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_artifact_checked_acceptance_evidence_core_and_sound",
        &[
            "RuntimeArtifactCheckedAcceptance system validation artifact publicInput proof",
            "RuntimeArtifactEvidence system validation artifact publicInput proof",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_artifact_checked_acceptance_evidence_core_and_sound",
        &["runtime_artifact_checked_acceptance_sound"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_artifact_checked_acceptance_accepts_evidence_core_and_sound",
        &[
            "RuntimeArtifactCheckedAcceptance system validation artifact publicInput proof",
            "system.accepts publicInput proof",
            "RuntimeArtifactEvidence system validation artifact publicInput proof",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_artifact_checked_acceptance_accepts_evidence_core_and_sound",
        &[
            "runtime_artifact_checked_acceptance_implies_verifier_accepts",
            "runtime_artifact_checked_acceptance_evidence_core_and_sound",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_artifact_checked_acceptance_accepts_evidence_core_and_sound",
        &[
            "runtime_artifact_checked_acceptance_sound",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_artifact_checked_acceptance_audited_sound",
        &[
            "RuntimeArtifactCheckedAcceptance system validation artifact publicInput proof",
            "RequiredCryptographicAssumptionStatements assumptions.crypto",
            "RequiredSemanticAssumptionStatements assumptions.semantic",
            "RuntimeArtifactSoundnessObligations",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_artifact_checked_acceptance_audited_sound",
        &[
            "assumption_bundle_carries_required_crypto_evidence",
            "assumption_bundle_carries_required_semantic_evidence",
            "runtime_artifact_checked_acceptance_sound",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_artifact_checked_acceptance_audited_sound",
        &["assumption_bundle_carries_required_evidence"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_artifact_checked_acceptance_audited_core_contract",
        &[
            "RequiredCryptographicAssumptionStatements assumptions.crypto",
            "RequiredSemanticAssumptionStatements assumptions.semantic",
            "RuntimeArtifactEvidence system validation artifact publicInput proof",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_artifact_checked_acceptance_audited_core_contract",
        &[
            "runtime_artifact_checked_acceptance_audited_sound",
            "obligations.left",
            "obligations.right.right",
            "audited.right.right.right",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_artifact_checked_acceptance_audited_core_contract",
        &[
            "runtime_artifact_checked_acceptance_verifier_core_contract",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_conformance_agreement_evidence_iff",
        &[
            "RuntimeConformanceValidationAgreement left right",
            "RuntimeArtifactEvidence system left artifact publicInput proof <->",
            "RuntimeArtifactEvidence system right artifact publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_conformance_agreement_evidence_iff",
        &[
            "(agreement.right.left artifact publicInput proof).mp evidence.left",
            "(agreement.right.right artifact publicInput proof).mp evidence.right",
            "(agreement.right.left artifact publicInput proof).mpr evidence.left",
            "(agreement.right.right artifact publicInput proof).mpr evidence.right",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_conformance_agreement_checked_acceptance_sound",
        &[
            "RuntimeConformanceValidationAgreement left right",
            "RuntimeArtifactCheckedAcceptance system left artifact publicInput proof",
            "RuntimeArtifactSoundnessObligations",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_conformance_agreement_checked_acceptance_sound",
        &[
            "runtime_conformance_agreement_checked_acceptance_iff",
            "runtime_artifact_checked_acceptance_sound",
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
            "RuntimeArtifactSoundnessObligations",
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
