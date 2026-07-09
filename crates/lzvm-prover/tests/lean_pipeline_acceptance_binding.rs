use std::path::Path;

#[path = "support/lean_binding.rs"]
mod lean_binding;

#[test]
fn lean_pipeline_binding_exports_acceptance_core_sound_contract() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source = lean_binding::read_lean_source(
        crate_root,
        "../../lean/Lzvm/PipelineBinding/Obligations/Soundness.lean",
    );
    let aggregate_source =
        lean_binding::read_lean_source(crate_root, "../../lean/Lzvm/PipelineBinding.lean");
    let top_level_source = lean_binding::read_lean_source(crate_root, "../../lean/Lzvm.lean");

    assert!(
        lean_binding::contains_import(&aggregate_source, "Lzvm.PipelineBinding.Obligations"),
        "pipeline binding aggregate should import obligations"
    );
    assert!(
        lean_binding::contains_import(&top_level_source, "Lzvm.PipelineBinding"),
        "top-level Lean module should import pipeline binding"
    );
    lean_binding::assert_theorem_declarations(
        &source,
        &["runtime_pipeline_binding_checked_acceptance_accepts_evidence_core_and_sound"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &source,
        "runtime_pipeline_binding_checked_acceptance_accepts_evidence_core_and_sound",
        &[
            "RuntimePipelineBindingCheckedAcceptance",
            "system.accepts publicInput proof",
            "RuntimePipelineBindingEvidence",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &source,
        "runtime_pipeline_binding_checked_acceptance_accepts_evidence_core_and_sound",
        &[
            "runtime_pipeline_binding_checked_acceptance_verifier_accepts",
            "runtime_pipeline_binding_checked_acceptance_evidence_core_and_sound",
            "And.intro verifierAccepts evidenceCoreSound",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &source,
        "runtime_pipeline_binding_checked_acceptance_accepts_evidence_core_and_sound",
        &[
            "abstract_verifier_sound_with_semantic_evidence",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
}
