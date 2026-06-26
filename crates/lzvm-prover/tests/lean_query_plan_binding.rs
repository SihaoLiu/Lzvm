use std::path::Path;

#[path = "support/lean_binding.rs"]
mod lean_binding;

#[test]
fn lean_query_plan_binding_exports_opening_segment_projections() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_path = crate_root.join("../../lean/Lzvm/QueryPlanBinding.lean");
    let lean_source =
        std::fs::read_to_string(&lean_path).expect("Lean query plan binding source should read");
    let query_plan_path = crate_root.join("src/pcs_query_plan.rs");
    let query_plan_source =
        std::fs::read_to_string(&query_plan_path).expect("PCS query plan source should read");
    let query_plan_build_path = crate_root.join("src/pcs_query_plan/build.rs");
    let query_plan_build_source = std::fs::read_to_string(&query_plan_build_path)
        .expect("PCS query plan build source should read");
    let fri_validation_path = crate_root.join("src/pcs_fri/validation.rs");
    let fri_validation_source =
        std::fs::read_to_string(&fri_validation_path).expect("FRI validation source should read");
    let proof_artifact_path = crate_root.join("src/proof_artifact.rs");
    let proof_artifact_source =
        std::fs::read_to_string(&proof_artifact_path).expect("proof artifact source should read");

    assert!(
        lean_source
            .contains("runtime_query_plan_binding_checked_acceptance_opening_segment_evidence")
            && lean_source.contains(
                "runtime_query_plan_binding_checked_acceptance_opening_segment_bound_contract"
            )
            && lean_source.contains("RuntimeOpeningSegmentBindingEvidence")
            && lean_source.contains("RuntimeOpeningSegmentBindingBoundContract")
            && lean_source.contains("RuntimeOpeningEvidence")
            && lean_source.contains("system.transcriptBound publicInput proof")
            && lean_source.contains("system.pcsOpeningsValid publicInput proof")
            && lean_source.contains("system.friQueriesValid publicInput proof")
            && lean_source.contains("queryPlanTranscriptInputsCanonical")
            && lean_source.contains("RuntimeVerifierCoreContract system publicInput proof"),
        "Lean query plan binding should expose opening segment and verifier core projections"
    );
    assert!(
        lean_source.contains("def RuntimeQueryPlanBindingSeededContract")
            && lean_source.contains("queryPlanSeedBindsWitnessTreeDigests")
            && lean_source.contains("queryPlanSeededFriOpeningRequirementsChecked"),
        "Lean query plan binding should expose seeded query-plan witness digest and FRI-opening checks"
    );
    assert!(
        function_body(
            &lean_source,
            "def RuntimeQueryPlanBindingEvidence",
            "def RuntimeQueryPlanBindingCheckedAcceptance",
        )
        .contains("RuntimeQueryPlanBindingSeededContract"),
        "Lean query plan evidence should retain seeded witness-digest and FRI-opening obligations"
    );
    lean_binding::assert_theorem_declarations(
        &lean_source,
        &[
            "runtime_query_plan_binding_evidence_implies_bound_contract",
            "runtime_query_plan_binding_evidence_implies_transcript_query_plan_bound",
            "runtime_query_plan_binding_evidence_implies_opening_query_plan_bound",
            "runtime_query_plan_binding_evidence_implies_transcript_inputs_canonical",
            "runtime_query_plan_binding_evidence_implies_seeded_contract",
            "runtime_query_plan_binding_seeded_contract_implies_seed_binds_witness_tree_digests",
            "runtime_query_plan_binding_seeded_contract_implies_seeded_fri_opening_requirements_checked",
            "runtime_query_plan_binding_checked_acceptance_bound_contract",
            "runtime_query_plan_binding_checked_acceptance_transcript_query_plan_bound",
            "runtime_query_plan_binding_checked_acceptance_transcript_inputs_canonical",
            "runtime_query_plan_binding_checked_acceptance_opening_query_plan_bound",
            "runtime_query_plan_binding_checked_acceptance_seeded_contract",
            "runtime_query_plan_binding_checked_acceptance_seed_binds_witness_tree_digests",
            "runtime_query_plan_binding_checked_acceptance_seeded_fri_opening_requirements_checked",
            "runtime_query_plan_binding_checked_acceptance_artifact_finalized",
            "runtime_query_plan_binding_checked_acceptance_artifact_structural_obligations",
            "runtime_query_plan_binding_checked_acceptance_segment_ids_unique",
            "runtime_query_plan_binding_checked_acceptance_unit_values_trace_identity_coverage",
            "runtime_query_plan_binding_checked_acceptance_container_canonical",
            "runtime_query_plan_binding_checked_acceptance_metadata_canonical",
            "runtime_query_plan_binding_checked_acceptance_segment_payloads_nonempty",
            "runtime_query_plan_binding_checked_acceptance_segment_ids_allowed",
            "runtime_query_plan_binding_checked_acceptance_segments_present",
            "runtime_query_plan_binding_checked_acceptance_opening_segment_evidence",
            "runtime_query_plan_binding_checked_acceptance_opening_segment_bound_contract",
            "runtime_query_plan_binding_checked_acceptance_pcs_and_fri",
            "runtime_query_plan_binding_checked_acceptance_sound",
            "runtime_query_plan_binding_checked_acceptance_sound_from_hash_concrete_opening",
            "runtime_query_plan_binding_checked_acceptance_sound_from_concrete_nary_merkle",
            "runtime_query_plan_binding_checked_acceptance_verifier_core_contract",
            "runtime_query_plan_binding_checked_acceptance_opening_and_core_contract",
            "runtime_query_plan_binding_checked_acceptance_full_soundness_contract",
            "runtime_query_plan_binding_checked_acceptance_seeded_opening_and_core_contract",
            "runtime_query_plan_binding_checked_acceptance_seeded_concrete_opening_and_core_contract",
            "runtime_query_plan_binding_checked_acceptance_seeded_hash_concrete_opening_and_core_contract",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_query_plan_binding_evidence_implies_transcript_query_plan_bound",
        &[
            "RuntimeQueryPlanBindingEvidence",
            "validation.challengeValidation.transcriptValidation.queryPlanBound",
            "artifact",
            "publicInput",
            "proof",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_query_plan_binding_evidence_implies_opening_query_plan_bound",
        &[
            "RuntimeQueryPlanBindingEvidence",
            "validation.openingValidation.queryPlanBound artifact publicInput proof",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_query_plan_binding_evidence_implies_transcript_inputs_canonical",
        &[
            "RuntimeQueryPlanBindingEvidence",
            "validation.queryPlanTranscriptInputsCanonical artifact publicInput proof",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_query_plan_binding_checked_acceptance_transcript_query_plan_bound",
        &[
            "RuntimeQueryPlanBindingCheckedAcceptance",
            "validation.challengeValidation.transcriptValidation.queryPlanBound",
            "artifact",
            "publicInput",
            "proof",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_query_plan_binding_checked_acceptance_opening_query_plan_bound",
        &[
            "RuntimeQueryPlanBindingCheckedAcceptance",
            "validation.openingValidation.queryPlanBound artifact publicInput proof",
        ],
    );
    lean_binding::assert_theorem_prefix_omits(
        &lean_source,
        "runtime_query_plan_binding_checked_acceptance_transcript_query_plan_bound",
        &["AssumptionBundle"],
    );
    lean_binding::assert_theorem_prefix_omits(
        &lean_source,
        "runtime_query_plan_binding_checked_acceptance_opening_query_plan_bound",
        &["AssumptionBundle"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_query_plan_binding_evidence_implies_transcript_query_plan_bound",
        &["transcriptInputsCanonical"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_query_plan_binding_checked_acceptance_transcript_query_plan_bound",
        &[
            "validation.queryPlanBindingAcceptedImpliesTranscriptInputsCanonical",
            "validation.queryPlanChecksImplyTranscriptQueryPlanBound",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_query_plan_binding_checked_acceptance_transcript_inputs_canonical",
        &["validation.queryPlanBindingAcceptedImpliesTranscriptInputsCanonical"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_query_plan_binding_checked_acceptance_opening_query_plan_bound",
        &["validation.queryPlanChecksImplyOpeningQueryPlanBound"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_query_plan_binding_checked_acceptance_opening_and_core_contract",
        &[
            "RuntimeQueryPlanBindingBoundContract",
            "RuntimeOpeningSegmentBindingBoundContract",
            "RuntimeOpeningEvidence",
            "system.transcriptBound publicInput proof",
            "system.pcsOpeningsValid publicInput proof",
            "system.friQueriesValid publicInput proof",
            "RuntimeVerifierCoreContract system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_query_plan_binding_checked_acceptance_opening_and_core_contract",
        &[
            "runtime_query_plan_binding_checked_acceptance_evidence",
            "runtime_query_plan_binding_checked_acceptance_opening",
            "runtime_opening_segment_binding_checked_acceptance_opening_and_core_contract",
            "runtime_query_plan_binding_checked_acceptance_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_query_plan_binding_checked_acceptance_opening_and_core_contract",
        &[
            "runtime_query_plan_binding_checked_acceptance_sound",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_query_plan_binding_checked_acceptance_full_soundness_contract",
        &[
            "RuntimeQueryPlanBindingEvidence",
            "RuntimeQueryPlanBindingBoundContract",
            "RuntimeChallengeSegmentBindingEvidence",
            "RuntimeOpeningSegmentBindingEvidence",
            "RuntimeOpeningSegmentBindingBoundContract",
            "RuntimeOpeningEvidence",
            "RuntimeOpeningBoundContract",
            "system.transcriptBound publicInput proof",
            "system.pcsOpeningsValid publicInput proof",
            "system.friQueriesValid publicInput proof",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_query_plan_binding_checked_acceptance_full_soundness_contract",
        &[
            "runtime_query_plan_binding_checked_acceptance_sound",
            "runtime_opening_evidence_implies_bound_contract",
            "runtime_query_plan_binding_checked_acceptance_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_query_plan_binding_checked_acceptance_full_soundness_contract",
        &["runtime_opening_segment_binding_checked_acceptance_full_soundness_contract"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_query_plan_binding_checked_acceptance_seeded_contract",
        &[
            "RuntimeQueryPlanBindingCheckedAcceptance",
            "RuntimeQueryPlanBindingSeededContract",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_query_plan_binding_seeded_contract_implies_seed_binds_witness_tree_digests",
        &[
            "RuntimeQueryPlanBindingSeededContract",
            "validation.queryPlanSeedBindsWitnessTreeDigests",
            "artifact",
            "publicInput",
            "proof",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_query_plan_binding_seeded_contract_implies_seeded_fri_opening_requirements_checked",
        &[
            "RuntimeQueryPlanBindingSeededContract",
            "validation.queryPlanSeededFriOpeningRequirementsChecked",
            "artifact",
            "publicInput",
            "proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_query_plan_binding_checked_acceptance_seed_binds_witness_tree_digests",
        &[
            "runtime_query_plan_binding_checked_acceptance_seeded_contract",
            "runtime_query_plan_binding_seeded_contract_implies_seed_binds_witness_tree_digests",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_query_plan_binding_checked_acceptance_seeded_fri_opening_requirements_checked",
        &[
            "runtime_query_plan_binding_checked_acceptance_seeded_contract",
            "runtime_query_plan_binding_seeded_contract_implies_seeded_fri_opening_requirements_checked",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_query_plan_binding_checked_acceptance_segment_ids_unique",
        &[
            "RuntimeQueryPlanBindingCheckedAcceptance",
            "let artifactValidation :=",
            "validation.challengeValidation.transcriptValidation.artifactBindingValidation",
            "artifactValidation.proofSegmentIdsUnique artifact publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_query_plan_binding_checked_acceptance_artifact_finalized",
        &[
            "runtime_query_plan_binding_checked_acceptance_challenge",
            "runtime_challenge_segment_binding_checked_acceptance_artifact_finalized",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_query_plan_binding_checked_acceptance_artifact_structural_obligations",
        &[
            "runtime_query_plan_binding_checked_acceptance_artifact_finalized",
            "runtime_proof_artifact_finalized_structural_obligations",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_query_plan_binding_checked_acceptance_segment_ids_unique",
        &[
            "runtime_query_plan_binding_checked_acceptance_artifact_structural_obligations",
            "artifactStructural.right.right.right.right.right.left",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_query_plan_binding_checked_acceptance_unit_values_trace_identity_coverage",
        &[
            "RuntimeQueryPlanBindingCheckedAcceptance",
            "let artifactValidation :=",
            "validation.challengeValidation.transcriptValidation.artifactBindingValidation",
            "artifactValidation.proofUnitValuesTraceIdentityCoverage artifact publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_query_plan_binding_checked_acceptance_unit_values_trace_identity_coverage",
        &[
            "runtime_query_plan_binding_checked_acceptance_artifact_structural_obligations",
            "artifactStructural.right.right.right.right.right.right",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_query_plan_binding_checked_acceptance_container_canonical",
        &[
            "RuntimeQueryPlanBindingCheckedAcceptance",
            "let artifactValidation :=",
            "validation.challengeValidation.transcriptValidation.artifactBindingValidation",
            "artifactValidation.proofContainerCanonical artifact publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_query_plan_binding_checked_acceptance_container_canonical",
        &[
            "runtime_query_plan_binding_checked_acceptance_artifact_structural_obligations",
            "artifactStructural.left",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_query_plan_binding_checked_acceptance_metadata_canonical",
        &[
            "RuntimeQueryPlanBindingCheckedAcceptance",
            "let artifactValidation :=",
            "validation.challengeValidation.transcriptValidation.artifactBindingValidation",
            "artifactValidation.proofMetadataCanonical artifact publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_query_plan_binding_checked_acceptance_metadata_canonical",
        &[
            "runtime_query_plan_binding_checked_acceptance_artifact_structural_obligations",
            "artifactStructural.right.left",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_query_plan_binding_checked_acceptance_segment_payloads_nonempty",
        &[
            "RuntimeQueryPlanBindingCheckedAcceptance",
            "let artifactValidation :=",
            "validation.challengeValidation.transcriptValidation.artifactBindingValidation",
            "artifactValidation.proofSegmentPayloadsNonempty artifact publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_query_plan_binding_checked_acceptance_segment_payloads_nonempty",
        &[
            "runtime_query_plan_binding_checked_acceptance_artifact_structural_obligations",
            "artifactStructural.right.right.right.left",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_query_plan_binding_checked_acceptance_segment_ids_allowed",
        &[
            "RuntimeQueryPlanBindingCheckedAcceptance",
            "let artifactValidation :=",
            "validation.challengeValidation.transcriptValidation.artifactBindingValidation",
            "artifactValidation.proofSegmentIdsAllowed artifact publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_query_plan_binding_checked_acceptance_segment_ids_allowed",
        &[
            "runtime_query_plan_binding_checked_acceptance_artifact_structural_obligations",
            "artifactStructural.right.right.right.right.left",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_query_plan_binding_checked_acceptance_segments_present",
        &[
            "RuntimeQueryPlanBindingCheckedAcceptance",
            "let artifactValidation :=",
            "validation.challengeValidation.transcriptValidation.artifactBindingValidation",
            "artifactValidation.proofSegmentsPresent artifact publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_query_plan_binding_checked_acceptance_segments_present",
        &[
            "runtime_query_plan_binding_checked_acceptance_artifact_structural_obligations",
            "artifactStructural.right.right.left",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_query_plan_binding_checked_acceptance_seeded_opening_and_core_contract",
        &[
            "RuntimeQueryPlanBindingCheckedAcceptance",
            "RuntimeQueryPlanBindingSeededContract",
            "RuntimeQueryPlanBindingBoundContract",
            "RuntimeOpeningSegmentBindingBoundContract",
            "RuntimeOpeningEvidence",
            "system.transcriptBound publicInput proof",
            "system.pcsOpeningsValid publicInput proof",
            "system.friQueriesValid publicInput proof",
            "RuntimeVerifierCoreContract system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_query_plan_binding_checked_acceptance_seeded_opening_and_core_contract",
        &[
            "runtime_query_plan_binding_checked_acceptance_seeded_contract",
            "runtime_query_plan_binding_checked_acceptance_opening_and_core_contract",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_query_plan_binding_checked_acceptance_seeded_concrete_opening_and_core_contract",
        &[
            "RuntimeQueryPlanBindingCheckedAcceptance",
            "RuntimeQueryPlanBindingSeededContract",
            "RuntimeQueryPlanBindingBoundContract",
            "RuntimeOpeningSegmentBindingBoundContract",
            "RuntimeOpeningEvidence",
            "RuntimeConstantOpeningNAryConcreteBinding",
            "RuntimeWitnessOpeningNAryConcreteBinding",
            "CentralizedNAryMerkleCompressionCollisionResistance",
            "system.transcriptBound publicInput proof",
            "system.publicInputBound publicInput proof",
            "system.pcsOpeningsValid publicInput proof",
            "system.friQueriesValid publicInput proof",
            "RuntimeVerifierCoreContract system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_query_plan_binding_checked_acceptance_seeded_concrete_opening_and_core_contract",
        &[
            "runtime_query_plan_binding_checked_acceptance_seeded_contract",
            "runtime_query_plan_binding_checked_acceptance_sound_from_concrete_nary_merkle",
            "runtime_query_plan_binding_checked_acceptance_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_query_plan_binding_checked_acceptance_seeded_concrete_opening_and_core_contract",
        &[
            "runtime_query_plan_binding_checked_acceptance_opening_and_core_contract",
            "runtime_opening_segment_binding_checked_acceptance_pcs_and_fri_from_hash_assumption_concrete_nary_merkle",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_query_plan_binding_checked_acceptance_seeded_hash_concrete_opening_and_core_contract",
        &[
            "AssumptionBundle system",
            "HashCollisionResistanceAssumption",
            "CentralizedNAryMerkleCompressionCollisionResistance",
            "hashAssumptions",
            "RuntimeConstantOpeningNAryConcreteBinding",
            "RuntimeWitnessOpeningNAryConcreteBinding",
            "RuntimeQueryPlanBindingCheckedAcceptance",
            "RuntimeQueryPlanBindingSeededContract",
            "RuntimeQueryPlanBindingBoundContract",
            "RuntimeOpeningSegmentBindingBoundContract",
            "RuntimeOpeningEvidence",
            "system.transcriptBound publicInput proof",
            "system.publicInputBound publicInput proof",
            "system.pcsOpeningsValid publicInput proof",
            "system.friQueriesValid publicInput proof",
            "RuntimeVerifierCoreContract system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_query_plan_binding_checked_acceptance_seeded_hash_concrete_opening_and_core_contract",
        &[
            "runtime_query_plan_binding_checked_acceptance_seeded_contract",
            "runtime_query_plan_binding_checked_acceptance_sound_from_hash_concrete_opening",
            "runtime_query_plan_binding_checked_acceptance_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_query_plan_binding_checked_acceptance_seeded_hash_concrete_opening_and_core_contract",
        &[
            "runtime_query_plan_binding_checked_acceptance_opening_and_core_contract",
            "assumptions.crypto.hashCollisionResistance",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_query_plan_binding_checked_acceptance_sound_from_hash_concrete_opening",
        &[
            "HashCollisionResistanceAssumption",
            "RuntimeConstantOpeningNAryConcreteBinding",
            "RuntimeWitnessOpeningNAryConcreteBinding",
            "RuntimeQueryPlanBindingEvidence",
            "RuntimeOpeningEvidence",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_query_plan_binding_checked_acceptance_sound_from_hash_concrete_opening",
        &[
            "runtime_challenge_segment_binding_checked_acceptance_sound",
            "runtime_opening_segment_binding_checked_acceptance_sound_from_hash_concrete_opening",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_query_plan_binding_checked_acceptance_sound_from_hash_concrete_opening",
        &["runtime_opening_segment_binding_checked_acceptance_sound\n"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_query_plan_binding_checked_acceptance_sound",
        &[
            "runtime_opening_segment_binding_checked_acceptance_full_soundness_contract",
            "openingFull.right.left",
            "openingFull.right.right.right.left",
            "openingFull.right.right.right.right.left",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_query_plan_binding_checked_acceptance_sound",
        &["runtime_opening_segment_binding_checked_acceptance_sound\n"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_query_plan_binding_checked_acceptance_sound_from_concrete_nary_merkle",
        &["runtime_query_plan_binding_checked_acceptance_sound_from_hash_concrete_opening"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_query_plan_binding_checked_acceptance_pcs_and_fri",
        &[
            "RuntimeQueryPlanBindingCheckedAcceptance",
            "system.pcsOpeningsValid publicInput proof",
            "system.friQueriesValid publicInput proof",
        ],
    );
    lean_binding::assert_theorem_prefix_omits(
        &lean_source,
        "runtime_query_plan_binding_checked_acceptance_pcs_and_fri",
        &["AssumptionBundle"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_query_plan_binding_evidence_implies_seeded_contract",
        &[
            "RuntimeQueryPlanBindingEvidence",
            "RuntimeQueryPlanBindingSeededContract",
        ],
    );
    assert!(
        query_plan_source.contains("validate_seeded_pcs_query_plan_segments")
            && query_plan_source.contains("build_pcs_query_plan_segment_with_bindings")
            && query_plan_build_source
                .contains("hash_loaded_witness_commitment_segment_for_query_seed")
            && query_plan_build_source.contains("stage.tree_digest")
            && fri_validation_source.contains("seeded_query_plan_requires_fri_opening")
            && fri_validation_source.contains("fri_opening_required_units"),
        "runtime seeded query-plan validation should bind witness tree digests and require FRI openings for FRI-bearing seeded units"
    );
    let transcript_builder_body = function_body(
        &proof_artifact_source,
        "fn build_witness_transcript_proof_artifact_for_all_units",
        "struct AllUnitsTranscriptProofInputs",
    );
    assert!(
        proof_artifact_source
            .contains("canonical_witness_trace_output_refs(request.schedule, request.outputs)")
            && transcript_builder_body.contains("let transcript_inputs = witness_outputs")
            && !transcript_builder_body.contains("request.outputs"),
        "runtime all-units transcript query-plan derivation should use canonical output refs"
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_query_plan_binding_checked_acceptance_opening_and_core_contract",
        &[".right.right.right"],
    );
}

fn function_body(source: &str, start: &str, end: &str) -> String {
    let start_index = source
        .find(start)
        .unwrap_or_else(|| panic!("source should contain {start}"));
    let rest = &source[start_index..];
    let end_index = rest
        .find(end)
        .unwrap_or_else(|| panic!("source should contain {end} after {start}"));
    rest[..end_index].to_owned()
}
