use std::path::Path;

#[path = "support/lean_binding.rs"]
mod lean_binding;

#[test]
fn lean_retained_parent_checkpoint_binding_tracks_runtime_opening_contract() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_path = crate_root.join("../../lean/Lzvm/RetainedParentCheckpointOpening.lean");
    let opening_source = std::fs::read_to_string(&lean_path)
        .expect("Lean retained parent checkpoint opening source should read");
    let core_path = crate_root.join("../../lean/Lzvm/RetainedParentCheckpointOpening/Core.lean");
    let core_source = std::fs::read_to_string(&core_path)
        .expect("Lean retained parent checkpoint opening core source should read");
    let contracts_path =
        crate_root.join("../../lean/Lzvm/RetainedParentCheckpointOpening/Contracts.lean");
    let contracts_source = std::fs::read_to_string(&contracts_path)
        .expect("Lean retained parent checkpoint opening contracts source should read");
    let arity_path = crate_root.join("../../lean/Lzvm/RetainedParentCheckpointOpening/Arity.lean");
    let arity_source = std::fs::read_to_string(&arity_path)
        .expect("Lean retained parent checkpoint opening arity source should read");
    let lean_source = format!("{core_source}\n{contracts_source}\n{arity_source}");
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
            && lean_source.contains("RuntimeRetainedParentCheckpointNAryConcretePathBinding")
            && lean_source.contains("RuntimeRetainedParentCheckpointNAryConcreteOpeningBinding")
            && lean_source.contains("NAryMerklePathLayer")
            && lean_source.contains("NAryMerklePathOpening")
            && lean_source.contains("NAryMerklePathRootCommitsToLeafAtIndex")
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
            "runtime_retained_parent_checkpoint_opening_checked_acceptance_batch_path_and_opening_evidence",
            "runtime_retained_parent_checkpoint_opening_evidence_implies_retained_rows_contract",
            "runtime_retained_parent_checkpoint_opening_checked_acceptance_retained_rows_contract",
            "runtime_retained_parent_checkpoint_opening_checked_acceptance_digest_contract",
            "runtime_retained_parent_checkpoint_concrete_path_bound_from_no_collision",
            "runtime_retained_parent_checkpoint_concrete_path_digest_contract_from_bundle",
            "runtime_retained_parent_checkpoint_concrete_path_position_bound_from_no_collision",
            "runtime_retained_parent_checkpoint_concrete_path_position_bound_from_bundle",
            "runtime_retained_parent_checkpoint_concrete_path_opening_and_core_contract_from_bundle",
            "runtime_retained_parent_checkpoint_nary_path_position_bound_from_no_collision",
            "runtime_retained_parent_checkpoint_nary_path_position_bound_from_bundle",
            "runtime_retained_parent_checkpoint_nary_path_arity_four_position_bound_from_no_collision",
            "runtime_retained_parent_checkpoint_nary_path_arity_four_position_bound_from_bundle",
            "runtime_retained_parent_checkpoint_nary_path_digest_contract_from_bundle",
            "runtime_retained_parent_checkpoint_nary_path_opening_and_core_contract_from_bundle",
            "runtime_retained_parent_checkpoint_nary_opening_position_bound_from_no_collision",
            "runtime_retained_parent_checkpoint_nary_opening_position_bound_from_bundle",
            "runtime_retained_parent_checkpoint_nary_opening_position_bound_from_hash_assumption",
            "runtime_retained_parent_checkpoint_nary_opening_arity_four_position_bound_from_no_collision",
            "runtime_retained_parent_checkpoint_nary_opening_arity_four_position_bound_from_bundle",
            "runtime_retained_parent_checkpoint_nary_opening_digest_contract_from_bundle",
            "runtime_retained_parent_checkpoint_nary_opening_digest_contract_from_hash_assumption",
            "runtime_retained_parent_checkpoint_nary_opening_opening_and_core_contract_from_bundle",
            "runtime_retained_parent_checkpoint_nary_opening_source_and_core_contract_from_bundle",
            "runtime_retained_parent_checkpoint_nary_opening_source_core_sound_contract_from_bundle",
            "runtime_retained_parent_checkpoint_nary_opening_source_core_sound_contract_from_concrete_opening_bundle",
            "runtime_retained_parent_checkpoint_nary_opening_source_core_sound_contract_from_hash_concrete_opening",
            "runtime_retained_parent_checkpoint_prefix_batch_implies_lower_prefix_bound",
            "runtime_retained_parent_checkpoint_opening_checked_acceptance_prefix_batch_contract",
            "runtime_retained_parent_checkpoint_opening_checked_acceptance_source_contract",
            "runtime_retained_parent_checkpoint_opening_checked_acceptance_sound",
            "runtime_retained_parent_checkpoint_opening_checked_acceptance_sound_from_hash_concrete_opening",
            "runtime_retained_parent_checkpoint_opening_checked_acceptance_sound_from_concrete_nary_merkle",
            "runtime_retained_parent_checkpoint_opening_checked_acceptance_verifier_core_contract",
            "runtime_retained_parent_checkpoint_opening_checked_acceptance_opening_and_core_contract",
            "runtime_retained_parent_checkpoint_opening_checked_acceptance_evidence_core_and_sound",
            "runtime_retained_parent_checkpoint_opening_checked_acceptance_audited_core_contract",
            "runtime_retained_parent_checkpoint_opening_checked_acceptance_source_and_core_contract",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_retained_parent_checkpoint_nary_path_position_bound_from_no_collision",
        &[
            "RuntimeRetainedParentCheckpointNAryConcretePathBinding",
            "NAryMerkleCompressionNoCollision compress",
            "retainedParentCheckpointStitchedPathBound",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_retained_parent_checkpoint_nary_path_position_bound_from_no_collision",
        &[
            "verified_concrete_nary_merkle_path_implies_root_commits_to_leaf_at_index_from_no_collision",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_retained_parent_checkpoint_nary_path_arity_four_position_bound_from_no_collision",
        &[
            "RuntimeRetainedParentCheckpointNAryConcretePathBinding",
            "NAryMerkleCompressionNoCollision compress",
            "NAryMerklePathHasArity 4",
            "retainedParentCheckpointStitchedPathBound",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_retained_parent_checkpoint_nary_path_arity_four_position_bound_from_no_collision",
        &[
            "verified_concrete_nary_merkle_path_arity_four_implies_root_commits_to_leaf_at_index_from_no_collision",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_retained_parent_checkpoint_nary_path_arity_four_position_bound_from_bundle",
        &[
            "AssumptionBundle system",
            "CentralizedNAryMerkleCompressionCollisionResistance",
            "RuntimeRetainedParentCheckpointNAryConcretePathBinding",
            "NAryMerklePathHasArity 4",
            "retainedParentCheckpointStitchedPathBound",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_retained_parent_checkpoint_nary_path_arity_four_position_bound_from_bundle",
        &[
            "runtime_retained_parent_checkpoint_nary_path_arity_four_position_bound_from_no_collision",
        ],
    );
    for theorem_name in [
        "runtime_retained_parent_checkpoint_nary_path_position_bound_from_bundle",
        "runtime_retained_parent_checkpoint_nary_opening_position_bound_from_bundle",
        "runtime_retained_parent_checkpoint_nary_path_arity_four_position_bound_from_bundle",
        "runtime_retained_parent_checkpoint_nary_opening_arity_four_position_bound_from_bundle",
    ] {
        lean_binding::assert_theorem_body_contains(
            &lean_source,
            theorem_name,
            &["assumption_bundle_nary_merkle_compression_no_collision"],
        );
        lean_binding::assert_theorem_body_omits(
            &lean_source,
            theorem_name,
            &["hashCollisionResistance.merkleHashCollisionResistance.evidence"],
        );
    }
    for theorem_name in [
        "runtime_retained_parent_checkpoint_concrete_path_bound_from_bundle",
        "runtime_retained_parent_checkpoint_concrete_path_position_bound_from_bundle",
    ] {
        lean_binding::assert_theorem_body_contains(
            &lean_source,
            theorem_name,
            &["assumption_bundle_merkle_compression_no_collision"],
        );
        lean_binding::assert_theorem_body_omits(
            &lean_source,
            theorem_name,
            &["hashCollisionResistance.merkleHashCollisionResistance.evidence"],
        );
    }
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_retained_parent_checkpoint_nary_path_position_bound_from_no_collision",
        &[
            "verified_concrete_nary_merkle_path_implies_root_commits_to_leaf_at_position_from_no_collision",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_retained_parent_checkpoint_nary_opening_position_bound_from_no_collision",
        &[
            "RuntimeRetainedParentCheckpointNAryConcreteOpeningBinding",
            "NAryMerkleCompressionNoCollision compress",
            "retainedParentCheckpointStitchedPathBound",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_retained_parent_checkpoint_nary_opening_position_bound_from_no_collision",
        &[
            "verified_concrete_nary_merkle_opening_implies_root_commits_to_leaf_at_index_from_no_collision",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_retained_parent_checkpoint_nary_opening_arity_four_position_bound_from_no_collision",
        &[
            "RuntimeRetainedParentCheckpointNAryConcreteOpeningBinding",
            "NAryMerkleCompressionNoCollision compress",
            "NAryMerklePathHasArity 4",
            "retainedParentCheckpointStitchedPathBound",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_retained_parent_checkpoint_nary_opening_arity_four_position_bound_from_no_collision",
        &[
            "verified_concrete_nary_merkle_opening_arity_four_implies_root_commits_to_leaf_at_index_from_no_collision",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_retained_parent_checkpoint_nary_opening_arity_four_position_bound_from_bundle",
        &[
            "AssumptionBundle system",
            "CentralizedNAryMerkleCompressionCollisionResistance",
            "RuntimeRetainedParentCheckpointNAryConcreteOpeningBinding",
            "NAryMerklePathHasArity 4",
            "retainedParentCheckpointStitchedPathBound",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_retained_parent_checkpoint_nary_opening_arity_four_position_bound_from_bundle",
        &[
            "runtime_retained_parent_checkpoint_nary_opening_arity_four_position_bound_from_no_collision",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_retained_parent_checkpoint_nary_opening_position_bound_from_no_collision",
        &[
            "verified_concrete_nary_merkle_path_implies_root_commits_to_leaf_at_index_from_no_collision",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_retained_parent_checkpoint_nary_opening_position_bound_from_hash_assumption",
        &[
            "HashCollisionResistanceAssumption",
            "CentralizedNAryMerkleCompressionCollisionResistance",
            "RuntimeRetainedParentCheckpointNAryConcreteOpeningBinding",
            "retainedParentCheckpointStitchedPathBound",
        ],
    );
    lean_binding::assert_theorem_prefix_omits(
        &lean_source,
        "runtime_retained_parent_checkpoint_nary_opening_position_bound_from_hash_assumption",
        &["AssumptionBundle"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_retained_parent_checkpoint_nary_opening_position_bound_from_hash_assumption",
        &["runtime_retained_parent_checkpoint_nary_opening_position_bound_from_no_collision"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_retained_parent_checkpoint_nary_path_digest_contract_from_bundle",
        &[
            "AssumptionBundle system",
            "CentralizedNAryMerkleCompressionCollisionResistance",
            "RuntimeRetainedParentCheckpointNAryConcretePathBinding",
            "RuntimeRetainedParentCheckpointOpeningDigestContract",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_retained_parent_checkpoint_nary_path_digest_contract_from_bundle",
        &["runtime_retained_parent_checkpoint_nary_path_position_bound_from_bundle"],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_retained_parent_checkpoint_nary_path_digest_contract_from_bundle",
        &["retainedParentCheckpointOpeningAcceptedImpliesStitchedPathBound"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_retained_parent_checkpoint_nary_opening_digest_contract_from_bundle",
        &[
            "AssumptionBundle system",
            "CentralizedNAryMerkleCompressionCollisionResistance",
            "RuntimeRetainedParentCheckpointNAryConcreteOpeningBinding",
            "RuntimeRetainedParentCheckpointOpeningDigestContract",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_retained_parent_checkpoint_nary_opening_digest_contract_from_bundle",
        &["runtime_retained_parent_checkpoint_nary_opening_position_bound_from_bundle"],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_retained_parent_checkpoint_nary_opening_digest_contract_from_bundle",
        &["retainedParentCheckpointOpeningAcceptedImpliesStitchedPathBound"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_retained_parent_checkpoint_nary_opening_digest_contract_from_hash_assumption",
        &[
            "HashCollisionResistanceAssumption",
            "CentralizedNAryMerkleCompressionCollisionResistance",
            "RuntimeRetainedParentCheckpointNAryConcreteOpeningBinding",
            "RuntimeRetainedParentCheckpointOpeningDigestContract",
        ],
    );
    lean_binding::assert_theorem_prefix_omits(
        &lean_source,
        "runtime_retained_parent_checkpoint_nary_opening_digest_contract_from_hash_assumption",
        &["AssumptionBundle"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_retained_parent_checkpoint_nary_opening_digest_contract_from_hash_assumption",
        &["runtime_retained_parent_checkpoint_nary_opening_position_bound_from_hash_assumption"],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_retained_parent_checkpoint_nary_opening_digest_contract_from_hash_assumption",
        &["retainedParentCheckpointOpeningAcceptedImpliesStitchedPathBound"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_retained_parent_checkpoint_nary_opening_opening_and_core_contract_from_bundle",
        &[
            "AssumptionBundle system",
            "CentralizedNAryMerkleCompressionCollisionResistance",
            "RuntimeRetainedParentCheckpointNAryConcreteOpeningBinding",
            "RuntimeOpeningEvidence",
            "RuntimeRetainedParentCheckpointOpeningDigestContract",
            "RuntimeRetainedParentCheckpointOpeningPrefixBatchContract",
            "RuntimeRetainedParentCheckpointOpeningRetainedRowsContract",
            "RuntimeVerifierCoreContract system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_retained_parent_checkpoint_nary_opening_opening_and_core_contract_from_bundle",
        &["runtime_retained_parent_checkpoint_nary_opening_digest_contract_from_bundle"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_retained_parent_checkpoint_nary_opening_source_and_core_contract_from_bundle",
        &[
            "AssumptionBundle system",
            "CentralizedNAryMerkleCompressionCollisionResistance",
            "RuntimeRetainedParentCheckpointNAryConcreteOpeningBinding",
            "RuntimeRetainedParentCheckpointOpeningDigestContract",
            "RuntimeRetainedParentCheckpointOpeningSourceContract",
            "RuntimeRetainedParentCheckpointOpeningRetainedRowsContract",
            "RuntimeVerifierCoreContract system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_retained_parent_checkpoint_nary_opening_source_and_core_contract_from_bundle",
        &[
            "runtime_retained_parent_checkpoint_nary_opening_digest_contract_from_bundle",
            "runtime_retained_parent_checkpoint_opening_checked_acceptance_source_contract",
            "runtime_retained_parent_checkpoint_opening_checked_acceptance_retained_rows_contract",
            "runtime_retained_parent_checkpoint_opening_checked_acceptance_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_retained_parent_checkpoint_opening_checked_acceptance_verifier_core_contract",
        &[
            "retainedParentCheckpointOpeningAcceptedImpliesBatchRowsAccepted",
            "runtime_batch_witness_opening_rows_checked_acceptance_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_retained_parent_checkpoint_opening_checked_acceptance_verifier_core_contract",
        &[
            "runtime_retained_parent_checkpoint_opening_checked_acceptance_sound",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_retained_parent_checkpoint_opening_checked_acceptance_evidence_core_and_sound",
        &[
            "RuntimeRetainedParentCheckpointOpeningEvidence",
            "RuntimeOpeningEvidence",
            "RuntimeRetainedParentCheckpointOpeningDigestContract",
            "RuntimeRetainedParentCheckpointOpeningPrefixBatchContract",
            "RuntimeRetainedParentCheckpointOpeningRetainedRowsContract",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_retained_parent_checkpoint_opening_checked_acceptance_evidence_core_and_sound",
        &[
            "runtime_retained_parent_checkpoint_opening_checked_acceptance_sound",
            "runtime_retained_parent_checkpoint_opening_checked_acceptance_opening_and_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_retained_parent_checkpoint_opening_checked_acceptance_evidence_core_and_sound",
        &["sound_witness_implies_verifier_core_contract"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_retained_parent_checkpoint_opening_checked_acceptance_audited_core_contract",
        &[
            "RequiredCryptographicAssumptionStatements assumptions.crypto",
            "RequiredSemanticAssumptionStatements assumptions.semantic",
            "RuntimeRetainedParentCheckpointOpeningEvidence",
            "RuntimeOpeningEvidence",
            "RuntimeRetainedParentCheckpointOpeningDigestContract",
            "RuntimeRetainedParentCheckpointOpeningPrefixBatchContract",
            "RuntimeRetainedParentCheckpointOpeningRetainedRowsContract",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_retained_parent_checkpoint_opening_checked_acceptance_audited_core_contract",
        &[
            "assumption_bundle_carries_required_crypto_evidence",
            "assumption_bundle_carries_required_semantic_evidence",
            "runtime_retained_parent_checkpoint_opening_checked_acceptance_sound",
            "runtime_retained_parent_checkpoint_opening_checked_acceptance_opening_and_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_retained_parent_checkpoint_opening_checked_acceptance_audited_core_contract",
        &[
            "assumption_bundle_carries_required_evidence",
            "runtime_retained_parent_checkpoint_opening_checked_acceptance_evidence_core_and_sound",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_retained_parent_checkpoint_nary_opening_source_core_sound_contract_from_bundle",
        &[
            "AssumptionBundle system",
            "CentralizedNAryMerkleCompressionCollisionResistance",
            "RuntimeRetainedParentCheckpointNAryConcreteOpeningBinding",
            "RuntimeRetainedParentCheckpointOpeningDigestContract",
            "RuntimeRetainedParentCheckpointOpeningSourceContract",
            "RuntimeRetainedParentCheckpointOpeningRetainedRowsContract",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_retained_parent_checkpoint_nary_opening_source_core_sound_contract_from_bundle",
        &[
            "runtime_retained_parent_checkpoint_nary_opening_source_and_core_contract_from_bundle",
            "runtime_retained_parent_checkpoint_opening_checked_acceptance_sound",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_retained_parent_checkpoint_nary_opening_source_core_sound_contract_from_bundle",
        &["retainedParentCheckpointOpeningAcceptedImpliesStitchedPathBound"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_retained_parent_checkpoint_nary_opening_source_core_sound_contract_from_concrete_opening_bundle",
        &[
            "AssumptionBundle system",
            "CentralizedNAryMerkleCompressionCollisionResistance",
            "RuntimeRetainedParentCheckpointNAryConcreteOpeningBinding",
            "RuntimeConstantOpeningNAryConcreteBinding",
            "RuntimeWitnessOpeningNAryConcreteBinding",
            "RuntimeRetainedParentCheckpointOpeningDigestContract",
            "RuntimeRetainedParentCheckpointOpeningSourceContract",
            "RuntimeRetainedParentCheckpointOpeningRetainedRowsContract",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_retained_parent_checkpoint_nary_opening_source_core_sound_contract_from_concrete_opening_bundle",
        &[
            "runtime_retained_parent_checkpoint_nary_opening_source_and_core_contract_from_bundle",
            "runtime_retained_parent_checkpoint_opening_checked_acceptance_sound_from_concrete_nary_merkle",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_retained_parent_checkpoint_nary_opening_source_core_sound_contract_from_concrete_opening_bundle",
        &["runtime_retained_parent_checkpoint_opening_checked_acceptance_sound\n"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_retained_parent_checkpoint_nary_opening_source_core_sound_contract_from_hash_concrete_opening",
        &[
            "AssumptionBundle system",
            "HashCollisionResistanceAssumption",
            "CentralizedNAryMerkleCompressionCollisionResistance",
            "RuntimeRetainedParentCheckpointNAryConcreteOpeningBinding",
            "RuntimeConstantOpeningNAryConcreteBinding",
            "RuntimeWitnessOpeningNAryConcreteBinding",
            "RuntimeRetainedParentCheckpointOpeningDigestContract",
            "RuntimeRetainedParentCheckpointOpeningSourceContract",
            "RuntimeRetainedParentCheckpointOpeningRetainedRowsContract",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_retained_parent_checkpoint_nary_opening_source_core_sound_contract_from_hash_concrete_opening",
        &[
            "runtime_retained_parent_checkpoint_nary_opening_digest_contract_from_hash_assumption",
            "runtime_retained_parent_checkpoint_opening_checked_acceptance_source_contract",
            "runtime_retained_parent_checkpoint_opening_checked_acceptance_retained_rows_contract",
            "runtime_retained_parent_checkpoint_opening_checked_acceptance_verifier_core_contract",
            "runtime_retained_parent_checkpoint_opening_checked_acceptance_sound_from_hash_concrete_opening",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_retained_parent_checkpoint_nary_opening_source_core_sound_contract_from_hash_concrete_opening",
        &[
            "runtime_retained_parent_checkpoint_nary_opening_source_and_core_contract_from_bundle",
            "runtime_retained_parent_checkpoint_opening_checked_acceptance_sound_from_concrete_nary_merkle",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_retained_parent_checkpoint_opening_checked_acceptance_sound_from_hash_concrete_opening",
        &[
            "HashCollisionResistanceAssumption",
            "RuntimeConstantOpeningNAryConcreteBinding",
            "RuntimeWitnessOpeningNAryConcreteBinding",
            "RuntimeRetainedParentCheckpointOpeningEvidence",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_retained_parent_checkpoint_opening_checked_acceptance_sound_from_hash_concrete_opening",
        &["runtime_batch_witness_opening_rows_checked_acceptance_sound_from_hash_concrete_opening"],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_retained_parent_checkpoint_opening_checked_acceptance_sound_from_hash_concrete_opening",
        &["runtime_batch_witness_opening_rows_checked_acceptance_sound\n"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_retained_parent_checkpoint_opening_checked_acceptance_sound_from_concrete_nary_merkle",
        &[
            "runtime_retained_parent_checkpoint_opening_checked_acceptance_sound_from_hash_concrete_opening",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_retained_parent_checkpoint_concrete_path_bound_from_no_collision",
        &[
            "RuntimeRetainedParentCheckpointConcretePathBinding",
            "MerkleCompressionNoCollision compress",
            "retainedParentCheckpointStitchedPathBound",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_retained_parent_checkpoint_concrete_path_bound_from_no_collision",
        &[
            "binding.concreteStitchedPathVerifies",
            "verified_concrete_merkle_path_implies_root_commits_to_leaf_at_index_from_no_collision",
            "binding.stitchedPathRootCommitsToLeafImpliesStitchedPathBound",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_retained_parent_checkpoint_concrete_path_position_bound_from_no_collision",
        &[
            "RuntimeRetainedParentCheckpointConcretePathBinding",
            "MerkleCompressionNoCollision compress",
            "retainedParentCheckpointStitchedPathBound",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_retained_parent_checkpoint_concrete_path_digest_contract_from_bundle",
        &[
            "AssumptionBundle system",
            "CentralizedMerkleCompressionCollisionResistance",
            "RuntimeRetainedParentCheckpointConcretePathBinding",
            "RuntimeRetainedParentCheckpointOpeningDigestContract",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_retained_parent_checkpoint_concrete_path_opening_and_core_contract_from_bundle",
        &[
            "AssumptionBundle system",
            "CentralizedMerkleCompressionCollisionResistance",
            "RuntimeRetainedParentCheckpointConcretePathBinding",
            "RuntimeOpeningEvidence",
            "RuntimeRetainedParentCheckpointOpeningDigestContract",
            "RuntimeRetainedParentCheckpointOpeningPrefixBatchContract",
            "RuntimeRetainedParentCheckpointOpeningRetainedRowsContract",
            "RuntimeVerifierCoreContract system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_retained_parent_checkpoint_concrete_path_opening_and_core_contract_from_bundle",
        &["runtime_retained_parent_checkpoint_concrete_path_digest_contract_from_bundle"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_retained_parent_checkpoint_nary_path_opening_and_core_contract_from_bundle",
        &[
            "AssumptionBundle system",
            "CentralizedNAryMerkleCompressionCollisionResistance",
            "RuntimeRetainedParentCheckpointNAryConcretePathBinding",
            "RuntimeOpeningEvidence",
            "RuntimeRetainedParentCheckpointOpeningDigestContract",
            "RuntimeRetainedParentCheckpointOpeningPrefixBatchContract",
            "RuntimeRetainedParentCheckpointOpeningRetainedRowsContract",
            "RuntimeVerifierCoreContract system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_retained_parent_checkpoint_nary_path_opening_and_core_contract_from_bundle",
        &["runtime_retained_parent_checkpoint_nary_path_digest_contract_from_bundle"],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_retained_parent_checkpoint_concrete_path_digest_contract_from_bundle",
        &["retainedParentCheckpointOpeningAcceptedImpliesStitchedPathBound"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_retained_parent_checkpoint_opening_checked_acceptance_batch_path_and_opening_evidence",
        &[
            "RuntimeBatchWitnessOpeningRowsEvidence",
            "RuntimeRetainedParentCheckpointOpeningPrefixBatchContract",
            "RuntimeRetainedParentCheckpointOpeningDigestContract",
            "RuntimeOpeningEvidence",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_retained_parent_checkpoint_opening_checked_acceptance_batch_path_and_opening_evidence",
        &[
            "runtime_retained_parent_checkpoint_opening_checked_acceptance_batch_rows_evidence",
            "runtime_retained_parent_checkpoint_opening_checked_acceptance_prefix_batch_contract",
            "runtime_retained_parent_checkpoint_opening_checked_acceptance_digest_contract",
            "runtime_retained_parent_checkpoint_opening_checked_acceptance_opening_evidence",
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
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_retained_parent_checkpoint_opening_checked_acceptance_source_and_core_contract",
        &[".right.right.right"],
    );
    assert!(
        lean_binding::contains_import(&top_level_source, "Lzvm.RetainedParentCheckpointOpening"),
        "top-level Lean module should import retained parent checkpoint opening binding"
    );
    assert!(
        lean_binding::contains_import(
            &opening_source,
            "Lzvm.RetainedParentCheckpointOpening.Contracts"
        ),
        "retained parent checkpoint opening module should aggregate the contracts module"
    );
    assert!(
        lean_binding::contains_import(
            &opening_source,
            "Lzvm.RetainedParentCheckpointOpening.Arity"
        ) && lean_binding::contains_import(
            &top_level_source,
            "Lzvm.RetainedParentCheckpointOpening.Arity"
        ),
        "top-level Lean modules should import retained parent checkpoint arity binding"
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
