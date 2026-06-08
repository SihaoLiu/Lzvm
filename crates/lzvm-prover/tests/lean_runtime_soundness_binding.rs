use std::path::Path;

#[path = "support/lean_binding.rs"]
mod lean_binding;

#[test]
fn lean_runtime_soundness_binding_exports_core_contract_projection() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_path = crate_root.join("../../lean/Lzvm/RuntimeSoundness.lean");
    let lean_source =
        std::fs::read_to_string(&lean_path).expect("Lean runtime soundness source should read");
    let top_level_path = crate_root.join("../../lean/Lzvm.lean");
    let top_level_source =
        std::fs::read_to_string(&top_level_path).expect("Lean top-level source should read");

    assert!(
        top_level_source.contains("import Lzvm.RuntimeSoundness"),
        "top-level Lean module should import runtime soundness"
    );
    assert!(
        lean_source.contains("RuntimeSoundnessValidation")
            && lean_source.contains("RuntimeVerifierCoreContract system publicInput proof")
            && lean_source.contains("system.publicInputBound publicInput proof")
            && lean_source.contains("SoundWitness system publicInput proof"),
        "Lean runtime soundness should expose checked soundness and verifier core projection"
    );
    lean_binding::assert_theorem_declarations(
        &lean_source,
        &[
            "runtime_soundness_checked_acceptance_sound",
            "runtime_soundness_evidence_implies_runtime_artifact_evidence",
            "runtime_soundness_evidence_implies_transcript_bound",
            "runtime_soundness_evidence_implies_public_input_bound",
            "runtime_soundness_evidence_implies_core_obligations",
            "runtime_soundness_checked_acceptance_runtime_artifact_evidence",
            "runtime_soundness_checked_acceptance_transcript_bound",
            "runtime_soundness_checked_acceptance_public_input_bound",
            "runtime_soundness_checked_acceptance_pcs_and_fri",
            "runtime_soundness_checked_acceptance_core_obligations",
            "runtime_soundness_checked_acceptance_verifier_core_contract",
            "runtime_soundness_checked_acceptance_verifier_sound_witness",
            "runtime_soundness_checked_acceptance_execution_obligations",
            "runtime_soundness_required_external_source_pcs_sound",
            "runtime_soundness_required_external_source_verifier_core_contract",
        ],
    );
}
