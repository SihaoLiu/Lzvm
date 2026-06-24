use std::path::Path;

#[path = "support/lean_binding.rs"]
mod lean_binding;

#[test]
fn lean_proof_artifact_binding_exports_core_contract_projection() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_path = crate_root.join("../../lean/Lzvm/ProofArtifactBinding.lean");
    let lean_source = std::fs::read_to_string(&lean_path)
        .expect("Lean proof artifact binding source should read");
    let top_level_path = crate_root.join("../../lean/Lzvm.lean");
    let top_level_source =
        std::fs::read_to_string(&top_level_path).expect("Lean top-level source should read");

    assert!(
        top_level_source.contains("import Lzvm.ProofArtifactBinding"),
        "top-level Lean module should import proof artifact binding"
    );
    assert!(
        lean_source.contains("RuntimeProofArtifactBindingValidation")
            && lean_source.contains("RuntimeVerifierCoreContract system publicInput proof")
            && lean_source.contains("proofContainerCanonical")
            && lean_source.contains("proofMetadataCanonical")
            && lean_source.contains("proofSegmentsPresent")
            && lean_source.contains("proofSegmentPayloadsNonempty")
            && lean_source.contains("proofSegmentIdsAllowed")
            && lean_source.contains("proofSegmentIdsUnique")
            && lean_source.contains("SoundWitness system publicInput proof"),
        "Lean proof artifact binding should expose checked soundness and verifier core projection"
    );
    lean_binding::assert_theorem_declarations(
        &lean_source,
        &[
            "runtime_proof_artifact_binding_checked_acceptance_runtime_accepted",
            "runtime_proof_artifact_binding_checked_acceptance_container_canonical",
            "runtime_proof_artifact_binding_checked_acceptance_metadata_canonical",
            "runtime_proof_artifact_binding_checked_acceptance_segments_present",
            "runtime_proof_artifact_binding_checked_acceptance_segment_payloads_nonempty",
            "runtime_proof_artifact_binding_checked_acceptance_segment_ids_allowed",
            "runtime_proof_artifact_binding_checked_acceptance_segment_ids_unique",
            "runtime_proof_artifact_binding_checked_acceptance_sound",
            "runtime_proof_artifact_binding_checked_acceptance_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_proof_artifact_binding_checked_acceptance_container_canonical",
        &["bindingAcceptedImpliesProofContainerCanonical"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_proof_artifact_binding_checked_acceptance_metadata_canonical",
        &["bindingAcceptedImpliesProofMetadataCanonical"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_proof_artifact_binding_checked_acceptance_segments_present",
        &["bindingAcceptedImpliesProofSegmentsPresent"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_proof_artifact_binding_checked_acceptance_segment_payloads_nonempty",
        &["bindingAcceptedImpliesProofSegmentPayloadsNonempty"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_proof_artifact_binding_checked_acceptance_segment_ids_allowed",
        &["bindingAcceptedImpliesProofSegmentIdsAllowed"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_proof_artifact_binding_checked_acceptance_segment_ids_unique",
        &["bindingAcceptedImpliesProofSegmentIdsUnique"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_proof_artifact_binding_checked_acceptance_sound",
        &[
            "runtime_proof_artifact_binding_checked_acceptance_obligations",
            "abstract_verifier_sound",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_proof_artifact_binding_checked_acceptance_sound",
        &["sound_witness_implies_verifier_core_contract"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_proof_artifact_binding_checked_acceptance_verifier_core_contract",
        &["runtime_proof_artifact_binding_checked_acceptance_obligations"],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_proof_artifact_binding_checked_acceptance_verifier_core_contract",
        &[
            "runtime_proof_artifact_binding_checked_acceptance_sound",
            "sound_witness_implies_verifier_core_contract",
            "abstract_verifier_sound",
        ],
    );
}
