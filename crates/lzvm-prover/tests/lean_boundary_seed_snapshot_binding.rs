use std::path::Path;

#[path = "support/lean_binding.rs"]
mod lean_binding;

const BOUNDARY_SOURCE_PATH: &str = "../../lean/Lzvm/BoundarySeedSnapshot.lean";
const TOP_LEVEL_SOURCE_PATH: &str = "../../lean/Lzvm.lean";

fn read_boundary_source() -> String {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    lean_binding::read_lean_source(crate_root, BOUNDARY_SOURCE_PATH)
}

fn read_top_level_source() -> String {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    lean_binding::read_lean_source(crate_root, TOP_LEVEL_SOURCE_PATH)
}

#[test]
fn lean_boundary_seed_snapshot_exports_pending_state_skip_contract() {
    let boundary_source = read_boundary_source();
    let top_level_source = read_top_level_source();

    assert!(
        lean_binding::contains_import(&top_level_source, "Lzvm.BoundarySeedSnapshot"),
        "top-level Lean module should import boundary seed snapshot checks"
    );
    assert!(
        boundary_source.contains("structure RuntimeBoundaryPendingState")
            && boundary_source.contains("RuntimeBoundaryPendingState.recordPlainShape")
            && boundary_source.contains("RuntimeBoundaryPendingState.recordShape"),
        "Lean boundary seed snapshot source should expose pending state transition helpers"
    );
    lean_binding::assert_theorem_declarations(
        &boundary_source,
        &[
            "runtime_boundary_pending_state_idle_plain_shape_skip_preserves_state",
            "runtime_boundary_pending_state_plain_shape_active_step_shifts_next",
            "runtime_boundary_pending_state_new_shape_records_pending",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &boundary_source,
        "runtime_boundary_pending_state_idle_plain_shape_skip_preserves_state",
        &[
            "lastEmpty : state.lastPending = none",
            "nextEmpty : state.nextPending = none",
            "state.recordShape none = state",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &boundary_source,
        "runtime_boundary_pending_state_plain_shape_active_step_shifts_next",
        &[
            "active : state.lastPending ≠ none \\/ state.nextPending ≠ none",
            "state.recordShape none = { lastPending := state.nextPending, nextPending := none }",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &boundary_source,
        "runtime_boundary_pending_state_new_shape_records_pending",
        &[
            "pending : Nat",
            "state.recordShape (some pending) =",
            "{ lastPending := state.nextPending, nextPending := some pending }",
        ],
    );
}
