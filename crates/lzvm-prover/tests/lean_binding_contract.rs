#[path = "support/lean_binding.rs"]
mod lean_binding;

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
