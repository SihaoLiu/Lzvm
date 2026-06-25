use std::path::Path;

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
        top_level_source.contains("import Lzvm.Assumptions"),
        "top-level Lean module should import centralized assumptions"
    );
    assert!(
        assumptions_source.contains("abbrev NamedCryptographicAssumption")
            && assumptions_source.contains("structure HashCollisionResistanceAssumption")
            && assumptions_source.contains("structure FiatShamirRandomOracleAssumption")
            && assumptions_source.contains("structure PcsOpeningSoundnessAssumption")
            && assumptions_source.contains("structure FriQuerySoundnessAssumption")
            && assumptions_source.contains("structure CryptographicAssumptions")
            && assumptions_source.contains("structure SemanticAssumptions")
            && assumptions_source.contains("structure AssumptionBundle"),
        "Lean assumptions should centralize named cryptographic, semantic, and bundled obligations"
    );
    assert_eq!(
        structure_field_lines(
            &assumptions_source,
            "structure CryptographicAssumptions",
            "namespace CryptographicAssumptions",
        ),
        vec![
            "hashCollisionResistance : HashCollisionResistanceAssumption",
            "randomOracleFiatShamir : FiatShamirRandomOracleAssumption system",
            "pcsSoundness : PcsOpeningSoundnessAssumption system",
            "friSoundness : FriQuerySoundnessAssumption system",
        ],
        "cryptographic assumptions should only expose audited hash, transcript, PCS, and FRI groups"
    );
    assert_eq!(
        structure_field_lines(
            &assumptions_source,
            "structure AssumptionBundle",
            "end Lzvm",
        ),
        vec![
            "crypto : CryptographicAssumptions system",
            "semantic : SemanticAssumptions system",
        ],
        "assumption bundle should not grow unaudited assumption fields"
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
        !assumptions_source.contains("merkleHashCollisionResistance.evidence")
            && !assumptions_source.contains("transcriptHashCollisionResistance.evidence")
            && !assumptions_source.contains("randomOracleModel.evidence")
            && !assumptions_source.contains("fiatShamirTranscriptBinding.evidence")
            && !assumptions_source.contains("pcsBinding.evidence")
            && !assumptions_source.contains("pcsOpeningSoundness.evidence")
            && !assumptions_source.contains("friLowDegreeSoundness.evidence")
            && !assumptions_source.contains("friQuerySoundness.evidence"),
        "Lean cryptographic assumptions should not expose raw evidence-field projections"
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
        audit_source.contains("def RequiredSemanticAssumptionStatements")
            && audit_source.contains("required_semantic_assumptions_public_input_binding")
            && audit_source.contains("required_semantic_assumptions_trace_extraction")
            && audit_source.contains("required_semantic_assumptions_constraint_satisfaction")
            && audit_source.contains("required_semantic_assumptions_witness_extraction")
            && audit_source.contains("assumptions.public_input_binding")
            && audit_source.contains("assumptions.trace_extraction")
            && audit_source.contains("assumptions.constraint_satisfaction")
            && audit_source.contains("assumptions.witness_extraction")
            && !audit_source.contains("_assumptions : SemanticAssumptions system) : Prop :=\n  SemanticAssumptions system"),
        "Lean assumption audit should expose semantic obligations as explicit public input, trace, constraint, and witness statements"
    );
}

fn structure_field_lines(source: &str, start: &str, end: &str) -> Vec<String> {
    let start_index = source
        .find(start)
        .unwrap_or_else(|| panic!("source should contain {start}"));
    let after_start = &source[start_index..];
    let end_index = after_start
        .find(end)
        .unwrap_or_else(|| panic!("source should contain {end} after {start}"));
    after_start[..end_index]
        .lines()
        .map(str::trim)
        .filter(|line| line.contains(" : ") && !line.starts_with("structure "))
        .map(ToOwned::to_owned)
        .collect()
}
