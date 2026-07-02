use std::path::Path;

#[path = "support/lean_binding.rs"]
mod lean_binding;

#[test]
fn lean_program_image_cache_binding_exports_core_contract_projection() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_path = crate_root.join("../../lean/Lzvm/ProgramImageCacheBinding.lean");
    let lean_source = std::fs::read_to_string(&lean_path)
        .expect("Lean program image cache binding source should read");
    let top_level_path = crate_root.join("../../lean/Lzvm.lean");
    let top_level_source =
        std::fs::read_to_string(&top_level_path).expect("Lean top-level source should read");

    assert!(
        lean_binding::contains_import(&top_level_source, "Lzvm.ProgramImageCacheBinding"),
        "top-level Lean module should import program image cache binding"
    );
    assert!(
        lean_source.contains("RuntimeProgramImageCacheBindingValidation")
            && lean_source.contains("RuntimeProgramImageCacheBindingEvidence")
            && lean_source.contains("RuntimeProgramImageCacheBindingStructuralObligations")
            && lean_source.contains("RuntimeProofArtifactBindingEvidence")
            && lean_source.contains("RuntimeProofArtifactBindingStructuralObligations")
            && lean_source.contains("RuntimeProofArtifactFinalized")
            && lean_source.contains("RuntimeArtifactEvidence")
            && lean_source.contains("proofSegmentIdsUnique")
            && lean_source.contains("proofUnitValuesTraceIdentityCoverage")
            && lean_source.contains("RuntimeVerifierCoreContract system publicInput proof")
            && lean_source.contains("SoundWitness system publicInput proof"),
        "Lean program image cache binding should expose checked evidence and verifier core clauses"
    );
    lean_binding::assert_theorem_declarations(
        &lean_source,
        &[
            "runtime_program_image_cache_binding_checked_acceptance_evidence",
            "runtime_program_image_cache_binding_checked_acceptance_artifact_binding",
            "runtime_program_image_cache_binding_checked_acceptance_artifact_finalized",
            "runtime_program_image_cache_binding_checked_acceptance_artifact_evidence_contract",
            "runtime_program_image_cache_binding_checked_acceptance_artifact_wellformed_contract",
            "runtime_program_image_cache_binding_checked_acceptance_concrete_segment_ids_allowed",
            "runtime_program_image_cache_binding_checked_acceptance_runtime_shape_contract",
            "runtime_program_image_cache_binding_checked_acceptance_sound",
            "runtime_program_image_cache_binding_checked_acceptance_verifier_core_contract",
            "runtime_program_image_cache_binding_checked_acceptance_soundness_contract",
            "runtime_program_image_cache_binding_checked_acceptance_structural_obligations",
            "runtime_program_image_cache_binding_checked_acceptance_full_contract",
            "runtime_program_image_cache_binding_checked_acceptance_evidence_core_and_sound",
            "runtime_program_image_cache_binding_audited_finalized_core_sound_witness_contract",
            "runtime_program_image_cache_binding_audited_finalized_concrete_segment_ids_contract",
            "runtime_program_image_cache_binding_checked_acceptance_unit_values_trace_identity_coverage",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_program_image_cache_binding_checked_acceptance_structural_obligations",
        &[
            "RuntimeProgramImageCacheBindingCheckedAcceptance",
            "RuntimeProgramImageCacheBindingStructuralObligations",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_program_image_cache_binding_checked_acceptance_structural_obligations",
        &[
            "runtime_program_image_cache_binding_checked_acceptance_evidence",
            "runtime_program_image_cache_binding_checked_acceptance_artifact_finalized",
            "runtime_proof_artifact_finalized_structural_obligations",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_program_image_cache_binding_checked_acceptance_artifact_finalized",
        &[
            "runtime_program_image_cache_binding_checked_acceptance_artifact_binding",
            "runtime_proof_artifact_finalized_from_checked_acceptance",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_program_image_cache_binding_checked_acceptance_unit_values_trace_identity_coverage",
        &[
            "RuntimeProgramImageCacheBindingCheckedAcceptance",
            "validation.proofArtifactBindingValidation.proofUnitValuesTraceIdentityCoverage",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_program_image_cache_binding_checked_acceptance_unit_values_trace_identity_coverage",
        &[
            "runtime_program_image_cache_binding_checked_acceptance_artifact_binding",
            "runtime_proof_artifact_binding_checked_acceptance_unit_values_trace_identity_coverage",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_program_image_cache_binding_checked_acceptance_artifact_wellformed_contract",
        &[
            "RuntimeProgramImageCacheBindingCheckedAcceptance",
            "validation.proofArtifactBindingValidation.proofContainerCanonical",
            "validation.proofArtifactBindingValidation.proofMetadataCanonical",
            "validation.proofArtifactBindingValidation.proofSegmentsPresent",
            "validation.proofArtifactBindingValidation.proofSegmentPayloadsNonempty",
            "validation.proofArtifactBindingValidation.proofSegmentIdsAllowed",
            "validation.proofArtifactBindingValidation.proofSegmentIdsUnique",
            "validation.proofArtifactBindingValidation.proofUnitValuesTraceIdentityCoverage",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_program_image_cache_binding_checked_acceptance_artifact_wellformed_contract",
        &[
            "runtime_program_image_cache_binding_checked_acceptance_artifact_finalized",
            "runtime_proof_artifact_finalized_structural_obligations",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_program_image_cache_binding_checked_acceptance_concrete_segment_ids_allowed",
        &[
            "RuntimeProgramImageCacheBindingCheckedAcceptance",
            "RuntimeProofArtifactConcreteSegmentIdBinding",
            "RuntimeProofArtifactConcreteSegmentIdsAllowed proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_program_image_cache_binding_checked_acceptance_concrete_segment_ids_allowed",
        &[
            "runtime_program_image_cache_binding_checked_acceptance_artifact_finalized",
            "runtime_proof_artifact_finalized_concrete_segment_ids_allowed",
        ],
    );
    lean_binding::assert_theorem_prefix_omits(
        &lean_source,
        "runtime_program_image_cache_binding_checked_acceptance_concrete_segment_ids_allowed",
        &["AssumptionBundle"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_program_image_cache_binding_checked_acceptance_runtime_shape_contract",
        &[
            "RuntimeProgramImageCacheBindingCheckedAcceptance",
            "RuntimeProgramImageCacheBindingStructuralObligations",
            "RuntimeProofArtifactConcreteSegmentIdsAllowed proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_program_image_cache_binding_checked_acceptance_runtime_shape_contract",
        &[
            "runtime_program_image_cache_binding_checked_acceptance_structural_obligations",
            "runtime_program_image_cache_binding_checked_acceptance_concrete_segment_ids_allowed",
        ],
    );
    lean_binding::assert_theorem_prefix_omits(
        &lean_source,
        "runtime_program_image_cache_binding_checked_acceptance_runtime_shape_contract",
        &["AssumptionBundle"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_program_image_cache_binding_checked_acceptance_verifier_core_contract",
        &[
            "runtime_program_image_cache_binding_checked_acceptance_artifact_finalized",
            "runtime_proof_artifact_finalized_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_program_image_cache_binding_checked_acceptance_full_contract",
        &[
            "runtime_program_image_cache_binding_checked_acceptance_sound",
            "runtime_program_image_cache_binding_checked_acceptance_structural_obligations",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_program_image_cache_binding_checked_acceptance_full_contract",
        &[
            "RuntimeProgramImageCacheBindingEvidence",
            "RuntimeProofArtifactBindingEvidence",
            "RuntimeProgramImageCacheBindingStructuralObligations",
            "RuntimeArtifactEvidence",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_program_image_cache_binding_checked_acceptance_sound",
        &[
            "runtime_program_image_cache_binding_checked_acceptance_artifact_finalized",
            "runtime_proof_artifact_finalized_full_contract",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_program_image_cache_binding_checked_acceptance_verifier_core_contract",
        &[
            "runtime_program_image_cache_binding_checked_acceptance_sound",
            "runtime_proof_artifact_binding_checked_acceptance_verifier_core_contract",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_program_image_cache_binding_checked_acceptance_evidence_core_and_sound",
        &[
            "RuntimeProgramImageCacheBindingCheckedAcceptance",
            "RuntimeProgramImageCacheBindingEvidence",
            "RuntimeProofArtifactBindingEvidence",
            "RuntimeProgramImageCacheBindingStructuralObligations",
            "RuntimeArtifactEvidence",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_program_image_cache_binding_checked_acceptance_evidence_core_and_sound",
        &[
            "runtime_program_image_cache_binding_checked_acceptance_full_contract",
            "runtime_program_image_cache_binding_checked_acceptance_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_program_image_cache_binding_checked_acceptance_evidence_core_and_sound",
        &["sound_witness_implies_verifier_core_contract"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_program_image_cache_binding_audited_finalized_core_sound_witness_contract",
        &[
            "RequiredCryptographicAssumptionStatements assumptions.crypto",
            "RequiredSemanticAssumptionStatements assumptions.semantic",
            "RuntimeProgramImageCacheBindingEvidence",
            "RuntimeProofArtifactFinalized",
            "RuntimeVerifierCoreContract system publicInput proof",
            "system.traceConsistent publicInput proof trace",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_prefix_omits(
        &lean_source,
        "runtime_program_image_cache_binding_audited_finalized_core_sound_witness_contract",
        &[
            "RuntimeProofArtifactBindingEvidence",
            "RuntimeProgramImageCacheBindingStructuralObligations",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_program_image_cache_binding_audited_finalized_core_sound_witness_contract",
        &[
            "runtime_program_image_cache_binding_checked_acceptance_evidence",
            "runtime_program_image_cache_binding_checked_acceptance_artifact_finalized",
            "runtime_program_image_cache_binding_checked_acceptance_artifact_binding",
            "runtime_proof_artifact_binding_checked_acceptance_runtime_accepted",
            "runtime_artifact_checked_acceptance_implies_verifier_accepts",
            "accepted_proof_audited_core_and_sound_witness",
            "sound_witness_implies_execution_obligations",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_program_image_cache_binding_audited_finalized_core_sound_witness_contract",
        &[
            "runtime_program_image_cache_binding_checked_acceptance_evidence_core_and_sound",
            "runtime_program_image_cache_binding_checked_acceptance_full_contract",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_program_image_cache_binding_audited_finalized_concrete_segment_ids_contract",
        &[
            "RuntimeProgramImageCacheBindingCheckedAcceptance",
            "RuntimeProofArtifactConcreteSegmentIdBinding",
            "RequiredCryptographicAssumptionStatements assumptions.crypto",
            "RequiredSemanticAssumptionStatements assumptions.semantic",
            "RuntimeProgramImageCacheBindingEvidence",
            "RuntimeProofArtifactFinalized",
            "RuntimeVerifierCoreContract system publicInput proof",
            "system.traceConsistent publicInput proof trace",
            "SoundWitness system publicInput proof",
            "RuntimeProofArtifactConcreteSegmentIdsAllowed proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_program_image_cache_binding_audited_finalized_concrete_segment_ids_contract",
        &[
            "runtime_program_image_cache_binding_audited_finalized_core_sound_witness_contract",
            "runtime_program_image_cache_binding_checked_acceptance_concrete_segment_ids_allowed",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_program_image_cache_binding_audited_finalized_concrete_segment_ids_contract",
        &[
            "runtime_program_image_cache_binding_checked_acceptance_evidence_core_and_sound",
            "runtime_program_image_cache_binding_checked_acceptance_full_contract",
        ],
    );
}
