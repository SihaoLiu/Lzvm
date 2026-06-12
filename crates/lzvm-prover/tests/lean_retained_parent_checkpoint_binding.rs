use std::path::Path;

#[path = "support/lean_binding.rs"]
mod lean_binding;

#[test]
fn lean_retained_parent_checkpoint_binding_tracks_runtime_opening_contract() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_path = crate_root.join("../../lean/Lzvm/RetainedParentCheckpointOpening.lean");
    let lean_source = std::fs::read_to_string(&lean_path)
        .expect("Lean retained parent checkpoint opening source should read");
    let top_level_path = crate_root.join("../../lean/Lzvm.lean");
    let top_level_source =
        std::fs::read_to_string(&top_level_path).expect("Lean top-level source should read");
    let merkle_path = crate_root.join("src/merkle_hash.rs");
    let merkle_source =
        std::fs::read_to_string(&merkle_path).expect("Merkle hash source should read");
    let source_hot_paths_path = crate_root.join("tests/source_hot_paths.rs");
    let source_hot_paths =
        std::fs::read_to_string(&source_hot_paths_path).expect("source hot path tests should read");
    let values_path = crate_root.join("src/witness_commitment/values.rs");
    let values_source = std::fs::read_to_string(&values_path)
        .expect("witness commitment values source should read");

    assert!(
        lean_source.contains("RuntimeRetainedParentCheckpointOpeningValidation")
            && lean_source.contains("retainedParentCheckpointLevelAvailable")
            && lean_source.contains("retainedParentCheckpointLowerPrefixBound")
            && lean_source.contains("retainedParentCheckpointUpperSuffixBound")
            && lean_source.contains("retainedParentCheckpointStitchedPathBound")
            && lean_source.contains("retainedParentCheckpointRootMatchesExpectedRoot")
            && lean_source.contains("retainedParentCheckpointRowsFromSource")
            && lean_source.contains("retainedParentCheckpointRowsBoundToQueryPlan")
            && lean_source.contains("retainedParentCheckpointPrefixBatchUsed")
            && lean_source.contains(
                "retainedParentCheckpointPrefixBatchImpliesLowerPrefixBound"
            )
            && lean_source.contains(
                "retainedParentCheckpointChecksImplyPerRowWitnessOpeningRowsBound"
            )
            && lean_source.contains("RuntimeRetainedParentCheckpointOpeningEvidence")
            && lean_source
                .contains("def RuntimeRetainedParentCheckpointOpeningDigestContract")
            && lean_source
                .contains("def RuntimeRetainedParentCheckpointOpeningPrefixBatchContract")
            && lean_source.contains("def RuntimeRetainedParentCheckpointOpeningSourceContract")
            && lean_source
                .contains("def RuntimeRetainedParentCheckpointOpeningRetainedRowsContract")
            && lean_source.contains("RuntimeBatchWitnessOpeningRowsBoundContract")
            && lean_source.contains("RuntimeVerifierCoreContract system publicInput proof")
            && lean_source.contains("SoundWitness system publicInput proof"),
        "Lean retained parent checkpoint opening binding should expose checkpoint lower-prefix, upper-suffix, stitched-path, root equality, and source-row evidence"
    );
    lean_binding::assert_theorem_declarations(
        &lean_source,
        &[
            "runtime_retained_parent_checkpoint_opening_checked_acceptance_evidence",
            "runtime_retained_parent_checkpoint_opening_evidence_implies_digest_contract",
            "runtime_retained_parent_checkpoint_opening_evidence_implies_batch_rows_evidence",
            "runtime_retained_parent_checkpoint_opening_checked_acceptance_batch_rows_evidence",
            "runtime_retained_parent_checkpoint_opening_evidence_implies_batch_rows_bound_contract",
            "runtime_retained_parent_checkpoint_opening_checked_acceptance_batch_rows_bound_contract",
            "runtime_retained_parent_checkpoint_opening_evidence_implies_opening_evidence",
            "runtime_retained_parent_checkpoint_opening_checked_acceptance_opening_evidence",
            "runtime_retained_parent_checkpoint_opening_evidence_implies_retained_rows_contract",
            "runtime_retained_parent_checkpoint_opening_checked_acceptance_retained_rows_contract",
            "runtime_retained_parent_checkpoint_opening_checked_acceptance_digest_contract",
            "runtime_retained_parent_checkpoint_prefix_batch_implies_lower_prefix_bound",
            "runtime_retained_parent_checkpoint_opening_checked_acceptance_prefix_batch_contract",
            "runtime_retained_parent_checkpoint_opening_checked_acceptance_source_contract",
            "runtime_retained_parent_checkpoint_opening_checked_acceptance_sound",
            "runtime_retained_parent_checkpoint_opening_checked_acceptance_verifier_core_contract",
            "runtime_retained_parent_checkpoint_opening_checked_acceptance_opening_and_core_contract",
            "runtime_retained_parent_checkpoint_opening_checked_acceptance_source_and_core_contract",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_retained_parent_checkpoint_opening_checked_acceptance_source_and_core_contract",
        &[
            "RuntimeRetainedParentCheckpointOpeningSourceContract",
            "RuntimeRetainedParentCheckpointOpeningRetainedRowsContract",
            "RuntimeVerifierCoreContract system publicInput proof",
        ],
    );
    assert!(
        top_level_source.contains("import Lzvm.RetainedParentCheckpointOpening"),
        "top-level Lean module should import retained parent checkpoint opening binding"
    );
    assert!(
        merkle_source.contains("opening_path_prefix_for_source_row")
            && merkle_source.contains("opening_path_prefix_batch_for_source_rows")
            && merkle_source.contains("cuda_poseidon2_width8_merkle_digest_opening_prefix_batch_device")
            && merkle_source.contains("cuda_poseidon2_width16_merkle_digest_opening_prefix_batch_device")
            && source_hot_paths.contains("cuda_retained_checkpoint_opening_batches_lower_prefix_work")
            && values_source.contains("RetainedCudaParentCheckpointLevel")
            && values_source.contains("opening_path_for_source_row")
            && values_source.contains("retained_parent_checkpoint_level"),
        "runtime retained parent checkpoint opening should expose batch lower-prefix and upper-suffix path interfaces"
    );
}
