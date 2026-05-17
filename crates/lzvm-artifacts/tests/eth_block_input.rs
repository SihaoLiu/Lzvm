use lzvm_artifacts::eth_block_input::{
    build_eth_block_input, encode_eth_block_input, parse_eth_block_input, EthBlockInputError,
};

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

fn sample_block_rlp_with_transactions(
    transactions_root: [u8; 32],
    transaction_items: Vec<Vec<u8>>,
) -> Vec<u8> {
    let header_rlp = rlp_list(&legacy_header_items(transactions_root, None));
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
    let mut items = vec![
        rlp_bytes(&[0x11; 32]),
        rlp_bytes(&hex32(
            "1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
        )),
        rlp_bytes(&[0x33; 20]),
        rlp_bytes(&[0x44; 32]),
        rlp_bytes(&transactions_root),
        rlp_bytes(&[0x66; 32]),
        rlp_bytes(&[0x77; 256]),
        rlp_bytes(&[1]),
        rlp_bytes(&[2]),
        rlp_bytes(&[0x0f, 0x42, 0x40]),
        rlp_bytes(&[0x0d, 0xbb, 0xa0]),
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
