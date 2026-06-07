use std::path::Path;

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
            && lean_source.contains("RuntimeOpeningSegmentBindingBoundContract"),
        "Lean query plan binding should expose direct opening segment evidence and bound contract projections"
    );
}
