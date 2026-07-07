use std::path::Path;

#[path = "support/lean_binding.rs"]
mod lean_binding;

#[test]
fn lean_trace_constraint_validation_binding_exports_core_contract_projection() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_path = crate_root.join("../../lean/Lzvm/TraceConstraintValidation.lean");
    let lean_source =
        std::fs::read_to_string(&lean_path).expect("Lean trace constraint validation should read");
    let top_level_path = crate_root.join("../../lean/Lzvm.lean");
    let top_level_source =
        std::fs::read_to_string(&top_level_path).expect("Lean top-level source should read");

    assert!(
        lean_binding::contains_import(&top_level_source, "Lzvm.TraceConstraintValidation"),
        "top-level Lean module should import trace constraint validation"
    );
    assert!(
        lean_source.contains("RuntimeTraceConstraintValidation")
            && lean_source.contains("RuntimeTraceConstraintSoundnessObligations")
            && lean_source.contains("RuntimeTraceConstraintSemanticEvidenceComplete")
            && lean_source.contains("RuntimeTraceConstraintBackendContract")
            && lean_source.contains("requiresExternalSource ->")
            && lean_source.contains("RuntimeVerifierCoreContract system publicInput proof")
            && lean_source.contains("SoundWitness system publicInput proof"),
        "Lean trace constraint validation should expose checked and required-source verifier core projections"
    );
    lean_binding::assert_theorem_declarations(
        &lean_source,
        &[
            "runtime_trace_constraint_checked_acceptance_sound",
            "runtime_trace_constraint_checked_acceptance_obligations",
            "runtime_trace_constraint_checked_acceptance_artifact_binding_evidence",
            "runtime_trace_constraint_checked_acceptance_opening_evidence",
            "runtime_trace_constraint_checked_acceptance_trace_witness_evidence",
            "runtime_trace_constraint_checked_acceptance_evidence",
            "runtime_trace_constraint_checked_acceptance_implies_verifier_accepts",
            "fromCheckedAcceptance",
            "runtime_trace_constraint_evidence_implies_opening_evidence",
            "runtime_trace_constraint_evidence_implies_artifact_binding_evidence",
            "runtime_trace_constraint_checked_acceptance_witness_commitment_binding",
            "runtime_trace_constraint_checked_acceptance_constraint_catalog_binding",
            "runtime_trace_constraint_soundness_obligations_imply_witness_commitment_binding",
            "runtime_trace_constraint_soundness_obligations_imply_constraint_catalog_binding",
            "runtime_trace_constraint_evidence_implies_trace_witness_evidence",
            "runtime_trace_constraint_evidence_implies_semantic_evidence_complete",
            "runtime_trace_constraint_checked_acceptance_semantic_evidence_complete",
            "runtime_trace_constraint_evidence_implies_backend_contract",
            "runtime_trace_constraint_checked_acceptance_backend_contract",
            "runtime_trace_constraint_checked_acceptance_verifier_core_contract",
            "runtime_trace_constraint_checked_acceptance_evidence_core_and_sound",
            "runtime_trace_constraint_checked_acceptance_audited_core_contract",
            "runtime_trace_constraint_required_external_source_sound",
            "runtime_trace_constraint_required_external_source_verifier_core_contract",
            "runtime_trace_constraint_required_external_source_evidence_core_and_sound",
            "runtime_trace_constraint_required_external_source_accepts_backend_core_sound_witness",
        ],
    );
    lean_binding::assert_theorem_declarations(
        &lean_source,
        &["runtime_trace_constraint_checked_acceptance_pcs_fri_backend_contract"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_trace_constraint_checked_acceptance_pcs_fri_backend_contract",
        &[
            "system.pcsOpeningsValid publicInput proof",
            "system.friQueriesValid publicInput proof",
            "RuntimeTraceConstraintArtifactBindingEvidence",
            "RuntimeTraceConstraintSemanticEvidenceComplete",
            "RuntimeTraceConstraintBackendContract",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_trace_constraint_checked_acceptance_artifact_binding_evidence",
        &[
            "RuntimeTraceConstraintArtifactBindingEvidence",
            "RuntimeTraceConstraintCheckedAcceptance",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_trace_constraint_checked_acceptance_artifact_binding_evidence",
        &[
            "traceConstraintAcceptedImpliesTraceEvidenceMatchesWitnessCommitments",
            "traceConstraintAcceptedImpliesTraceEvidenceMatchesConstraintCatalog",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_trace_constraint_checked_acceptance_opening_evidence",
        &[
            "(assumptions : AssumptionBundle system)",
            "RuntimeOpeningEvidence",
            "requiresExternalSource",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_trace_constraint_checked_acceptance_opening_evidence",
        &[
            "traceConstraintAcceptedImpliesOpeningAccepted",
            "runtime_opening_checked_acceptance_evidence",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_trace_constraint_checked_acceptance_evidence",
        &[
            "(assumptions : AssumptionBundle system)",
            "RuntimeTraceConstraintEvidence",
            "requiresExternalSource",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_trace_constraint_checked_acceptance_evidence",
        &[
            "runtime_trace_constraint_checked_acceptance_opening_evidence",
            "runtime_trace_constraint_checked_acceptance_artifact_binding_evidence",
            "runtime_trace_constraint_checked_acceptance_trace_witness_evidence",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_trace_constraint_checked_acceptance_implies_verifier_accepts",
        &["system.accepts publicInput proof"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_trace_constraint_checked_acceptance_implies_verifier_accepts",
        &[
            "traceConstraintAcceptedImpliesOpeningAccepted",
            "openingAcceptedImpliesRuntimeSoundnessAccepted",
            "runtime_artifact_checked_acceptance_implies_verifier_accepts",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_trace_constraint_evidence_implies_opening_evidence",
        &["RuntimeOpeningEvidence"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_trace_constraint_evidence_implies_opening_evidence",
        &["exact evidence.left"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_trace_constraint_evidence_implies_artifact_binding_evidence",
        &["RuntimeTraceConstraintArtifactBindingEvidence"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_trace_constraint_evidence_implies_artifact_binding_evidence",
        &["exact evidence.right.left"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_trace_constraint_checked_acceptance_witness_commitment_binding",
        &[
            "runtime_trace_constraint_checked_acceptance_artifact_binding_evidence",
            "accepted).left",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_trace_constraint_checked_acceptance_constraint_catalog_binding",
        &[
            "runtime_trace_constraint_checked_acceptance_artifact_binding_evidence",
            "accepted).right",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_trace_constraint_soundness_obligations_imply_witness_commitment_binding",
        &["exact obligations.left.right.left.left"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_trace_constraint_soundness_obligations_imply_constraint_catalog_binding",
        &["exact obligations.left.right.left.right"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_trace_constraint_evidence_implies_trace_witness_evidence",
        &[
            "exists witness trace constraints",
            "validation.traceExtracted artifact publicInput proof trace",
            "validation.constraintsEvaluated artifact publicInput proof constraints trace",
            "validation.witnessExtractedFromTrace artifact publicInput proof witness trace",
            "validation.constraintBackendConformant",
            "system.traceConsistent publicInput proof trace",
            "system.constraintsSatisfied constraints trace",
            "system.witnessMatchesTrace witness trace",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_trace_constraint_evidence_implies_trace_witness_evidence",
        &["exact evidence.right.right"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_trace_constraint_checked_acceptance_semantic_evidence_complete",
        &["runtime_trace_constraint_checked_acceptance_trace_witness_evidence"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_trace_constraint_checked_acceptance_obligations",
        &["RuntimeTraceConstraintSoundnessObligations.fromCheckedAcceptance"],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_trace_constraint_checked_acceptance_obligations",
        &["assumptions.semantic.public_input_binding"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_trace_constraint_checked_acceptance_sound",
        &["runtime_trace_constraint_evidence_implies_sound_witness"],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_trace_constraint_checked_acceptance_sound",
        &["assumptions.semantic.public_input_binding"],
    );
    for theorem_name in [
        "fromCheckedAcceptance",
        "runtime_trace_constraint_evidence_implies_sound_witness",
    ] {
        lean_binding::assert_theorem_body_contains(
            &lean_source,
            theorem_name,
            &["assumption_bundle_public_input_binding"],
        );
        lean_binding::assert_theorem_body_omits(
            &lean_source,
            theorem_name,
            &["assumptions.semantic.public_input_binding"],
        );
    }
    lean_binding::assert_theorem_prefix_omits(
        &lean_source,
        "runtime_trace_constraint_checked_acceptance_pcs_fri_backend_contract",
        &["AssumptionBundle"],
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_trace_constraint_required_external_source_accepts_backend_core_sound_witness"
        )
        .contains("(assumptions : AssumptionBundle system)")
            && theorem_prefix(
                &lean_source,
                "runtime_trace_constraint_required_external_source_accepts_backend_core_sound_witness"
            )
            .contains("requiresExternalSource ->")
            && theorem_prefix(
                &lean_source,
                "runtime_trace_constraint_required_external_source_accepts_backend_core_sound_witness"
            )
            .contains("system.accepts publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_trace_constraint_required_external_source_accepts_backend_core_sound_witness"
            )
            .contains("ExternalSourceOpeningEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_trace_constraint_required_external_source_accepts_backend_core_sound_witness"
            )
            .contains("RuntimeTraceConstraintBackendContract")
            && theorem_prefix(
                &lean_source,
                "runtime_trace_constraint_required_external_source_accepts_backend_core_sound_witness"
            )
            .contains("RuntimeVerifierCoreContract system publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_trace_constraint_required_external_source_accepts_backend_core_sound_witness"
            )
            .contains("SoundWitness system publicInput proof"),
        "required external-source trace constraints should package acceptance, external-source evidence, backend contract, verifier core obligations, and sound witness"
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_trace_constraint_required_external_source_verifier_core_contract",
        &["runtime_trace_constraint_checked_acceptance_verifier_core_contract"],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_trace_constraint_required_external_source_verifier_core_contract",
        &[
            "runtime_trace_constraint_required_external_source_sound",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_trace_constraint_checked_acceptance_evidence_core_and_sound",
        &[
            "RuntimeTraceConstraintEvidence",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_trace_constraint_checked_acceptance_evidence_core_and_sound",
        &[
            "runtime_trace_constraint_checked_acceptance_sound",
            "runtime_trace_constraint_checked_acceptance_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_trace_constraint_checked_acceptance_evidence_core_and_sound",
        &["sound_witness_implies_verifier_core_contract"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_trace_constraint_checked_acceptance_audited_core_contract",
        &[
            "RequiredCryptographicAssumptionStatements assumptions.crypto",
            "RequiredSemanticAssumptionStatements assumptions.semantic",
            "RuntimeTraceConstraintEvidence",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
            "requiresExternalSource",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_trace_constraint_checked_acceptance_audited_core_contract",
        &[
            "assumption_bundle_carries_required_evidence",
            "runtime_trace_constraint_checked_acceptance_evidence_core_and_sound",
            "auditedAssumptions.left",
            "auditedAssumptions.right",
            "contracts",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_trace_constraint_checked_acceptance_audited_core_contract",
        &[
            "runtime_trace_constraint_checked_acceptance_sound",
            "runtime_trace_constraint_checked_acceptance_verifier_core_contract",
            "sound_witness_implies_verifier_core_contract",
            "abstract_verifier_sound",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_trace_constraint_required_external_source_evidence_core_and_sound",
        &[
            "requiresExternalSource ->",
            "RuntimeTraceConstraintEvidence",
            "ExternalSourceOpeningEvidence",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_trace_constraint_required_external_source_evidence_core_and_sound",
        &[
            "runtime_trace_constraint_required_external_source_sound",
            "runtime_trace_constraint_required_external_source_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_trace_constraint_required_external_source_evidence_core_and_sound",
        &["sound_witness_implies_verifier_core_contract"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_trace_constraint_required_external_source_accepts_backend_core_sound_witness",
        &["runtime_trace_constraint_checked_acceptance_verifier_core_contract"],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_trace_constraint_required_external_source_accepts_backend_core_sound_witness",
        &["sound_witness_implies_verifier_core_contract"],
    );
}

fn theorem_prefix(source: &str, name: &str) -> String {
    let theorem_start = source
        .find(&format!("theorem {name}"))
        .unwrap_or_else(|| panic!("Lean source should contain theorem {name}"));
    let proof_start = source[theorem_start..]
        .find(" := by")
        .unwrap_or_else(|| panic!("Lean theorem {name} should have a proof body"));
    source[theorem_start..theorem_start + proof_start].to_owned()
}
