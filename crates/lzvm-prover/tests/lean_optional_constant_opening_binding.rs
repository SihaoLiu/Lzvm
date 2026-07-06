use std::path::Path;

#[path = "support/lean_binding.rs"]
mod lean_binding;

#[test]
fn lean_optional_constant_opening_binding_exports_contract_projections() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_path = crate_root.join("../../lean/Lzvm/OptionalConstantOpening.lean");
    let lean_source = std::fs::read_to_string(&lean_path)
        .expect("Lean optional constant opening source should read");
    let top_level_path = crate_root.join("../../lean/Lzvm.lean");
    let top_level_source =
        std::fs::read_to_string(&top_level_path).expect("Lean top-level source should read");

    assert!(
        lean_binding::contains_import(&top_level_source, "Lzvm.OptionalConstantOpening"),
        "top-level Lean module should import optional constant opening"
    );
    assert!(
        lean_source.contains("RuntimeOptionalConstantOpeningValidation")
            && lean_source.contains("RuntimeOptionalConstantOpeningContract")
            && lean_source.contains("RuntimeOptionalConstantOpeningAbsentSegmentContract")
            && lean_source.contains("RuntimeOptionalConstantOpeningZeroWidthQueryContract"),
        "Lean optional constant opening should expose checked contract projections"
    );
    lean_binding::assert_theorem_declarations(
        &lean_source,
        &[
            "runtime_optional_constant_opening_checked_acceptance_contract",
            "runtime_optional_constant_opening_checked_acceptance_absent_segment_contract",
            "runtime_optional_constant_opening_checked_acceptance_zero_width_query_contract",
            "runtime_optional_constant_opening_required_unit_has_matching_segment",
            "runtime_optional_constant_opening_present_segment_rejects_unexpected_units",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_optional_constant_opening_checked_acceptance_contract",
        &[
            "RuntimeOptionalConstantOpeningCheckedAcceptance",
            "RuntimeOptionalConstantOpeningContract",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_optional_constant_opening_checked_acceptance_contract",
        &[
            "runtime_opening_segment_binding_checked_acceptance_opening_bound_contract",
            "optionalConstantOpeningAcceptedImpliesConstantOpeningSegmentsValid",
            "optionalConstantOpeningAcceptedAndAbsentImpliesQueriedUnitZeroWidth",
            "optionalConstantOpeningAcceptedAndRequiresConstantsImpliesMatchingUnitPresent",
            "optionalConstantOpeningAcceptedAndZeroWidthImpliesVerifierConstantValuesEmpty",
            "optionalConstantOpeningAcceptedAndPresentImpliesUnexpectedUnitsRejected",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_optional_constant_opening_checked_acceptance_absent_segment_contract",
        &[
            "runtime_optional_constant_opening_checked_acceptance_contract",
            "optionalContract.right.right.left absent",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_optional_constant_opening_checked_acceptance_zero_width_query_contract",
        &[
            "runtime_optional_constant_opening_checked_acceptance_contract",
            "optionalContract.right.right.right.right.left",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_optional_constant_opening_required_unit_has_matching_segment",
        &[
            "runtime_optional_constant_opening_checked_acceptance_contract",
            "optionalContract.right.right.right.left unit queried requiresConstants",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_optional_constant_opening_present_segment_rejects_unexpected_units",
        &[
            "runtime_optional_constant_opening_checked_acceptance_contract",
            "optionalContract.right.right.right.right.right present",
        ],
    );
}
