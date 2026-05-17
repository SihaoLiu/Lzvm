use lzvm_artifacts::eth_block::{parse_eth_block_rlp, EthBlockError, EthBlockRlp};
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
