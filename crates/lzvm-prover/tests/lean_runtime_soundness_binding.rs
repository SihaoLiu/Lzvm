use std::path::Path;

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
            && lean_source.contains("runtime_soundness_checked_acceptance_sound")
            && lean_source.contains("runtime_soundness_checked_acceptance_core_obligations")
            && lean_source.contains("runtime_soundness_checked_acceptance_verifier_core_contract")
            && lean_source.contains("RuntimeVerifierCoreContract system publicInput proof")
            && lean_source.contains("SoundWitness system publicInput proof"),
        "Lean runtime soundness should expose checked soundness and verifier core projection"
    );
}
