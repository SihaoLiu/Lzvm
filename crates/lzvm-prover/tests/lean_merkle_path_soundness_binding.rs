use std::path::Path;

#[path = "support/lean_binding.rs"]
mod lean_binding;

#[test]
fn lean_merkle_path_soundness_binds_central_hash_assumption() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_source = read_merkle_path_soundness_sources(crate_root);
    let top_level_path = crate_root.join("../../lean/Lzvm.lean");
    let top_level_source =
        std::fs::read_to_string(&top_level_path).expect("Lean top-level source should read");

    assert!(
        top_level_source.contains("import Lzvm.MerklePathSoundness"),
        "top-level Lean module should import Merkle path soundness"
    );
    assert!(
        !lean_source.contains("structure MerklePathModel")
            && !lean_source.contains("verifies : Root -> Leaf -> Path -> Prop")
            && !lean_source.contains("def MerklePathCollisionResistance"),
        "Lean Merkle path soundness should not package the binding conclusion as an abstract verification model"
    );
    assert!(
        lean_source.contains("inductive MerklePathDirection")
            && lean_source.contains("structure MerklePathLayer")
            && lean_source.contains("structure MerklePathOpening")
            && lean_source.contains("def MerklePathFold")
            && lean_source.contains("namespace MerklePathDirection")
            && lean_source.contains("def indexBit")
            && lean_source.contains("def MerklePathIndex")
            && lean_source.contains("def MerklePathVerifies")
            && lean_source.contains("structure MerkleCompressionCollision")
            && lean_source.contains("def MerkleCompressionNoCollision")
            && lean_source.contains("def MerkleCompressionCollisionFree")
            && lean_source.contains("def MerklePathRootCommitsToLeafAtIndex")
            && lean_source.contains("def MerklePathRootCommitsToLeafAtPosition")
            && lean_source.contains("def CentralizedMerkleCompressionCollisionResistance"),
        "Lean Merkle path model should expose concrete path data, fold verification, compression collision witnesses, indexed commitment, and centralized assumption binding"
    );
    assert!(
        lean_source.contains(
            "hashAssumptions.merkleHashCollisionResistanceStatement =\n    MerkleCompressionNoCollision compress"
        ),
        "centralized Merkle collision-resistance binding should expose no concrete compression collision as the bundled hash assumption"
    );
    assert!(
        lean_source.contains("structure NAryMerklePathLayer")
            && lean_source.contains("def NAryMerklePathFold")
            && lean_source.contains("def NAryMerklePathVerifies")
            && lean_source.contains("def NAryMerklePathIndex")
            && lean_source.contains("def NAryMerklePathSamePosition")
            && lean_source.contains("structure NAryMerkleCompressionCollision")
            && lean_source.contains("def NAryMerkleCompressionNoCollision")
            && lean_source.contains("def NAryMerklePathRootCommitsToLeafAtSamePositionIndex")
            && lean_source.contains("def NAryMerklePathRootCommitsToLeafAtIndex")
            && lean_source.contains("def NAryMerklePathRootCommitsToLeafAtPosition")
            && lean_source.contains("def NAryMerklePathLayerHasArity")
            && lean_source.contains("def NAryMerklePathHasArity")
            && lean_source.contains("def CentralizedNAryMerkleCompressionCollisionResistance"),
        "Lean Merkle path model should also expose n-ary path data, same-position indexed binding, numeric position, fixed-arity shape checks, and fold binding for runtime arity-2 and arity-4 opening paths"
    );
    lean_binding::assert_theorem_declarations(
        &lean_source,
        &[
            "different_leaf_same_index_verified_paths_imply_merkle_compression_collision",
            "different_leaf_same_index_verified_openings_imply_merkle_compression_collision",
            "different_leaf_same_position_verified_paths_imply_merkle_compression_collision",
            "different_leaf_same_position_verified_openings_imply_merkle_compression_collision",
            "different_leaf_same_position_verified_paths_contradict_no_collision",
            "different_leaf_same_position_verified_openings_contradict_no_collision",
            "different_leaf_same_position_verified_openings_contradict_assumption",
            "different_leaf_same_position_verified_openings_contradict_bundle",
            "different_leaf_same_position_verified_nary_paths_imply_merkle_compression_collision",
            "different_leaf_same_position_verified_nary_openings_imply_merkle_compression_collision",
            "different_leaf_same_position_verified_nary_paths_contradict_no_collision",
            "different_leaf_same_position_verified_nary_openings_contradict_no_collision",
            "different_leaf_same_position_verified_nary_openings_contradict_assumption",
            "different_leaf_same_position_verified_nary_openings_contradict_bundle",
            "different_leaf_same_arity_two_index_verified_nary_paths_imply_merkle_compression_collision",
            "different_leaf_same_arity_two_index_verified_nary_openings_imply_merkle_compression_collision",
            "different_leaf_same_arity_four_index_verified_nary_paths_imply_merkle_compression_collision",
            "different_leaf_same_arity_four_index_verified_nary_openings_imply_merkle_compression_collision",
            "different_leaf_same_arity_two_index_verified_nary_openings_contradict_no_collision",
            "different_leaf_same_arity_four_index_verified_nary_openings_contradict_no_collision",
            "different_leaf_same_arity_two_index_verified_nary_openings_contradict_bundle",
            "different_leaf_same_arity_four_index_verified_nary_openings_contradict_bundle",
            "merkle_compression_collision_free_of_no_collision",
            "nary_merkle_compression_collision_free_of_no_collision",
            "centralized_merkle_compression_collision_free",
            "centralized_nary_merkle_compression_collision_free",
            "merkle_path_same_index_implies_index_depth_eq",
            "nary_merkle_path_same_position_implies_index_depth_eq",
            "concrete_merkle_path_same_index_binding",
            "concrete_merkle_path_same_index_binding_from_no_collision",
            "concrete_nary_merkle_path_same_position_binding_from_no_collision",
            "nary_merkle_path_root_commits_to_leaf_at_same_position_index_from_no_collision",
            "nary_merkle_path_root_commits_to_leaf_at_same_position_index_from_bundle",
            "verified_concrete_nary_merkle_path_implies_root_commits_to_leaf_at_index_from_no_collision",
            "verified_concrete_nary_merkle_path_implies_root_commits_to_leaf_at_index_from_bundle",
            "verified_concrete_nary_merkle_path_implies_root_commits_to_leaf_at_position_from_no_collision",
            "verified_concrete_nary_merkle_path_implies_root_commits_to_leaf_at_position_from_bundle",
            "nary_merkle_opening_root_commits_to_leaf_at_same_position_index_from_no_collision",
            "nary_merkle_opening_root_commits_to_leaf_at_same_position_index_from_bundle",
            "verified_concrete_nary_merkle_opening_implies_root_commits_to_leaf_at_index_from_no_collision",
            "verified_concrete_nary_merkle_opening_implies_root_commits_to_leaf_at_index_from_bundle",
            "verified_concrete_nary_merkle_opening_implies_root_commits_to_leaf_at_position_from_no_collision",
            "verified_concrete_nary_merkle_opening_implies_root_commits_to_leaf_at_position_from_bundle",
            "nary_merkle_path_arity_two_index_implies_same_position",
            "nary_merkle_path_arity_four_index_implies_same_position",
            "nary_merkle_path_arity_two_index_binding_from_no_collision",
            "nary_merkle_path_arity_four_index_binding_from_no_collision",
            "nary_merkle_path_arity_two_index_binding_from_bundle",
            "nary_merkle_path_arity_four_index_binding_from_bundle",
            "nary_merkle_opening_arity_two_index_binding_from_bundle",
            "nary_merkle_opening_arity_four_index_binding_from_bundle",
            "verified_concrete_merkle_path_implies_root_commits_to_leaf_at_index",
            "verified_concrete_merkle_path_implies_root_commits_to_leaf_at_index_from_no_collision",
            "verified_concrete_merkle_path_implies_root_commits_to_leaf_at_index_from_assumption",
            "verified_concrete_merkle_path_implies_root_commits_to_leaf_at_index_from_bundle",
            "merkle_path_same_position_implies_same_index",
            "verified_concrete_merkle_path_implies_root_commits_to_leaf_at_position_from_no_collision",
            "verified_concrete_merkle_path_implies_root_commits_to_leaf_at_position_from_bundle",
            "verified_concrete_merkle_opening_implies_root_commits_to_leaf_at_index_from_no_collision",
            "verified_concrete_merkle_opening_implies_root_commits_to_leaf_at_index_from_bundle",
            "verified_concrete_merkle_opening_implies_root_commits_to_leaf_at_position_from_no_collision",
            "verified_concrete_merkle_opening_implies_root_commits_to_leaf_at_position_from_bundle",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "different_leaf_same_index_verified_openings_imply_merkle_compression_collision",
        &[
            "MerklePathOpeningVerifies compress root opening",
            "MerklePathOpeningVerifies compress root otherOpening",
            "MerklePathSameIndex opening.layers otherOpening.layers",
            "otherOpening.leaf ≠ opening.leaf",
            "Nonempty (MerkleCompressionCollision compress)",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "different_leaf_same_index_verified_openings_imply_merkle_compression_collision",
        &["different_leaf_same_index_verified_paths_imply_merkle_compression_collision"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "different_leaf_same_position_verified_paths_imply_merkle_compression_collision",
        &[
            "MerklePathIndex path = MerklePathIndex otherPath",
            "path.length = otherPath.length",
            "MerklePathVerifies compress root leaf path",
            "MerklePathVerifies compress root otherLeaf otherPath",
            "otherLeaf ≠ leaf",
            "Nonempty (MerkleCompressionCollision compress)",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "different_leaf_same_position_verified_paths_imply_merkle_compression_collision",
        &[
            "merkle_path_same_position_implies_same_index",
            "different_leaf_same_index_verified_paths_imply_merkle_compression_collision",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "different_leaf_same_position_verified_openings_imply_merkle_compression_collision",
        &[
            "MerklePathOpeningVerifies compress root opening",
            "MerklePathOpeningVerifies compress root otherOpening",
            "MerklePathIndex opening.layers = MerklePathIndex otherOpening.layers",
            "opening.layers.length = otherOpening.layers.length",
            "otherOpening.leaf ≠ opening.leaf",
            "Nonempty (MerkleCompressionCollision compress)",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "different_leaf_same_position_verified_openings_imply_merkle_compression_collision",
        &["different_leaf_same_position_verified_paths_imply_merkle_compression_collision"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "different_leaf_same_position_verified_openings_contradict_no_collision",
        &[
            "MerkleCompressionNoCollision compress",
            "MerklePathOpeningVerifies compress root opening",
            "MerklePathOpeningVerifies compress root otherOpening",
            "MerklePathIndex opening.layers = MerklePathIndex otherOpening.layers",
            "opening.layers.length = otherOpening.layers.length",
            "otherOpening.leaf ≠ opening.leaf",
            "False",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "different_leaf_same_position_verified_openings_contradict_no_collision",
        &[
            "different_leaf_same_position_verified_openings_imply_merkle_compression_collision",
            "noCollision collision",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "different_leaf_same_position_verified_openings_contradict_assumption",
        &[
            "HashCollisionResistanceAssumption",
            "CentralizedMerkleCompressionCollisionResistance",
            "MerklePathOpeningVerifies compress root opening",
            "MerklePathOpeningVerifies compress root otherOpening",
            "MerklePathIndex opening.layers = MerklePathIndex otherOpening.layers",
            "opening.layers.length = otherOpening.layers.length",
            "otherOpening.leaf ≠ opening.leaf",
            "False",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "different_leaf_same_position_verified_openings_contradict_assumption",
        &[
            "different_leaf_same_position_verified_openings_contradict_no_collision",
            "hashAssumptions.merkleHashCollisionResistance.evidence",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "different_leaf_same_position_verified_openings_contradict_bundle",
        &[
            "AssumptionBundle system",
            "CentralizedMerkleCompressionCollisionResistance",
            "MerklePathOpeningVerifies compress root opening",
            "MerklePathOpeningVerifies compress root otherOpening",
            "False",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "different_leaf_same_position_verified_openings_contradict_bundle",
        &["different_leaf_same_position_verified_openings_contradict_assumption"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "different_leaf_same_position_verified_nary_openings_imply_merkle_compression_collision",
        &[
            "NAryMerklePathOpeningVerifies compress root opening",
            "NAryMerklePathOpeningVerifies compress root otherOpening",
            "NAryMerklePathSamePosition opening.layers otherOpening.layers",
            "otherOpening.leaf ≠ opening.leaf",
            "Nonempty (NAryMerkleCompressionCollision compress)",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "different_leaf_same_position_verified_nary_openings_imply_merkle_compression_collision",
        &["different_leaf_same_position_verified_nary_paths_imply_merkle_compression_collision"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "different_leaf_same_position_verified_nary_openings_contradict_no_collision",
        &[
            "NAryMerkleCompressionNoCollision compress",
            "NAryMerklePathOpeningVerifies compress root opening",
            "NAryMerklePathOpeningVerifies compress root otherOpening",
            "NAryMerklePathSamePosition opening.layers otherOpening.layers",
            "otherOpening.leaf ≠ opening.leaf",
            "False",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "different_leaf_same_position_verified_nary_openings_contradict_no_collision",
        &[
            "different_leaf_same_position_verified_nary_openings_imply_merkle_compression_collision",
            "noCollision collision",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "different_leaf_same_position_verified_nary_openings_contradict_assumption",
        &[
            "HashCollisionResistanceAssumption",
            "CentralizedNAryMerkleCompressionCollisionResistance",
            "NAryMerklePathOpeningVerifies compress root opening",
            "NAryMerklePathOpeningVerifies compress root otherOpening",
            "NAryMerklePathSamePosition opening.layers otherOpening.layers",
            "otherOpening.leaf ≠ opening.leaf",
            "False",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "different_leaf_same_position_verified_nary_openings_contradict_assumption",
        &[
            "different_leaf_same_position_verified_nary_openings_contradict_no_collision",
            "hashAssumptions.merkleHashCollisionResistance.evidence",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "different_leaf_same_position_verified_nary_openings_contradict_bundle",
        &[
            "AssumptionBundle system",
            "CentralizedNAryMerkleCompressionCollisionResistance",
            "NAryMerklePathOpeningVerifies compress root opening",
            "NAryMerklePathOpeningVerifies compress root otherOpening",
            "False",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "different_leaf_same_position_verified_nary_openings_contradict_bundle",
        &["different_leaf_same_position_verified_nary_openings_contradict_assumption"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "different_leaf_same_arity_two_index_verified_nary_paths_imply_merkle_compression_collision",
        &[
            "NAryMerklePathHasArity 2 path",
            "NAryMerklePathHasArity 2 otherPath",
            "NAryMerklePathIndex path = NAryMerklePathIndex otherPath",
            "path.length = otherPath.length",
            "NAryMerklePathVerifies compress root leaf path",
            "NAryMerklePathVerifies compress root otherLeaf otherPath",
            "otherLeaf ≠ leaf",
            "Nonempty (NAryMerkleCompressionCollision compress)",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "different_leaf_same_arity_two_index_verified_nary_paths_imply_merkle_compression_collision",
        &[
            "nary_merkle_path_arity_two_index_implies_same_position",
            "different_leaf_same_position_verified_nary_paths_imply_merkle_compression_collision",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "different_leaf_same_arity_four_index_verified_nary_paths_imply_merkle_compression_collision",
        &[
            "NAryMerklePathHasArity 4 path",
            "NAryMerklePathHasArity 4 otherPath",
            "NAryMerklePathIndex path = NAryMerklePathIndex otherPath",
            "path.length = otherPath.length",
            "NAryMerklePathVerifies compress root leaf path",
            "NAryMerklePathVerifies compress root otherLeaf otherPath",
            "otherLeaf ≠ leaf",
            "Nonempty (NAryMerkleCompressionCollision compress)",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "different_leaf_same_arity_four_index_verified_nary_paths_imply_merkle_compression_collision",
        &[
            "nary_merkle_path_arity_four_index_implies_same_position",
            "different_leaf_same_position_verified_nary_paths_imply_merkle_compression_collision",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "different_leaf_same_arity_two_index_verified_nary_openings_imply_merkle_compression_collision",
        &[
            "NAryMerklePathHasArity 2 opening.layers",
            "NAryMerklePathHasArity 2 otherOpening.layers",
            "NAryMerklePathOpeningVerifies compress root opening",
            "NAryMerklePathOpeningVerifies compress root otherOpening",
            "NAryMerklePathIndex opening.layers = NAryMerklePathIndex otherOpening.layers",
            "opening.layers.length = otherOpening.layers.length",
            "otherOpening.leaf ≠ opening.leaf",
            "Nonempty (NAryMerkleCompressionCollision compress)",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "different_leaf_same_arity_two_index_verified_nary_openings_imply_merkle_compression_collision",
        &["different_leaf_same_arity_two_index_verified_nary_paths_imply_merkle_compression_collision"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "different_leaf_same_arity_four_index_verified_nary_openings_imply_merkle_compression_collision",
        &[
            "NAryMerklePathHasArity 4 opening.layers",
            "NAryMerklePathHasArity 4 otherOpening.layers",
            "NAryMerklePathOpeningVerifies compress root opening",
            "NAryMerklePathOpeningVerifies compress root otherOpening",
            "NAryMerklePathIndex opening.layers = NAryMerklePathIndex otherOpening.layers",
            "opening.layers.length = otherOpening.layers.length",
            "otherOpening.leaf ≠ opening.leaf",
            "Nonempty (NAryMerkleCompressionCollision compress)",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "different_leaf_same_arity_four_index_verified_nary_openings_imply_merkle_compression_collision",
        &["different_leaf_same_arity_four_index_verified_nary_paths_imply_merkle_compression_collision"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "different_leaf_same_arity_two_index_verified_nary_openings_contradict_no_collision",
        &[
            "NAryMerkleCompressionNoCollision compress",
            "NAryMerklePathHasArity 2 opening.layers",
            "NAryMerklePathHasArity 2 otherOpening.layers",
            "NAryMerklePathOpeningVerifies compress root opening",
            "NAryMerklePathOpeningVerifies compress root otherOpening",
            "NAryMerklePathIndex opening.layers = NAryMerklePathIndex otherOpening.layers",
            "opening.layers.length = otherOpening.layers.length",
            "otherOpening.leaf ≠ opening.leaf",
            "False",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "different_leaf_same_arity_two_index_verified_nary_openings_contradict_no_collision",
        &[
            "different_leaf_same_arity_two_index_verified_nary_openings_imply_merkle_compression_collision",
            "noCollision collision",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "different_leaf_same_arity_four_index_verified_nary_openings_contradict_no_collision",
        &[
            "NAryMerkleCompressionNoCollision compress",
            "NAryMerklePathHasArity 4 opening.layers",
            "NAryMerklePathHasArity 4 otherOpening.layers",
            "NAryMerklePathOpeningVerifies compress root opening",
            "NAryMerklePathOpeningVerifies compress root otherOpening",
            "NAryMerklePathIndex opening.layers = NAryMerklePathIndex otherOpening.layers",
            "opening.layers.length = otherOpening.layers.length",
            "otherOpening.leaf ≠ opening.leaf",
            "False",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "different_leaf_same_arity_four_index_verified_nary_openings_contradict_no_collision",
        &[
            "different_leaf_same_arity_four_index_verified_nary_openings_imply_merkle_compression_collision",
            "noCollision collision",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "different_leaf_same_arity_two_index_verified_nary_openings_contradict_bundle",
        &[
            "AssumptionBundle system",
            "CentralizedNAryMerkleCompressionCollisionResistance",
            "NAryMerklePathHasArity 2 opening.layers",
            "NAryMerklePathHasArity 2 otherOpening.layers",
            "NAryMerklePathOpeningVerifies compress root opening",
            "NAryMerklePathOpeningVerifies compress root otherOpening",
            "NAryMerklePathIndex opening.layers = NAryMerklePathIndex otherOpening.layers",
            "opening.layers.length = otherOpening.layers.length",
            "otherOpening.leaf ≠ opening.leaf",
            "False",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "different_leaf_same_arity_two_index_verified_nary_openings_contradict_bundle",
        &["different_leaf_same_arity_two_index_verified_nary_openings_contradict_no_collision"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "different_leaf_same_arity_four_index_verified_nary_openings_contradict_bundle",
        &[
            "AssumptionBundle system",
            "CentralizedNAryMerkleCompressionCollisionResistance",
            "NAryMerklePathHasArity 4 opening.layers",
            "NAryMerklePathHasArity 4 otherOpening.layers",
            "NAryMerklePathOpeningVerifies compress root opening",
            "NAryMerklePathOpeningVerifies compress root otherOpening",
            "NAryMerklePathIndex opening.layers = NAryMerklePathIndex otherOpening.layers",
            "opening.layers.length = otherOpening.layers.length",
            "otherOpening.leaf ≠ opening.leaf",
            "False",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "different_leaf_same_arity_four_index_verified_nary_openings_contradict_bundle",
        &["different_leaf_same_arity_four_index_verified_nary_openings_contradict_no_collision"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "verified_concrete_merkle_path_implies_root_commits_to_leaf_at_index_from_bundle",
        &[
            "AssumptionBundle system",
            "CentralizedMerkleCompressionCollisionResistance",
            "MerklePathRootCommitsToLeafAtIndex",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "verified_concrete_merkle_path_implies_root_commits_to_leaf_at_position_from_no_collision",
        &[
            "MerkleCompressionNoCollision compress",
            "MerklePathRootCommitsToLeafAtPosition",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "verified_concrete_merkle_path_implies_root_commits_to_leaf_at_index_from_no_collision",
        &[
            "MerkleCompressionNoCollision compress",
            "MerklePathRootCommitsToLeafAtIndex",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "verified_concrete_merkle_opening_implies_root_commits_to_leaf_at_index_from_no_collision",
        &[
            "MerkleCompressionNoCollision compress",
            "MerklePathOpeningVerifies compress root opening",
            "MerklePathRootCommitsToLeafAtIndex",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "concrete_merkle_path_same_index_binding",
        &[
            "MerkleCompressionCollisionFree compress",
            "MerklePathSameIndex path otherPath",
            "MerklePathVerifies compress root leaf path",
            "MerklePathVerifies compress root otherLeaf otherPath",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "concrete_nary_merkle_path_same_position_binding_from_no_collision",
        &[
            "NAryMerkleCompressionNoCollision compress",
            "NAryMerklePathSamePosition path otherPath",
            "NAryMerklePathVerifies compress root leaf path",
            "NAryMerklePathVerifies compress root otherLeaf otherPath",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "nary_merkle_path_root_commits_to_leaf_at_same_position_index_from_no_collision",
        &[
            "NAryMerkleCompressionNoCollision compress",
            "NAryMerklePathVerifies compress root leaf path",
            "NAryMerklePathRootCommitsToLeafAtSamePositionIndex",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "nary_merkle_path_root_commits_to_leaf_at_same_position_index_from_no_collision",
        &["concrete_nary_merkle_path_same_position_binding_from_no_collision"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "nary_merkle_path_root_commits_to_leaf_at_same_position_index_from_bundle",
        &[
            "AssumptionBundle system",
            "CentralizedNAryMerkleCompressionCollisionResistance",
            "NAryMerklePathVerifies compress root leaf path",
            "NAryMerklePathRootCommitsToLeafAtSamePositionIndex",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "verified_concrete_nary_merkle_path_implies_root_commits_to_leaf_at_position_from_bundle",
        &[
            "AssumptionBundle system",
            "CentralizedNAryMerkleCompressionCollisionResistance",
            "NAryMerklePathRootCommitsToLeafAtPosition",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "verified_concrete_nary_merkle_path_implies_root_commits_to_leaf_at_index_from_no_collision",
        &[
            "NAryMerkleCompressionNoCollision compress",
            "NAryMerklePathVerifies compress root leaf path",
            "NAryMerklePathRootCommitsToLeafAtIndex",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "verified_concrete_nary_merkle_path_implies_root_commits_to_leaf_at_index_from_no_collision",
        &["nary_merkle_path_root_commits_to_leaf_at_same_position_index_from_no_collision"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "verified_concrete_nary_merkle_path_implies_root_commits_to_leaf_at_index_from_bundle",
        &[
            "AssumptionBundle system",
            "CentralizedNAryMerkleCompressionCollisionResistance",
            "NAryMerklePathVerifies compress root leaf path",
            "NAryMerklePathRootCommitsToLeafAtIndex",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "verified_concrete_nary_merkle_opening_implies_root_commits_to_leaf_at_index_from_no_collision",
        &[
            "NAryMerkleCompressionNoCollision compress",
            "NAryMerklePathOpeningVerifies compress root opening",
            "NAryMerklePathRootCommitsToLeafAtIndex",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "nary_merkle_opening_root_commits_to_leaf_at_same_position_index_from_no_collision",
        &[
            "NAryMerkleCompressionNoCollision compress",
            "NAryMerklePathOpeningVerifies compress root opening",
            "NAryMerklePathRootCommitsToLeafAtSamePositionIndex",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "nary_merkle_opening_root_commits_to_leaf_at_same_position_index_from_no_collision",
        &["nary_merkle_path_root_commits_to_leaf_at_same_position_index_from_no_collision"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "nary_merkle_opening_root_commits_to_leaf_at_same_position_index_from_bundle",
        &[
            "AssumptionBundle system",
            "CentralizedNAryMerkleCompressionCollisionResistance",
            "NAryMerklePathOpeningVerifies compress root opening",
            "NAryMerklePathRootCommitsToLeafAtSamePositionIndex",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "verified_concrete_nary_merkle_opening_implies_root_commits_to_leaf_at_index_from_no_collision",
        &["nary_merkle_opening_root_commits_to_leaf_at_same_position_index_from_no_collision"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "verified_concrete_nary_merkle_opening_implies_root_commits_to_leaf_at_position_from_bundle",
        &[
            "AssumptionBundle system",
            "CentralizedNAryMerkleCompressionCollisionResistance",
            "NAryMerklePathOpeningVerifies compress root opening",
            "NAryMerklePathRootCommitsToLeafAtPosition",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "nary_merkle_path_same_position_implies_index_depth_eq",
        &[
            "NAryMerklePathSamePosition path otherPath",
            "NAryMerklePathIndex path = NAryMerklePathIndex otherPath",
            "path.length = otherPath.length",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "nary_merkle_path_arity_two_index_implies_same_position",
        &[
            "NAryMerklePathHasArity 2 path",
            "NAryMerklePathHasArity 2 otherPath",
            "NAryMerklePathIndex path = NAryMerklePathIndex otherPath",
            "path.length = otherPath.length",
            "NAryMerklePathSamePosition path otherPath",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "nary_merkle_path_arity_four_index_implies_same_position",
        &[
            "NAryMerklePathHasArity 4 path",
            "NAryMerklePathHasArity 4 otherPath",
            "NAryMerklePathIndex path = NAryMerklePathIndex otherPath",
            "path.length = otherPath.length",
            "NAryMerklePathSamePosition path otherPath",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "nary_merkle_path_arity_four_index_binding_from_no_collision",
        &[
            "NAryMerkleCompressionNoCollision compress",
            "NAryMerklePathHasArity 4 path",
            "NAryMerklePathVerifies compress root leaf path",
            "NAryMerklePathIndex path = NAryMerklePathIndex otherPath",
            "path.length = otherPath.length",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "nary_merkle_path_arity_four_index_binding_from_no_collision",
        &[
            "nary_merkle_path_arity_four_index_implies_same_position",
            "concrete_nary_merkle_path_same_position_binding_from_no_collision",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "nary_merkle_path_arity_two_index_binding_from_bundle",
        &[
            "AssumptionBundle system",
            "CentralizedNAryMerkleCompressionCollisionResistance",
            "NAryMerklePathHasArity 2 path",
            "NAryMerklePathVerifies compress root leaf path",
            "NAryMerklePathIndex path = NAryMerklePathIndex otherPath",
            "path.length = otherPath.length",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "nary_merkle_path_arity_two_index_binding_from_bundle",
        &["nary_merkle_path_arity_two_index_binding_from_no_collision"],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "nary_merkle_path_arity_four_index_binding_from_bundle",
        &[
            "AssumptionBundle system",
            "CentralizedNAryMerkleCompressionCollisionResistance",
            "NAryMerklePathHasArity 4 path",
            "NAryMerklePathVerifies compress root leaf path",
            "NAryMerklePathIndex path = NAryMerklePathIndex otherPath",
            "path.length = otherPath.length",
        ],
    );
    lean_binding::assert_theorem_body_contains(
        &lean_source,
        "nary_merkle_path_arity_four_index_binding_from_bundle",
        &["nary_merkle_path_arity_four_index_binding_from_no_collision"],
    );
}

fn read_merkle_path_soundness_sources(crate_root: &Path) -> String {
    [
        "../../lean/Lzvm/MerklePathSoundness.lean",
        "../../lean/Lzvm/MerklePathSoundness/Core.lean",
        "../../lean/Lzvm/MerklePathSoundness/NAry.lean",
        "../../lean/Lzvm/MerklePathSoundness/Binary.lean",
    ]
    .into_iter()
    .map(|relative| {
        std::fs::read_to_string(crate_root.join(relative)).unwrap_or_else(|error| {
            panic!("Lean Merkle path source {relative} should read: {error}")
        })
    })
    .collect::<Vec<_>>()
    .join("\n")
}
