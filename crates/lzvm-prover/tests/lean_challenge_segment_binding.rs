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
        top_level_source.contains("import Lzvm.ChallengeSegmentBinding"),
        "top-level Lean module should import challenge segment binding"
    );
    assert!(
        lean_source.contains("RuntimeChallengeSegmentBindingValidation")
            && lean_source.contains("RuntimeChallengeSegmentBindingEvidence")
            && lean_source.contains("RuntimeTranscriptBindingEvidence")
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
            "runtime_challenge_segment_binding_checked_acceptance_payload_valid",
            "runtime_challenge_segment_binding_checked_acceptance_segment_matches_transcript",
            "runtime_challenge_segment_binding_checked_acceptance_challenge_segment_bound",
            "runtime_challenge_segment_binding_checked_acceptance_transcript_payload_contract",
            "runtime_challenge_segment_binding_checked_acceptance_sound",
            "runtime_challenge_segment_binding_checked_acceptance_verifier_core_contract",
            "runtime_challenge_segment_binding_checked_acceptance_challenge_and_core_contract",
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
        "runtime_challenge_segment_binding_checked_acceptance_challenge_segment_bound",
        &["validation.challengeSegmentChecksImplyBound"],
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
}
