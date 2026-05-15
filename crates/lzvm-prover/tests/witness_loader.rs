use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use lzvm_prover::witness_loader::{load_witness_library, WitnessLoadError, WITNESS_ABI_VERSION};

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("lzvm-witness-loader-{}-{name}", std::process::id()))
}

fn build_shared_library(dir: &Path, name: &str, source: &str) -> PathBuf {
    fs::create_dir_all(dir).expect("fixture directory should be created");
    let source_path = dir.join(format!("{name}.c"));
    let library_path = dir.join(format!("lib{name}.so"));
    fs::write(&source_path, source).expect("fixture source should be written");
    let status = Command::new("cc")
        .arg("-shared")
        .arg("-fPIC")
        .arg(&source_path)
        .arg("-o")
        .arg(&library_path)
        .status()
        .expect("cc should run");
    assert!(status.success(), "cc should build the fixture library");
    library_path
}

#[test]
fn loads_witness_library_abi_version() {
    let dir = temp_dir("valid");
    let _ = fs::remove_dir_all(&dir);
    let library_path = build_shared_library(
        &dir,
        "valid",
        "unsigned int lzvm_witness_abi_version(void) { return 1; }\nint lzvm_witness_compute(void) { return 7; }\n",
    );

    let library = load_witness_library(&library_path).expect("witness library should load");
    let compute_result = unsafe { library.call_compute_for_smoke() };
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(library.path, library_path);
    assert_eq!(library.abi_version, WITNESS_ABI_VERSION);
    assert_eq!(compute_result, 7);
}

#[test]
fn rejects_witness_library_without_abi_version() {
    let dir = temp_dir("missing-version");
    let _ = fs::remove_dir_all(&dir);
    let library_path = build_shared_library(
        &dir,
        "missing",
        "unsigned int other_symbol(void) { return 1; }\n",
    );

    let result = load_witness_library(&library_path);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert!(matches!(
        result,
        Err(WitnessLoadError::MissingAbiVersion { path, .. }) if path == library_path
    ));
}

#[test]
fn rejects_witness_library_with_unsupported_abi_version() {
    let dir = temp_dir("bad-version");
    let _ = fs::remove_dir_all(&dir);
    let library_path = build_shared_library(
        &dir,
        "bad",
        "unsigned int lzvm_witness_abi_version(void) { return 999; }\n",
    );

    let result = load_witness_library(&library_path);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert!(matches!(
        result,
        Err(WitnessLoadError::UnsupportedAbiVersion {
            path,
            expected: WITNESS_ABI_VERSION,
            found: 999
        }) if path == library_path
    ));
}

#[test]
fn rejects_witness_library_without_compute_symbol() {
    let dir = temp_dir("missing-compute");
    let _ = fs::remove_dir_all(&dir);
    let library_path = build_shared_library(
        &dir,
        "missing_compute",
        "unsigned int lzvm_witness_abi_version(void) { return 1; }\n",
    );

    let result = load_witness_library(&library_path);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert!(matches!(
        result,
        Err(WitnessLoadError::MissingCompute { path, .. }) if path == library_path
    ));
}
