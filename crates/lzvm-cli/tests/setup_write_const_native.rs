use std::fs;
use std::path::PathBuf;

use lzvm_artifacts::constant_tree::read_constant_tree_file;
use lzvm_artifacts::fixed::{
    encode_fixed_columns, encode_raw_fixed_columns, FixedColumn, FixedColumns,
};
use lzvm_artifacts::setup_info::{encode_unit_setup_info, parse_unit_setup_info_json};
use lzvm_artifacts::verification_key::{encode_verification_key_binary, VerificationKeyRoot};
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
        "lzvm-cli-write-const-native-{}-{name}",
        std::process::id()
    ))
}

#[test]
fn writes_native_constant_tree_from_binary_setup_and_columns() {
    let dir = temp_dir("valid");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let setup_path = dir.join("unit.setup.bin");
    let columns_path = dir.join("unit.fixed.bin");
    let out_path = dir.join("unit.consttree");
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

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "setup",
            "write-const-native",
            setup_path.to_str().expect("setup path should be utf-8"),
            columns_path.to_str().expect("columns path should be utf-8"),
            out_path.to_str().expect("output path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    let tree = read_constant_tree_file(&out_path, &setup).expect("tree output should parse");
    let root = tree.root().expect("root should extract");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\nbytes_written=288\nroot={}\noutput={}\n",
            match root {
                lzvm_artifacts::verification_key::VerificationKeyRoot::FieldElements(values) =>
                    values
                        .into_iter()
                        .map(|value| value.to_string())
                        .collect::<Vec<_>>()
                        .join(","),
            },
            out_path.display()
        )
    );
    assert!(stderr.is_empty());
}

#[test]
fn writes_native_constant_tree_from_raw_columns_with_expected_root() {
    let dir = temp_dir("raw-root");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let setup_path = dir.join("unit.setup.bin");
    let columns_path = dir.join("unit.raw.bin");
    let root_path = dir.join("unit.root.bin");
    let out_path = dir.join("unit.consttree");
    let setup_json = sample_setup_info_json()
        .replace("\"merkleTreeArity\": 2", "\"merkleTreeArity\": 4")
        .replace("\"transcriptArity\": 2", "\"transcriptArity\": 4");
    let setup = parse_unit_setup_info_json(&setup_json).expect("setup should parse");
    let columns = sample_columns();
    let expected_tree =
        build_constant_tree_from_fixed_columns(&columns, &setup).expect("tree should build");
    let expected_root = VerificationKeyRoot::FieldElements(
        expected_tree[expected_tree.len() - 32..]
            .chunks_exact(8)
            .map(|chunk| u64::from_le_bytes(chunk.try_into().expect("slice length checked")))
            .collect(),
    );
    fs::write(
        &setup_path,
        encode_unit_setup_info(&setup).expect("setup should encode"),
    )
    .expect("setup fixture should be written");
    fs::write(
        &columns_path,
        encode_raw_fixed_columns(&columns, &setup).expect("raw columns should encode"),
    )
    .expect("columns fixture should be written");
    fs::write(
        &root_path,
        encode_verification_key_binary(&expected_root).expect("root should encode"),
    )
    .expect("root fixture should be written");

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "setup",
            "write-const-native",
            setup_path.to_str().expect("setup path should be utf-8"),
            columns_path.to_str().expect("columns path should be utf-8"),
            root_path.to_str().expect("root path should be utf-8"),
            out_path.to_str().expect("output path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    let tree = fs::read(&out_path).expect("tree output should be written");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0);
    assert_eq!(tree, expected_tree);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\nbytes_written=224\nroot={}\noutput={}\n",
            match expected_root {
                VerificationKeyRoot::FieldElements(values) => values
                    .into_iter()
                    .map(|value| value.to_string())
                    .collect::<Vec<_>>()
                    .join(","),
            },
            out_path.display()
        )
    );
    assert!(stderr.is_empty());
}

#[test]
#[cfg(feature = "cuda")]
fn writes_native_constant_tree_with_cuda_backend_option() {
    let dir = temp_dir("cuda");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let setup_path = dir.join("unit.setup.bin");
    let columns_path = dir.join("unit.fixed.bin");
    let cpu_out = dir.join("unit.cpu.consttree");
    let cuda_out = dir.join("unit.cuda.consttree");
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
            "write-const-native",
            setup_path.to_str().expect("setup path should be utf-8"),
            columns_path.to_str().expect("columns path should be utf-8"),
            cpu_out.to_str().expect("cpu output path should be utf-8"),
        ],
        &mut cpu_stdout,
        &mut cpu_stderr,
    );
    let mut cuda_stdout = Vec::new();
    let mut cuda_stderr = Vec::new();
    let cuda_code = run_cli(
        &[
            "setup",
            "write-const-native",
            "--backend",
            "cuda",
            setup_path.to_str().expect("setup path should be utf-8"),
            columns_path.to_str().expect("columns path should be utf-8"),
            cuda_out.to_str().expect("cuda output path should be utf-8"),
        ],
        &mut cuda_stdout,
        &mut cuda_stderr,
    );

    let cpu_tree = fs::read(&cpu_out).expect("cpu tree output should be written");
    let cuda_tree = fs::read(&cuda_out).expect("cuda tree output should be written");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(cpu_code, 0);
    assert_eq!(cuda_code, 0);
    assert!(cpu_stderr.is_empty());
    assert!(cuda_stderr.is_empty());
    assert_eq!(cuda_tree, cpu_tree);
}

#[test]
fn reports_usage_for_missing_native_constant_tree_output_path() {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "setup",
            "write-const-native",
            "unit.setup.bin",
            "unit.fixed.bin",
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "usage: lzvm setup write-const-native [--backend cpu|cuda] <setup-info-bin> <columns-bin> [root-bin] <out-consttree>\n"
    );
}
