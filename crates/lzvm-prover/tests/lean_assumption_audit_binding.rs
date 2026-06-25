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
            "cryptographic_assumptions_required_groups_fields",
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
            "assumption_bundle_fiat_shamir_transcript_binding",
            "assumption_bundle_pcs_opening_soundness",
            "assumption_bundle_fri_query_soundness",
            "required_semantic_assumptions_public_input_binding",
            "required_semantic_assumptions_trace_extraction",
            "required_semantic_assumptions_constraint_satisfaction",
            "required_semantic_assumptions_witness_extraction",
            "semantic_assumptions_carry_required_evidence",
            "assumption_bundle_carries_required_semantic_evidence",
            "assumption_bundle_public_input_binding",
            "assumption_bundle_trace_extraction",
            "assumption_bundle_constraint_satisfaction",
            "assumption_bundle_witness_extraction",
        ],
    );
    assert!(
        audit_source.contains("structure RequiredCryptographicAssumptionStatements")
            && audit_source.contains("structure RequiredSemanticAssumptionStatements"),
        "assumption audit should express required crypto and semantic obligations as explicit structures"
    );
    assert!(
        audit_source
            .contains("HashCollisionResistanceAssumption.transcript_hash_collision_resistance")
            && audit_source.contains("FiatShamirRandomOracleAssumption.random_oracle_model")
            && audit_source.contains("CryptographicAssumptions.transcript_binding")
            && audit_source.contains("PcsOpeningSoundnessAssumption.pcs_binding")
            && audit_source.contains("CryptographicAssumptions.pcs_opening_sound")
            && audit_source.contains("FriQuerySoundnessAssumption.fri_low_degree_soundness")
            && audit_source.contains("CryptographicAssumptions.fri_query_sound"),
        "assumption audit should route cryptographic evidence through named accessors"
    );
    assert!(
        !audit_source.contains("transcriptHashCollisionResistance.evidence")
            && !audit_source.contains("randomOracleFiatShamir.randomOracleModel.evidence")
            && !audit_source.contains("fiatShamirTranscriptBinding.evidence")
            && !audit_source.contains("pcsBinding.evidence")
            && !audit_source.contains("pcsOpeningSoundness.evidence")
            && !audit_source.contains("friLowDegreeSoundness.evidence")
            && !audit_source.contains("friQuerySoundness.evidence"),
        "assumption audit should not expose raw cryptographic evidence fields"
    );
    lean_binding::assert_theorem_prefix_contains(
        &audit_source,
        "cryptographic_assumptions_required_groups_fields",
        &[
            "hashCollisionResistance",
            "randomOracleFiatShamir",
            "pcsSoundness",
            "friSoundness",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &audit_source,
        "cryptographic_assumptions_required_groups_fields",
        &["And.intro rfl", "And.intro rfl (And.intro rfl rfl)"],
    );
    assert!(
        audit_source.contains("publicInputBinding :")
            && audit_source.contains("traceExtraction :")
            && audit_source.contains("constraintSatisfaction :")
            && audit_source.contains("witnessExtraction :")
            && audit_source.contains("system.publicInputBound publicInput proof")
            && audit_source.contains("exists trace, system.traceConsistent publicInput proof trace")
            && audit_source.contains("exists constraints, system.constraintsSatisfied constraints trace")
            && audit_source.contains("exists witness, system.witnessMatchesTrace witness trace"),
        "assumption audit should expose semantic soundness obligations as explicit structure fields"
    );
    lean_binding::assert_theorem_body_contains(
        &audit_source,
        "required_semantic_assumptions_public_input_binding",
        &["required.publicInputBinding"],
    );
    lean_binding::assert_theorem_body_contains(
        &audit_source,
        "required_semantic_assumptions_trace_extraction",
        &["required.traceExtraction"],
    );
    lean_binding::assert_theorem_body_contains(
        &audit_source,
        "required_semantic_assumptions_constraint_satisfaction",
        &["required.constraintSatisfaction"],
    );
    lean_binding::assert_theorem_body_contains(
        &audit_source,
        "required_semantic_assumptions_witness_extraction",
        &["required.witnessExtraction"],
    );
    lean_binding::assert_theorem_body_contains(
        &audit_source,
        "semantic_assumptions_carry_required_evidence",
        &[
            "publicInputBinding := assumptions.public_input_binding",
            "traceExtraction := assumptions.trace_extraction",
            "constraintSatisfaction := assumptions.constraint_satisfaction",
            "witnessExtraction := assumptions.witness_extraction",
        ],
    );
    for theorem_name in [
        "assumption_bundle_public_input_binding",
        "assumption_bundle_trace_extraction",
        "assumption_bundle_constraint_satisfaction",
        "assumption_bundle_witness_extraction",
    ] {
        lean_binding::assert_theorem_body_contains(
            &audit_source,
            theorem_name,
            &[
                "assumption_bundle_carries_required_semantic_evidence",
                "required_semantic_assumptions_",
            ],
        );
    }
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
        "required_crypto_assumptions_merkle_hash_collision_resistance",
        "required_crypto_assumptions_transcript_hash_collision_resistance",
        "required_crypto_assumptions_random_oracle_model",
        "required_crypto_assumptions_fiat_shamir_transcript_binding",
        "required_crypto_assumptions_pcs_opening_soundness",
        "required_crypto_assumptions_fri_query_soundness",
    ] {
        lean_binding::assert_theorem_prefix_contains(
            &audit_source,
            theorem_name,
            &["RequiredCryptographicAssumptionStatements assumptions"],
        );
        let expected_field = match theorem_name {
            "required_crypto_assumptions_merkle_hash_collision_resistance" => {
                "required.merkleHashCollisionResistance"
            }
            "required_crypto_assumptions_transcript_hash_collision_resistance" => {
                "required.transcriptHashCollisionResistance"
            }
            "required_crypto_assumptions_random_oracle_model" => "required.randomOracleModel",
            "required_crypto_assumptions_fiat_shamir_transcript_binding" => {
                "required.fiatShamirTranscriptBinding"
            }
            "required_crypto_assumptions_pcs_opening_soundness" => "required.pcsOpeningSoundness",
            "required_crypto_assumptions_fri_query_soundness" => "required.friQuerySoundness",
            _ => unreachable!("unexpected theorem name"),
        };
        lean_binding::assert_theorem_body_contains(&audit_source, theorem_name, &[expected_field]);
        lean_binding::assert_theorem_body_omits(&audit_source, theorem_name, &["rcases required"]);
    }
    for theorem_name in [
        "assumption_bundle_fiat_shamir_transcript_binding",
        "assumption_bundle_pcs_opening_soundness",
        "assumption_bundle_fri_query_soundness",
    ] {
        lean_binding::assert_theorem_prefix_contains(
            &audit_source,
            theorem_name,
            &[
                "AssumptionBundle system",
                "system.accepts publicInput proof",
            ],
        );
        lean_binding::assert_theorem_body_contains(
            &audit_source,
            theorem_name,
            &["assumption_bundle_carries_required_crypto_evidence"],
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
