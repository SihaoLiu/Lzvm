use std::fs;
use std::path::{Path, PathBuf};

use lzvm_artifacts::constant_tree::read_constant_tree_file;
use lzvm_artifacts::constraint_program::{
    encode_global_constraint_program, encode_regular_constraint_program, ConstraintEntry,
    ConstraintProgram, GlobalConstraintProgram,
};
use lzvm_artifacts::expression_info::{
    encode_expression_info, parse_expression_info_json, read_expression_info_binary_file,
};
use lzvm_artifacts::expression_program::{
    encode_expression_program, ExpressionEntry, ExpressionProgram,
};
use lzvm_artifacts::fixed::{
    encode_raw_fixed_columns, read_fixed_columns_file_for_setup, FixedColumn, FixedColumns,
};
use lzvm_artifacts::global_info::{
    encode_global_info, parse_global_info_json, read_global_info_binary_file,
};
use lzvm_artifacts::hint_program::{
    encode_global_hint_program, read_regular_hint_program_file,
    regular_hint_program_from_expression_info, HintProgram,
};
use lzvm_artifacts::key_directory::{read_key_directory_layout, KeyUnitPaths};
use lzvm_artifacts::sectioned::{encode_sectioned_file, parse_sectioned_file, SectionedFile};
use lzvm_artifacts::setup_info::{
    encode_unit_setup_info, parse_unit_setup_info_json, read_unit_setup_info_binary_file,
    UnitSetupInfo,
};
use lzvm_artifacts::verification_key::{read_verification_key_binary_file, VerificationKeyRoot};
use lzvm_artifacts::verifier_info::{
    encode_verifier_info, parse_verifier_info_json, read_verifier_info_binary_file,
};
use lzvm_setup::{
    build_constant_tree_from_fixed_columns, write_base_directory, BaseDirectoryWriteReport,
    FixedExtensionBackend,
};

fn sample_global_info_json() -> &'static str {
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
        "hintsInfo": [
            {
                "name": "hint-a",
                "fields": [
                    {
                        "name": "field-a",
                        "values": [
                            {"op": "cm", "id": 0, "rowOffset": 0, "rowOffsetIndex": 0, "stage": 1, "stageId": 0, "dim": 1, "pos": [0]}
                        ]
                    }
                ]
            }
        ],
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

fn sample_regular_constraint_program() -> ConstraintProgram {
    ConstraintProgram {
        entries: vec![ConstraintEntry {
            stage: 1,
            destination_dimension: 1,
            destination_id: 0,
            first_row: 0,
            last_row: 1,
            temp1_count: 1,
            temp3_count: 0,
            ops_count: 1,
            ops_offset: 0,
            args_count: 8,
            args_offset: 0,
            intermediate: false,
            source_line: "fixture regular constraint".to_owned(),
        }],
        ops: vec![0],
        args: vec![1, 0, 0, 0, 0, 8, 0, 0],
        numbers: vec![1],
    }
}

fn sample_program_file() -> Vec<u8> {
    let expression = encode_expression_program(&sample_expression_program())
        .expect("expression program should encode");
    let regular = encode_regular_constraint_program(&sample_regular_constraint_program())
        .expect("regular constraints should encode");
    let mut expression_file =
        parse_sectioned_file(&expression, *b"chps", 1).expect("expression file should parse");
    let regular_file =
        parse_sectioned_file(&regular, *b"chps", 1).expect("regular file should parse");
    expression_file.sections.extend(regular_file.sections);
    encode_sectioned_file(&SectionedFile {
        kind: *b"chps",
        version: 1,
        sections: expression_file.sections,
    })
    .expect("combined program should encode")
}

fn sample_columns() -> FixedColumns {
    FixedColumns {
        group_name: "group-a".to_owned(),
        unit_name: "unit-a".to_owned(),
        row_count: 2,
        columns: vec![
            FixedColumn {
                name: "main.left".to_owned(),
                dimensions: vec![1],
                values: vec![5, 1],
            },
            FixedColumn {
                name: "main.right".to_owned(),
                dimensions: vec![1],
                values: vec![9, 9],
            },
        ],
    }
}

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "lzvm-setup-base-directory-{}-{name}",
        std::process::id()
    ))
}

fn write_bytes(path: &Path, bytes: impl AsRef<[u8]>) {
    fs::create_dir_all(path.parent().expect("path should have a parent"))
        .expect("fixture directory should be created");
    fs::write(path, bytes).expect("fixture should be written");
}

fn empty_hint_program() -> HintProgram {
    HintProgram { hints: Vec::new() }
}

fn global_constraint_program_file(program: &GlobalConstraintProgram) -> Vec<u8> {
    let constraints =
        encode_global_constraint_program(program).expect("global constraints should encode");
    let hints =
        encode_global_hint_program(&empty_hint_program()).expect("global hints should encode");
    let mut constraints_file =
        parse_sectioned_file(&constraints, *b"chps", 1).expect("constraints should parse");
    let hint_file = parse_sectioned_file(&hints, *b"chps", 1).expect("hints should parse");
    constraints_file.sections.extend(hint_file.sections);
    encode_sectioned_file(&SectionedFile {
        kind: *b"chps",
        version: 1,
        sections: constraints_file.sections,
    })
    .expect("combined global program should encode")
}

fn write_global_files(root: &Path) {
    fs::create_dir_all(root).expect("fixture root should be created");
    let info =
        parse_global_info_json(sample_global_info_json()).expect("global metadata should parse");
    write_bytes(
        &root.join("pilout.globalInfo.bin"),
        encode_global_info(&info).expect("global metadata should encode"),
    );
    write_bytes(
        &root.join("pilout.globalConstraints.bin"),
        global_constraint_program_file(&GlobalConstraintProgram {
            entries: vec![],
            ops: vec![],
            args: vec![],
            numbers: vec![],
        }),
    );
}

fn write_unit_files(paths: &KeyUnitPaths, setup: &UnitSetupInfo, raw_fixed: &[u8]) {
    if let Some(path) = paths.setup_info_binary() {
        let bytes = encode_unit_setup_info(setup).expect("setup metadata should encode");
        write_bytes(&path, bytes);
    }
    if let Some(path) = paths.expression_info_binary() {
        let expressions = parse_expression_info_json(sample_expression_info_json())
            .expect("expressions should parse");
        let bytes = encode_expression_info(&expressions).expect("expressions should encode");
        write_bytes(&path, bytes);
    }
    if let Some(path) = paths.verifier_info_binary() {
        let verifier = parse_verifier_info_json(sample_verifier_info_json())
            .expect("verifier metadata should parse");
        let bytes = encode_verifier_info(&verifier).expect("verifier metadata should encode");
        write_bytes(&path, bytes);
    }

    let program = sample_program_file();
    if let Some(path) = paths.expression_program() {
        write_bytes(&path, &program);
    }
    let verifier_program = encode_expression_program(&sample_expression_program())
        .expect("verifier program should encode");
    if let Some(path) = paths.verifier_program() {
        write_bytes(&path, verifier_program);
    }

    write_bytes(&paths.fixed_columns, raw_fixed);
}

fn create_base_directory_fixture(name: &str) -> PathBuf {
    let dir = temp_dir(name);
    let _ = fs::remove_dir_all(&dir);
    write_global_files(&dir);

    let setup = parse_unit_setup_info_json(sample_setup_info_json()).expect("setup should parse");
    let columns = sample_columns();
    let raw_fixed = encode_raw_fixed_columns(&columns, &setup).expect("raw fixed should encode");

    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    for unit in &layout.units {
        write_unit_files(unit, &setup, &raw_fixed);
    }

    dir
}

#[test]
fn writes_base_directory_artifacts_and_derives_keys() {
    let dir = create_base_directory_fixture("derive-key");
    let setup = parse_unit_setup_info_json(sample_setup_info_json()).expect("setup should parse");
    let expressions = parse_expression_info_json(sample_expression_info_json())
        .expect("expressions should parse");
    let verifier =
        parse_verifier_info_json(sample_verifier_info_json()).expect("verifier should parse");
    let columns = sample_columns();
    let expected_tree =
        build_constant_tree_from_fixed_columns(&columns, &setup).expect("tree should build");
    let expected_root = read_constant_tree_file_for_bytes(expected_tree, &setup);

    let report = write_base_directory(&dir, FixedExtensionBackend::Cpu, true)
        .expect("base directory should write");

    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let global = read_global_info_binary_file(dir.join("pilout.globalInfo.bin"))
        .expect("global metadata should parse");
    assert_eq!(
        global,
        parse_global_info_json(sample_global_info_json()).expect("global metadata should parse")
    );

    let mut fixed_bytes = 0_u64;
    let mut tree_bytes = 0_u64;
    let mut verkey_bytes = 0_u64;
    for unit in &layout.units {
        assert_eq!(
            read_unit_setup_info_binary_file(unit.setup_info().expect("setup path should derive"))
                .expect("setup output should parse"),
            setup
        );
        assert_eq!(
            read_expression_info_binary_file(
                unit.expression_info()
                    .expect("expression path should derive")
            )
            .expect("expression output should parse"),
            expressions
        );
        assert_eq!(
            read_verifier_info_binary_file(
                unit.verifier_info().expect("verifier path should derive")
            )
            .expect("verifier output should parse"),
            verifier
        );
        let hint_program = read_regular_hint_program_file(
            unit.expression_program()
                .expect("expression program path should derive"),
        )
        .expect("regular hints should parse");
        assert_eq!(
            hint_program,
            regular_hint_program_from_expression_info(&expressions)
                .expect("regular hints should derive")
        );

        let group_name = unit.group_name.as_deref().unwrap_or("raw");
        let unit_name = unit.unit_name.as_deref().unwrap_or("unit");
        let fixed =
            read_fixed_columns_file_for_setup(&unit.fixed_columns, &setup, group_name, unit_name)
                .expect("fixed output should parse");
        let mut expected_columns = columns.clone();
        expected_columns.group_name = group_name.to_owned();
        expected_columns.unit_name = unit_name.to_owned();
        assert_eq!(fixed, expected_columns);

        let tree = read_constant_tree_file(&unit.constant_tree, &setup)
            .expect("constant tree output should parse");
        assert_eq!(
            tree.root().expect("constant tree root should derive"),
            expected_root
        );
        let verkey = read_verification_key_binary_file(unit.verification_key_binary())
            .expect("derived key should parse");
        assert_eq!(verkey, expected_root);

        fixed_bytes += fs::metadata(&unit.fixed_columns)
            .expect("fixed output should exist")
            .len();
        tree_bytes += fs::metadata(&unit.constant_tree)
            .expect("constant tree output should exist")
            .len();
        verkey_bytes += fs::metadata(unit.verification_key_binary())
            .expect("derived key output should exist")
            .len();
    }

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(
        report,
        BaseDirectoryWriteReport {
            unit_count: layout.units.len(),
            fixed_bytes,
            tree_bytes,
            verkey_bytes: Some(verkey_bytes)
        }
    );
}

fn read_constant_tree_file_for_bytes(bytes: Vec<u8>, setup: &UnitSetupInfo) -> VerificationKeyRoot {
    let dir = temp_dir("root");
    let path = dir.join("tree.bin");
    let _ = fs::remove_dir_all(&dir);
    write_bytes(&path, bytes);
    let root = read_constant_tree_file(&path, setup)
        .expect("constant tree should parse")
        .root()
        .expect("constant tree root should derive");
    fs::remove_dir_all(&dir).expect("root fixture directory should be removed");
    root
}
