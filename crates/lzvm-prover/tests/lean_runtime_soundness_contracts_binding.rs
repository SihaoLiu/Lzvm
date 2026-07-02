use std::path::Path;

#[path = "support/lean_binding.rs"]
mod lean_binding;

#[test]
fn lean_runtime_soundness_contracts_exports_artifact_audited_segment_contract() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_path = crate_root.join("../../lean/Lzvm/RuntimeSoundness/Contracts.lean");
    let lean_source = std::fs::read_to_string(&lean_path)
        .expect("Lean runtime soundness contracts source should read");
    let top_level_path = crate_root.join("../../lean/Lzvm.lean");
    let top_level_source =
        std::fs::read_to_string(&top_level_path).expect("Lean top-level source should read");

    assert!(
        lean_binding::contains_import(&top_level_source, "Lzvm.RuntimeSoundness.Contracts"),
        "top-level Lean module should import runtime soundness contracts"
    );
    lean_binding::assert_theorem_declarations(
        &lean_source,
        &["runtime_soundness_checked_acceptance_artifact_audited_segment_ids_contract"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_soundness_checked_acceptance_artifact_audited_segment_ids_contract",
        &[
            "RuntimeSoundnessCheckedAcceptance",
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
            "exists witness trace constraints",
            "system.traceConsistent publicInput proof trace",
            "system.constraintsSatisfied constraints trace",
            "system.witnessMatchesTrace witness trace",
            "SoundWitness system publicInput proof",
            "proofContainerCanonical artifact publicInput proof",
            "proofSegmentsPresent artifact publicInput proof",
            "proofMetadataCanonical artifact publicInput proof",
            "proofSegmentPayloadsNonempty artifact publicInput proof",
            "proofSegmentIdsAllowed artifact publicInput proof",
            "proofSegmentIdsUnique artifact publicInput proof",
            "proofUnitValuesTraceIdentityCoverage",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_soundness_checked_acceptance_artifact_audited_segment_ids_contract",
        &[
            "runtime_soundness_checked_acceptance_artifact_audited_soundness_contracts_core_contract",
            "runtime_soundness_checked_acceptance_artifact_segment_ids_contract",
            "artifactEvidence",
            "auditedCrypto",
            "auditedSemantic",
            "proofSystemSound",
            "verifierAccepts",
            "transcriptBound",
            "publicInputBound",
            "pcsOpenings",
            "friQueries",
            "verifierCore",
            "executionObligations",
            "soundWitness",
            "containerCanonical",
            "segmentsPresent",
            "metadataCanonical",
            "segmentPayloadsNonempty",
            "segmentIdsAllowed",
            "segmentIdsUnique",
            "unitValuesTraceIdentityCoverage",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_soundness_checked_acceptance_artifact_audited_segment_ids_contract",
        &[
            "assumptions.crypto.transcript_binding",
            "assumptions.semantic.public_input_binding",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
}

#[test]
fn lean_runtime_soundness_contracts_exports_concrete_segment_contract() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_path = crate_root.join("../../lean/Lzvm/RuntimeSoundness/Contracts.lean");
    let lean_source = std::fs::read_to_string(&lean_path)
        .expect("Lean runtime soundness contracts source should read");

    lean_binding::assert_theorem_declarations(
        &lean_source,
        &["runtime_soundness_checked_acceptance_artifact_audited_concrete_segment_ids_contract"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_soundness_checked_acceptance_artifact_audited_concrete_segment_ids_contract",
        &[
            "RuntimeProofArtifactConcreteSegmentIdBinding",
            "RuntimeSoundnessCheckedAcceptance",
            "RuntimeArtifactEvidence",
            "RequiredCryptographicAssumptionStatements assumptions.crypto",
            "RequiredSemanticAssumptionStatements assumptions.semantic",
            "ProofSystemSound system",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
            "proofSegmentIdsAllowed artifact publicInput proof",
            "proofUnitValuesTraceIdentityCoverage",
            "RuntimeProofArtifactConcreteSegmentIdsAllowed proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_soundness_checked_acceptance_artifact_audited_concrete_segment_ids_contract",
        &[
            "runtime_soundness_checked_acceptance_artifact_audited_segment_ids_contract",
            "runtime_soundness_checked_acceptance_concrete_segment_ids_allowed",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_soundness_checked_acceptance_artifact_audited_concrete_segment_ids_contract",
        &[
            "assumptions.crypto.transcript_binding",
            "assumptions.semantic.public_input_binding",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
}

#[test]
fn lean_runtime_soundness_contracts_exports_required_source_finalized_segment_contract() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_path = crate_root.join("../../lean/Lzvm/RuntimeSoundness/Contracts.lean");
    let lean_source = std::fs::read_to_string(&lean_path)
        .expect("Lean runtime soundness contracts source should read");

    lean_binding::assert_theorem_declarations(
        &lean_source,
        &["runtime_soundness_required_external_source_artifact_audited_finalized_segment_ids_contract"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_soundness_required_external_source_artifact_audited_finalized_segment_ids_contract",
        &[
            "RuntimeSoundnessCheckedAcceptance",
            "requiresExternalSource",
            "RuntimeArtifactEvidence",
            "RequiredCryptographicAssumptionStatements assumptions.crypto",
            "RequiredSemanticAssumptionStatements assumptions.semantic",
            "RuntimeProofArtifactFinalized",
            "ProofSystemSound system",
            "system.accepts publicInput proof",
            "ExternalSourceOpeningEvidence",
            "system.transcriptBound publicInput proof",
            "system.publicInputBound publicInput proof",
            "system.pcsOpeningsValid publicInput proof",
            "system.friQueriesValid publicInput proof",
            "RuntimeVerifierCoreContract system publicInput proof",
            "exists witness trace constraints",
            "SoundWitness system publicInput proof",
            "proofContainerCanonical artifact publicInput proof",
            "proofSegmentsPresent artifact publicInput proof",
            "proofMetadataCanonical artifact publicInput proof",
            "proofSegmentPayloadsNonempty artifact publicInput proof",
            "proofSegmentIdsAllowed artifact publicInput proof",
            "proofSegmentIdsUnique artifact publicInput proof",
            "proofUnitValuesTraceIdentityCoverage",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_soundness_required_external_source_artifact_audited_finalized_segment_ids_contract",
        &[
            "runtime_soundness_required_external_source_artifact_audited_segment_ids_contract",
            "runtime_soundness_required_external_source_audited_finalized_core_sound_witness_contract",
            "artifactFinalized",
            "externalSourceEvidence",
            "executionObligations",
            "unitValuesTraceIdentityCoverage",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_soundness_required_external_source_artifact_audited_finalized_segment_ids_contract",
        &[
            "assumptions.crypto.transcript_binding",
            "assumptions.semantic.public_input_binding",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
}
