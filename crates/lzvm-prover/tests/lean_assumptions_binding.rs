use std::path::Path;

#[path = "support/lean_binding.rs"]
mod lean_binding;

#[test]
fn lean_assumptions_exports_centralized_soundness_obligations() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let assumptions_path = crate_root.join("../../lean/Lzvm/Assumptions.lean");
    let assumptions_source =
        std::fs::read_to_string(&assumptions_path).expect("Lean assumptions source should read");
    let audit_path = crate_root.join("../../lean/Lzvm/AssumptionAudit.lean");
    let audit_source =
        std::fs::read_to_string(&audit_path).expect("Lean assumption audit source should read");
    let top_level_path = crate_root.join("../../lean/Lzvm.lean");
    let top_level_source =
        std::fs::read_to_string(&top_level_path).expect("Lean top-level source should read");

    assert!(
        lean_binding::contains_import(&top_level_source, "Lzvm.Assumptions"),
        "top-level Lean module should import centralized assumptions"
    );
    assert!(
        assumptions_source.contains("structure HashCollisionResistanceAssumption")
            && assumptions_source.contains("structure FiatShamirRandomOracleAssumption")
            && assumptions_source.contains("structure PcsOpeningSoundnessAssumption")
            && assumptions_source.contains("structure FriQuerySoundnessAssumption")
            && assumptions_source.contains("structure CryptographicAssumptions")
            && assumptions_source.contains("structure SemanticAssumptions")
            && assumptions_source.contains("structure AssumptionBundle"),
        "Lean assumptions should centralize named cryptographic, semantic, and bundled obligations"
    );
    assert_eq!(
        lean_binding::structure_field_names(
            &assumptions_source,
            "structure HashCollisionResistanceAssumption",
            "namespace HashCollisionResistanceAssumption",
        ),
        vec![
            "merkleHashCollisionResistanceStatement",
            "transcriptHashCollisionResistanceStatement",
            "merkleHashCollisionResistance",
            "transcriptHashCollisionResistance",
        ],
        "hash collision resistance assumptions should retain explicit statements and proof fields"
    );
    assert_eq!(
        lean_binding::structure_field_names(
            &assumptions_source,
            "structure FiatShamirRandomOracleAssumption",
            "namespace FiatShamirRandomOracleAssumption",
        ),
        vec![
            "randomOracleModelStatement",
            "randomOracleModel",
            "fiatShamirTranscriptBinding",
        ],
        "Fiat-Shamir assumptions should retain explicit statement and proof fields"
    );
    assert_eq!(
        lean_binding::structure_field_names(
            &assumptions_source,
            "structure PcsOpeningSoundnessAssumption",
            "namespace PcsOpeningSoundnessAssumption",
        ),
        vec!["pcsBindingStatement", "pcsBinding", "pcsOpeningSoundness"],
        "PCS assumptions should retain explicit statement and proof fields"
    );
    assert_eq!(
        lean_binding::structure_field_names(
            &assumptions_source,
            "structure FriQuerySoundnessAssumption",
            "namespace FriQuerySoundnessAssumption",
        ),
        vec![
            "friLowDegreeSoundnessStatement",
            "friLowDegreeSoundness",
            "friQuerySoundness",
        ],
        "FRI assumptions should retain explicit statement and proof fields"
    );
    assert_eq!(
        lean_binding::structure_field_names(
            &assumptions_source,
            "structure CryptographicAssumptions",
            "namespace CryptographicAssumptions",
        ),
        vec![
            "hashCollisionResistance",
            "randomOracleFiatShamir",
            "pcsSoundness",
            "friSoundness",
        ],
        "cryptographic assumptions should only expose audited hash, transcript, PCS, and FRI groups"
    );
    assert_eq!(
        lean_binding::structure_field_names(
            &assumptions_source,
            "structure AssumptionBundle",
            "end Lzvm",
        ),
        vec!["crypto", "semantic"],
        "assumption bundle should not grow unaudited assumption fields"
    );
    lean_binding::assert_theorem_declarations(
        &assumptions_source,
        &[
            "merkle_hash_collision_resistance",
            "transcript_hash_collision_resistance",
            "random_oracle_model",
            "fiat_shamir_transcript_binding",
            "pcs_binding",
            "pcs_opening_soundness",
            "fri_low_degree_soundness",
            "fri_query_soundness",
        ],
    );
    for snippet in [
        "theorem merkle_hash_collision_resistance\n    (assumptions : HashCollisionResistanceAssumption) :\n    assumptions.merkleHashCollisionResistanceStatement :=",
        "theorem transcript_hash_collision_resistance\n    (assumptions : HashCollisionResistanceAssumption) :\n    assumptions.transcriptHashCollisionResistanceStatement :=",
        "theorem random_oracle_model\n    {system : VerifierModel}\n    (assumptions : FiatShamirRandomOracleAssumption system) :\n    assumptions.randomOracleModelStatement :=",
        "theorem fiat_shamir_transcript_binding\n    {system : VerifierModel}\n    (assumptions : FiatShamirRandomOracleAssumption system) :\n    forall publicInput proof,\n      system.accepts publicInput proof ->\n        system.transcriptBound publicInput proof :=",
        "theorem pcs_binding\n    {system : VerifierModel}\n    (assumptions : PcsOpeningSoundnessAssumption system) :\n    assumptions.pcsBindingStatement :=",
        "theorem pcs_opening_soundness\n    {system : VerifierModel}\n    (assumptions : PcsOpeningSoundnessAssumption system) :\n    forall publicInput proof,\n      system.accepts publicInput proof ->\n        system.pcsOpeningsValid publicInput proof :=",
        "theorem fri_low_degree_soundness\n    {system : VerifierModel}\n    (assumptions : FriQuerySoundnessAssumption system) :\n    assumptions.friLowDegreeSoundnessStatement :=",
        "theorem fri_query_soundness\n    {system : VerifierModel}\n    (assumptions : FriQuerySoundnessAssumption system) :\n    forall publicInput proof,\n      system.accepts publicInput proof ->\n        system.friQueriesValid publicInput proof :=",
    ] {
        assert!(
            assumptions_source.contains(snippet),
            "Lean assumption accessor should keep the expected signature: {snippet}"
        );
    }
    assert!(
        assumptions_source.contains(":=\n  assumptions.merkleHashCollisionResistance")
            && assumptions_source.contains(":=\n  assumptions.transcriptHashCollisionResistance")
            && assumptions_source.contains(":=\n  assumptions.randomOracleModel")
            && assumptions_source.contains(":=\n  assumptions.fiatShamirTranscriptBinding")
            && assumptions_source.contains(":=\n  assumptions.pcsBinding")
            && assumptions_source.contains(":=\n  assumptions.pcsOpeningSoundness")
            && assumptions_source.contains(":=\n  assumptions.friLowDegreeSoundness")
            && assumptions_source.contains(":=\n  assumptions.friQuerySoundness"),
        "Lean assumption accessors should project their matching structure fields directly"
    );
    assert!(
        assumptions_source.contains("def transcript_binding")
            && assumptions_source.contains("def pcs_opening_sound")
            && assumptions_source.contains("def fri_query_sound")
            && assumptions_source.contains("theorem fiat_shamir_transcript_binding")
            && assumptions_source.contains("theorem pcs_opening_soundness")
            && assumptions_source.contains("theorem fri_query_soundness"),
        "Lean cryptographic assumptions should expose bundle-level projections and direct named accessors"
    );
    assert!(
        !assumptions_source.contains("NamedCryptographicAssumption"),
        "Lean cryptographic assumptions should not expose the removed wrapper alias"
    );
    assert!(
        assumptions_source.contains("system.accepts publicInput proof ->")
            && assumptions_source.contains("system.publicInputBound publicInput proof")
            && assumptions_source.contains("system.traceConsistent publicInput proof trace")
            && assumptions_source.contains("system.constraintsSatisfied constraints trace")
            && assumptions_source.contains("system.witnessMatchesTrace witness trace"),
        "Lean semantic assumptions should make accepted-proof binding, trace, constraints, and witness obligations explicit"
    );
    assert!(
        audit_source.contains("structure RequiredSemanticAssumptionStatements")
            && audit_source.contains("required_semantic_assumptions_public_input_binding")
            && audit_source.contains("required_semantic_assumptions_trace_extraction")
            && audit_source.contains("required_semantic_assumptions_constraint_satisfaction")
            && audit_source.contains("required_semantic_assumptions_witness_extraction")
            && audit_source.contains("assumptions.public_input_binding")
            && audit_source.contains("assumptions.trace_extraction")
            && audit_source.contains("assumptions.constraint_satisfaction")
            && audit_source.contains("assumptions.witness_extraction"),
        "Lean assumption audit should expose semantic obligations as explicit public input, trace, constraint, and witness statements"
    );
    lean_binding::assert_theorem_declarations(
        &audit_source,
        &["required_assumption_statements_verifier_core_contract"],
    );
    lean_binding::assert_theorem_body_contains_identifier(
        &audit_source,
        "required_assumption_statements_verifier_core_contract",
        "cryptoRequired",
    );
    lean_binding::assert_theorem_body_contains_identifier(
        &audit_source,
        "required_assumption_statements_verifier_core_contract",
        "semanticRequired",
    );
    for identifier in [
        "required_crypto_assumptions_fiat_shamir_transcript_binding",
        "required_semantic_assumptions_public_input_binding",
        "required_crypto_assumptions_pcs_opening_soundness",
        "required_crypto_assumptions_fri_query_soundness",
    ] {
        lean_binding::assert_theorem_body_contains_identifier(
            &audit_source,
            "required_assumption_statements_verifier_core_contract",
            identifier,
        );
    }
    lean_binding::assert_theorem_body_contains_identifier(
        &audit_source,
        "assumption_bundle_verifier_core_contract",
        "required_assumption_statements_verifier_core_contract",
    );
    for identifier in [
        "assumption_bundle_fiat_shamir_transcript_binding",
        "assumption_bundle_public_input_binding",
        "assumption_bundle_pcs_opening_soundness",
        "assumption_bundle_fri_query_soundness",
    ] {
        lean_binding::assert_theorem_body_omits_identifier(
            &audit_source,
            "assumption_bundle_verifier_core_contract",
            identifier,
        );
    }
}
