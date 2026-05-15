use std::fs;
use std::path::{Path, PathBuf};

use lzvm_artifacts::constant_tree::parse_constant_tree_bytes;
use lzvm_artifacts::constraint_program::{
    encode_global_constraint_program, encode_regular_constraint_program, ConstraintEntry,
    ConstraintProgram, GlobalConstraintProgram,
};
use lzvm_artifacts::expression_info::{
    parse_expression_info_json, read_expression_info_binary_file,
};
use lzvm_artifacts::expression_program::{
    encode_expression_program, ExpressionEntry, ExpressionProgram,
};
use lzvm_artifacts::fixed::{encode_raw_fixed_columns, FixedColumn, FixedColumns};
use lzvm_artifacts::global_info::read_global_info_binary_file;
use lzvm_artifacts::key_directory::{read_key_directory_layout, KeyUnitPaths};
use lzvm_artifacts::pcs_material::{build_pcs_setup_material, read_pcs_setup_material_file};
use lzvm_artifacts::pcs_plan::{derive_pcs_setup_plan, read_pcs_setup_plan_file};
use lzvm_artifacts::sectioned::{encode_sectioned_file, parse_sectioned_file, SectionedFile};
use lzvm_artifacts::setup_info::{
    parse_unit_setup_info_json, read_unit_setup_info_binary_file, UnitSetupInfo,
};
use lzvm_artifacts::verification_key::{
    encode_verification_key_binary, read_verification_key_binary_file, VerificationKeyRoot,
};
use lzvm_artifacts::verifier_info::{parse_verifier_info_json, read_verifier_info_binary_file};
use lzvm_cli::run_cli;
use lzvm_setup::build_constant_tree_from_fixed_columns;

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
        "lzvm-cli-write-base-directory-{}-{name}",
        std::process::id()
    ))
}

fn write_bytes(path: &Path, bytes: impl AsRef<[u8]>) {
    fs::create_dir_all(path.parent().expect("path should have a parent"))
        .expect("fixture directory should be created");
    fs::write(path, bytes).expect("fixture should be written");
}

fn write_text(path: &Path, value: &str) {
    write_bytes(path, value.as_bytes());
}

fn root_from_tree(tree: &[u8]) -> VerificationKeyRoot {
    VerificationKeyRoot::FieldElements(
        tree[tree.len() - 32..]
            .chunks_exact(8)
            .map(|chunk| u64::from_le_bytes(chunk.try_into().expect("slice length checked")))
            .collect(),
    )
}

fn write_global_files(root: &Path) {
    fs::create_dir_all(root).expect("fixture root should be created");
    write_text(
        &root.join("pilout.globalInfo.json"),
        sample_global_info_json(),
    );
    write_text(&root.join("pilout.globalConstraints.json"), "{}");
    write_bytes(
        &root.join("pilout.globalConstraints.bin"),
        encode_global_constraint_program(&GlobalConstraintProgram {
            entries: vec![],
            ops: vec![],
            args: vec![],
            numbers: vec![],
        })
        .expect("global constraints should encode"),
    );
}

fn write_unit_files(
    paths: &KeyUnitPaths,
    setup: &UnitSetupInfo,
    raw_fixed: &[u8],
    root: &VerificationKeyRoot,
) {
    if let Some(path) = paths.setup_info() {
        write_text(&path, sample_setup_info_json());
    }
    if let Some(path) = paths.expression_info() {
        write_text(&path, sample_expression_info_json());
    }
    if let Some(path) = paths.verifier_info() {
        write_text(&path, sample_verifier_info_json());
    }

    let program = sample_program_file();
    if let Some(path) = paths.expression_program() {
        write_bytes(&path, &program);
    }
    let verifier_program = encode_expression_program(&sample_expression_program())
        .expect("verifier program should encode");
    if let Some(path) = paths.verifier_program() {
        write_bytes(&path, &verifier_program);
    }

    write_bytes(
        &paths.verification_key_binary(),
        encode_verification_key_binary(root).expect("root should encode"),
    );
    write_bytes(&paths.fixed_columns, raw_fixed);

    let expected_len = lzvm_artifacts::fixed::expected_raw_fixed_column_byte_count(setup)
        .expect("raw fixed length should derive");
    assert_eq!(raw_fixed.len(), expected_len);
}

fn create_key_directory(name: &str) -> (PathBuf, VerificationKeyRoot) {
    let dir = temp_dir(name);
    let _ = fs::remove_dir_all(&dir);
    write_global_files(&dir);

    let setup = parse_unit_setup_info_json(sample_setup_info_json()).expect("setup should parse");
    let columns = sample_columns();
    let raw_fixed = encode_raw_fixed_columns(&columns, &setup).expect("raw fixed should encode");
    let tree = build_constant_tree_from_fixed_columns(&columns, &setup).expect("tree should build");
    let root = root_from_tree(&tree);

    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    for unit in &layout.units {
        write_unit_files(unit, &setup, &raw_fixed, &root);
    }

    (dir, root)
}

fn remove_verification_keys(dir: &Path) {
    let layout = read_key_directory_layout(dir).expect("layout should derive");
    for unit in &layout.units {
        fs::remove_file(unit.verification_key_binary()).expect("binary key should be removed");
    }
}

#[test]
fn writes_base_directory_constant_trees_for_all_units() {
    let (dir, root) = create_key_directory("valid");

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "setup",
            "write-base-directory",
            dir.to_str().expect("directory path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let unit_count = layout.units.len();
    let mut tree_bytes = 0_u64;
    let mut fixed_bytes = 0_u64;
    for unit in &layout.units {
        let tree = fs::read(&unit.constant_tree).expect("constant tree should be written");
        tree_bytes += u64::try_from(tree.len()).expect("tree length should fit");
        fixed_bytes += fs::metadata(&unit.fixed_columns)
            .expect("fixed output should exist")
            .len();
        assert_eq!(root_from_tree(&tree), root);
    }
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\nunits={unit_count}\nfixed_bytes={fixed_bytes}\ntree_bytes={tree_bytes}\n"
        )
    );
    assert!(stderr.is_empty());
}

#[test]
fn writes_base_directory_global_metadata_binary() {
    let (dir, _) = create_key_directory("global-info-bin");

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "setup",
            "write-base-directory",
            dir.to_str().expect("directory path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    let global = read_global_info_binary_file(dir.join("pilout.globalInfo.bin"))
        .expect("binary global metadata should parse");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0);
    assert_eq!(global.name, "sample-program");
    assert!(stderr.is_empty());
}

#[test]
fn writes_base_directory_unit_setup_metadata_binary() {
    let (dir, _) = create_key_directory("unit-info-bin");

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "setup",
            "write-base-directory",
            dir.to_str().expect("directory path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let expected =
        parse_unit_setup_info_json(sample_setup_info_json()).expect("setup should parse");
    for unit in &layout.units {
        let path = unit
            .setup_info_binary()
            .expect("binary setup metadata path should derive");
        let setup = read_unit_setup_info_binary_file(path).expect("binary setup should parse");
        assert_eq!(setup, expected);
    }
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0);
    assert!(stderr.is_empty());
}

#[test]
fn writes_base_directory_verifier_metadata_binary() {
    let (dir, _) = create_key_directory("verifier-info-bin");

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "setup",
            "write-base-directory",
            dir.to_str().expect("directory path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let expected =
        parse_verifier_info_json(sample_verifier_info_json()).expect("verifier should parse");
    for unit in &layout.units {
        let path = unit
            .verifier_info_binary()
            .expect("binary verifier metadata path should derive");
        let verifier = read_verifier_info_binary_file(path).expect("binary verifier should parse");
        assert_eq!(verifier, expected);
    }
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0);
    assert!(stderr.is_empty());
}

#[test]
fn writes_base_directory_expression_metadata_binary() {
    let (dir, _) = create_key_directory("expression-info-bin");

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "setup",
            "write-base-directory",
            dir.to_str().expect("directory path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let expected = parse_expression_info_json(sample_expression_info_json())
        .expect("expressions should parse");
    for unit in &layout.units {
        let path = unit
            .expression_info_binary()
            .expect("binary expression metadata path should derive");
        let expressions =
            read_expression_info_binary_file(path).expect("binary expressions should parse");
        assert_eq!(expressions, expected);
    }
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0);
    assert!(stderr.is_empty());
}

#[test]
fn derives_verification_keys_for_base_directory_outputs() {
    let (dir, root) = create_key_directory("derive-verkey");
    remove_verification_keys(&dir);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "setup",
            "write-base-directory",
            "--derive-verkey",
            dir.to_str().expect("directory path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let unit_count = layout.units.len();
    let mut tree_bytes = 0_u64;
    let mut fixed_bytes = 0_u64;
    let mut verkey_bytes = 0_u64;
    for unit in &layout.units {
        let tree = fs::read(&unit.constant_tree).expect("constant tree should be written");
        let binary_root =
            read_verification_key_binary_file(unit.verification_key_binary()).expect("binary key");
        tree_bytes += u64::try_from(tree.len()).expect("tree length should fit");
        fixed_bytes += fs::metadata(&unit.fixed_columns)
            .expect("fixed output should exist")
            .len();
        verkey_bytes += fs::metadata(unit.verification_key_binary())
            .expect("binary key should exist")
            .len();
        assert_eq!(root_from_tree(&tree), root);
        assert_eq!(binary_root, root);
    }
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\nunits={unit_count}\nfixed_bytes={fixed_bytes}\ntree_bytes={tree_bytes}\nverkey_bytes={verkey_bytes}\n"
        )
    );
    assert!(stderr.is_empty());
}

#[test]
fn writes_pcs_setup_plans_for_all_units() {
    let (dir, _) = create_key_directory("pcs-plan");

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "setup",
            "write-pcs-directory",
            dir.to_str().expect("directory path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let unit_count = layout.units.len();
    let setup = parse_unit_setup_info_json(sample_setup_info_json()).expect("setup should parse");
    let expected = derive_pcs_setup_plan(&setup).expect("plan should derive");
    let mut bytes_written = 0_u64;
    for unit in &layout.units {
        let path = unit.pcs_setup_plan().expect("PCS plan path should derive");
        let plan = read_pcs_setup_plan_file(&path).expect("PCS plan should parse");
        bytes_written += fs::metadata(path)
            .expect("PCS plan output should exist")
            .len();
        assert_eq!(plan, expected);
    }
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!("status=ok\nunits={unit_count}\nbytes_written={bytes_written}\n")
    );
    assert!(stderr.is_empty());
}

#[test]
fn reports_usage_for_missing_pcs_directory_path() {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(&["setup", "write-pcs-directory"], &mut stdout, &mut stderr);

    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "usage: lzvm setup write-pcs-directory <setup-dir>\n"
    );
}

#[test]
fn writes_pcs_setup_materials_for_all_units() {
    let (dir, _) = create_key_directory("pcs-material");

    let mut base_stdout = Vec::new();
    let mut base_stderr = Vec::new();
    let base_code = run_cli(
        &[
            "setup",
            "write-base-directory",
            dir.to_str().expect("directory path should be utf-8"),
        ],
        &mut base_stdout,
        &mut base_stderr,
    );
    assert_eq!(base_code, 0);
    assert!(base_stderr.is_empty());

    let mut plan_stdout = Vec::new();
    let mut plan_stderr = Vec::new();
    let plan_code = run_cli(
        &[
            "setup",
            "write-pcs-directory",
            dir.to_str().expect("directory path should be utf-8"),
        ],
        &mut plan_stdout,
        &mut plan_stderr,
    );
    assert_eq!(plan_code, 0);
    assert!(plan_stderr.is_empty());

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "setup",
            "write-pcs-material-directory",
            dir.to_str().expect("directory path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let unit_count = layout.units.len();
    let mut bytes_written = 0_u64;
    for unit in &layout.units {
        let setup =
            parse_unit_setup_info_json(sample_setup_info_json()).expect("setup should parse");
        let plan =
            read_pcs_setup_plan_file(unit.pcs_setup_plan().expect("PCS plan path should derive"))
                .expect("PCS plan should parse");
        let fixed = fs::read(&unit.fixed_columns).expect("fixed columns should read");
        let tree_bytes = fs::read(&unit.constant_tree).expect("constant tree should read");
        let tree =
            parse_constant_tree_bytes(tree_bytes, &setup).expect("constant tree should parse");
        let expected =
            build_pcs_setup_material(&plan, &fixed, &tree).expect("material should build");
        let path = unit
            .pcs_setup_material()
            .expect("PCS material path should derive");
        let material = read_pcs_setup_material_file(&path).expect("PCS material should parse");
        bytes_written += fs::metadata(path)
            .expect("PCS material output should exist")
            .len();
        assert_eq!(material, expected);
    }
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!("status=ok\nunits={unit_count}\nbytes_written={bytes_written}\n")
    );
    assert!(stderr.is_empty());
}

#[test]
fn reports_usage_for_missing_pcs_material_directory_path() {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &["setup", "write-pcs-material-directory"],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "usage: lzvm setup write-pcs-material-directory <setup-dir>\n"
    );
}

#[test]
#[cfg(feature = "cuda")]
fn writes_base_directory_with_cuda_backend_option() {
    let (cpu_dir, _) = create_key_directory("cpu");
    let (cuda_dir, _) = create_key_directory("cuda");

    let mut cpu_stdout = Vec::new();
    let mut cpu_stderr = Vec::new();
    let cpu_code = run_cli(
        &[
            "setup",
            "write-base-directory",
            cpu_dir.to_str().expect("directory path should be utf-8"),
        ],
        &mut cpu_stdout,
        &mut cpu_stderr,
    );
    let mut cuda_stdout = Vec::new();
    let mut cuda_stderr = Vec::new();
    let cuda_code = run_cli(
        &[
            "setup",
            "write-base-directory",
            "--backend",
            "cuda",
            cuda_dir.to_str().expect("directory path should be utf-8"),
        ],
        &mut cuda_stdout,
        &mut cuda_stderr,
    );

    let cpu_layout = read_key_directory_layout(&cpu_dir).expect("cpu layout should derive");
    let cuda_layout = read_key_directory_layout(&cuda_dir).expect("cuda layout should derive");
    for (cpu_unit, cuda_unit) in cpu_layout.units.iter().zip(&cuda_layout.units) {
        let cpu_tree = fs::read(&cpu_unit.constant_tree).expect("cpu tree should be written");
        let cuda_tree = fs::read(&cuda_unit.constant_tree).expect("cuda tree should be written");
        assert_eq!(cuda_tree, cpu_tree);
    }
    fs::remove_dir_all(&cpu_dir).expect("cpu fixture directory should be removed");
    fs::remove_dir_all(&cuda_dir).expect("cuda fixture directory should be removed");

    assert_eq!(cpu_code, 0);
    assert_eq!(cuda_code, 0);
    assert!(cpu_stderr.is_empty());
    assert!(cuda_stderr.is_empty());
}

#[test]
fn reports_usage_for_missing_base_directory_path() {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(&["setup", "write-base-directory"], &mut stdout, &mut stderr);

    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "usage: lzvm setup write-base-directory [--derive-verkey] [--backend cpu|cuda] <setup-dir>\n"
    );
}
