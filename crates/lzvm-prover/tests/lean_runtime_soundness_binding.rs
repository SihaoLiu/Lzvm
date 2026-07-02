use std::path::Path;

#[path = "support/lean_binding.rs"]
mod lean_binding;

#[test]
fn lean_runtime_soundness_binding_exports_core_contract_projection() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_path = crate_root.join("../../lean/Lzvm/RuntimeSoundness.lean");
    let runtime_soundness_source =
        std::fs::read_to_string(&lean_path).expect("Lean runtime soundness source should read");
    let runtime_soundness_core_path = crate_root.join("../../lean/Lzvm/RuntimeSoundness/Core.lean");
    let runtime_soundness_core_source = std::fs::read_to_string(&runtime_soundness_core_path)
        .expect("Lean runtime soundness core source should read");
    let runtime_soundness_external_source_path =
        crate_root.join("../../lean/Lzvm/RuntimeSoundness/ExternalSource.lean");
    let runtime_soundness_external_source =
        std::fs::read_to_string(&runtime_soundness_external_source_path)
            .expect("Lean runtime soundness external-source source should read");
    let contracts_path = crate_root.join("../../lean/Lzvm/RuntimeSoundness/Contracts.lean");
    let contracts_source = std::fs::read_to_string(&contracts_path)
        .expect("Lean runtime soundness contracts source should read");
    let segment_ids_path = crate_root.join("../../lean/Lzvm/RuntimeSoundness/SegmentIds.lean");
    let segment_ids_source = std::fs::read_to_string(&segment_ids_path)
        .expect("Lean runtime soundness segment-id source should read");
    let lean_source = format!(
        "{runtime_soundness_source}\n{runtime_soundness_core_source}\n{runtime_soundness_external_source}\n{segment_ids_source}\n{contracts_source}"
    );
    let top_level_path = crate_root.join("../../lean/Lzvm.lean");
    let top_level_source =
        std::fs::read_to_string(&top_level_path).expect("Lean top-level source should read");

    assert!(
        lean_binding::contains_import(&top_level_source, "Lzvm.RuntimeSoundness"),
        "top-level Lean module should import runtime soundness"
    );
    assert!(
        lean_binding::contains_import(&top_level_source, "Lzvm.RuntimeSoundness.Contracts"),
        "top-level Lean module should import runtime soundness contracts"
    );
    assert!(
        lean_binding::contains_import(&top_level_source, "Lzvm.RuntimeSoundness.SegmentIds"),
        "top-level Lean module should import runtime soundness segment-id projections"
    );
    assert!(
        lean_source.contains("RuntimeSoundnessValidation")
            && lean_source.contains("RuntimeVerifierCoreContract system publicInput proof")
            && lean_source.contains("system.publicInputBound publicInput proof")
            && lean_source.contains("SoundWitness system publicInput proof"),
        "Lean runtime soundness should expose checked soundness and verifier core projection"
    );
    lean_binding::assert_theorem_declarations(
        &lean_source,
        &[
            "runtime_soundness_checked_acceptance_sound",
            "runtime_soundness_evidence_implies_runtime_artifact_evidence",
            "runtime_soundness_evidence_implies_transcript_bound",
            "runtime_soundness_evidence_implies_public_input_bound",
            "runtime_soundness_evidence_implies_core_obligations",
            "runtime_soundness_evidence_implies_external_source_requirement",
            "runtime_soundness_evidence_implies_binding_pcs_fri_contract",
            "runtime_soundness_evidence_implies_runtime_artifact_core_contract",
            "runtime_soundness_evidence_audited_runtime_artifact_core_contract",
            "runtime_soundness_checked_acceptance_runtime_artifact_evidence",
            "runtime_soundness_checked_acceptance_transcript_bound",
            "runtime_soundness_checked_acceptance_public_input_bound",
            "runtime_soundness_checked_acceptance_pcs_and_fri",
            "runtime_soundness_checked_acceptance_external_source_requirement",
            "runtime_soundness_checked_acceptance_core_obligations",
            "runtime_soundness_checked_acceptance_runtime_artifact_core_contract",
            "runtime_soundness_checked_acceptance_segments_present",
            "runtime_soundness_checked_acceptance_container_canonical",
            "runtime_soundness_checked_acceptance_metadata_canonical",
            "runtime_soundness_checked_acceptance_segment_payloads_nonempty",
            "runtime_soundness_checked_acceptance_segment_ids_allowed",
            "runtime_soundness_checked_acceptance_concrete_segment_ids_allowed",
            "runtime_soundness_checked_acceptance_segment_ids_unique",
            "runtime_soundness_checked_acceptance_unit_values_trace_identity_coverage",
            "runtime_soundness_checked_acceptance_verifier_core_contract",
            "runtime_soundness_checked_acceptance_evidence_core_and_sound",
            "runtime_soundness_checked_acceptance_concrete_core_sound_contract",
            "runtime_soundness_checked_acceptance_verifier_sound_witness",
            "runtime_soundness_checked_acceptance_execution_obligations",
            "runtime_soundness_checked_acceptance_audited_soundness_obligations",
            "runtime_soundness_checked_acceptance_audited_core_contract",
            "runtime_soundness_checked_acceptance_verifier_accepts",
            "runtime_soundness_checked_acceptance_accepts_core_sound_witness",
            "runtime_soundness_checked_acceptance_proof_system_sound",
            "runtime_soundness_checked_acceptance_full_soundness_contract",
            "runtime_soundness_checked_acceptance_finalized_full_soundness_contract",
            "runtime_soundness_checked_acceptance_accepts_full_soundness_contract",
            "runtime_soundness_checked_acceptance_proof_system_full_soundness_contract",
            "runtime_soundness_checked_acceptance_audited_proof_system_contract",
            "runtime_soundness_checked_acceptance_audited_soundness_proof_system_contract",
            "runtime_soundness_checked_acceptance_audited_soundness_finalized_proof_system_contract",
            "runtime_soundness_checked_acceptance_audited_finalized_core_sound_witness_contract",
            "runtime_soundness_checked_acceptance_audited_finalized_concrete_segment_ids_contract",
            "runtime_soundness_checked_acceptance_audited_accepts_sound_witness_contract",
            "runtime_soundness_required_external_source_pcs_sound",
            "runtime_soundness_required_external_source_verifier_core_contract",
            "runtime_soundness_required_external_source_evidence_core_and_sound",
            "runtime_soundness_required_external_source_accepts_core_sound_witness",
            "runtime_soundness_required_external_source_full_soundness_contract",
            "runtime_soundness_required_external_source_proof_system_full_soundness_contract",
            "runtime_soundness_required_external_source_audited_proof_system_contract",
            "runtime_soundness_required_external_source_audited_soundness_proof_system_contract",
            "runtime_soundness_required_external_source_audited_proof_system_core_contract",
            "runtime_soundness_required_external_source_audited_accepts_sound_witness_contract",
            "runtime_soundness_required_external_source_audited_pcs_accepts_sound_witness_contract",
            "runtime_soundness_required_external_source_audited_pcs_fri_witness_contract",
            "runtime_soundness_required_external_source_audited_pcs_fri_core_witness_contract",
            "runtime_soundness_required_external_source_audited_finalized_core_sound_witness_contract",
            "runtime_soundness_checked_acceptance_audited_binding_pcs_fri_core_witness_contract",
            "runtime_soundness_checked_acceptance_artifact_segment_ids_contract",
            "runtime_soundness_checked_acceptance_concrete_artifact_segment_ids_contract",
            "runtime_soundness_checked_acceptance_contracts_core_contract",
            "runtime_soundness_checked_acceptance_audited_soundness_contracts_core_contract",
            "runtime_soundness_checked_acceptance_artifact_contracts_core_contract",
            "runtime_soundness_checked_acceptance_artifact_audited_soundness_contracts_core_contract",
            "runtime_soundness_required_external_source_contracts_core_contract",
            "runtime_soundness_required_external_source_contracts_audited_soundness_core_contract",
            "runtime_soundness_required_external_source_artifact_contracts_core_contract",
            "runtime_soundness_required_external_source_artifact_audited_soundness_core_contract",
            "runtime_soundness_required_external_source_artifact_segment_ids_contract",
            "runtime_soundness_required_external_source_artifact_audited_segment_ids_contract",
        ],
    );
    for name in [
        "runtime_soundness_required_external_source_artifact_contracts_core_contract",
        "runtime_soundness_required_external_source_artifact_audited_soundness_core_contract",
        "runtime_soundness_required_external_source_artifact_segment_ids_contract",
        "runtime_soundness_required_external_source_artifact_audited_segment_ids_contract",
    ] {
        lean_binding::assert_theorem_prefix_contains(
            &lean_source,
            name,
            &[
                "(requiresExternalSource : Prop)",
                "RuntimeSoundnessCheckedAcceptance",
                "proof\n          requiresExternalSource ->\n        requiresExternalSource ->",
            ],
        );
    }
    assert!(theorem_prefix(
        &lean_source,
        "runtime_soundness_checked_acceptance_runtime_artifact_evidence"
    )
    .contains("(validation : RuntimeSoundnessValidation system)"));
    assert!(
        !theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_runtime_artifact_evidence"
        )
        .contains("AssumptionBundle"),
        "runtime artifact evidence projection should not require cryptographic assumptions"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_segments_present"
        )
        .contains("validation.transcriptValidation.artifactBindingValidation.proofSegmentsPresent"),
        "runtime soundness should expose proof segment presence from artifact validation"
    );
    assert!(
        !theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_segments_present"
        )
        .contains("AssumptionBundle"),
        "proof segment presence projection should not require cryptographic assumptions"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_container_canonical"
        )
        .contains(
            "validation.transcriptValidation.artifactBindingValidation.proofContainerCanonical"
        ),
        "runtime soundness should expose proof container canonicality from artifact validation"
    );
    assert!(
        !theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_container_canonical"
        )
        .contains("AssumptionBundle"),
        "proof container canonicality projection should not require cryptographic assumptions"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_metadata_canonical"
        )
        .contains(
            "validation.transcriptValidation.artifactBindingValidation.proofMetadataCanonical"
        ),
        "runtime soundness should expose proof metadata canonicality from artifact validation"
    );
    assert!(
        !theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_metadata_canonical"
        )
        .contains("AssumptionBundle"),
        "proof metadata canonicality projection should not require cryptographic assumptions"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_segment_payloads_nonempty"
        )
        .contains(
            "validation.transcriptValidation.artifactBindingValidation.proofSegmentPayloadsNonempty"
        ),
        "runtime soundness should expose nonempty proof segment payloads from artifact validation"
    );
    assert!(
        !theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_segment_payloads_nonempty"
        )
        .contains("AssumptionBundle"),
        "proof segment payload nonempty projection should not require cryptographic assumptions"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_segment_ids_allowed"
        )
        .contains(
            "validation.transcriptValidation.artifactBindingValidation.proofSegmentIdsAllowed"
        ),
        "runtime soundness should expose allowed proof segment IDs from artifact validation"
    );
    assert!(
        !theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_segment_ids_allowed"
        )
        .contains("AssumptionBundle"),
        "allowed proof segment-id projection should not require cryptographic assumptions"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_concrete_segment_ids_allowed"
        )
        .contains("RuntimeProofArtifactConcreteSegmentIdBinding")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_concrete_segment_ids_allowed"
            )
            .contains("RuntimeProofArtifactConcreteSegmentIdsAllowed proof"),
        "runtime soundness should expose concrete allowed proof segment IDs"
    );
    assert!(
        !theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_concrete_segment_ids_allowed"
        )
        .contains("AssumptionBundle"),
        "concrete allowed proof segment-id projection should not require cryptographic assumptions"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_segment_ids_unique"
        )
        .contains(
            "validation.transcriptValidation.artifactBindingValidation.proofSegmentIdsUnique"
        ),
        "runtime soundness should expose proof segment-id uniqueness from artifact validation"
    );
    assert!(
        !theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_segment_ids_unique"
        )
        .contains("AssumptionBundle"),
        "proof segment-id uniqueness projection should not require cryptographic assumptions"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_unit_values_trace_identity_coverage"
        )
        .contains("let artifactValidation := validation.transcriptValidation.artifactBindingValidation")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_unit_values_trace_identity_coverage"
            )
            .contains("artifactValidation.proofUnitValuesTraceIdentityCoverage"),
        "runtime soundness should expose unit-values trace identity coverage from artifact validation"
    );
    assert!(
        !theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_unit_values_trace_identity_coverage"
        )
        .contains("AssumptionBundle"),
        "unit-values trace identity coverage projection should not require cryptographic assumptions"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_artifact_segment_ids_contract"
        )
        .contains("RuntimeArtifactEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_artifact_segment_ids_contract"
            )
            .contains("let artifactValidation := validation.transcriptValidation.artifactBindingValidation")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_artifact_segment_ids_contract"
            )
            .contains("artifactValidation.proofContainerCanonical")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_artifact_segment_ids_contract"
            )
            .contains("artifactValidation.proofSegmentsPresent")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_artifact_segment_ids_contract"
            )
            .contains("artifactValidation.proofMetadataCanonical")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_artifact_segment_ids_contract"
            )
            .contains("artifactValidation.proofSegmentPayloadsNonempty")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_artifact_segment_ids_contract"
            )
            .contains("artifactValidation.proofSegmentIdsAllowed")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_artifact_segment_ids_contract"
            )
            .contains("artifactValidation.proofSegmentIdsUnique")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_artifact_segment_ids_contract"
            )
            .contains("artifactValidation.proofUnitValuesTraceIdentityCoverage"),
        "artifact segment contract should expose artifact evidence, proof container canonicality, proof metadata canonicality, proof segment presence, nonempty proof segment payloads, allowed proof segment IDs, proof segment-id uniqueness, and unit-values trace identity coverage"
    );
    assert!(
        !theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_artifact_segment_ids_contract"
        )
        .contains("AssumptionBundle"),
        "artifact segment-id contract should not require cryptographic assumptions"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_concrete_artifact_segment_ids_contract"
        )
        .contains("RuntimeProofArtifactConcreteSegmentIdBinding")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_concrete_artifact_segment_ids_contract"
            )
            .contains("RuntimeProofArtifactConcreteSegmentIdsAllowed proof"),
        "concrete artifact segment contract should expose the concrete allowlist conclusion"
    );
    assert!(
        !theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_concrete_artifact_segment_ids_contract"
        )
        .contains("AssumptionBundle"),
        "concrete artifact segment-id contract should not require cryptographic assumptions"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_external_source_requirement"
        )
        .contains("(validation : RuntimeSoundnessValidation system)"),
        "external-source requirement projection should be tied to runtime soundness validation"
    );
    assert!(
        !theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_external_source_requirement"
        )
        .contains("AssumptionBundle"),
        "external-source requirement projection should not require cryptographic assumptions"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_evidence_implies_external_source_requirement"
        )
        .contains("(validation : RuntimeSoundnessValidation system)"),
        "runtime soundness evidence projection should be tied to runtime soundness validation"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_evidence_implies_external_source_requirement"
        )
        .contains("ExternalSourceOpeningRequirement")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_evidence_implies_external_source_requirement"
            )
            .contains("validation.sourceValidation"),
        "runtime soundness evidence should expose the external-source requirement field"
    );
    assert!(
        !theorem_prefix(
            &lean_source,
            "runtime_soundness_evidence_implies_external_source_requirement"
        )
        .contains("AssumptionBundle"),
        "runtime soundness evidence external-source projection should not require cryptographic assumptions"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_evidence_implies_binding_pcs_fri_contract"
        )
        .contains("system.transcriptBound publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_evidence_implies_binding_pcs_fri_contract"
            )
            .contains("system.publicInputBound publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_evidence_implies_binding_pcs_fri_contract"
            )
            .contains("system.pcsOpeningsValid publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_evidence_implies_binding_pcs_fri_contract"
            )
            .contains("system.friQueriesValid publicInput proof"),
        "runtime soundness evidence should expose the compact transcript/public-input/PCS/FRI contract"
    );
    assert!(
        !theorem_prefix(
            &lean_source,
            "runtime_soundness_evidence_implies_binding_pcs_fri_contract"
        )
        .contains("AssumptionBundle"),
        "runtime soundness evidence binding PCS/FRI projection should not require cryptographic assumptions"
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_soundness_checked_acceptance_evidence",
        &["runtime_soundness_checked_acceptance_core_obligations"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_soundness_checked_acceptance_evidence",
        &[
            "runtime_transcript_binding_checked_acceptance_full_contract",
            "runtime_transcript_binding_evidence_implies_transcript_bound",
            "transcriptFull.left",
            "transcriptFull.right.right.left",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_soundness_checked_acceptance_evidence",
        &[
            "runtime_transcript_binding_checked_acceptance_sound",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_soundness_checked_acceptance_sound",
        &[
            "runtime_transcript_binding_checked_acceptance_full_contract",
            "transcriptFull.right.right.right",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_soundness_checked_acceptance_sound",
        &["runtime_transcript_binding_checked_acceptance_sound"],
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_evidence_implies_runtime_artifact_core_contract"
        )
        .contains("RuntimeArtifactEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_evidence_implies_runtime_artifact_core_contract"
            )
            .contains("RuntimeVerifierCoreContract system publicInput proof"),
        "runtime soundness evidence should package runtime artifact evidence with verifier core obligations"
    );
    assert!(
        !theorem_prefix(
            &lean_source,
            "runtime_soundness_evidence_implies_runtime_artifact_core_contract"
        )
        .contains("AssumptionBundle"),
        "runtime soundness evidence artifact-core projection should not require cryptographic assumptions"
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_soundness_evidence_audited_runtime_artifact_core_contract",
        &[
            "RuntimeSoundnessEvidence",
            "RequiredCryptographicAssumptionStatements assumptions.crypto",
            "RequiredSemanticAssumptionStatements assumptions.semantic",
            "RuntimeArtifactEvidence",
            "system.transcriptBound publicInput proof",
            "system.publicInputBound publicInput proof",
            "ExternalSourceOpeningRequirement",
            "system.pcsOpeningsValid publicInput proof",
            "system.friQueriesValid publicInput proof",
            "RuntimeVerifierCoreContract system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_prefix_omits(
        &lean_source,
        "runtime_soundness_evidence_audited_runtime_artifact_core_contract",
        &["RuntimeSoundnessCheckedAcceptance"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_soundness_evidence_audited_runtime_artifact_core_contract",
        &[
            "assumption_bundle_carries_required_evidence",
            "runtime_soundness_evidence_implies_runtime_artifact_evidence",
            "runtime_soundness_evidence_implies_external_source_requirement",
            "runtime_soundness_evidence_implies_pcs_and_fri",
            "runtime_soundness_evidence_implies_core_obligations",
        ],
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_runtime_artifact_core_contract"
        )
        .contains("(assumptions : AssumptionBundle system)")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_runtime_artifact_core_contract"
            )
            .contains("RuntimeArtifactEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_runtime_artifact_core_contract"
            )
            .contains("RuntimeVerifierCoreContract system publicInput proof"),
        "checked runtime soundness should expose artifact evidence plus verifier core obligations under audited assumptions"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_audited_core_contract"
        )
        .contains("(assumptions : AssumptionBundle system)"),
        "audited runtime core contract should require the audited assumption bundle"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_audited_soundness_obligations"
        )
        .contains("RequiredCryptographicAssumptionStatements assumptions.crypto")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_audited_soundness_obligations"
            )
            .contains("RequiredSemanticAssumptionStatements assumptions.semantic")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_audited_soundness_obligations"
            )
            .contains("RuntimeSoundnessEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_audited_soundness_obligations"
            )
            .contains("SoundWitness system publicInput proof"),
        "checked runtime soundness should package audited crypto and semantic obligations with runtime evidence"
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_soundness_checked_acceptance_audited_soundness_obligations",
        &[
            "runtime_soundness_checked_acceptance_audited_assumptions",
            "assumption_bundle_carries_required_evidence",
        ],
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_audited_core_contract"
        )
        .contains("RequiredCryptographicAssumptionStatements assumptions.crypto")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_audited_core_contract"
            )
            .contains("RuntimeVerifierCoreContract system publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_audited_core_contract"
            )
            .contains("SoundWitness system publicInput proof"),
        "audited runtime core contract should package crypto evidence, verifier core obligations, and sound witness"
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_soundness_checked_acceptance_audited_core_contract",
        &["runtime_soundness_checked_acceptance_core_obligations"],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_soundness_checked_acceptance_audited_core_contract",
        &["sound_witness_implies_verifier_core_contract"],
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_verifier_accepts"
        )
        .contains("(validation : RuntimeSoundnessValidation system)")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_verifier_accepts"
            )
            .contains("system.accepts publicInput proof"),
        "checked runtime soundness should expose verifier acceptance through validation"
    );
    assert!(
        !theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_verifier_accepts"
        )
        .contains("AssumptionBundle"),
        "verifier acceptance projection should not require cryptographic assumptions"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_accepts_core_sound_witness"
        )
        .contains("(assumptions : AssumptionBundle system)")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_accepts_core_sound_witness"
            )
            .contains("system.accepts publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_accepts_core_sound_witness"
            )
            .contains("RuntimeVerifierCoreContract system publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_accepts_core_sound_witness"
            )
            .contains("SoundWitness system publicInput proof"),
        "checked runtime soundness should package acceptance, core obligations, and sound witness"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_proof_system_sound"
        )
        .contains("ProofSystemSound system")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_proof_system_sound"
            )
            .contains("system.accepts publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_proof_system_sound"
            )
            .contains("SoundWitness system publicInput proof"),
        "checked runtime soundness should expose model-wide proof-system soundness and the accepted proof witness"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_audited_accepts_sound_witness_contract"
        )
        .contains("RequiredCryptographicAssumptionStatements assumptions.crypto")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_audited_accepts_sound_witness_contract"
            )
            .contains("ProofSystemSound system")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_audited_accepts_sound_witness_contract"
            )
            .contains("system.accepts publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_audited_accepts_sound_witness_contract"
            )
            .contains("SoundWitness system publicInput proof"),
        "checked runtime soundness should expose compact audited acceptance and witness evidence"
    );
    assert!(
        !theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_audited_accepts_sound_witness_contract"
        )
        .contains("RuntimeSoundnessEvidence"),
        "compact audited runtime acceptance contract should not force callers to unpack full runtime evidence"
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_soundness_checked_acceptance_execution_obligations",
        &["sound_witness_implies_execution_obligations"],
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_full_soundness_contract"
        )
        .contains("RuntimeSoundnessEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_full_soundness_contract"
            )
            .contains("RuntimeArtifactSoundnessObligations")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_full_soundness_contract"
            )
            .contains("RuntimeVerifierCoreContract system publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_full_soundness_contract"
            )
            .contains("SoundWitness system publicInput proof"),
        "checked runtime soundness should package evidence, artifact obligations, core obligations, and sound witness"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_accepts_full_soundness_contract"
        )
        .contains("system.accepts publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_accepts_full_soundness_contract"
            )
            .contains("RuntimeSoundnessEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_accepts_full_soundness_contract"
            )
            .contains("RuntimeArtifactSoundnessObligations")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_accepts_full_soundness_contract"
            )
            .contains("SoundWitness system publicInput proof"),
        "checked runtime soundness should package verifier acceptance with the full soundness contract"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_proof_system_full_soundness_contract"
        )
        .contains("ProofSystemSound system")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_proof_system_full_soundness_contract"
            )
            .contains("system.accepts publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_proof_system_full_soundness_contract"
            )
            .contains("RuntimeSoundnessEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_proof_system_full_soundness_contract"
            )
            .contains("RuntimeArtifactSoundnessObligations")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_proof_system_full_soundness_contract"
            )
            .contains("RuntimeVerifierCoreContract system publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_proof_system_full_soundness_contract"
            )
            .contains("SoundWitness system publicInput proof"),
        "checked runtime soundness should package model soundness with the accepted full soundness contract"
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_soundness_checked_acceptance_finalized_full_soundness_contract",
        &[
            "RuntimeProofArtifactFinalized",
            "RuntimeSoundnessEvidence",
            "RuntimeArtifactSoundnessObligations",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_soundness_checked_acceptance_finalized_full_soundness_contract",
        &[
            "runtime_transcript_binding_checked_acceptance_artifact_finalized",
            "runtime_soundness_checked_acceptance_full_soundness_contract",
        ],
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_audited_proof_system_contract"
        )
        .contains("RequiredCryptographicAssumptionStatements assumptions.crypto")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_audited_proof_system_contract"
            )
            .contains("ProofSystemSound system")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_audited_proof_system_contract"
            )
            .contains("system.accepts publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_audited_proof_system_contract"
            )
            .contains("RuntimeSoundnessEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_audited_proof_system_contract"
            )
            .contains("RuntimeVerifierCoreContract system publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_audited_proof_system_contract"
            )
            .contains("SoundWitness system publicInput proof"),
        "checked runtime soundness should package audited crypto assumptions with proof-system soundness"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_audited_soundness_proof_system_contract"
        )
        .contains("RequiredCryptographicAssumptionStatements assumptions.crypto")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_audited_soundness_proof_system_contract"
            )
            .contains("RequiredSemanticAssumptionStatements assumptions.semantic")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_audited_soundness_proof_system_contract"
            )
            .contains("ProofSystemSound system")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_audited_soundness_proof_system_contract"
            )
            .contains("system.accepts publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_audited_soundness_proof_system_contract"
            )
            .contains("RuntimeVerifierCoreContract system publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_audited_soundness_proof_system_contract"
            )
            .contains("SoundWitness system publicInput proof"),
        "checked runtime soundness should package audited crypto and semantic assumptions with proof-system soundness"
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_soundness_checked_acceptance_audited_soundness_proof_system_contract",
        &[
            "runtime_soundness_checked_acceptance_audited_proof_system_contract",
            "assumption_bundle_carries_required_evidence",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_soundness_checked_acceptance_audited_soundness_finalized_proof_system_contract",
        &[
            "RequiredCryptographicAssumptionStatements assumptions.crypto",
            "RequiredSemanticAssumptionStatements assumptions.semantic",
            "RuntimeProofArtifactFinalized",
            "ProofSystemSound system",
            "system.accepts publicInput proof",
            "RuntimeSoundnessEvidence",
            "RuntimeArtifactSoundnessObligations",
            "RuntimeVerifierCoreContract system publicInput proof",
            "system.traceConsistent publicInput proof trace",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_soundness_checked_acceptance_audited_soundness_finalized_proof_system_contract",
        &[
            "runtime_soundness_checked_acceptance_finalized_full_soundness_contract",
            "runtime_soundness_checked_acceptance_audited_soundness_proof_system_contract",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_soundness_checked_acceptance_audited_finalized_core_sound_witness_contract",
        &[
            "RequiredCryptographicAssumptionStatements assumptions.crypto",
            "RequiredSemanticAssumptionStatements assumptions.semantic",
            "RuntimeProofArtifactFinalized",
            "RuntimeVerifierCoreContract system publicInput proof",
            "system.traceConsistent publicInput proof trace",
            "SoundWitness system publicInput proof",
        ],
    );
    assert!(
        !theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_audited_finalized_core_sound_witness_contract"
        )
        .contains("RuntimeSoundnessEvidence"),
        "compact finalized core sound witness contract should not force callers to unpack full runtime evidence"
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_soundness_checked_acceptance_audited_finalized_core_sound_witness_contract",
        &[
            "runtime_soundness_checked_acceptance_verifier_accepts",
            "accepted_proof_audited_core_and_sound_witness",
            "runtime_transcript_binding_checked_acceptance_artifact_finalized",
            "sound_witness_implies_execution_obligations",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_soundness_checked_acceptance_audited_finalized_core_sound_witness_contract",
        &[
            "runtime_soundness_checked_acceptance_audited_soundness_finalized_proof_system_contract",
            "RuntimeSoundnessEvidence",
        ],
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_required_external_source_accepts_core_sound_witness"
        )
        .contains("(assumptions : AssumptionBundle system)")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_accepts_core_sound_witness"
            )
            .contains("requiresExternalSource ->")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_accepts_core_sound_witness"
            )
            .contains("system.accepts publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_accepts_core_sound_witness"
            )
            .contains("ExternalSourceOpeningEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_accepts_core_sound_witness"
            )
            .contains("RuntimeVerifierCoreContract system publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_accepts_core_sound_witness"
            )
            .contains("SoundWitness system publicInput proof"),
        "required external-source runtime soundness should package acceptance, external-source evidence, core obligations, and sound witness"
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_soundness_required_external_source_verifier_core_contract",
        &["runtime_soundness_checked_acceptance_core_obligations"],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_soundness_required_external_source_verifier_core_contract",
        &[
            "runtime_soundness_required_external_source_sound",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_soundness_required_external_source_evidence_core_and_sound",
        &[
            "requiresExternalSource ->",
            "RuntimeSoundnessEvidence",
            "ExternalSourceOpeningEvidence",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_soundness_required_external_source_evidence_core_and_sound",
        &[
            "runtime_soundness_required_external_source_sound",
            "runtime_soundness_required_external_source_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_soundness_required_external_source_evidence_core_and_sound",
        &["sound_witness_implies_verifier_core_contract"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_soundness_required_external_source_accepts_core_sound_witness",
        &["runtime_soundness_required_external_source_verifier_core_contract"],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_soundness_required_external_source_accepts_core_sound_witness",
        &["sound_witness_implies_verifier_core_contract"],
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_required_external_source_full_soundness_contract"
        )
        .contains("(assumptions : AssumptionBundle system)")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_full_soundness_contract"
            )
            .contains("requiresExternalSource ->")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_full_soundness_contract"
            )
            .contains("ExternalSourceOpeningEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_full_soundness_contract"
            )
            .contains("RuntimeSoundnessEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_full_soundness_contract"
            )
            .contains("RuntimeArtifactSoundnessObligations")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_full_soundness_contract"
            )
            .contains("RuntimeVerifierCoreContract system publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_full_soundness_contract"
            )
            .contains("SoundWitness system publicInput proof"),
        "required external-source runtime soundness should expose the full soundness contract"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_required_external_source_proof_system_full_soundness_contract"
        )
        .contains("ProofSystemSound system")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_proof_system_full_soundness_contract"
            )
            .contains("system.accepts publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_proof_system_full_soundness_contract"
            )
            .contains("ExternalSourceOpeningEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_proof_system_full_soundness_contract"
            )
            .contains("RuntimeSoundnessEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_proof_system_full_soundness_contract"
            )
            .contains("RuntimeArtifactSoundnessObligations")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_proof_system_full_soundness_contract"
            )
            .contains("RuntimeVerifierCoreContract system publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_proof_system_full_soundness_contract"
            )
            .contains("SoundWitness system publicInput proof"),
        "required external-source runtime soundness should package model soundness with the full soundness contract"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_required_external_source_audited_proof_system_contract"
        )
        .contains("RequiredCryptographicAssumptionStatements assumptions.crypto")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_proof_system_contract"
            )
            .contains("ProofSystemSound system")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_proof_system_contract"
            )
            .contains("system.accepts publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_proof_system_contract"
            )
            .contains("RuntimeSoundnessEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_proof_system_contract"
            )
            .contains("RuntimeVerifierCoreContract system publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_proof_system_contract"
            )
            .contains("SoundWitness system publicInput proof"),
        "required external-source runtime soundness should package audited crypto assumptions with proof-system soundness"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_required_external_source_audited_soundness_proof_system_contract"
        )
        .contains("RequiredCryptographicAssumptionStatements assumptions.crypto")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_soundness_proof_system_contract"
            )
            .contains("RequiredSemanticAssumptionStatements assumptions.semantic")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_soundness_proof_system_contract"
            )
            .contains("ProofSystemSound system")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_soundness_proof_system_contract"
            )
            .contains("ExternalSourceOpeningEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_soundness_proof_system_contract"
            )
            .contains("RuntimeVerifierCoreContract system publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_soundness_proof_system_contract"
            )
            .contains("SoundWitness system publicInput proof"),
        "required external-source runtime soundness should package audited crypto and semantic assumptions with proof-system soundness"
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_soundness_required_external_source_audited_soundness_proof_system_contract",
        &[
            "runtime_soundness_required_external_source_audited_proof_system_contract",
            "assumption_bundle_carries_required_evidence",
        ],
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_required_external_source_audited_proof_system_core_contract"
        )
        .contains("RequiredCryptographicAssumptionStatements assumptions.crypto")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_proof_system_core_contract"
            )
            .contains("ProofSystemSound system")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_proof_system_core_contract"
            )
            .contains("system.accepts publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_proof_system_core_contract"
            )
            .contains("ExternalSourceOpeningEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_proof_system_core_contract"
            )
            .contains("validation.sourceValidation")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_proof_system_core_contract"
            )
            .contains("system.transcriptBound publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_proof_system_core_contract"
            )
            .contains("system.publicInputBound publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_proof_system_core_contract"
            )
            .contains("system.pcsOpeningsValid publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_proof_system_core_contract"
            )
            .contains("system.friQueriesValid publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_proof_system_core_contract"
            )
            .contains("RuntimeVerifierCoreContract system publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_proof_system_core_contract"
            )
            .contains("system.traceConsistent publicInput proof trace")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_proof_system_core_contract"
            )
            .contains("SoundWitness system publicInput proof"),
        "required external-source runtime soundness should package audited proof-system core obligations"
    );
    assert!(
        !theorem_prefix(
            &lean_source,
            "runtime_soundness_required_external_source_audited_proof_system_core_contract"
        )
        .contains("RuntimeSoundnessEvidence")
            && !theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_proof_system_core_contract"
            )
            .contains("RuntimeArtifactSoundnessObligations"),
        "compact required external-source proof-system core contract should not force full runtime artifact evidence"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_required_external_source_audited_accepts_sound_witness_contract"
        )
        .contains("RequiredCryptographicAssumptionStatements assumptions.crypto")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_accepts_sound_witness_contract"
            )
            .contains("ProofSystemSound system")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_accepts_sound_witness_contract"
            )
            .contains("system.accepts publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_accepts_sound_witness_contract"
            )
            .contains("ExternalSourceOpeningEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_accepts_sound_witness_contract"
            )
            .contains("SoundWitness system publicInput proof"),
        "required external-source runtime soundness should expose compact audited acceptance, external-source evidence, and witness evidence"
    );
    assert!(
        !theorem_prefix(
            &lean_source,
            "runtime_soundness_required_external_source_audited_accepts_sound_witness_contract"
        )
        .contains("RuntimeSoundnessEvidence"),
        "compact required external-source runtime contract should not force callers to unpack full runtime evidence"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_required_external_source_audited_pcs_accepts_sound_witness_contract"
        )
        .contains("RequiredCryptographicAssumptionStatements assumptions.crypto")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_pcs_accepts_sound_witness_contract"
            )
            .contains("ProofSystemSound system")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_pcs_accepts_sound_witness_contract"
            )
            .contains("system.accepts publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_pcs_accepts_sound_witness_contract"
            )
            .contains("ExternalSourceOpeningEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_pcs_accepts_sound_witness_contract"
            )
            .contains("system.pcsOpeningsValid publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_pcs_accepts_sound_witness_contract"
            )
            .contains("SoundWitness system publicInput proof"),
        "required external-source runtime soundness should expose compact audited acceptance, external-source evidence, PCS openings, and witness evidence"
    );
    assert!(
        !theorem_prefix(
            &lean_source,
            "runtime_soundness_required_external_source_audited_pcs_accepts_sound_witness_contract"
        )
        .contains("RuntimeSoundnessEvidence"),
        "compact required external-source runtime PCS contract should not force callers to unpack full runtime evidence"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_required_external_source_audited_pcs_fri_witness_contract"
        )
        .contains("RequiredCryptographicAssumptionStatements assumptions.crypto")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_pcs_fri_witness_contract"
            )
            .contains("ProofSystemSound system")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_pcs_fri_witness_contract"
            )
            .contains("system.accepts publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_pcs_fri_witness_contract"
            )
            .contains("ExternalSourceOpeningEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_pcs_fri_witness_contract"
            )
            .contains("system.pcsOpeningsValid publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_pcs_fri_witness_contract"
            )
            .contains("system.friQueriesValid publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_pcs_fri_witness_contract"
            )
            .contains("SoundWitness system publicInput proof"),
        "required external-source runtime soundness should expose compact audited acceptance, external-source evidence, PCS openings, FRI queries, and witness evidence"
    );
    assert!(
        !theorem_prefix(
            &lean_source,
            "runtime_soundness_required_external_source_audited_pcs_fri_witness_contract"
        )
        .contains("RuntimeSoundnessEvidence"),
        "compact required external-source runtime PCS/FRI contract should not force callers to unpack full runtime evidence"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_required_external_source_audited_pcs_fri_core_witness_contract"
        )
        .contains("RequiredCryptographicAssumptionStatements assumptions.crypto")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_pcs_fri_core_witness_contract"
            )
            .contains("ProofSystemSound system")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_pcs_fri_core_witness_contract"
            )
            .contains("system.accepts publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_pcs_fri_core_witness_contract"
            )
            .contains("ExternalSourceOpeningEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_pcs_fri_core_witness_contract"
            )
            .contains("system.pcsOpeningsValid publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_pcs_fri_core_witness_contract"
            )
            .contains("system.friQueriesValid publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_pcs_fri_core_witness_contract"
            )
            .contains("RuntimeVerifierCoreContract system publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_audited_pcs_fri_core_witness_contract"
            )
            .contains("SoundWitness system publicInput proof"),
        "required external-source runtime soundness should expose compact audited PCS/FRI, core obligations, and witness evidence"
    );
    assert!(
        !theorem_prefix(
            &lean_source,
            "runtime_soundness_required_external_source_audited_pcs_fri_core_witness_contract"
        )
        .contains("RuntimeSoundnessEvidence"),
        "compact required external-source runtime PCS/FRI core contract should not force callers to unpack full runtime evidence"
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_soundness_required_external_source_audited_finalized_core_sound_witness_contract",
        &[
            "requiresExternalSource ->",
            "RequiredCryptographicAssumptionStatements assumptions.crypto",
            "RequiredSemanticAssumptionStatements assumptions.semantic",
            "RuntimeProofArtifactFinalized",
            "ExternalSourceOpeningEvidence",
            "RuntimeVerifierCoreContract system publicInput proof",
            "system.traceConsistent publicInput proof trace",
            "SoundWitness system publicInput proof",
        ],
    );
    assert!(
        !theorem_prefix(
            &lean_source,
            "runtime_soundness_required_external_source_audited_finalized_core_sound_witness_contract"
        )
        .contains("RuntimeSoundnessEvidence"),
        "compact required external-source finalized core contract should not force callers to unpack full runtime evidence"
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_soundness_required_external_source_audited_finalized_core_sound_witness_contract",
        &[
            "runtime_soundness_checked_acceptance_audited_finalized_core_sound_witness_contract",
            "external_source_opening_requirement_implies_evidence",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_soundness_required_external_source_audited_finalized_core_sound_witness_contract",
        &[
            "runtime_soundness_required_external_source_sound",
            "RuntimeSoundnessEvidence",
        ],
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_required_external_source_contracts_core_contract"
        )
        .contains("RequiredCryptographicAssumptionStatements assumptions.crypto")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_contracts_core_contract"
            )
            .contains("ProofSystemSound system")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_contracts_core_contract"
            )
            .contains("system.accepts publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_contracts_core_contract"
            )
            .contains("ExternalSourceOpeningEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_contracts_core_contract"
            )
            .contains("validation.sourceValidation")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_contracts_core_contract"
            )
            .contains("system.transcriptBound publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_contracts_core_contract"
            )
            .contains("system.publicInputBound publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_contracts_core_contract"
            )
            .contains("system.pcsOpeningsValid publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_contracts_core_contract"
            )
            .contains("system.friQueriesValid publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_contracts_core_contract"
            )
            .contains("RuntimeVerifierCoreContract system publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_contracts_core_contract"
            )
            .contains("system.traceConsistent publicInput proof trace")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_contracts_core_contract"
            )
            .contains("SoundWitness system publicInput proof"),
        "runtime contracts should expose required external-source proof-system core obligations"
    );
    assert!(
        !theorem_prefix(
            &lean_source,
            "runtime_soundness_required_external_source_contracts_core_contract"
        )
        .contains("RuntimeSoundnessEvidence")
            && !theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_contracts_core_contract"
            )
            .contains("RuntimeArtifactSoundnessObligations"),
        "runtime required external-source contracts wrapper should keep the compact core surface"
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_soundness_required_external_source_contracts_audited_soundness_core_contract",
        &[
            "RequiredCryptographicAssumptionStatements assumptions.crypto",
            "RequiredSemanticAssumptionStatements assumptions.semantic",
            "ProofSystemSound system",
            "system.accepts publicInput proof",
            "ExternalSourceOpeningEvidence",
            "validation.sourceValidation",
            "system.transcriptBound publicInput proof",
            "system.publicInputBound publicInput proof",
            "system.pcsOpeningsValid publicInput proof",
            "system.friQueriesValid publicInput proof",
            "RuntimeVerifierCoreContract system publicInput proof",
            "system.traceConsistent publicInput proof trace",
            "system.constraintsSatisfied constraints trace",
            "system.witnessMatchesTrace witness trace",
            "SoundWitness system publicInput proof",
        ],
    );
    assert!(
        !theorem_prefix(
            &lean_source,
            "runtime_soundness_required_external_source_contracts_audited_soundness_core_contract"
        )
        .contains("RuntimeSoundnessEvidence")
            && !theorem_prefix(
                &lean_source,
                "runtime_soundness_required_external_source_contracts_audited_soundness_core_contract"
            )
            .contains("RuntimeArtifactSoundnessObligations"),
        "runtime audited required external-source contracts wrapper should keep the compact core surface"
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_soundness_required_external_source_contracts_audited_soundness_core_contract",
        &[
            "runtime_soundness_required_external_source_contracts_core_contract",
            "assumption_bundle_carries_required_evidence",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_soundness_required_external_source_contracts_audited_soundness_core_contract",
        &[
            "runtime_soundness_required_external_source_audited_soundness_proof_system_contract",
            "RuntimeArtifactSoundnessObligations",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_soundness_required_external_source_artifact_contracts_core_contract",
        &[
            "RuntimeArtifactEvidence",
            "validation.transcriptValidation.artifactBindingValidation.runtimeValidation",
            "RequiredCryptographicAssumptionStatements assumptions.crypto",
            "ProofSystemSound system",
            "system.accepts publicInput proof",
            "ExternalSourceOpeningEvidence",
            "validation.sourceValidation",
            "system.transcriptBound publicInput proof",
            "system.publicInputBound publicInput proof",
            "system.pcsOpeningsValid publicInput proof",
            "system.friQueriesValid publicInput proof",
            "RuntimeVerifierCoreContract system publicInput proof",
            "system.traceConsistent publicInput proof trace",
            "system.constraintsSatisfied constraints trace",
            "system.witnessMatchesTrace witness trace",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_prefix_omits(
        &lean_source,
        "runtime_soundness_required_external_source_artifact_contracts_core_contract",
        &[
            "RequiredSemanticAssumptionStatements assumptions.semantic",
            "RuntimeSoundnessEvidence",
            "RuntimeArtifactSoundnessObligations",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_soundness_required_external_source_artifact_contracts_core_contract",
        &[
            "runtime_soundness_checked_acceptance_runtime_artifact_evidence",
            "runtime_soundness_required_external_source_contracts_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_soundness_required_external_source_artifact_contracts_core_contract",
        &[
            "runtime_soundness_required_external_source_full_soundness_contract",
            "RuntimeSoundnessEvidence",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_soundness_required_external_source_artifact_audited_soundness_core_contract",
        &[
            "RuntimeArtifactEvidence",
            "validation.transcriptValidation.artifactBindingValidation.runtimeValidation",
            "RequiredCryptographicAssumptionStatements assumptions.crypto",
            "RequiredSemanticAssumptionStatements assumptions.semantic",
            "ProofSystemSound system",
            "system.accepts publicInput proof",
            "ExternalSourceOpeningEvidence",
            "validation.sourceValidation",
            "system.transcriptBound publicInput proof",
            "system.publicInputBound publicInput proof",
            "system.pcsOpeningsValid publicInput proof",
            "system.friQueriesValid publicInput proof",
            "RuntimeVerifierCoreContract system publicInput proof",
            "system.traceConsistent publicInput proof trace",
            "system.constraintsSatisfied constraints trace",
            "system.witnessMatchesTrace witness trace",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_prefix_omits(
        &lean_source,
        "runtime_soundness_required_external_source_artifact_audited_soundness_core_contract",
        &[
            "RuntimeSoundnessEvidence",
            "RuntimeArtifactSoundnessObligations",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_soundness_required_external_source_artifact_audited_soundness_core_contract",
        &[
            "runtime_soundness_required_external_source_artifact_contracts_core_contract",
            "assumption_bundle_carries_required_evidence",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_soundness_required_external_source_artifact_audited_soundness_core_contract",
        &[
            "runtime_soundness_required_external_source_audited_soundness_proof_system_contract",
            "RuntimeSoundnessEvidence",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_soundness_required_external_source_artifact_segment_ids_contract",
        &[
            "let artifactValidation :=",
            "RuntimeArtifactEvidence",
            "artifactValidation.runtimeValidation",
            "RequiredCryptographicAssumptionStatements assumptions.crypto",
            "ProofSystemSound system",
            "system.accepts publicInput proof",
            "ExternalSourceOpeningEvidence",
            "validation.sourceValidation",
            "system.transcriptBound publicInput proof",
            "system.publicInputBound publicInput proof",
            "system.pcsOpeningsValid publicInput proof",
            "system.friQueriesValid publicInput proof",
            "RuntimeVerifierCoreContract system publicInput proof",
            "system.traceConsistent publicInput proof trace",
            "system.constraintsSatisfied constraints trace",
            "system.witnessMatchesTrace witness trace",
            "SoundWitness system publicInput proof",
            "artifactValidation.proofContainerCanonical artifact publicInput proof",
            "artifactValidation.proofSegmentsPresent artifact publicInput proof",
            "artifactValidation.proofMetadataCanonical artifact publicInput proof",
            "artifactValidation.proofSegmentPayloadsNonempty artifact publicInput proof",
            "artifactValidation.proofSegmentIdsAllowed artifact publicInput proof",
            "artifactValidation.proofSegmentIdsUnique artifact publicInput proof",
            "artifactValidation.proofUnitValuesTraceIdentityCoverage",
        ],
    );
    lean_binding::assert_theorem_prefix_omits(
        &lean_source,
        "runtime_soundness_required_external_source_artifact_segment_ids_contract",
        &[
            "RequiredSemanticAssumptionStatements assumptions.semantic",
            "RuntimeSoundnessEvidence",
            "RuntimeArtifactSoundnessObligations",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_soundness_required_external_source_artifact_segment_ids_contract",
        &[
            "runtime_soundness_required_external_source_artifact_contracts_core_contract",
            "runtime_soundness_checked_acceptance_artifact_segment_ids_contract",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_soundness_required_external_source_artifact_segment_ids_contract",
        &[
            "runtime_soundness_required_external_source_full_soundness_contract",
            "RuntimeSoundnessEvidence",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_soundness_required_external_source_artifact_audited_segment_ids_contract",
        &[
            "let artifactValidation :=",
            "RuntimeArtifactEvidence",
            "artifactValidation.runtimeValidation",
            "RequiredCryptographicAssumptionStatements assumptions.crypto",
            "RequiredSemanticAssumptionStatements assumptions.semantic",
            "ProofSystemSound system",
            "system.accepts publicInput proof",
            "ExternalSourceOpeningEvidence",
            "validation.sourceValidation",
            "system.transcriptBound publicInput proof",
            "system.publicInputBound publicInput proof",
            "system.pcsOpeningsValid publicInput proof",
            "system.friQueriesValid publicInput proof",
            "RuntimeVerifierCoreContract system publicInput proof",
            "system.traceConsistent publicInput proof trace",
            "system.constraintsSatisfied constraints trace",
            "system.witnessMatchesTrace witness trace",
            "SoundWitness system publicInput proof",
            "artifactValidation.proofContainerCanonical artifact publicInput proof",
            "artifactValidation.proofSegmentsPresent artifact publicInput proof",
            "artifactValidation.proofMetadataCanonical artifact publicInput proof",
            "artifactValidation.proofSegmentPayloadsNonempty artifact publicInput proof",
            "artifactValidation.proofSegmentIdsAllowed artifact publicInput proof",
            "artifactValidation.proofSegmentIdsUnique artifact publicInput proof",
            "artifactValidation.proofUnitValuesTraceIdentityCoverage",
        ],
    );
    lean_binding::assert_theorem_prefix_omits(
        &lean_source,
        "runtime_soundness_required_external_source_artifact_audited_segment_ids_contract",
        &[
            "RuntimeSoundnessEvidence",
            "RuntimeArtifactSoundnessObligations",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_soundness_required_external_source_artifact_audited_segment_ids_contract",
        &[
            "runtime_soundness_required_external_source_artifact_segment_ids_contract",
            "assumption_bundle_carries_required_evidence",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_soundness_required_external_source_artifact_audited_segment_ids_contract",
        &[
            "runtime_soundness_required_external_source_audited_soundness_proof_system_contract",
            "RuntimeSoundnessEvidence",
        ],
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_audited_binding_pcs_fri_core_witness_contract"
        )
        .contains("RequiredCryptographicAssumptionStatements assumptions.crypto")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_audited_binding_pcs_fri_core_witness_contract"
            )
            .contains("ProofSystemSound system")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_audited_binding_pcs_fri_core_witness_contract"
            )
            .contains("system.accepts publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_audited_binding_pcs_fri_core_witness_contract"
            )
            .contains("system.transcriptBound publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_audited_binding_pcs_fri_core_witness_contract"
            )
            .contains("system.publicInputBound publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_audited_binding_pcs_fri_core_witness_contract"
            )
            .contains("system.pcsOpeningsValid publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_audited_binding_pcs_fri_core_witness_contract"
            )
            .contains("system.friQueriesValid publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_audited_binding_pcs_fri_core_witness_contract"
            )
            .contains("RuntimeVerifierCoreContract system publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_audited_binding_pcs_fri_core_witness_contract"
            )
            .contains("SoundWitness system publicInput proof"),
        "checked runtime soundness should expose compact audited transcript/public-input/PCS/FRI core witness evidence"
    );
    assert!(
        !theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_audited_binding_pcs_fri_core_witness_contract"
        )
        .contains("RuntimeSoundnessEvidence"),
        "compact checked runtime binding PCS/FRI core contract should not force callers to unpack full runtime evidence"
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_soundness_checked_acceptance_verifier_sound_witness",
        &["runtime_soundness_checked_acceptance_sound", "sound.right"],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_soundness_checked_acceptance_verifier_sound_witness",
        &["sound_witness_implies_verifier_core_contract"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_soundness_checked_acceptance_evidence_core_and_sound",
        &[
            "(assumptions : AssumptionBundle system)",
            "RuntimeSoundnessEvidence",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_soundness_checked_acceptance_evidence_core_and_sound",
        &[
            "runtime_soundness_checked_acceptance_sound",
            "runtime_soundness_checked_acceptance_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_soundness_checked_acceptance_evidence_core_and_sound",
        &["sound_witness_implies_verifier_core_contract"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_soundness_checked_acceptance_concrete_core_sound_contract",
        &[
            "(assumptions : AssumptionBundle system)",
            "RuntimeProofArtifactConcreteSegmentIdBinding",
            "validation.transcriptValidation.artifactBindingValidation",
            "RuntimeSoundnessEvidence",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
            "RuntimeProofArtifactConcreteSegmentIdsAllowed proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_soundness_checked_acceptance_concrete_core_sound_contract",
        &[
            "runtime_soundness_checked_acceptance_evidence_core_and_sound",
            "runtime_soundness_checked_acceptance_concrete_segment_ids_allowed",
        ],
    );
    for identifier in [
        "runtime_soundness_checked_acceptance_sound",
        "sound_witness_implies_verifier_core_contract",
        "abstract_verifier_sound",
    ] {
        lean_binding::assert_theorem_body_omits_identifier(
            &lean_source,
            "runtime_soundness_checked_acceptance_concrete_core_sound_contract",
            identifier,
        );
    }
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_soundness_checked_acceptance_audited_finalized_concrete_segment_ids_contract",
        &[
            "(assumptions : AssumptionBundle system)",
            "RuntimeProofArtifactConcreteSegmentIdBinding",
            "validation.transcriptValidation.artifactBindingValidation",
            "RequiredCryptographicAssumptionStatements assumptions.crypto",
            "RequiredSemanticAssumptionStatements assumptions.semantic",
            "RuntimeProofArtifactFinalized",
            "RuntimeVerifierCoreContract system publicInput proof",
            "system.traceConsistent publicInput proof trace",
            "SoundWitness system publicInput proof",
            "RuntimeProofArtifactConcreteSegmentIdsAllowed proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_soundness_checked_acceptance_audited_finalized_concrete_segment_ids_contract",
        &[
            "runtime_soundness_checked_acceptance_audited_finalized_core_sound_witness_contract",
            "runtime_soundness_checked_acceptance_concrete_segment_ids_allowed",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_soundness_checked_acceptance_audited_finalized_concrete_segment_ids_contract",
        &[
            "accepted_proof_audited_core_and_sound_witness",
            "runtime_soundness_checked_acceptance_audited_soundness_finalized_proof_system_contract",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_soundness_checked_acceptance_proof_system_sound",
        &[
            "abstract_verifier_sound",
            "runtime_soundness_checked_acceptance_verifier_accepts",
            "runtime_soundness_checked_acceptance_verifier_sound_witness",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_soundness_checked_acceptance_proof_system_sound",
        &["sound_witness_implies_verifier_core_contract"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_soundness_checked_acceptance_audited_binding_pcs_fri_core_witness_contract",
        &[
            "assumption_bundle_carries_required_crypto_evidence",
            "abstract_verifier_sound",
            "runtime_soundness_checked_acceptance_verifier_accepts",
            "runtime_soundness_checked_acceptance_transcript_bound",
            "runtime_soundness_checked_acceptance_public_input_bound",
            "runtime_soundness_checked_acceptance_pcs_and_fri",
            "runtime_soundness_checked_acceptance_verifier_core_contract",
            "runtime_soundness_checked_acceptance_verifier_sound_witness",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_soundness_checked_acceptance_audited_binding_pcs_fri_core_witness_contract",
        &["sound_witness_implies_verifier_core_contract"],
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_contracts_core_contract"
        )
        .contains("RequiredCryptographicAssumptionStatements assumptions.crypto")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_contracts_core_contract"
            )
            .contains("ProofSystemSound system")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_contracts_core_contract"
            )
            .contains("system.accepts publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_contracts_core_contract"
            )
            .contains("system.transcriptBound publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_contracts_core_contract"
            )
            .contains("system.publicInputBound publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_contracts_core_contract"
            )
            .contains("system.pcsOpeningsValid publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_contracts_core_contract"
            )
            .contains("system.friQueriesValid publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_contracts_core_contract"
            )
            .contains("RuntimeVerifierCoreContract system publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_contracts_core_contract"
            )
            .contains("system.traceConsistent publicInput proof trace")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_contracts_core_contract"
            )
            .contains("SoundWitness system publicInput proof"),
        "checked runtime contracts should expose compact proof-system core obligations"
    );
    assert!(
        !theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_contracts_core_contract"
        )
        .contains("RuntimeSoundnessEvidence")
            && !theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_contracts_core_contract"
            )
            .contains("RuntimeArtifactSoundnessObligations"),
        "checked runtime contracts wrapper should keep the compact core surface"
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_soundness_checked_acceptance_contracts_core_contract",
        &[
            "runtime_soundness_checked_acceptance_audited_binding_pcs_fri_core_witness_contract",
            "runtime_soundness_checked_acceptance_execution_obligations",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_soundness_checked_acceptance_contracts_core_contract",
        &[
            "abstract_verifier_sound",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_soundness_checked_acceptance_audited_soundness_contracts_core_contract",
        &[
            "RequiredCryptographicAssumptionStatements assumptions.crypto",
            "RequiredSemanticAssumptionStatements assumptions.semantic",
            "ProofSystemSound system",
            "system.accepts publicInput proof",
            "system.transcriptBound publicInput proof",
            "system.publicInputBound publicInput proof",
            "system.pcsOpeningsValid publicInput proof",
            "system.friQueriesValid publicInput proof",
            "RuntimeVerifierCoreContract system publicInput proof",
            "system.traceConsistent publicInput proof trace",
            "system.constraintsSatisfied constraints trace",
            "system.witnessMatchesTrace witness trace",
            "SoundWitness system publicInput proof",
        ],
    );
    assert!(
        !theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_audited_soundness_contracts_core_contract"
        )
        .contains("RuntimeSoundnessEvidence")
            && !theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_audited_soundness_contracts_core_contract"
            )
            .contains("RuntimeArtifactSoundnessObligations"),
        "checked audited runtime contracts wrapper should keep the compact core surface"
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_soundness_checked_acceptance_audited_soundness_contracts_core_contract",
        &[
            "runtime_soundness_checked_acceptance_contracts_core_contract",
            "assumption_bundle_carries_required_evidence",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_soundness_checked_acceptance_audited_soundness_contracts_core_contract",
        &[
            "runtime_soundness_checked_acceptance_audited_soundness_proof_system_contract",
            "RuntimeArtifactSoundnessObligations",
        ],
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_artifact_contracts_core_contract"
        )
        .contains("RuntimeArtifactEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_artifact_contracts_core_contract"
            )
            .contains("RequiredCryptographicAssumptionStatements assumptions.crypto")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_artifact_contracts_core_contract"
            )
            .contains("ProofSystemSound system")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_artifact_contracts_core_contract"
            )
            .contains("system.accepts publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_artifact_contracts_core_contract"
            )
            .contains("system.transcriptBound publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_artifact_contracts_core_contract"
            )
            .contains("system.publicInputBound publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_artifact_contracts_core_contract"
            )
            .contains("system.pcsOpeningsValid publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_artifact_contracts_core_contract"
            )
            .contains("system.friQueriesValid publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_artifact_contracts_core_contract"
            )
            .contains("RuntimeVerifierCoreContract system publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_artifact_contracts_core_contract"
            )
            .contains("system.traceConsistent publicInput proof trace")
            && theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_artifact_contracts_core_contract"
            )
            .contains("SoundWitness system publicInput proof"),
        "checked runtime artifact contracts should expose artifact evidence plus compact proof-system core obligations"
    );
    assert!(
        !theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_artifact_contracts_core_contract"
        )
        .contains("RuntimeSoundnessEvidence")
            && !theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_artifact_contracts_core_contract"
            )
            .contains("RuntimeArtifactSoundnessObligations"),
        "checked runtime artifact contracts wrapper should keep the compact artifact-core surface"
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_soundness_checked_acceptance_segments_present",
        &["runtime_transcript_binding_checked_acceptance_segments_present"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_soundness_checked_acceptance_artifact_audited_soundness_contracts_core_contract",
        &[
            "RuntimeArtifactEvidence",
            "RequiredCryptographicAssumptionStatements assumptions.crypto",
            "RequiredSemanticAssumptionStatements assumptions.semantic",
            "ProofSystemSound system",
            "system.accepts publicInput proof",
            "system.transcriptBound publicInput proof",
            "system.publicInputBound publicInput proof",
            "system.pcsOpeningsValid publicInput proof",
            "system.friQueriesValid publicInput proof",
            "RuntimeVerifierCoreContract system publicInput proof",
            "system.traceConsistent publicInput proof trace",
            "system.constraintsSatisfied constraints trace",
            "system.witnessMatchesTrace witness trace",
            "SoundWitness system publicInput proof",
        ],
    );
    assert!(
        !theorem_prefix(
            &lean_source,
            "runtime_soundness_checked_acceptance_artifact_audited_soundness_contracts_core_contract"
        )
        .contains("RuntimeSoundnessEvidence")
            && !theorem_prefix(
                &lean_source,
                "runtime_soundness_checked_acceptance_artifact_audited_soundness_contracts_core_contract"
            )
            .contains("RuntimeArtifactSoundnessObligations"),
        "checked audited runtime artifact contracts wrapper should keep the compact artifact-core surface"
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_soundness_checked_acceptance_artifact_audited_soundness_contracts_core_contract",
        &[
            "runtime_soundness_checked_acceptance_artifact_contracts_core_contract",
            "assumption_bundle_carries_required_evidence",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_soundness_checked_acceptance_artifact_audited_soundness_contracts_core_contract",
        &[
            "runtime_soundness_checked_acceptance_audited_soundness_proof_system_contract",
            "RuntimeArtifactSoundnessObligations",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_soundness_checked_acceptance_container_canonical",
        &["runtime_transcript_binding_checked_acceptance_container_canonical"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_soundness_checked_acceptance_metadata_canonical",
        &["runtime_transcript_binding_checked_acceptance_metadata_canonical"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_soundness_checked_acceptance_segment_payloads_nonempty",
        &["runtime_transcript_binding_checked_acceptance_segment_payloads_nonempty"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_soundness_checked_acceptance_segment_ids_allowed",
        &["runtime_transcript_binding_checked_acceptance_segment_ids_allowed"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_soundness_checked_acceptance_concrete_segment_ids_allowed",
        &["runtime_transcript_binding_checked_acceptance_concrete_segment_ids_allowed"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_soundness_checked_acceptance_segment_ids_unique",
        &["runtime_transcript_binding_checked_acceptance_segment_ids_unique"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_soundness_checked_acceptance_artifact_segment_ids_contract",
        &[
            "runtime_soundness_checked_acceptance_runtime_artifact_evidence",
            "runtime_soundness_checked_acceptance_segments_present",
            "runtime_soundness_checked_acceptance_container_canonical",
            "runtime_soundness_checked_acceptance_metadata_canonical",
            "runtime_soundness_checked_acceptance_segment_payloads_nonempty",
            "runtime_soundness_checked_acceptance_segment_ids_allowed",
            "runtime_soundness_checked_acceptance_segment_ids_unique",
            "runtime_soundness_checked_acceptance_unit_values_trace_identity_coverage",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_soundness_checked_acceptance_concrete_artifact_segment_ids_contract",
        &[
            "runtime_soundness_checked_acceptance_artifact_segment_ids_contract",
            "runtime_soundness_checked_acceptance_concrete_segment_ids_allowed",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_soundness_checked_acceptance_core_obligations",
        &["runtime_transcript_binding_checked_acceptance_verifier_core_contract"],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_soundness_checked_acceptance_core_obligations",
        &[
            "runtime_soundness_checked_acceptance_evidence",
            "runtime_soundness_evidence_implies_core_obligations",
            "runtime_soundness_checked_acceptance_sound",
            "sound_witness_implies_verifier_core_contract",
            "sound.right",
        ],
    );
}

fn theorem_prefix(source: &str, name: &str) -> String {
    lean_binding::theorem_prefix(source, name)
}
