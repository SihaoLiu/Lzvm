use std::path::Path;

#[path = "support/lean_binding.rs"]
mod lean_binding;

#[test]
fn lean_opening_segment_binding_exports_core_contract_projection() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_path = crate_root.join("../../lean/Lzvm/OpeningSegmentBinding.lean");
    let lean_source =
        std::fs::read_to_string(&lean_path).expect("Lean opening segment binding should read");
    let top_level_path = crate_root.join("../../lean/Lzvm.lean");
    let top_level_source =
        std::fs::read_to_string(&top_level_path).expect("Lean top-level source should read");

    assert!(
        top_level_source.contains("import Lzvm.OpeningSegmentBinding"),
        "top-level Lean module should import opening segment binding"
    );
    assert!(
        lean_source.contains("RuntimeOpeningSegmentBindingValidation")
            && lean_source.contains("RuntimeOpeningSegmentBindingBoundContract")
            && lean_source.contains("RuntimeOpeningEvidence")
            && lean_source.contains("RuntimeVerifierCoreContract system publicInput proof")
            && lean_source.contains("SoundWitness system publicInput proof"),
        "Lean opening segment binding should expose checked soundness and verifier core projection"
    );
    lean_binding::assert_theorem_declarations(
        &lean_source,
        &[
            "runtime_opening_segment_binding_evidence_implies_bound_contract",
            "runtime_opening_segment_binding_evidence_implies_fri_opening_checks",
            "runtime_opening_segment_binding_checked_acceptance_bound_contract",
            "runtime_opening_segment_binding_checked_acceptance_fri_opening_checks",
            "runtime_opening_segment_binding_checked_acceptance_sound",
            "runtime_opening_segment_binding_checked_acceptance_verifier_core_contract",
            "runtime_opening_segment_binding_checked_acceptance_opening_and_core_contract",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_opening_segment_binding_evidence_implies_fri_opening_checks",
        &[
            "validation.friOpeningSegmentsValid artifact publicInput proof",
            "validation.friFoldsValid artifact publicInput proof",
            "validation.verifierQueryOutputsValid artifact publicInput proof",
            "validation.openingValidation.friOpeningBound artifact publicInput proof",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_opening_segment_binding_checked_acceptance_fri_opening_checks",
        &[
            "RuntimeOpeningSegmentBindingCheckedAcceptance",
            "validation.friOpeningSegmentsValid artifact publicInput proof",
            "validation.friFoldsValid artifact publicInput proof",
            "validation.verifierQueryOutputsValid artifact publicInput proof",
            "validation.openingValidation.friOpeningBound artifact publicInput proof",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_opening_segment_binding_checked_acceptance_opening_and_core_contract",
        &[
            "RuntimeOpeningSegmentBindingBoundContract",
            "RuntimeOpeningEvidence",
            "RuntimeVerifierCoreContract system publicInput proof",
        ],
    );
}
