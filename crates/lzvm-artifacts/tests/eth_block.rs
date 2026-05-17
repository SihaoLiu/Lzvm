use lzvm_artifacts::eth_block::{
    decode_eth_header_rlp, parse_eth_block_rlp, EthBlockError, EthBlockRlp, HeaderField,
};
use lzvm_artifacts::rlp::RlpItem;

#[test]
fn parses_block_body_with_transactions_and_ommers() {
    let block =
        parse_eth_block_rlp(&[0xc4, 0xc0, 0xc1, 0x80, 0xc0]).expect("block body should decode");

    assert_eq!(
        block,
        EthBlockRlp {
            header: Vec::new(),
            transactions: vec![RlpItem::Bytes(Vec::new())],
            ommers: Vec::new(),
            withdrawals: None,
            extra_body_fields: Vec::new(),
        }
    );
}

#[test]
fn parses_block_body_with_withdrawals() {
    let block = parse_eth_block_rlp(&[0xc5, 0xc0, 0xc1, 0x80, 0xc0, 0xc0])
        .expect("block body should decode");

    assert_eq!(block.withdrawals, Some(Vec::new()));
}

#[test]
fn preserves_extra_body_fields_after_withdrawals() {
    let block = parse_eth_block_rlp(&[0xc5, 0xc0, 0xc0, 0xc0, 0xc0, 0x01])
        .expect("block body should decode");

    assert_eq!(block.withdrawals, Some(Vec::new()));
    assert_eq!(block.extra_body_fields, vec![RlpItem::Bytes(vec![1])]);
}

#[test]
fn rejects_non_list_block_bodies() {
    let error = parse_eth_block_rlp(&[0x80]).expect_err("top-level item should be a list");

    assert!(matches!(error, EthBlockError::ExpectedBlockList));
}

#[test]
fn rejects_short_block_bodies() {
    let error =
        parse_eth_block_rlp(&[0xc2, 0xc0, 0xc0]).expect_err("block body should have fields");

    assert!(matches!(error, EthBlockError::BodyFieldCount { found: 2 }));
}

#[test]
fn rejects_non_list_header_fields() {
    let error =
        parse_eth_block_rlp(&[0xc3, 0x80, 0xc0, 0xc0]).expect_err("header should be a list");

    assert!(matches!(error, EthBlockError::ExpectedHeaderList));
}

#[test]
fn rejects_non_list_transaction_fields() {
    let error =
        parse_eth_block_rlp(&[0xc3, 0xc0, 0x80, 0xc0]).expect_err("transactions should be a list");

    assert!(matches!(error, EthBlockError::ExpectedTransactionsList));
}

#[test]
fn rejects_non_list_ommer_fields() {
    let error =
        parse_eth_block_rlp(&[0xc3, 0xc0, 0xc0, 0x80]).expect_err("ommers should be a list");

    assert!(matches!(error, EthBlockError::ExpectedOmmersList));
}

#[test]
fn rejects_non_list_withdrawal_fields() {
    let error = parse_eth_block_rlp(&[0xc4, 0xc0, 0xc0, 0xc0, 0x80])
        .expect_err("withdrawals should be a list");

    assert!(matches!(error, EthBlockError::ExpectedWithdrawalsList));
}

#[test]
fn decodes_legacy_header_fields() {
    let header = decode_header(legacy_header_items());

    assert_eq!(header.parent_hash, [0x11; 32]);
    assert_eq!(header.ommers_hash, [0x22; 32]);
    assert_eq!(header.beneficiary, [0x33; 20]);
    assert_eq!(header.state_root, [0x44; 32]);
    assert_eq!(header.transactions_root, [0x55; 32]);
    assert_eq!(header.receipts_root, [0x66; 32]);
    assert_eq!(header.logs_bloom, [0x77; 256]);
    assert_eq!(header.difficulty, vec![0x01]);
    assert_eq!(header.number, 2);
    assert_eq!(header.gas_limit, 1_000_000);
    assert_eq!(header.gas_used, 900_000);
    assert_eq!(header.timestamp, 0x65);
    assert_eq!(header.extra_data, b"lzvm".to_vec());
    assert_eq!(header.mix_hash, [0xaa; 32]);
    assert_eq!(header.nonce, [0xbb; 8]);
    assert_eq!(header.base_fee_per_gas, None);
    assert_eq!(header.withdrawals_root, None);
    assert_eq!(header.extra_header_fields, Vec::new());
}

#[test]
fn decodes_optional_header_fields_and_preserves_extra_header_fields() {
    let mut items = legacy_header_items();
    items.push(rlp_bytes(&[0x64]));
    items.push(rlp_bytes(&[0xdd; 32]));
    items.push(rlp_bytes(&[0xee]));

    let header = decode_header(items);

    assert_eq!(header.base_fee_per_gas, Some(vec![0x64]));
    assert_eq!(header.withdrawals_root, Some([0xdd; 32]));
    assert_eq!(header.extra_header_fields, vec![RlpItem::Bytes(vec![0xee])]);
}

#[test]
fn rejects_short_header_lists() {
    let error = decode_header_error(vec![rlp_bytes(&[]); 14]);

    assert!(matches!(
        error,
        EthBlockError::HeaderFieldCount { found: 14 }
    ));
}

#[test]
fn rejects_fixed_header_field_length_mismatch() {
    let mut items = legacy_header_items();
    items[0] = rlp_bytes(&[0x11; 31]);

    let error = decode_header_error(items);

    assert!(matches!(
        error,
        EthBlockError::HeaderFieldLength {
            field: HeaderField::ParentHash,
            expected: 32,
            found: 31
        }
    ));
}

#[test]
fn rejects_noncanonical_header_quantities() {
    let mut items = legacy_header_items();
    items[8] = rlp_bytes(&[0x00, 0x01]);

    let error = decode_header_error(items);

    assert!(matches!(
        error,
        EthBlockError::NonCanonicalQuantity {
            field: HeaderField::Number
        }
    ));
}

fn decode_header(items: Vec<Vec<u8>>) -> lzvm_artifacts::eth_block::EthHeaderRlp {
    let header_rlp = rlp_list(&items);
    let empty_list = rlp_list(&[]);
    let block_rlp = rlp_list(&[header_rlp, empty_list.clone(), empty_list]);
    let block = parse_eth_block_rlp(&block_rlp).expect("block body should decode");

    decode_eth_header_rlp(&block.header).expect("header should decode")
}

fn decode_header_error(items: Vec<Vec<u8>>) -> EthBlockError {
    let header_rlp = rlp_list(&items);
    let empty_list = rlp_list(&[]);
    let block_rlp = rlp_list(&[header_rlp, empty_list.clone(), empty_list]);
    let block = parse_eth_block_rlp(&block_rlp).expect("block body should decode");

    decode_eth_header_rlp(&block.header).expect_err("header should fail")
}

fn legacy_header_items() -> Vec<Vec<u8>> {
    vec![
        rlp_bytes(&[0x11; 32]),
        rlp_bytes(&[0x22; 32]),
        rlp_bytes(&[0x33; 20]),
        rlp_bytes(&[0x44; 32]),
        rlp_bytes(&[0x55; 32]),
        rlp_bytes(&[0x66; 32]),
        rlp_bytes(&[0x77; 256]),
        rlp_bytes(&[0x01]),
        rlp_bytes(&[0x02]),
        rlp_bytes(&[0x0f, 0x42, 0x40]),
        rlp_bytes(&[0x0d, 0xbb, 0xa0]),
        rlp_bytes(&[0x65]),
        rlp_bytes(b"lzvm"),
        rlp_bytes(&[0xaa; 32]),
        rlp_bytes(&[0xbb; 8]),
    ]
}

fn rlp_bytes(payload: &[u8]) -> Vec<u8> {
    if payload.len() == 1 && payload[0] <= 0x7f {
        return vec![payload[0]];
    }
    rlp_with_payload(0x80, 0xb7, payload)
}

fn rlp_list(items: &[Vec<u8>]) -> Vec<u8> {
    let payload = items.iter().flatten().copied().collect::<Vec<_>>();
    rlp_with_payload(0xc0, 0xf7, &payload)
}

fn rlp_with_payload(short_base: u8, long_base: u8, payload: &[u8]) -> Vec<u8> {
    if payload.len() <= 55 {
        let mut output = vec![short_base + payload.len() as u8];
        output.extend_from_slice(payload);
        return output;
    }

    let length = length_bytes(payload.len());
    let mut output = vec![long_base + length.len() as u8];
    output.extend_from_slice(&length);
    output.extend_from_slice(payload);
    output
}

fn length_bytes(mut value: usize) -> Vec<u8> {
    let mut bytes = Vec::new();
    while value > 0 {
        bytes.push((value & 0xff) as u8);
        value >>= 8;
    }
    bytes.reverse();
    bytes
}
