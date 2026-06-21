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
fn lean_identifier_occurrence_count_ignores_similar_names_comments_and_strings() {
    let source = r#"
-- abstract_verifier_sound
def abstract_verifier_sound_mutation_guard := True
def uses_exact_symbol := abstract_verifier_sound assumptions publicInput proof checked.left
def string_literal := "abstract_verifier_sound"
"#;

    assert_eq!(
        lean_binding::visible_identifier_occurrence_count(source, "abstract_verifier_sound"),
        1
    );
}

#[test]
fn lean_identifier_occurrence_count_preserves_code_after_comment_markers_in_strings() {
    let source = r#"
-- abstract_verifier_sound
def abstract_verifier_sound_mutation_guard := True
def uses_exact_symbol := ("-- not a comment", abstract_verifier_sound assumptions publicInput proof checked.left)
"#;

    assert_eq!(
        lean_binding::visible_identifier_occurrence_count(source, "abstract_verifier_sound"),
        1
    );
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

#[test]
fn auxiliary_checked_acceptance_chokepoints_use_identifier_body_pins() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let auxiliary_source =
        std::fs::read_to_string(crate_root.join("../../lean/Lzvm/AuxiliaryChecks.lean"))
            .expect("Lean auxiliary checks source should read");
    let binding_source =
        std::fs::read_to_string(crate_root.join("tests/lean_auxiliary_checks_binding.rs"))
            .expect("Lean auxiliary checks binding test source should read");

    let chokepoints = auxiliary_source
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            let rest = trimmed.strip_prefix("theorem auxiliary_checked_acceptance_")?;
            rest.split_whitespace()
                .next()
                .map(|name| format!("auxiliary_checked_acceptance_{name}"))
        })
        .collect::<Vec<_>>();
    assert!(
        !chokepoints.is_empty(),
        "Lean auxiliary checks should declare checked acceptance chokepoints"
    );

    for chokepoint in chokepoints {
        assert!(
            binding_test_calls_identifier_body_pin(
                &binding_source,
                "assert_theorem_body_contains_identifier(",
                &chokepoint,
            ),
            "Lean auxiliary checked acceptance chokepoint {chokepoint} should have an identifier-level body contains pin"
        );
        assert!(
            binding_test_calls_identifier_body_pin(
                &binding_source,
                "assert_theorem_body_omits_identifier(",
                &chokepoint,
            ),
            "Lean auxiliary checked acceptance chokepoint {chokepoint} should have an identifier-level body omits pin"
        );
    }
}

fn binding_test_calls_identifier_body_pin(source: &str, call: &str, theorem: &str) -> bool {
    source.match_indices(call).any(|(start, _)| {
        source[start..]
            .lines()
            .take(6)
            .any(|line| line.contains(&format!("\"{theorem}\"")))
    })
}

#[test]
fn lean_soundness_sources_stay_modular() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_root = crate_root.join("../../lean/Lzvm");
    let mut oversized = Vec::new();
    collect_oversized_lean_sources(&lean_root, &mut oversized);

    assert!(
        oversized.is_empty(),
        "Lean soundness sources should stay at or below 1800 lines: {oversized:?}"
    );
}

#[test]
fn lean_soundness_sources_do_not_use_uncontrolled_placeholders() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_root = crate_root.join("../../lean/Lzvm");

    lean_binding::assert_no_uncontrolled_lean_placeholders(&lean_root);
}

#[test]
fn top_level_lean_module_reaches_all_soundness_sources() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_entrypoint = crate_root.join("../../lean/Lzvm.lean");
    let lean_root = crate_root.join("../../lean/Lzvm");

    lean_binding::assert_all_lean_modules_reachable_from_entrypoint(&lean_entrypoint, &lean_root);
}

fn collect_oversized_lean_sources(path: &Path, oversized: &mut Vec<(String, usize)>) {
    for entry in std::fs::read_dir(path).expect("Lean source directory should read") {
        let entry = entry.expect("Lean source entry should read");
        let path = entry.path();
        if path.is_dir() {
            collect_oversized_lean_sources(&path, oversized);
            continue;
        }
        if path.extension().and_then(|extension| extension.to_str()) != Some("lean") {
            continue;
        }
        let source = std::fs::read_to_string(&path).expect("Lean source should read");
        let line_count = source.lines().count();
        if line_count > 1800 {
            oversized.push((path.display().to_string(), line_count));
        }
    }
}
