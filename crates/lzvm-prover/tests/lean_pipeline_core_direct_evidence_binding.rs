use std::path::Path;

#[path = "support/lean_binding.rs"]
mod lean_binding;

const CORE_DERIVED_SOURCE_PATH: &str = "../../lean/Lzvm/PipelineBinding/Core/Derived.lean";

#[test]
fn lean_pipeline_core_derived_routes_required_evidence_directly() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_source = lean_binding::read_lean_source(crate_root, CORE_DERIVED_SOURCE_PATH);
    let theorem = "runtime_pipeline_binding_checked_acceptance_audited_soundness_obligations";

    lean_binding::assert_theorem_declarations(&lean_source, &[theorem]);
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        theorem,
        &[
            "assumption_bundle_carries_required_crypto_evidence",
            "assumption_bundle_carries_required_semantic_evidence",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        theorem,
        &["assumption_bundle_carries_required_evidence"],
    );
}
