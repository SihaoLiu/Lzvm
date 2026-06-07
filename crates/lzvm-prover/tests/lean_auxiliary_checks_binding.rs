use std::path::Path;

#[test]
fn lean_auxiliary_checks_binding_exports_core_contract_projections() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_path = crate_root.join("../../lean/Lzvm/AuxiliaryChecks.lean");
    let lean_source =
        std::fs::read_to_string(&lean_path).expect("Lean auxiliary checks should read");
    let top_level_path = crate_root.join("../../lean/Lzvm.lean");
    let top_level_source =
        std::fs::read_to_string(&top_level_path).expect("Lean top-level source should read");

    assert!(
        top_level_source.contains("import Lzvm.AuxiliaryChecks"),
        "top-level Lean module should import auxiliary checks"
    );
    assert!(
        lean_source.contains("SourceLookupCheckedAcceptance")
            && lean_source.contains("source_lookup_auxiliary_acceptance_sound")
            && lean_source.contains("source_lookup_checked_acceptance_verifier_core_contract")
            && lean_source.contains("WitnessLeafDigestCheckedAcceptance")
            && lean_source.contains("witness_leaf_digest_acceptance_sound")
            && lean_source
                .contains("witness_leaf_digest_checked_acceptance_verifier_core_contract")
            && lean_source.contains("GpuCanonicalLeafCheckedAcceptance")
            && lean_source.contains("gpu_canonical_leaf_checked_acceptance_sound")
            && lean_source.contains("gpu_canonical_leaf_checked_acceptance_verifier_core_contract")
            && lean_source.contains("GpuSetupCheckedAcceptance")
            && lean_source.contains("gpu_setup_checked_acceptance_sound")
            && lean_source.contains("gpu_setup_checked_acceptance_verifier_core_contract")
            && lean_source.contains("GpuAllocationCheckedAcceptance")
            && lean_source.contains("gpu_allocation_checked_acceptance_sound")
            && lean_source.contains("gpu_allocation_checked_acceptance_verifier_core_contract")
            && lean_source.contains("RuntimeVerifierCoreContract system publicInput proof")
            && lean_source.contains("SoundWitness system publicInput proof"),
        "Lean auxiliary checks should expose checked soundness and verifier core projections"
    );
}
