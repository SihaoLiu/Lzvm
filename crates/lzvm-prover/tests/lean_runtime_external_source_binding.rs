use std::path::Path;

#[path = "support/lean_binding.rs"]
mod lean_binding;

#[test]
fn lean_runtime_external_source_binding_exports_core_contract_projection() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_path = crate_root.join("../../lean/Lzvm/RuntimeExternalSource.lean");
    let lean_source =
        std::fs::read_to_string(&lean_path).expect("Lean runtime external source should read");
    let top_level_path = crate_root.join("../../lean/Lzvm.lean");
    let top_level_source =
        std::fs::read_to_string(&top_level_path).expect("Lean top-level source should read");

    assert!(
        lean_binding::contains_import(&top_level_source, "Lzvm.RuntimeExternalSource"),
        "top-level Lean module should import runtime external source"
    );
    assert!(
        lean_source.contains("RuntimeExternalSourceCheckedAcceptance")
            && lean_source.contains("RuntimeGuardedExternalSourceCheckedAcceptance")
            && lean_source.contains("RuntimeVerifierCoreContract system publicInput proof")
            && lean_source.contains("SoundWitness system publicInput proof"),
        "Lean runtime external source binding should expose checked soundness and verifier core projections"
    );
    lean_binding::assert_theorem_declarations(
        &lean_source,
        &[
            "runtime_external_source_checked_acceptance_obligations",
            "runtime_external_source_checked_acceptance_pcs_without_assumptions",
            "runtime_external_source_checked_acceptance_sound",
            "runtime_external_source_checked_acceptance_verifier_core_contract",
            "runtime_external_source_checked_acceptance_evidence_core_and_sound",
            "runtime_external_source_checked_acceptance_audited_core_contract",
            "runtime_guarded_external_source_required_pcs_without_assumptions",
            "runtime_guarded_external_source_checked_acceptance_sound",
            "runtime_guarded_external_source_checked_acceptance_verifier_core_contract",
            "runtime_guarded_external_source_checked_acceptance_evidence_core_and_sound",
            "runtime_guarded_external_source_checked_acceptance_audited_core_contract",
        ],
    );
    lean_binding::assert_theorem_prefix_omits(
        &lean_source,
        "runtime_external_source_checked_acceptance_pcs_without_assumptions",
        &["AssumptionBundle"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_external_source_checked_acceptance_pcs_without_assumptions",
        &["system.pcsOpeningsValid publicInput proof"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_external_source_checked_acceptance_obligations",
        &[
            "(assumptions : AssumptionBundle system)",
            "RuntimeExternalSourceCheckedAcceptance",
            "RuntimeArtifactSoundnessObligations",
            "ExternalSourceOpeningSoundnessObligations",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_external_source_checked_acceptance_obligations",
        &[
            "runtime_artifact_checked_acceptance_obligations",
            "runtime_artifact_checked_acceptance_implies_verifier_accepts",
            "external_source_opening_checked_acceptance_obligations",
        ],
    );
    lean_binding::assert_theorem_prefix_omits(
        &lean_source,
        "runtime_guarded_external_source_required_pcs_without_assumptions",
        &["AssumptionBundle"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_guarded_external_source_required_pcs_without_assumptions",
        &[
            "requiresExternalSource ->",
            "system.pcsOpeningsValid publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_external_source_checked_acceptance_verifier_core_contract",
        &["runtime_artifact_checked_acceptance_verifier_core_contract"],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_external_source_checked_acceptance_verifier_core_contract",
        &[
            "runtime_external_source_checked_acceptance_sound",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_external_source_checked_acceptance_evidence_core_and_sound",
        &[
            "RuntimeExternalSourceCheckedAcceptance",
            "RuntimeArtifactEvidence",
            "ExternalSourceOpeningEvidence",
            "system.pcsOpeningsValid publicInput proof",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_external_source_checked_acceptance_evidence_core_and_sound",
        &[
            "runtime_external_source_checked_acceptance_sound",
            "runtime_external_source_checked_acceptance_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_external_source_checked_acceptance_evidence_core_and_sound",
        &["sound_witness_implies_verifier_core_contract"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_external_source_checked_acceptance_audited_core_contract",
        &[
            "RequiredCryptographicAssumptionStatements assumptions.crypto",
            "RequiredSemanticAssumptionStatements assumptions.semantic",
            "RuntimeExternalSourceCheckedAcceptance",
            "RuntimeArtifactEvidence",
            "ExternalSourceOpeningEvidence",
            "system.pcsOpeningsValid publicInput proof",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_external_source_checked_acceptance_audited_core_contract",
        &[
            "assumption_bundle_carries_required_evidence",
            "runtime_external_source_checked_acceptance_evidence_core_and_sound",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_guarded_external_source_checked_acceptance_verifier_core_contract",
        &["runtime_artifact_checked_acceptance_verifier_core_contract"],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_guarded_external_source_checked_acceptance_verifier_core_contract",
        &[
            "runtime_guarded_external_source_checked_acceptance_sound",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_guarded_external_source_checked_acceptance_evidence_core_and_sound",
        &[
            "RuntimeGuardedExternalSourceCheckedAcceptance",
            "ExternalSourceOpeningRequirement",
            "system.pcsOpeningsValid publicInput proof",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_guarded_external_source_checked_acceptance_evidence_core_and_sound",
        &[
            "runtime_guarded_external_source_checked_acceptance_sound",
            "runtime_guarded_external_source_checked_acceptance_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_guarded_external_source_checked_acceptance_evidence_core_and_sound",
        &["sound_witness_implies_verifier_core_contract"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_guarded_external_source_checked_acceptance_audited_core_contract",
        &[
            "RequiredCryptographicAssumptionStatements assumptions.crypto",
            "RequiredSemanticAssumptionStatements assumptions.semantic",
            "RuntimeGuardedExternalSourceCheckedAcceptance",
            "ExternalSourceOpeningRequirement",
            "system.pcsOpeningsValid publicInput proof",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_guarded_external_source_checked_acceptance_audited_core_contract",
        &[
            "assumption_bundle_carries_required_evidence",
            "runtime_guarded_external_source_checked_acceptance_evidence_core_and_sound",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_guarded_external_source_checked_acceptance_sound",
        &["assumption_bundle_pcs_opening_soundness"],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_guarded_external_source_checked_acceptance_sound",
        &["assumptions.crypto.pcs_opening_sound"],
    );
}
