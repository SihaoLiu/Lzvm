use std::fs;
use std::path::PathBuf;

use lzvm_artifacts::fixed::{
    encode_fixed_columns, encode_raw_fixed_columns, FixedColumn, FixedColumns,
};
use lzvm_artifacts::setup_info::{encode_unit_setup_info, parse_unit_setup_info_json};
use lzvm_artifacts::verification_key::VerificationKeyRoot;
use lzvm_cli::run_cli;
use lzvm_setup::build_constant_tree_from_fixed_columns;

fn sample_setup_info_json() -> &'static str {
    r#"{
        "nStages": 1,
        "nConstants": 2,
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
            "nQueries": 2,
            "steps": [
                {"nBits": 2},
                {"nBits": 1}
            ],
            "hashCommits": true,
            "lastLevelVerification": 2,
            "powBits": 0,
            "merkleTreeArity": 2,
            "verificationHashType": "GL",
            "transcriptArity": 2,
            "merkleTreeCustom": true
        }
    }"#
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
        "lzvm-cli-write-base-native-{}-{name}",
        std::process::id()
    ))
}

fn root_from_tree(tree: &[u8]) -> VerificationKeyRoot {
    VerificationKeyRoot::FieldElements(
        tree[tree.len() - 32..]
            .chunks_exact(8)
            .map(|chunk| u64::from_le_bytes(chunk.try_into().expect("slice length checked")))
            .collect(),
    )
}

fn format_root(root: VerificationKeyRoot) -> String {
    match root {
        VerificationKeyRoot::FieldElements(values) => values
            .into_iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>()
            .join(","),
        VerificationKeyRoot::DecimalScalar(value) => value,
    }
}

#[test]
fn writes_base_fixed_columns_and_constant_tree_from_native_inputs() {
    let dir = temp_dir("valid");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let setup_path = dir.join("unit.setup.bin");
    let columns_path = dir.join("unit.fixed.bin");
    let out_const = dir.join("unit.const");
    let out_consttree = dir.join("unit.consttree");
    let setup = parse_unit_setup_info_json(sample_setup_info_json()).expect("setup should parse");
    let columns = sample_columns();
    let expected_const =
        encode_raw_fixed_columns(&columns, &setup).expect("raw fixed should encode");
    let expected_tree =
        build_constant_tree_from_fixed_columns(&columns, &setup).expect("tree should build");
    let expected_root = root_from_tree(&expected_tree);
    fs::write(
        &setup_path,
        encode_unit_setup_info(&setup).expect("setup should encode"),
    )
    .expect("setup fixture should be written");
    fs::write(
        &columns_path,
        encode_fixed_columns(&columns).expect("columns should encode"),
    )
    .expect("columns fixture should be written");

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "setup",
            "write-base-native",
            setup_path.to_str().expect("setup path should be utf-8"),
            columns_path.to_str().expect("columns path should be utf-8"),
            out_const
                .to_str()
                .expect("fixed output path should be utf-8"),
            out_consttree
                .to_str()
                .expect("tree output path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    let fixed_bytes = fs::read(&out_const).expect("fixed output should be written");
    let tree_bytes = fs::read(&out_consttree).expect("tree output should be written");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0);
    assert_eq!(fixed_bytes, expected_const);
    assert_eq!(tree_bytes, expected_tree);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\nfixed_bytes=32\ntree_bytes=288\nroot={}\nfixed_output={}\ntree_output={}\n",
            format_root(expected_root),
            out_const.display(),
            out_consttree.display()
        )
    );
    assert!(stderr.is_empty());
}

#[test]
#[cfg(feature = "cuda")]
fn writes_base_outputs_with_cuda_backend_option() {
    let dir = temp_dir("cuda");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let setup_path = dir.join("unit.setup.bin");
    let columns_path = dir.join("unit.fixed.bin");
    let cpu_const = dir.join("unit.cpu.const");
    let cpu_tree = dir.join("unit.cpu.consttree");
    let cuda_const = dir.join("unit.cuda.const");
    let cuda_tree = dir.join("unit.cuda.consttree");
    let setup = parse_unit_setup_info_json(sample_setup_info_json()).expect("setup should parse");
    fs::write(
        &setup_path,
        encode_unit_setup_info(&setup).expect("setup should encode"),
    )
    .expect("setup fixture should be written");
    fs::write(
        &columns_path,
        encode_fixed_columns(&sample_columns()).expect("columns should encode"),
    )
    .expect("columns fixture should be written");

    let mut cpu_stdout = Vec::new();
    let mut cpu_stderr = Vec::new();
    let cpu_code = run_cli(
        &[
            "setup",
            "write-base-native",
            setup_path.to_str().expect("setup path should be utf-8"),
            columns_path.to_str().expect("columns path should be utf-8"),
            cpu_const
                .to_str()
                .expect("cpu fixed output path should be utf-8"),
            cpu_tree
                .to_str()
                .expect("cpu tree output path should be utf-8"),
        ],
        &mut cpu_stdout,
        &mut cpu_stderr,
    );
    let mut cuda_stdout = Vec::new();
    let mut cuda_stderr = Vec::new();
    let cuda_code = run_cli(
        &[
            "setup",
            "write-base-native",
            "--backend",
            "cuda",
            setup_path.to_str().expect("setup path should be utf-8"),
            columns_path.to_str().expect("columns path should be utf-8"),
            cuda_const
                .to_str()
                .expect("cuda fixed output path should be utf-8"),
            cuda_tree
                .to_str()
                .expect("cuda tree output path should be utf-8"),
        ],
        &mut cuda_stdout,
        &mut cuda_stderr,
    );

    let cpu_const_bytes = fs::read(&cpu_const).expect("cpu fixed output should be written");
    let cpu_tree_bytes = fs::read(&cpu_tree).expect("cpu tree output should be written");
    let cuda_const_bytes = fs::read(&cuda_const).expect("cuda fixed output should be written");
    let cuda_tree_bytes = fs::read(&cuda_tree).expect("cuda tree output should be written");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(cpu_code, 0);
    assert_eq!(cuda_code, 0);
    assert!(cpu_stderr.is_empty());
    assert!(cuda_stderr.is_empty());
    assert_eq!(cuda_const_bytes, cpu_const_bytes);
    assert_eq!(cuda_tree_bytes, cpu_tree_bytes);
}

#[test]
fn reports_usage_for_missing_base_output_path() {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "setup",
            "write-base-native",
            "unit.setup.bin",
            "unit.fixed.bin",
            "unit.const",
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "usage: lzvm setup write-base-native [--backend cpu|cuda] <setup-info-bin> <columns-bin> <out-const> <out-consttree>\n"
    );
}
