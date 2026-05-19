use std::fs;
use std::path::{Path, PathBuf};

use lzvm_artifacts::constant_tree::parse_constant_tree_bytes;
use lzvm_artifacts::constraint_program::{
    encode_global_constraint_program, encode_regular_constraint_program, ConstraintEntry,
    ConstraintProgram, GlobalConstraintProgram,
};
use lzvm_artifacts::expression_info::{
    encode_expression_info, read_expression_info_binary_file, ExpressionInfo,
};
use lzvm_artifacts::expression_program::{
    encode_expression_program, ExpressionEntry, ExpressionProgram,
};
use lzvm_artifacts::fixed::{encode_raw_fixed_columns, FixedColumn, FixedColumns};
use lzvm_artifacts::global_info::{encode_global_info, read_global_info_binary_file, GlobalInfo};
use lzvm_artifacts::hint_program::{
    encode_global_hint_program, read_regular_hint_program_file, HintOperand, HintProgram,
};
use lzvm_artifacts::key_directory::{
    key_directory_catalog_digest, key_directory_catalog_digest_hex, read_key_directory_catalog,
    read_key_directory_layout, KeyUnitPaths,
};
use lzvm_artifacts::pcs_material::{build_pcs_setup_material, read_pcs_setup_material_file};
use lzvm_artifacts::pcs_plan::{derive_pcs_setup_plan, read_pcs_setup_plan_file};
use lzvm_artifacts::sectioned::{encode_sectioned_file, parse_sectioned_file, SectionedFile};
use lzvm_artifacts::setup_info::{
    encode_unit_setup_info, read_unit_setup_info_binary_file, UnitSetupInfo,
};
use lzvm_artifacts::setup_manifest::{
    read_setup_directory_manifest_file, SETUP_DIRECTORY_MANIFEST_FILE,
};
use lzvm_artifacts::verification_key::{
    encode_verification_key_binary, read_verification_key_binary_file, VerificationKeyRoot,
};
use lzvm_artifacts::verifier_info::{
    encode_verifier_info, read_verifier_info_binary_file, VerifierInfo,
};
use lzvm_cli::run_cli;
use lzvm_setup::build_constant_tree_from_fixed_columns;

mod fixtures;

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

fn write_unit_setup_metadata(path: &Path, setup: &UnitSetupInfo) {
    let bytes = encode_unit_setup_info(setup).expect("setup metadata should encode");
    write_bytes(path, bytes);
}

fn write_expression_metadata(path: &Path, expressions: &ExpressionInfo) {
    let bytes = encode_expression_info(expressions).expect("expression metadata should encode");
    write_bytes(path, bytes);
}

fn write_verifier_metadata(path: &Path, verifier: &VerifierInfo) {
    let bytes = encode_verifier_info(verifier).expect("verifier metadata should encode");
    write_bytes(path, bytes);
}

fn write_global_metadata(path: &Path, info: &GlobalInfo) {
    let bytes = encode_global_info(info).expect("global metadata should encode");
    write_bytes(path, bytes);
}

fn root_from_tree(tree: &[u8]) -> VerificationKeyRoot {
    VerificationKeyRoot::FieldElements(
        tree[tree.len() - 32..]
            .chunks_exact(8)
            .map(|chunk| u64::from_le_bytes(chunk.try_into().expect("slice length checked")))
            .collect(),
    )
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
    write_global_metadata(
        &root.join("pilout.globalInfo.bin"),
        &fixtures::sample_global_info(),
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

fn write_unit_files(
    paths: &KeyUnitPaths,
    setup: &UnitSetupInfo,
    raw_fixed: &[u8],
    root: &VerificationKeyRoot,
) {
    if let Some(path) = paths.setup_info_binary() {
        write_unit_setup_metadata(&path, setup);
    }
    if let Some(path) = paths.expression_info_binary() {
        write_expression_metadata(&path, &fixtures::sample_expression_info_with_hint());
    }
    if let Some(path) = paths.verifier_info_binary() {
        write_verifier_metadata(&path, &fixtures::sample_verifier_info());
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

    let setup = fixtures::sample_setup_info();
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
    let expected = fixtures::sample_setup_info();
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
    let expected = fixtures::sample_verifier_info();
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
    let expected = fixtures::sample_expression_info_with_hint();
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
fn writes_base_directory_regular_hint_programs_from_expression_metadata() {
    let (dir, _) = create_key_directory("regular-hints");

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
    for unit in &layout.units {
        let path = unit
            .expression_program()
            .expect("expression program path should derive");
        let hints = read_regular_hint_program_file(path).expect("hint program should parse");
        assert_eq!(hints.hints.len(), 1);
        assert_eq!(hints.hints[0].name, "hint-a");
        assert_eq!(
            hints.hints[0].fields[0].values[0].operand,
            HintOperand::Commitment {
                id: 0,
                row_offset_index: 0
            }
        );
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
    let setup = fixtures::sample_setup_info();
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
        let setup = fixtures::sample_setup_info();
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
fn writes_key_directory_outputs_with_one_command() {
    let (dir, root) = create_key_directory("key-directory");
    remove_verification_keys(&dir);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "setup",
            "write-key-directory",
            dir.to_str().expect("directory path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0);

    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let unit_count = layout.units.len();
    let setup = fixtures::sample_setup_info();
    let expected_plan = derive_pcs_setup_plan(&setup).expect("plan should derive");
    let mut fixed_bytes = 0_u64;
    let mut tree_bytes = 0_u64;
    let mut verkey_bytes = 0_u64;
    let mut pcs_plan_bytes = 0_u64;
    let mut pcs_material_bytes = 0_u64;

    for unit in &layout.units {
        let tree = fs::read(&unit.constant_tree).expect("constant tree should be written");
        let binary_root =
            read_verification_key_binary_file(unit.verification_key_binary()).expect("binary key");
        let fixed = fs::read(&unit.fixed_columns).expect("fixed columns should read");
        let parsed_tree =
            parse_constant_tree_bytes(tree.clone(), &setup).expect("constant tree should parse");
        let plan_path = unit.pcs_setup_plan().expect("PCS plan path should derive");
        let plan = read_pcs_setup_plan_file(&plan_path).expect("PCS plan should parse");
        let material_path = unit
            .pcs_setup_material()
            .expect("PCS material path should derive");
        let material =
            read_pcs_setup_material_file(&material_path).expect("PCS material should parse");
        let expected_material =
            build_pcs_setup_material(&plan, &fixed, &parsed_tree).expect("material should build");

        fixed_bytes += fs::metadata(&unit.fixed_columns)
            .expect("fixed output should exist")
            .len();
        tree_bytes += u64::try_from(tree.len()).expect("tree length should fit");
        verkey_bytes += fs::metadata(unit.verification_key_binary())
            .expect("binary key should exist")
            .len();
        pcs_plan_bytes += fs::metadata(plan_path)
            .expect("PCS plan output should exist")
            .len();
        pcs_material_bytes += fs::metadata(material_path)
            .expect("PCS material output should exist")
            .len();

        assert_eq!(root_from_tree(&tree), root);
        assert_eq!(binary_root, root);
        assert_eq!(plan, expected_plan);
        assert_eq!(material, expected_material);
    }
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let manifest_path = dir.join(SETUP_DIRECTORY_MANIFEST_FILE);
    let manifest =
        read_setup_directory_manifest_file(&manifest_path).expect("manifest should parse");
    let manifest_bytes = fs::metadata(&manifest_path)
        .expect("manifest output should exist")
        .len();
    let setup_hash = key_directory_catalog_digest_hex(&catalog).expect("digest should encode");
    assert_eq!(manifest.unit_count, unit_count as u64);
    assert_eq!(manifest.global_constraint_count, 0);
    assert_eq!(manifest.fixed_byte_count, fixed_bytes);
    assert_eq!(manifest.pcs_material_unit_count, unit_count as u64);
    assert_eq!(manifest.pcs_material_byte_count, pcs_material_bytes);
    assert_eq!(
        manifest.catalog_digest,
        key_directory_catalog_digest(&catalog).expect("digest should compute")
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\nunits={unit_count}\nfixed_bytes={fixed_bytes}\ntree_bytes={tree_bytes}\nverkey_bytes={verkey_bytes}\npcs_plan_bytes={pcs_plan_bytes}\npcs_material_bytes={pcs_material_bytes}\nmanifest_bytes={manifest_bytes}\nsetup_hash={setup_hash}\nsetup_directory_manifest={}\n",
            manifest_path.display()
        )
    );
    assert!(stderr.is_empty());
}

#[test]
fn reports_usage_for_missing_key_directory_path() {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(&["setup", "write-key-directory"], &mut stdout, &mut stderr);

    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "usage: lzvm setup write-key-directory [--backend cpu|cuda] <setup-dir>\n"
    );
}

#[test]
fn generates_key_directory_outputs_with_public_command() {
    let (dir, _) = create_key_directory("generate-key");
    remove_verification_keys(&dir);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "setup",
            "generate-key",
            dir.to_str().expect("directory path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0);

    let layout = read_key_directory_layout(&dir).expect("layout should derive");
    let unit_count = layout.units.len();
    for unit in &layout.units {
        assert!(unit.verification_key_binary().is_file());
        assert!(unit
            .pcs_setup_plan()
            .expect("PCS plan path should derive")
            .is_file());
        assert!(unit
            .pcs_setup_material()
            .expect("PCS material path should derive")
            .is_file());
    }
    let manifest_path = dir.join(SETUP_DIRECTORY_MANIFEST_FILE);
    assert!(manifest_path.is_file());
    let catalog = read_key_directory_catalog(&dir).expect("catalog should load");
    let setup_hash = key_directory_catalog_digest_hex(&catalog).expect("digest should encode");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    let stdout = String::from_utf8(stdout).expect("stdout should be utf-8");
    assert!(stdout.starts_with(&format!("status=ok\nunits={unit_count}\n")));
    assert!(stdout.contains("pcs_plan_bytes="));
    assert!(stdout.contains("pcs_material_bytes="));
    assert!(stdout.contains("manifest_bytes="));
    assert!(stdout.contains(&format!("setup_hash={setup_hash}\n")));
    assert!(stdout.contains(&format!(
        "setup_directory_manifest={}\n",
        manifest_path.display()
    )));
    assert!(stderr.is_empty());
}

#[test]
fn reports_usage_for_missing_generate_key_directory_path() {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(&["setup", "generate-key"], &mut stdout, &mut stderr);

    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "usage: lzvm setup generate-key [--backend cpu|cuda] <setup-dir>\n"
    );
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
#[cfg(feature = "cuda")]
fn generates_key_directory_with_cuda_backend_option() {
    let (cpu_dir, _) = create_key_directory("generate-cpu");
    let (cuda_dir, _) = create_key_directory("generate-cuda");
    remove_verification_keys(&cpu_dir);
    remove_verification_keys(&cuda_dir);

    let mut cpu_stdout = Vec::new();
    let mut cpu_stderr = Vec::new();
    let cpu_code = run_cli(
        &[
            "setup",
            "generate-key",
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
            "generate-key",
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
        assert_eq!(
            read_verification_key_binary_file(cpu_unit.verification_key_binary())
                .expect("cpu key should parse"),
            read_verification_key_binary_file(cuda_unit.verification_key_binary())
                .expect("cuda key should parse")
        );
        assert_eq!(
            read_pcs_setup_plan_file(cpu_unit.pcs_setup_plan().expect("cpu PCS plan"))
                .expect("cpu PCS plan should parse"),
            read_pcs_setup_plan_file(cuda_unit.pcs_setup_plan().expect("cuda PCS plan"))
                .expect("cuda PCS plan should parse")
        );
        assert_eq!(
            read_pcs_setup_material_file(cpu_unit.pcs_setup_material().expect("cpu material"))
                .expect("cpu material should parse"),
            read_pcs_setup_material_file(cuda_unit.pcs_setup_material().expect("cuda material"))
                .expect("cuda material should parse")
        );
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
