use std::path::Path;

#[path = "support/lean_binding.rs"]
mod lean_binding;

#[test]
fn lean_assumption_audit_exports_runtime_soundness_coverage() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let audit_path = crate_root.join("../../lean/Lzvm/AssumptionAudit.lean");
    let audit_source =
        std::fs::read_to_string(&audit_path).expect("Lean assumption audit source should read");
    let runtime_path = crate_root.join("../../lean/Lzvm/RuntimeSoundness.lean");
    let runtime_entrypoint_source =
        std::fs::read_to_string(&runtime_path).expect("Lean runtime soundness source should read");
    let runtime_core_path = crate_root.join("../../lean/Lzvm/RuntimeSoundness/Core.lean");
    let runtime_core_source = std::fs::read_to_string(&runtime_core_path)
        .expect("Lean runtime soundness core source should read");
    let runtime_external_source_path =
        crate_root.join("../../lean/Lzvm/RuntimeSoundness/ExternalSource.lean");
    let runtime_external_source = std::fs::read_to_string(&runtime_external_source_path)
        .expect("Lean runtime soundness external-source source should read");
    let runtime_source =
        format!("{runtime_entrypoint_source}\n{runtime_core_source}\n{runtime_external_source}");
    let soundness_path = crate_root.join("../../lean/Lzvm/Soundness.lean");
    let soundness_source =
        std::fs::read_to_string(&soundness_path).expect("Lean soundness source should read");

    assert!(
        runtime_source.contains("import Lzvm.AssumptionAudit"),
        "runtime soundness should import the centralized assumption audit"
    );
    assert!(
        audit_source.contains("import Lzvm.MerklePathSoundness.Binary"),
        "assumption audit should connect centralized hash assumptions to concrete Merkle path collision resistance"
    );
    assert!(
        audit_source.contains("import Lzvm.MerklePathSoundness.NAry"),
        "assumption audit should also connect centralized hash assumptions to concrete N-ary Merkle path collision resistance"
    );
    assert!(
        runtime_source.contains("assumption_bundle_carries_required_crypto_evidence"),
        "runtime soundness should use the audited assumption bundle projection"
    );
    lean_binding::assert_theorem_declarations(
        &audit_source,
        &[
            "cryptographic_assumptions_carry_required_evidence",
            "assumption_bundle_carries_required_crypto_evidence",
            "required_crypto_assumptions_merkle_hash_collision_resistance",
            "required_crypto_assumptions_merkle_compression_no_collision",
            "required_crypto_assumptions_merkle_compression_collision_free",
            "required_crypto_assumptions_nary_merkle_compression_no_collision",
            "required_crypto_assumptions_nary_merkle_compression_collision_free",
            "assumption_bundle_merkle_compression_no_collision",
            "assumption_bundle_merkle_compression_collision_free",
            "assumption_bundle_nary_merkle_compression_no_collision",
            "assumption_bundle_nary_merkle_compression_collision_free",
            "required_crypto_assumptions_transcript_hash_collision_resistance",
            "required_crypto_assumptions_random_oracle_model",
            "required_crypto_assumptions_fiat_shamir_transcript_binding",
            "required_crypto_assumptions_pcs_binding",
            "required_crypto_assumptions_pcs_opening_soundness",
            "required_crypto_assumptions_fri_low_degree_soundness",
            "required_crypto_assumptions_fri_query_soundness",
            "required_semantic_assumptions_public_input_binding",
            "required_semantic_assumptions_trace_extraction",
            "required_semantic_assumptions_constraint_satisfaction",
            "required_semantic_assumptions_witness_extraction",
            "semantic_assumptions_carry_required_evidence",
            "assumption_bundle_carries_required_semantic_evidence",
        ],
    );
    assert!(
        audit_source.contains("RequiredSemanticAssumptionStatements")
            && audit_source.contains("(_assumptions : SemanticAssumptions system) : Prop :=")
            && audit_source.contains("system.publicInputBound publicInput proof")
            && audit_source.contains("exists trace, system.traceConsistent publicInput proof trace")
            && audit_source.contains("exists constraints, system.constraintsSatisfied constraints trace")
            && audit_source.contains("exists witness, system.witnessMatchesTrace witness trace")
            && !audit_source.contains("_assumptions : SemanticAssumptions system) : Prop :=\n  SemanticAssumptions system"),
        "assumption audit should expose semantic soundness obligations as explicit public input, trace, constraint, and witness statements"
    );
    lean_binding::assert_theorem_body_contains(
        &audit_source,
        "required_semantic_assumptions_public_input_binding",
        &["rcases required", "publicInputBinding"],
    );
    lean_binding::assert_theorem_body_contains(
        &audit_source,
        "required_semantic_assumptions_trace_extraction",
        &["rcases required", "traceExtraction"],
    );
    lean_binding::assert_theorem_body_contains(
        &audit_source,
        "required_semantic_assumptions_constraint_satisfaction",
        &["rcases required", "constraintSatisfaction"],
    );
    lean_binding::assert_theorem_body_contains(
        &audit_source,
        "required_semantic_assumptions_witness_extraction",
        &["rcases required", "witnessExtraction"],
    );
    lean_binding::assert_theorem_body_contains(
        &audit_source,
        "semantic_assumptions_carry_required_evidence",
        &[
            "assumptions.public_input_binding",
            "assumptions.trace_extraction",
            "assumptions.constraint_satisfaction",
            "assumptions.witness_extraction",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &audit_source,
        "required_crypto_assumptions_merkle_compression_no_collision",
        &[
            "RequiredCryptographicAssumptionStatements assumptions",
            "CentralizedMerkleCompressionCollisionResistance",
            "MerkleCompressionNoCollision compress",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &audit_source,
        "required_crypto_assumptions_merkle_compression_no_collision",
        &["required_crypto_assumptions_merkle_hash_collision_resistance"],
    );
    lean_binding::assert_theorem_body_contains(
        &audit_source,
        "required_crypto_assumptions_merkle_compression_collision_free",
        &[
            "required_crypto_assumptions_merkle_compression_no_collision",
            "merkle_compression_collision_free_of_no_collision",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &audit_source,
        "assumption_bundle_merkle_compression_collision_free",
        &[
            "assumption_bundle_carries_required_crypto_evidence",
            "required_crypto_assumptions_merkle_compression_collision_free",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &audit_source,
        "required_crypto_assumptions_nary_merkle_compression_no_collision",
        &[
            "RequiredCryptographicAssumptionStatements assumptions",
            "CentralizedNAryMerkleCompressionCollisionResistance",
            "NAryMerkleCompressionNoCollision compress",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &audit_source,
        "required_crypto_assumptions_nary_merkle_compression_no_collision",
        &["required_crypto_assumptions_merkle_hash_collision_resistance"],
    );
    lean_binding::assert_theorem_body_contains(
        &audit_source,
        "required_crypto_assumptions_nary_merkle_compression_collision_free",
        &[
            "required_crypto_assumptions_nary_merkle_compression_no_collision",
            "nary_merkle_compression_collision_free_of_no_collision",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &audit_source,
        "assumption_bundle_nary_merkle_compression_collision_free",
        &[
            "assumption_bundle_carries_required_crypto_evidence",
            "required_crypto_assumptions_nary_merkle_compression_collision_free",
        ],
    );
    for theorem_name in [
        "required_crypto_assumptions_pcs_opening_soundness",
        "required_crypto_assumptions_fri_query_soundness",
        "required_crypto_assumptions_fiat_shamir_transcript_binding",
    ] {
        lean_binding::assert_theorem_prefix_contains(
            &audit_source,
            theorem_name,
            &["RequiredCryptographicAssumptionStatements assumptions"],
        );
        lean_binding::assert_theorem_body_omits(
            &audit_source,
            theorem_name,
            &[".right.right.right"],
        );
    }
    lean_binding::assert_theorem_declarations(
        &runtime_source,
        &["runtime_soundness_checked_acceptance_audited_assumptions"],
    );
    lean_binding::assert_theorem_body_contains(
        &runtime_source,
        "runtime_soundness_checked_acceptance_evidence",
        &[
            "assumption_bundle_carries_required_crypto_evidence",
            "required_crypto_assumptions_pcs_opening_soundness",
            "required_crypto_assumptions_fri_query_soundness",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &runtime_source,
        "runtime_soundness_checked_acceptance_evidence",
        &[
            "assumptions.crypto.pcs_opening_sound",
            "assumptions.crypto.fri_query_sound",
        ],
    );
    assert!(
        soundness_source.contains("import Lzvm.AssumptionAudit"),
        "abstract soundness should import the centralized assumption audit"
    );
    assert!(
        soundness_source.contains("assumption_bundle_carries_required_crypto_evidence"),
        "abstract soundness should use the audited assumption bundle projection"
    );
    lean_binding::assert_theorem_declarations(
        &soundness_source,
        &[
            "abstract_verifier_sound_with_audited_assumptions",
            "abstract_verifier_sound_with_audited_soundness_obligations",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &soundness_source,
        "abstract_verifier_sound_with_audited_soundness_obligations",
        &[
            "RequiredCryptographicAssumptionStatements assumptions.crypto",
            "RequiredSemanticAssumptionStatements assumptions.semantic",
            "ProofSystemSound system",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &soundness_source,
        "abstract_verifier_sound_with_audited_soundness_obligations",
        &[
            "abstract_verifier_sound_with_audited_assumptions",
            "assumption_bundle_carries_required_semantic_evidence",
        ],
    );
}
