use std::path::Path;

#[path = "support/lean_binding.rs"]
mod lean_binding;

#[test]
fn lean_runtime_external_source_binding_exports_core_contract_projection() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_path = crate_root.join("../../lean/Lzvm/RuntimeExternalSource.lean");
    let lean_source =
        std::fs::read_to_string(&lean_path).expect("Lean runtime external source should read");
    let top_level_path = crate_root.join("../../lean/Lzvm.lean");
    let top_level_source =
        std::fs::read_to_string(&top_level_path).expect("Lean top-level source should read");

    assert!(
        top_level_source.contains("import Lzvm.RuntimeExternalSource"),
        "top-level Lean module should import runtime external source"
    );
    assert!(
        lean_source.contains("RuntimeExternalSourceCheckedAcceptance")
            && lean_source.contains("RuntimeGuardedExternalSourceCheckedAcceptance")
            && lean_source.contains("RuntimeVerifierCoreContract system publicInput proof")
            && lean_source.contains("SoundWitness system publicInput proof"),
        "Lean runtime external source binding should expose checked soundness and verifier core projections"
    );
    lean_binding::assert_theorem_declarations(
        &lean_source,
        &[
            "runtime_external_source_checked_acceptance_sound",
            "runtime_external_source_checked_acceptance_verifier_core_contract",
            "runtime_guarded_external_source_checked_acceptance_sound",
            "runtime_guarded_external_source_checked_acceptance_verifier_core_contract",
        ],
    );
}
