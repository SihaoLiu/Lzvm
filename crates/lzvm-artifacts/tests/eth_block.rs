use lzvm_artifacts::eth_block::{
    decode_eth_header_rlp, decode_eth_receipt_rlp, decode_eth_receipts_rlp,
    decode_eth_transaction_rlp, decode_eth_transactions_rlp, decode_eth_withdrawal_rlp,
    decode_eth_withdrawals_rlp, eth_header_hash, eth_ommers_hash, keccak256, parse_eth_block_rlp,
    EthBlockError, EthBlockRlp, EthReceiptError, EthReceiptRlp, EthTransactionError,
    EthTransactionRlp, EthWithdrawalError, EthWithdrawalRlp, HeaderField, ReceiptField,
    WithdrawalField,
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

#[test]
fn hashes_with_keccak_256() {
    assert_eq!(
        keccak256(&[]),
        hex32("c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470")
    );
}

#[test]
fn hashes_mainnet_genesis_header() {
    let header_rlp = rlp_list(&mainnet_genesis_header_items());
    let header = match lzvm_artifacts::rlp::parse_rlp(&header_rlp).expect("header should parse") {
        RlpItem::List(header) => header,
        RlpItem::Bytes(_) => panic!("header should be a list"),
    };

    assert_eq!(
        eth_header_hash(&header),
        hex32("d4e56740f876aef8c010b86a40d5f56745a118d0906a34e69aec8c0db1cb8fa3")
    );
}

#[test]
fn hashes_empty_ommers_list() {
    assert_eq!(
        eth_ommers_hash(&[]),
        hex32("1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347")
    );
}

#[test]
fn empty_ommers_hash_matches_mainnet_genesis_header() {
    let header = decode_header(mainnet_genesis_header_items());

    assert_eq!(eth_ommers_hash(&[]), header.ommers_hash);
}

#[test]
fn decodes_legacy_transactions_as_field_lists() {
    let transaction = RlpItem::List(vec![RlpItem::Bytes(vec![1]), RlpItem::Bytes(vec![2])]);

    let decoded = decode_eth_transaction_rlp(&transaction).expect("transaction should decode");

    assert_eq!(
        decoded,
        EthTransactionRlp::Legacy(vec![RlpItem::Bytes(vec![1]), RlpItem::Bytes(vec![2])])
    );
}

#[test]
fn decodes_typed_transaction_envelopes_as_opaque_payloads() {
    let transaction = RlpItem::Bytes(vec![2, 0xc0]);

    let decoded = decode_eth_transaction_rlp(&transaction).expect("transaction should decode");

    assert_eq!(
        decoded,
        EthTransactionRlp::Typed {
            transaction_type: 2,
            payload: vec![0xc0],
        }
    );
}

#[test]
fn decodes_transaction_lists() {
    let transactions = vec![
        RlpItem::List(vec![RlpItem::Bytes(vec![1])]),
        RlpItem::Bytes(vec![3, 0xc0]),
    ];

    let decoded = decode_eth_transactions_rlp(&transactions).expect("transactions should decode");

    assert_eq!(decoded.len(), 2);
    assert!(matches!(decoded[0], EthTransactionRlp::Legacy(_)));
    assert!(matches!(
        decoded[1],
        EthTransactionRlp::Typed {
            transaction_type: 3,
            ..
        }
    ));
}

#[test]
fn rejects_empty_typed_transaction_envelopes() {
    let error = decode_eth_transaction_rlp(&RlpItem::Bytes(Vec::new()))
        .expect_err("transaction should fail");

    assert!(matches!(error, EthTransactionError::EmptyTypedTransaction));
}

#[test]
fn rejects_out_of_range_typed_transaction_envelopes() {
    let error = decode_eth_transaction_rlp(&RlpItem::Bytes(vec![0x80]))
        .expect_err("transaction should fail");

    assert!(matches!(
        error,
        EthTransactionError::InvalidTransactionType { found: 0x80 }
    ));
}

#[test]
fn decodes_legacy_receipts() {
    let decoded = decode_eth_receipt_rlp(&receipt_item()).expect("receipt should decode");

    assert_eq!(
        decoded,
        EthReceiptRlp::Legacy {
            status_or_post_state: vec![1],
            cumulative_gas_used: 0x5208,
            logs_bloom: Box::new([0x11; 256]),
            logs: vec![log_item()],
        }
    );
}

#[test]
fn decodes_typed_receipt_envelopes_as_opaque_payloads() {
    let receipt = RlpItem::Bytes(vec![2, 0xf9, 0x01]);

    let decoded = decode_eth_receipt_rlp(&receipt).expect("receipt should decode");

    assert_eq!(
        decoded,
        EthReceiptRlp::Typed {
            receipt_type: 2,
            payload: vec![0xf9, 0x01],
        }
    );
}

#[test]
fn decodes_receipt_lists() {
    let receipts = vec![receipt_item(), RlpItem::Bytes(vec![3, 0xc0])];

    let decoded = decode_eth_receipts_rlp(&receipts).expect("receipts should decode");

    assert_eq!(decoded.len(), 2);
    assert!(matches!(decoded[0], EthReceiptRlp::Legacy { .. }));
    assert!(matches!(
        decoded[1],
        EthReceiptRlp::Typed {
            receipt_type: 3,
            ..
        }
    ));
}

#[test]
fn rejects_malformed_receipts() {
    let error =
        decode_eth_receipt_rlp(&RlpItem::Bytes(Vec::new())).expect_err("receipt should fail");
    assert!(matches!(error, EthReceiptError::EmptyTypedReceipt));

    let error =
        decode_eth_receipt_rlp(&RlpItem::Bytes(vec![0x80])).expect_err("receipt should fail");
    assert!(matches!(
        error,
        EthReceiptError::InvalidReceiptType { found: 0x80 }
    ));

    let error = decode_eth_receipt_rlp(&RlpItem::List(vec![RlpItem::Bytes(Vec::new()); 3]))
        .expect_err("receipt should fail");
    assert!(matches!(
        error,
        EthReceiptError::ReceiptFieldCount { found: 3 }
    ));

    let mut wrong_bloom = match receipt_item() {
        RlpItem::List(fields) => fields,
        RlpItem::Bytes(_) => panic!("receipt should be a list"),
    };
    wrong_bloom[2] = RlpItem::Bytes(vec![0x11; 255]);
    let error =
        decode_eth_receipt_rlp(&RlpItem::List(wrong_bloom)).expect_err("receipt should fail");
    assert!(matches!(
        error,
        EthReceiptError::ReceiptFieldLength {
            field: ReceiptField::LogsBloom,
            expected: 256,
            found: 255,
        }
    ));

    let mut wrong_logs = match receipt_item() {
        RlpItem::List(fields) => fields,
        RlpItem::Bytes(_) => panic!("receipt should be a list"),
    };
    wrong_logs[3] = RlpItem::Bytes(Vec::new());
    let error =
        decode_eth_receipt_rlp(&RlpItem::List(wrong_logs)).expect_err("receipt should fail");
    assert!(matches!(error, EthReceiptError::ExpectedLogsList));
}

#[test]
fn decodes_withdrawals() {
    let decoded = decode_eth_withdrawal_rlp(&withdrawal_item()).expect("withdrawal should decode");

    assert_eq!(
        decoded,
        EthWithdrawalRlp {
            index: 1,
            validator_index: 2,
            address: [0x33; 20],
            amount: 0x40,
        }
    );
}

#[test]
fn decodes_withdrawal_lists() {
    let withdrawals = vec![withdrawal_item(), withdrawal_item()];

    let decoded = decode_eth_withdrawals_rlp(&withdrawals).expect("withdrawals should decode");

    assert_eq!(decoded.len(), 2);
    assert_eq!(decoded[0].address, [0x33; 20]);
}

#[test]
fn rejects_malformed_withdrawals() {
    let error =
        decode_eth_withdrawal_rlp(&RlpItem::Bytes(Vec::new())).expect_err("withdrawal should fail");
    assert!(matches!(error, EthWithdrawalError::ExpectedWithdrawalList));

    let error = decode_eth_withdrawal_rlp(&RlpItem::List(vec![RlpItem::Bytes(Vec::new()); 3]))
        .expect_err("withdrawal should fail");
    assert!(matches!(
        error,
        EthWithdrawalError::WithdrawalFieldCount { found: 3 }
    ));

    let mut wrong_address = match withdrawal_item() {
        RlpItem::List(fields) => fields,
        RlpItem::Bytes(_) => panic!("withdrawal should be a list"),
    };
    wrong_address[2] = RlpItem::Bytes(vec![0x33; 19]);
    let error = decode_eth_withdrawal_rlp(&RlpItem::List(wrong_address))
        .expect_err("withdrawal should fail");
    assert!(matches!(
        error,
        EthWithdrawalError::WithdrawalFieldLength {
            field: WithdrawalField::Address,
            expected: 20,
            found: 19
        }
    ));

    let mut noncanonical = match withdrawal_item() {
        RlpItem::List(fields) => fields,
        RlpItem::Bytes(_) => panic!("withdrawal should be a list"),
    };
    noncanonical[0] = RlpItem::Bytes(vec![0, 1]);
    let error = decode_eth_withdrawal_rlp(&RlpItem::List(noncanonical))
        .expect_err("withdrawal should fail");
    assert!(matches!(
        error,
        EthWithdrawalError::NonCanonicalWithdrawalQuantity {
            field: WithdrawalField::Index
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

fn withdrawal_item() -> RlpItem {
    RlpItem::List(vec![
        RlpItem::Bytes(vec![1]),
        RlpItem::Bytes(vec![2]),
        RlpItem::Bytes(vec![0x33; 20]),
        RlpItem::Bytes(vec![0x40]),
    ])
}

fn receipt_item() -> RlpItem {
    RlpItem::List(vec![
        RlpItem::Bytes(vec![1]),
        RlpItem::Bytes(vec![0x52, 0x08]),
        RlpItem::Bytes(vec![0x11; 256]),
        RlpItem::List(vec![log_item()]),
    ])
}

fn log_item() -> RlpItem {
    RlpItem::List(vec![
        RlpItem::Bytes(vec![0x22; 20]),
        RlpItem::List(vec![RlpItem::Bytes(vec![0x33; 32])]),
        RlpItem::Bytes(vec![0x44, 0x55]),
    ])
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

fn mainnet_genesis_header_items() -> Vec<Vec<u8>> {
    vec![
        rlp_bytes(&[0; 32]),
        rlp_bytes(&hex32(
            "1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
        )),
        rlp_bytes(&[0; 20]),
        rlp_bytes(&hex32(
            "d7f8974fb5ac78d9ac099b9ad5018bedc2ce0a72dad1827a1709da30580f0544",
        )),
        rlp_bytes(&hex32(
            "56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421",
        )),
        rlp_bytes(&hex32(
            "56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421",
        )),
        rlp_bytes(&[0; 256]),
        rlp_bytes(&[0x04, 0x00, 0x00, 0x00, 0x00]),
        rlp_bytes(&[]),
        rlp_bytes(&[0x13, 0x88]),
        rlp_bytes(&[]),
        rlp_bytes(&[]),
        rlp_bytes(&hex_bytes(
            "11bbe8db4e347b4e8c937c1c8370e4b5ed33adb3db69cbdb7a38e1e50b1b82fa",
        )),
        rlp_bytes(&[0; 32]),
        rlp_bytes(&[0, 0, 0, 0, 0, 0, 0, 0x42]),
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

fn hex32(value: &str) -> [u8; 32] {
    hex_bytes(value)
        .try_into()
        .expect("hex value should have 32 bytes")
}

fn hex_bytes(value: &str) -> Vec<u8> {
    assert!(value.len().is_multiple_of(2));
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| (hex_value(pair[0]) << 4) | hex_value(pair[1]))
        .collect()
}

fn hex_value(value: u8) -> u8 {
    match value {
        b'0'..=b'9' => value - b'0',
        b'a'..=b'f' => value - b'a' + 10,
        b'A'..=b'F' => value - b'A' + 10,
        _ => panic!("invalid hex digit"),
    }
}
