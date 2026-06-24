use std::path::Path;

#[path = "support/lean_binding.rs"]
mod lean_binding;

#[test]
fn lean_soundness_binding_exports_abstract_soundness_theorems() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_path = crate_root.join("../../lean/Lzvm/Soundness.lean");
    let lean_source =
        std::fs::read_to_string(&lean_path).expect("Lean soundness source should read");
    let top_level_path = crate_root.join("../../lean/Lzvm.lean");
    let top_level_source =
        std::fs::read_to_string(&top_level_path).expect("Lean top-level source should read");

    assert!(
        top_level_source.contains("import Lzvm.Soundness"),
        "top-level Lean module should import abstract soundness"
    );
    assert!(
        lean_source.contains("ProofSystemSound system")
            && lean_source.contains("RequiredCryptographicAssumptionStatements assumptions.crypto")
            && lean_source.contains("RequiredSemanticAssumptionStatements assumptions.semantic"),
        "Lean abstract soundness should expose proof-system soundness and audited crypto/semantic assumptions"
    );
    lean_binding::assert_theorem_declarations(
        &lean_source,
        &[
            "abstract_verifier_sound",
            "abstract_verifier_sound_with_audited_assumptions",
            "abstract_verifier_sound_with_audited_soundness_obligations",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "abstract_verifier_sound_with_audited_soundness_obligations",
        &[
            "RequiredCryptographicAssumptionStatements assumptions.crypto",
            "RequiredSemanticAssumptionStatements assumptions.semantic",
            "ProofSystemSound system",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "abstract_verifier_sound_with_audited_soundness_obligations",
        &[
            "abstract_verifier_sound_with_audited_assumptions",
            "assumption_bundle_carries_required_semantic_evidence",
        ],
    );
}
