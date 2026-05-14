use lzvm_artifacts::key_directory::{
    read_key_directory_layout, validate_key_directory_layout, KeyDirectoryError, KeyUnitKind,
};
use std::fs;
use std::path::{Path, PathBuf};

fn sample_global_info_json() -> &'static str {
    r#"{
        "name": "sample-program",
        "air_groups": ["group-a", "group-b"],
        "airs": [
            [
                {"name": "unit-a", "num_rows": 16, "hasCompressor": true},
                {"name": "unit-b", "num_rows": 16}
            ],
            [
                {"name": "unit-c", "num_rows": 32}
            ]
        ],
        "curve": "None",
        "latticeSize": 368,
        "aggTypes": [[], []],
        "nPublics": 0,
        "numChallenges": [1, 2],
        "numProofValues": [],
        "publicsMap": [],
        "transcriptArity": 4
    }"#
}

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("lzvm-key-directory-{}-{name}", std::process::id()))
}

fn write_file(path: &Path) {
    fs::create_dir_all(path.parent().expect("path should have a parent"))
        .expect("fixture directory should be created");
    fs::write(path, []).expect("fixture file should be written");
}

fn write_global_files(root: &Path) {
    fs::create_dir_all(root).expect("fixture root should be created");
    fs::write(
        root.join("pilout.globalInfo.json"),
        sample_global_info_json(),
    )
    .expect("global metadata should be written");
    fs::write(root.join("pilout.globalConstraints.json"), "{}")
        .expect("global constraints metadata should be written");
    fs::write(root.join("pilout.globalConstraints.bin"), [])
        .expect("global constraints program should be written");
}

#[test]
fn derives_key_directory_units_from_global_metadata() {
    let dir = temp_dir("derive");
    let _ = fs::remove_dir_all(&dir);
    write_global_files(&dir);

    let layout = read_key_directory_layout(&dir).expect("layout should parse");
    let kinds = layout
        .units
        .iter()
        .map(|unit| unit.kind)
        .collect::<Vec<_>>();

    assert_eq!(
        kinds
            .iter()
            .filter(|kind| **kind == KeyUnitKind::Basic)
            .count(),
        3
    );
    assert_eq!(
        kinds
            .iter()
            .filter(|kind| **kind == KeyUnitKind::Compressor)
            .count(),
        1
    );
    assert_eq!(
        kinds
            .iter()
            .filter(|kind| **kind == KeyUnitKind::RecursiveFirst)
            .count(),
        3
    );
    assert_eq!(
        kinds
            .iter()
            .filter(|kind| **kind == KeyUnitKind::RecursiveSecond)
            .count(),
        2
    );
    assert_eq!(
        kinds
            .iter()
            .filter(|kind| **kind == KeyUnitKind::FinalAggregation)
            .count(),
        1
    );

    let basic = layout
        .units
        .iter()
        .find(|unit| {
            unit.kind == KeyUnitKind::Basic && unit.group_id == Some(0) && unit.unit_id == Some(0)
        })
        .expect("basic unit should exist");
    assert_eq!(
        basic.prefix,
        dir.join("sample-program")
            .join("group-a")
            .join("airs")
            .join("unit-a")
            .join("air")
            .join("unit-a")
    );

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn validates_required_key_directory_files() {
    let dir = temp_dir("validate");
    let _ = fs::remove_dir_all(&dir);
    write_global_files(&dir);

    let layout = read_key_directory_layout(&dir).expect("layout should parse");
    for required in layout.required_paths() {
        write_file(&required.path);
    }

    validate_key_directory_layout(&layout).expect("layout should validate");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn reports_missing_required_key_directory_files() {
    let dir = temp_dir("missing");
    let _ = fs::remove_dir_all(&dir);
    write_global_files(&dir);

    let layout = read_key_directory_layout(&dir).expect("layout should parse");
    let error = validate_key_directory_layout(&layout).expect_err("layout should be incomplete");

    assert!(matches!(
        error,
        KeyDirectoryError::MissingPath {
            role: "unit setup metadata",
            ..
        }
    ));

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn validates_external_key_directory_when_requested() {
    let Some(root) = std::env::var_os("LZVM_EXTERNAL_KEY_DIR") else {
        return;
    };

    let layout = read_key_directory_layout(root).expect("external layout should parse");
    validate_key_directory_layout(&layout).expect("external layout should validate");
    assert!(!layout.units.is_empty());
}
