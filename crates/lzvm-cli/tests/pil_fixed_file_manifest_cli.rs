use std::fs;
use std::path::{Path, PathBuf};

use lzvm_artifacts::source_fixed_file_manifest::read_source_fixed_file_manifest_file;
use lzvm_cli::run_cli;

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "lzvm-cli-pil-fixed-file-manifest-{}-{name}",
        std::process::id()
    ))
}

fn write_file(path: &Path, contents: &str) {
    fs::create_dir_all(path.parent().expect("path should have a parent"))
        .expect("fixture directory should be created");
    fs::write(path, contents).expect("fixture should be written");
}

#[test]
fn writes_source_fixed_file_manifest_from_static_sources() {
    let dir = temp_dir("manifest");
    let _ = fs::remove_dir_all(&dir);
    let main_path = dir.join("main.pil");
    let output_path = dir.join("source-fixed-files.bin");
    write_file(
        &main_path,
        "airtemplate Main() {\n\
             #pragma output_fixed_file `${AIR_NAME}.fixed`\n\
         }\n\
         airgroup Main { Main(); Main() alias Second; }",
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "pil",
            "fixed-file-manifest",
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
            "status=ok\nbytes_written={}\nmodules=1\nfixed_file_pragmas=1\nair_template_fixed_file_pragmas=1\nair_units=2\nentries=2\noutput={}\n",
            bytes_written,
            output_path.display()
        )
    );
    assert!(stderr.is_empty());
    assert_eq!(manifest.entries.len(), 2);
    assert_eq!(
        manifest
            .entries
            .iter()
            .map(|entry| entry.path.as_deref())
            .collect::<Vec<_>>(),
        vec![Some("Main.fixed"), Some("Second.fixed")]
    );

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}
