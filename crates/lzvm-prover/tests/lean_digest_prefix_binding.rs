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
        top_level_source.contains("import Lzvm.DigestPrefix"),
        "top-level Lean module should import digest prefix"
    );
    assert!(
        lean_source.contains("RowMajorDigestPrefixValidation")
            && lean_source.contains("RowMajorDigestPrefixEvidence")
            && lean_source.contains("RuntimeVerifierCoreContract system publicInput proof")
            && lean_source.contains("SoundWitness system publicInput proof"),
        "Lean digest prefix binding should expose checked soundness and verifier core projection"
    );
    lean_binding::assert_theorem_declarations(
        &lean_source,
        &[
            "row_major_digest_prefix_checked_acceptance_sound",
            "row_major_digest_prefix_checked_acceptance_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "row_major_digest_prefix_checked_acceptance_sound",
        &["abstract_verifier_sound"],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "row_major_digest_prefix_checked_acceptance_sound",
        &["sound_witness_implies_verifier_core_contract"],
    );
    lean_binding::assert_theorem_body_contains_identifier(
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
}
