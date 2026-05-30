use super::*;
use lzvm_artifacts::eth_block_input::parse_eth_block_input;
use lzvm_artifacts::eth_public_input::parse_eth_public_block_prefix;

#[test]
fn rejects_trace_bytes_with_all_units_during_parse() {
    let result = parse_witness_args(&[
        "--trace-bytes",
        "trace.bin",
        "--all-units",
        "setup-dir",
        "out-dir",
        "guest.elf",
    ]);

    assert!(matches!(
        result,
        Err(ParseError::Invalid(message))
            if message == "--trace-bytes requires a single-unit witness run"
    ));
}

#[test]
fn rejects_trace_bytes_with_aggregate_during_parse() {
    let result = parse_witness_args(&[
        "--trace-bytes",
        "trace.bin",
        "--aggregate",
        "setup-dir",
        "out-dir",
        "guest.elf",
    ]);

    assert!(matches!(
        result,
        Err(ParseError::Invalid(message))
            if message == "--trace-bytes requires a single-unit witness run"
    ));
}

#[test]
fn rejects_evaluation_values_segment_without_all_units_during_parse() {
    let result = parse_witness_args(&[
        "--evaluation-values-segment",
        "evaluations.bin",
        "setup-dir",
        "out-dir",
        "witness.so",
        "guest.elf",
    ]);

    assert!(matches!(
        result,
        Err(ParseError::Invalid(message))
            if message == "--evaluation-values-segment requires all-units mode"
    ));
}

#[test]
fn parses_eth_block_input_option_for_witness_args() {
    let result = parse_witness_args(&[
        "--eth-block-input",
        "block.input",
        "setup-dir",
        "out-dir",
        "witness.so",
        "guest.elf",
    ])
    .expect("witness args should parse");

    assert_eq!(result.eth_block_input, Some("block.input".into()));
}

#[test]
fn parses_eth_public_input_option_for_witness_args() {
    let result = parse_witness_args(&[
        "--eth-public-input",
        "public.bin",
        "setup-dir",
        "out-dir",
        "witness.so",
        "guest.elf",
    ])
    .expect("witness args should parse");

    assert_eq!(result.eth_public_input, Some("public.bin".into()));
}

#[test]
fn rejects_combined_eth_block_and_public_input_options() {
    let result = parse_witness_args(&[
        "--eth-block-input",
        "block.input",
        "--eth-public-input",
        "public.bin",
        "setup-dir",
        "out-dir",
        "witness.so",
        "guest.elf",
    ]);

    assert!(matches!(
        result,
        Err(ParseError::Invalid(message))
            if message == "cannot combine --eth-block-input and --eth-public-input"
    ));
}

#[test]
fn rejects_missing_eth_public_input_value_during_parse() {
    let result = parse_witness_args(&[
        "--eth-public-input",
        "--trace-bytes",
        "trace.bin",
        "setup-dir",
        "out-dir",
        "guest.elf",
    ]);

    assert!(matches!(
        result,
        Err(ParseError::Invalid(message)) if message == "missing --eth-public-input value"
    ));
}

#[test]
fn writes_eth_public_input_option_as_block_input_artifact() {
    let dir = std::env::temp_dir().join(format!(
        "lzvm-prove-witness-eth-public-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("temp dir should be created");
    let input_path = dir.join("public.bin");
    let output_dir = dir.join("proof-out");
    fs::write(&input_path, sample_public_block_bytes_with_matching_roots())
        .expect("public input should be written");
    let parsed = parse_witness_args(&[
        "--eth-public-input",
        input_path.to_str().expect("input path should be utf-8"),
        "setup-dir",
        output_dir.to_str().expect("output path should be utf-8"),
        "witness.so",
        "guest.elf",
    ])
    .expect("witness args should parse");

    let prepared =
        prepare_eth_block_input(&parsed).expect("public input should prepare block input");
    let summary = prepared
        .summary
        .expect("block input summary should be present");
    let output_path = output_dir.join("eth-block.input");
    let encoded = fs::read(&output_path).expect("block input should be written");
    let parsed_input = parse_eth_block_input(&encoded).expect("block input should parse");

    assert!(prepared.generated_from_public_input);
    assert_eq!(summary.path, output_path);
    assert_eq!(summary.byte_len, encoded.len() as u64);
    assert_eq!(summary.input, parsed_input);
    assert_eq!(summary.block_number, 42);
    assert_eq!(summary.transaction_preimage_count, 1);
    assert_eq!(summary.withdrawal_count, Some(1));
    fs::remove_dir_all(&dir).expect("temp dir should be removed");
}

fn sample_public_block_bytes_with_matching_roots() -> Vec<u8> {
    let mut input = sample_public_header_bytes();
    input.extend_from_slice(&1_u64.to_le_bytes());
    input.extend_from_slice(&eip1559_transaction_bytes());
    input.extend_from_slice(&0_u64.to_le_bytes());
    input.push(1);
    input.extend_from_slice(&1_u64.to_le_bytes());
    input.extend_from_slice(&withdrawal_bytes());

    let parsed = parse_eth_public_block_prefix(&input).expect("block should parse");
    let transaction_root = parsed.transactions_root();
    let ommers_hash = parsed.ommers_hash();
    let withdrawal_root = parsed
        .withdrawals_root()
        .expect("withdrawals root should be present");
    input[48..80].copy_from_slice(&ommers_hash);
    input[156..188].copy_from_slice(&transaction_root);
    input[237..269].copy_from_slice(&withdrawal_root);
    input
}

fn sample_public_header_bytes() -> Vec<u8> {
    let mut input = Vec::new();
    push_public_bytes(&mut input, &[1; 32]);
    push_public_bytes(&mut input, &[2; 32]);
    push_public_bytes(&mut input, &[3; 20]);
    push_public_bytes(&mut input, &[4; 32]);
    push_public_bytes(&mut input, &[5; 32]);
    push_public_bytes(&mut input, &[6; 32]);
    push_public_option_bytes(&mut input, Some(&[7; 32]));
    push_public_bytes(&mut input, &[8; 256]);
    push_public_bytes(&mut input, &u256_bytes(9));
    input.extend_from_slice(&42_u64.to_le_bytes());
    input.extend_from_slice(&100_u64.to_le_bytes());
    input.extend_from_slice(&90_u64.to_le_bytes());
    input.extend_from_slice(&77_u64.to_le_bytes());
    push_public_bytes(&mut input, &[10; 32]);
    push_public_bytes(&mut input, &[11; 8]);
    push_public_option_u64(&mut input, Some(123));
    push_public_option_u64(&mut input, Some(456));
    push_public_option_u64(&mut input, Some(789));
    push_public_option_bytes(&mut input, Some(&[12; 32]));
    push_public_option_bytes(&mut input, Some(&[13; 32]));
    push_public_bytes(&mut input, b"abc");
    input
}

fn eip1559_transaction_bytes() -> Vec<u8> {
    let mut bytes = Vec::new();
    push_public_u256(&mut bytes, 0x11);
    push_public_u256(&mut bytes, 0x22);
    push_public_uint_u64(&mut bytes, 1);
    bytes.extend_from_slice(&2_u32.to_le_bytes());
    bytes.extend_from_slice(&1_u64.to_le_bytes());
    bytes.extend_from_slice(&7_u64.to_le_bytes());
    bytes.extend_from_slice(&21_000_u64.to_le_bytes());
    bytes.extend_from_slice(&300_u128.to_le_bytes());
    bytes.extend_from_slice(&20_u128.to_le_bytes());
    push_public_option_bytes(&mut bytes, Some(&[9; 20]));
    push_public_u256(&mut bytes, 123);
    bytes.extend_from_slice(&0_u64.to_le_bytes());
    push_public_bytes(&mut bytes, b"call-data");
    bytes
}

fn withdrawal_bytes() -> Vec<u8> {
    let mut bytes = Vec::new();
    push_public_uint_u64(&mut bytes, 7);
    push_public_uint_u64(&mut bytes, 8);
    push_public_bytes(&mut bytes, &[6; 20]);
    push_public_uint_u64(&mut bytes, 9);
    bytes
}

fn push_public_bytes(out: &mut Vec<u8>, bytes: &[u8]) {
    out.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
    out.extend_from_slice(bytes);
}

fn push_public_option_bytes(out: &mut Vec<u8>, bytes: Option<&[u8]>) {
    match bytes {
        Some(bytes) => {
            out.push(1);
            push_public_bytes(out, bytes);
        }
        None => out.push(0),
    }
}

fn push_public_option_u64(out: &mut Vec<u8>, value: Option<u64>) {
    match value {
        Some(value) => {
            out.push(1);
            out.extend_from_slice(&value.to_le_bytes());
        }
        None => out.push(0),
    }
}

fn push_public_u256(out: &mut Vec<u8>, value: u8) {
    let mut bytes = [0; 32];
    bytes[31] = value;
    push_public_bytes(out, &bytes);
}

fn push_public_uint_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&8_u64.to_le_bytes());
    out.extend_from_slice(&value.to_be_bytes());
}

fn u256_bytes(value: u8) -> [u8; 32] {
    let mut bytes = [0; 32];
    bytes[31] = value;
    bytes
}
