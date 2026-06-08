use std::path::Path;

#[test]
fn lean_assumptions_exports_centralized_soundness_obligations() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let assumptions_path = crate_root.join("../../lean/Lzvm/Assumptions.lean");
    let assumptions_source =
        std::fs::read_to_string(&assumptions_path).expect("Lean assumptions source should read");
    let top_level_path = crate_root.join("../../lean/Lzvm.lean");
    let top_level_source =
        std::fs::read_to_string(&top_level_path).expect("Lean top-level source should read");

    assert!(
        top_level_source.contains("import Lzvm.Assumptions"),
        "top-level Lean module should import centralized assumptions"
    );
    assert!(
        assumptions_source.contains("structure NamedCryptographicAssumption")
            && assumptions_source.contains("structure HashCollisionResistanceAssumption")
            && assumptions_source.contains("structure FiatShamirRandomOracleAssumption")
            && assumptions_source.contains("structure PcsOpeningSoundnessAssumption")
            && assumptions_source.contains("structure FriQuerySoundnessAssumption")
            && assumptions_source.contains("structure CryptographicAssumptions")
            && assumptions_source.contains("structure SemanticAssumptions")
            && assumptions_source.contains("structure AssumptionBundle"),
        "Lean assumptions should centralize named cryptographic, semantic, and bundled obligations"
    );
    assert!(
        assumptions_source.contains("def transcript_binding")
            && assumptions_source.contains("def pcs_opening_sound")
            && assumptions_source.contains("def fri_query_sound"),
        "Lean cryptographic assumptions should expose transcript, PCS, and FRI evidence projections"
    );
    assert!(
        assumptions_source.contains("system.accepts publicInput proof ->")
            && assumptions_source.contains("system.publicInputBound publicInput proof")
            && assumptions_source.contains("system.traceConsistent publicInput proof trace")
            && assumptions_source.contains("system.constraintsSatisfied constraints trace")
            && assumptions_source.contains("system.witnessMatchesTrace witness trace"),
        "Lean semantic assumptions should make accepted-proof binding, trace, constraints, and witness obligations explicit"
    );
}
