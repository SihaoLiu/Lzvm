use std::path::Path;

#[path = "support/lean_binding.rs"]
mod lean_binding;

#[test]
fn lean_conformance_binding_exports_core_contract_projection() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_path = crate_root.join("../../lean/Lzvm/Conformance.lean");
    let lean_source =
        std::fs::read_to_string(&lean_path).expect("Lean conformance source should read");
    let top_level_path = crate_root.join("../../lean/Lzvm.lean");
    let top_level_source =
        std::fs::read_to_string(&top_level_path).expect("Lean top-level source should read");

    assert!(
        top_level_source.contains("import Lzvm.Conformance"),
        "top-level Lean module should import conformance"
    );
    assert!(
        lean_source.contains("RuntimeConformanceValidation")
            && lean_source.contains("RuntimeArtifactSoundnessObligations")
            && lean_source.contains("RuntimeVerifierCoreContract system publicInput proof")
            && lean_source.contains("SoundWitness system publicInput proof"),
        "Lean runtime conformance should expose checked soundness and verifier core projection"
    );
    lean_binding::assert_theorem_declarations(
        &lean_source,
        &[
            "runtime_artifact_checked_acceptance_sound",
            "runtime_artifact_checked_acceptance_verifier_core_contract",
        ],
    );
}
