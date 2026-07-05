use std::path::Path;

#[path = "support/lean_binding.rs"]
mod lean_binding;

const BOUNDARY_SOURCE_PATH: &str = "../../lean/Lzvm/BoundarySeedSnapshot.lean";
const TOP_LEVEL_SOURCE_PATH: &str = "../../lean/Lzvm.lean";
const RUNTIME_SOURCE_PATH: &str = "src/guest_pc_trace_backend.rs";

fn read_boundary_source() -> String {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    lean_binding::read_lean_source(crate_root, BOUNDARY_SOURCE_PATH)
}

fn read_top_level_source() -> String {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    lean_binding::read_lean_source(crate_root, TOP_LEVEL_SOURCE_PATH)
}

fn read_runtime_source() -> String {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(crate_root.join(RUNTIME_SOURCE_PATH))
        .expect("runtime boundary source should read")
}

fn compact_source(source: &str) -> String {
    source.chars().filter(|ch| !ch.is_whitespace()).collect()
}

fn assert_compact_contains(source: &str, snippet: &str) {
    let compact = compact_source(source);
    let expected = compact_source(snippet);
    assert!(
        compact.contains(&expected),
        "runtime boundary source should contain shape {snippet}"
    );
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

#[test]
fn lean_boundary_seed_snapshot_matches_runtime_pending_state_skip() {
    let boundary_source = read_boundary_source();
    let runtime_source = read_runtime_source();

    lean_binding::assert_theorem_declarations(
        &boundary_source,
        &[
            "runtime_boundary_pending_state_idle_plain_shape_skip_preserves_state",
            "runtime_boundary_pending_state_plain_shape_active_step_shifts_next",
            "runtime_boundary_pending_state_new_shape_records_pending",
        ],
    );
    assert_compact_contains(
        &runtime_source,
        "fn record_report_shape_state(&mut self, shape: GuestMachineReportShape)",
    );
    assert_compact_contains(
        &runtime_source,
        "let next_pending_dma = zisk_main_pending_dma_from_report_shape(shape);",
    );
    assert_compact_contains(
        &runtime_source,
        "if next_pending_dma.is_none()
            && self.last_report_pending_dma.is_none()
            && self.next_report_pending_dma.is_none()
        {
            return;
        }",
    );
    assert_compact_contains(
        &runtime_source,
        "self.last_report_pending_dma = self.next_report_pending_dma;
        self.next_report_pending_dma = next_pending_dma;",
    );
    assert_compact_contains(
        &runtime_source,
        "pending_dma: input
                .boundary_snapshot
                .next_report_pending_dma
                .or_else(|| {
                    input
                        .last_report_shape()
                        .and_then(zisk_main_pending_dma_from_report_shape)
                }),",
    );
}
