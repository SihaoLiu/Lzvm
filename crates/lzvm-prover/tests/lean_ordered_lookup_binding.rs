use std::path::Path;

#[path = "support/lean_binding.rs"]
mod lean_binding;

#[test]
fn lean_ordered_lookup_binding_exports_guarded_lookup_contracts() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_path = crate_root.join("../../lean/Lzvm/OrderedLookup.lean");
    let lean_source =
        std::fs::read_to_string(&lean_path).expect("Lean ordered lookup source should read");
    let top_level_path = crate_root.join("../../lean/Lzvm.lean");
    let top_level_source =
        std::fs::read_to_string(&top_level_path).expect("Lean top-level source should read");

    assert!(
        lean_binding::contains_import(&top_level_source, "Lzvm.OrderedLookup"),
        "top-level Lean module should import ordered lookup"
    );
    assert!(
        lean_source.contains("def guardedStageLookup")
            && lean_source.contains("def orderedStageSlotMatch")
            && lean_source.contains("def firstStageMatch"),
        "Lean ordered lookup should expose fast-path and fallback lookup models"
    );
    lean_binding::assert_theorem_declarations(
        &lean_source,
        &[
            "guarded_stage_lookup_uses_fallback_when_fast_path_declines",
            "guarded_stage_lookup_preserves_first_match_when_fast_path_matches",
            "guarded_stage_lookup_preserves_first_match",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "guarded_stage_lookup_uses_fallback_when_fast_path_declines",
        &["unfold guardedStageLookup", "rw [fastPathDeclines]"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "guarded_stage_lookup_preserves_first_match_when_fast_path_matches",
        &[
            "unfold guardedStageLookup",
            "rw [fastPathMatches]",
            "cases firstStageMatch entries stageIndexOf stageIndex",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "guarded_stage_lookup_preserves_first_match",
        &[
            "guarded_stage_lookup_uses_fallback_when_fast_path_declines",
            "guarded_stage_lookup_preserves_first_match_when_fast_path_matches",
        ],
    );
}
