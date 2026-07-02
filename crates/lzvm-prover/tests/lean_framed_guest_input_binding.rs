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
            "runtime_framed_guest_input_binding_checked_acceptance_concrete_segment_ids_allowed",
            "runtime_framed_guest_input_binding_checked_acceptance_runtime_shape_contract",
            "runtime_framed_guest_input_binding_checked_acceptance_soundness_and_structural_contract",
            "runtime_framed_guest_input_binding_checked_acceptance_concrete_core_sound_contract",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_framed_guest_input_binding_checked_acceptance_concrete_segment_ids_allowed",
        &[
            "RuntimeFramedGuestInputBindingCheckedAcceptance",
            "RuntimeProofArtifactConcreteSegmentIdBinding",
            "RuntimeProofArtifactConcreteSegmentIdsAllowed proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_framed_guest_input_binding_checked_acceptance_concrete_segment_ids_allowed",
        &[
            "runtime_framed_guest_input_binding_checked_acceptance_eth_block_acceptance",
            "runtime_eth_block_public_input_binding_checked_acceptance_concrete_segment_ids_allowed",
        ],
    );
    lean_binding::assert_theorem_prefix_omits(
        &lean_source,
        "runtime_framed_guest_input_binding_checked_acceptance_concrete_segment_ids_allowed",
        &["AssumptionBundle"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_framed_guest_input_binding_checked_acceptance_runtime_shape_contract",
        &[
            "RuntimeFramedGuestInputBindingCheckedAcceptance",
            "RuntimeFramedGuestInputBindingStructuralObligations",
            "RuntimeProofArtifactConcreteSegmentIdsAllowed proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_framed_guest_input_binding_checked_acceptance_runtime_shape_contract",
        &[
            "runtime_framed_guest_input_binding_checked_acceptance_structural_obligations",
            "runtime_framed_guest_input_binding_checked_acceptance_concrete_segment_ids_allowed",
        ],
    );
    lean_binding::assert_theorem_prefix_omits(
        &lean_source,
        "runtime_framed_guest_input_binding_checked_acceptance_runtime_shape_contract",
        &["AssumptionBundle"],
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
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_framed_guest_input_binding_checked_acceptance_concrete_core_sound_contract",
        &[
            "RuntimeFramedGuestInputBindingCheckedAcceptance",
            "RuntimeProofArtifactConcreteSegmentIdBinding",
            "validation.ethBlockValidation.proofArtifactBindingValidation",
            "RuntimeFramedGuestInputBindingEvidence",
            "RuntimeFramedGuestInputBindingStructuralObligations",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
            "RuntimeProofArtifactConcreteSegmentIdsAllowed proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_framed_guest_input_binding_checked_acceptance_concrete_core_sound_contract",
        &[
            "runtime_framed_guest_input_binding_checked_acceptance_full_contract",
            "runtime_framed_guest_input_binding_checked_acceptance_concrete_segment_ids_allowed",
        ],
    );
    for identifier in [
        "runtime_framed_guest_input_binding_checked_acceptance_sound",
        "sound_witness_implies_verifier_core_contract",
        "abstract_verifier_sound",
    ] {
        lean_binding::assert_theorem_body_omits_identifier(
            &lean_source,
            "runtime_framed_guest_input_binding_checked_acceptance_concrete_core_sound_contract",
            identifier,
        );
    }
}
