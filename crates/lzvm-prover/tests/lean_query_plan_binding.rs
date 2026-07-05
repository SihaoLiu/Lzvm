use std::path::Path;

#[path = "support/lean_binding.rs"]
mod lean_binding;

#[test]
fn lean_query_plan_binding_exports_opening_segment_projections() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_source = lean_binding::read_lean_sources(
        crate_root,
        &[
            "../../lean/Lzvm/QueryPlanBinding.lean",
            "../../lean/Lzvm/QueryPlanBinding/Core.lean",
            "../../lean/Lzvm/QueryPlanBinding/Soundness.lean",
        ],
    );
    let top_level_path = crate_root.join("../../lean/Lzvm.lean");
    let top_level_source =
        std::fs::read_to_string(&top_level_path).expect("Lean top-level source should read");
    let query_plan_path = crate_root.join("src/pcs_query_plan.rs");
    let query_plan_source =
        std::fs::read_to_string(&query_plan_path).expect("PCS query plan source should read");
    let query_plan_build_path = crate_root.join("src/pcs_query_plan/build.rs");
    let query_plan_build_source = std::fs::read_to_string(&query_plan_build_path)
        .expect("PCS query plan build source should read");
    let material_manifest_path = crate_root.join("src/pcs_material_manifest.rs");
    let material_manifest_source = std::fs::read_to_string(&material_manifest_path)
        .expect("PCS material manifest source should read");
    let query_plan_tests_path = crate_root.join("tests/pcs_query_plan.rs");
    let query_plan_tests_source = std::fs::read_to_string(&query_plan_tests_path)
        .expect("PCS query plan tests source should read");
    let fri_validation_path = crate_root.join("src/pcs_fri/validation.rs");
    let fri_validation_source =
        std::fs::read_to_string(&fri_validation_path).expect("FRI validation source should read");
    let proof_artifact_path = crate_root.join("src/proof_artifact.rs");
    let proof_artifact_source =
        std::fs::read_to_string(&proof_artifact_path).expect("proof artifact source should read");
    let parsed_material_manifest_body = function_body(
        &material_manifest_source,
        "pub(crate) fn validate_parsed_pcs_material_manifest_matches_schedule",
        "pub fn build_pcs_material_manifest_segment",
    );
    let material_manifest_unit_body =
        source_from(&material_manifest_source, "fn validate_manifest_unit");
    let query_plan_evidence_body = function_body(
        &lean_source,
        "def RuntimeQueryPlanBindingEvidence",
        "def RuntimeQueryPlanBindingCheckedAcceptance",
    );

    assert!(
        lean_binding::contains_import(&top_level_source, "Lzvm.QueryPlanBinding"),
        "top-level Lean module should import query plan binding"
    );
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
            && lean_source.contains("queryPlanMaterialManifestMatchesSchedule")
            && lean_source.contains("RuntimeQueryPlanMaterialManifestContract")
            && lean_source.contains("RuntimeVerifierCoreContract system publicInput proof"),
        "Lean query plan binding should expose opening segment and verifier core projections"
    );
    assert!(
        query_plan_source.matches("validate_material_manifest_matches_schedule(schedule, &material)?").count()
            == 2
            && query_plan_source
                .contains("validate_parsed_pcs_material_manifest_matches_schedule")
            && parsed_material_manifest_body.contains("manifest.units.len() != schedule.units.len()")
            && parsed_material_manifest_body.contains("manifest_unit.unit_index != expected_unit_index")
            && material_manifest_unit_body.contains("manifest.plan_digest != plan_digest")
            && material_manifest_unit_body
                .contains("manifest.fixed_column_digest != fixed_column_digest")
            && material_manifest_unit_body
                .contains("manifest.constant_tree_digest != constant_tree_digest")
            && material_manifest_unit_body.contains("manifest.constant_tree_root != constant_tree_root")
            && material_manifest_unit_body.contains("manifest.fixed_byte_count != fixed_byte_count")
            && material_manifest_unit_body
                .contains("manifest.constant_tree_byte_count != constant_tree_byte_count")
            && material_manifest_unit_body.contains("manifest.leaf_byte_count != leaf_byte_count")
            && material_manifest_unit_body.contains("manifest.node_byte_count != node_byte_count"),
        "runtime query-plan validation should require exact PCS material manifest schedule matching"
    );
    assert!(
        lean_source.contains("def RuntimeQueryPlanBindingSeededContract")
            && lean_source.contains("queryPlanSeedBindsWitnessTreeDigests")
            && lean_source.contains("queryPlanSeededFriOpeningRequirementsChecked"),
        "Lean query plan binding should expose seeded query-plan witness digest and FRI-opening checks"
    );
    assert!(
        query_plan_evidence_body.contains("RuntimeQueryPlanBindingSeededContract")
            && query_plan_evidence_body.contains("RuntimeQueryPlanMaterialManifestContract"),
        "Lean query plan evidence should retain material manifest, witness-digest, and FRI-opening obligations"
    );
    lean_binding::assert_theorem_declarations(
        &lean_source,
        &[
            "runtime_query_plan_binding_evidence_implies_bound_contract",
            "runtime_query_plan_binding_evidence_implies_transcript_query_plan_bound",
            "runtime_query_plan_binding_evidence_implies_opening_query_plan_bound",
            "runtime_query_plan_binding_evidence_implies_transcript_inputs_canonical",
            "runtime_query_plan_binding_evidence_implies_seeded_contract",
            "runtime_query_plan_binding_evidence_implies_material_manifest_contract",
            "runtime_query_plan_material_manifest_contract_implies_segment_canonical",
            "runtime_query_plan_material_manifest_contract_implies_matches_schedule",
            "runtime_query_plan_binding_evidence_implies_segment_canonical",
            "runtime_query_plan_binding_evidence_implies_material_manifest_matches_schedule",
            "runtime_query_plan_binding_seeded_contract_implies_seed_binds_witness_tree_digests",
            "runtime_query_plan_binding_seeded_contract_implies_seeded_fri_opening_requirements_checked",
            "runtime_query_plan_binding_checked_acceptance_bound_contract",
            "runtime_query_plan_binding_checked_acceptance_transcript_query_plan_bound",
            "runtime_query_plan_binding_checked_acceptance_transcript_inputs_canonical",
            "runtime_query_plan_binding_checked_acceptance_material_manifest_contract",
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
            "runtime_query_plan_binding_checked_acceptance_concrete_segment_ids_allowed",
            "runtime_query_plan_binding_checked_acceptance_segments_present",
            "runtime_query_plan_binding_checked_acceptance_opening_segment_evidence",
            "runtime_query_plan_binding_checked_acceptance_opening_segment_bound_contract",
            "runtime_query_plan_binding_checked_acceptance_pcs_and_fri",
            "runtime_query_plan_binding_checked_acceptance_sound",
            "runtime_query_plan_binding_checked_acceptance_sound_from_hash_concrete_opening",
            "runtime_query_plan_binding_checked_acceptance_sound_from_concrete_nary_merkle",
            "runtime_query_plan_binding_checked_acceptance_verifier_core_contract",
            "runtime_query_plan_binding_checked_acceptance_evidence_core_and_sound",
            "runtime_query_plan_binding_checked_acceptance_concrete_core_sound_contract",
            "runtime_query_plan_binding_checked_acceptance_opening_and_core_contract",
            "runtime_query_plan_binding_checked_acceptance_full_soundness_contract",
            "runtime_query_plan_binding_checked_acceptance_seeded_opening_and_core_contract",
            "runtime_query_plan_binding_checked_acceptance_seeded_concrete_opening_and_core_contract",
            "runtime_query_plan_binding_checked_acceptance_seeded_hash_concrete_opening_and_core_contract",
            "runtime_query_plan_binding_audited_finalized_core_sound_witness_contract",
            "runtime_query_plan_binding_audited_finalized_manifest_core_sound_witness_contract",
            "runtime_query_plan_binding_audited_finalized_segment_ids_contract",
            "runtime_query_plan_binding_audited_finalized_concrete_segment_ids_contract",
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
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_query_plan_binding_evidence_implies_material_manifest_contract",
        &[
            "RuntimeQueryPlanBindingEvidence",
            "RuntimeQueryPlanMaterialManifestContract",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_query_plan_binding_checked_acceptance_material_manifest_contract",
        &[
            "RuntimeQueryPlanBindingCheckedAcceptance",
            "RuntimeQueryPlanMaterialManifestContract",
        ],
    );
    lean_binding::assert_theorem_prefix_omits(
        &lean_source,
        "runtime_query_plan_binding_checked_acceptance_material_manifest_contract",
        &["AssumptionBundle"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_query_plan_binding_checked_acceptance_material_manifest_contract",
        &[
            "validation.queryPlanBindingAcceptedImpliesSegmentCanonical",
            "validation.queryPlanBindingAcceptedImpliesMaterialManifestMatchesSchedule",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_query_plan_material_manifest_contract_implies_segment_canonical",
        &[
            "RuntimeQueryPlanMaterialManifestContract",
            "validation.queryPlanSegmentCanonical artifact publicInput proof",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_query_plan_material_manifest_contract_implies_matches_schedule",
        &[
            "RuntimeQueryPlanMaterialManifestContract",
            "validation.queryPlanMaterialManifestMatchesSchedule artifact publicInput proof",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_query_plan_binding_evidence_implies_segment_canonical",
        &[
            "RuntimeQueryPlanBindingEvidence",
            "validation.queryPlanSegmentCanonical artifact publicInput proof",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_query_plan_binding_evidence_implies_material_manifest_matches_schedule",
        &[
            "RuntimeQueryPlanBindingEvidence",
            "validation.queryPlanMaterialManifestMatchesSchedule artifact publicInput proof",
        ],
    );
    for name in [
        "runtime_query_plan_binding_evidence_implies_segment_canonical",
        "runtime_query_plan_binding_evidence_implies_material_manifest_matches_schedule",
    ] {
        lean_binding::assert_theorem_body_contains(
            &lean_source,
            name,
            &["runtime_query_plan_binding_evidence_implies_material_manifest_contract"],
        );
    }
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
        "runtime_query_plan_binding_checked_acceptance_evidence_core_and_sound",
        &[
            "RuntimeQueryPlanBindingEvidence",
            "RuntimeChallengeSegmentBindingEvidence",
            "RuntimeOpeningSegmentBindingEvidence",
            "RuntimeOpeningEvidence",
            "system.transcriptBound publicInput proof",
            "system.pcsOpeningsValid publicInput proof",
            "system.friQueriesValid publicInput proof",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_query_plan_binding_checked_acceptance_evidence_core_and_sound",
        &[
            "runtime_query_plan_binding_checked_acceptance_sound",
            "runtime_query_plan_binding_checked_acceptance_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_query_plan_binding_checked_acceptance_evidence_core_and_sound",
        &["sound_witness_implies_verifier_core_contract"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_query_plan_binding_checked_acceptance_concrete_core_sound_contract",
        &[
            "RuntimeQueryPlanBindingCheckedAcceptance",
            "RuntimeProofArtifactConcreteSegmentIdBinding",
            "validation.challengeValidation.transcriptValidation.artifactBindingValidation",
            "RuntimeQueryPlanBindingEvidence",
            "RuntimeChallengeSegmentBindingEvidence",
            "RuntimeOpeningSegmentBindingEvidence",
            "RuntimeOpeningEvidence",
            "system.transcriptBound publicInput proof",
            "system.pcsOpeningsValid publicInput proof",
            "system.friQueriesValid publicInput proof",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
            "RuntimeProofArtifactConcreteSegmentIdsAllowed proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_query_plan_binding_checked_acceptance_concrete_core_sound_contract",
        &[
            "runtime_query_plan_binding_checked_acceptance_evidence_core_and_sound",
            "runtime_query_plan_binding_checked_acceptance_concrete_segment_ids_allowed",
        ],
    );
    for identifier in [
        "runtime_query_plan_binding_checked_acceptance_sound",
        "sound_witness_implies_verifier_core_contract",
        "abstract_verifier_sound",
    ] {
        lean_binding::assert_theorem_body_omits_identifier(
            &lean_source,
            "runtime_query_plan_binding_checked_acceptance_concrete_core_sound_contract",
            identifier,
        );
    }
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
        "runtime_query_plan_binding_audited_finalized_core_sound_witness_contract",
        &[
            "RuntimeQueryPlanBindingCheckedAcceptance",
            "RequiredCryptographicAssumptionStatements assumptions.crypto",
            "RequiredSemanticAssumptionStatements assumptions.semantic",
            "RuntimeQueryPlanBindingEvidence",
            "RuntimeProofArtifactFinalized",
            "validation.queryPlanSeedBindsWitnessTreeDigests artifact publicInput proof",
            "validation.queryPlanSeededFriOpeningRequirementsChecked",
            "RuntimeVerifierCoreContract system publicInput proof",
            "system.traceConsistent publicInput proof trace",
            "system.constraintsSatisfied constraints trace",
            "system.witnessMatchesTrace witness trace",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_prefix_omits(
        &lean_source,
        "runtime_query_plan_binding_audited_finalized_core_sound_witness_contract",
        &[
            "RuntimeOpeningEvidence",
            "RuntimeOpeningSegmentBindingEvidence",
            "requiresExternalSource",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_query_plan_binding_audited_finalized_core_sound_witness_contract",
        &[
            "runtime_query_plan_binding_checked_acceptance_evidence",
            "runtime_query_plan_binding_checked_acceptance_artifact_finalized",
            "runtime_query_plan_binding_checked_acceptance_seed_binds_witness_tree_digests",
            "runtime_query_plan_binding_checked_acceptance_seeded_fri_opening_requirements_checked",
            "runtime_query_plan_binding_checked_acceptance_challenge",
            "runtime_challenge_segment_binding_checked_acceptance_transcript",
            "runtime_proof_artifact_binding_checked_acceptance_runtime_accepted",
            "runtime_artifact_checked_acceptance_implies_verifier_accepts",
            "accepted_proof_audited_core_execution_and_sound_witness",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_query_plan_binding_audited_finalized_core_sound_witness_contract",
        &[
            "accepted_proof_audited_core_and_sound_witness",
            "sound_witness_implies_execution_obligations",
            "runtime_query_plan_binding_checked_acceptance_evidence_core_and_sound",
            "runtime_query_plan_binding_checked_acceptance_full_soundness_contract",
            "runtime_query_plan_binding_checked_acceptance_verifier_core_contract",
            "runtime_opening_segment_binding_checked_acceptance_full_soundness_contract",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_query_plan_binding_audited_finalized_manifest_core_sound_witness_contract",
        &[
            "RuntimeQueryPlanMaterialManifestContract",
            "validation.queryPlanSegmentCanonical artifact publicInput proof",
            "validation.queryPlanMaterialManifestMatchesSchedule artifact publicInput proof",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_query_plan_binding_audited_finalized_manifest_core_sound_witness_contract",
        &[
            "runtime_query_plan_binding_audited_finalized_core_sound_witness_contract",
            "runtime_query_plan_binding_evidence_implies_material_manifest_contract",
            "runtime_query_plan_material_manifest_contract_implies_segment_canonical",
            "runtime_query_plan_material_manifest_contract_implies_matches_schedule",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_query_plan_binding_audited_finalized_segment_ids_contract",
        &[
            "RuntimeQueryPlanBindingCheckedAcceptance",
            "RequiredCryptographicAssumptionStatements assumptions.crypto",
            "RequiredSemanticAssumptionStatements assumptions.semantic",
            "RuntimeQueryPlanBindingEvidence",
            "RuntimeProofArtifactFinalized",
            "validation.queryPlanSeedBindsWitnessTreeDigests artifact publicInput proof",
            "validation.queryPlanSeededFriOpeningRequirementsChecked",
            "RuntimeProofArtifactBindingStructuralObligations",
            "RuntimeVerifierCoreContract system publicInput proof",
            "system.traceConsistent publicInput proof trace",
            "system.constraintsSatisfied constraints trace",
            "system.witnessMatchesTrace witness trace",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_query_plan_binding_audited_finalized_segment_ids_contract",
        &[
            "runtime_query_plan_binding_audited_finalized_core_sound_witness_contract",
            "runtime_query_plan_binding_checked_acceptance_artifact_structural_obligations",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_query_plan_binding_audited_finalized_segment_ids_contract",
        &[
            "runtime_query_plan_binding_checked_acceptance_evidence_core_and_sound",
            "runtime_query_plan_binding_checked_acceptance_full_soundness_contract",
            "runtime_opening_segment_binding_checked_acceptance_full_soundness_contract",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_query_plan_binding_audited_finalized_concrete_segment_ids_contract",
        &[
            "RuntimeQueryPlanBindingCheckedAcceptance",
            "RuntimeProofArtifactConcreteSegmentIdBinding",
            "RequiredCryptographicAssumptionStatements assumptions.crypto",
            "RequiredSemanticAssumptionStatements assumptions.semantic",
            "RuntimeQueryPlanBindingEvidence",
            "RuntimeProofArtifactFinalized",
            "validation.queryPlanSeedBindsWitnessTreeDigests artifact publicInput proof",
            "validation.queryPlanSeededFriOpeningRequirementsChecked",
            "RuntimeProofArtifactBindingStructuralObligations",
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
        "runtime_query_plan_binding_audited_finalized_concrete_segment_ids_contract",
        &[
            "runtime_query_plan_binding_audited_finalized_segment_ids_contract",
            "runtime_query_plan_binding_checked_acceptance_concrete_segment_ids_allowed",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_query_plan_binding_audited_finalized_concrete_segment_ids_contract",
        &[
            "runtime_query_plan_binding_checked_acceptance_evidence_core_and_sound",
            "runtime_query_plan_binding_checked_acceptance_full_soundness_contract",
            "runtime_opening_segment_binding_checked_acceptance_full_soundness_contract",
        ],
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
        &["runtime_query_plan_binding_checked_acceptance_artifact_structural_obligations"],
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
        &["runtime_query_plan_binding_checked_acceptance_artifact_structural_obligations"],
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
        &["runtime_query_plan_binding_checked_acceptance_artifact_structural_obligations"],
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
        &["runtime_query_plan_binding_checked_acceptance_artifact_structural_obligations"],
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
        &["runtime_query_plan_binding_checked_acceptance_artifact_structural_obligations"],
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
        &["runtime_query_plan_binding_checked_acceptance_artifact_structural_obligations"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_query_plan_binding_checked_acceptance_concrete_segment_ids_allowed",
        &[
            "RuntimeQueryPlanBindingCheckedAcceptance",
            "RuntimeProofArtifactConcreteSegmentIdBinding",
            "RuntimeProofArtifactConcreteSegmentIdsAllowed proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_query_plan_binding_checked_acceptance_concrete_segment_ids_allowed",
        &[
            "validation.queryPlanBindingAcceptedImpliesChallengeAccepted",
            "runtime_challenge_segment_binding_checked_acceptance_concrete_segment_ids_allowed",
        ],
    );
    lean_binding::assert_theorem_prefix_omits(
        &lean_source,
        "runtime_query_plan_binding_checked_acceptance_concrete_segment_ids_allowed",
        &["AssumptionBundle"],
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
        &["runtime_query_plan_binding_checked_acceptance_artifact_structural_obligations"],
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
        &["runtime_opening_segment_binding_checked_acceptance_full_soundness_contract"],
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
    assert!(
        query_plan_source.contains("fn is_proof_binding_id")
            && query_plan_source.contains("ETH_BLOCK_INPUT_SEGMENT_ID")
            && query_plan_source.contains("FRAMED_GUEST_INPUT_SEGMENT_ID")
            && query_plan_tests_source.contains(
                "rejects_seeded_pcs_query_plan_mismatches_with_pipeline_input_binding_segments"
            )
            && query_plan_tests_source.contains(
                "rejects_transcript_pcs_query_plan_mismatches_with_pipeline_input_binding_segments"
            ),
        "runtime query-plan binding should include and test pipeline input proof segment IDs"
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

fn source_from(source: &str, start: &str) -> String {
    let start_index = source
        .find(start)
        .unwrap_or_else(|| panic!("source should contain {start}"));
    source[start_index..].to_owned()
}
