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
        lean_binding::contains_import(&top_level_source, "Lzvm.EthBlockPublicInputBinding"),
        "top-level Lean module should import ETH block public-input binding"
    );
    assert!(
        lean_source.contains("RuntimeEthBlockPublicInputBindingValidation")
            && lean_source.contains("RuntimeEthBlockPublicInputBindingEvidence")
            && lean_source.contains("RuntimeEthBlockPublicInputBindingStructuralObligations")
            && lean_source.contains("RuntimeProofArtifactBindingEvidence")
            && lean_source.contains("RuntimeProofArtifactBindingStructuralObligations")
            && lean_source.contains("RuntimeArtifactEvidence")
            && lean_source.contains("proofContainerCanonical")
            && lean_source.contains("proofMetadataCanonical")
            && lean_source.contains("proofSegmentPayloadsNonempty")
            && lean_source.contains("proofSegmentIdsAllowed")
            && lean_source.contains("proofSegmentIdsUnique")
            && lean_source.contains("proofUnitValuesTraceIdentityCoverage")
            && lean_source.contains("RuntimeVerifierCoreContract system publicInput proof")
            && lean_source.contains("SoundWitness system publicInput proof"),
        "Lean ETH block public-input binding should expose checked evidence and verifier core clauses"
    );
    lean_binding::assert_theorem_declarations(
        &lean_source,
        &[
            "runtime_eth_block_public_input_binding_checked_acceptance_artifact_evidence_contract",
            "runtime_eth_block_public_input_binding_checked_acceptance_artifact_wellformed_contract",
            "runtime_eth_block_public_input_binding_checked_acceptance_concrete_segment_ids_allowed",
            "runtime_eth_block_public_input_binding_checked_acceptance_sound",
            "runtime_eth_block_public_input_binding_checked_acceptance_verifier_core_contract",
            "runtime_eth_block_public_input_binding_checked_acceptance_structural_obligations",
            "runtime_eth_block_public_input_binding_checked_acceptance_full_contract",
            "runtime_eth_block_public_input_binding_checked_acceptance_evidence_core_and_sound",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_eth_block_public_input_binding_checked_acceptance_structural_obligations",
        &[
            "RuntimeEthBlockPublicInputBindingCheckedAcceptance",
            "RuntimeEthBlockPublicInputBindingStructuralObligations",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_eth_block_public_input_binding_checked_acceptance_structural_obligations",
        &[
            "runtime_eth_block_public_input_binding_checked_acceptance_evidence",
            "runtime_eth_block_public_input_binding_checked_acceptance_artifact_binding",
            "runtime_proof_artifact_binding_checked_acceptance_structural_obligations",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_eth_block_public_input_binding_checked_acceptance_artifact_evidence_contract",
        &[
            "RuntimeEthBlockPublicInputBindingEvidence",
            "RuntimeProofArtifactBindingEvidence",
            "RuntimeArtifactEvidence",
        ],
    );
    lean_binding::assert_theorem_prefix_omits(
        &lean_source,
        "runtime_eth_block_public_input_binding_checked_acceptance_artifact_evidence_contract",
        &["AssumptionBundle"],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_eth_block_public_input_binding_checked_acceptance_artifact_evidence_contract",
        &["abstract_verifier_sound"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_eth_block_public_input_binding_checked_acceptance_artifact_wellformed_contract",
        &[
            "RuntimeEthBlockPublicInputBindingCheckedAcceptance",
            "validation.proofArtifactBindingValidation.proofContainerCanonical",
            "validation.proofArtifactBindingValidation.proofMetadataCanonical",
            "validation.proofArtifactBindingValidation.proofSegmentsPresent",
            "validation.proofArtifactBindingValidation.proofSegmentPayloadsNonempty",
            "validation.proofArtifactBindingValidation.proofSegmentIdsAllowed",
            "validation.proofArtifactBindingValidation.proofSegmentIdsUnique",
            "validation.proofArtifactBindingValidation.proofUnitValuesTraceIdentityCoverage",
        ],
    );
    lean_binding::assert_theorem_prefix_omits(
        &lean_source,
        "runtime_eth_block_public_input_binding_checked_acceptance_artifact_wellformed_contract",
        &["AssumptionBundle"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_eth_block_public_input_binding_checked_acceptance_artifact_wellformed_contract",
        &[
            "runtime_eth_block_public_input_binding_checked_acceptance_artifact_binding",
            "runtime_proof_artifact_binding_checked_acceptance_container_canonical",
            "runtime_proof_artifact_binding_checked_acceptance_metadata_canonical",
            "runtime_proof_artifact_binding_checked_acceptance_segments_present",
            "runtime_proof_artifact_binding_checked_acceptance_segment_payloads_nonempty",
            "runtime_proof_artifact_binding_checked_acceptance_segment_ids_allowed",
            "runtime_proof_artifact_binding_checked_acceptance_segment_ids_unique",
            "runtime_proof_artifact_binding_checked_acceptance_unit_values_trace_identity_coverage",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_eth_block_public_input_binding_checked_acceptance_artifact_wellformed_contract",
        &[
            "abstract_verifier_sound",
            "RuntimeProofArtifactBindingEvidence",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_eth_block_public_input_binding_checked_acceptance_concrete_segment_ids_allowed",
        &[
            "RuntimeEthBlockPublicInputBindingCheckedAcceptance",
            "RuntimeProofArtifactConcreteSegmentIdBinding",
            "RuntimeProofArtifactConcreteSegmentIdsAllowed proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_eth_block_public_input_binding_checked_acceptance_concrete_segment_ids_allowed",
        &[
            "runtime_eth_block_public_input_binding_checked_acceptance_artifact_binding",
            "runtime_proof_artifact_binding_checked_acceptance_concrete_segment_ids_allowed",
        ],
    );
    lean_binding::assert_theorem_prefix_omits(
        &lean_source,
        "runtime_eth_block_public_input_binding_checked_acceptance_concrete_segment_ids_allowed",
        &["AssumptionBundle"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_eth_block_public_input_binding_checked_acceptance_verifier_core_contract",
        &["runtime_proof_artifact_binding_checked_acceptance_verifier_core_contract"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_eth_block_public_input_binding_checked_acceptance_full_contract",
        &[
            "runtime_eth_block_public_input_binding_checked_acceptance_sound",
            "runtime_eth_block_public_input_binding_checked_acceptance_structural_obligations",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_eth_block_public_input_binding_checked_acceptance_full_contract",
        &[
            "RuntimeEthBlockPublicInputBindingEvidence",
            "RuntimeProofArtifactBindingEvidence",
            "RuntimeEthBlockPublicInputBindingStructuralObligations",
            "RuntimeArtifactEvidence",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_eth_block_public_input_binding_checked_acceptance_verifier_core_contract",
        &[
            "runtime_eth_block_public_input_binding_checked_acceptance_sound",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_eth_block_public_input_binding_checked_acceptance_evidence_core_and_sound",
        &[
            "RuntimeEthBlockPublicInputBindingCheckedAcceptance",
            "RuntimeEthBlockPublicInputBindingEvidence",
            "RuntimeProofArtifactBindingEvidence",
            "RuntimeEthBlockPublicInputBindingStructuralObligations",
            "RuntimeArtifactEvidence",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_eth_block_public_input_binding_checked_acceptance_evidence_core_and_sound",
        &[
            "runtime_eth_block_public_input_binding_checked_acceptance_full_contract",
            "runtime_eth_block_public_input_binding_checked_acceptance_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_eth_block_public_input_binding_checked_acceptance_evidence_core_and_sound",
        &["sound_witness_implies_verifier_core_contract"],
    );
}
