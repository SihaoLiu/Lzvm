use std::fs;
use std::path::{Path, PathBuf};

mod fixtures;

use fixtures::{sample_base_setup_info, sample_global_info};
use lzvm_artifacts::constant_tree::parse_constant_tree_bytes;
use lzvm_artifacts::fixed::{encode_raw_fixed_columns, FixedColumn, FixedColumns};
use lzvm_artifacts::key_directory::{
    GlobalKeyPaths, KeyDirectoryLayout, KeyUnitKind, KeyUnitPaths,
};
use lzvm_artifacts::pcs_material::{build_pcs_setup_material, read_pcs_setup_material_file};
use lzvm_artifacts::pcs_plan::{derive_pcs_setup_plan, read_pcs_setup_plan_file};
use lzvm_artifacts::setup_info::{encode_unit_setup_info, UnitSetupInfo};
use lzvm_setup::{
    build_constant_tree_from_fixed_columns, write_pcs_directory_from_layout,
    write_pcs_material_directory_from_layout, write_pcs_setup_material_file,
    write_pcs_setup_plan_file, PcsDirectoryWriteReport, PcsFileWriteReport,
};

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
        "lzvm-setup-pcs-directory-{}-{name}",
        std::process::id()
    ))
}

fn one_unit_layout(root: &Path) -> KeyDirectoryLayout {
    let prefix = root.join("unit");
    KeyDirectoryLayout {
        root: root.to_path_buf(),
        global_info: sample_global_info(),
        global_paths: GlobalKeyPaths {
            info: root.join("pilout.globalInfo.bin"),
            constraints_program: root.join("pilout.globalConstraints.bin"),
        },
        source_fixed_file_manifest: root.join("lzvm.source-fixed-file-manifest"),
        source_program_archive: root.join("lzvm.source-program-archive"),
        units: vec![KeyUnitPaths {
            kind: KeyUnitKind::Basic,
            group_id: Some(0),
            unit_id: Some(0),
            group_name: Some("group-a".to_owned()),
            unit_name: Some("unit-a".to_owned()),
            prefix: prefix.clone(),
            metadata_prefix: Some(prefix.clone()),
            program_prefix: None,
            verification_key_prefix: prefix.clone(),
            fixed_columns: root.join("unit.const"),
            constant_tree: root.join("unit.consttree"),
        }],
    }
}

fn write_bytes(path: &Path, bytes: impl AsRef<[u8]>) {
    fs::create_dir_all(path.parent().expect("path should have a parent"))
        .expect("fixture directory should be created");
    fs::write(path, bytes).expect("fixture should be written");
}

fn write_setup_fixture(layout: &KeyDirectoryLayout, setup: &UnitSetupInfo) {
    let unit = &layout.units[0];
    write_bytes(
        &unit.setup_info().expect("setup path should derive"),
        encode_unit_setup_info(setup).expect("setup metadata should encode"),
    );
}

#[test]
fn writes_pcs_plan_and_material_from_layout() {
    let dir = temp_dir("valid");
    let _ = fs::remove_dir_all(&dir);
    let layout = one_unit_layout(&dir);
    let unit = &layout.units[0];
    let setup = sample_base_setup_info();
    let columns = sample_columns();
    let fixed = encode_raw_fixed_columns(&columns, &setup).expect("fixed columns should encode");
    let tree_bytes =
        build_constant_tree_from_fixed_columns(&columns, &setup).expect("tree should build");
    let tree = parse_constant_tree_bytes(tree_bytes.clone(), &setup).expect("tree should parse");

    write_setup_fixture(&layout, &setup);
    write_bytes(&unit.fixed_columns, &fixed);
    write_bytes(&unit.constant_tree, &tree_bytes);

    let plan_report = write_pcs_directory_from_layout(&layout).expect("plan should write");
    let plan_path = unit.pcs_setup_plan().expect("plan path should derive");
    let plan = read_pcs_setup_plan_file(&plan_path).expect("plan should parse");
    let expected_plan = derive_pcs_setup_plan(&setup).expect("plan should derive");
    let plan_bytes = fs::metadata(&plan_path)
        .expect("plan output should exist")
        .len();
    assert_eq!(plan, expected_plan);
    assert_eq!(
        plan_report,
        PcsDirectoryWriteReport {
            unit_count: 1,
            bytes_written: plan_bytes
        }
    );

    let material_report =
        write_pcs_material_directory_from_layout(&layout).expect("material should write");
    let material_path = unit
        .pcs_setup_material()
        .expect("material path should derive");
    let material = read_pcs_setup_material_file(&material_path).expect("material should parse");
    let expected_material =
        build_pcs_setup_material(&plan, &fixed, &tree).expect("material should build");
    let material_bytes = fs::metadata(&material_path)
        .expect("material output should exist")
        .len();

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(material, expected_material);
    assert_eq!(
        material_report,
        PcsDirectoryWriteReport {
            unit_count: 1,
            bytes_written: material_bytes
        }
    );
}

#[test]
fn writes_pcs_plan_and_material_from_files() {
    let dir = temp_dir("files");
    let _ = fs::remove_dir_all(&dir);
    let layout = one_unit_layout(&dir);
    let unit = &layout.units[0];
    let setup = sample_base_setup_info();
    let columns = sample_columns();
    let fixed = encode_raw_fixed_columns(&columns, &setup).expect("fixed columns should encode");
    let tree_bytes =
        build_constant_tree_from_fixed_columns(&columns, &setup).expect("tree should build");
    let tree = parse_constant_tree_bytes(tree_bytes.clone(), &setup).expect("tree should parse");

    write_setup_fixture(&layout, &setup);
    write_bytes(&unit.fixed_columns, &fixed);
    write_bytes(&unit.constant_tree, &tree_bytes);

    let setup_path = unit.setup_info().expect("setup path should derive");
    let plan_path = dir.join("single.pcs-plan");
    let material_path = dir.join("single.pcs-material");
    let plan_report =
        write_pcs_setup_plan_file(&setup_path, &plan_path).expect("plan should write");
    let plan = read_pcs_setup_plan_file(&plan_path).expect("plan should parse");
    let plan_bytes = fs::metadata(&plan_path)
        .expect("plan output should exist")
        .len();
    assert_eq!(
        plan,
        derive_pcs_setup_plan(&setup).expect("plan should derive")
    );
    assert_eq!(
        plan_report,
        PcsFileWriteReport {
            path: plan_path.clone(),
            bytes_written: plan_bytes
        }
    );

    let material_report = write_pcs_setup_material_file(
        &setup_path,
        &plan_path,
        &unit.fixed_columns,
        &unit.constant_tree,
        &material_path,
    )
    .expect("material should write");
    let material = read_pcs_setup_material_file(&material_path).expect("material should parse");
    let expected_material =
        build_pcs_setup_material(&plan, &fixed, &tree).expect("material should build");
    let material_bytes = fs::metadata(&material_path)
        .expect("material output should exist")
        .len();

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(material, expected_material);
    assert_eq!(
        material_report,
        PcsFileWriteReport {
            path: material_path,
            bytes_written: material_bytes
        }
    );
}

#[test]
#[cfg(unix)]
fn pcs_file_writes_publish_by_replacing_the_output_path() {
    use std::os::unix::fs::symlink;

    let dir = temp_dir("files-symlink-output");
    let _ = fs::remove_dir_all(&dir);
    let layout = one_unit_layout(&dir);
    let unit = &layout.units[0];
    let setup = sample_base_setup_info();
    let columns = sample_columns();
    let fixed = encode_raw_fixed_columns(&columns, &setup).expect("fixed columns should encode");
    let tree_bytes =
        build_constant_tree_from_fixed_columns(&columns, &setup).expect("tree should build");

    write_setup_fixture(&layout, &setup);
    write_bytes(&unit.fixed_columns, &fixed);
    write_bytes(&unit.constant_tree, &tree_bytes);

    let setup_path = unit.setup_info().expect("setup path should derive");
    let plan_path = dir.join("single.pcs-plan");
    let material_path = dir.join("single.pcs-material");
    let plan_target = dir.join("plan-target.bin");
    let material_target = dir.join("material-target.bin");
    let sentinel = b"preserve existing symlink target";
    write_bytes(&plan_target, sentinel);
    write_bytes(&material_target, sentinel);
    symlink(&plan_target, &plan_path).expect("plan symlink should be created");
    symlink(&material_target, &material_path).expect("material symlink should be created");

    write_pcs_setup_plan_file(&setup_path, &plan_path).expect("plan should write");
    write_pcs_setup_material_file(
        &setup_path,
        &plan_path,
        &unit.fixed_columns,
        &unit.constant_tree,
        &material_path,
    )
    .expect("material should write");

    let plan = read_pcs_setup_plan_file(&plan_path).expect("plan should parse");
    let material = read_pcs_setup_material_file(&material_path).expect("material should parse");
    let tree = parse_constant_tree_bytes(tree_bytes, &setup).expect("tree should parse");
    let expected_material =
        build_pcs_setup_material(&plan, &fixed, &tree).expect("material should build");

    assert_eq!(
        fs::read(&plan_target).expect("target should read"),
        sentinel
    );
    assert_eq!(
        fs::read(&material_target).expect("target should read"),
        sentinel
    );
    assert!(fs::read_link(&plan_path).is_err());
    assert!(fs::read_link(&material_path).is_err());
    assert_eq!(material, expected_material);

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}
