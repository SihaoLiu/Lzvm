use std::path::Path;

#[path = "support/lean_binding.rs"]
mod lean_binding;

#[test]
fn lean_framed_guest_input_binding_exports_soundness_structural_contract() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_path = crate_root.join("../../lean/Lzvm/FramedGuestInputBinding.lean");
    let lean_source =
        std::fs::read_to_string(&lean_path).expect("Lean framed input source should read");
    let top_level_path = crate_root.join("../../lean/Lzvm.lean");
    let top_level_source =
        std::fs::read_to_string(&top_level_path).expect("Lean top-level source should read");

    assert!(
        lean_binding::contains_import(&top_level_source, "Lzvm.FramedGuestInputBinding"),
        "top-level Lean module should import framed guest input binding"
    );
    assert!(
        lean_source.contains("RuntimeFramedGuestInputBindingValidation")
            && lean_source.contains("RuntimeFramedGuestInputBindingEvidence")
            && lean_source.contains("RuntimeFramedGuestInputBindingStructuralObligations")
            && lean_source.contains("framedGuestInputProofSegmentPresent")
            && lean_source.contains("framedGuestInputProofSegmentPayloadExact")
            && lean_source.contains("framedGuestInputProofSegmentPayloadNonempty")
            && lean_source.contains("framedGuestInputCoBoundWithEthBlock")
            && lean_source.contains("framedGuestInputCoBoundWithProgramImage"),
        "Lean framed guest input binding should expose checked evidence and co-binding clauses"
    );
    lean_binding::assert_theorem_declarations(
        &lean_source,
        &[
            "runtime_framed_guest_input_binding_checked_acceptance_eth_block_acceptance",
            "runtime_framed_guest_input_binding_checked_acceptance_program_image_cache_acceptance",
            "runtime_framed_guest_input_binding_checked_acceptance_evidence",
            "runtime_framed_guest_input_binding_checked_acceptance_segment_present",
            "runtime_framed_guest_input_binding_checked_acceptance_segment_payload_exact",
            "runtime_framed_guest_input_binding_checked_acceptance_segment_payload_nonempty",
            "runtime_framed_guest_input_binding_checked_acceptance_eth_block_co_binding",
            "runtime_framed_guest_input_binding_checked_acceptance_program_image_cache_co_binding",
            "runtime_framed_guest_input_binding_checked_acceptance_concrete_segment_ids_allowed",
            "runtime_framed_guest_input_binding_checked_acceptance_runtime_shape_contract",
            "runtime_framed_guest_input_binding_checked_acceptance_soundness_and_structural_contract",
            "runtime_framed_guest_input_binding_checked_acceptance_concrete_core_sound_contract",
            "runtime_framed_guest_input_binding_audited_finalized_core_sound_witness_contract",
            "runtime_framed_guest_input_binding_audited_core_sound_witness_contract",
            "runtime_framed_guest_input_binding_audited_finalized_segment_ids_contract",
            "runtime_framed_guest_input_binding_audited_finalized_concrete_segment_ids_contract",
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
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_framed_guest_input_binding_audited_finalized_core_sound_witness_contract",
        &[
            "RuntimeFramedGuestInputBindingCheckedAcceptance",
            "RequiredCryptographicAssumptionStatements assumptions.crypto",
            "RequiredSemanticAssumptionStatements assumptions.semantic",
            "RuntimeFramedGuestInputBindingEvidence",
            "RuntimeProofArtifactFinalized",
            "validation.ethBlockValidation.proofArtifactBindingValidation",
            "validation.programImageCacheValidation.proofArtifactBindingValidation",
            "RuntimeVerifierCoreContract system publicInput proof",
            "system.traceConsistent publicInput proof trace",
            "system.constraintsSatisfied constraints trace",
            "system.witnessMatchesTrace witness trace",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_framed_guest_input_binding_audited_finalized_core_sound_witness_contract",
        &[
            "runtime_framed_guest_input_binding_checked_acceptance_evidence",
            "runtime_framed_guest_input_binding_checked_acceptance_eth_block_acceptance",
            "runtime_framed_guest_input_binding_checked_acceptance_program_image_cache_acceptance",
            "runtime_eth_block_public_input_binding_audited_finalized_core_sound_witness_contract",
            "runtime_program_image_cache_binding_audited_finalized_core_sound_witness_contract",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_framed_guest_input_binding_audited_finalized_core_sound_witness_contract",
        &[
            "runtime_framed_guest_input_binding_checked_acceptance_full_contract",
            "runtime_framed_guest_input_binding_checked_acceptance_sound",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_framed_guest_input_binding_audited_core_sound_witness_contract",
        &[
            "RuntimeFramedGuestInputBindingCheckedAcceptance",
            "RequiredCryptographicAssumptionStatements assumptions.crypto",
            "RequiredSemanticAssumptionStatements assumptions.semantic",
            "RuntimeVerifierCoreContract system publicInput proof",
            "system.traceConsistent publicInput proof trace",
            "system.constraintsSatisfied constraints trace",
            "system.witnessMatchesTrace witness trace",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_prefix_omits(
        &lean_source,
        "runtime_framed_guest_input_binding_audited_core_sound_witness_contract",
        &[
            "RuntimeFramedGuestInputBindingEvidence",
            "RuntimeProofArtifactFinalized",
            "RuntimeFramedGuestInputBindingStructuralObligations",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_framed_guest_input_binding_audited_core_sound_witness_contract",
        &["runtime_framed_guest_input_binding_audited_finalized_core_sound_witness_contract"],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_framed_guest_input_binding_audited_core_sound_witness_contract",
        &[
            "runtime_framed_guest_input_binding_checked_acceptance_full_contract",
            "runtime_framed_guest_input_binding_checked_acceptance_sound",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_framed_guest_input_binding_audited_finalized_segment_ids_contract",
        &[
            "RequiredCryptographicAssumptionStatements assumptions.crypto",
            "RequiredSemanticAssumptionStatements assumptions.semantic",
            "RuntimeFramedGuestInputBindingEvidence",
            "RuntimeProofArtifactFinalized",
            "validation.ethBlockValidation.proofArtifactBindingValidation",
            "validation.programImageCacheValidation.proofArtifactBindingValidation",
            "RuntimeFramedGuestInputBindingStructuralObligations",
            "RuntimeVerifierCoreContract system publicInput proof",
            "system.traceConsistent publicInput proof trace",
            "system.constraintsSatisfied constraints trace",
            "system.witnessMatchesTrace witness trace",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_framed_guest_input_binding_audited_finalized_segment_ids_contract",
        &[
            "runtime_framed_guest_input_binding_audited_finalized_core_sound_witness_contract",
            "runtime_framed_guest_input_binding_checked_acceptance_structural_obligations",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_framed_guest_input_binding_audited_finalized_segment_ids_contract",
        &[
            "runtime_framed_guest_input_binding_checked_acceptance_full_contract",
            "runtime_framed_guest_input_binding_checked_acceptance_sound",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_framed_guest_input_binding_audited_finalized_concrete_segment_ids_contract",
        &[
            "RuntimeFramedGuestInputBindingCheckedAcceptance",
            "RuntimeProofArtifactConcreteSegmentIdBinding",
            "RequiredCryptographicAssumptionStatements assumptions.crypto",
            "RequiredSemanticAssumptionStatements assumptions.semantic",
            "RuntimeFramedGuestInputBindingEvidence",
            "RuntimeProofArtifactFinalized",
            "validation.ethBlockValidation.proofArtifactBindingValidation",
            "validation.programImageCacheValidation.proofArtifactBindingValidation",
            "RuntimeFramedGuestInputBindingStructuralObligations",
            "RuntimeVerifierCoreContract system publicInput proof",
            "system.traceConsistent publicInput proof trace",
            "system.constraintsSatisfied constraints trace",
            "system.witnessMatchesTrace witness trace",
            "SoundWitness system publicInput proof",
            "RuntimeProofArtifactConcreteSegmentIdsAllowed proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_framed_guest_input_binding_audited_finalized_concrete_segment_ids_contract",
        &[
            "runtime_framed_guest_input_binding_audited_finalized_segment_ids_contract",
            "runtime_framed_guest_input_binding_checked_acceptance_concrete_segment_ids_allowed",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_framed_guest_input_binding_audited_finalized_concrete_segment_ids_contract",
        &[
            "runtime_framed_guest_input_binding_checked_acceptance_full_contract",
            "runtime_framed_guest_input_binding_checked_acceptance_sound",
        ],
    );
}
