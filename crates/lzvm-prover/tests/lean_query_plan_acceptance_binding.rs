use std::path::Path;

#[path = "support/lean_binding.rs"]
mod lean_binding;

#[test]
fn lean_query_plan_binding_exports_acceptance_core_sound_contract() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source = lean_binding::read_lean_source(
        crate_root,
        "../../lean/Lzvm/QueryPlanBinding/Soundness.lean",
    );
    let aggregate_source =
        lean_binding::read_lean_source(crate_root, "../../lean/Lzvm/QueryPlanBinding.lean");
    let top_level_source = lean_binding::read_lean_source(crate_root, "../../lean/Lzvm.lean");

    assert!(
        lean_binding::contains_import(&aggregate_source, "Lzvm.QueryPlanBinding.Soundness"),
        "query plan aggregate should import soundness"
    );
    assert!(
        lean_binding::contains_import(&top_level_source, "Lzvm.QueryPlanBinding"),
        "top-level Lean module should import query plan binding"
    );
    lean_binding::assert_theorem_declarations(
        &source,
        &["runtime_query_plan_binding_checked_acceptance_accepts_evidence_core_and_sound"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &source,
        "runtime_query_plan_binding_checked_acceptance_accepts_evidence_core_and_sound",
        &[
            "RuntimeQueryPlanBindingCheckedAcceptance",
            "system.accepts publicInput proof",
            "RuntimeQueryPlanBindingEvidence",
            "RuntimeChallengeSegmentBindingEvidence",
            "RuntimeOpeningSegmentBindingEvidence",
            "RuntimeOpeningEvidence",
            "system.transcriptBound publicInput proof",
            "system.pcsOpeningsValid publicInput proof",
            "system.friQueriesValid publicInput proof",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &source,
        "runtime_query_plan_binding_checked_acceptance_accepts_evidence_core_and_sound",
        &[
            "runtime_query_plan_binding_checked_acceptance_opening",
            "runtime_opening_segment_binding_checked_acceptance_accepts_evidence_core_and_sound",
            "runtime_query_plan_binding_checked_acceptance_evidence_core_and_sound",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &source,
        "runtime_query_plan_binding_checked_acceptance_accepts_evidence_core_and_sound",
        &[
            "runtime_query_plan_binding_checked_acceptance_sound",
            "runtime_opening_segment_binding_checked_acceptance_evidence_core_and_sound",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
}
