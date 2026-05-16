use std::fs;
use std::path::PathBuf;

use lzvm_artifacts::constant_tree::parse_constant_tree_bytes;
use lzvm_artifacts::fixed::{encode_raw_fixed_columns, FixedColumn, FixedColumns};
use lzvm_artifacts::pcs_material::{build_pcs_setup_material, read_pcs_setup_material_file};
use lzvm_artifacts::pcs_plan::{derive_pcs_setup_plan, encode_pcs_setup_plan};
use lzvm_artifacts::setup_info::encode_unit_setup_info;
use lzvm_cli::run_cli;
use lzvm_setup::build_constant_tree_from_fixed_columns;

mod fixtures;

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
        "lzvm-cli-write-pcs-material-{}-{name}",
        std::process::id()
    ))
}

#[test]
fn writes_pcs_setup_material_from_native_artifacts() {
    let dir = temp_dir("valid");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let setup_path = dir.join("unit.setup.bin");
    let plan_path = dir.join("unit.pcs-plan");
    let fixed_path = dir.join("unit.const");
    let tree_path = dir.join("unit.consttree");
    let material_path = dir.join("unit.pcs-material");

    let setup = fixtures::sample_setup_info();
    let plan = derive_pcs_setup_plan(&setup).expect("plan should derive");
    let columns = sample_columns();
    let fixed = encode_raw_fixed_columns(&columns, &setup).expect("fixed columns should encode");
    let tree_bytes =
        build_constant_tree_from_fixed_columns(&columns, &setup).expect("tree should build");
    let tree = parse_constant_tree_bytes(tree_bytes.clone(), &setup).expect("tree should parse");
    let expected = build_pcs_setup_material(&plan, &fixed, &tree).expect("material should build");

    fs::write(
        &setup_path,
        encode_unit_setup_info(&setup).expect("setup should encode"),
    )
    .expect("setup fixture should be written");
    fs::write(
        &plan_path,
        encode_pcs_setup_plan(&plan).expect("plan should encode"),
    )
    .expect("plan fixture should be written");
    fs::write(&fixed_path, &fixed).expect("fixed fixture should be written");
    fs::write(&tree_path, &tree_bytes).expect("tree fixture should be written");

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "setup",
            "write-pcs-material",
            setup_path.to_str().expect("setup path should be utf-8"),
            plan_path.to_str().expect("plan path should be utf-8"),
            fixed_path.to_str().expect("fixed path should be utf-8"),
            tree_path.to_str().expect("tree path should be utf-8"),
            material_path
                .to_str()
                .expect("material path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    let actual =
        read_pcs_setup_material_file(&material_path).expect("PCS material output should parse");
    let byte_count = fs::metadata(&material_path)
        .expect("PCS material output should exist")
        .len();
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0);
    assert_eq!(actual, expected);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\nbytes_written={byte_count}\noutput={}\n",
            material_path.display()
        )
    );
    assert!(stderr.is_empty());
}

#[test]
fn reports_usage_for_missing_pcs_setup_material_output_path() {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "setup",
            "write-pcs-material",
            "unit.setup.bin",
            "unit.pcs-plan",
            "unit.const",
            "unit.consttree",
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "usage: lzvm setup write-pcs-material <setup-info-bin> <pcs-plan> <fixed-const> <consttree> <out-pcs-material>\n"
    );
}

#[test]
fn rejects_pcs_setup_material_with_mismatched_plan() {
    let dir = temp_dir("mismatched-plan");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let setup_path = dir.join("unit.setup.bin");
    let plan_path = dir.join("unit.pcs-plan");
    let fixed_path = dir.join("unit.const");
    let tree_path = dir.join("unit.consttree");
    let material_path = dir.join("unit.pcs-material");

    let setup = fixtures::sample_setup_info();
    let mut plan = derive_pcs_setup_plan(&setup).expect("plan should derive");
    plan.query_count += 1;
    let columns = sample_columns();
    let fixed = encode_raw_fixed_columns(&columns, &setup).expect("fixed columns should encode");
    let tree_bytes =
        build_constant_tree_from_fixed_columns(&columns, &setup).expect("tree should build");

    fs::write(
        &setup_path,
        encode_unit_setup_info(&setup).expect("setup should encode"),
    )
    .expect("setup fixture should be written");
    fs::write(
        &plan_path,
        encode_pcs_setup_plan(&plan).expect("plan should encode"),
    )
    .expect("plan fixture should be written");
    fs::write(&fixed_path, &fixed).expect("fixed fixture should be written");
    fs::write(&tree_path, &tree_bytes).expect("tree fixture should be written");

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "setup",
            "write-pcs-material",
            setup_path.to_str().expect("setup path should be utf-8"),
            plan_path.to_str().expect("plan path should be utf-8"),
            fixed_path.to_str().expect("fixed path should be utf-8"),
            tree_path.to_str().expect("tree path should be utf-8"),
            material_path
                .to_str()
                .expect("material path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    let material_exists = material_path.exists();
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert!(!material_exists);
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "setup PCS material write failed: PCS setup plan does not match setup metadata\n"
    );
}
