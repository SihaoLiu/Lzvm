use std::path::Path;

#[path = "support/lean_binding.rs"]
mod lean_binding;

const QUERY_PLAN_SOUNDNESS_SOURCE_PATH: &str = "../../lean/Lzvm/QueryPlanBinding/Soundness.lean";

#[test]
fn lean_query_plan_routes_accepted_evidence_by_split_helpers() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_source = lean_binding::read_lean_source(crate_root, QUERY_PLAN_SOUNDNESS_SOURCE_PATH);
    let theorem = "runtime_query_plan_binding_direct_finalized_core_sound_witness_contract";

    lean_binding::assert_theorem_declarations(&lean_source, &[theorem]);
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        theorem,
        &[
            "RuntimeQueryPlanBindingCheckedAcceptance",
            "RequiredCryptographicAssumptionStatements assumptions.crypto",
            "RequiredSemanticAssumptionStatements assumptions.semantic",
            "RuntimeQueryPlanBindingEvidence",
            "RuntimeProofArtifactFinalized",
            "validation.queryPlanSeedBindsWitnessTreeDigests artifact publicInput proof",
            "validation.queryPlanSeededFriOpeningRequirementsChecked",
            "RuntimeVerifierCoreContract system publicInput proof",
            "system.traceConsistent publicInput proof trace",
            "system.constraintsSatisfied constraints trace",
            "system.witnessMatchesTrace witness trace",
            "SoundWitness system publicInput proof",
        ],
    );
    for identifier in [
        "runtime_query_plan_binding_checked_acceptance_evidence",
        "runtime_query_plan_binding_checked_acceptance_artifact_finalized",
        "runtime_query_plan_binding_checked_acceptance_seed_binds_witness_tree_digests",
        "runtime_query_plan_binding_checked_acceptance_seeded_fri_opening_requirements_checked",
        "runtime_query_plan_binding_checked_acceptance_challenge",
        "runtime_challenge_segment_binding_checked_acceptance_transcript",
        "runtime_proof_artifact_binding_checked_acceptance_runtime_accepted",
        "runtime_artifact_checked_acceptance_implies_verifier_accepts",
        "accepted_proof_crypto_core_contract",
        "accepted_proof_semantic_execution_obligations",
        "abstract_verifier_sound",
    ] {
        lean_binding::assert_theorem_body_contains_identifier(&lean_source, theorem, identifier);
    }
    for identifier in [
        "accepted_proof_audited_core_execution_and_sound_witness",
        "accepted_proof_audited_core_and_sound_witness",
        "sound_witness_implies_execution_obligations",
        "runtime_query_plan_binding_checked_acceptance_evidence_core_and_sound",
        "runtime_query_plan_binding_checked_acceptance_full_soundness_contract",
        "runtime_query_plan_binding_checked_acceptance_verifier_core_contract",
        "runtime_opening_segment_binding_checked_acceptance_full_soundness_contract",
    ] {
        lean_binding::assert_theorem_body_omits_identifier(&lean_source, theorem, identifier);
    }
}
