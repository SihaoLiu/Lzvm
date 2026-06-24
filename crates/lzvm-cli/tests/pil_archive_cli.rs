use std::fs;
use std::path::{Path, PathBuf};

use lzvm_artifacts::source_program::read_source_program_archive_file;
use lzvm_cli::run_cli;

fn temp_dir(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root should resolve")
        .join("temp")
        .join(format!(
            "lzvm-cli-pil-archive-{}-{name}",
            std::process::id()
        ))
}

fn write_file(path: &Path, contents: &str) {
    fs::create_dir_all(path.parent().expect("path should have a parent"))
        .expect("fixture directory should be created");
    fs::write(path, contents).expect("fixture should be written");
}

fn target_temp_dir(name: &str) -> PathBuf {
    std::env::current_dir()
        .expect("current directory should be available")
        .join("..")
        .join("..")
        .join("temp")
        .join(format!(
            "lzvm-cli-pil-archive-{}-{name}",
            std::process::id()
        ))
}

fn relative_to_current_dir(path: &Path) -> PathBuf {
    let current_dir = std::env::current_dir().expect("current directory should be available");
    path.strip_prefix(&current_dir)
        .expect("path should be inside current directory")
        .to_path_buf()
}

#[test]
fn writes_source_program_archive_from_static_sources() {
    let dir = temp_dir("archive");
    let _ = fs::remove_dir_all(&dir);
    let main_path = dir.join("main.pil");
    let child_path = dir.join("shared.pil");
    let output_path = dir.join("source-program.bin");
    write_file(
        &main_path,
        "include \"shared.pil\";\n\
         container air.main;\n\
         airtemplate Main() {\n\
             #pragma output_fixed_file `${AIR_NAME}.fixed`\n\
         }\n\
         airgroup Main { Main(); }",
    );
    write_file(&child_path, "col fixed shared = [1, 2];");

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "pil",
            "archive",
            main_path.to_str().expect("main path should be utf-8"),
            output_path.to_str().expect("output path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0);
    let archive = read_source_program_archive_file(&output_path)
        .expect("source program archive should parse");
    let bytes_written = fs::metadata(&output_path)
        .expect("output should exist")
        .len();
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\nbytes_written={}\nsources=2\nedges=1\nmodules=2\nfixed_file_pragmas=1\nair_template_fixed_file_pragmas=1\nair_units=1\noutput={}\n",
            bytes_written,
            output_path.display()
        )
    );
    assert!(stderr.is_empty());
    assert_eq!(archive.sources.len(), 2);
    assert_eq!(archive.edges.len(), 1);
    assert_eq!(archive.sources[0].source_name, "main.pil");
    assert_eq!(archive.sources[1].source_name, "shared.pil");

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn writes_nested_include_path_sources_relative_to_include_root() {
    let dir = temp_dir("archive-include-root");
    let _ = fs::remove_dir_all(&dir);
    let source_dir = dir.join("source");
    let lib_dir = dir.join("lib");
    let main_path = source_dir.join("main.pil");
    let output_path = dir.join("source-program.bin");
    write_file(&main_path, "require \"entry.pil\";");
    write_file(&lib_dir.join("entry.pil"), "include \"nested/child.pil\";");
    write_file(&lib_dir.join("nested/child.pil"), "constant CHILD = 1;");

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "pil",
            "archive",
            "--include-path",
            lib_dir.to_str().expect("include path should be utf-8"),
            main_path.to_str().expect("main path should be utf-8"),
            output_path.to_str().expect("output path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    let archive = read_source_program_archive_file(&output_path)
        .expect("source program archive should parse");
    let source_names = archive
        .sources
        .iter()
        .map(|source| source.source_name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        source_names,
        vec!["main.pil", "entry.pil", "nested/child.pil"]
    );
    for source_name in source_names {
        assert!(!Path::new(source_name).is_absolute());
        assert!(!source_name.contains(dir.to_str().expect("dir should be utf-8")));
    }

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn deduplicates_require_sources_loaded_through_relative_include_paths() {
    let dir = target_temp_dir("relative-include-root");
    let _ = fs::remove_dir_all(&dir);
    let source_dir = dir.join("pil");
    let lib_dir = dir.join("lib");
    let main_path = source_dir.join("main.pil");
    let output_path = dir.join("source-program.bin");
    write_file(&main_path, "require \"ops.pil\";\nrequire \"child.pil\";");
    write_file(&source_dir.join("ops.pil"), "constant OPS = 1;");
    write_file(&lib_dir.join("child.pil"), "require \"ops.pil\";");

    let main_arg = relative_to_current_dir(&main_path);
    let source_arg = relative_to_current_dir(&source_dir);
    let lib_arg = relative_to_current_dir(&lib_dir);
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "pil",
            "archive",
            "--include-path",
            source_arg.to_str().expect("source path should be utf-8"),
            "--include-path",
            lib_arg.to_str().expect("library path should be utf-8"),
            main_arg.to_str().expect("main path should be utf-8"),
            output_path.to_str().expect("output path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    let archive = read_source_program_archive_file(&output_path)
        .expect("source program archive should parse");
    let source_names = archive
        .sources
        .iter()
        .map(|source| source.source_name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(source_names, vec!["main.pil", "ops.pil", "child.pil"]);
    assert_eq!(archive.edges.len(), 2);

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}
