use std::fs;
use std::path::{Path, PathBuf};

use lzvm_artifacts::source_fixed_file_manifest::read_source_fixed_file_manifest_file;
use lzvm_cli::run_cli;

fn temp_dir(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root should resolve")
        .join("temp")
        .join(format!(
            "lzvm-cli-setup-source-fixed-file-manifest-{}-{name}",
            std::process::id()
        ))
}

fn write_file(path: &Path, contents: &str) {
    fs::create_dir_all(path.parent().expect("path should have a parent"))
        .expect("fixture directory should be created");
    fs::write(path, contents).expect("fixture should be written");
}

#[test]
fn writes_source_fixed_file_manifest_through_setup_namespace() {
    let dir = temp_dir("manifest");
    let _ = fs::remove_dir_all(&dir);
    let main_path = dir.join("main.pil");
    let output_path = dir.join("source-fixed-files.bin");
    write_file(
        &main_path,
        "airtemplate Main() {\n\
             #pragma fixed_load `values/${AIR_ID}.bin` 3\n\
         }\n\
         airgroup Main { Main(); }",
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "setup",
            "write-source-fixed-file-manifest",
            main_path.to_str().expect("main path should be utf-8"),
            output_path.to_str().expect("output path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0);
    let manifest = read_source_fixed_file_manifest_file(&output_path)
        .expect("source fixed-file manifest should parse");
    let bytes_written = fs::metadata(&output_path)
        .expect("output should exist")
        .len();
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\nbytes_written={}\nmodules=1\nfixed_file_pragmas=1\nair_template_fixed_file_pragmas=1\nair_units=1\nentries=1\noutput={}\n",
            bytes_written,
            output_path.display()
        )
    );
    assert!(stderr.is_empty());
    assert_eq!(manifest.entries.len(), 1);
    assert_eq!(manifest.entries[0].path.as_deref(), Some("values/0.bin"));
    assert_eq!(manifest.entries[0].column, Some(3));

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}
