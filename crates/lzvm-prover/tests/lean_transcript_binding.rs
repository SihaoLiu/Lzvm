use std::path::Path;

#[path = "support/lean_binding.rs"]
mod lean_binding;

#[test]
fn lean_transcript_binding_exports_core_contract_projection() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_path = crate_root.join("../../lean/Lzvm/TranscriptBinding.lean");
    let lean_source =
        std::fs::read_to_string(&lean_path).expect("Lean transcript binding source should read");
    let top_level_path = crate_root.join("../../lean/Lzvm.lean");
    let top_level_source =
        std::fs::read_to_string(&top_level_path).expect("Lean top-level source should read");

    assert!(
        top_level_source.contains("import Lzvm.TranscriptBinding"),
        "top-level Lean module should import transcript binding"
    );
    assert!(
        lean_source.contains("RuntimeTranscriptBindingValidation")
            && lean_source.contains("RuntimeTranscriptBindingEvidence")
            && lean_source.contains("RuntimeArtifactEvidence")
            && lean_source.contains("system.transcriptBound publicInput proof")
            && lean_source.contains("RuntimeVerifierCoreContract system publicInput proof")
            && lean_source.contains("SoundWitness system publicInput proof"),
        "Lean transcript binding should expose checked transcript soundness and verifier core projection"
    );
    lean_binding::assert_theorem_declarations(
        &lean_source,
        &[
            "runtime_transcript_binding_checked_acceptance_sound",
            "runtime_transcript_binding_checked_acceptance_verifier_core_contract",
            "runtime_transcript_binding_checked_acceptance_transcript_and_core_contract",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_transcript_binding_checked_acceptance_transcript_and_core_contract",
        &[
            "RuntimeTranscriptBindingEvidence",
            "RuntimeArtifactEvidence",
            "system.transcriptBound publicInput proof",
            "RuntimeVerifierCoreContract system publicInput proof",
        ],
    );
}
