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
        top_level_source.contains("import Lzvm.TraceConstraintValidation"),
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
            "runtime_trace_constraint_evidence_implies_semantic_evidence_complete",
            "runtime_trace_constraint_checked_acceptance_semantic_evidence_complete",
            "runtime_trace_constraint_evidence_implies_backend_contract",
            "runtime_trace_constraint_checked_acceptance_backend_contract",
            "runtime_trace_constraint_checked_acceptance_verifier_core_contract",
            "runtime_trace_constraint_required_external_source_sound",
            "runtime_trace_constraint_required_external_source_verifier_core_contract",
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
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_trace_constraint_checked_acceptance_semantic_evidence_complete",
        &["runtime_trace_constraint_checked_acceptance_trace_witness_evidence"],
    );
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
