use std::path::Path;

#[path = "support/lean_binding.rs"]
mod lean_binding;

#[test]
fn lean_query_plan_binding_exports_opening_segment_projections() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_path = crate_root.join("../../lean/Lzvm/QueryPlanBinding.lean");
    let lean_source =
        std::fs::read_to_string(&lean_path).expect("Lean query plan binding source should read");
    let query_plan_path = crate_root.join("src/pcs_query_plan.rs");
    let query_plan_source =
        std::fs::read_to_string(&query_plan_path).expect("PCS query plan source should read");
    let query_plan_build_path = crate_root.join("src/pcs_query_plan/build.rs");
    let query_plan_build_source = std::fs::read_to_string(&query_plan_build_path)
        .expect("PCS query plan build source should read");
    let fri_validation_path = crate_root.join("src/pcs_fri/validation.rs");
    let fri_validation_source =
        std::fs::read_to_string(&fri_validation_path).expect("FRI validation source should read");

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
    assert!(
        lean_source.contains("def RuntimeQueryPlanBindingSeededContract")
            && lean_source.contains("queryPlanSeedBindsWitnessTreeDigests")
            && lean_source.contains("queryPlanSeededFriOpeningRequirementsChecked"),
        "Lean query plan binding should expose seeded query-plan witness digest and FRI-opening checks"
    );
    lean_binding::assert_theorem_declarations(
        &lean_source,
        &[
            "runtime_query_plan_binding_evidence_implies_bound_contract",
            "runtime_query_plan_binding_checked_acceptance_bound_contract",
            "runtime_query_plan_binding_checked_acceptance_seeded_contract",
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
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_query_plan_binding_checked_acceptance_seeded_contract",
        &[
            "RuntimeQueryPlanBindingCheckedAcceptance",
            "RuntimeQueryPlanBindingSeededContract",
        ],
    );
    assert!(
        query_plan_source.contains("validate_seeded_pcs_query_plan_segments")
            && query_plan_source.contains("build_pcs_query_plan_segment_with_bindings")
            && query_plan_build_source.contains("hash_witness_commitment_segment_for_query_seed")
            && query_plan_build_source.contains("stage.tree_digest")
            && fri_validation_source.contains("seeded_query_plan_requires_fri_opening")
            && fri_validation_source.contains("fri_opening_required_units"),
        "runtime seeded query-plan validation should bind witness tree digests and require FRI openings for FRI-bearing seeded units"
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_query_plan_binding_checked_acceptance_opening_and_core_contract",
        &[".right.right.right"],
    );
}
