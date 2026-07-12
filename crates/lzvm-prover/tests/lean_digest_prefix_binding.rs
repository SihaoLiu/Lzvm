use std::path::Path;

#[path = "support/lean_binding.rs"]
mod lean_binding;

#[test]
fn lean_digest_prefix_binding_exports_core_contract_projection() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_path = crate_root.join("../../lean/Lzvm/DigestPrefix.lean");
    let lean_source = std::fs::read_to_string(&lean_path).expect("Lean digest prefix should read");
    let top_level_path = crate_root.join("../../lean/Lzvm.lean");
    let top_level_source =
        std::fs::read_to_string(&top_level_path).expect("Lean top-level source should read");

    assert!(
        lean_binding::contains_import(&top_level_source, "Lzvm.DigestPrefix"),
        "top-level Lean module should import digest prefix"
    );
    assert!(
        lean_source.contains("RowMajorDigestPrefixValidation")
            && lean_source.contains("RowMajorDigestPrefixEvidence")
            && lean_source.contains("ColumnMajorDigestPrefixValidation")
            && lean_source.contains("ColumnMajorDigestPrefixEvidence")
            && lean_source.contains("RuntimeVerifierCoreContract system publicInput proof")
            && lean_source.contains("SoundWitness system publicInput proof"),
        "Lean digest prefix binding should expose checked soundness and verifier core projection"
    );
    lean_binding::assert_theorem_declarations(
        &lean_source,
        &[
            "digest_prefix_round_visible_words_eq_full_state_prefix",
            "digest_prefix_round_merkle_observation_eq_full_state",
            "row_major_digest_prefix_evidence_implies_wide_linear_digests",
            "row_major_digest_prefix_checked_acceptance_sound",
            "row_major_digest_prefix_checked_acceptance_verifier_core_contract",
            "row_major_digest_prefix_checked_acceptance_evidence_core_and_sound",
            "row_major_digest_prefix_checked_acceptance_accepts_evidence_core_and_sound",
            "row_major_digest_prefix_checked_acceptance_audited_core_contract",
            "column_major_layout_evidence_preserves_word_observation",
            "column_major_digest_prefix_evidence_projects_row_major_evidence",
            "column_major_digest_prefix_evidence_implies_wide_linear_digests",
            "column_major_digest_prefix_checked_acceptance_sound",
            "column_major_digest_prefix_checked_acceptance_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "digest_prefix_round_visible_words_eq_full_state_prefix",
        &[
            "evidence : DigestPrefixRoundEvidence α",
            "DigestPrefixRoundVisibleWords evidence =",
            "Poseidon2DigestPrefix evidence.fullStateWords",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "digest_prefix_round_visible_words_eq_full_state_prefix",
        &["evidence.digestWordsMatchFullStatePrefix"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "digest_prefix_round_merkle_observation_eq_full_state",
        &[
            "evidence : DigestPrefixRoundEvidence α",
            "DigestPrefixMerkleObservation (DigestPrefixRoundVisibleWords evidence)",
            "FullStateMerkleObservation evidence.fullStateWords",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "digest_prefix_round_merkle_observation_eq_full_state",
        &["evidence.digestWordsMatchFullStatePrefix"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "row_major_digest_prefix_evidence_implies_wide_linear_digests",
        &[
            "validation : RowMajorDigestPrefixValidation system",
            "RowMajorDigestPrefixEvidence system validation publicInput proof",
            "validation.leafValidation.wideLinearDigestsBindRows",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "row_major_digest_prefix_evidence_implies_wide_linear_digests",
        &[
            "validation.matchedPrefixImpliesWideLinearDigestsBindRows",
            "evidence.left",
            "evidence.right",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "row_major_digest_prefix_checked_acceptance_sound",
        &["abstract_verifier_sound_with_semantic_evidence"],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "row_major_digest_prefix_checked_acceptance_sound",
        &["sound_witness_implies_verifier_core_contract"],
    );
    lean_binding::assert_theorem_body_omits_identifier(
        &lean_source,
        "row_major_digest_prefix_checked_acceptance_sound",
        "abstract_verifier_sound",
    );
    lean_binding::assert_theorem_body_omits_identifier(
        &lean_source,
        "row_major_digest_prefix_checked_acceptance_sound",
        "sound_witness_implies_verifier_core_contract",
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "row_major_digest_prefix_checked_acceptance_verifier_core_contract",
        &[
            "assumption_bundle_fiat_shamir_transcript_binding",
            "assumption_bundle_public_input_binding",
            "assumption_bundle_pcs_opening_soundness",
            "assumption_bundle_fri_query_soundness",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "row_major_digest_prefix_checked_acceptance_verifier_core_contract",
        &[
            "row_major_digest_prefix_checked_acceptance_sound",
            "sound_witness_implies_verifier_core_contract",
            "assumptions.crypto.transcript_binding",
            "assumptions.semantic.public_input_binding",
            "assumptions.crypto.pcs_opening_sound",
            "assumptions.crypto.fri_query_sound",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "row_major_digest_prefix_checked_acceptance_evidence_core_and_sound",
        &[
            "RowMajorDigestPrefixEvidence system validation publicInput proof",
            "validation.leafValidation.wideLinearDigestsBindRows publicInput proof",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "row_major_digest_prefix_checked_acceptance_evidence_core_and_sound",
        &[
            "row_major_digest_prefix_checked_acceptance_sound",
            "row_major_digest_prefix_checked_acceptance_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "row_major_digest_prefix_checked_acceptance_evidence_core_and_sound",
        &["sound_witness_implies_verifier_core_contract"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "row_major_digest_prefix_checked_acceptance_accepts_evidence_core_and_sound",
        &[
            "system.accepts publicInput proof",
            "RowMajorDigestPrefixEvidence system validation publicInput proof",
            "validation.leafValidation.wideLinearDigestsBindRows publicInput proof",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "row_major_digest_prefix_checked_acceptance_accepts_evidence_core_and_sound",
        &[
            "row_major_digest_prefix_checked_acceptance_evidence_core_and_sound",
            "checked.left",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "row_major_digest_prefix_checked_acceptance_accepts_evidence_core_and_sound",
        &[
            "abstract_verifier_sound_with_semantic_evidence",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "row_major_digest_prefix_checked_acceptance_audited_core_contract",
        &[
            "RequiredCryptographicAssumptionStatements assumptions.crypto",
            "RequiredSemanticAssumptionStatements assumptions.semantic",
            "RowMajorDigestPrefixEvidence system validation publicInput proof",
            "validation.leafValidation.wideLinearDigestsBindRows publicInput proof",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "row_major_digest_prefix_checked_acceptance_audited_core_contract",
        &[
            "assumption_bundle_carries_required_crypto_evidence",
            "assumption_bundle_carries_required_semantic_evidence",
            "row_major_digest_prefix_evidence_implies_wide_linear_digests",
            "row_major_digest_prefix_checked_acceptance_verifier_core_contract",
            "abstract_verifier_sound_with_semantic_evidence",
            "checked.right",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "row_major_digest_prefix_checked_acceptance_audited_core_contract",
        &[
            "assumption_bundle_carries_required_evidence",
            "row_major_digest_prefix_checked_acceptance_evidence_core_and_sound",
            "row_major_digest_prefix_checked_acceptance_sound",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "column_major_layout_evidence_preserves_word_observation",
        &[
            "evidence : ColumnMajorLayoutEvidence α",
            "ColumnMajorMatrixWordAt",
            "RowMajorMatrixWordAt",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "column_major_layout_evidence_preserves_word_observation",
        &["evidence.wordsMatch row column rowBound columnBound"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "column_major_digest_prefix_evidence_projects_row_major_evidence",
        &["validation.layoutMatchImpliesRowMajorEvidence", "evidence"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "column_major_digest_prefix_evidence_implies_wide_linear_digests",
        &[
            "row_major_digest_prefix_evidence_implies_wide_linear_digests",
            "column_major_digest_prefix_evidence_projects_row_major_evidence",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "column_major_digest_prefix_checked_acceptance_sound",
        &[
            "column_major_digest_prefix_evidence_implies_wide_linear_digests",
            "abstract_verifier_sound_with_semantic_evidence",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "column_major_digest_prefix_checked_acceptance_verifier_core_contract",
        &[
            "assumption_bundle_fiat_shamir_transcript_binding",
            "assumption_bundle_public_input_binding",
            "assumption_bundle_pcs_opening_soundness",
            "assumption_bundle_fri_query_soundness",
        ],
    );
}
