use lzvm_artifacts::eth_public_input::parse_eth_public_block_prefix;

pub(super) fn sample_public_block_bytes_with_matching_roots() -> Vec<u8> {
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
