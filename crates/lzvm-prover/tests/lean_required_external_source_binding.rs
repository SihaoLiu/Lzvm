use std::path::Path;

#[path = "support/lean_binding.rs"]
mod lean_binding;

#[test]
fn lean_required_external_source_binding_exports_core_contract_projection() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_path = crate_root.join("../../lean/Lzvm/RequiredExternalSource.lean");
    let lean_source =
        std::fs::read_to_string(&lean_path).expect("Lean required external source should read");
    let top_level_path = crate_root.join("../../lean/Lzvm.lean");
    let top_level_source =
        std::fs::read_to_string(&top_level_path).expect("Lean top-level source should read");

    assert!(
        top_level_source.contains("import Lzvm.RequiredExternalSource"),
        "top-level Lean module should import required external source"
    );
    assert!(
        lean_source.contains("runtime_guarded_external_source_required_evidence")
            && lean_source.contains("runtime_guarded_external_source_required_sound")
            && lean_source.contains("requiresExternalSource ->")
            && lean_source.contains("RuntimeVerifierCoreContract system publicInput proof")
            && lean_source.contains("SoundWitness system publicInput proof"),
        "Lean required external source binding should expose required evidence, soundness, and verifier core projection"
    );
    lean_binding::assert_theorem_declarations(
        &lean_source,
        &["runtime_guarded_external_source_required_verifier_core_contract"],
    );
}
