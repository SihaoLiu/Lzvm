use std::path::Path;

#[test]
fn lean_merkle_path_soundness_exposes_direct_leaf_binding_theorems() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let core_path = crate_root.join("../../lean/Lzvm/MerklePathSoundness/Core.lean");
    let core_source =
        std::fs::read_to_string(&core_path).expect("Merkle path core source should read");
    let binary_path = crate_root.join("../../lean/Lzvm/MerklePathSoundness/Binary.lean");
    let binary_source =
        std::fs::read_to_string(&binary_path).expect("binary Merkle path source should read");

    assert!(
        core_source.contains("def MerklePathFold")
            && core_source.contains("def MerklePathOpeningVerifies")
            && core_source.contains("def MerklePathRootCommitsToLeafAtPosition"),
        "Merkle path model should be concrete fold-based path verification"
    );
    assert!(
        binary_source.contains(
            "different_leaf_same_position_verified_paths_imply_merkle_compression_collision"
        ) && binary_source
            .contains("different_leaf_same_position_verified_openings_contradict_no_collision"),
        "binary Merkle path soundness should derive collisions from concrete verified paths"
    );

    for theorem_name in [
        "verified_concrete_merkle_path_same_position_leaf_eq_from_no_collision",
        "verified_concrete_merkle_opening_same_position_leaf_eq_from_no_collision",
        "verified_concrete_merkle_opening_same_position_leaf_eq_from_assumption",
        "verified_concrete_merkle_opening_same_position_leaf_eq_from_bundle",
    ] {
        assert!(
            binary_source.contains(theorem_name),
            "binary Merkle path soundness should expose {theorem_name}"
        );
    }
    assert!(
        !binary_source.contains("MerklePathCollisionResistance"),
        "binary Merkle path soundness should not package the target binding result as an abstract assumption"
    );
}
