use std::path::Path;

#[path = "support/lean_binding.rs"]
mod lean_binding;

#[test]
fn lean_framed_guest_input_binding_exports_soundness_structural_contract() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_path = crate_root.join("../../lean/Lzvm/FramedGuestInputBinding.lean");
    let lean_source =
        std::fs::read_to_string(&lean_path).expect("Lean framed input source should read");

    lean_binding::assert_theorem_declarations(
        &lean_source,
        &[
            "runtime_framed_guest_input_binding_checked_acceptance_soundness_and_structural_contract",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_framed_guest_input_binding_checked_acceptance_soundness_and_structural_contract",
        &[
            "RuntimeFramedGuestInputBindingSoundnessContract",
            "RuntimeFramedGuestInputBindingStructuralObligations",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_framed_guest_input_binding_checked_acceptance_soundness_and_structural_contract",
        &[
            "runtime_framed_guest_input_binding_checked_acceptance_soundness_contract",
            "runtime_framed_guest_input_binding_checked_acceptance_structural_obligations",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_framed_guest_input_binding_checked_acceptance_soundness_and_structural_contract",
        &["abstract_verifier_sound"],
    );
}
