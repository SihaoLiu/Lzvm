use std::path::Path;

#[path = "support/lean_binding.rs"]
mod lean_binding;

#[test]
fn lean_opening_validation_binding_exports_core_contract_projection() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_path = crate_root.join("../../lean/Lzvm/OpeningValidation.lean");
    let lean_source =
        std::fs::read_to_string(&lean_path).expect("Lean opening validation should read");
    let top_level_path = crate_root.join("../../lean/Lzvm.lean");
    let top_level_source =
        std::fs::read_to_string(&top_level_path).expect("Lean top-level source should read");
    let witness_tree_path = crate_root.join("src/witness_commitment/tree.rs");
    let witness_tree_source =
        std::fs::read_to_string(&witness_tree_path).expect("witness tree source should read");

    assert!(
        top_level_source.contains("import Lzvm.OpeningValidation"),
        "top-level Lean module should import opening validation"
    );
    assert!(
        lean_source.contains("import Lzvm.MerklePathSoundness")
            && witness_tree_source.contains("validate_witness_commitment_arity(arity)?")
            && witness_tree_source.contains("matches!(arity, 2 | 4)")
            && witness_tree_source.contains("row_index % arity_u64")
            && witness_tree_source.contains("parent_hash(&children, arity)?"),
        "runtime witness opening root checks should be represented by concrete arity-2/4 Merkle path folding in Lean"
    );
    assert!(
        lean_source.contains("RuntimeOpeningValidation")
            && lean_source.contains("RuntimeOpeningBoundContract")
            && lean_source.contains("requiresExternalSource ->")
            && lean_source.contains("RuntimeVerifierCoreContract system publicInput proof")
            && lean_source.contains("SoundWitness system publicInput proof"),
        "Lean opening validation should expose compact opening-bound, checked, and required-source verifier core projections"
    );
    lean_binding::assert_theorem_declarations(
        &lean_source,
        &[
            "runtime_opening_evidence_implies_bound_contract",
            "runtime_opening_checked_acceptance_bound_contract",
            "runtime_opening_evidence_implies_external_source_requirement",
            "runtime_opening_checked_acceptance_pcs_and_fri_without_assumptions",
            "runtime_opening_checked_acceptance_bound_pcs_fri_contract",
            "runtime_opening_checked_acceptance_pcs_and_fri",
            "runtime_opening_checked_acceptance_sound",
            "runtime_opening_checked_acceptance_verifier_core_contract",
            "runtime_opening_required_external_source_sound",
            "runtime_opening_required_external_source_verifier_core_contract",
            "runtime_witness_opening_arity_two_same_index_leaf_binding_from_bundle",
            "runtime_witness_opening_arity_four_same_index_leaf_binding_from_bundle",
            "runtime_witness_opening_arity_two_root_commits_to_leaf_at_index_from_bundle",
            "runtime_witness_opening_arity_four_root_commits_to_leaf_at_index_from_bundle",
            "runtime_witness_opening_nary_checked_acceptance_witness_bound_from_bundle",
        ],
    );
    assert!(
        lean_source.contains("structure RuntimeWitnessOpeningNAryConcreteBinding"),
        "Lean opening validation should model concrete runtime witness openings separately from abstract witnessOpeningsBound fields"
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_witness_opening_nary_checked_acceptance_witness_bound_from_bundle",
        &[
            "AssumptionBundle system",
            "RuntimeWitnessOpeningNAryConcreteBinding",
            "CentralizedNAryMerkleCompressionCollisionResistance",
            "RuntimeOpeningCheckedAcceptance system validation artifact publicInput proof",
            "validation.witnessOpeningsBound artifact publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_witness_opening_nary_checked_acceptance_witness_bound_from_bundle",
        &[
            "binding.concreteOpeningVerifies",
            "verified_concrete_nary_merkle_opening_implies_root_commits_to_leaf_at_index_from_bundle",
            "binding.witnessRootCommitsToLeafImpliesWitnessOpeningsBound",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_witness_opening_arity_two_same_index_leaf_binding_from_bundle",
        &[
            "AssumptionBundle system",
            "CentralizedNAryMerkleCompressionCollisionResistance",
            "NAryMerklePathHasArity 2 opening.layers",
            "NAryMerklePathOpeningVerifies compress root opening",
            "NAryMerklePathIndex opening.layers = NAryMerklePathIndex otherOpening.layers",
            "opening.layers.length = otherOpening.layers.length",
            "otherOpening.leaf = opening.leaf",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_witness_opening_arity_two_same_index_leaf_binding_from_bundle",
        &["verified_concrete_nary_merkle_opening_arity_two_same_index_leaf_eq_from_bundle"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_witness_opening_arity_four_same_index_leaf_binding_from_bundle",
        &[
            "AssumptionBundle system",
            "CentralizedNAryMerkleCompressionCollisionResistance",
            "NAryMerklePathHasArity 4 opening.layers",
            "NAryMerklePathOpeningVerifies compress root opening",
            "NAryMerklePathIndex opening.layers = NAryMerklePathIndex otherOpening.layers",
            "opening.layers.length = otherOpening.layers.length",
            "otherOpening.leaf = opening.leaf",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_witness_opening_arity_four_same_index_leaf_binding_from_bundle",
        &["verified_concrete_nary_merkle_opening_arity_four_same_index_leaf_eq_from_bundle"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_witness_opening_arity_two_root_commits_to_leaf_at_index_from_bundle",
        &[
            "AssumptionBundle system",
            "CentralizedNAryMerkleCompressionCollisionResistance",
            "NAryMerklePathHasArity 2 opening.layers",
            "NAryMerklePathOpeningVerifies compress root opening",
            "NAryMerklePathRootCommitsToLeafAtIndex",
            "opening.leaf",
            "opening.layers",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_witness_opening_arity_two_root_commits_to_leaf_at_index_from_bundle",
        &["verified_concrete_nary_merkle_opening_implies_root_commits_to_leaf_at_index_from_bundle"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_witness_opening_arity_four_root_commits_to_leaf_at_index_from_bundle",
        &[
            "AssumptionBundle system",
            "CentralizedNAryMerkleCompressionCollisionResistance",
            "NAryMerklePathHasArity 4 opening.layers",
            "NAryMerklePathOpeningVerifies compress root opening",
            "NAryMerklePathRootCommitsToLeafAtIndex",
            "opening.leaf",
            "opening.layers",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_witness_opening_arity_four_root_commits_to_leaf_at_index_from_bundle",
        &["verified_concrete_nary_merkle_opening_implies_root_commits_to_leaf_at_index_from_bundle"],
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_opening_evidence_implies_external_source_requirement"
        )
        .contains("RuntimeOpeningEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_opening_evidence_implies_external_source_requirement"
            )
            .contains("ExternalSourceOpeningRequirement")
            && theorem_prefix(
                &lean_source,
                "runtime_opening_evidence_implies_external_source_requirement"
            )
            .contains("validation.runtimeSoundnessValidation.sourceValidation"),
        "opening evidence should expose the external-source requirement carried by runtime soundness evidence"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_opening_evidence_implies_bound_contract"
        )
        .contains("RuntimeOpeningEvidence")
            && theorem_prefix(
                &lean_source,
                "runtime_opening_evidence_implies_bound_contract"
            )
            .contains("RuntimeOpeningBoundContract"),
        "opening evidence should project compact constant, witness, and FRI opening bounds"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_opening_checked_acceptance_bound_contract"
        )
        .contains("RuntimeOpeningCheckedAcceptance")
            && theorem_prefix(
                &lean_source,
                "runtime_opening_checked_acceptance_bound_contract"
            )
            .contains("RuntimeOpeningBoundContract"),
        "checked opening acceptance should project compact opening-bound evidence"
    );
    assert!(
        theorem_prefix(
            &lean_source,
            "runtime_opening_checked_acceptance_pcs_and_fri"
        )
        .contains("system.pcsOpeningsValid publicInput proof")
            && theorem_prefix(
                &lean_source,
                "runtime_opening_checked_acceptance_pcs_and_fri"
            )
            .contains("system.friQueriesValid publicInput proof"),
        "checked opening acceptance should directly expose PCS opening and FRI query validity"
    );
    lean_binding::assert_theorem_prefix_omits(
        &lean_source,
        "runtime_opening_checked_acceptance_pcs_and_fri_without_assumptions",
        &["AssumptionBundle"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_opening_checked_acceptance_pcs_and_fri_without_assumptions",
        &[
            "RuntimeOpeningCheckedAcceptance system validation artifact publicInput proof",
            "system.pcsOpeningsValid publicInput proof",
            "system.friQueriesValid publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_opening_checked_acceptance_pcs_and_fri_without_assumptions",
        &[
            "validation.openingAcceptedImpliesConstantOpeningsBound",
            "validation.openingChecksImplyPcsOpeningsValid",
            "validation.friOpeningImpliesFriQueriesValid",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_opening_checked_acceptance_pcs_and_fri_without_assumptions",
        &["runtime_opening_checked_acceptance_evidence"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_opening_checked_acceptance_bound_pcs_fri_contract",
        &[
            "RuntimeOpeningCheckedAcceptance system validation artifact publicInput proof",
            "RuntimeOpeningBoundContract system validation artifact publicInput proof",
            "system.pcsOpeningsValid publicInput proof",
            "system.friQueriesValid publicInput proof",
        ],
    );
    lean_binding::assert_theorem_prefix_omits(
        &lean_source,
        "runtime_opening_checked_acceptance_bound_pcs_fri_contract",
        &["AssumptionBundle"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_opening_checked_acceptance_bound_pcs_fri_contract",
        &[
            "runtime_opening_checked_acceptance_bound_contract",
            "runtime_opening_checked_acceptance_pcs_and_fri_without_assumptions",
        ],
    );
}

fn theorem_prefix(source: &str, name: &str) -> String {
    let theorem_start = source
        .find(&format!("theorem {name}"))
        .unwrap_or_else(|| panic!("Lean source should contain theorem {name}"));
    let proof_start = source[theorem_start..]
        .find(" := by")
        .unwrap_or_else(|| panic!("Lean theorem {name} should have a proof body"));
    source[theorem_start..theorem_start + proof_start].to_owned()
}
