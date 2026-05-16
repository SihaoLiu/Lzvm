use std::path::PathBuf;

use lzvm_artifacts::constant_tree::parse_constant_tree_bytes;
use lzvm_artifacts::fixed::read_raw_fixed_columns_file;
use lzvm_artifacts::setup_info::read_unit_setup_info_binary_file;
use lzvm_artifacts::verification_key::read_verification_key_binary_file;
use lzvm_setup::build_constant_tree_from_fixed_columns;

fn fixture_path(name: &str) -> PathBuf {
    std::env::var_os(name)
        .map(PathBuf::from)
        .unwrap_or_else(|| panic!("set {name} to run this parity test"))
}

#[test]
#[ignore = "requires external setup, raw fixed, and root fixture paths"]
fn native_constant_tree_root_matches_external_fixture() {
    let setup_path = fixture_path("LZVM_PARITY_SETUP_BIN");
    let raw_fixed_path = fixture_path("LZVM_PARITY_RAW_FIXED");
    let root_path = fixture_path("LZVM_PARITY_ROOT_BIN");

    let setup = read_unit_setup_info_binary_file(setup_path).expect("setup fixture should parse");
    let columns = read_raw_fixed_columns_file(raw_fixed_path, &setup, "external", "unit")
        .expect("raw fixed fixture should parse");
    let tree = build_constant_tree_from_fixed_columns(&columns, &setup).expect("tree should build");
    let generated = parse_constant_tree_bytes(tree, &setup)
        .expect("generated tree should parse")
        .root()
        .expect("generated root should extract");
    let expected = read_verification_key_binary_file(root_path).expect("root fixture should parse");

    assert_eq!(generated, expected);
}
