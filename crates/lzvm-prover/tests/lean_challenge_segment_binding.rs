use std::path::Path;

#[path = "support/lean_binding.rs"]
mod lean_binding;

#[test]
fn lean_challenge_segment_binding_exports_core_contract_projection() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_path = crate_root.join("../../lean/Lzvm/ChallengeSegmentBinding.lean");
    let lean_source =
        std::fs::read_to_string(&lean_path).expect("Lean challenge binding source should read");
    let top_level_path = crate_root.join("../../lean/Lzvm.lean");
    let top_level_source =
        std::fs::read_to_string(&top_level_path).expect("Lean top-level source should read");

    assert!(
        lean_binding::contains_import(&top_level_source, "Lzvm.ChallengeSegmentBinding"),
        "top-level Lean module should import challenge segment binding"
    );
    assert!(
        lean_source.contains("RuntimeChallengeSegmentBindingValidation")
            && lean_source.contains("RuntimeChallengeSegmentBindingEvidence")
            && lean_source.contains("RuntimeChallengeQueryDerivationContract")
            && lean_source.contains("RuntimeChallengeSegmentPayloadReuseContract")
            && lean_source.contains("RuntimeTranscriptBindingEvidence")
            && lean_source.contains("RuntimeProofArtifactFinalized")
            && lean_source.contains("RuntimeProofArtifactBindingStructuralObligations")
            && lean_source.contains("system.transcriptBound publicInput proof")
            && lean_source.contains("RuntimeVerifierCoreContract system publicInput proof")
            && lean_source.contains("SoundWitness system publicInput proof"),
        "Lean challenge segment binding should expose transcript evidence and verifier core clauses"
    );
    lean_binding::assert_theorem_declarations(
        &lean_source,
        &[
            "runtime_challenge_segment_binding_evidence_implies_payload_valid",
            "runtime_challenge_segment_binding_evidence_implies_segment_matches_transcript",
            "runtime_challenge_segment_binding_evidence_implies_challenge_segment_bound",
            "runtime_challenge_segment_binding_checked_acceptance_artifact_finalized",
            "runtime_challenge_segment_binding_checked_acceptance_artifact_structural_obligations",
            "runtime_challenge_segment_binding_checked_acceptance_payload_valid",
            "runtime_challenge_segment_binding_checked_acceptance_segment_matches_transcript",
            "runtime_challenge_segment_binding_checked_acceptance_challenge_segment_bound",
            "runtime_challenge_segment_binding_checked_acceptance_segment_ids_unique",
            "runtime_challenge_segment_binding_checked_acceptance_unit_values_trace_identity_coverage",
            "runtime_challenge_segment_binding_checked_acceptance_payload_reuse_contract",
            "runtime_challenge_segment_binding_checked_acceptance_query_derivation_contract",
            "runtime_challenge_segment_binding_checked_acceptance_container_canonical",
            "runtime_challenge_segment_binding_checked_acceptance_metadata_canonical",
            "runtime_challenge_segment_binding_checked_acceptance_segment_payloads_nonempty",
            "runtime_challenge_segment_binding_checked_acceptance_segment_ids_allowed",
            "runtime_challenge_segment_binding_checked_acceptance_concrete_segment_ids_allowed",
            "runtime_challenge_segment_binding_checked_acceptance_segments_present",
            "runtime_challenge_segment_binding_checked_acceptance_transcript_payload_contract",
            "runtime_challenge_segment_binding_checked_acceptance_sound",
            "runtime_challenge_segment_binding_checked_acceptance_verifier_core_contract",
            "runtime_challenge_segment_binding_checked_acceptance_challenge_and_core_contract",
            "runtime_challenge_segment_binding_checked_acceptance_evidence_core_and_sound",
            "runtime_challenge_segment_binding_checked_acceptance_concrete_core_sound_contract",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_challenge_segment_binding_evidence_implies_payload_valid",
        &[
            "RuntimeChallengeSegmentBindingEvidence",
            "validation.challengeSegmentPayloadValid artifact publicInput proof",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_challenge_segment_binding_evidence_implies_segment_matches_transcript",
        &[
            "RuntimeChallengeSegmentBindingEvidence",
            "validation.challengeSegmentMatchesTranscript artifact publicInput proof",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_challenge_segment_binding_evidence_implies_challenge_segment_bound",
        &[
            "RuntimeChallengeSegmentBindingEvidence",
            "validation.transcriptValidation.challengeSegmentBound artifact publicInput proof",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_challenge_segment_binding_checked_acceptance_challenge_segment_bound",
        &[
            "RuntimeChallengeSegmentBindingCheckedAcceptance",
            "validation.transcriptValidation.challengeSegmentBound artifact publicInput proof",
        ],
    );
    lean_binding::assert_theorem_prefix_omits(
        &lean_source,
        "runtime_challenge_segment_binding_checked_acceptance_challenge_segment_bound",
        &["AssumptionBundle"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_challenge_segment_binding_checked_acceptance_artifact_finalized",
        &[
            "runtime_challenge_segment_binding_checked_acceptance_transcript",
            "runtime_transcript_binding_checked_acceptance_artifact_finalized",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_challenge_segment_binding_checked_acceptance_artifact_structural_obligations",
        &[
            "runtime_challenge_segment_binding_checked_acceptance_artifact_finalized",
            "runtime_proof_artifact_finalized_structural_obligations",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_challenge_segment_binding_checked_acceptance_challenge_segment_bound",
        &["validation.challengeSegmentChecksImplyBound"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_challenge_segment_binding_checked_acceptance_segment_ids_unique",
        &[
            "RuntimeChallengeSegmentBindingCheckedAcceptance",
            "validation.transcriptValidation.artifactBindingValidation.proofSegmentIdsUnique",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_challenge_segment_binding_checked_acceptance_segment_ids_unique",
        &["runtime_challenge_segment_binding_checked_acceptance_artifact_structural_obligations"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_challenge_segment_binding_checked_acceptance_payload_reuse_contract",
        &[
            "RuntimeChallengeSegmentBindingCheckedAcceptance",
            "RuntimeChallengeSegmentPayloadReuseContract",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_challenge_segment_binding_checked_acceptance_payload_reuse_contract",
        &[
            "runtime_challenge_segment_binding_checked_acceptance_evidence",
            "runtime_challenge_segment_binding_checked_acceptance_segment_ids_unique",
            "runtime_challenge_segment_binding_checked_acceptance_unit_values_trace_identity_coverage",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_challenge_segment_binding_checked_acceptance_query_derivation_contract",
        &[
            "RuntimeChallengeSegmentBindingCheckedAcceptance",
            "RuntimeChallengeQueryDerivationContract",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_challenge_segment_binding_checked_acceptance_query_derivation_contract",
        &[
            "validation.challengeBindingAcceptedImpliesQueryNonceValid",
            "validation.challengeBindingAcceptedImpliesQueriesDerivedFromNonce",
            "validation.transcriptValidation.transcriptAcceptedImpliesQueryPlanBound",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_challenge_segment_binding_checked_acceptance_unit_values_trace_identity_coverage",
        &[
            "RuntimeChallengeSegmentBindingCheckedAcceptance",
            "let artifactValidation := validation.transcriptValidation.artifactBindingValidation",
            "artifactValidation.proofUnitValuesTraceIdentityCoverage",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_challenge_segment_binding_checked_acceptance_unit_values_trace_identity_coverage",
        &["runtime_challenge_segment_binding_checked_acceptance_artifact_structural_obligations"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_challenge_segment_binding_checked_acceptance_container_canonical",
        &[
            "RuntimeChallengeSegmentBindingCheckedAcceptance",
            "validation.transcriptValidation.artifactBindingValidation.proofContainerCanonical",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_challenge_segment_binding_checked_acceptance_container_canonical",
        &["runtime_challenge_segment_binding_checked_acceptance_artifact_structural_obligations"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_challenge_segment_binding_checked_acceptance_metadata_canonical",
        &[
            "RuntimeChallengeSegmentBindingCheckedAcceptance",
            "validation.transcriptValidation.artifactBindingValidation.proofMetadataCanonical",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_challenge_segment_binding_checked_acceptance_metadata_canonical",
        &["runtime_challenge_segment_binding_checked_acceptance_artifact_structural_obligations"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_challenge_segment_binding_checked_acceptance_segment_payloads_nonempty",
        &[
            "RuntimeChallengeSegmentBindingCheckedAcceptance",
            "validation.transcriptValidation.artifactBindingValidation.proofSegmentPayloadsNonempty",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_challenge_segment_binding_checked_acceptance_segment_payloads_nonempty",
        &["runtime_challenge_segment_binding_checked_acceptance_artifact_structural_obligations"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_challenge_segment_binding_checked_acceptance_segment_ids_allowed",
        &[
            "RuntimeChallengeSegmentBindingCheckedAcceptance",
            "validation.transcriptValidation.artifactBindingValidation.proofSegmentIdsAllowed",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_challenge_segment_binding_checked_acceptance_segment_ids_allowed",
        &["runtime_challenge_segment_binding_checked_acceptance_artifact_structural_obligations"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_challenge_segment_binding_checked_acceptance_concrete_segment_ids_allowed",
        &[
            "RuntimeChallengeSegmentBindingCheckedAcceptance",
            "RuntimeProofArtifactConcreteSegmentIdBinding",
            "RuntimeProofArtifactConcreteSegmentIdsAllowed proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_challenge_segment_binding_checked_acceptance_concrete_segment_ids_allowed",
        &[
            "validation.challengeBindingAcceptedImpliesTranscriptAccepted",
            "runtime_transcript_binding_checked_acceptance_concrete_segment_ids_allowed",
        ],
    );
    lean_binding::assert_theorem_prefix_omits(
        &lean_source,
        "runtime_challenge_segment_binding_checked_acceptance_concrete_segment_ids_allowed",
        &["AssumptionBundle"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_challenge_segment_binding_checked_acceptance_segments_present",
        &[
            "RuntimeChallengeSegmentBindingCheckedAcceptance",
            "validation.transcriptValidation.artifactBindingValidation.proofSegmentsPresent",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_challenge_segment_binding_checked_acceptance_segments_present",
        &["runtime_challenge_segment_binding_checked_acceptance_artifact_structural_obligations"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_challenge_segment_binding_checked_acceptance_transcript_payload_contract",
        &[
            "RuntimeChallengeSegmentBindingEvidence",
            "RuntimeTranscriptBindingEvidence",
            "RuntimeArtifactEvidence",
            "RuntimeTranscriptBindingPayloadContract",
        ],
    );
    lean_binding::assert_theorem_prefix_omits(
        &lean_source,
        "runtime_challenge_segment_binding_checked_acceptance_transcript_payload_contract",
        &["AssumptionBundle"],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_challenge_segment_binding_checked_acceptance_transcript_payload_contract",
        &["abstract_verifier_sound"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_challenge_segment_binding_checked_acceptance_challenge_and_core_contract",
        &[
            "RuntimeChallengeSegmentBindingEvidence",
            "RuntimeTranscriptBindingEvidence",
            "system.transcriptBound publicInput proof",
            "RuntimeVerifierCoreContract system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_challenge_segment_binding_checked_acceptance_sound",
        &[
            "runtime_transcript_binding_checked_acceptance_full_contract",
            "runtime_transcript_binding_evidence_implies_transcript_bound",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_challenge_segment_binding_checked_acceptance_sound",
        &["runtime_transcript_binding_checked_acceptance_sound"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_challenge_segment_binding_checked_acceptance_verifier_core_contract",
        &["runtime_transcript_binding_checked_acceptance_verifier_core_contract"],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_challenge_segment_binding_checked_acceptance_verifier_core_contract",
        &[
            "runtime_challenge_segment_binding_checked_acceptance_sound",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_challenge_segment_binding_checked_acceptance_challenge_and_core_contract",
        &["runtime_challenge_segment_binding_checked_acceptance_verifier_core_contract"],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_challenge_segment_binding_checked_acceptance_challenge_and_core_contract",
        &[
            "runtime_challenge_segment_binding_checked_acceptance_sound",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_challenge_segment_binding_checked_acceptance_evidence_core_and_sound",
        &[
            "RuntimeChallengeSegmentBindingCheckedAcceptance",
            "RuntimeChallengeSegmentBindingEvidence",
            "RuntimeTranscriptBindingEvidence",
            "system.transcriptBound publicInput proof",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_challenge_segment_binding_checked_acceptance_evidence_core_and_sound",
        &[
            "runtime_challenge_segment_binding_checked_acceptance_sound",
            "runtime_challenge_segment_binding_checked_acceptance_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_challenge_segment_binding_checked_acceptance_evidence_core_and_sound",
        &["sound_witness_implies_verifier_core_contract"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_challenge_segment_binding_checked_acceptance_concrete_core_sound_contract",
        &[
            "RuntimeChallengeSegmentBindingCheckedAcceptance",
            "RuntimeProofArtifactConcreteSegmentIdBinding",
            "RuntimeChallengeSegmentBindingEvidence",
            "RuntimeTranscriptBindingEvidence",
            "system.transcriptBound publicInput proof",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
            "RuntimeProofArtifactConcreteSegmentIdsAllowed proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_challenge_segment_binding_checked_acceptance_concrete_core_sound_contract",
        &[
            "runtime_challenge_segment_binding_checked_acceptance_evidence_core_and_sound",
            "runtime_challenge_segment_binding_checked_acceptance_concrete_segment_ids_allowed",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_challenge_segment_binding_checked_acceptance_concrete_core_sound_contract",
        &[
            "runtime_challenge_segment_binding_checked_acceptance_sound",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
}

#[test]
fn lean_challenge_query_derivation_contract_matches_runtime_helpers() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_path = crate_root.join("../../lean/Lzvm/ChallengeSegmentBinding.lean");
    let lean_source =
        std::fs::read_to_string(&lean_path).expect("Lean challenge binding source should read");
    let runtime_path = crate_root.join("src/pcs_challenge.rs");
    let runtime_source =
        std::fs::read_to_string(&runtime_path).expect("PCS challenge source should read");

    assert!(
        lean_source.contains("challengeQueryNonceValid")
            && lean_source.contains("challengeQueriesDerivedFromNonce")
            && lean_source.contains("RuntimeChallengeQueryDerivationContract"),
        "Lean challenge binding should name nonce validation and derived-query obligations"
    );
    assert!(
        runtime_source.contains("pub fn verify_query_nonce")
            && runtime_source.contains("pub fn derive_fri_queries")
            && runtime_source.contains("poseidon2_hash_4([challenge.c0, challenge.c1, challenge.c2, nonce])")
            && runtime_source.contains("transcript.put(&[challenge.c0, challenge.c1, challenge.c2]);")
            && runtime_source.contains("transcript.put(&[nonce]);")
            && runtime_source.contains("transcript.get_permutations(count, bits)"),
        "Runtime PCS challenge helpers should validate the nonce and derive queries from challenge plus nonce"
    );
    assert!(
        runtime_source.contains("if bits > 64")
            && runtime_source
                .contains("let target = if bits == 64 { 1 } else { 1_u64 << (64 - bits) };")
            && runtime_source.contains("digest[0].to_u64() < target"),
        "Runtime nonce validation should retain the checked work-bit bound"
    );
}
