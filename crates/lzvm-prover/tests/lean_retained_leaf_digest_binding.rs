use std::path::Path;

#[path = "support/lean_binding.rs"]
mod lean_binding;

#[test]
fn lean_retained_leaf_digest_binding_tracks_runtime_opening_contract() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_path = crate_root.join("../../lean/Lzvm/RetainedLeafDigestOpening.lean");
    let mut lean_source = std::fs::read_to_string(&lean_path)
        .expect("Lean retained leaf digest opening source should read");
    let contracts_path =
        crate_root.join("../../lean/Lzvm/RetainedLeafDigestOpening/Contracts.lean");
    let arity_path = crate_root.join("../../lean/Lzvm/RetainedLeafDigestOpening/Arity.lean");
    lean_source.push('\n');
    lean_source.push_str(
        &std::fs::read_to_string(&contracts_path)
            .expect("Lean retained leaf digest opening contracts source should read"),
    );
    lean_source.push('\n');
    lean_source.push_str(
        &std::fs::read_to_string(&arity_path)
            .expect("Lean retained leaf digest opening arity source should read"),
    );
    let top_level_path = crate_root.join("../../lean/Lzvm.lean");
    let top_level_source =
        std::fs::read_to_string(&top_level_path).expect("Lean top-level source should read");
    let values_path = crate_root.join("src/witness_commitment/values.rs");
    let values_source = std::fs::read_to_string(&values_path)
        .expect("witness commitment values source should read");
    let hot_paths_path = crate_root.join("tests/source_hot_paths.rs");
    let hot_paths_source =
        std::fs::read_to_string(&hot_paths_path).expect("source hot-path tests should read");

    assert!(
        lean_source.contains("RuntimeRetainedLeafDigestOpeningValidation")
            && lean_source.contains("retainedLeafDigestRowsFromSource")
            && lean_source.contains("retainedLeafDigestPathBound")
            && lean_source.contains("retainedLeafDigestRootMatchesExpectedRoot")
            && lean_source.contains("retainedLeafDigestRowsBoundToQueryPlan")
            && lean_source.contains("retainedLeafDigestShiftedRowWeightCacheUsed")
            && lean_source.contains(
                "retainedLeafDigestOpeningAcceptedImpliesShiftedRowWeightCacheUsed"
            )
            && lean_source.contains("retainedLeafDigestShiftedRowWeightCacheImpliesRowsFromSource")
            && lean_source.contains("retainedLeafDigestChecksImplyPerRowWitnessOpeningRowsBound")
            && lean_source.contains("RuntimeRetainedLeafDigestOpeningEvidence")
            && lean_source.contains("def RuntimeRetainedLeafDigestOpeningDigestContract")
            && lean_source.contains("def RuntimeRetainedLeafDigestOpeningRetainedRowsContract")
            && lean_source
                .contains("def RuntimeRetainedLeafDigestOpeningShiftedRowSourceContract")
            && lean_source.contains("RuntimeRetainedLeafDigestOpeningDigestContract")
            && lean_source.contains("RuntimeRetainedLeafDigestOpeningRetainedRowsContract")
            && lean_source.contains("RuntimeRetainedLeafDigestOpeningShiftedRowSourceContract")
            && lean_source.contains("RuntimeRetainedLeafDigestNAryConcretePathBinding")
            && lean_source.contains("RuntimeRetainedLeafDigestNAryConcreteOpeningBinding")
            && lean_source.contains("NAryMerklePathLayer")
            && lean_source.contains("NAryMerklePathOpening")
            && lean_source.contains("NAryMerklePathRootCommitsToLeafAtIndex")
            && lean_source.contains("RuntimeBatchWitnessOpeningRowsEvidence")
            && lean_source.contains("RuntimeBatchWitnessOpeningRowsBoundContract")
            && lean_source.contains("RuntimeVerifierCoreContract system publicInput proof")
            && lean_source.contains("SoundWitness system publicInput proof"),
        "Lean retained leaf digest opening binding should expose source rows, shifted-row cache evidence, Merkle path, root equality, and soundness evidence"
    );
    lean_binding::assert_theorem_declarations(
        &lean_source,
        &[
            "runtime_retained_leaf_digest_opening_checked_acceptance_evidence",
            "runtime_retained_leaf_digest_shifted_row_weight_cache_implies_source_rows",
            "runtime_retained_leaf_digest_opening_checked_acceptance_shifted_row_source_contract",
            "runtime_retained_leaf_digest_opening_evidence_implies_digest_contract",
            "runtime_retained_leaf_digest_opening_evidence_implies_batch_rows_evidence",
            "runtime_retained_leaf_digest_opening_checked_acceptance_batch_rows_evidence",
            "runtime_retained_leaf_digest_opening_evidence_implies_batch_rows_bound_contract",
            "runtime_retained_leaf_digest_opening_checked_acceptance_batch_rows_bound_contract",
            "runtime_retained_leaf_digest_opening_evidence_implies_opening_evidence",
            "runtime_retained_leaf_digest_opening_checked_acceptance_opening_evidence",
            "runtime_retained_leaf_digest_opening_checked_acceptance_batch_path_and_opening_evidence",
            "runtime_retained_leaf_digest_opening_evidence_implies_retained_rows_contract",
            "runtime_retained_leaf_digest_opening_checked_acceptance_retained_rows_contract",
            "runtime_retained_leaf_digest_opening_checked_acceptance_digest_contract",
            "runtime_retained_leaf_digest_concrete_path_digest_contract_from_bundle",
            "runtime_retained_leaf_digest_concrete_path_position_bound_from_no_collision",
            "runtime_retained_leaf_digest_concrete_path_position_bound_from_bundle",
            "runtime_retained_leaf_digest_concrete_path_opening_and_core_contract_from_bundle",
            "runtime_retained_leaf_digest_nary_path_position_bound_from_no_collision",
            "runtime_retained_leaf_digest_nary_path_position_bound_from_bundle",
            "runtime_retained_leaf_digest_nary_path_arity_four_position_bound_from_no_collision",
            "runtime_retained_leaf_digest_nary_path_arity_four_position_bound_from_bundle",
            "runtime_retained_leaf_digest_nary_path_digest_contract_from_bundle",
            "runtime_retained_leaf_digest_nary_opening_position_bound_from_no_collision",
            "runtime_retained_leaf_digest_nary_opening_position_bound_from_bundle",
            "runtime_retained_leaf_digest_nary_opening_arity_four_position_bound_from_no_collision",
            "runtime_retained_leaf_digest_nary_opening_arity_four_position_bound_from_bundle",
            "runtime_retained_leaf_digest_nary_opening_checked_acceptance_evidence_from_bundle",
            "runtime_retained_leaf_digest_nary_opening_digest_contract_from_bundle",
            "runtime_retained_leaf_digest_nary_opening_opening_and_core_contract_from_bundle",
            "runtime_retained_leaf_digest_nary_opening_source_and_core_contract_from_bundle",
            "runtime_retained_leaf_digest_nary_opening_source_core_sound_contract_from_bundle",
            "runtime_retained_leaf_digest_nary_path_opening_and_core_contract_from_bundle",
            "runtime_retained_leaf_digest_opening_checked_acceptance_sound",
            "runtime_retained_leaf_digest_opening_checked_acceptance_verifier_core_contract",
            "runtime_retained_leaf_digest_opening_checked_acceptance_opening_and_core_contract",
            "runtime_retained_leaf_digest_opening_checked_acceptance_source_and_core_contract",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_retained_leaf_digest_nary_path_position_bound_from_no_collision",
        &[
            "RuntimeRetainedLeafDigestNAryConcretePathBinding",
            "NAryMerkleCompressionNoCollision compress",
            "retainedLeafDigestPathBound",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_retained_leaf_digest_nary_path_position_bound_from_no_collision",
        &[
            "verified_concrete_nary_merkle_path_implies_root_commits_to_leaf_at_index_from_no_collision",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_retained_leaf_digest_nary_path_arity_four_position_bound_from_no_collision",
        &[
            "RuntimeRetainedLeafDigestNAryConcretePathBinding",
            "NAryMerkleCompressionNoCollision compress",
            "NAryMerklePathHasArity 4",
            "retainedLeafDigestPathBound",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_retained_leaf_digest_nary_path_arity_four_position_bound_from_no_collision",
        &[
            "verified_concrete_nary_merkle_path_arity_four_implies_root_commits_to_leaf_at_index_from_no_collision",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_retained_leaf_digest_nary_path_arity_four_position_bound_from_bundle",
        &[
            "AssumptionBundle system",
            "CentralizedNAryMerkleCompressionCollisionResistance",
            "RuntimeRetainedLeafDigestNAryConcretePathBinding",
            "NAryMerklePathHasArity 4",
            "retainedLeafDigestPathBound",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_retained_leaf_digest_nary_path_arity_four_position_bound_from_bundle",
        &["runtime_retained_leaf_digest_nary_path_arity_four_position_bound_from_no_collision"],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_retained_leaf_digest_nary_path_position_bound_from_no_collision",
        &[
            "verified_concrete_nary_merkle_path_implies_root_commits_to_leaf_at_position_from_no_collision",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_retained_leaf_digest_nary_opening_position_bound_from_no_collision",
        &[
            "RuntimeRetainedLeafDigestNAryConcreteOpeningBinding",
            "NAryMerkleCompressionNoCollision compress",
            "retainedLeafDigestPathBound",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_retained_leaf_digest_nary_opening_position_bound_from_no_collision",
        &[
            "verified_concrete_nary_merkle_opening_implies_root_commits_to_leaf_at_index_from_no_collision",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_retained_leaf_digest_nary_opening_arity_four_position_bound_from_no_collision",
        &[
            "RuntimeRetainedLeafDigestNAryConcreteOpeningBinding",
            "NAryMerkleCompressionNoCollision compress",
            "NAryMerklePathHasArity 4",
            "retainedLeafDigestPathBound",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_retained_leaf_digest_nary_opening_arity_four_position_bound_from_no_collision",
        &[
            "verified_concrete_nary_merkle_opening_arity_four_implies_root_commits_to_leaf_at_index_from_no_collision",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_retained_leaf_digest_nary_opening_arity_four_position_bound_from_bundle",
        &[
            "AssumptionBundle system",
            "CentralizedNAryMerkleCompressionCollisionResistance",
            "RuntimeRetainedLeafDigestNAryConcreteOpeningBinding",
            "NAryMerklePathHasArity 4",
            "retainedLeafDigestPathBound",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_retained_leaf_digest_nary_opening_arity_four_position_bound_from_bundle",
        &["runtime_retained_leaf_digest_nary_opening_arity_four_position_bound_from_no_collision"],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_retained_leaf_digest_nary_opening_position_bound_from_no_collision",
        &[
            "verified_concrete_nary_merkle_path_implies_root_commits_to_leaf_at_index_from_no_collision",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_retained_leaf_digest_nary_path_digest_contract_from_bundle",
        &[
            "AssumptionBundle system",
            "CentralizedNAryMerkleCompressionCollisionResistance",
            "RuntimeRetainedLeafDigestNAryConcretePathBinding",
            "RuntimeRetainedLeafDigestOpeningDigestContract",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_retained_leaf_digest_nary_path_digest_contract_from_bundle",
        &["runtime_retained_leaf_digest_nary_path_position_bound_from_bundle"],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_retained_leaf_digest_nary_path_digest_contract_from_bundle",
        &["retainedLeafDigestOpeningAcceptedImpliesPathBound"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_retained_leaf_digest_nary_opening_digest_contract_from_bundle",
        &[
            "AssumptionBundle system",
            "CentralizedNAryMerkleCompressionCollisionResistance",
            "RuntimeRetainedLeafDigestNAryConcreteOpeningBinding",
            "RuntimeRetainedLeafDigestOpeningDigestContract",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_retained_leaf_digest_nary_opening_digest_contract_from_bundle",
        &["runtime_retained_leaf_digest_nary_opening_position_bound_from_bundle"],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_retained_leaf_digest_nary_opening_digest_contract_from_bundle",
        &["retainedLeafDigestOpeningAcceptedImpliesPathBound"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_retained_leaf_digest_nary_opening_checked_acceptance_evidence_from_bundle",
        &[
            "AssumptionBundle system",
            "CentralizedNAryMerkleCompressionCollisionResistance",
            "RuntimeRetainedLeafDigestNAryConcreteOpeningBinding",
            "RuntimeRetainedLeafDigestOpeningEvidence",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_retained_leaf_digest_nary_opening_checked_acceptance_evidence_from_bundle",
        &["runtime_retained_leaf_digest_nary_opening_position_bound_from_bundle"],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_retained_leaf_digest_nary_opening_checked_acceptance_evidence_from_bundle",
        &["retainedLeafDigestOpeningAcceptedImpliesPathBound"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_retained_leaf_digest_nary_opening_opening_and_core_contract_from_bundle",
        &[
            "AssumptionBundle system",
            "CentralizedNAryMerkleCompressionCollisionResistance",
            "RuntimeRetainedLeafDigestNAryConcreteOpeningBinding",
            "RuntimeOpeningEvidence",
            "RuntimeRetainedLeafDigestOpeningDigestContract",
            "RuntimeRetainedLeafDigestOpeningRetainedRowsContract",
            "RuntimeVerifierCoreContract system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_retained_leaf_digest_nary_opening_opening_and_core_contract_from_bundle",
        &["runtime_retained_leaf_digest_nary_opening_digest_contract_from_bundle"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_retained_leaf_digest_nary_opening_source_and_core_contract_from_bundle",
        &[
            "AssumptionBundle system",
            "CentralizedNAryMerkleCompressionCollisionResistance",
            "RuntimeRetainedLeafDigestNAryConcreteOpeningBinding",
            "RuntimeRetainedLeafDigestOpeningDigestContract",
            "RuntimeRetainedLeafDigestOpeningShiftedRowSourceContract",
            "RuntimeRetainedLeafDigestOpeningRetainedRowsContract",
            "RuntimeVerifierCoreContract system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_retained_leaf_digest_nary_opening_source_and_core_contract_from_bundle",
        &[
            "runtime_retained_leaf_digest_nary_opening_checked_acceptance_evidence_from_bundle",
            "runtime_retained_leaf_digest_nary_opening_digest_contract_from_bundle",
            "runtime_retained_leaf_digest_opening_checked_acceptance_shifted_row_source_contract",
            "runtime_retained_leaf_digest_opening_checked_acceptance_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_retained_leaf_digest_nary_opening_source_and_core_contract_from_bundle",
        &["runtime_retained_leaf_digest_opening_checked_acceptance_retained_rows_contract"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_retained_leaf_digest_nary_opening_source_core_sound_contract_from_bundle",
        &[
            "AssumptionBundle system",
            "CentralizedNAryMerkleCompressionCollisionResistance",
            "RuntimeRetainedLeafDigestNAryConcreteOpeningBinding",
            "RuntimeRetainedLeafDigestOpeningDigestContract",
            "RuntimeRetainedLeafDigestOpeningShiftedRowSourceContract",
            "RuntimeRetainedLeafDigestOpeningRetainedRowsContract",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_retained_leaf_digest_nary_opening_source_core_sound_contract_from_bundle",
        &[
            "runtime_retained_leaf_digest_nary_opening_source_and_core_contract_from_bundle",
            "runtime_retained_leaf_digest_opening_checked_acceptance_sound",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_retained_leaf_digest_nary_opening_source_core_sound_contract_from_bundle",
        &["retainedLeafDigestOpeningAcceptedImpliesPathBound"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_retained_leaf_digest_nary_path_opening_and_core_contract_from_bundle",
        &[
            "AssumptionBundle system",
            "CentralizedNAryMerkleCompressionCollisionResistance",
            "RuntimeRetainedLeafDigestNAryConcretePathBinding",
            "RuntimeOpeningEvidence",
            "RuntimeRetainedLeafDigestOpeningDigestContract",
            "RuntimeVerifierCoreContract system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_retained_leaf_digest_nary_path_opening_and_core_contract_from_bundle",
        &["runtime_retained_leaf_digest_nary_path_digest_contract_from_bundle"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_retained_leaf_digest_concrete_path_position_bound_from_no_collision",
        &[
            "RuntimeRetainedLeafDigestConcretePathBinding",
            "MerkleCompressionNoCollision compress",
            "retainedLeafDigestPathBound",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_retained_leaf_digest_concrete_path_digest_contract_from_bundle",
        &[
            "AssumptionBundle system",
            "CentralizedMerkleCompressionCollisionResistance",
            "RuntimeRetainedLeafDigestConcretePathBinding",
            "RuntimeRetainedLeafDigestOpeningDigestContract",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_retained_leaf_digest_concrete_path_opening_and_core_contract_from_bundle",
        &[
            "AssumptionBundle system",
            "CentralizedMerkleCompressionCollisionResistance",
            "RuntimeRetainedLeafDigestConcretePathBinding",
            "RuntimeOpeningEvidence",
            "RuntimeRetainedLeafDigestOpeningDigestContract",
            "RuntimeRetainedLeafDigestOpeningRetainedRowsContract",
            "RuntimeVerifierCoreContract system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_retained_leaf_digest_concrete_path_opening_and_core_contract_from_bundle",
        &["retainedLeafDigestOpeningAcceptedImpliesPathBound"],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_retained_leaf_digest_concrete_path_digest_contract_from_bundle",
        &["retainedLeafDigestOpeningAcceptedImpliesPathBound"],
    );
    assert!(
        top_level_source.contains("import Lzvm.RetainedLeafDigestOpening")
            && top_level_source.contains("import Lzvm.RetainedLeafDigestOpening.Arity")
            && top_level_source.contains("import Lzvm.RetainedLeafDigestOpening.Contracts"),
        "top-level Lean module should import retained leaf digest opening binding"
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_retained_leaf_digest_opening_checked_acceptance_batch_path_and_opening_evidence",
        &[
            "runtime_retained_leaf_digest_opening_checked_acceptance_batch_rows_evidence",
            "retainedLeafDigestOpeningAcceptedImpliesPathBound",
            "retainedLeafDigestOpeningAcceptedImpliesRootMatchesExpectedRoot",
            "runtime_retained_leaf_digest_opening_checked_acceptance_opening_evidence",
        ],
    );
    let retained_leaf_fast_path = function_body(
        &values_source,
        "fn open_batch_with_retained_leaf_digest_level_cuda",
        "fn copy_extended_row_values_batch_from_device",
    );
    assert!(
        retained_leaf_fast_path.contains("retained_leaf_digest_level")
            && retained_leaf_fast_path.contains("extended_row_values_batch_from_source_cuda")
            && values_source
                .contains("cuda_goldilocks_coset_extend_row_major_columns_shifted_row_device")
            && values_source.contains(
                "cuda_goldilocks_coset_extend_row_major_columns_strided_shifted_row_device"
            )
            && retained_leaf_fast_path.contains("opening_path_siblings_batch(rows)")
            && !retained_leaf_fast_path.contains("opening_path_siblings(*row)")
            && !values_source.contains("path.root != expected_root")
            && hot_paths_source
                .contains("cuda_compact_opening_avoids_redundant_path_root_downloads")
            && hot_paths_source.contains("retained_leaf_digest_opening_uses_shifted_row_weight_cache")
            && hot_paths_source.contains("extended_row_values_batch_from_source_cuda")
            && hot_paths_source.contains("residue weight cache"),
        "runtime retained leaf digest opening should bind retained paths to source-derived rows, shifted-row cache use, and host-known expected roots"
    );
}

fn function_body<'a>(source: &'a str, start: &str, end: &str) -> &'a str {
    let start_index = source
        .find(start)
        .unwrap_or_else(|| panic!("{start} should appear in source"));
    let rest = &source[start_index..];
    let end_index = rest
        .find(end)
        .unwrap_or_else(|| panic!("{end} should appear after {start}"));
    &rest[..end_index]
}
