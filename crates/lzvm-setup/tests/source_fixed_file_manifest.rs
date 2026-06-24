use std::fs;
use std::path::{Path, PathBuf};

use lzvm_artifacts::source_fixed_file_manifest::{
    read_source_fixed_file_manifest_file, SourceFixedFileManifestKind,
};
use lzvm_setup::{write_source_fixed_file_manifest, SourceFixedFileManifestWriteRequest};

fn temp_dir(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root should resolve")
        .join("temp")
        .join(format!(
            "lzvm-setup-source-fixed-file-manifest-{}-{name}",
            std::process::id()
        ))
}

fn write_file(path: &Path, contents: &str) {
    fs::create_dir_all(path.parent().expect("path should have a parent"))
        .expect("fixture directory should be created");
    fs::write(path, contents).expect("fixture should be written");
}

#[test]
fn writes_source_fixed_file_manifests_from_setup_requests() {
    let dir = temp_dir("write");
    let _ = fs::remove_dir_all(&dir);
    let main_path = dir.join("main.pil");
    let output_path = dir.join("source-fixed-files.bin");
    write_file(
        &main_path,
        "airtemplate Main(const string stem = \"main\") {\n\
             #pragma output_fixed_file `${AIRGROUP}/${AIR_ID}/${AIR_NAME}/${stem}.fixed`\n\
             #pragma fixed_load `${stem}.fixed` 2\n\
         }\n\
         airgroup Main {\n\
             Main();\n\
             virtual Main(stem: \"virtual\") alias VirtualMain;\n\
         }",
    );

    let report = write_source_fixed_file_manifest(&SourceFixedFileManifestWriteRequest {
        working_dir: dir.clone(),
        include_paths: Vec::new(),
        include_path_first: false,
        main_file: main_path.clone(),
        output_path: output_path.clone(),
    })
    .expect("manifest should be written");

    let manifest = read_source_fixed_file_manifest_file(&output_path)
        .expect("source fixed-file manifest should parse");
    let bytes_written = fs::metadata(&output_path)
        .expect("output should exist")
        .len();

    assert_eq!(report.output_path, output_path);
    assert_eq!(report.bytes_written, bytes_written);
    assert_eq!(report.module_count, 1);
    assert_eq!(report.fixed_file_pragma_count, 2);
    assert_eq!(report.air_template_fixed_file_pragma_count, 2);
    assert_eq!(report.air_unit_count, 2);
    assert_eq!(report.entry_count, 4);
    assert_eq!(manifest.entries.len(), 4);
    assert_eq!(
        manifest
            .entries
            .iter()
            .map(|entry| (
                entry.kind,
                entry.path.as_deref(),
                entry.column,
                entry.group_name.as_str(),
                entry.group_id,
                entry.unit_id,
                entry.unit_name.as_str(),
                entry.template_name.as_str(),
                entry.virtual_instance
            ))
            .collect::<Vec<_>>(),
        vec![
            (
                SourceFixedFileManifestKind::OutputFixedFile,
                Some("Main/0/Main/main.fixed"),
                None,
                "Main",
                0,
                0,
                "Main",
                "Main",
                false
            ),
            (
                SourceFixedFileManifestKind::FixedLoad,
                Some("main.fixed"),
                Some(2),
                "Main",
                0,
                0,
                "Main",
                "Main",
                false
            ),
            (
                SourceFixedFileManifestKind::OutputFixedFile,
                Some("Main/10000/VirtualMain/virtual.fixed"),
                None,
                "Main",
                0,
                10_000,
                "VirtualMain",
                "Main",
                true
            ),
            (
                SourceFixedFileManifestKind::FixedLoad,
                Some("virtual.fixed"),
                Some(2),
                "Main",
                0,
                10_000,
                "VirtualMain",
                "Main",
                true
            ),
        ]
    );

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
#[cfg(unix)]
fn source_fixed_file_manifest_write_replaces_output_paths() {
    use std::os::unix::fs::symlink;

    let dir = temp_dir("symlink-output");
    let _ = fs::remove_dir_all(&dir);
    let main_path = dir.join("main.pil");
    let output_path = dir.join("source-fixed-files.bin");
    let target_path = dir.join("target.bin");
    write_file(
        &main_path,
        "airtemplate Main(const string stem = \"main\") {\n\
             #pragma fixed_load `${stem}.fixed` 2\n\
         }\n\
         airgroup Main { Main(); }",
    );
    write_file(&target_path, "preserve source fixed-file target");
    symlink(&target_path, &output_path).expect("output symlink should be created");

    write_source_fixed_file_manifest(&SourceFixedFileManifestWriteRequest {
        working_dir: dir.clone(),
        include_paths: Vec::new(),
        include_path_first: false,
        main_file: main_path,
        output_path: output_path.clone(),
    })
    .expect("manifest should be written");

    read_source_fixed_file_manifest_file(&output_path)
        .expect("source fixed-file manifest should parse");
    assert!(fs::read_link(&output_path).is_err());
    assert_eq!(
        fs::read_to_string(&target_path).expect("target should read"),
        "preserve source fixed-file target"
    );

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}
