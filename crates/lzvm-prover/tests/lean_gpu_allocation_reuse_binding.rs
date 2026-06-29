use std::path::Path;

#[path = "support/lean_binding.rs"]
mod lean_binding;

#[test]
fn lean_gpu_allocation_reuse_exports_cached_written_contents_projection() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_source = lean_binding::read_gpu_runtime_sources(crate_root);

    lean_binding::assert_theorem_declarations(
        &lean_source,
        &[
            "gpu_allocation_cache_reuse_preserves_written_contents",
            "gpu_allocation_checked_acceptance_projects_cached_written_contents",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "gpu_allocation_checked_acceptance_projects_cached_written_contents",
        &[
            "gpu_allocation_checked_acceptance_projects_written_contents",
            "gpu_allocation_cache_reuse_preserves_written_contents",
        ],
    );
}
