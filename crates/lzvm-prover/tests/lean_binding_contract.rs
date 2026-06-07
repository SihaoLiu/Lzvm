#[path = "support/lean_binding.rs"]
mod lean_binding;

use std::path::Path;

#[test]
fn lean_theorem_declaration_matching_ignores_comments_and_similar_names() {
    let source = r#"
/- theorem commented_out_verifier_core_contract -/
-- theorem line_commented_verifier_core_contract
theorem real_verifier_core_contract
    {system : VerifierModel} :
    RuntimeVerifierCoreContract system publicInput proof := by
  exact core
theorem real_verifier_core_contract_suffix :
    True := by
  trivial
"#;

    assert!(lean_binding::contains_theorem_declaration(
        source,
        "real_verifier_core_contract"
    ));
    lean_binding::assert_theorem_declarations(source, &["real_verifier_core_contract"]);
    assert!(!lean_binding::contains_theorem_declaration(
        source,
        "commented_out_verifier_core_contract"
    ));
    assert!(!lean_binding::contains_theorem_declaration(
        source,
        "line_commented_verifier_core_contract"
    ));
    assert!(!lean_binding::contains_theorem_declaration(
        source,
        "real_verifier_core_contract_suff"
    ));
}

#[test]
fn source_hot_paths_does_not_own_lean_binding_theorem_exports() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_hot_paths = std::fs::read_to_string(crate_root.join("tests/source_hot_paths.rs"))
        .expect("source hot paths test source should read");

    for theorem in [
        "runtime_eth_block_public_input_binding_checked_acceptance_sound",
        "runtime_eth_block_public_input_binding_checked_acceptance_verifier_core_contract",
        "runtime_challenge_segment_binding_checked_acceptance_sound",
        "runtime_challenge_segment_binding_checked_acceptance_verifier_core_contract",
        "runtime_trace_constraint_artifact_binding_checked_acceptance_sound",
        "runtime_trace_constraint_artifact_binding_checked_acceptance_verifier_core_contract",
        "runtime_trace_constraint_checked_acceptance_sound",
        "runtime_trace_constraint_evidence_implies_backend_contract",
        "runtime_trace_constraint_checked_acceptance_backend_contract",
        "runtime_opening_segment_binding_checked_acceptance_sound",
        "runtime_opening_segment_binding_evidence_implies_bound_contract",
        "runtime_opening_segment_binding_checked_acceptance_bound_contract",
        "runtime_opening_checked_acceptance_sound",
        "runtime_query_plan_binding_checked_acceptance_sound",
        "runtime_query_plan_binding_evidence_implies_bound_contract",
        "runtime_query_plan_binding_checked_acceptance_bound_contract",
        "runtime_pipeline_binding_checked_acceptance_sound",
    ] {
        assert!(
            !source_hot_paths.contains(theorem),
            "dedicated Lean binding tests should own theorem export check for {theorem}"
        );
    }
}

#[test]
fn pipeline_binding_uses_theorem_declaration_export_checks() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let pipeline_binding =
        std::fs::read_to_string(crate_root.join("tests/lean_pipeline_binding.rs"))
            .expect("pipeline binding test source should read");

    assert!(
        pipeline_binding.contains("lean_binding::assert_theorem_declarations"),
        "pipeline binding should use theorem declaration checks for exported theorem names"
    );
}

#[test]
fn retained_opening_bindings_use_theorem_declaration_export_checks() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));

    for test_source in [
        "lean_batch_opening_binding.rs",
        "lean_retained_leaf_digest_binding.rs",
        "lean_retained_parent_checkpoint_binding.rs",
    ] {
        let source = std::fs::read_to_string(crate_root.join("tests").join(test_source))
            .expect("Lean opening binding test source should read");

        assert!(
            source.contains("lean_binding::assert_theorem_declarations"),
            "{test_source} should use theorem declaration checks for exported theorem names"
        );
    }
}
