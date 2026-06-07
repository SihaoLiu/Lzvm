use std::path::Path;

#[test]
fn lean_retained_leaf_digest_binding_tracks_runtime_opening_contract() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_path = crate_root.join("../../lean/Lzvm/RetainedLeafDigestOpening.lean");
    let lean_source = std::fs::read_to_string(&lean_path)
        .expect("Lean retained leaf digest opening source should read");
    let top_level_path = crate_root.join("../../lean/Lzvm.lean");
    let top_level_source =
        std::fs::read_to_string(&top_level_path).expect("Lean top-level source should read");
    let values_path = crate_root.join("src/witness_commitment/values.rs");
    let values_source = std::fs::read_to_string(&values_path)
        .expect("witness commitment values source should read");

    assert!(
        lean_source.contains("RuntimeRetainedLeafDigestOpeningValidation")
            && lean_source.contains("retainedLeafDigestRowsFromSource")
            && lean_source.contains("retainedLeafDigestPathBound")
            && lean_source.contains("retainedLeafDigestRootMatchesExpectedRoot")
            && lean_source.contains("retainedLeafDigestRowsBoundToQueryPlan")
            && lean_source.contains("retainedLeafDigestChecksImplyPerRowWitnessOpeningRowsBound")
            && lean_source.contains("RuntimeRetainedLeafDigestOpeningEvidence")
            && lean_source.contains("def RuntimeRetainedLeafDigestOpeningDigestContract")
            && lean_source.contains("def RuntimeRetainedLeafDigestOpeningRetainedRowsContract")
            && lean_source.contains("RuntimeRetainedLeafDigestOpeningDigestContract")
            && lean_source.contains("RuntimeRetainedLeafDigestOpeningRetainedRowsContract")
            && lean_source.contains("RuntimeBatchWitnessOpeningRowsEvidence")
            && lean_source
                .contains("runtime_retained_leaf_digest_opening_checked_acceptance_evidence")
            && lean_source
                .contains("runtime_retained_leaf_digest_opening_evidence_implies_digest_contract")
            && lean_source.contains(
                "runtime_retained_leaf_digest_opening_checked_acceptance_digest_contract"
            )
            && lean_source.contains("runtime_retained_leaf_digest_opening_checked_acceptance_sound")
            && lean_source.contains(
                "runtime_retained_leaf_digest_opening_checked_acceptance_verifier_core_contract"
            )
            && lean_source.contains("RuntimeVerifierCoreContract system publicInput proof")
            && lean_source.contains("SoundWitness system publicInput proof"),
        "Lean retained leaf digest opening binding should expose source rows, Merkle path, root equality, and soundness evidence"
    );
    assert!(
        top_level_source.contains("import Lzvm.RetainedLeafDigestOpening"),
        "top-level Lean module should import retained leaf digest opening binding"
    );
    assert!(
        values_source.contains("open_batch_with_retained_leaf_digest_level_cuda")
            && values_source.contains("retained_leaf_digest_level")
            && values_source.contains("extended_row_values_from_source_cuda")
            && values_source.contains("path.root != expected_root"),
        "runtime retained leaf digest opening should bind retained paths to source-derived rows and expected roots"
    );
}
