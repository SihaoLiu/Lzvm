use std::path::Path;

#[test]
fn lean_opening_validation_binding_exports_core_contract_projection() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_path = crate_root.join("../../lean/Lzvm/OpeningValidation.lean");
    let lean_source =
        std::fs::read_to_string(&lean_path).expect("Lean opening validation should read");
    let top_level_path = crate_root.join("../../lean/Lzvm.lean");
    let top_level_source =
        std::fs::read_to_string(&top_level_path).expect("Lean top-level source should read");

    assert!(
        top_level_source.contains("import Lzvm.OpeningValidation"),
        "top-level Lean module should import opening validation"
    );
    assert!(
        lean_source.contains("RuntimeOpeningValidation")
            && lean_source.contains("runtime_opening_checked_acceptance_sound")
            && lean_source.contains("runtime_opening_checked_acceptance_verifier_core_contract")
            && lean_source.contains("runtime_opening_required_external_source_sound")
            && lean_source
                .contains("runtime_opening_required_external_source_verifier_core_contract")
            && lean_source.contains("requiresExternalSource ->")
            && lean_source.contains("RuntimeVerifierCoreContract system publicInput proof")
            && lean_source.contains("SoundWitness system publicInput proof"),
        "Lean opening validation should expose checked and required-source verifier core projections"
    );
}
