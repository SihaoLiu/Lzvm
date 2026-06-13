use std::path::Path;

#[path = "support/lean_binding.rs"]
mod lean_binding;

#[test]
fn lean_merkle_path_soundness_binds_central_hash_assumption() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_path = crate_root.join("../../lean/Lzvm/MerklePathSoundness.lean");
    let lean_source =
        std::fs::read_to_string(&lean_path).expect("Lean Merkle path source should read");
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
            && lean_source.contains("def NAryMerklePathRootCommitsToLeafAtIndex")
            && lean_source.contains("def NAryMerklePathRootCommitsToLeafAtPosition")
            && lean_source.contains("def CentralizedNAryMerkleCompressionCollisionResistance"),
        "Lean Merkle path model should also expose n-ary path data, numeric position, and fold binding for runtime arity-2 and arity-4 opening paths"
    );
    lean_binding::assert_theorem_declarations(
        &lean_source,
        &[
            "different_leaf_same_index_verified_paths_imply_merkle_compression_collision",
            "different_leaf_same_position_verified_nary_paths_imply_merkle_compression_collision",
            "merkle_compression_collision_free_of_no_collision",
            "nary_merkle_compression_collision_free_of_no_collision",
            "centralized_merkle_compression_collision_free",
            "centralized_nary_merkle_compression_collision_free",
            "merkle_path_same_index_implies_index_depth_eq",
            "nary_merkle_path_same_position_implies_index_depth_eq",
            "concrete_merkle_path_same_index_binding",
            "concrete_merkle_path_same_index_binding_from_no_collision",
            "concrete_nary_merkle_path_same_position_binding_from_no_collision",
            "verified_concrete_nary_merkle_path_implies_root_commits_to_leaf_at_index_from_no_collision",
            "verified_concrete_nary_merkle_path_implies_root_commits_to_leaf_at_index_from_bundle",
            "verified_concrete_nary_merkle_path_implies_root_commits_to_leaf_at_position_from_no_collision",
            "verified_concrete_nary_merkle_path_implies_root_commits_to_leaf_at_position_from_bundle",
            "verified_concrete_nary_merkle_opening_implies_root_commits_to_leaf_at_index_from_no_collision",
            "verified_concrete_nary_merkle_opening_implies_root_commits_to_leaf_at_index_from_bundle",
            "verified_concrete_nary_merkle_opening_implies_root_commits_to_leaf_at_position_from_no_collision",
            "verified_concrete_nary_merkle_opening_implies_root_commits_to_leaf_at_position_from_bundle",
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
}
