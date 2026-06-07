use std::path::Path;

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
            && lean_source.contains("runtime_transcript_binding_checked_acceptance_sound")
            && lean_source
                .contains("runtime_transcript_binding_checked_acceptance_verifier_core_contract")
            && lean_source.contains("RuntimeVerifierCoreContract system publicInput proof")
            && lean_source.contains("SoundWitness system publicInput proof"),
        "Lean transcript binding should expose checked transcript soundness and verifier core projection"
    );
}
