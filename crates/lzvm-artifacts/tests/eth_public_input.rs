use lzvm_artifacts::eth_block::{decode_eth_header_rlp, eth_header_hash};
use lzvm_artifacts::eth_public_input::{
    eth_public_header_hash, eth_public_header_rlp_items, parse_eth_public_header_prefix,
};
use lzvm_artifacts::rlp::RlpItem;

#[test]
fn parses_public_header_prefix() {
    let header_bytes = sample_public_header_bytes();
    let mut input = header_bytes.clone();
    input.extend_from_slice(b"tail");

    let parsed = parse_eth_public_header_prefix(&input).expect("public header should parse");

    assert_eq!(parsed.consumed, header_bytes.len());
    assert_eq!(parsed.header.parent_hash, [1; 32]);
    assert_eq!(parsed.header.ommers_hash, [2; 32]);
    assert_eq!(parsed.header.beneficiary, [3; 20]);
    assert_eq!(parsed.header.state_root, [4; 32]);
    assert_eq!(parsed.header.transactions_root, [5; 32]);
    assert_eq!(parsed.header.receipts_root, [6; 32]);
    assert_eq!(parsed.header.withdrawals_root, Some([7; 32]));
    assert_eq!(parsed.header.logs_bloom, [8; 256]);
    assert_eq!(parsed.header.difficulty, u256_bytes(9));
    assert_eq!(parsed.header.block_number, 42);
    assert_eq!(parsed.header.gas_limit, 100);
    assert_eq!(parsed.header.gas_used, 90);
    assert_eq!(parsed.header.timestamp, 77);
    assert_eq!(parsed.header.mix_hash, [10; 32]);
    assert_eq!(parsed.header.nonce, [11; 8]);
    assert_eq!(parsed.header.base_fee_per_gas, Some(123));
    assert_eq!(parsed.header.blob_gas_used, Some(456));
    assert_eq!(parsed.header.excess_blob_gas, Some(789));
    assert_eq!(parsed.header.parent_beacon_block_root, Some([12; 32]));
    assert_eq!(parsed.header.requests_hash, Some([13; 32]));
    assert_eq!(parsed.header.extra_data, b"abc");

    let expected_rlp_items = expected_header_rlp_items();
    assert_eq!(
        eth_public_header_rlp_items(&parsed.header),
        expected_rlp_items
    );
    assert_eq!(
        eth_public_header_hash(&parsed.header),
        eth_header_hash(&expected_rlp_items)
    );

    let decoded = decode_eth_header_rlp(&expected_rlp_items).expect("RLP header should decode");
    assert_eq!(decoded.number, 42);
    assert_eq!(decoded.gas_limit, 100);
    assert_eq!(decoded.gas_used, 90);
    assert_eq!(decoded.timestamp, 77);
    assert_eq!(decoded.extra_header_fields.len(), 4);
}

#[test]
fn rejects_public_header_fixed_bytes_with_unexpected_length() {
    let mut input = Vec::new();
    push_bytes(&mut input, &[1; 31]);

    let error = parse_eth_public_header_prefix(&input).expect_err("length should be rejected");

    assert_eq!(
        error.to_string(),
        "invalid ETH public input parent_hash length: expected 32, found 31"
    );
}

fn expected_header_rlp_items() -> Vec<RlpItem> {
    vec![
        bytes([1; 32]),
        bytes([2; 32]),
        bytes([3; 20]),
        bytes([4; 32]),
        bytes([5; 32]),
        bytes([6; 32]),
        bytes([8; 256]),
        RlpItem::Bytes(vec![9]),
        RlpItem::Bytes(vec![42]),
        RlpItem::Bytes(vec![100]),
        RlpItem::Bytes(vec![90]),
        RlpItem::Bytes(vec![77]),
        RlpItem::Bytes(b"abc".to_vec()),
        bytes([10; 32]),
        bytes([11; 8]),
        RlpItem::Bytes(vec![123]),
        bytes([7; 32]),
        RlpItem::Bytes(vec![0x01, 0xc8]),
        RlpItem::Bytes(vec![0x03, 0x15]),
        bytes([12; 32]),
        bytes([13; 32]),
    ]
}

pub fn sample_public_header_bytes() -> Vec<u8> {
    let mut input = Vec::new();
    push_bytes(&mut input, &[1; 32]);
    push_bytes(&mut input, &[2; 32]);
    push_bytes(&mut input, &[3; 20]);
    push_bytes(&mut input, &[4; 32]);
    push_bytes(&mut input, &[5; 32]);
    push_bytes(&mut input, &[6; 32]);
    push_option_bytes(&mut input, Some(&[7; 32]));
    push_bytes(&mut input, &[8; 256]);
    push_bytes(&mut input, &u256_bytes(9));
    input.extend_from_slice(&42_u64.to_le_bytes());
    input.extend_from_slice(&100_u64.to_le_bytes());
    input.extend_from_slice(&90_u64.to_le_bytes());
    input.extend_from_slice(&77_u64.to_le_bytes());
    push_bytes(&mut input, &[10; 32]);
    push_bytes(&mut input, &[11; 8]);
    push_option_u64(&mut input, Some(123));
    push_option_u64(&mut input, Some(456));
    push_option_u64(&mut input, Some(789));
    push_option_bytes(&mut input, Some(&[12; 32]));
    push_option_bytes(&mut input, Some(&[13; 32]));
    push_bytes(&mut input, b"abc");
    input
}

fn bytes<const N: usize>(value: [u8; N]) -> RlpItem {
    RlpItem::Bytes(value.to_vec())
}

fn u256_bytes(value: u8) -> [u8; 32] {
    let mut bytes = [0; 32];
    bytes[31] = value;
    bytes
}

fn push_bytes(out: &mut Vec<u8>, bytes: &[u8]) {
    out.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
    out.extend_from_slice(bytes);
}

fn push_option_bytes(out: &mut Vec<u8>, bytes: Option<&[u8]>) {
    match bytes {
        Some(bytes) => {
            out.push(1);
            push_bytes(out, bytes);
        }
        None => out.push(0),
    }
}

fn push_option_u64(out: &mut Vec<u8>, value: Option<u64>) {
    match value {
        Some(value) => {
            out.push(1);
            out.extend_from_slice(&value.to_le_bytes());
        }
        None => out.push(0),
    }
}
