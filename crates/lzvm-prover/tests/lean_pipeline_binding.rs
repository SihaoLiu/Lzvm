use std::path::Path;

#[test]
fn lean_pipeline_binding_exports_required_external_source_soundness() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_path = crate_root.join("../../lean/Lzvm/PipelineBinding.lean");
    let lean_source =
        std::fs::read_to_string(&lean_path).expect("Lean pipeline binding source should read");
    let setup_preflight_path = crate_root.join("src/setup_preflight.rs");
    let setup_preflight_source =
        std::fs::read_to_string(&setup_preflight_path).expect("setup preflight source should read");

    assert!(
        lean_source.contains("runtime_pipeline_binding_required_external_source_sound")
            && lean_source.contains("runtime_trace_constraint_required_external_source_pcs_sound")
            && lean_source.contains("runtime_opening_required_external_source_sound"),
        "Lean pipeline binding should expose required external source evidence through trace and opening soundness"
    );
    assert!(
        setup_preflight_source.contains("validate_global_source_lookup_hints")
            && setup_preflight_source.contains("SourceLookupBalance::default")
            && setup_preflight_source.contains("validate_all_units"),
        "setup preflight should keep source lookup balance validation wired into proof checks"
    );
}
