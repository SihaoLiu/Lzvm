use std::path::Path;

#[path = "support/lean_binding.rs"]
mod lean_binding;

#[test]
fn lean_framed_guest_input_binding_exports_acceptance_core_sound_contract() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source =
        lean_binding::read_lean_source(crate_root, "../../lean/Lzvm/FramedGuestInputBinding.lean");
    let top_level_source = lean_binding::read_lean_source(crate_root, "../../lean/Lzvm.lean");

    assert!(
        lean_binding::contains_import(&top_level_source, "Lzvm.FramedGuestInputBinding"),
        "top-level Lean module should import framed guest input binding"
    );
    lean_binding::assert_theorem_declarations(
        &source,
        &["runtime_framed_guest_input_binding_checked_acceptance_accepts_evidence_core_and_sound"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &source,
        "runtime_framed_guest_input_binding_checked_acceptance_accepts_evidence_core_and_sound",
        &[
            "RuntimeFramedGuestInputBindingCheckedAcceptance",
            "system.accepts publicInput proof",
            "RuntimeFramedGuestInputBindingEvidence",
            "RuntimeFramedGuestInputBindingStructuralObligations",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &source,
        "runtime_framed_guest_input_binding_checked_acceptance_accepts_evidence_core_and_sound",
        &[
            "runtime_framed_guest_input_binding_checked_acceptance_eth_block_acceptance",
            "runtime_eth_block_public_input_binding_checked_acceptance_accepts_evidence_core_and_sound",
            "runtime_framed_guest_input_binding_checked_acceptance_full_contract",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &source,
        "runtime_framed_guest_input_binding_checked_acceptance_accepts_evidence_core_and_sound",
        &[
            "runtime_framed_guest_input_binding_checked_acceptance_sound",
            "runtime_eth_block_public_input_binding_checked_acceptance_evidence_core_and_sound",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
}
