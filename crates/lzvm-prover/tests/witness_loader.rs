use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use lzvm_prover::witness_loader::{load_witness_library, WitnessLoadError, WITNESS_ABI_VERSION};
use lzvm_prover::witness_loader::{
    WitnessCall, WitnessCallError, WitnessResult, WitnessTraceBuffers, WITNESS_STATUS_OK,
};

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
        r#"#include <stddef.h>
typedef struct {
    const unsigned char *input_ptr;
    size_t input_len;
    unsigned char *output_ptr;
    size_t output_len;
} LzvmWitnessCall;
typedef struct {
    int status;
    size_t produced_len;
} LzvmWitnessResult;
unsigned int lzvm_witness_abi_version(void) { return 1; }
int lzvm_witness_compute(const LzvmWitnessCall *call, LzvmWitnessResult *result) {
    if (!call || !result || call->input_len == 0 || call->output_len == 0) {
        return -1;
    }
    result->status = 0;
    result->produced_len = 1;
    call->output_ptr[0] = (unsigned char)(call->input_ptr[0] + 1);
    return 0;
}
"#,
    );

    let library = load_witness_library(&library_path).expect("witness library should load");
    let input = [41_u8];
    let mut output = [0_u8];
    let call = WitnessCall {
        input_ptr: input.as_ptr(),
        input_len: input.len(),
        output_ptr: output.as_mut_ptr(),
        output_len: output.len(),
    };
    let mut result = WitnessResult::default();
    let compute_result = unsafe { library.compute_unchecked(&call, &mut result) };
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(library.path, library_path);
    assert_eq!(library.abi_version, WITNESS_ABI_VERSION);
    assert_eq!(compute_result, WITNESS_STATUS_OK);
    assert_eq!(result.status, WITNESS_STATUS_OK);
    assert_eq!(result.produced_len, 1);
    assert_eq!(output[0], 42);
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

#[test]
fn runs_witness_compute_with_owned_buffers() {
    let dir = temp_dir("owned-buffers");
    let _ = fs::remove_dir_all(&dir);
    let library_path = build_shared_library(
        &dir,
        "owned",
        r#"#include <stddef.h>
typedef struct {
    const unsigned char *input_ptr;
    size_t input_len;
    unsigned char *output_ptr;
    size_t output_len;
} LzvmWitnessCall;
typedef struct {
    int status;
    size_t produced_len;
} LzvmWitnessResult;
unsigned int lzvm_witness_abi_version(void) { return 1; }
int lzvm_witness_compute(const LzvmWitnessCall *call, LzvmWitnessResult *result) {
    if (!call || !result || call->input_len == 0 || call->output_len == 0) {
        return -1;
    }
    result->status = 0;
    result->produced_len = 1;
    call->output_ptr[0] = (unsigned char)(call->input_ptr[0] + 1);
    return 0;
}
"#,
    );

    let library = load_witness_library(&library_path).expect("witness library should load");
    let mut buffers =
        WitnessTraceBuffers::new(vec![41], 1).expect("witness buffers should allocate");
    let output = library
        .compute(&mut buffers)
        .expect("witness compute should run");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(output.produced_len, 1);
    assert_eq!(buffers.output(), &[42]);
}

#[test]
fn rejects_witness_compute_output_overflow() {
    let dir = temp_dir("overflow");
    let _ = fs::remove_dir_all(&dir);
    let library_path = build_shared_library(
        &dir,
        "overflow",
        r#"#include <stddef.h>
typedef struct {
    const unsigned char *input_ptr;
    size_t input_len;
    unsigned char *output_ptr;
    size_t output_len;
} LzvmWitnessCall;
typedef struct {
    int status;
    size_t produced_len;
} LzvmWitnessResult;
unsigned int lzvm_witness_abi_version(void) { return 1; }
int lzvm_witness_compute(const LzvmWitnessCall *call, LzvmWitnessResult *result) {
    if (!call || !result || call->output_len == 0) {
        return -1;
    }
    result->status = 0;
    result->produced_len = call->output_len + 1;
    return 0;
}
"#,
    );

    let library = load_witness_library(&library_path).expect("witness library should load");
    let mut buffers =
        WitnessTraceBuffers::new(vec![41], 1).expect("witness buffers should allocate");
    let result = library.compute(&mut buffers);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert!(matches!(
        result,
        Err(WitnessCallError::OutputOverflow {
            produced_len: 2,
            output_len: 1
        })
    ));
}

#[test]
fn rejects_witness_compute_return_failures() {
    let dir = temp_dir("return-failure");
    let _ = fs::remove_dir_all(&dir);
    let library_path = build_shared_library(
        &dir,
        "return_failure",
        r#"#include <stddef.h>
typedef struct {
    const unsigned char *input_ptr;
    size_t input_len;
    unsigned char *output_ptr;
    size_t output_len;
} LzvmWitnessCall;
typedef struct {
    int status;
    size_t produced_len;
} LzvmWitnessResult;
unsigned int lzvm_witness_abi_version(void) { return 1; }
int lzvm_witness_compute(const LzvmWitnessCall *call, LzvmWitnessResult *result) {
    (void)call;
    (void)result;
    return -11;
}
"#,
    );

    let library = load_witness_library(&library_path).expect("witness library should load");
    let mut buffers =
        WitnessTraceBuffers::new(vec![41], 1).expect("witness buffers should allocate");
    let result = library.compute(&mut buffers);
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert!(matches!(
        result,
        Err(WitnessCallError::NativeReturn { code: -11 })
    ));
}
