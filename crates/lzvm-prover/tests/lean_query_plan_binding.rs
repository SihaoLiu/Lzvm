use std::path::Path;

#[path = "support/lean_binding.rs"]
mod lean_binding;

#[test]
fn lean_query_plan_binding_exports_opening_segment_projections() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_path = crate_root.join("../../lean/Lzvm/QueryPlanBinding.lean");
    let lean_source =
        std::fs::read_to_string(&lean_path).expect("Lean query plan binding source should read");

    assert!(
        lean_source
            .contains("runtime_query_plan_binding_checked_acceptance_opening_segment_evidence")
            && lean_source.contains(
                "runtime_query_plan_binding_checked_acceptance_opening_segment_bound_contract"
            )
            && lean_source.contains("RuntimeOpeningSegmentBindingEvidence")
            && lean_source.contains("RuntimeOpeningSegmentBindingBoundContract")
            && lean_source.contains("RuntimeOpeningEvidence")
            && lean_source.contains("system.transcriptBound publicInput proof")
            && lean_source.contains("system.pcsOpeningsValid publicInput proof")
            && lean_source.contains("system.friQueriesValid publicInput proof")
            && lean_source.contains("RuntimeVerifierCoreContract system publicInput proof"),
        "Lean query plan binding should expose opening segment and verifier core projections"
    );
    lean_binding::assert_theorem_declarations(
        &lean_source,
        &[
            "runtime_query_plan_binding_evidence_implies_bound_contract",
            "runtime_query_plan_binding_checked_acceptance_bound_contract",
            "runtime_query_plan_binding_checked_acceptance_opening_segment_evidence",
            "runtime_query_plan_binding_checked_acceptance_opening_segment_bound_contract",
            "runtime_query_plan_binding_checked_acceptance_sound",
            "runtime_query_plan_binding_checked_acceptance_verifier_core_contract",
            "runtime_query_plan_binding_checked_acceptance_opening_and_core_contract",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_query_plan_binding_checked_acceptance_opening_and_core_contract",
        &[
            "RuntimeQueryPlanBindingBoundContract",
            "RuntimeOpeningSegmentBindingBoundContract",
            "RuntimeOpeningEvidence",
            "system.transcriptBound publicInput proof",
            "system.pcsOpeningsValid publicInput proof",
            "system.friQueriesValid publicInput proof",
            "RuntimeVerifierCoreContract system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_query_plan_binding_checked_acceptance_opening_and_core_contract",
        &[".right.right.right"],
    );
}
