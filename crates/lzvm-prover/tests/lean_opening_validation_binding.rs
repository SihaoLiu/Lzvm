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
    let constant_opening_path = crate_root.join("src/constant_opening.rs");
    let constant_opening_source = std::fs::read_to_string(&constant_opening_path)
        .expect("constant opening source should read");
    let constant_segment_path =
        crate_root.join("../lzvm-artifacts/src/constant_opening_segment.rs");
    let constant_segment_source = std::fs::read_to_string(&constant_segment_path)
        .expect("constant opening segment source should read");
    let witness_opening_path = crate_root.join("src/witness_opening.rs");
    let witness_opening_source =
        std::fs::read_to_string(&witness_opening_path).expect("witness opening source should read");
    let witness_segment_path = crate_root.join("../lzvm-artifacts/src/witness_opening_segment.rs");
    let witness_segment_source = std::fs::read_to_string(&witness_segment_path)
        .expect("witness opening segment source should read");
    let fri_validation_path = crate_root.join("src/pcs_fri/validation.rs");
    let fri_validation_source =
        std::fs::read_to_string(&fri_validation_path).expect("FRI validation source should read");
    let fri_segment_path = crate_root.join("../lzvm-artifacts/src/pcs_fri_segment.rs");
    let fri_segment_source =
        std::fs::read_to_string(&fri_segment_path).expect("FRI segment source should read");
    let constant_opening_validation_body = function_body(
        &constant_opening_source,
        "pub fn validate_constant_opening_segments",
        "pub fn build_constant_opening_segment",
    );
    let witness_opening_validation_body = function_body(
        &witness_opening_source,
        "pub fn validate_witness_opening_segments",
        "pub fn build_witness_opening_segment",
    );
    let fri_opening_validation_body = function_body(
        &fri_validation_source,
        "fn validate_pcs_fri_opening_units",
        "pub fn validate_pcs_fri_opening_folds_from_units",
    );
    let fri_extension_helper_body = function_body(
        &fri_validation_source,
        "fn field_extension_from_words",
        "fn field_digest_from_words",
    );

    assert!(
        lean_binding::contains_import(&top_level_source, "Lzvm.OpeningValidation"),
        "top-level Lean module should import opening validation"
    );
    assert!(
        lean_binding::contains_import(&lean_source, "Lzvm.MerklePathSoundness")
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
            "runtime_opening_checked_acceptance_parser_boundary_contract",
            "runtime_constant_opening_checked_acceptance_parser_boundary_contract",
            "runtime_witness_opening_checked_acceptance_parser_boundary_contract",
            "runtime_fri_opening_checked_acceptance_parser_boundary_contract",
            "runtime_opening_checked_acceptance_all_parser_boundary_contracts",
            "runtime_opening_evidence_implies_external_source_requirement",
            "runtime_opening_evidence_implies_pcs_and_fri",
            "runtime_opening_checked_acceptance_pcs_and_fri_without_assumptions",
            "runtime_opening_checked_acceptance_bound_pcs_fri_contract",
            "runtime_opening_checked_acceptance_pcs_and_fri",
            "runtime_opening_checked_acceptance_sound",
            "runtime_opening_checked_acceptance_verifier_core_contract",
            "runtime_opening_checked_acceptance_evidence_core_and_sound",
            "runtime_opening_checked_acceptance_accepts_evidence_core_and_sound",
            "runtime_opening_checked_acceptance_audited_core_contract",
            "runtime_opening_checked_acceptance_concrete_segment_ids_allowed",
            "runtime_opening_checked_acceptance_concrete_core_sound_contract",
            "runtime_opening_checked_acceptance_full_soundness_contract",
            "runtime_opening_required_external_source_sound",
            "runtime_opening_required_external_source_verifier_core_contract",
            "runtime_opening_required_external_source_evidence_core_and_sound",
            "runtime_constant_opening_nary_checked_acceptance_constant_bound_from_bundle",
            "runtime_constant_opening_arity_two_same_index_leaf_binding_from_bundle",
            "runtime_constant_opening_arity_four_same_index_leaf_binding_from_bundle",
            "runtime_constant_opening_arity_two_root_commits_to_leaf_at_index_from_bundle",
            "runtime_constant_opening_arity_four_root_commits_to_leaf_at_index_from_bundle",
            "runtime_witness_opening_arity_two_same_index_leaf_binding_from_bundle",
            "runtime_witness_opening_arity_four_same_index_leaf_binding_from_bundle",
            "runtime_witness_opening_arity_two_root_commits_to_leaf_at_index_from_bundle",
            "runtime_witness_opening_arity_four_root_commits_to_leaf_at_index_from_bundle",
            "runtime_witness_opening_nary_checked_acceptance_witness_bound_from_bundle",
            "runtime_opening_checked_acceptance_pcs_and_fri_from_concrete_nary_merkle",
            "runtime_opening_checked_acceptance_runtime_soundness_evidence_from_opening_checks",
            "runtime_constant_opening_nary_checked_acceptance_constant_bound_from_hash_assumption",
            "runtime_witness_opening_nary_checked_acceptance_witness_bound_from_hash_assumption",
            "runtime_opening_checked_acceptance_pcs_and_fri_from_hash_assumption_concrete_nary_merkle",
            "runtime_opening_checked_acceptance_bound_pcs_fri_contract_from_hash_concrete_opening",
            "runtime_opening_checked_acceptance_runtime_soundness_evidence_from_concrete_nary_merkle",
            "runtime_opening_checked_acceptance_runtime_soundness_evidence_from_hash_concrete_opening",
        ],
    );
    assert!(
        lean_source.contains("structure RuntimeWitnessOpeningNAryConcreteBinding"),
        "Lean opening validation should model concrete runtime witness openings separately from abstract witnessOpeningsBound fields"
    );
    assert!(
        lean_source.contains("structure RuntimeConstantOpeningNAryConcreteBinding"),
        "Lean opening validation should model concrete runtime constant openings separately from abstract constantOpeningsBound fields"
    );
    assert!(
        lean_source.contains("structure RuntimeOpeningParserBoundary")
            && lean_source.contains("RuntimeConstantOpeningParserBoundaryContract")
            && lean_source.contains("RuntimeWitnessOpeningParserBoundaryContract")
            && lean_source.contains("RuntimeFriOpeningParserBoundaryContract")
            && constant_segment_source.contains("ValueNonCanonical")
            && constant_segment_source.contains("SiblingRootNonCanonical")
            && constant_opening_validation_body.contains(".map(Felt::from_u64)")
            && constant_opening_validation_body
                .contains("field_digest_from_words(*digest)")
            && !constant_opening_validation_body.contains("Felt::from_canonical(*value)")
            && witness_segment_source.contains("ValueNonCanonical")
            && witness_segment_source.contains("SiblingRootNonCanonical")
            && witness_opening_validation_body.contains(".map(Felt::from_u64)")
            && witness_opening_validation_body.contains("field_digest_from_words(*digest)")
            && !witness_opening_validation_body.contains("Felt::from_canonical(*value)")
            && fri_segment_source.contains("FinalPolynomialValueNonCanonical")
            && fri_segment_source.contains("QueryValueNonCanonical")
            && fri_opening_validation_body.contains(".map(field_extension_from_words)")
            && fri_opening_validation_body.contains("field_digest_from_words(*digest)")
            && !fri_opening_validation_body.contains("Felt::from_canonical(words")
            && fri_extension_helper_body.contains("Felt::from_u64(words[0])")
            && !fri_extension_helper_body.contains("Felt::from_canonical"),
        "Lean opening validation should expose parser boundaries used by runtime opening value validation while digest roots remain separately checked"
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_opening_checked_acceptance_parser_boundary_contract",
        &[
            "RuntimeOpeningParserBoundary system validation",
            "RuntimeOpeningCheckedAcceptance system validation artifact publicInput proof",
            "RuntimeOpeningParserBoundaryContract boundary artifact publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_opening_checked_acceptance_parser_boundary_contract",
        &[
            "boundary.openingAcceptedImpliesParserAccepted",
            "boundary.parserAcceptedImpliesOpeningValuesCanonical",
            "boundary.parserAcceptedImpliesDigestRootsCanonical",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_constant_opening_checked_acceptance_parser_boundary_contract",
        &[
            "RuntimeConstantOpeningParserBoundary system validation",
            "RuntimeOpeningCheckedAcceptance system validation artifact publicInput proof",
            "RuntimeConstantOpeningParserBoundaryContract boundary artifact publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_constant_opening_checked_acceptance_parser_boundary_contract",
        &["runtime_opening_checked_acceptance_parser_boundary_contract"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_witness_opening_checked_acceptance_parser_boundary_contract",
        &[
            "RuntimeWitnessOpeningParserBoundary system validation",
            "RuntimeOpeningCheckedAcceptance system validation artifact publicInput proof",
            "RuntimeWitnessOpeningParserBoundaryContract boundary artifact publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_witness_opening_checked_acceptance_parser_boundary_contract",
        &["runtime_opening_checked_acceptance_parser_boundary_contract"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_fri_opening_checked_acceptance_parser_boundary_contract",
        &[
            "RuntimeFriOpeningParserBoundary system validation",
            "RuntimeOpeningCheckedAcceptance system validation artifact publicInput proof",
            "RuntimeFriOpeningParserBoundaryContract boundary artifact publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_fri_opening_checked_acceptance_parser_boundary_contract",
        &["runtime_opening_checked_acceptance_parser_boundary_contract"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_opening_checked_acceptance_all_parser_boundary_contracts",
        &[
            "RuntimeConstantOpeningParserBoundary system validation",
            "RuntimeWitnessOpeningParserBoundary system validation",
            "RuntimeFriOpeningParserBoundary system validation",
            "RuntimeConstantOpeningParserBoundaryContract",
            "RuntimeWitnessOpeningParserBoundaryContract",
            "RuntimeFriOpeningParserBoundaryContract",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_opening_checked_acceptance_all_parser_boundary_contracts",
        &[
            "runtime_constant_opening_checked_acceptance_parser_boundary_contract",
            "runtime_witness_opening_checked_acceptance_parser_boundary_contract",
            "runtime_fri_opening_checked_acceptance_parser_boundary_contract",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_constant_opening_nary_checked_acceptance_constant_bound_from_bundle",
        &[
            "AssumptionBundle system",
            "RuntimeConstantOpeningNAryConcreteBinding",
            "CentralizedNAryMerkleCompressionCollisionResistance",
            "RuntimeOpeningCheckedAcceptance system validation artifact publicInput proof",
            "validation.constantOpeningsBound artifact publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_constant_opening_nary_checked_acceptance_constant_bound_from_bundle",
        &[
            "binding.concreteOpeningVerifies",
            "verified_concrete_nary_merkle_opening_implies_root_commits_to_leaf_at_index_from_bundle",
            "binding.constantRootCommitsToLeafImpliesConstantOpeningsBound",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_constant_opening_nary_checked_acceptance_constant_bound_from_bundle",
        &["validation.openingAcceptedImpliesConstantOpeningsBound"],
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
        "runtime_opening_checked_acceptance_pcs_and_fri_from_concrete_nary_merkle",
        &[
            "AssumptionBundle system",
            "RuntimeConstantOpeningNAryConcreteBinding",
            "RuntimeWitnessOpeningNAryConcreteBinding",
            "CentralizedNAryMerkleCompressionCollisionResistance",
            "RuntimeOpeningCheckedAcceptance system validation artifact publicInput proof",
            "system.pcsOpeningsValid publicInput proof",
            "system.friQueriesValid publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_opening_checked_acceptance_pcs_and_fri_from_concrete_nary_merkle",
        &[
            "runtime_constant_opening_nary_checked_acceptance_constant_bound_from_bundle",
            "runtime_witness_opening_nary_checked_acceptance_witness_bound_from_bundle",
            "validation.openingAcceptedImpliesFriOpeningBound",
            "validation.openingChecksImplyPcsOpeningsValid",
            "validation.friOpeningImpliesFriQueriesValid",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_opening_checked_acceptance_pcs_and_fri_from_concrete_nary_merkle",
        &[
            "validation.openingAcceptedImpliesConstantOpeningsBound",
            "validation.openingAcceptedImpliesWitnessOpeningsBound",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_opening_checked_acceptance_runtime_soundness_evidence_from_opening_checks",
        &[
            "AssumptionBundle system",
            "RuntimeOpeningValidation system",
            "RuntimeOpeningCheckedAcceptance system validation artifact publicInput proof",
            "RuntimeSoundnessEvidence",
            "validation.runtimeSoundnessValidation",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_opening_checked_acceptance_runtime_soundness_evidence_from_opening_checks",
        &[
            "runtime_transcript_binding_checked_acceptance_transcript_and_core_contract",
            "runtime_opening_checked_acceptance_pcs_and_fri_without_assumptions",
            "validation.openingAcceptedImpliesRuntimeSoundnessAccepted",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_opening_checked_acceptance_runtime_soundness_evidence_from_opening_checks",
        &[
            "required_crypto_assumptions_pcs_opening_soundness",
            "required_crypto_assumptions_fri_query_soundness",
            "runtime_soundness_checked_acceptance_evidence",
            "runtime_transcript_binding_checked_acceptance_sound",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_opening_checked_acceptance_runtime_soundness_evidence_from_hash_concrete_opening",
        &[
            "AssumptionBundle system",
            "HashCollisionResistanceAssumption",
            "RuntimeConstantOpeningNAryConcreteBinding",
            "RuntimeWitnessOpeningNAryConcreteBinding",
            "CentralizedNAryMerkleCompressionCollisionResistance",
            "RuntimeOpeningValidation system",
            "RuntimeOpeningCheckedAcceptance system validation artifact publicInput proof",
            "RuntimeSoundnessEvidence",
            "validation.runtimeSoundnessValidation",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_opening_checked_acceptance_runtime_soundness_evidence_from_hash_concrete_opening",
        &[
            "runtime_transcript_binding_checked_acceptance_transcript_and_core_contract",
            "runtime_opening_checked_acceptance_pcs_and_fri_from_hash_assumption_concrete_nary_merkle",
            "validation.openingAcceptedImpliesRuntimeSoundnessAccepted",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_opening_checked_acceptance_runtime_soundness_evidence_from_hash_concrete_opening",
        &[
            "runtime_opening_checked_acceptance_pcs_and_fri_without_assumptions",
            "runtime_opening_checked_acceptance_evidence",
            "runtime_soundness_checked_acceptance_evidence",
            "runtime_transcript_binding_checked_acceptance_sound",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_opening_checked_acceptance_runtime_soundness_evidence_from_concrete_nary_merkle",
        &[
            "AssumptionBundle system",
            "RuntimeConstantOpeningNAryConcreteBinding",
            "RuntimeWitnessOpeningNAryConcreteBinding",
            "CentralizedNAryMerkleCompressionCollisionResistance",
            "assumptions.crypto.hashCollisionResistance",
            "RuntimeOpeningValidation system",
            "RuntimeOpeningCheckedAcceptance system validation artifact publicInput proof",
            "RuntimeSoundnessEvidence",
            "validation.runtimeSoundnessValidation",
        ],
    );
    lean_binding::assert_theorem_prefix_omits(
        &lean_source,
        "runtime_opening_checked_acceptance_runtime_soundness_evidence_from_concrete_nary_merkle",
        &["HashCollisionResistanceAssumption"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_opening_checked_acceptance_runtime_soundness_evidence_from_concrete_nary_merkle",
        &[
            "runtime_opening_checked_acceptance_runtime_soundness_evidence_from_hash_concrete_opening",
            "assumptions.crypto.hashCollisionResistance",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_opening_checked_acceptance_runtime_soundness_evidence_from_concrete_nary_merkle",
        &[
            "runtime_opening_checked_acceptance_pcs_and_fri_without_assumptions",
            "runtime_opening_checked_acceptance_evidence",
            "runtime_soundness_checked_acceptance_evidence",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_constant_opening_nary_checked_acceptance_constant_bound_from_hash_assumption",
        &[
            "HashCollisionResistanceAssumption",
            "RuntimeConstantOpeningNAryConcreteBinding",
            "CentralizedNAryMerkleCompressionCollisionResistance",
            "RuntimeOpeningCheckedAcceptance system validation artifact publicInput proof",
            "validation.constantOpeningsBound artifact publicInput proof",
        ],
    );
    lean_binding::assert_theorem_prefix_omits(
        &lean_source,
        "runtime_constant_opening_nary_checked_acceptance_constant_bound_from_hash_assumption",
        &["AssumptionBundle"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_constant_opening_nary_checked_acceptance_constant_bound_from_hash_assumption",
        &[
            "binding.concreteOpeningVerifies",
            "verified_concrete_nary_merkle_opening_implies_root_commits_to_leaf_at_index_from_assumption",
            "binding.constantRootCommitsToLeafImpliesConstantOpeningsBound",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_witness_opening_nary_checked_acceptance_witness_bound_from_hash_assumption",
        &[
            "HashCollisionResistanceAssumption",
            "RuntimeWitnessOpeningNAryConcreteBinding",
            "CentralizedNAryMerkleCompressionCollisionResistance",
            "RuntimeOpeningCheckedAcceptance system validation artifact publicInput proof",
            "validation.witnessOpeningsBound artifact publicInput proof",
        ],
    );
    lean_binding::assert_theorem_prefix_omits(
        &lean_source,
        "runtime_witness_opening_nary_checked_acceptance_witness_bound_from_hash_assumption",
        &["AssumptionBundle"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_witness_opening_nary_checked_acceptance_witness_bound_from_hash_assumption",
        &[
            "binding.concreteOpeningVerifies",
            "verified_concrete_nary_merkle_opening_implies_root_commits_to_leaf_at_index_from_assumption",
            "binding.witnessRootCommitsToLeafImpliesWitnessOpeningsBound",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_opening_checked_acceptance_pcs_and_fri_from_hash_assumption_concrete_nary_merkle",
        &[
            "HashCollisionResistanceAssumption",
            "RuntimeConstantOpeningNAryConcreteBinding",
            "RuntimeWitnessOpeningNAryConcreteBinding",
            "CentralizedNAryMerkleCompressionCollisionResistance",
            "RuntimeOpeningCheckedAcceptance system validation artifact publicInput proof",
            "system.pcsOpeningsValid publicInput proof",
            "system.friQueriesValid publicInput proof",
        ],
    );
    lean_binding::assert_theorem_prefix_omits(
        &lean_source,
        "runtime_opening_checked_acceptance_pcs_and_fri_from_hash_assumption_concrete_nary_merkle",
        &["AssumptionBundle"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_opening_checked_acceptance_pcs_and_fri_from_hash_assumption_concrete_nary_merkle",
        &[
            "runtime_constant_opening_nary_checked_acceptance_constant_bound_from_hash_assumption",
            "runtime_witness_opening_nary_checked_acceptance_witness_bound_from_hash_assumption",
            "validation.openingAcceptedImpliesFriOpeningBound",
            "validation.openingChecksImplyPcsOpeningsValid",
            "validation.friOpeningImpliesFriQueriesValid",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_opening_checked_acceptance_bound_pcs_fri_contract_from_hash_concrete_opening",
        &[
            "HashCollisionResistanceAssumption",
            "RuntimeConstantOpeningNAryConcreteBinding",
            "RuntimeWitnessOpeningNAryConcreteBinding",
            "CentralizedNAryMerkleCompressionCollisionResistance",
            "RuntimeOpeningCheckedAcceptance system validation artifact publicInput proof",
            "RuntimeOpeningBoundContract system validation artifact publicInput proof",
            "system.pcsOpeningsValid publicInput proof",
            "system.friQueriesValid publicInput proof",
        ],
    );
    lean_binding::assert_theorem_prefix_omits(
        &lean_source,
        "runtime_opening_checked_acceptance_bound_pcs_fri_contract_from_hash_concrete_opening",
        &["AssumptionBundle"],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_opening_checked_acceptance_bound_pcs_fri_contract_from_hash_concrete_opening",
        &[
            "runtime_constant_opening_nary_checked_acceptance_constant_bound_from_hash_assumption",
            "runtime_witness_opening_nary_checked_acceptance_witness_bound_from_hash_assumption",
            "validation.openingAcceptedImpliesFriOpeningBound",
            "runtime_opening_checked_acceptance_pcs_and_fri_from_hash_assumption_concrete_nary_merkle",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_opening_checked_acceptance_bound_pcs_fri_contract_from_hash_concrete_opening",
        &[
            "runtime_opening_checked_acceptance_bound_contract",
            "runtime_opening_checked_acceptance_pcs_and_fri_without_assumptions",
            "validation.openingAcceptedImpliesConstantOpeningsBound",
            "validation.openingAcceptedImpliesWitnessOpeningsBound",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_constant_opening_arity_two_same_index_leaf_binding_from_bundle",
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
        "runtime_constant_opening_arity_two_same_index_leaf_binding_from_bundle",
        &["verified_concrete_nary_merkle_opening_arity_two_same_index_leaf_eq_from_bundle"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_constant_opening_arity_four_same_index_leaf_binding_from_bundle",
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
        "runtime_constant_opening_arity_four_same_index_leaf_binding_from_bundle",
        &["verified_concrete_nary_merkle_opening_arity_four_same_index_leaf_eq_from_bundle"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_constant_opening_arity_two_root_commits_to_leaf_at_index_from_bundle",
        &[
            "AssumptionBundle system",
            "CentralizedNAryMerkleCompressionCollisionResistance",
            "NAryMerklePathHasArity 2 opening.layers",
            "NAryMerklePathOpeningVerifies compress root opening",
            "NAryMerklePathRootCommitsToLeafAtArityIndex",
            "2",
            "opening.leaf",
            "opening.layers",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_constant_opening_arity_two_root_commits_to_leaf_at_index_from_bundle",
        &["verified_concrete_nary_merkle_opening_arity_two_implies_root_commits_to_leaf_at_index_from_bundle"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_constant_opening_arity_four_root_commits_to_leaf_at_index_from_bundle",
        &[
            "AssumptionBundle system",
            "CentralizedNAryMerkleCompressionCollisionResistance",
            "NAryMerklePathHasArity 4 opening.layers",
            "NAryMerklePathOpeningVerifies compress root opening",
            "NAryMerklePathRootCommitsToLeafAtArityIndex",
            "4",
            "opening.leaf",
            "opening.layers",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_constant_opening_arity_four_root_commits_to_leaf_at_index_from_bundle",
        &["verified_concrete_nary_merkle_opening_arity_four_implies_root_commits_to_leaf_at_index_from_bundle"],
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
            "NAryMerklePathRootCommitsToLeafAtArityIndex",
            "2",
            "opening.leaf",
            "opening.layers",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_witness_opening_arity_two_root_commits_to_leaf_at_index_from_bundle",
        &["verified_concrete_nary_merkle_opening_arity_two_implies_root_commits_to_leaf_at_index_from_bundle"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_witness_opening_arity_four_root_commits_to_leaf_at_index_from_bundle",
        &[
            "AssumptionBundle system",
            "CentralizedNAryMerkleCompressionCollisionResistance",
            "NAryMerklePathHasArity 4 opening.layers",
            "NAryMerklePathOpeningVerifies compress root opening",
            "NAryMerklePathRootCommitsToLeafAtArityIndex",
            "4",
            "opening.leaf",
            "opening.layers",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_witness_opening_arity_four_root_commits_to_leaf_at_index_from_bundle",
        &["verified_concrete_nary_merkle_opening_arity_four_implies_root_commits_to_leaf_at_index_from_bundle"],
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
        "runtime_opening_evidence_implies_pcs_and_fri",
        &[
            "RuntimeOpeningEvidence",
            "system.pcsOpeningsValid publicInput proof",
            "system.friQueriesValid publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_opening_evidence_implies_pcs_and_fri",
        &["exact evidence.right.right.right.right"],
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
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_opening_checked_acceptance_verifier_core_contract",
        &[
            "openingAcceptedImpliesRuntimeSoundnessAccepted",
            "runtime_soundness_checked_acceptance_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_opening_checked_acceptance_verifier_core_contract",
        &[
            "runtime_opening_checked_acceptance_sound",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_opening_checked_acceptance_evidence_core_and_sound",
        &[
            "RuntimeOpeningEvidence",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_opening_checked_acceptance_evidence_core_and_sound",
        &[
            "runtime_opening_checked_acceptance_sound",
            "runtime_opening_checked_acceptance_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_opening_checked_acceptance_evidence_core_and_sound",
        &["sound_witness_implies_verifier_core_contract"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_opening_checked_acceptance_accepts_evidence_core_and_sound",
        &[
            "RuntimeOpeningCheckedAcceptance system validation artifact publicInput proof",
            "system.accepts publicInput proof",
            "RuntimeOpeningEvidence",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_opening_checked_acceptance_accepts_evidence_core_and_sound",
        &[
            "openingAcceptedImpliesRuntimeSoundnessAccepted",
            "runtime_soundness_checked_acceptance_accepts_evidence_core_and_sound",
            "runtime_opening_checked_acceptance_evidence_core_and_sound",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_opening_checked_acceptance_accepts_evidence_core_and_sound",
        &[
            "runtime_soundness_checked_acceptance_verifier_accepts",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_opening_checked_acceptance_audited_core_contract",
        &[
            "RequiredCryptographicAssumptionStatements assumptions.crypto",
            "RequiredSemanticAssumptionStatements assumptions.semantic",
            "RuntimeOpeningEvidence",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
            "requiresExternalSource",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_opening_checked_acceptance_audited_core_contract",
        &[
            "assumption_bundle_carries_required_crypto_evidence",
            "assumption_bundle_carries_required_semantic_evidence",
            "runtime_opening_checked_acceptance_sound",
            "runtime_opening_checked_acceptance_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_opening_checked_acceptance_audited_core_contract",
        &[
            "assumption_bundle_carries_required_evidence",
            "runtime_opening_checked_acceptance_evidence_core_and_sound",
            "sound_witness_implies_verifier_core_contract",
            "abstract_verifier_sound",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_opening_checked_acceptance_concrete_segment_ids_allowed",
        &[
            "RuntimeOpeningCheckedAcceptance system validation artifact publicInput proof",
            "RuntimeProofArtifactConcreteSegmentIdBinding",
            "validation.runtimeSoundnessValidation.transcriptValidation.artifactBindingValidation",
            "RuntimeProofArtifactConcreteSegmentIdsAllowed proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_opening_checked_acceptance_concrete_segment_ids_allowed",
        &[
            "openingAcceptedImpliesRuntimeSoundnessAccepted",
            "runtime_soundness_checked_acceptance_concrete_segment_ids_allowed",
        ],
    );
    lean_binding::assert_theorem_prefix_omits(
        &lean_source,
        "runtime_opening_checked_acceptance_concrete_segment_ids_allowed",
        &["AssumptionBundle"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_opening_checked_acceptance_concrete_core_sound_contract",
        &[
            "RuntimeOpeningCheckedAcceptance system validation artifact publicInput proof",
            "RuntimeProofArtifactConcreteSegmentIdBinding",
            "validation.runtimeSoundnessValidation.transcriptValidation.artifactBindingValidation",
            "RuntimeOpeningEvidence",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
            "RuntimeProofArtifactConcreteSegmentIdsAllowed proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_opening_checked_acceptance_concrete_core_sound_contract",
        &[
            "runtime_opening_checked_acceptance_evidence_core_and_sound",
            "runtime_opening_checked_acceptance_concrete_segment_ids_allowed",
        ],
    );
    for identifier in [
        "runtime_opening_checked_acceptance_sound",
        "sound_witness_implies_verifier_core_contract",
        "abstract_verifier_sound",
    ] {
        lean_binding::assert_theorem_body_omits_identifier(
            &lean_source,
            "runtime_opening_checked_acceptance_concrete_core_sound_contract",
            identifier,
        );
    }
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_opening_checked_acceptance_full_soundness_contract",
        &[
            "RuntimeOpeningEvidence",
            "RuntimeOpeningBoundContract system validation artifact publicInput proof",
            "system.pcsOpeningsValid publicInput proof",
            "system.friQueriesValid publicInput proof",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_opening_checked_acceptance_full_soundness_contract",
        &[
            "runtime_opening_checked_acceptance_sound",
            "runtime_opening_checked_acceptance_bound_pcs_fri_contract",
            "runtime_opening_checked_acceptance_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_opening_required_external_source_verifier_core_contract",
        &["runtime_opening_checked_acceptance_verifier_core_contract"],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_opening_required_external_source_verifier_core_contract",
        &[
            "runtime_opening_required_external_source_sound",
            "sound_witness_implies_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "runtime_opening_required_external_source_evidence_core_and_sound",
        &[
            "requiresExternalSource ->",
            "RuntimeOpeningEvidence",
            "ExternalSourceOpeningEvidence",
            "RuntimeVerifierCoreContract system publicInput proof",
            "SoundWitness system publicInput proof",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "runtime_opening_required_external_source_evidence_core_and_sound",
        &[
            "runtime_opening_required_external_source_sound",
            "runtime_opening_required_external_source_verifier_core_contract",
        ],
    );
    lean_binding::assert_theorem_body_omits(
        &lean_source,
        "runtime_opening_required_external_source_evidence_core_and_sound",
        &["sound_witness_implies_verifier_core_contract"],
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

fn function_body(source: &str, start: &str, end: &str) -> String {
    let start_index = source
        .find(start)
        .unwrap_or_else(|| panic!("source should contain {start}"));
    let rest = &source[start_index..];
    let end_index = rest
        .find(end)
        .unwrap_or_else(|| panic!("source should contain {end} after {start}"));
    rest[..end_index].to_owned()
}
