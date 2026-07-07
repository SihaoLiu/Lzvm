use std::path::Path;

#[path = "support/lean_binding.rs"]
mod lean_binding;

#[test]
fn lean_workspace_modules_are_reachable_from_entrypoint() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_entrypoint = crate_root.join("../../lean/Lzvm.lean");
    let lean_workspace = crate_root.join("../../lean");

    lean_binding::assert_all_workspace_lean_modules_reachable_from_entrypoint(
        &lean_entrypoint,
        &lean_workspace,
    );
}

#[test]
fn lean_workspace_sources_have_no_open_proof_tokens() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_workspace = crate_root.join("../../lean");

    lean_binding::assert_no_uncontrolled_lean_placeholders(&lean_workspace);
}
