use std::fs;
use std::path::{Path, PathBuf};

mod fixtures;

use lzvm_artifacts::fixed::read_fixed_columns_file;
use lzvm_artifacts::setup_info::encode_unit_setup_info;
use lzvm_field::MODULUS;
use lzvm_setup::{write_fixed_columns_from_source_file, SourceFixedColumnsWriteRequest};

fn temp_dir(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root should resolve")
        .join("temp")
        .join(format!(
            "lzvm-setup-source-fixed-sequence-progressions-{}-{name}",
            std::process::id()
        ))
}

fn write_file(path: &Path, contents: impl AsRef<[u8]>) {
    fs::create_dir_all(path.parent().expect("path should have a parent"))
        .expect("fixture directory should be created");
    fs::write(path, contents).expect("fixture should be written");
}

#[test]
fn writes_fixed_column_source_artifacts_from_field_wrapped_open_mul_progressions() {
    let dir = temp_dir("field-wrapped-open-mul-progressions");
    let _ = fs::remove_dir_all(&dir);
    let setup = fixtures::sample_two_column_setup_info(2, 3, 1, 4);
    let setup_path = dir.join("unit.starkinfo.bin");
    let main_path = dir.join("main.pil");
    let output_path = dir.join("unit.fixed-source.bin");
    write_file(
        &setup_path,
        encode_unit_setup_info(&setup).expect("setup should encode"),
    );
    write_file(
        &main_path,
        format!(
            "col fixed main.left = [1, {}..*..];\n\
             col fixed main.right = [1, 2..*..];",
            MODULUS - 1
        ),
    );

    write_fixed_columns_from_source_file(&SourceFixedColumnsWriteRequest {
        working_dir: dir.clone(),
        include_paths: Vec::new(),
        include_path_first: false,
        main_file: main_path,
        setup_info_path: setup_path,
        group_name: "group-a".to_owned(),
        unit_name: "unit-a".to_owned(),
        output_path: output_path.clone(),
    })
    .expect("fixed columns should be written");

    let columns = read_fixed_columns_file(&output_path).expect("fixed columns should parse");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(
        columns.columns[0].values,
        vec![1, MODULUS - 1, 1, MODULUS - 1]
    );
    assert_eq!(columns.columns[1].values, vec![1, 2, 4, 8]);
}
