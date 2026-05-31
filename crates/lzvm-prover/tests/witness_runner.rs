use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use lzvm_artifacts::guest_image::parse_guest_image;
use lzvm_field::{Felt, MODULUS};
use lzvm_prover::native_guest_backend::NativeGuestBackend;
use lzvm_prover::witness_loader::{
    load_witness_library, WitnessBackend, WitnessCallError, WitnessComputeContext,
    WitnessTraceBuffers, WitnessTraceOutput,
};
use lzvm_prover::witness_runner::{
    run_witness_trace, run_witness_trace_with_context, WitnessTraceRequest, WitnessTraceRunError,
};
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

const ENTRY: u64 = 0x8000_0000;

fn sample_guest_image_with_words(words: &[u32]) -> Vec<u8> {
    let mut code = Vec::with_capacity(words.len() * 4);
    for word in words {
        code.extend_from_slice(&word.to_le_bytes());
    }
    let header = program_header(120, code.len() as u64);
    let mut image = sample_guest_image_with_program_headers(&[header]);
    image.extend_from_slice(&code);
    image
}

fn sample_guest_image() -> Vec<u8> {
    let mut bytes = vec![0_u8; 64];
    bytes[0..4].copy_from_slice(b"\x7fELF");
    bytes[4] = 2;
    bytes[5] = 1;
    bytes[6] = 1;
    bytes[16..18].copy_from_slice(&2_u16.to_le_bytes());
    bytes[18..20].copy_from_slice(&243_u16.to_le_bytes());
    bytes[20..24].copy_from_slice(&1_u32.to_le_bytes());
    bytes[24..32].copy_from_slice(&ENTRY.to_le_bytes());
    bytes[32..40].copy_from_slice(&64_u64.to_le_bytes());
    bytes[52..54].copy_from_slice(&64_u16.to_le_bytes());
    bytes
}

fn sample_guest_image_with_program_headers(program_headers: &[[u8; 56]]) -> Vec<u8> {
    let mut bytes = sample_guest_image();
    bytes[32..40].copy_from_slice(&64_u64.to_le_bytes());
    bytes[54..56].copy_from_slice(&56_u16.to_le_bytes());
    bytes[56..58].copy_from_slice(&(program_headers.len() as u16).to_le_bytes());
    for header in program_headers {
        bytes.extend_from_slice(header);
    }
    bytes
}

fn program_header(file_offset: u64, file_size: u64) -> [u8; 56] {
    let mut bytes = [0_u8; 56];
    bytes[0..4].copy_from_slice(&1_u32.to_le_bytes());
    bytes[4..8].copy_from_slice(&5_u32.to_le_bytes());
    bytes[8..16].copy_from_slice(&file_offset.to_le_bytes());
    bytes[16..24].copy_from_slice(&ENTRY.to_le_bytes());
    bytes[24..32].copy_from_slice(&ENTRY.to_le_bytes());
    bytes[32..40].copy_from_slice(&file_size.to_le_bytes());
    bytes[40..48].copy_from_slice(&file_size.to_le_bytes());
    bytes[48..56].copy_from_slice(&0x1000_u64.to_le_bytes());
    bytes
}

fn encode_i(immediate: i16, rs1: u8, funct3: u8, rd: u8, opcode: u8) -> u32 {
    (((immediate as i32 as u32) & 0x0fff) << 20)
        | (u32::from(rs1) << 15)
        | (u32::from(funct3) << 12)
        | (u32::from(rd) << 7)
        | u32::from(opcode)
}

fn addi(rd: u8, rs1: u8, immediate: i16) -> u32 {
    encode_i(immediate, rs1, 0, rd, 0x13)
}

struct NativeBackend;

impl WitnessBackend for NativeBackend {
    fn compute(
        &self,
        buffers: &mut WitnessTraceBuffers,
    ) -> Result<WitnessTraceOutput, WitnessCallError> {
        if buffers.input().len() < 2 || buffers.output().len() < 16 {
            return Err(WitnessCallError::NativeReturn { code: -1 });
        }
        let first = buffers.input()[0];
        let second = buffers.input()[1];
        let output = buffers.output_mut();
        output[0..8].copy_from_slice(&(u64::from(first) + 1).to_le_bytes());
        output[8..16].copy_from_slice(&(u64::from(second) + 1).to_le_bytes());
        Ok(WitnessTraceOutput { produced_len: 16 })
    }
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

    let trace = run_witness_trace(&library, WitnessTraceRequest::new(vec![6, 8], 1, 2))
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
fn runs_witness_trace_with_native_backend() {
    let trace = run_witness_trace(&NativeBackend, WitnessTraceRequest::new(vec![6, 8], 1, 2))
        .expect("witness trace should run and parse");

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
fn native_guest_backend_runs_guest_image_from_context() {
    let dir = temp_dir("native-guest");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let guest_image_bytes =
        sample_guest_image_with_words(&[addi(1, 0, 7), addi(2, 1, 3), 0x0000_0073]);
    fs::write(&guest_image, &guest_image_bytes).expect("guest image should be written");
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");

    let trace = run_witness_trace_with_context(
        &NativeGuestBackend::new(16),
        WitnessComputeContext {
            guest_image: Some(&guest_image),
            guest_image_info: Some(&guest_image_info),
        },
        WitnessTraceRequest::new(Vec::new(), 2, 2),
    )
    .expect("native guest trace should run");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(
        trace.value(0, 0),
        Some(Felt::from_canonical(ENTRY).expect("canonical"))
    );
    assert_eq!(
        trace.value(0, 1),
        Some(Felt::from_canonical(ENTRY + 4).expect("canonical"))
    );
    assert_eq!(
        trace.value(1, 0),
        Some(Felt::from_canonical(ENTRY + 4).expect("canonical"))
    );
    assert_eq!(
        trace.value(1, 1),
        Some(Felt::from_canonical(ENTRY + 8).expect("canonical"))
    );
}

#[test]
fn native_guest_backend_requires_guest_image_context() {
    let error = run_witness_trace_with_context(
        &NativeGuestBackend::new(16),
        WitnessComputeContext::empty(),
        WitnessTraceRequest::new(Vec::new(), 1, 2),
    )
    .expect_err("missing guest image context should reject");

    assert!(matches!(
        error,
        WitnessTraceRunError::Call(WitnessCallError::NativeReturn { code: -1 })
    ));
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

    let result = run_witness_trace(&library, WitnessTraceRequest::new(Vec::new(), 1, 1));
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
