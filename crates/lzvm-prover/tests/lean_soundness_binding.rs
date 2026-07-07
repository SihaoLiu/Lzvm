use std::path::Path;

#[path = "support/lean_binding.rs"]
mod lean_binding;

#[test]
fn lean_soundness_binding_exports_abstract_soundness_theorems() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_path = crate_root.join("../../lean/Lzvm/Soundness.lean");
    let lean_source =
        std::fs::read_to_string(&lean_path).expect("Lean soundness source should read");
    let top_level_path = crate_root.join("../../lean/Lzvm.lean");
    let top_level_source =
        std::fs::read_to_string(&top_level_path).expect("Lean top-level source should read");

    assert!(
        lean_binding::contains_import(&top_level_source, "Lzvm.Soundness"),
        "top-level Lean module should import abstract soundness"
    );
    assert!(
        lean_source.contains("ProofSystemSound system")
            && lean_source.contains("RequiredCryptographicAssumptionStatements assumptions.crypto")
            && lean_source.contains("RequiredSemanticAssumptionStatements assumptions.semantic"),
        "Lean abstract soundness should expose proof-system soundness and audited crypto/semantic assumptions"
    );
    lean_binding::assert_theorem_declarations(
        &lean_source,
        &[
            "abstract_verifier_sound",
            "abstract_verifier_sound_with_audited_assumptions",
            "abstract_verifier_sound_with_semantic_evidence",
            "abstract_verifier_sound_with_audited_soundness_obligations",
            "accepted_proof_audited_core_and_sound_witness",
            "accepted_proof_crypto_core_contract",
            "accepted_proof_semantic_execution_obligations",
            "accepted_proof_audited_core_execution_and_sound_witness",
            "accepted_proof_audited_core_and_execution_obligations",
            "accepted_proof_audited_full_evidence",
            "accepted_proof_audited_sound_witness_components",
            "accepted_proof_audited_core_and_sound_witness_components",
            "accepted_proof_audited_proof_system_and_components",
            "accepted_proof_audited_proof_system_core_and_execution_obligations",
            "accepted_proof_audited_flat_proof_system_components",
        ],
    );
    for theorem in [
        "accepted_proof_audited_core_execution_and_sound_witness",
        "accepted_proof_audited_proof_system_core_and_execution_obligations",
        "accepted_proof_audited_flat_proof_system_components",
    ] {
        lean_binding::assert_theorem_routes_accepted_evidence_by_split_helpers(
            &lean_source,
            theorem,
        );
    }
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "abstract_verifier_sound",
        &[
            "assumption_bundle_verifier_core_contract",
            "assumption_bundle_trace_extraction",
            "assumption_bundle_constraint_satisfaction",
            "assumption_bundle_witness_extraction",
            "transcriptBound",
            "publicInputBound",
            "pcsOpeningsValid",
            "friQueriesValid",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "abstract_verifier_sound",
        &[
            "assumption_bundle_fiat_shamir_transcript_binding",
            "assumption_bundle_pcs_opening_soundness",
            "assumption_bundle_fri_query_soundness",
            "assumption_bundle_public_input_binding",
            "assumptions.crypto.transcript_binding",
            "assumptions.crypto.pcs_opening_sound",
            "assumptions.crypto.fri_query_sound",
            "assumptions.semantic.public_input_binding",
            "assumptions.semantic.trace_extraction",
            "assumptions.semantic.constraint_satisfaction",
            "assumptions.semantic.witness_extraction",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "abstract_verifier_sound_with_audited_soundness_obligations",
        &[
            "RequiredCryptographicAssumptionStatements assumptions.crypto",
            "RequiredSemanticAssumptionStatements assumptions.semantic",
            "ProofSystemSound system",
        ],
    );
    for identifier in [
        "assumption_bundle_carries_required_crypto_evidence",
        "assumption_bundle_carries_required_semantic_evidence",
        "abstract_verifier_sound",
    ] {
        lean_binding::assert_theorem_body_contains_identifier(
            &lean_source,
            "abstract_verifier_sound_with_audited_soundness_obligations",
            identifier,
        );
    }
    for identifier in [
        "abstract_verifier_sound_with_audited_assumptions",
        "assumption_bundle_carries_required_evidence",
    ] {
        lean_binding::assert_theorem_body_omits_identifier(
            &lean_source,
            "abstract_verifier_sound_with_audited_soundness_obligations",
            identifier,
        );
    }
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "abstract_verifier_sound_with_semantic_evidence",
        &[
            "RequiredSemanticAssumptionStatements assumptions.semantic",
            "ProofSystemSound system",
        ],
    );
    for identifier in [
        "assumption_bundle_carries_required_semantic_evidence",
        "abstract_verifier_sound",
    ] {
        lean_binding::assert_theorem_body_contains_identifier(
            &lean_source,
            "abstract_verifier_sound_with_semantic_evidence",
            identifier,
        );
    }
    for identifier in [
        "assumption_bundle_carries_required_crypto_evidence",
        "assumption_bundle_carries_required_evidence",
    ] {
        lean_binding::assert_theorem_body_omits_identifier(
            &lean_source,
            "abstract_verifier_sound_with_semantic_evidence",
            identifier,
        );
    }
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "accepted_proof_audited_core_and_sound_witness",
        &[
            "system.accepts publicInput proof",
            "RequiredCryptographicAssumptionStatements assumptions.crypto",
            "RequiredSemanticAssumptionStatements assumptions.semantic",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
        ],
    );
    for identifier in [
        "accepted_proof_crypto_core_contract",
        "abstract_verifier_sound_with_semantic_evidence",
        "cryptoEvidence",
        "semanticEvidence",
        "coreContract",
        "proofSystemSound",
    ] {
        lean_binding::assert_theorem_body_contains_identifier(
            &lean_source,
            "accepted_proof_audited_core_and_sound_witness",
            identifier,
        );
    }
    for identifier in [
        "assumption_bundle_carries_required_crypto_evidence",
        "assumption_bundle_carries_required_semantic_evidence",
        "assumption_bundle_verifier_core_contract",
        "abstract_verifier_sound",
        "assumption_bundle_carries_required_evidence",
    ] {
        lean_binding::assert_theorem_body_omits_identifier(
            &lean_source,
            "accepted_proof_audited_core_and_sound_witness",
            identifier,
        );
    }
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "accepted_proof_crypto_core_contract",
        &[
            "system.accepts publicInput proof",
            "RequiredCryptographicAssumptionStatements assumptions.crypto",
            "RuntimeVerifierCoreContract system publicInput proof",
        ],
    );
    for identifier in [
        "assumption_bundle_carries_required_crypto_evidence",
        "assumption_bundle_verifier_core_contract",
    ] {
        lean_binding::assert_theorem_body_contains_identifier(
            &lean_source,
            "accepted_proof_crypto_core_contract",
            identifier,
        );
    }
    for identifier in [
        "assumption_bundle_carries_required_semantic_evidence",
        "accepted_proof_audited_core_and_sound_witness",
        "assumption_bundle_carries_required_evidence",
    ] {
        lean_binding::assert_theorem_body_omits_identifier(
            &lean_source,
            "accepted_proof_crypto_core_contract",
            identifier,
        );
    }
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "accepted_proof_semantic_execution_obligations",
        &[
            "system.accepts publicInput proof",
            "RequiredSemanticAssumptionStatements assumptions.semantic",
            "exists witness trace constraints",
            "system.traceConsistent publicInput proof trace",
            "system.constraintsSatisfied constraints trace",
            "system.witnessMatchesTrace witness trace",
        ],
    );
    for identifier in [
        "abstract_verifier_sound_with_semantic_evidence",
        "semanticEvidence",
        "soundWitness",
        "traceConsistent",
        "constraintsSatisfied",
        "witnessMatchesTrace",
    ] {
        lean_binding::assert_theorem_body_contains_identifier(
            &lean_source,
            "accepted_proof_semantic_execution_obligations",
            identifier,
        );
    }
    for identifier in [
        "assumption_bundle_carries_required_crypto_evidence",
        "assumption_bundle_carries_required_semantic_evidence",
        "assumption_bundle_verifier_core_contract",
        "abstract_verifier_sound",
        "accepted_proof_audited_core_and_sound_witness",
        "assumption_bundle_carries_required_evidence",
    ] {
        lean_binding::assert_theorem_body_omits_identifier(
            &lean_source,
            "accepted_proof_semantic_execution_obligations",
            identifier,
        );
    }
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "accepted_proof_audited_core_execution_and_sound_witness",
        &[
            "system.accepts publicInput proof",
            "RequiredCryptographicAssumptionStatements assumptions.crypto",
            "RequiredSemanticAssumptionStatements assumptions.semantic",
            "RuntimeVerifierCoreContract system publicInput proof",
            "exists witness trace constraints",
            "system.traceConsistent publicInput proof trace",
            "system.constraintsSatisfied constraints trace",
            "system.witnessMatchesTrace witness trace",
            "SoundWitness system publicInput proof",
        ],
    );
    for identifier in [
        "accepted_proof_crypto_core_contract",
        "accepted_proof_semantic_execution_obligations",
        "abstract_verifier_sound_with_semantic_evidence",
        "proofSystemSound",
        "soundWitness",
    ] {
        lean_binding::assert_theorem_body_contains_identifier(
            &lean_source,
            "accepted_proof_audited_core_execution_and_sound_witness",
            identifier,
        );
    }
    for identifier in [
        "abstract_verifier_sound",
        "accepted_proof_audited_core_and_sound_witness",
        "sound_witness_implies_execution_obligations",
    ] {
        lean_binding::assert_theorem_body_omits_identifier(
            &lean_source,
            "accepted_proof_audited_core_execution_and_sound_witness",
            identifier,
        );
    }
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "accepted_proof_audited_core_and_execution_obligations",
        &[
            "system.accepts publicInput proof",
            "RequiredCryptographicAssumptionStatements assumptions.crypto",
            "RequiredSemanticAssumptionStatements assumptions.semantic",
            "RuntimeVerifierCoreContract system publicInput proof",
            "exists witness trace constraints",
            "system.traceConsistent publicInput proof trace",
            "system.constraintsSatisfied constraints trace",
            "system.witnessMatchesTrace witness trace",
        ],
    );
    for identifier in [
        "accepted_proof_crypto_core_contract",
        "accepted_proof_semantic_execution_obligations",
    ] {
        lean_binding::assert_theorem_body_contains_identifier(
            &lean_source,
            "accepted_proof_audited_core_and_execution_obligations",
            identifier,
        );
    }
    for identifier in [
        "accepted_proof_audited_core_execution_and_sound_witness",
        "accepted_proof_audited_core_and_sound_witness",
        "sound_witness_implies_execution_obligations",
    ] {
        lean_binding::assert_theorem_body_omits_identifier(
            &lean_source,
            "accepted_proof_audited_core_and_execution_obligations",
            identifier,
        );
    }
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "accepted_proof_audited_full_evidence",
        &[
            "system.accepts publicInput proof",
            "RequiredCryptographicAssumptionStatements assumptions.crypto",
            "RequiredSemanticAssumptionStatements assumptions.semantic",
            "exists witness trace constraints",
            "RuntimeVerifierCoreContract system publicInput proof",
            "system.traceConsistent publicInput proof trace",
            "system.constraintsSatisfied constraints trace",
            "system.witnessMatchesTrace witness trace",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "accepted_proof_audited_full_evidence",
        &[
            "coreContract",
            "executionObligations",
            "traceConsistent",
            "constraintsSatisfied",
            "witnessMatchesTrace",
        ],
    );
    for identifier in [
        "accepted_proof_crypto_core_contract",
        "accepted_proof_semantic_execution_obligations",
    ] {
        lean_binding::assert_theorem_body_contains_identifier(
            &lean_source,
            "accepted_proof_audited_full_evidence",
            identifier,
        );
    }
    for identifier in [
        "accepted_proof_audited_core_and_sound_witness",
        "accepted_proof_audited_core_execution_and_sound_witness",
    ] {
        lean_binding::assert_theorem_body_omits_identifier(
            &lean_source,
            "accepted_proof_audited_full_evidence",
            identifier,
        );
    }
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "accepted_proof_audited_sound_witness_components",
        &[
            "system.accepts publicInput proof",
            "RequiredCryptographicAssumptionStatements assumptions.crypto",
            "RequiredSemanticAssumptionStatements assumptions.semantic",
            "exists witness trace constraints",
            "system.transcriptBound publicInput proof",
            "system.publicInputBound publicInput proof",
            "system.pcsOpeningsValid publicInput proof",
            "system.friQueriesValid publicInput proof",
            "system.traceConsistent publicInput proof trace",
            "system.constraintsSatisfied constraints trace",
            "system.witnessMatchesTrace witness trace",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "accepted_proof_audited_sound_witness_components",
        &[
            "auditedSoundness",
            "proofSystemSound",
            "soundWitness",
            "transcriptBound",
            "publicInputBound",
            "pcsOpeningsValid",
            "friQueriesValid",
            "traceConsistent",
            "constraintsSatisfied",
            "witnessMatchesTrace",
        ],
    );
    lean_binding::assert_theorem_body_contains_identifier(
        &lean_source,
        "accepted_proof_audited_sound_witness_components",
        "abstract_verifier_sound_with_audited_soundness_obligations",
    );
    for identifier in ["cryptoEvidence", "semanticEvidence"] {
        lean_binding::assert_theorem_body_contains_identifier(
            &lean_source,
            "accepted_proof_audited_sound_witness_components",
            identifier,
        );
    }
    for identifier in [
        "assumption_bundle_carries_required_crypto_evidence",
        "assumption_bundle_carries_required_semantic_evidence",
        "abstract_verifier_sound",
        "accepted_proof_audited_core_and_sound_witness",
        "accepted_proof_audited_core_execution_and_sound_witness",
    ] {
        lean_binding::assert_theorem_body_omits_identifier(
            &lean_source,
            "accepted_proof_audited_sound_witness_components",
            identifier,
        );
    }
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "accepted_proof_audited_core_and_sound_witness_components",
        &[
            "system.accepts publicInput proof",
            "RequiredCryptographicAssumptionStatements assumptions.crypto",
            "RequiredSemanticAssumptionStatements assumptions.semantic",
            "RuntimeVerifierCoreContract system publicInput proof",
            "exists witness trace constraints",
            "system.transcriptBound publicInput proof",
            "system.publicInputBound publicInput proof",
            "system.pcsOpeningsValid publicInput proof",
            "system.friQueriesValid publicInput proof",
            "system.traceConsistent publicInput proof trace",
            "system.constraintsSatisfied constraints trace",
            "system.witnessMatchesTrace witness trace",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "accepted_proof_audited_core_and_sound_witness_components",
        &[
            "coreContract",
            "soundWitness",
            "transcriptBound",
            "publicInputBound",
            "pcsOpeningsValid",
            "friQueriesValid",
            "traceConsistent",
            "constraintsSatisfied",
            "witnessMatchesTrace",
        ],
    );
    for identifier in [
        "accepted_proof_crypto_core_contract",
        "abstract_verifier_sound_with_semantic_evidence",
        "semanticEvidence",
        "proofSystemSound",
    ] {
        lean_binding::assert_theorem_body_contains_identifier(
            &lean_source,
            "accepted_proof_audited_core_and_sound_witness_components",
            identifier,
        );
    }
    for identifier in [
        "abstract_verifier_sound",
        "assumption_bundle_carries_required_semantic_evidence",
        "accepted_proof_audited_core_and_sound_witness",
        "accepted_proof_audited_core_execution_and_sound_witness",
    ] {
        lean_binding::assert_theorem_body_omits_identifier(
            &lean_source,
            "accepted_proof_audited_core_and_sound_witness_components",
            identifier,
        );
    }
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "accepted_proof_audited_proof_system_and_components",
        &[
            "system.accepts publicInput proof",
            "RequiredCryptographicAssumptionStatements assumptions.crypto",
            "RequiredSemanticAssumptionStatements assumptions.semantic",
            "ProofSystemSound system",
            "RuntimeVerifierCoreContract system publicInput proof",
            "exists witness trace constraints",
            "system.transcriptBound publicInput proof",
            "system.publicInputBound publicInput proof",
            "system.pcsOpeningsValid publicInput proof",
            "system.friQueriesValid publicInput proof",
            "system.traceConsistent publicInput proof trace",
            "system.constraintsSatisfied constraints trace",
            "system.witnessMatchesTrace witness trace",
        ],
    );
    for identifier in [
        "accepted_proof_crypto_core_contract",
        "abstract_verifier_sound_with_semantic_evidence",
        "semanticEvidence",
        "proofSystemSound",
    ] {
        lean_binding::assert_theorem_body_contains_identifier(
            &lean_source,
            "accepted_proof_audited_proof_system_and_components",
            identifier,
        );
    }
    for identifier in [
        "abstract_verifier_sound",
        "assumption_bundle_carries_required_semantic_evidence",
        "accepted_proof_audited_core_and_sound_witness_components",
    ] {
        lean_binding::assert_theorem_body_omits_identifier(
            &lean_source,
            "accepted_proof_audited_proof_system_and_components",
            identifier,
        );
    }
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "accepted_proof_audited_proof_system_core_and_execution_obligations",
        &[
            "system.accepts publicInput proof",
            "RequiredCryptographicAssumptionStatements assumptions.crypto",
            "RequiredSemanticAssumptionStatements assumptions.semantic",
            "ProofSystemSound system",
            "RuntimeVerifierCoreContract system publicInput proof",
            "exists witness trace constraints",
            "system.traceConsistent publicInput proof trace",
            "system.constraintsSatisfied constraints trace",
            "system.witnessMatchesTrace witness trace",
        ],
    );
    for identifier in [
        "accepted_proof_crypto_core_contract",
        "accepted_proof_semantic_execution_obligations",
        "abstract_verifier_sound_with_semantic_evidence",
        "proofSystemSound",
    ] {
        lean_binding::assert_theorem_body_contains_identifier(
            &lean_source,
            "accepted_proof_audited_proof_system_core_and_execution_obligations",
            identifier,
        );
    }
    for identifier in [
        "abstract_verifier_sound",
        "accepted_proof_audited_core_and_execution_obligations",
        "proof_system_sound_accepts_core_contract_and_execution_obligations",
    ] {
        lean_binding::assert_theorem_body_omits_identifier(
            &lean_source,
            "accepted_proof_audited_proof_system_core_and_execution_obligations",
            identifier,
        );
    }
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "accepted_proof_audited_flat_proof_system_components",
        &[
            "system.accepts publicInput proof",
            "RequiredCryptographicAssumptionStatements assumptions.crypto",
            "RequiredSemanticAssumptionStatements assumptions.semantic",
            "ProofSystemSound system",
            "system.transcriptBound publicInput proof",
            "system.publicInputBound publicInput proof",
            "system.pcsOpeningsValid publicInput proof",
            "system.friQueriesValid publicInput proof",
            "exists witness trace constraints",
            "system.traceConsistent publicInput proof trace",
            "system.constraintsSatisfied constraints trace",
            "system.witnessMatchesTrace witness trace",
        ],
    );
    for identifier in [
        "accepted_proof_crypto_core_contract",
        "accepted_proof_semantic_execution_obligations",
        "abstract_verifier_sound_with_semantic_evidence",
        "proofSystemSound",
        "coreContract",
        "transcriptBound",
        "publicInputBound",
        "pcsOpeningsValid",
        "friQueriesValid",
        "traceConsistent",
        "constraintsSatisfied",
        "witnessMatchesTrace",
    ] {
        lean_binding::assert_theorem_body_contains_identifier(
            &lean_source,
            "accepted_proof_audited_flat_proof_system_components",
            identifier,
        );
    }
    for identifier in [
        "abstract_verifier_sound",
        "accepted_proof_audited_proof_system_and_components",
        "assumption_bundle_verifier_core_contract",
        "assumption_bundle_carries_required_evidence",
        "proof_system_sound_accepts_core_contract_and_execution_obligations",
        "sound_witness_implies_execution_obligations",
    ] {
        lean_binding::assert_theorem_body_omits_identifier(
            &lean_source,
            "accepted_proof_audited_flat_proof_system_components",
            identifier,
        );
    }
}

#[test]
fn lean_soundness_root_calls_stay_in_wrapper_theorems() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_path = crate_root.join("../../lean/Lzvm/Soundness.lean");
    let lean_source =
        std::fs::read_to_string(&lean_path).expect("Lean soundness source should read");

    let mut root_callers = lean_binding::theorem_names(&lean_source)
        .into_iter()
        .filter(|theorem| {
            lean_binding::visible_identifier_occurrence_count(
                &lean_binding::theorem_body(&lean_source, theorem),
                "abstract_verifier_sound",
            ) > 0
        })
        .collect::<Vec<_>>();
    root_callers.sort();

    assert_eq!(
        root_callers,
        vec![
            "abstract_verifier_sound_with_audited_assumptions",
            "abstract_verifier_sound_with_audited_soundness_obligations",
            "abstract_verifier_sound_with_semantic_evidence",
        ],
        "direct root theorem calls should stay isolated in wrapper theorems"
    );
}
