use lzvm_artifacts::constraint_program::{
    encode_global_constraint_program, GlobalConstraintProgram,
};
use lzvm_artifacts::expression_program::{
    encode_expression_program, ExpressionEntry, ExpressionProgram,
};
use lzvm_artifacts::key_directory::{
    key_directory_catalog_digest, key_directory_catalog_digest_hex, read_key_directory_catalog,
    read_key_directory_layout, validate_key_directory_layout, KeyDirectoryError, KeyUnitKind,
    KeyUnitPaths,
};
use lzvm_artifacts::verification_key::{encode_verification_key_binary, VerificationKeyRoot};
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

fn sample_catalog_global_info_json() -> &'static str {
    r#"{
        "name": "sample-program",
        "air_groups": ["group-a"],
        "airs": [[{"name": "unit-a", "num_rows": 2}]],
        "curve": "None",
        "latticeSize": 368,
        "aggTypes": [[]],
        "nPublics": 0,
        "numChallenges": [1],
        "numProofValues": [],
        "publicsMap": [],
        "transcriptArity": 4
    }"#
}

fn sample_setup_info_json() -> &'static str {
    r#"{
        "nStages": 1,
        "nConstants": 2,
        "nPublics": 0,
        "nConstraints": 0,
        "qDeg": 3,
        "openingPoints": [0],
        "mapSectionsN": {
            "const": 2,
            "cm1": 1,
            "cm2": 1
        },
        "constPolsMap": [
            {"stage": 0, "name": "main.left", "dim": 1, "polsMapId": 0, "stageId": 0},
            {"stage": 0, "name": "main.right", "dim": 1, "polsMapId": 1, "stageId": 1}
        ],
        "challengesMap": [],
        "evMap": [],
        "boundaries": [],
        "starkStruct": {
            "nBits": 1,
            "nBitsExt": 2,
            "nQueries": 1,
            "steps": [
                {"nBits": 2},
                {"nBits": 1}
            ],
            "hashCommits": true,
            "lastLevelVerification": 2,
            "powBits": 0,
            "merkleTreeArity": 4,
            "verificationHashType": "GL",
            "transcriptArity": 4,
            "merkleTreeCustom": true
        }
    }"#
}

fn sample_expression_info_json() -> &'static str {
    r#"{
        "hintsInfo": [],
        "expressionsCode": [
            {
                "expId": 7,
                "stage": 2,
                "line": "query-expression",
                "tmpUsed": 0,
                "code": []
            }
        ],
        "constraints": []
    }"#
}

fn sample_verifier_info_json() -> &'static str {
    r#"{
        "qVerifier": {
            "tmpUsed": 1,
            "code": [
                {
                    "op": "copy",
                    "dest": {"type": "tmp", "id": 0, "dim": 3},
                    "src": [{"type": "number", "value": "1", "dim": 1}]
                }
            ]
        },
        "queryVerifier": {
            "expId": 7,
            "stage": 2,
            "tmpUsed": 1,
            "line": "query-expression",
            "code": [
                {
                    "op": "copy",
                    "dest": {"type": "tmp", "id": 0, "dim": 3},
                    "src": [{"type": "eval", "id": 0, "dim": 3}]
                }
            ]
        }
    }"#
}

fn sample_expression_program() -> ExpressionProgram {
    ExpressionProgram {
        max_tmp1: 1,
        max_tmp3: 1,
        max_args: 1,
        max_ops: 1,
        entries: vec![ExpressionEntry {
            expression_id: 7,
            destination_dimension: 1,
            destination_id: 0,
            stage: 1,
            temp1_count: 0,
            temp3_count: 0,
            ops_count: 1,
            ops_offset: 0,
            args_count: 1,
            args_offset: 0,
            source_line: "program-line".to_owned(),
        }],
        ops: vec![1],
        args: vec![2],
        numbers: vec![],
    }
}

fn sample_raw_fixed_columns() -> Vec<u8> {
    let mut bytes = Vec::new();
    for value in [1_u64, 10, 2, 20] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

fn sample_constant_tree(root: &VerificationKeyRoot) -> Vec<u8> {
    let VerificationKeyRoot::FieldElements(values) = root else {
        panic!("sample root should use field elements");
    };
    let mut bytes = vec![7_u8; 224];
    for (index, value) in values.iter().enumerate() {
        let offset = bytes.len() - 32 + index * 8;
        bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }
    bytes
}

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("lzvm-key-directory-{}-{name}", std::process::id()))
}

fn write_file(path: &Path) {
    fs::create_dir_all(path.parent().expect("path should have a parent"))
        .expect("fixture directory should be created");
    fs::write(path, []).expect("fixture file should be written");
}

fn write_text(path: &Path, value: &str) {
    fs::create_dir_all(path.parent().expect("path should have a parent"))
        .expect("fixture directory should be created");
    fs::write(path, value).expect("fixture file should be written");
}

fn write_bytes(path: &Path, value: impl AsRef<[u8]>) {
    fs::create_dir_all(path.parent().expect("path should have a parent"))
        .expect("fixture directory should be created");
    fs::write(path, value).expect("fixture file should be written");
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

fn write_catalog_global_files(root: &Path) {
    fs::create_dir_all(root).expect("fixture root should be created");
    fs::write(
        root.join("pilout.globalInfo.json"),
        sample_catalog_global_info_json(),
    )
    .expect("global metadata should be written");
    fs::write(root.join("pilout.globalConstraints.json"), "{}")
        .expect("global constraints metadata should be written");
    let constraints = encode_global_constraint_program(&GlobalConstraintProgram {
        entries: vec![],
        ops: vec![],
        args: vec![],
        numbers: vec![],
    })
    .expect("global constraints should encode");
    fs::write(root.join("pilout.globalConstraints.bin"), constraints)
        .expect("global constraints program should be written");
}

fn write_catalog_unit_files(unit: &KeyUnitPaths) {
    if let Some(path) = unit.setup_info() {
        write_text(&path, sample_setup_info_json());
    }
    if let Some(path) = unit.expression_info() {
        write_text(&path, sample_expression_info_json());
    }
    if let Some(path) = unit.verifier_info() {
        write_text(&path, sample_verifier_info_json());
    }

    let program = encode_expression_program(&sample_expression_program())
        .expect("expression program should encode");
    if let Some(path) = unit.expression_program() {
        write_bytes(&path, &program);
    }
    if let Some(path) = unit.verifier_program() {
        write_bytes(&path, &program);
    }

    write_text(&unit.verification_key_json(), "[1,2,3,4]");
    let root = VerificationKeyRoot::FieldElements(vec![1, 2, 3, 4]);
    write_bytes(
        &unit.verification_key_binary(),
        encode_verification_key_binary(&root).expect("verification key should encode"),
    );
    write_bytes(&unit.fixed_columns, sample_raw_fixed_columns());
}

fn write_catalog_constant_trees(layout: &lzvm_artifacts::key_directory::KeyDirectoryLayout) {
    let root = VerificationKeyRoot::FieldElements(vec![1, 2, 3, 4]);
    for unit in &layout.units {
        write_bytes(&unit.constant_tree, sample_constant_tree(&root));
    }
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

#[test]
fn reads_external_key_directory_catalog_when_requested() {
    let Some(root) = std::env::var_os("LZVM_EXTERNAL_KEY_DIR") else {
        return;
    };

    let catalog = read_key_directory_catalog(root).expect("external catalog should load");
    assert!(!catalog.units.is_empty());
    assert!(catalog
        .units
        .iter()
        .all(|unit| unit.expected_fixed_bytes as u64 == unit.actual_fixed_bytes));
}

#[test]
fn reads_key_directory_catalog_without_loading_fixed_values() {
    let dir = temp_dir("catalog");
    let _ = fs::remove_dir_all(&dir);
    write_catalog_global_files(&dir);
    let layout = read_key_directory_layout(&dir).expect("layout should parse");
    for unit in &layout.units {
        write_catalog_unit_files(unit);
    }

    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");

    assert_eq!(catalog.global_constraints.entries.len(), 0);
    assert_eq!(catalog.units.len(), 4);
    assert!(catalog
        .units
        .iter()
        .any(|unit| unit.paths.kind == KeyUnitKind::FinalAggregation));
    assert!(catalog
        .units
        .iter()
        .all(|unit| unit.expected_fixed_bytes == 32));
    assert!(catalog
        .units
        .iter()
        .all(|unit| unit.actual_fixed_bytes == 32));
    assert!(catalog.units.iter().all(|unit| !unit.constant_tree_present));
    assert!(catalog.units.iter().all(|unit| {
        unit.pcs_plan.base_domain_bits == 1
            && unit.pcs_plan.extended_domain_bits == 2
            && unit.pcs_plan.blowup_factor == 2
            && unit.pcs_plan.stage_commit_widths == vec![1, 1]
    }));

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn reads_key_directory_catalog_constant_tree_roots_when_present() {
    let dir = temp_dir("catalog-tree");
    let _ = fs::remove_dir_all(&dir);
    write_catalog_global_files(&dir);
    let layout = read_key_directory_layout(&dir).expect("layout should parse");
    for unit in &layout.units {
        write_catalog_unit_files(unit);
    }
    write_catalog_constant_trees(&layout);

    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");

    assert!(catalog.units.iter().all(|unit| unit.constant_tree_present));
    assert!(catalog
        .units
        .iter()
        .all(|unit| unit.constant_tree_bytes == Some(224)));
    assert!(catalog.units.iter().all(|unit| {
        unit.constant_tree_root == Some(VerificationKeyRoot::FieldElements(vec![1, 2, 3, 4]))
    }));

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn hashes_key_directory_catalogs_deterministically() {
    let dir = temp_dir("catalog-digest");
    let _ = fs::remove_dir_all(&dir);
    write_catalog_global_files(&dir);
    let layout = read_key_directory_layout(&dir).expect("layout should parse");
    for unit in &layout.units {
        write_catalog_unit_files(unit);
    }
    write_catalog_constant_trees(&layout);

    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let digest = key_directory_catalog_digest(&catalog).expect("digest should compute");
    let hex = key_directory_catalog_digest_hex(&catalog).expect("digest should encode");

    assert_eq!(hex.len(), 64);
    assert!(hex.chars().all(|value| value.is_ascii_hexdigit()));
    assert_eq!(
        key_directory_catalog_digest(&catalog).expect("digest should compute"),
        digest
    );

    let mut changed_plan = catalog.clone();
    changed_plan.units[0].pcs_plan.query_count += 1;
    assert_ne!(
        key_directory_catalog_digest(&changed_plan).expect("changed digest should compute"),
        digest
    );

    let mut changed_global = catalog.clone();
    changed_global.layout.global_info.transcript_arity += 1;
    assert_ne!(
        key_directory_catalog_digest(&changed_global).expect("changed digest should compute"),
        digest
    );

    write_text(&layout.units[0].verification_key_json(), "[2,2,2,2]");
    let changed_root = VerificationKeyRoot::FieldElements(vec![2, 2, 2, 2]);
    write_bytes(
        &layout.units[0].verification_key_binary(),
        encode_verification_key_binary(&changed_root).expect("verification key should encode"),
    );
    write_bytes(
        &layout.units[0].constant_tree,
        sample_constant_tree(&changed_root),
    );
    let changed = read_key_directory_catalog(&dir).expect("changed catalog should load");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_ne!(
        key_directory_catalog_digest(&changed).expect("changed digest should compute"),
        digest
    );
}

#[test]
fn rejects_catalog_entries_with_wrong_constant_tree_roots() {
    let dir = temp_dir("catalog-bad-tree-root");
    let _ = fs::remove_dir_all(&dir);
    write_catalog_global_files(&dir);
    let layout = read_key_directory_layout(&dir).expect("layout should parse");
    for unit in &layout.units {
        write_catalog_unit_files(unit);
    }
    write_catalog_constant_trees(&layout);
    let wrong_root = VerificationKeyRoot::FieldElements(vec![9, 9, 9, 9]);
    write_bytes(
        &layout.units[0].constant_tree,
        sample_constant_tree(&wrong_root),
    );

    let error = read_key_directory_catalog(&dir).expect_err("catalog should be rejected");

    assert!(matches!(
        error,
        KeyDirectoryError::ConstantTreeRootMismatch {
            kind: KeyUnitKind::Basic,
            ..
        }
    ));

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn rejects_catalog_entries_with_wrong_fixed_column_size() {
    let dir = temp_dir("catalog-bad-fixed");
    let _ = fs::remove_dir_all(&dir);
    write_catalog_global_files(&dir);
    let layout = read_key_directory_layout(&dir).expect("layout should parse");
    for unit in &layout.units {
        write_catalog_unit_files(unit);
    }
    fs::write(&layout.units[0].fixed_columns, [1_u8; 31]).expect("bad fixture should be written");

    let error = read_key_directory_catalog(&dir).expect_err("catalog should be rejected");

    assert!(matches!(
        error,
        KeyDirectoryError::FixedByteCountMismatch {
            kind: KeyUnitKind::Basic,
            expected: 32,
            found: 31,
            ..
        }
    ));

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}
