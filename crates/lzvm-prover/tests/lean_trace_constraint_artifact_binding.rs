use std::path::Path;

#[path = "support/lean_binding.rs"]
mod lean_binding;

#[test]
fn lean_trace_constraint_artifact_binding_exports_core_contract_projection() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_path = crate_root.join("../../lean/Lzvm/TraceConstraintArtifactBinding.lean");
    let lean_source = std::fs::read_to_string(&lean_path)
        .expect("Lean trace constraint artifact binding source should read");
    let top_level_path = crate_root.join("../../lean/Lzvm.lean");
    let top_level_source =
        std::fs::read_to_string(&top_level_path).expect("Lean top-level source should read");

    assert!(
        top_level_source.contains("import Lzvm.TraceConstraintArtifactBinding"),
        "top-level Lean module should import trace constraint artifact binding"
    );
    assert!(
        lean_source.contains("RuntimeTraceConstraintArtifactBindingValidation")
            && lean_source.contains("RuntimeTraceConstraintPreflightBindingEvidence")
            && lean_source.contains("RuntimeTraceConstraintEvidence")
            && lean_source.contains("RuntimeTraceConstraintSoundnessObligations")
            && lean_source.contains("RuntimeTraceConstraintCheckedAcceptance")
            && lean_source.contains("RuntimeVerifierCoreContract system publicInput proof")
            && lean_source.contains("SoundWitness system publicInput proof"),
        "Lean trace constraint artifact binding should expose preflight evidence and verifier core clauses"
    );
    lean_binding::assert_theorem_declarations(
        &lean_source,
        &[
            "runtime_trace_constraint_artifact_binding_checked_acceptance_pcs_fri_backend_contract",
            "runtime_trace_constraint_artifact_binding_checked_acceptance_sound",
            "runtime_trace_constraint_artifact_binding_checked_acceptance_soundness_obligations",
            "runtime_trace_constraint_artifact_binding_checked_acceptance_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_trace_constraint_artifact_binding_checked_acceptance_pcs_fri_backend_contract",
        &[
            "RuntimeTraceConstraintPreflightBindingEvidence",
            "system.pcsOpeningsValid publicInput proof",
            "system.friQueriesValid publicInput proof",
            "RuntimeTraceConstraintArtifactBindingEvidence",
            "RuntimeTraceConstraintBackendContract",
        ],
    );
    lean_binding::assert_theorem_prefix_omits(
        &lean_source,
        "runtime_trace_constraint_artifact_binding_checked_acceptance_pcs_fri_backend_contract",
        &["AssumptionBundle"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_trace_constraint_artifact_binding_checked_acceptance_verifier_core_contract",
        &[
            "runtime_trace_constraint_artifact_binding_checked_acceptance_trace_constraint",
            "runtime_trace_constraint_checked_acceptance_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_trace_constraint_artifact_binding_checked_acceptance_verifier_core_contract",
        &[
            "runtime_trace_constraint_artifact_binding_checked_acceptance_sound",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
}
