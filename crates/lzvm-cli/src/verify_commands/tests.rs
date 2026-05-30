use super::*;
use lzvm_artifacts::eth_block_input::{build_eth_block_input, encode_eth_block_input};
use lzvm_artifacts::eth_block_input_segment::{
    encode_eth_block_input_segment, ETH_BLOCK_INPUT_SEGMENT_ID,
};
use lzvm_artifacts::eth_block_public_values::public_values_from_eth_block_input;
use lzvm_artifacts::eth_public_input::parse_eth_public_block_prefix;
use lzvm_artifacts::proof::{encode_proof_artifact, ProofArtifact, ProofSegment};
use lzvm_artifacts::public_values::{encode_public_values, public_values_digest};

#[test]
fn parses_eth_public_input_option_for_verify_proof_args() {
    let result = parse_verify_proof_args(&[
        "--eth-public-input",
        "public.bin",
        "setup",
        "proof.bin",
        "public-values.bin",
    ])
    .expect("verify args should parse");

    assert_eq!(result.eth_public_input, Some("public.bin"));
}

#[test]
fn rejects_combined_eth_block_and_public_input_options() {
    let result = parse_verify_proof_args(&[
        "--eth-block-input",
        "block.input",
        "--eth-public-input",
        "public.bin",
        "setup",
        "proof.bin",
        "public-values.bin",
    ]);

    assert!(matches!(
        result,
        Err(VerifyProofArgError::Invalid(message))
            if message == "cannot combine --eth-block-input and --eth-public-input"
    ));
}

#[test]
fn rejects_missing_eth_public_input_value_during_parse() {
    let result = parse_verify_proof_args(&[
        "--eth-public-input",
        "--program-image-cache",
        "cache.bin",
        "setup",
        "proof.bin",
        "public-values.bin",
    ]);

    assert!(matches!(
        result,
        Err(VerifyProofArgError::Invalid(message)) if message == "missing --eth-public-input value"
    ));
}

#[test]
fn verifies_eth_public_input_against_embedded_block_input_segment() {
    let dir = std::env::temp_dir().join(format!(
        "lzvm-verify-proof-eth-public-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture directory should be created");
    let public_input_path = dir.join("public.bin");
    let proof_path = dir.join("proof.bin");
    let public_values_path = dir.join("public-values.bin");
    let public_input = sample_public_block_bytes_with_matching_roots();
    std::fs::write(&public_input_path, &public_input).expect("public input should write");
    let public_block = parse_eth_public_block_prefix(&public_input).expect("block should parse");
    let block_rlp = public_block.block_rlp();
    let block_input = build_eth_block_input(&block_rlp).expect("block input should build");
    let block_input_bytes = encode_eth_block_input(&block_input).expect("input should encode");
    let setup_hash = [7; 32];
    let public_values = public_values_from_eth_block_input(setup_hash, &block_input);
    std::fs::write(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    )
    .expect("public values should write");
    let proof = ProofArtifact {
        setup_hash,
        public_values_hash: public_values_digest(&public_values).expect("digest should compute"),
        segments: vec![ProofSegment {
            id: ETH_BLOCK_INPUT_SEGMENT_ID,
            data: encode_eth_block_input_segment(&block_input).expect("segment should encode"),
        }],
    };
    std::fs::write(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    )
    .expect("proof should write");

    let binding = eth_block_input::verify_eth_public_input_binding(
        proof_path.to_str().expect("proof path should be utf-8"),
        public_values_path
            .to_str()
            .expect("public values path should be utf-8"),
        public_input_path
            .to_str()
            .expect("public input path should be utf-8"),
    )
    .expect("public input should match proof");

    assert_eq!(binding.bytes, block_input_bytes.len());
    assert_eq!(binding.block_hash, block_input.block_hash);
    assert_eq!(binding.transaction_preimage_count, 1);
    assert_eq!(binding.withdrawal_count, Some(1));
    std::fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn rejects_eth_public_input_with_trailing_bytes_for_verify_binding() {
    let dir = std::env::temp_dir().join(format!(
        "lzvm-verify-proof-eth-public-trailing-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture directory should be created");
    let public_input_path = dir.join("public.bin");
    let proof_path = dir.join("proof.bin");
    let public_values_path = dir.join("public-values.bin");
    let public_input = sample_public_block_bytes_with_matching_roots();
    let public_block = parse_eth_public_block_prefix(&public_input).expect("block should parse");
    let block_rlp = public_block.block_rlp();
    let block_input = build_eth_block_input(&block_rlp).expect("block input should build");
    let setup_hash = [7; 32];
    let public_values = public_values_from_eth_block_input(setup_hash, &block_input);
    let mut public_input_with_tail = public_input;
    public_input_with_tail.extend_from_slice(b"tail");
    std::fs::write(&public_input_path, public_input_with_tail).expect("public input should write");
    std::fs::write(
        &public_values_path,
        encode_public_values(&public_values).expect("public values should encode"),
    )
    .expect("public values should write");
    let proof = ProofArtifact {
        setup_hash,
        public_values_hash: public_values_digest(&public_values).expect("digest should compute"),
        segments: vec![ProofSegment {
            id: ETH_BLOCK_INPUT_SEGMENT_ID,
            data: encode_eth_block_input_segment(&block_input).expect("segment should encode"),
        }],
    };
    std::fs::write(
        &proof_path,
        encode_proof_artifact(&proof).expect("proof should encode"),
    )
    .expect("proof should write");

    let result = eth_block_input::verify_eth_public_input_binding(
        proof_path.to_str().expect("proof path should be utf-8"),
        public_values_path
            .to_str()
            .expect("public values path should be utf-8"),
        public_input_path
            .to_str()
            .expect("public input path should be utf-8"),
    );
    std::fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert!(matches!(
        result,
        Err(message)
            if message
                == format!(
                    "ETH public input failed: {}: unexpected trailing bytes in ETH public input: 4",
                    public_input_path.display()
                )
    ));
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
