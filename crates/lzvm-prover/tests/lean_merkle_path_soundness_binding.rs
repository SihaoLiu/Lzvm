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
            && lean_source.contains("def CentralizedMerkleCompressionCollisionResistance"),
        "Lean Merkle path model should expose concrete path data, fold verification, compression collision witnesses, indexed commitment, and centralized assumption binding"
    );
    assert!(
        lean_source.contains(
            "hashAssumptions.merkleHashCollisionResistanceStatement =\n    MerkleCompressionNoCollision compress"
        ),
        "centralized Merkle collision-resistance binding should expose no concrete compression collision as the bundled hash assumption"
    );
    lean_binding::assert_theorem_declarations(
        &lean_source,
        &[
            "different_leaf_same_index_verified_paths_imply_merkle_compression_collision",
            "merkle_compression_collision_free_of_no_collision",
            "centralized_merkle_compression_collision_free",
            "merkle_path_same_index_implies_index_depth_eq",
            "concrete_merkle_path_same_index_binding",
            "concrete_merkle_path_same_index_binding_from_no_collision",
            "verified_concrete_merkle_path_implies_root_commits_to_leaf_at_index",
            "verified_concrete_merkle_path_implies_root_commits_to_leaf_at_index_from_no_collision",
            "verified_concrete_merkle_path_implies_root_commits_to_leaf_at_index_from_assumption",
            "verified_concrete_merkle_path_implies_root_commits_to_leaf_at_index_from_bundle",
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
        "verified_concrete_merkle_path_implies_root_commits_to_leaf_at_index_from_no_collision",
        &[
            "MerkleCompressionNoCollision compress",
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
}
