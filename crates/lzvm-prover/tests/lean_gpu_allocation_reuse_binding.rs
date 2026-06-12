use std::path::Path;

#[path = "support/lean_binding.rs"]
mod lean_binding;

#[test]
fn lean_gpu_allocation_reuse_exports_cached_written_contents_projection() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_path = crate_root.join("../../lean/Lzvm/AuxiliaryChecks/GpuRuntime.lean");
    let lean_source = std::fs::read_to_string(&lean_path).expect("Lean GPU runtime should read");

    lean_binding::assert_theorem_declarations(
        &lean_source,
        &[
            "gpu_allocation_cache_reuse_preserves_written_contents",
            "gpu_allocation_checked_acceptance_projects_cached_written_contents",
        ],
    );
    assert!(
        lean_source.contains("gpu_allocation_checked_acceptance_projects_written_contents")
            && lean_source.contains("gpu_allocation_cache_reuse_preserves_written_contents"),
        "cached allocation written contents should be projected through checked fresh contents and same-request reuse"
    );
}
