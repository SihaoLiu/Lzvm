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
    ] {
        assert!(
            !source_hot_paths.contains(theorem),
            "dedicated Lean binding tests should own theorem export check for {theorem}"
        );
    }
}
