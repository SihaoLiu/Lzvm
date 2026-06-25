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
        top_level_source.contains("import Lzvm.ExternalSource"),
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
            "external_source_opening_checked_acceptance_obligations",
            "external_source_opening_checked_acceptance_sound",
            "external_source_opening_checked_acceptance_verifier_core_contract",
        ],
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
}
