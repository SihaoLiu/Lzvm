use lzvm_artifacts::eth_block::{
    decode_eth_header_rlp, eth_header_hash, eth_ommers_hash, parse_eth_block_rlp,
};
use lzvm_artifacts::eth_public_input::{
    eth_public_header_hash, eth_public_header_rlp_items, parse_eth_public_block_prefix,
    parse_eth_public_header_prefix, parse_eth_public_transactions_prefix,
};
use lzvm_artifacts::eth_trie::{transaction_trie_root, withdrawals_trie_root};
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

#[test]
fn parses_public_transaction_prefix_and_root() {
    let expected_transaction = expected_eip1559_transaction();
    let expected_root = transaction_trie_root(std::slice::from_ref(&expected_transaction))
        .expect("transaction root should build");
    let header_bytes = sample_public_header_bytes_with_transactions_root(expected_root);
    let transaction = eip1559_transaction_bytes();
    let mut input = header_bytes.clone();
    input.extend_from_slice(&1_u64.to_le_bytes());
    input.extend_from_slice(&transaction);
    input.extend_from_slice(b"tail");

    let parsed =
        parse_eth_public_transactions_prefix(&input).expect("public transactions should parse");

    assert_eq!(parsed.header.block_number, 42);
    assert_eq!(parsed.transactions, vec![expected_transaction.clone()]);
    assert_eq!(parsed.consumed, header_bytes.len() + 8 + transaction.len());
    assert_eq!(parsed.transactions_root(), expected_root);
    assert!(parsed.transactions_root_matches());
}

#[test]
fn parses_eip7702_authorization_zero_parity_as_empty_quantity() {
    let expected_transaction = expected_eip7702_transaction();
    let expected_root = transaction_trie_root(std::slice::from_ref(&expected_transaction))
        .expect("transaction root should build");
    let header_bytes = sample_public_header_bytes_with_transactions_root(expected_root);
    let transaction = eip7702_transaction_bytes();
    let mut input = header_bytes.clone();
    input.extend_from_slice(&1_u64.to_le_bytes());
    input.extend_from_slice(&transaction);

    let parsed =
        parse_eth_public_transactions_prefix(&input).expect("public transactions should parse");

    assert_eq!(parsed.transactions, vec![expected_transaction]);
    assert_eq!(parsed.consumed, header_bytes.len() + 8 + transaction.len());
    assert_eq!(parsed.transactions_root(), expected_root);
    assert!(parsed.transactions_root_matches());
}

#[test]
fn parses_public_block_prefix_and_roots() {
    let expected_transaction = expected_eip1559_transaction();
    let expected_ommer = RlpItem::List(expected_header_rlp_items());
    let expected_withdrawal = expected_withdrawal();
    let transaction_root = transaction_trie_root(std::slice::from_ref(&expected_transaction))
        .expect("transaction root should build");
    let ommers_hash = eth_ommers_hash(std::slice::from_ref(&expected_ommer));
    let withdrawal_root = withdrawals_trie_root(std::slice::from_ref(&expected_withdrawal));
    let header_bytes =
        sample_public_header_bytes_with_roots(transaction_root, ommers_hash, withdrawal_root);
    let transaction = eip1559_transaction_bytes();
    let ommer = sample_public_header_bytes();
    let withdrawal = withdrawal_bytes();
    let mut input = header_bytes.clone();
    input.extend_from_slice(&1_u64.to_le_bytes());
    input.extend_from_slice(&transaction);
    input.extend_from_slice(&1_u64.to_le_bytes());
    input.extend_from_slice(&ommer);
    input.push(1);
    input.extend_from_slice(&1_u64.to_le_bytes());
    input.extend_from_slice(&withdrawal);
    input.extend_from_slice(b"tail");

    let parsed = parse_eth_public_block_prefix(&input).expect("public block should parse");

    assert_eq!(parsed.header.block_number, 42);
    assert_eq!(parsed.transactions, vec![expected_transaction]);
    assert_eq!(parsed.ommers, vec![expected_ommer]);
    assert_eq!(parsed.withdrawals, Some(vec![expected_withdrawal]));
    assert_eq!(
        parsed.consumed,
        header_bytes.len() + 8 + transaction.len() + 8 + ommer.len() + 1 + 8 + withdrawal.len()
    );
    assert_eq!(parsed.transactions_root(), transaction_root);
    assert_eq!(parsed.ommers_hash(), ommers_hash);
    assert_eq!(parsed.withdrawals_root(), Some(withdrawal_root));
    assert!(parsed.transactions_root_matches());
    assert!(parsed.ommers_hash_matches());
    assert!(parsed.withdrawals_root_matches());
}

#[test]
fn public_block_prefix_writes_canonical_block_rlp() {
    let expected_transaction = expected_eip1559_transaction();
    let expected_ommer = RlpItem::List(expected_header_rlp_items());
    let expected_withdrawal = expected_withdrawal();
    let transaction_root = transaction_trie_root(std::slice::from_ref(&expected_transaction))
        .expect("transaction root should build");
    let ommers_hash = eth_ommers_hash(std::slice::from_ref(&expected_ommer));
    let withdrawal_root = withdrawals_trie_root(std::slice::from_ref(&expected_withdrawal));
    let header_bytes =
        sample_public_header_bytes_with_roots(transaction_root, ommers_hash, withdrawal_root);
    let mut input = header_bytes;
    input.extend_from_slice(&1_u64.to_le_bytes());
    input.extend_from_slice(&eip1559_transaction_bytes());
    input.extend_from_slice(&1_u64.to_le_bytes());
    input.extend_from_slice(&sample_public_header_bytes());
    input.push(1);
    input.extend_from_slice(&1_u64.to_le_bytes());
    input.extend_from_slice(&withdrawal_bytes());

    let parsed = parse_eth_public_block_prefix(&input).expect("public block should parse");
    let block_rlp = parsed.block_rlp();
    let block = parse_eth_block_rlp(&block_rlp).expect("block RLP should parse");

    assert_eq!(block.header, eth_public_header_rlp_items(&parsed.header));
    assert_eq!(block.transactions, vec![expected_transaction]);
    assert_eq!(block.ommers, vec![expected_ommer]);
    assert_eq!(block.withdrawals, Some(vec![expected_withdrawal]));
    assert!(block.extra_body_fields.is_empty());
    assert_eq!(
        eth_header_hash(&block.header),
        eth_public_header_hash(&parsed.header)
    );
    let decoded = decode_eth_header_rlp(&block.header).expect("RLP header should decode");
    assert_eq!(decoded.number, 42);
    assert_eq!(decoded.gas_used, 90);
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

fn expected_eip7702_transaction() -> RlpItem {
    let authorization = RlpItem::List(vec![
        rlp_quantity_u256(1),
        RlpItem::Bytes([4; 20].to_vec()),
        RlpItem::Bytes(Vec::new()),
        RlpItem::Bytes(Vec::new()),
        rlp_quantity_u256(0x33),
        rlp_quantity_u256(0x44),
    ]);
    let payload = vec![
        rlp_quantity_u64(1),
        rlp_quantity_u64(9),
        rlp_quantity_u128(20),
        rlp_quantity_u128(300),
        rlp_quantity_u64(30_000),
        RlpItem::Bytes([8; 20].to_vec()),
        rlp_quantity_u256(123),
        RlpItem::Bytes(b"auth-call".to_vec()),
        RlpItem::List(Vec::new()),
        RlpItem::List(vec![authorization]),
        RlpItem::Bytes(Vec::new()),
        rlp_quantity_u256(0x11),
        rlp_quantity_u256(0x22),
    ];
    let mut encoded = vec![4];
    encoded.extend_from_slice(&lzvm_artifacts::rlp::encode_rlp(&RlpItem::List(payload)));
    RlpItem::Bytes(encoded)
}

fn expected_eip1559_transaction() -> RlpItem {
    let payload = vec![
        rlp_quantity_u64(1),
        rlp_quantity_u64(7),
        rlp_quantity_u128(20),
        rlp_quantity_u128(300),
        rlp_quantity_u64(21_000),
        RlpItem::Bytes([9; 20].to_vec()),
        rlp_quantity_u256(123),
        RlpItem::Bytes(b"call-data".to_vec()),
        RlpItem::List(Vec::new()),
        rlp_quantity_u64(1),
        rlp_quantity_u256(0x11),
        rlp_quantity_u256(0x22),
    ];
    let mut encoded = vec![2];
    encoded.extend_from_slice(&lzvm_artifacts::rlp::encode_rlp(&RlpItem::List(payload)));
    RlpItem::Bytes(encoded)
}

fn expected_withdrawal() -> RlpItem {
    RlpItem::List(vec![
        rlp_quantity_u64(7),
        rlp_quantity_u64(8),
        RlpItem::Bytes([6; 20].to_vec()),
        rlp_quantity_u64(9),
    ])
}

fn eip7702_transaction_bytes() -> Vec<u8> {
    let mut bytes = Vec::new();
    push_u256(&mut bytes, 0x11);
    push_u256(&mut bytes, 0x22);
    push_uint_u64(&mut bytes, 0);
    bytes.extend_from_slice(&4_u32.to_le_bytes());
    bytes.extend_from_slice(&1_u64.to_le_bytes());
    bytes.extend_from_slice(&9_u64.to_le_bytes());
    bytes.extend_from_slice(&30_000_u64.to_le_bytes());
    bytes.extend_from_slice(&300_u128.to_le_bytes());
    bytes.extend_from_slice(&20_u128.to_le_bytes());
    push_bytes(&mut bytes, &[8; 20]);
    push_u256(&mut bytes, 123);
    bytes.extend_from_slice(&0_u64.to_le_bytes());
    bytes.extend_from_slice(&1_u64.to_le_bytes());
    push_u256(&mut bytes, 1);
    push_bytes(&mut bytes, &[4; 20]);
    push_uint_u64(&mut bytes, 0);
    push_uint_u8(&mut bytes, 0);
    push_u256(&mut bytes, 0x33);
    push_u256(&mut bytes, 0x44);
    push_bytes(&mut bytes, b"auth-call");
    bytes
}

fn eip1559_transaction_bytes() -> Vec<u8> {
    let mut bytes = Vec::new();
    push_u256(&mut bytes, 0x11);
    push_u256(&mut bytes, 0x22);
    push_uint_u64(&mut bytes, 1);
    bytes.extend_from_slice(&2_u32.to_le_bytes());
    bytes.extend_from_slice(&1_u64.to_le_bytes());
    bytes.extend_from_slice(&7_u64.to_le_bytes());
    bytes.extend_from_slice(&21_000_u64.to_le_bytes());
    bytes.extend_from_slice(&300_u128.to_le_bytes());
    bytes.extend_from_slice(&20_u128.to_le_bytes());
    push_option_bytes(&mut bytes, Some(&[9; 20]));
    push_u256(&mut bytes, 123);
    bytes.extend_from_slice(&0_u64.to_le_bytes());
    push_bytes(&mut bytes, b"call-data");
    bytes
}

fn withdrawal_bytes() -> Vec<u8> {
    let mut bytes = Vec::new();
    push_uint_u64(&mut bytes, 7);
    push_uint_u64(&mut bytes, 8);
    push_bytes(&mut bytes, &[6; 20]);
    push_uint_u64(&mut bytes, 9);
    bytes
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

fn sample_public_header_bytes_with_transactions_root(root: [u8; 32]) -> Vec<u8> {
    let mut input = sample_public_header_bytes();
    let offset = 40 + 40 + 28 + 40 + 8;
    input[offset..offset + 32].copy_from_slice(&root);
    input
}

fn sample_public_header_bytes_with_roots(
    transaction_root: [u8; 32],
    ommers_hash: [u8; 32],
    withdrawal_root: [u8; 32],
) -> Vec<u8> {
    let mut input = sample_public_header_bytes();
    input[48..80].copy_from_slice(&ommers_hash);
    input[156..188].copy_from_slice(&transaction_root);
    input[237..269].copy_from_slice(&withdrawal_root);
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

fn push_u256(out: &mut Vec<u8>, value: u8) {
    let mut bytes = [0; 32];
    bytes[31] = value;
    push_bytes(out, &bytes);
}

fn push_uint_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&8_u64.to_le_bytes());
    out.extend_from_slice(&value.to_be_bytes());
}

fn push_uint_u8(out: &mut Vec<u8>, value: u8) {
    out.extend_from_slice(&1_u64.to_le_bytes());
    out.push(value);
}

fn rlp_quantity_u64(value: u64) -> RlpItem {
    RlpItem::Bytes(rlp_quantity(&value.to_be_bytes()))
}

fn rlp_quantity_u128(value: u128) -> RlpItem {
    RlpItem::Bytes(rlp_quantity(&value.to_be_bytes()))
}

fn rlp_quantity_u256(value: u8) -> RlpItem {
    let mut bytes = [0; 32];
    bytes[31] = value;
    RlpItem::Bytes(rlp_quantity(&bytes))
}

fn rlp_quantity(bytes: &[u8]) -> Vec<u8> {
    let offset = bytes
        .iter()
        .position(|byte| *byte != 0)
        .unwrap_or(bytes.len());
    bytes[offset..].to_vec()
}
