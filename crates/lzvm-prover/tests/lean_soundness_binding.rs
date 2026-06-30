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
            "abstract_verifier_sound_with_audited_soundness_obligations",
            "accepted_proof_audited_core_and_sound_witness",
            "accepted_proof_audited_full_evidence",
            "accepted_proof_audited_sound_witness_components",
        ],
    );
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
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "abstract_verifier_sound_with_audited_soundness_obligations",
        &[
            "abstract_verifier_sound_with_audited_assumptions",
            "assumption_bundle_carries_required_evidence",
        ],
    );
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
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "accepted_proof_audited_core_and_sound_witness",
        &[
            "assumption_bundle_carries_required_evidence",
            "assumption_bundle_verifier_core_contract",
            "abstract_verifier_sound",
        ],
    );
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
            "accepted_proof_audited_core_and_sound_witness",
            "coreContract",
            "soundWitness",
            "traceConsistent",
            "constraintsSatisfied",
            "witnessMatchesTrace",
        ],
    );
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
            "accepted_proof_audited_core_and_sound_witness",
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
}
