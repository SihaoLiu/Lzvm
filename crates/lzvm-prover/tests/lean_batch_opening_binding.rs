use std::path::Path;

#[test]
fn lean_batch_opening_binding_tracks_runtime_batch_helpers() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_path = crate_root.join("../../lean/Lzvm/BatchOpeningBinding.lean");
    let lean_source =
        std::fs::read_to_string(&lean_path).expect("Lean batch opening binding source should read");
    let top_level_path = crate_root.join("../../lean/Lzvm.lean");
    let top_level_source =
        std::fs::read_to_string(&top_level_path).expect("Lean top-level source should read");
    let opening_path = crate_root.join("src/witness_opening.rs");
    let opening_source =
        std::fs::read_to_string(&opening_path).expect("witness opening source should read");
    let tree_path = crate_root.join("src/witness_commitment/tree.rs");
    let tree_source =
        std::fs::read_to_string(&tree_path).expect("witness commitment tree source should read");

    assert!(
        lean_source.contains("RuntimeBatchWitnessOpeningRowsValidation")
            && lean_source.contains("perRowWitnessOpeningRowsBound")
            && lean_source
                .contains("runtime_batch_witness_opening_rows_checked_acceptance_evidence")
            && lean_source.contains("runtime_batch_witness_opening_rows_checked_acceptance_sound")
            && lean_source.contains("RuntimeOpeningSegmentBindingEvidence")
            && lean_source.contains("RuntimeOpeningEvidence")
            && lean_source.contains("SoundWitness system publicInput proof"),
        "Lean batch opening binding should expose per-row batch opening soundness evidence"
    );
    assert!(
        top_level_source.contains("import Lzvm.BatchOpeningBinding"),
        "top-level Lean module should import batch opening binding"
    );
    assert!(
        opening_source.contains("open_witness_stage_commitments_with_source_device_timing")
            && opening_source.contains("open_witness_stage_commitments("),
        "runtime witness opening builder should call batch stage opening helpers"
    );
    assert!(
        tree_source.contains("open_witness_stage_commitments_with_source_device_timing")
            && tree_source.contains("open_compact_batch_on_demand_with_source_device"),
        "runtime witness commitment tree should expose compact batch opening helpers"
    );
}
