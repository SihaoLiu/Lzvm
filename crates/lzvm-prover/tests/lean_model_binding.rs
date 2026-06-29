use std::path::Path;

#[path = "support/lean_binding.rs"]
mod lean_binding;

#[test]
fn lean_model_exports_verifier_core_contract() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let model_path = crate_root.join("../../lean/Lzvm/Model.lean");
    let model_source = std::fs::read_to_string(&model_path).expect("Lean model source should read");
    let top_level_path = crate_root.join("../../lean/Lzvm.lean");
    let top_level_source =
        std::fs::read_to_string(&top_level_path).expect("Lean top-level source should read");

    assert!(
        lean_binding::contains_import(&top_level_source, "Lzvm.Model"),
        "top-level Lean module should import the verifier model"
    );
    assert!(
        model_source.contains("structure VerifierModel where")
            && model_source.contains("structure Proof where")
            && model_source.contains("segmentIds : List Nat := []")
            && model_source.contains("def RuntimeVerifierCoreContract")
            && model_source.contains("def SoundWitness")
            && model_source.contains("def ProofSystemSound")
            && model_source.contains("system.accepts publicInput proof -> SoundWitness system publicInput proof"),
        "Lean model should expose proof segment IDs, verifier acceptance, core contract, sound witnesses, and proof-system soundness"
    );
    assert!(
        model_source.contains("system.transcriptBound publicInput proof")
            && model_source.contains("system.publicInputBound publicInput proof")
            && model_source.contains("system.pcsOpeningsValid publicInput proof")
            && model_source.contains("system.friQueriesValid publicInput proof"),
        "Lean model core contract should keep transcript, public input, PCS, and FRI obligations explicit"
    );
    lean_binding::assert_theorem_declarations(
        &model_source,
        &[
            "sound_witness_implies_verifier_core_contract",
            "sound_witness_implies_execution_obligations",
        ],
    );
}
