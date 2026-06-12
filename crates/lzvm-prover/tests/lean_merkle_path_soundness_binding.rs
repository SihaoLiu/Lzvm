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
        lean_source.contains("structure MerklePathModel")
            && lean_source.contains("def MerklePathCollisionResistance")
            && lean_source.contains("def MerkleRootCommitsToLeaf")
            && lean_source.contains("def CentralizedMerklePathCollisionResistance"),
        "Lean Merkle path model should expose path verification, collision resistance, commitment, and centralized assumption binding"
    );
    lean_binding::assert_theorem_declarations(
        &lean_source,
        &[
            "centralized_merkle_path_collision_resistance",
            "verified_merkle_path_implies_root_commits_to_leaf",
            "verified_merkle_path_implies_root_commits_to_leaf_from_assumption",
            "verified_merkle_path_implies_root_commits_to_leaf_from_bundle",
        ],
    );
    lean_binding::assert_theorem_prefix_contains(
        &lean_source,
        "verified_merkle_path_implies_root_commits_to_leaf_from_bundle",
        &[
            "AssumptionBundle system",
            "CentralizedMerklePathCollisionResistance",
            "MerkleRootCommitsToLeaf",
        ],
    );
}
