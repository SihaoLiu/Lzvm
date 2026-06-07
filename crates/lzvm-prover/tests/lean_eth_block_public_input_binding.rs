use std::path::Path;

#[path = "support/lean_binding.rs"]
mod lean_binding;

#[test]
fn lean_eth_block_public_input_binding_exports_core_contract_projection() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_path = crate_root.join("../../lean/Lzvm/EthBlockPublicInputBinding.lean");
    let lean_source =
        std::fs::read_to_string(&lean_path).expect("Lean ETH binding source should read");
    let top_level_path = crate_root.join("../../lean/Lzvm.lean");
    let top_level_source =
        std::fs::read_to_string(&top_level_path).expect("Lean top-level source should read");

    assert!(
        top_level_source.contains("import Lzvm.EthBlockPublicInputBinding"),
        "top-level Lean module should import ETH block public-input binding"
    );
    assert!(
        lean_source.contains("RuntimeEthBlockPublicInputBindingValidation")
            && lean_source.contains("RuntimeEthBlockPublicInputBindingEvidence")
            && lean_source.contains("RuntimeProofArtifactBindingEvidence")
            && lean_source.contains("RuntimeArtifactEvidence")
            && lean_source.contains("RuntimeVerifierCoreContract system publicInput proof")
            && lean_source.contains("SoundWitness system publicInput proof"),
        "Lean ETH block public-input binding should expose checked evidence and verifier core clauses"
    );
    lean_binding::assert_theorem_declarations(
        &lean_source,
        &[
            "runtime_eth_block_public_input_binding_checked_acceptance_sound",
            "runtime_eth_block_public_input_binding_checked_acceptance_verifier_core_contract",
        ],
    );
}
