use lzvm_artifacts::eth_block_input::{
    build_eth_block_input, build_eth_block_input_with_receipts, encode_eth_block_input,
    parse_eth_block_input, EthBlockInputError, EthBlockInputTrie,
};
use lzvm_artifacts::eth_trie::{receipt_trie_build, transaction_trie_build, TrieHashPreimage};
use lzvm_artifacts::rlp::parse_rlp;
use lzvm_artifacts::sectioned::{encode_sectioned_file, parse_sectioned_file, SectionedSection};

const ETH_BLOCK_INPUT_KIND: [u8; 4] = *b"ethi";
const ETH_BLOCK_INPUT_VERSION: u32 = 1;
const METADATA_SECTION_ID: u32 = 1;
const TRANSACTION_PREIMAGES_SECTION_ID: u32 = 3;
const WITHDRAWAL_PREIMAGES_SECTION_ID: u32 = 4;
const RECEIPT_PREIMAGES_SECTION_ID: u32 = 5;
const RECEIPTS_RLP_SECTION_ID: u32 = 6;

#[test]
fn encodes_and_parses_eth_block_inputs() {
    let block_rlp = sample_block_rlp_with_transactions(
        hex32("e52f61e61ebdce920205cfca55e00c70bf219b45ea432febbf96152313e61db5"),
        vec![rlp_list(&[rlp_bytes(&[1])])],
    );

    let input = build_eth_block_input(&block_rlp).expect("block input should build");
    assert_eq!(input.block_rlp, block_rlp);
    assert_eq!(input.block_number, 2);
    assert_eq!(input.timestamp, 101);
    assert_eq!(input.transactions.root, input.transactions_root);
    assert_eq!(input.transactions.hash_preimages.len(), 1);
    assert_eq!(input.withdrawals, None);

    let encoded = encode_eth_block_input(&input).expect("block input should encode");
    assert_eq!(&encoded[..4], b"ethi");

    let parsed = parse_eth_block_input(&encoded).expect("block input should parse");
    assert_eq!(parsed, input);
}

#[test]
fn rejects_unsupported_eth_block_input_versions() {
    let block_rlp = sample_block_rlp_with_transactions(
        hex32("e52f61e61ebdce920205cfca55e00c70bf219b45ea432febbf96152313e61db5"),
        vec![rlp_list(&[rlp_bytes(&[1])])],
    );
    let input = build_eth_block_input(&block_rlp).expect("block input should build");
    let encoded = encode_eth_block_input(&input).expect("block input should encode");
    let mut file = parse_sectioned_file(&encoded, ETH_BLOCK_INPUT_KIND, ETH_BLOCK_INPUT_VERSION)
        .expect("sectioned input should parse");

    for version in [0, ETH_BLOCK_INPUT_VERSION + 1] {
        file.version = version;
        let encoded = encode_sectioned_file(&file).expect("sectioned input should encode");

        assert_eq!(
            parse_eth_block_input(&encoded)
                .expect_err("unsupported block input version should reject"),
            EthBlockInputError::UnsupportedVersion {
                found: version,
                expected: ETH_BLOCK_INPUT_VERSION,
            }
        );
    }
}

#[test]
fn rejects_unknown_eth_block_input_sections() {
    let block_rlp = sample_block_rlp_with_transactions(
        hex32("e52f61e61ebdce920205cfca55e00c70bf219b45ea432febbf96152313e61db5"),
        vec![rlp_list(&[rlp_bytes(&[1])])],
    );
    let input = build_eth_block_input(&block_rlp).expect("block input should build");
    let encoded = encode_eth_block_input(&input).expect("block input should encode");
    let mut file = parse_sectioned_file(&encoded, ETH_BLOCK_INPUT_KIND, ETH_BLOCK_INPUT_VERSION)
        .expect("sectioned input should parse");
    file.sections.push(SectionedSection {
        id: 99,
        data: vec![1, 2, 3, 4],
    });
    let encoded = encode_sectioned_file(&file).expect("sectioned input should encode");

    let error = parse_eth_block_input(&encoded).expect_err("unknown section should reject");

    assert_eq!(error.to_string(), "invalid ETH block input section id 99");
}

#[test]
fn rejects_duplicate_eth_block_input_sections() {
    let block_rlp = sample_block_rlp_with_transactions(
        hex32("e52f61e61ebdce920205cfca55e00c70bf219b45ea432febbf96152313e61db5"),
        vec![rlp_list(&[rlp_bytes(&[1])])],
    );
    let input = build_eth_block_input(&block_rlp).expect("block input should build");
    let encoded = encode_eth_block_input(&input).expect("block input should encode");
    let mut file = parse_sectioned_file(&encoded, ETH_BLOCK_INPUT_KIND, ETH_BLOCK_INPUT_VERSION)
        .expect("sectioned input should parse");
    let metadata = file
        .sections
        .iter()
        .find(|section| section.id == METADATA_SECTION_ID)
        .expect("metadata section should exist")
        .clone();
    file.sections.push(metadata);
    let encoded = encode_sectioned_file(&file).expect("sectioned input should encode");

    let error = parse_eth_block_input(&encoded).expect_err("duplicate section should reject");

    assert_eq!(
        error,
        EthBlockInputError::DuplicateSection {
            id: METADATA_SECTION_ID
        }
    );
}

#[test]
fn builds_withdrawals_eth_block_inputs() {
    let block_rlp = sample_block_rlp_with_withdrawals(
        hex32("51c445cba96d0dfd446eec8b2b94f104608cf8443a92f7f87c76a383d6687300"),
        vec![withdrawal_item()],
    );

    let input = build_eth_block_input(&block_rlp).expect("block input should build");
    let withdrawals = input
        .withdrawals
        .expect("withdrawals trie should be present");

    assert_eq!(
        input.withdrawals_root,
        Some(hex32(
            "51c445cba96d0dfd446eec8b2b94f104608cf8443a92f7f87c76a383d6687300"
        ))
    );
    assert_eq!(
        withdrawals.root,
        input.withdrawals_root.expect("root should exist")
    );
    assert_eq!(withdrawals.hash_preimages.len(), 1);
}

#[test]
fn rejects_encoding_withdrawals_root_without_preimages() {
    let block_rlp = sample_block_rlp_with_withdrawals(
        hex32("51c445cba96d0dfd446eec8b2b94f104608cf8443a92f7f87c76a383d6687300"),
        vec![withdrawal_item()],
    );
    let mut input = build_eth_block_input(&block_rlp).expect("block input should build");
    input.withdrawals = None;

    let error = encode_eth_block_input(&input).expect_err("block input should reject withdrawals");

    assert!(matches!(
        error,
        EthBlockInputError::UnexpectedWithdrawalsRoot
    ));
}

#[test]
fn rejects_encoding_withdrawal_preimages_without_root() {
    let withdrawal_item = withdrawal_item();
    let block_rlp = sample_block_rlp_with_withdrawals(
        hex32("51c445cba96d0dfd446eec8b2b94f104608cf8443a92f7f87c76a383d6687300"),
        vec![withdrawal_item.clone()],
    );
    let withdrawal_input = build_eth_block_input(&block_rlp).expect("block input should build");
    let mut input = build_eth_block_input(&sample_block_rlp_with_transactions(
        empty_trie_root(),
        Vec::new(),
    ))
    .expect("block input should build");
    input.withdrawals = withdrawal_input.withdrawals;

    let error = encode_eth_block_input(&input).expect_err("block input should reject withdrawals");

    assert!(matches!(error, EthBlockInputError::MissingWithdrawalsRoot));
}

#[test]
fn rejects_withdrawal_preimages_without_metadata_root() {
    let input = build_eth_block_input(&sample_block_rlp_with_transactions(
        empty_trie_root(),
        Vec::new(),
    ))
    .expect("block input should build");
    let encoded = encode_eth_block_input(&input).expect("block input should encode");
    let mut file = parse_sectioned_file(&encoded, ETH_BLOCK_INPUT_KIND, ETH_BLOCK_INPUT_VERSION)
        .expect("sectioned input should parse");
    file.sections.push(SectionedSection {
        id: WITHDRAWAL_PREIMAGES_SECTION_ID,
        data: Vec::new(),
    });
    let encoded = encode_sectioned_file(&file).expect("sectioned input should encode");

    let error = parse_eth_block_input(&encoded).expect_err("block input should reject withdrawals");

    assert!(matches!(error, EthBlockInputError::MissingWithdrawalsRoot));
}

#[test]
fn builds_receipt_eth_block_inputs() {
    let receipt_item = receipt_item();
    let receipts = vec![parse_rlp(&receipt_item).expect("receipt item should parse")];
    let receipt_build = receipt_trie_build(&receipts);
    let transaction_items = vec![rlp_list(&[rlp_bytes(&[1])])];
    let block_rlp = sample_block_rlp_with_transactions_and_receipts(
        hex32("e52f61e61ebdce920205cfca55e00c70bf219b45ea432febbf96152313e61db5"),
        receipt_build.root,
        transaction_items,
    );
    let receipts_rlp = rlp_list(&[receipt_item]);

    let input = build_eth_block_input_with_receipts(&block_rlp, &receipts_rlp)
        .expect("block input should build with receipts");
    let input_receipts = input.receipts.as_ref().expect("receipts trie should exist");

    assert_eq!(input.receipts_rlp.as_deref(), Some(receipts_rlp.as_slice()));
    assert_eq!(input.receipts_root, receipt_build.root);
    assert_eq!(input_receipts.root, input.receipts_root);
    assert_eq!(input_receipts.hash_preimages, receipt_build.hash_preimages);

    let encoded = encode_eth_block_input(&input).expect("block input should encode");
    let file = parse_sectioned_file(&encoded, ETH_BLOCK_INPUT_KIND, ETH_BLOCK_INPUT_VERSION)
        .expect("sectioned input should parse");
    assert!(file
        .sections
        .iter()
        .any(|section| section.id == RECEIPT_PREIMAGES_SECTION_ID));
    assert!(file
        .sections
        .iter()
        .any(|section| section.id == RECEIPTS_RLP_SECTION_ID));

    let parsed = parse_eth_block_input(&encoded).expect("block input should parse");
    assert_eq!(
        parsed.receipts_rlp.as_deref(),
        Some(receipts_rlp.as_slice())
    );
    assert_eq!(parsed, input);
}

#[test]
fn rejects_receipts_rlp_without_receipt_preimages() {
    let receipt_item = receipt_item();
    let receipts = vec![parse_rlp(&receipt_item).expect("receipt item should parse")];
    let receipt_build = receipt_trie_build(&receipts);
    let transaction_items = vec![rlp_list(&[rlp_bytes(&[1])])];
    let block_rlp = sample_block_rlp_with_transactions_and_receipts(
        hex32("e52f61e61ebdce920205cfca55e00c70bf219b45ea432febbf96152313e61db5"),
        receipt_build.root,
        transaction_items,
    );
    let receipts_rlp = rlp_list(&[receipt_item]);
    let input = build_eth_block_input_with_receipts(&block_rlp, &receipts_rlp)
        .expect("block input should build with receipts");
    let encoded = encode_eth_block_input(&input).expect("block input should encode");
    let mut file = parse_sectioned_file(&encoded, ETH_BLOCK_INPUT_KIND, ETH_BLOCK_INPUT_VERSION)
        .expect("sectioned input should parse");
    file.sections
        .retain(|section| section.id != RECEIPT_PREIMAGES_SECTION_ID);
    let encoded = encode_sectioned_file(&file).expect("sectioned input should encode");

    let error = parse_eth_block_input(&encoded).expect_err("block input should reject receipts");

    assert!(matches!(error, EthBlockInputError::MissingReceiptPreimages));
}

#[test]
fn rejects_receipt_preimages_without_receipts_rlp() {
    let receipt_item = receipt_item();
    let receipts = vec![parse_rlp(&receipt_item).expect("receipt item should parse")];
    let receipt_build = receipt_trie_build(&receipts);
    let transaction_items = vec![rlp_list(&[rlp_bytes(&[1])])];
    let block_rlp = sample_block_rlp_with_transactions_and_receipts(
        hex32("e52f61e61ebdce920205cfca55e00c70bf219b45ea432febbf96152313e61db5"),
        receipt_build.root,
        transaction_items,
    );
    let receipts_rlp = rlp_list(&[receipt_item]);
    let input = build_eth_block_input_with_receipts(&block_rlp, &receipts_rlp)
        .expect("block input should build with receipts");
    let encoded = encode_eth_block_input(&input).expect("block input should encode");
    let mut file = parse_sectioned_file(&encoded, ETH_BLOCK_INPUT_KIND, ETH_BLOCK_INPUT_VERSION)
        .expect("sectioned input should parse");
    file.sections
        .retain(|section| section.id != RECEIPTS_RLP_SECTION_ID);
    let encoded = encode_sectioned_file(&file).expect("sectioned input should encode");

    let error = parse_eth_block_input(&encoded).expect_err("block input should reject receipts");

    assert!(matches!(error, EthBlockInputError::MissingReceiptsRlp));
}

#[test]
fn rejects_encoding_receipts_rlp_without_receipt_preimages() {
    let receipt_item = receipt_item();
    let receipts = vec![parse_rlp(&receipt_item).expect("receipt item should parse")];
    let receipt_build = receipt_trie_build(&receipts);
    let transaction_items = vec![rlp_list(&[rlp_bytes(&[1])])];
    let block_rlp = sample_block_rlp_with_transactions_and_receipts(
        hex32("e52f61e61ebdce920205cfca55e00c70bf219b45ea432febbf96152313e61db5"),
        receipt_build.root,
        transaction_items,
    );
    let receipts_rlp = rlp_list(&[receipt_item]);
    let mut input = build_eth_block_input_with_receipts(&block_rlp, &receipts_rlp)
        .expect("block input should build with receipts");
    input.receipts = None;

    let error = encode_eth_block_input(&input).expect_err("block input should reject receipts");

    assert!(matches!(error, EthBlockInputError::MissingReceiptPreimages));
}

#[test]
fn rejects_encoding_receipt_preimages_without_receipts_rlp() {
    let receipt_item = receipt_item();
    let receipts = vec![parse_rlp(&receipt_item).expect("receipt item should parse")];
    let receipt_build = receipt_trie_build(&receipts);
    let transaction_items = vec![rlp_list(&[rlp_bytes(&[1])])];
    let block_rlp = sample_block_rlp_with_transactions_and_receipts(
        hex32("e52f61e61ebdce920205cfca55e00c70bf219b45ea432febbf96152313e61db5"),
        receipt_build.root,
        transaction_items,
    );
    let receipts_rlp = rlp_list(&[receipt_item]);
    let mut input = build_eth_block_input_with_receipts(&block_rlp, &receipts_rlp)
        .expect("block input should build with receipts");
    input.receipts_rlp = None;

    let error = encode_eth_block_input(&input).expect_err("block input should reject receipts");

    assert!(matches!(error, EthBlockInputError::MissingReceiptsRlp));
}

#[test]
fn rejects_malformed_receipt_bodies() {
    let receipt_item = rlp_list(&[
        rlp_bytes(&[1]),
        rlp_bytes(&[0x52, 0x08]),
        rlp_bytes(&[0x11; 256]),
    ]);
    let receipts = vec![parse_rlp(&receipt_item).expect("receipt item should parse")];
    let receipt_build = receipt_trie_build(&receipts);
    let block_rlp = sample_block_rlp_with_transactions_receipts_and_logs_bloom(
        empty_trie_root(),
        receipt_build.root,
        [0x77; 256],
        Vec::new(),
    );
    let receipts_rlp = rlp_list(&[receipt_item]);

    let error = build_eth_block_input_with_receipts(&block_rlp, &receipts_rlp)
        .expect_err("block input should reject malformed receipts");

    assert_eq!(error.to_string(), "expected 4 receipt fields, found 3");
}

#[test]
fn rejects_block_logs_bloom_mismatches() {
    let receipt_item = receipt_item();
    let receipts = vec![parse_rlp(&receipt_item).expect("receipt item should parse")];
    let receipt_build = receipt_trie_build(&receipts);
    let block_rlp = sample_block_rlp_with_transactions_receipts_and_logs_bloom(
        empty_trie_root(),
        receipt_build.root,
        [0x77; 256],
        Vec::new(),
    );
    let receipts_rlp = rlp_list(&[receipt_item]);

    let error = build_eth_block_input_with_receipts(&block_rlp, &receipts_rlp)
        .expect_err("block input should reject mismatched block logs bloom");

    assert!(matches!(error, EthBlockInputError::LogsBloomMismatch));
}

#[test]
fn rejects_encoded_receipt_logs_bloom_mismatches() {
    let receipt_item = receipt_item();
    let receipts = vec![parse_rlp(&receipt_item).expect("receipt item should parse")];
    let receipt_build = receipt_trie_build(&receipts);
    let block_rlp = sample_block_rlp_with_transactions_receipts_and_logs_bloom(
        empty_trie_root(),
        receipt_build.root,
        [0x77; 256],
        Vec::new(),
    );
    let receipts_rlp = rlp_list(&[receipt_item]);
    let input = build_eth_block_input(&block_rlp).expect("block input should build");
    let encoded = encode_eth_block_input(&input).expect("block input should encode");
    let mut file = parse_sectioned_file(&encoded, ETH_BLOCK_INPUT_KIND, ETH_BLOCK_INPUT_VERSION)
        .expect("sectioned input should parse");
    file.sections.push(SectionedSection {
        id: RECEIPT_PREIMAGES_SECTION_ID,
        data: encode_hash_preimages(&receipt_build.hash_preimages),
    });
    file.sections.push(SectionedSection {
        id: RECEIPTS_RLP_SECTION_ID,
        data: receipts_rlp,
    });
    let encoded = encode_sectioned_file(&file).expect("sectioned input should encode");

    let error = parse_eth_block_input(&encoded).expect_err("block input should reject receipts");

    assert!(matches!(error, EthBlockInputError::LogsBloomMismatch));
}

#[test]
fn rejects_receipt_count_mismatches() {
    let receipt_item = receipt_item();
    let receipts = vec![parse_rlp(&receipt_item).expect("receipt item should parse")];
    let receipt_build = receipt_trie_build(&receipts);
    let block_rlp = sample_block_rlp_with_transactions_and_receipts(
        empty_trie_root(),
        receipt_build.root,
        Vec::new(),
    );
    let receipts_rlp = rlp_list(&[receipt_item]);

    let error = build_eth_block_input_with_receipts(&block_rlp, &receipts_rlp)
        .expect_err("block input should reject receipt count");

    assert!(matches!(error, EthBlockInputError::ReceiptCountMismatch));
}

#[test]
fn rejects_receipt_gas_used_mismatches() {
    let receipt_item = receipt_item();
    let receipts = vec![parse_rlp(&receipt_item).expect("receipt item should parse")];
    let receipt_build = receipt_trie_build(&receipts);
    let transaction_items = vec![rlp_list(&[rlp_bytes(&[1])])];
    let block_rlp = sample_block_rlp_with_transactions_receipts_logs_bloom_and_gas_used(
        hex32("e52f61e61ebdce920205cfca55e00c70bf219b45ea432febbf96152313e61db5"),
        receipt_build.root,
        [0; 256],
        &[0x0d, 0xbb, 0xa0],
        transaction_items,
    );
    let receipts_rlp = rlp_list(&[receipt_item]);

    let error = build_eth_block_input_with_receipts(&block_rlp, &receipts_rlp)
        .expect_err("block input should reject receipt gas used");

    assert!(matches!(error, EthBlockInputError::GasUsedMismatch));
}

#[test]
fn rejects_decreasing_receipt_cumulative_gas() {
    let receipt_items = vec![
        receipt_item_with_cumulative_gas(&[0x52, 0x08]),
        receipt_item_with_cumulative_gas(&[0x10]),
    ];
    let receipts = receipt_items
        .iter()
        .map(|item| parse_rlp(item).expect("receipt item should parse"))
        .collect::<Vec<_>>();
    let receipt_build = receipt_trie_build(&receipts);
    let transaction_items = vec![rlp_list(&[rlp_bytes(&[1])]), rlp_list(&[rlp_bytes(&[2])])];
    let transaction_rlp_items = transaction_items
        .iter()
        .map(|item| parse_rlp(item).expect("transaction item should parse"))
        .collect::<Vec<_>>();
    let transaction_build =
        transaction_trie_build(&transaction_rlp_items).expect("transaction trie should build");
    let block_rlp = sample_block_rlp_with_transactions_receipts_logs_bloom_and_gas_used(
        transaction_build.root,
        receipt_build.root,
        [0; 256],
        &[0x10],
        transaction_items,
    );
    let receipts_rlp = rlp_list(&receipt_items);

    let error = build_eth_block_input_with_receipts(&block_rlp, &receipts_rlp)
        .expect_err("block input should reject decreasing cumulative gas");

    assert!(matches!(error, EthBlockInputError::GasUsedMismatch));
}

#[test]
fn rejects_transaction_root_mismatches() {
    let block_rlp =
        sample_block_rlp_with_transactions([0x55; 32], vec![rlp_list(&[rlp_bytes(&[1])])]);

    let error = build_eth_block_input(&block_rlp).expect_err("block input should fail");

    assert!(matches!(
        error,
        EthBlockInputError::TransactionsRootMismatch
    ));
}

#[test]
fn rejects_gas_used_above_gas_limit() {
    let block_rlp = sample_block_rlp_with_transactions_and_gas(
        hex32("e52f61e61ebdce920205cfca55e00c70bf219b45ea432febbf96152313e61db5"),
        &[1],
        &[2],
        vec![rlp_list(&[rlp_bytes(&[1])])],
    );

    let error = build_eth_block_input(&block_rlp).expect_err("block input should reject gas");

    assert!(matches!(error, EthBlockInputError::GasUsedExceedsGasLimit));
}

#[test]
fn rejects_missing_withdrawals_body() {
    let header_rlp = rlp_list(&legacy_header_items(
        empty_trie_root(),
        Some(hex32(
            "56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421",
        )),
    ));
    let empty_list = rlp_list(&[]);
    let block_rlp = rlp_list(&[header_rlp, empty_list.clone(), empty_list]);

    let error = build_eth_block_input(&block_rlp).expect_err("block input should fail");

    assert!(matches!(
        error,
        EthBlockInputError::UnexpectedWithdrawalsRoot
    ));
}

#[test]
fn accepts_extra_header_fields() {
    let mut header_items = legacy_header_items(empty_trie_root(), Some(empty_trie_root()));
    header_items.push(rlp_bytes(&[0xee]));
    let header_rlp = rlp_list(&header_items);
    let empty_list = rlp_list(&[]);
    let block_rlp = rlp_list(&[
        header_rlp,
        empty_list.clone(),
        empty_list.clone(),
        empty_list,
    ]);

    let input = build_eth_block_input(&block_rlp).expect("block input should build");
    let encoded = encode_eth_block_input(&input).expect("block input should encode");
    let parsed = parse_eth_block_input(&encoded).expect("block input should parse");

    assert_eq!(input.block_rlp, block_rlp);
    assert_eq!(parsed, input);
}

#[test]
fn accepts_extra_body_fields() {
    let header_rlp = rlp_list(&legacy_header_items(
        empty_trie_root(),
        Some(empty_trie_root()),
    ));
    let empty_list = rlp_list(&[]);
    let block_rlp = rlp_list(&[
        header_rlp,
        empty_list.clone(),
        empty_list.clone(),
        empty_list,
        rlp_bytes(&[0xee]),
    ]);

    let input = build_eth_block_input(&block_rlp).expect("block input should build");
    let encoded = encode_eth_block_input(&input).expect("block input should encode");
    let parsed = parse_eth_block_input(&encoded).expect("block input should parse");

    assert_eq!(input.block_rlp, block_rlp);
    assert_eq!(parsed, input);
}

#[test]
fn rejects_preimage_hash_mismatches() {
    let block_rlp = sample_block_rlp_with_transactions(
        hex32("e52f61e61ebdce920205cfca55e00c70bf219b45ea432febbf96152313e61db5"),
        vec![rlp_list(&[rlp_bytes(&[1])])],
    );
    let input = build_eth_block_input(&block_rlp).expect("block input should build");
    let encoded = encode_eth_block_input(&input).expect("block input should encode");
    let mut file = parse_sectioned_file(&encoded, ETH_BLOCK_INPUT_KIND, ETH_BLOCK_INPUT_VERSION)
        .expect("sectioned input should parse");
    let transaction_preimages = file
        .sections
        .iter_mut()
        .find(|section| section.id == TRANSACTION_PREIMAGES_SECTION_ID)
        .expect("transaction preimage section should exist");
    transaction_preimages.data[4] ^= 1;
    let encoded = encode_sectioned_file(&file).expect("sectioned input should encode");

    let error = parse_eth_block_input(&encoded).expect_err("block input should fail");

    assert!(matches!(
        error,
        EthBlockInputError::PreimageHashMismatch {
            trie: EthBlockInputTrie::Transactions,
            index: 0,
        }
    ));
}

#[test]
fn rejects_encoding_transaction_preimage_hash_mismatches() {
    let block_rlp = sample_block_rlp_with_transactions(
        hex32("e52f61e61ebdce920205cfca55e00c70bf219b45ea432febbf96152313e61db5"),
        vec![rlp_list(&[rlp_bytes(&[1])])],
    );
    let mut input = build_eth_block_input(&block_rlp).expect("block input should build");
    input.transactions.hash_preimages[0].hash[0] ^= 1;

    let error =
        encode_eth_block_input(&input).expect_err("block input should reject preimage hash");

    assert!(matches!(
        error,
        EthBlockInputError::PreimageHashMismatch {
            trie: EthBlockInputTrie::Transactions,
            index: 0,
        }
    ));
}

#[test]
fn rejects_encoding_extra_transaction_preimages() {
    let block_rlp = sample_block_rlp_with_transactions(
        hex32("e52f61e61ebdce920205cfca55e00c70bf219b45ea432febbf96152313e61db5"),
        vec![rlp_list(&[rlp_bytes(&[1])])],
    );
    let mut input = build_eth_block_input(&block_rlp).expect("block input should build");
    input.transactions.hash_preimages.push(TrieHashPreimage {
        hash: empty_trie_root(),
        rlp: vec![0x80],
    });

    let error =
        encode_eth_block_input(&input).expect_err("block input should reject trie preimages");

    assert_eq!(
        error.to_string(),
        "ETH block input transactions trie preimages mismatch"
    );
}

#[test]
fn rejects_encoding_transaction_trie_root_mismatches() {
    let block_rlp = sample_block_rlp_with_transactions(
        hex32("e52f61e61ebdce920205cfca55e00c70bf219b45ea432febbf96152313e61db5"),
        vec![rlp_list(&[rlp_bytes(&[1])])],
    );
    let mut input = build_eth_block_input(&block_rlp).expect("block input should build");
    input.transactions.root[0] ^= 1;

    let error = encode_eth_block_input(&input).expect_err("block input should reject trie root");

    assert!(matches!(
        error,
        EthBlockInputError::TransactionsRootMismatch
    ));
}

#[test]
fn rejects_encoding_receipt_preimage_hash_mismatches() {
    let receipt_item = receipt_item();
    let receipts = vec![parse_rlp(&receipt_item).expect("receipt item should parse")];
    let receipt_build = receipt_trie_build(&receipts);
    let transaction_items = vec![rlp_list(&[rlp_bytes(&[1])])];
    let block_rlp = sample_block_rlp_with_transactions_and_receipts(
        hex32("e52f61e61ebdce920205cfca55e00c70bf219b45ea432febbf96152313e61db5"),
        receipt_build.root,
        transaction_items,
    );
    let receipts_rlp = rlp_list(&[receipt_item]);
    let mut input = build_eth_block_input_with_receipts(&block_rlp, &receipts_rlp)
        .expect("block input should build");
    input
        .receipts
        .as_mut()
        .expect("receipts should exist")
        .hash_preimages[0]
        .hash[0] ^= 1;

    let error =
        encode_eth_block_input(&input).expect_err("block input should reject preimage hash");

    assert!(matches!(
        error,
        EthBlockInputError::PreimageHashMismatch {
            trie: EthBlockInputTrie::Receipts,
            index: 0,
        }
    ));
}

#[test]
fn rejects_encoding_extra_receipt_preimages() {
    let receipt_item = receipt_item();
    let receipts = vec![parse_rlp(&receipt_item).expect("receipt item should parse")];
    let receipt_build = receipt_trie_build(&receipts);
    let transaction_items = vec![rlp_list(&[rlp_bytes(&[1])])];
    let block_rlp = sample_block_rlp_with_transactions_and_receipts(
        hex32("e52f61e61ebdce920205cfca55e00c70bf219b45ea432febbf96152313e61db5"),
        receipt_build.root,
        transaction_items,
    );
    let receipts_rlp = rlp_list(&[receipt_item]);
    let mut input = build_eth_block_input_with_receipts(&block_rlp, &receipts_rlp)
        .expect("block input should build");
    input
        .receipts
        .as_mut()
        .expect("receipts should exist")
        .hash_preimages
        .push(TrieHashPreimage {
            hash: empty_trie_root(),
            rlp: vec![0x80],
        });

    let error =
        encode_eth_block_input(&input).expect_err("block input should reject trie preimages");

    assert_eq!(
        error.to_string(),
        "ETH block input receipts trie preimages mismatch"
    );
}

#[test]
fn rejects_encoding_receipt_trie_root_mismatches() {
    let receipt_item = receipt_item();
    let receipts = vec![parse_rlp(&receipt_item).expect("receipt item should parse")];
    let receipt_build = receipt_trie_build(&receipts);
    let transaction_items = vec![rlp_list(&[rlp_bytes(&[1])])];
    let block_rlp = sample_block_rlp_with_transactions_and_receipts(
        hex32("e52f61e61ebdce920205cfca55e00c70bf219b45ea432febbf96152313e61db5"),
        receipt_build.root,
        transaction_items,
    );
    let receipts_rlp = rlp_list(&[receipt_item]);
    let mut input = build_eth_block_input_with_receipts(&block_rlp, &receipts_rlp)
        .expect("block input should build");
    input.receipts.as_mut().expect("receipts should exist").root[0] ^= 1;

    let error = encode_eth_block_input(&input).expect_err("block input should reject trie root");

    assert!(matches!(error, EthBlockInputError::ReceiptsRootMismatch));
}

#[test]
fn rejects_encoding_withdrawal_preimage_hash_mismatches() {
    let block_rlp = sample_block_rlp_with_withdrawals(
        hex32("51c445cba96d0dfd446eec8b2b94f104608cf8443a92f7f87c76a383d6687300"),
        vec![withdrawal_item()],
    );
    let mut input = build_eth_block_input(&block_rlp).expect("block input should build");
    input
        .withdrawals
        .as_mut()
        .expect("withdrawals should exist")
        .hash_preimages[0]
        .hash[0] ^= 1;

    let error =
        encode_eth_block_input(&input).expect_err("block input should reject preimage hash");

    assert!(matches!(
        error,
        EthBlockInputError::PreimageHashMismatch {
            trie: EthBlockInputTrie::Withdrawals,
            index: 0,
        }
    ));
}

#[test]
fn rejects_encoding_extra_withdrawal_preimages() {
    let block_rlp = sample_block_rlp_with_withdrawals(
        hex32("51c445cba96d0dfd446eec8b2b94f104608cf8443a92f7f87c76a383d6687300"),
        vec![withdrawal_item()],
    );
    let mut input = build_eth_block_input(&block_rlp).expect("block input should build");
    input
        .withdrawals
        .as_mut()
        .expect("withdrawals should exist")
        .hash_preimages
        .push(TrieHashPreimage {
            hash: empty_trie_root(),
            rlp: vec![0x80],
        });

    let error =
        encode_eth_block_input(&input).expect_err("block input should reject trie preimages");

    assert_eq!(
        error.to_string(),
        "ETH block input withdrawals trie preimages mismatch"
    );
}

#[test]
fn rejects_encoding_withdrawal_trie_root_mismatches() {
    let block_rlp = sample_block_rlp_with_withdrawals(
        hex32("51c445cba96d0dfd446eec8b2b94f104608cf8443a92f7f87c76a383d6687300"),
        vec![withdrawal_item()],
    );
    let mut input = build_eth_block_input(&block_rlp).expect("block input should build");
    input
        .withdrawals
        .as_mut()
        .expect("withdrawals should exist")
        .root[0] ^= 1;

    let error = encode_eth_block_input(&input).expect_err("block input should reject trie root");

    assert!(matches!(error, EthBlockInputError::WithdrawalsRootMismatch));
}

#[test]
fn rejects_encoding_block_hash_mismatches() {
    let block_rlp = sample_block_rlp_with_transactions(
        hex32("e52f61e61ebdce920205cfca55e00c70bf219b45ea432febbf96152313e61db5"),
        vec![rlp_list(&[rlp_bytes(&[1])])],
    );
    let mut input = build_eth_block_input(&block_rlp).expect("block input should build");
    input.block_hash[0] ^= 1;

    let error = encode_eth_block_input(&input).expect_err("block input should reject block hash");

    assert!(matches!(error, EthBlockInputError::BlockHashMismatch));
}

#[test]
fn rejects_missing_transaction_root_preimages() {
    let block_rlp = sample_block_rlp_with_transactions(
        hex32("e52f61e61ebdce920205cfca55e00c70bf219b45ea432febbf96152313e61db5"),
        vec![rlp_list(&[rlp_bytes(&[1])])],
    );
    let input = build_eth_block_input(&block_rlp).expect("block input should build");
    let encoded = encode_eth_block_input(&input).expect("block input should encode");
    let mut file = parse_sectioned_file(&encoded, ETH_BLOCK_INPUT_KIND, ETH_BLOCK_INPUT_VERSION)
        .expect("sectioned input should parse");
    let transaction_preimages = file
        .sections
        .iter_mut()
        .find(|section| section.id == TRANSACTION_PREIMAGES_SECTION_ID)
        .expect("transaction preimage section should exist");
    transaction_preimages.data = encode_preimage_section(&[(empty_trie_root(), vec![0x80])]);
    let encoded = encode_sectioned_file(&file).expect("sectioned input should encode");

    let error = parse_eth_block_input(&encoded).expect_err("block input should fail");

    assert!(matches!(
        error,
        EthBlockInputError::MissingRootPreimage {
            trie: EthBlockInputTrie::Transactions,
        }
    ));
}

#[test]
fn rejects_extra_transaction_preimages() {
    let block_rlp = sample_block_rlp_with_transactions(
        hex32("e52f61e61ebdce920205cfca55e00c70bf219b45ea432febbf96152313e61db5"),
        vec![rlp_list(&[rlp_bytes(&[1])])],
    );
    let input = build_eth_block_input(&block_rlp).expect("block input should build");
    let encoded = encode_eth_block_input(&input).expect("block input should encode");
    let mut file = parse_sectioned_file(&encoded, ETH_BLOCK_INPUT_KIND, ETH_BLOCK_INPUT_VERSION)
        .expect("sectioned input should parse");
    let transaction_preimages = file
        .sections
        .iter_mut()
        .find(|section| section.id == TRANSACTION_PREIMAGES_SECTION_ID)
        .expect("transaction preimage section should exist");
    let mut preimages = input.transactions.hash_preimages.clone();
    preimages.push(TrieHashPreimage {
        hash: empty_trie_root(),
        rlp: vec![0x80],
    });
    transaction_preimages.data = encode_hash_preimages(&preimages);
    let encoded = encode_sectioned_file(&file).expect("sectioned input should encode");

    let error = parse_eth_block_input(&encoded).expect_err("block input should fail");

    assert_eq!(
        error.to_string(),
        "ETH block input transactions trie preimages mismatch"
    );
}

#[test]
fn rejects_extra_receipt_preimages() {
    let receipt_item = receipt_item();
    let receipts = vec![parse_rlp(&receipt_item).expect("receipt item should parse")];
    let receipt_build = receipt_trie_build(&receipts);
    let transaction_items = vec![rlp_list(&[rlp_bytes(&[1])])];
    let block_rlp = sample_block_rlp_with_transactions_and_receipts(
        hex32("e52f61e61ebdce920205cfca55e00c70bf219b45ea432febbf96152313e61db5"),
        receipt_build.root,
        transaction_items,
    );
    let receipts_rlp = rlp_list(&[receipt_item]);
    let input = build_eth_block_input_with_receipts(&block_rlp, &receipts_rlp)
        .expect("block input should build");
    let encoded = encode_eth_block_input(&input).expect("block input should encode");
    let mut file = parse_sectioned_file(&encoded, ETH_BLOCK_INPUT_KIND, ETH_BLOCK_INPUT_VERSION)
        .expect("sectioned input should parse");
    let receipt_preimages = file
        .sections
        .iter_mut()
        .find(|section| section.id == RECEIPT_PREIMAGES_SECTION_ID)
        .expect("receipt preimage section should exist");
    let mut preimages = input
        .receipts
        .as_ref()
        .expect("receipts should exist")
        .hash_preimages
        .clone();
    preimages.push(TrieHashPreimage {
        hash: empty_trie_root(),
        rlp: vec![0x80],
    });
    receipt_preimages.data = encode_hash_preimages(&preimages);
    let encoded = encode_sectioned_file(&file).expect("sectioned input should encode");

    let error = parse_eth_block_input(&encoded).expect_err("block input should fail");

    assert_eq!(
        error.to_string(),
        "ETH block input receipts trie preimages mismatch"
    );
}

#[test]
fn rejects_extra_withdrawal_preimages() {
    let block_rlp = sample_block_rlp_with_withdrawals(
        hex32("51c445cba96d0dfd446eec8b2b94f104608cf8443a92f7f87c76a383d6687300"),
        vec![withdrawal_item()],
    );
    let input = build_eth_block_input(&block_rlp).expect("block input should build");
    let encoded = encode_eth_block_input(&input).expect("block input should encode");
    let mut file = parse_sectioned_file(&encoded, ETH_BLOCK_INPUT_KIND, ETH_BLOCK_INPUT_VERSION)
        .expect("sectioned input should parse");
    let withdrawal_preimages = file
        .sections
        .iter_mut()
        .find(|section| section.id == WITHDRAWAL_PREIMAGES_SECTION_ID)
        .expect("withdrawal preimage section should exist");
    let mut preimages = input
        .withdrawals
        .as_ref()
        .expect("withdrawals should exist")
        .hash_preimages
        .clone();
    preimages.push(TrieHashPreimage {
        hash: empty_trie_root(),
        rlp: vec![0x80],
    });
    withdrawal_preimages.data = encode_hash_preimages(&preimages);
    let encoded = encode_sectioned_file(&file).expect("sectioned input should encode");

    let error = parse_eth_block_input(&encoded).expect_err("block input should fail");

    assert_eq!(
        error.to_string(),
        "ETH block input withdrawals trie preimages mismatch"
    );
}

#[test]
fn rejects_missing_transaction_child_preimages() {
    let transaction_items = vec![typed_transaction_item(1), typed_transaction_item(2)];
    let transactions = transaction_items
        .iter()
        .map(|item| parse_rlp(item).expect("transaction item should parse"))
        .collect::<Vec<_>>();
    let trie = transaction_trie_build(&transactions).expect("transaction trie should build");
    let block_rlp = sample_block_rlp_with_transactions(trie.root, transaction_items);
    let input = build_eth_block_input(&block_rlp).expect("block input should build");
    assert!(
        input.transactions.hash_preimages.len() > 2,
        "fixture should contain child preimages"
    );
    let encoded = encode_eth_block_input(&input).expect("block input should encode");
    let mut file = parse_sectioned_file(&encoded, ETH_BLOCK_INPUT_KIND, ETH_BLOCK_INPUT_VERSION)
        .expect("sectioned input should parse");
    let transaction_preimages = file
        .sections
        .iter_mut()
        .find(|section| section.id == TRANSACTION_PREIMAGES_SECTION_ID)
        .expect("transaction preimage section should exist");
    let retained = input
        .transactions
        .hash_preimages
        .iter()
        .enumerate()
        .filter(|(index, _)| *index != 1)
        .map(|(_, preimage)| (preimage.hash, preimage.rlp.clone()))
        .collect::<Vec<_>>();
    transaction_preimages.data = encode_preimage_section(&retained);
    let encoded = encode_sectioned_file(&file).expect("sectioned input should encode");

    let error = parse_eth_block_input(&encoded).expect_err("block input should fail");

    assert_eq!(
        error.to_string(),
        "ETH block input transactions child preimage missing at 0"
    );
}

#[test]
fn rejects_metadata_block_hash_mismatches() {
    let block_rlp = sample_block_rlp_with_transactions(
        hex32("e52f61e61ebdce920205cfca55e00c70bf219b45ea432febbf96152313e61db5"),
        vec![rlp_list(&[rlp_bytes(&[1])])],
    );
    let input = build_eth_block_input(&block_rlp).expect("block input should build");
    let encoded = encode_eth_block_input(&input).expect("block input should encode");
    let mut file = parse_sectioned_file(&encoded, ETH_BLOCK_INPUT_KIND, ETH_BLOCK_INPUT_VERSION)
        .expect("sectioned input should parse");
    let metadata = file
        .sections
        .iter_mut()
        .find(|section| section.id == METADATA_SECTION_ID)
        .expect("metadata section should exist");
    metadata.data[0] ^= 1;
    let encoded = encode_sectioned_file(&file).expect("sectioned input should encode");

    let error = parse_eth_block_input(&encoded).expect_err("block input should fail");

    assert!(matches!(error, EthBlockInputError::BlockHashMismatch));
}

fn sample_block_rlp_with_transactions(
    transactions_root: [u8; 32],
    transaction_items: Vec<Vec<u8>>,
) -> Vec<u8> {
    let header_rlp = rlp_list(&legacy_header_items(transactions_root, None));
    let transactions = rlp_list(&transaction_items);
    let empty_list = rlp_list(&[]);
    rlp_list(&[header_rlp, transactions, empty_list])
}

fn sample_block_rlp_with_transactions_and_gas(
    transactions_root: [u8; 32],
    gas_limit: &[u8],
    gas_used: &[u8],
    transaction_items: Vec<Vec<u8>>,
) -> Vec<u8> {
    let mut header_items = legacy_header_items(transactions_root, None);
    header_items[9] = rlp_bytes(gas_limit);
    header_items[10] = rlp_bytes(gas_used);
    let header_rlp = rlp_list(&header_items);
    let transactions = rlp_list(&transaction_items);
    let empty_list = rlp_list(&[]);
    rlp_list(&[header_rlp, transactions, empty_list])
}

fn sample_block_rlp_with_transactions_and_receipts(
    transactions_root: [u8; 32],
    receipts_root: [u8; 32],
    transaction_items: Vec<Vec<u8>>,
) -> Vec<u8> {
    sample_block_rlp_with_transactions_receipts_and_logs_bloom(
        transactions_root,
        receipts_root,
        [0; 256],
        transaction_items,
    )
}

fn sample_block_rlp_with_transactions_receipts_and_logs_bloom(
    transactions_root: [u8; 32],
    receipts_root: [u8; 32],
    logs_bloom: [u8; 256],
    transaction_items: Vec<Vec<u8>>,
) -> Vec<u8> {
    sample_block_rlp_with_transactions_receipts_logs_bloom_and_gas_used(
        transactions_root,
        receipts_root,
        logs_bloom,
        &[0x52, 0x08],
        transaction_items,
    )
}

fn sample_block_rlp_with_transactions_receipts_logs_bloom_and_gas_used(
    transactions_root: [u8; 32],
    receipts_root: [u8; 32],
    logs_bloom: [u8; 256],
    gas_used: &[u8],
    transaction_items: Vec<Vec<u8>>,
) -> Vec<u8> {
    let header_rlp = rlp_list(&legacy_header_items_with_receipts(
        transactions_root,
        receipts_root,
        logs_bloom,
        gas_used,
        None,
    ));
    let transactions = rlp_list(&transaction_items);
    let empty_list = rlp_list(&[]);
    rlp_list(&[header_rlp, transactions, empty_list])
}

fn sample_block_rlp_with_withdrawals(
    withdrawals_root: [u8; 32],
    withdrawal_items: Vec<Vec<u8>>,
) -> Vec<u8> {
    let header_rlp = rlp_list(&legacy_header_items(
        empty_trie_root(),
        Some(withdrawals_root),
    ));
    let empty_list = rlp_list(&[]);
    let withdrawals = rlp_list(&withdrawal_items);
    rlp_list(&[header_rlp, empty_list.clone(), empty_list, withdrawals])
}

fn legacy_header_items(
    transactions_root: [u8; 32],
    withdrawals_root: Option<[u8; 32]>,
) -> Vec<Vec<u8>> {
    legacy_header_items_with_receipts(
        transactions_root,
        [0x66; 32],
        [0x77; 256],
        &[0x0d, 0xbb, 0xa0],
        withdrawals_root,
    )
}

fn legacy_header_items_with_receipts(
    transactions_root: [u8; 32],
    receipts_root: [u8; 32],
    logs_bloom: [u8; 256],
    gas_used: &[u8],
    withdrawals_root: Option<[u8; 32]>,
) -> Vec<Vec<u8>> {
    let mut items = vec![
        rlp_bytes(&[0x11; 32]),
        rlp_bytes(&hex32(
            "1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
        )),
        rlp_bytes(&[0x33; 20]),
        rlp_bytes(&[0x44; 32]),
        rlp_bytes(&transactions_root),
        rlp_bytes(&receipts_root),
        rlp_bytes(&logs_bloom),
        rlp_bytes(&[1]),
        rlp_bytes(&[2]),
        rlp_bytes(&[0x0f, 0x42, 0x40]),
        rlp_bytes(gas_used),
        rlp_bytes(&[0x65]),
        rlp_bytes(b"lzvm"),
        rlp_bytes(&[0xaa; 32]),
        rlp_bytes(&[0xbb; 8]),
    ];
    if let Some(root) = withdrawals_root {
        items.push(rlp_bytes(&[1]));
        items.push(rlp_bytes(&root));
    }
    items
}

fn receipt_item() -> Vec<u8> {
    receipt_item_with_cumulative_gas(&[0x52, 0x08])
}

fn receipt_item_with_cumulative_gas(cumulative_gas_used: &[u8]) -> Vec<u8> {
    rlp_list(&[
        rlp_bytes(&[1]),
        rlp_bytes(cumulative_gas_used),
        rlp_bytes(&[0; 256]),
        rlp_list(&[]),
    ])
}

fn typed_transaction_item(byte: u8) -> Vec<u8> {
    let mut bytes = vec![1];
    bytes.extend_from_slice(&rlp_list(&[rlp_bytes(&[byte; 40])]));
    rlp_bytes(&bytes)
}

fn encode_hash_preimages(preimages: &[TrieHashPreimage]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(
        &u32::try_from(preimages.len())
            .expect("preimage count should fit")
            .to_le_bytes(),
    );
    for preimage in preimages {
        out.extend_from_slice(&preimage.hash);
        out.extend_from_slice(
            &u64::try_from(preimage.rlp.len())
                .expect("preimage length should fit")
                .to_le_bytes(),
        );
        out.extend_from_slice(&preimage.rlp);
    }
    out
}

fn withdrawal_item() -> Vec<u8> {
    rlp_list(&[
        rlp_bytes(&[]),
        rlp_bytes(&[1]),
        rlp_bytes(&[0x22; 20]),
        rlp_bytes(&[0x40]),
    ])
}

fn empty_trie_root() -> [u8; 32] {
    hex32("56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421")
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

fn encode_preimage_section(preimages: &[([u8; 32], Vec<u8>)]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&(preimages.len() as u32).to_le_bytes());
    for (hash, rlp) in preimages {
        out.extend_from_slice(hash);
        out.extend_from_slice(&(rlp.len() as u64).to_le_bytes());
        out.extend_from_slice(rlp);
    }
    out
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
