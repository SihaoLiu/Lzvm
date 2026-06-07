use std::path::Path;

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
            && lean_source.contains("RuntimeTraceConstraintBackendContract")
            && lean_source.contains("runtime_trace_constraint_checked_acceptance_sound")
            && lean_source.contains(
                "runtime_trace_constraint_checked_acceptance_verifier_core_contract"
            )
            && lean_source.contains("runtime_trace_constraint_required_external_source_sound")
            && lean_source.contains(
                "runtime_trace_constraint_required_external_source_verifier_core_contract"
            )
            && lean_source.contains("requiresExternalSource ->")
            && lean_source.contains("RuntimeVerifierCoreContract system publicInput proof")
            && lean_source.contains("SoundWitness system publicInput proof"),
        "Lean trace constraint validation should expose checked and required-source verifier core projections"
    );
}
