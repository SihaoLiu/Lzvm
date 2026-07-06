use std::path::Path;

#[path = "support/lean_binding.rs"]
mod lean_binding;

#[test]
fn lean_required_external_source_binding_exports_core_contract_projection() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_path = crate_root.join("../../lean/Lzvm/RequiredExternalSource.lean");
    let lean_source =
        std::fs::read_to_string(&lean_path).expect("Lean required external source should read");
    let top_level_path = crate_root.join("../../lean/Lzvm.lean");
    let top_level_source =
        std::fs::read_to_string(&top_level_path).expect("Lean top-level source should read");

    assert!(
        lean_binding::contains_import(&top_level_source, "Lzvm.RequiredExternalSource"),
        "top-level Lean module should import required external source"
    );
    assert!(
        lean_source.contains("runtime_guarded_external_source_required_evidence")
            && lean_source.contains("runtime_guarded_external_source_required_sound")
            && lean_source.contains("requiresExternalSource ->")
            && lean_source.contains("RuntimeVerifierCoreContract system publicInput proof")
            && lean_source.contains("SoundWitness system publicInput proof"),
        "Lean required external source binding should expose required evidence, soundness, and verifier core projection"
    );
    lean_binding::assert_theorem_declarations(
        &lean_source,
        &[
            "runtime_guarded_external_source_required_verifier_core_contract",
            "runtime_guarded_external_source_required_evidence_core_and_sound",
            "runtime_guarded_external_source_required_pcs_and_fri_from_hash_concrete_opening",
            "runtime_guarded_external_source_required_pcs_and_fri_from_concrete_opening",
            "runtime_guarded_external_source_required_hash_concrete_opening_sound",
            "runtime_guarded_external_source_required_audited_hash_concrete_opening_sound",
            "runtime_guarded_external_source_required_audited_hash_concrete_opening_sound_with_required_evidence",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_guarded_external_source_required_verifier_core_contract",
        &["runtime_guarded_external_source_checked_acceptance_verifier_core_contract"],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_guarded_external_source_required_verifier_core_contract",
        &[
            "runtime_guarded_external_source_required_sound",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_guarded_external_source_required_evidence_core_and_sound",
        &[
            "RuntimeGuardedExternalSourceCheckedAcceptance",
            "requiresExternalSource ->",
            "RuntimeArtifactEvidence",
            "ExternalSourceOpeningEvidence",
            "system.pcsOpeningsValid publicInput proof",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_guarded_external_source_required_evidence_core_and_sound",
        &[
            "runtime_guarded_external_source_required_sound",
            "runtime_guarded_external_source_required_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_guarded_external_source_required_evidence_core_and_sound",
        &["sound_witness_implies_verifier_core_contract"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_guarded_external_source_required_pcs_and_fri_from_hash_concrete_opening",
        &[
            "HashCollisionResistanceAssumption",
            "RuntimeOpeningValidation system",
            "RuntimeConstantOpeningNAryConcreteBinding",
            "RuntimeWitnessOpeningNAryConcreteBinding",
            "system.pcsOpeningsValid publicInput proof",
            "system.friQueriesValid publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_guarded_external_source_required_pcs_and_fri_from_hash_concrete_opening",
        &[
            "external_source_opening_requirement_implies_evidence",
            "runtime_opening_checked_acceptance_pcs_and_fri_from_hash_assumption_concrete_nary_merkle",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_guarded_external_source_required_pcs_and_fri_from_hash_concrete_opening",
        &[
            "external_source_opening_evidence_implies_pcs_openings",
            "providerEvidenceImpliesPcsOpenings",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_guarded_external_source_required_pcs_and_fri_from_concrete_opening",
        &[
            "AssumptionBundle system",
            "assumptions.crypto.hashCollisionResistance",
            "RuntimeOpeningValidation system",
            "RuntimeConstantOpeningNAryConcreteBinding",
            "RuntimeWitnessOpeningNAryConcreteBinding",
            "system.pcsOpeningsValid publicInput proof",
            "system.friQueriesValid publicInput proof",
        ],
    );
    lean_binding::assert_theorem_prefix_omits(
        &lean_source,
        "runtime_guarded_external_source_required_pcs_and_fri_from_concrete_opening",
        &["HashCollisionResistanceAssumption"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_guarded_external_source_required_pcs_and_fri_from_concrete_opening",
        &[
            "runtime_guarded_external_source_required_pcs_and_fri_from_hash_concrete_opening",
            "assumptions.crypto.hashCollisionResistance",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_guarded_external_source_required_hash_concrete_opening_sound",
        &[
            "AssumptionBundle system",
            "HashCollisionResistanceAssumption",
            "RuntimeGuardedExternalSourceCheckedAcceptance",
            "RuntimeOpeningCheckedAcceptance",
            "RuntimeArtifactEvidence",
            "ExternalSourceOpeningEvidence",
            "system.pcsOpeningsValid publicInput proof",
            "system.friQueriesValid publicInput proof",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_guarded_external_source_required_hash_concrete_opening_sound",
        &[
            "runtime_guarded_external_source_required_evidence_core_and_sound",
            "runtime_guarded_external_source_required_pcs_and_fri_from_hash_concrete_opening",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_guarded_external_source_required_hash_concrete_opening_sound",
        &[
            "external_source_opening_evidence_implies_pcs_openings",
            "providerEvidenceImpliesPcsOpenings",
            "runtime_artifact_checked_acceptance_evidence",
            "runtime_artifact_checked_acceptance_implies_verifier_accepts",
            "abstract_verifier_sound",
            "runtime_guarded_external_source_required_verifier_core_contract",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_guarded_external_source_required_audited_hash_concrete_opening_sound",
        &[
            "AssumptionBundle system",
            "RuntimeGuardedExternalSourceCheckedAcceptance",
            "RuntimeOpeningCheckedAcceptance",
            "RuntimeArtifactEvidence",
            "ExternalSourceOpeningEvidence",
            "system.pcsOpeningsValid publicInput proof",
            "system.friQueriesValid publicInput proof",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
            "assumptions.crypto.hashCollisionResistance",
        ],
    );
    lean_binding::assert_theorem_prefix_omits(
        &lean_source,
        "runtime_guarded_external_source_required_audited_hash_concrete_opening_sound",
        &["HashCollisionResistanceAssumption"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_guarded_external_source_required_audited_hash_concrete_opening_sound",
        &[
            "runtime_guarded_external_source_required_pcs_and_fri_from_concrete_opening",
            "runtime_guarded_external_source_required_evidence_core_and_sound",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_guarded_external_source_required_audited_hash_concrete_opening_sound",
        &[
            "runtime_guarded_external_source_required_hash_concrete_opening_sound",
            "external_source_opening_evidence_implies_pcs_openings",
            "providerEvidenceImpliesPcsOpenings",
            "runtime_artifact_checked_acceptance_evidence",
            "runtime_artifact_checked_acceptance_implies_verifier_accepts",
            "abstract_verifier_sound",
            "runtime_guarded_external_source_required_verifier_core_contract",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_guarded_external_source_required_audited_hash_concrete_opening_sound_with_required_evidence",
        &[
            "AssumptionBundle system",
            "RuntimeGuardedExternalSourceCheckedAcceptance",
            "RuntimeOpeningCheckedAcceptance",
            "RequiredCryptographicAssumptionStatements assumptions.crypto",
            "RequiredSemanticAssumptionStatements assumptions.semantic",
            "RuntimeArtifactEvidence",
            "ExternalSourceOpeningEvidence",
            "system.pcsOpeningsValid publicInput proof",
            "system.friQueriesValid publicInput proof",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
            "assumptions.crypto.hashCollisionResistance",
        ],
    );
    lean_binding::assert_theorem_prefix_omits(
        &lean_source,
        "runtime_guarded_external_source_required_audited_hash_concrete_opening_sound_with_required_evidence",
        &["HashCollisionResistanceAssumption"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_guarded_external_source_required_audited_hash_concrete_opening_sound_with_required_evidence",
        &[
            "assumption_bundle_carries_required_evidence",
            "runtime_guarded_external_source_required_audited_hash_concrete_opening_sound",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_guarded_external_source_required_audited_hash_concrete_opening_sound_with_required_evidence",
        &[
            "runtime_guarded_external_source_required_hash_concrete_opening_sound",
            "external_source_opening_evidence_implies_pcs_openings",
            "providerEvidenceImpliesPcsOpenings",
            "runtime_artifact_checked_acceptance_evidence",
            "runtime_artifact_checked_acceptance_implies_verifier_accepts",
            "abstract_verifier_sound",
            "runtime_guarded_external_source_required_verifier_core_contract",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
}
