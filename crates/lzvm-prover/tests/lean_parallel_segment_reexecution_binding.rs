use std::path::Path;

#[path = "support/lean_binding.rs"]
mod lean_binding;

#[test]
fn lean_parallel_segment_reexecution_binding_exports_streamed_lower_contracts() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_path = crate_root.join("../../lean/Lzvm/ParallelSegmentReexecution.lean");
    let lean_source =
        std::fs::read_to_string(&lean_path).expect("Lean parallel segment source should read");
    let top_level_path = crate_root.join("../../lean/Lzvm.lean");
    let top_level_source =
        std::fs::read_to_string(&top_level_path).expect("Lean top-level source should read");

    assert!(
        lean_binding::contains_import(&top_level_source, "Lzvm.ParallelSegmentReexecution"),
        "top-level Lean module should import parallel segment reexecution"
    );
    assert!(
        lean_source.contains("RuntimeStreamedSegmentLowerCheckedAcceptance")
            && lean_source.contains("RuntimeStreamedSegmentLowerContract")
            && lean_source.contains("streamedLowerAccepted")
            && lean_source.contains("streamedLowerEmittedSegment")
            && lean_source.contains("streamedLowerSeedCheckBypassed"),
        "Lean parallel segment binding should expose streamed lower acceptance and emitted segment obligations"
    );
    for field in [
        "streamedLowerAcceptedImpliesParallelReexecutionAccepted",
        "streamedLowerAcceptedImpliesSegmentIndex",
        "streamedLowerAcceptedImpliesSeedChain",
        "streamedLowerAcceptedImpliesSerialEquivalentSegment",
        "streamedLowerAcceptedImpliesNoSeedCheckBypass",
    ] {
        assert!(
            lean_source.contains(field),
            "streamed lower validation should expose {field}"
        );
    }
    lean_binding::assert_theorem_declarations(
        &lean_source,
        &[
            "runtime_parallel_segment_reexecution_checked_acceptance_contract",
            "runtime_parallel_segment_reexecution_checked_acceptance_rejection_contract",
            "runtime_streamed_segment_lower_checked_acceptance_contract",
            "runtime_streamed_segment_lower_checked_acceptance_parallel_contract",
            "runtime_streamed_segment_lower_checked_acceptance_parallel_rejection_contract",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_streamed_segment_lower_checked_acceptance_contract",
        &[
            "RuntimeStreamedSegmentLowerCheckedAcceptance",
            "RuntimeStreamedSegmentLowerContract",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_streamed_segment_lower_checked_acceptance_contract",
        &[
            "streamedLowerAcceptedImpliesParallelReexecutionAccepted",
            "streamedLowerAcceptedImpliesSegmentIndex",
            "streamedLowerAcceptedImpliesSeedChain",
            "streamedLowerAcceptedImpliesSerialEquivalentSegment",
            "streamedLowerAcceptedImpliesNoSeedCheckBypass",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_streamed_segment_lower_checked_acceptance_parallel_contract",
        &[
            "runtime_parallel_segment_reexecution_checked_acceptance_contract",
            "streamedLowerAcceptedImpliesParallelReexecutionAccepted",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_streamed_segment_lower_checked_acceptance_parallel_rejection_contract",
        &[
            "runtime_parallel_segment_reexecution_checked_acceptance_rejection_contract",
            "streamedLowerAcceptedImpliesParallelReexecutionAccepted",
        ],
    );
}
