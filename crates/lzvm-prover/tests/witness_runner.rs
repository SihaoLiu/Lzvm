use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use lzvm_field::{Felt, MODULUS};
use lzvm_prover::witness_loader::load_witness_library;
use lzvm_prover::witness_runner::{run_witness_trace, WitnessTraceRequest, WitnessTraceRunError};
use lzvm_prover::witness_trace::WitnessTraceError;

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("lzvm-witness-runner-{}-{name}", std::process::id()))
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
fn runs_native_witness_and_parses_trace_values() {
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
static void write_u64_le(unsigned char *out, unsigned long long value) {
    for (size_t i = 0; i < 8; i++) {
        out[i] = (unsigned char)((value >> (i * 8)) & 0xff);
    }
}
unsigned int lzvm_witness_abi_version(void) { return 1; }
int lzvm_witness_compute(const LzvmWitnessCall *call, LzvmWitnessResult *result) {
    if (!call || !result || call->input_len < 2 || call->output_len < 16) {
        return -1;
    }
    write_u64_le(call->output_ptr, (unsigned long long)call->input_ptr[0] + 1);
    write_u64_le(call->output_ptr + 8, (unsigned long long)call->input_ptr[1] + 1);
    result->status = 0;
    result->produced_len = 16;
    return 0;
}
"#,
    );
    let library = load_witness_library(&library_path).expect("witness library should load");

    let trace = run_witness_trace(
        &library,
        WitnessTraceRequest {
            input: vec![6, 8],
            rows: 1,
            columns: 2,
        },
    )
    .expect("witness trace should run and parse");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(trace.row_count(), 1);
    assert_eq!(trace.column_count(), 2);
    assert_eq!(
        trace.value(0, 0),
        Some(Felt::from_canonical(7).expect("canonical"))
    );
    assert_eq!(
        trace.value(0, 1),
        Some(Felt::from_canonical(9).expect("canonical"))
    );
}

#[test]
fn propagates_trace_parse_errors_from_native_output() {
    let dir = temp_dir("noncanonical");
    let _ = fs::remove_dir_all(&dir);
    let library_path = build_shared_library(
        &dir,
        "noncanonical",
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
static void write_u64_le(unsigned char *out, unsigned long long value) {
    for (size_t i = 0; i < 8; i++) {
        out[i] = (unsigned char)((value >> (i * 8)) & 0xff);
    }
}
unsigned int lzvm_witness_abi_version(void) { return 1; }
int lzvm_witness_compute(const LzvmWitnessCall *call, LzvmWitnessResult *result) {
    (void)call;
    if (!call || !result || call->output_len < 8) {
        return -1;
    }
    write_u64_le(call->output_ptr, 0xffffffff00000001ULL);
    result->status = 0;
    result->produced_len = 8;
    return 0;
}
"#,
    );
    let library = load_witness_library(&library_path).expect("witness library should load");

    let result = run_witness_trace(
        &library,
        WitnessTraceRequest {
            input: Vec::new(),
            rows: 1,
            columns: 1,
        },
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert!(matches!(
        result,
        Err(WitnessTraceRunError::Trace(
            WitnessTraceError::NonCanonicalElement {
                index: 0,
                value: MODULUS
            }
        ))
    ));
}
