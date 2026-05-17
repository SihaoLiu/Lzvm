use lzvm_artifacts::eth_trie::{
    compact_encode_nibbles, empty_transaction_trie_root, empty_trie_root, receipt_trie_build,
    receipt_trie_root, transaction_trie_build, transaction_trie_root, withdrawals_trie_build,
    withdrawals_trie_root,
};
use lzvm_artifacts::rlp::{encode_rlp, RlpItem};

#[test]
fn computes_empty_trie_root() {
    assert_eq!(
        empty_trie_root(),
        hex32("56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421")
    );
}

#[test]
fn computes_empty_transaction_trie_root() {
    assert_eq!(empty_transaction_trie_root(), empty_trie_root());
    assert_eq!(
        transaction_trie_root(&[]).expect("empty transaction trie should build"),
        empty_trie_root()
    );
}

#[test]
fn computes_empty_withdrawals_trie_root() {
    assert_eq!(withdrawals_trie_root(&[]), empty_trie_root());
}

#[test]
fn empty_transaction_trie_root_matches_genesis_header_root() {
    let header = mainnet_genesis_header_items();
    let transactions_root = match &header[4] {
        RlpItem::Bytes(bytes) => bytes.clone(),
        RlpItem::List(_) => panic!("transactions root should be bytes"),
    };

    assert_eq!(empty_transaction_trie_root(), transactions_root.as_slice());
}

#[test]
fn compact_encodes_leaf_paths() {
    assert_eq!(compact_encode_nibbles(&[1, 2, 3], true), vec![0x31, 0x23]);
    assert_eq!(compact_encode_nibbles(&[15, 1], true), vec![0x20, 0xf1]);
}

#[test]
fn compact_encodes_extension_paths() {
    assert_eq!(compact_encode_nibbles(&[1, 2, 3], false), vec![0x11, 0x23]);
    assert_eq!(
        compact_encode_nibbles(&[15, 1, 12, 11], false),
        vec![0x00, 0xf1, 0xcb]
    );
}

#[test]
fn computes_single_legacy_transaction_trie_root() {
    let transaction = legacy_transaction();
    let transaction_bytes = encode_rlp(&transaction);
    let expected_leaf = RlpItem::List(vec![
        RlpItem::Bytes(compact_encode_nibbles(&[8, 0], true)),
        RlpItem::Bytes(transaction_bytes),
    ]);

    assert_eq!(
        transaction_trie_root(&[transaction]).expect("transaction trie should build"),
        lzvm_artifacts::eth_block::keccak256(&encode_rlp(&expected_leaf))
    );
}

#[test]
fn computes_typed_transaction_branch_trie_root() {
    let first = RlpItem::Bytes(vec![1, 0xc0]);
    let second = RlpItem::Bytes(vec![2, 0xc0]);

    let mut branch = Vec::with_capacity(17);
    branch.push(RlpItem::List(vec![
        RlpItem::Bytes(compact_encode_nibbles(&[1], true)),
        RlpItem::Bytes(vec![2, 0xc0]),
    ]));
    for _ in 1..8 {
        branch.push(RlpItem::Bytes(Vec::new()));
    }
    branch.push(RlpItem::List(vec![
        RlpItem::Bytes(compact_encode_nibbles(&[0], true)),
        RlpItem::Bytes(vec![1, 0xc0]),
    ]));
    for _ in 9..16 {
        branch.push(RlpItem::Bytes(Vec::new()));
    }
    branch.push(RlpItem::Bytes(Vec::new()));

    assert_eq!(
        transaction_trie_root(&[first, second]).expect("transaction trie should build"),
        lzvm_artifacts::eth_block::keccak256(&encode_rlp(&RlpItem::List(branch)))
    );
}

#[test]
fn computes_single_withdrawal_trie_root() {
    let withdrawal = withdrawal();
    let withdrawal_bytes = encode_rlp(&withdrawal);
    let expected_leaf = RlpItem::List(vec![
        RlpItem::Bytes(compact_encode_nibbles(&[8, 0], true)),
        RlpItem::Bytes(withdrawal_bytes),
    ]);

    assert_eq!(
        withdrawals_trie_root(&[withdrawal]),
        lzvm_artifacts::eth_block::keccak256(&encode_rlp(&expected_leaf))
    );
}

#[test]
fn computes_single_receipt_trie_root() {
    let receipt = legacy_receipt();
    let receipt_bytes = encode_rlp(&receipt);
    let expected_leaf = RlpItem::List(vec![
        RlpItem::Bytes(compact_encode_nibbles(&[8, 0], true)),
        RlpItem::Bytes(receipt_bytes),
    ]);

    assert_eq!(
        receipt_trie_root(&[receipt]),
        lzvm_artifacts::eth_block::keccak256(&encode_rlp(&expected_leaf))
    );
}

#[test]
fn computes_typed_receipt_branch_trie_root() {
    let first = RlpItem::Bytes(vec![1, 0xc0]);
    let second = RlpItem::Bytes(vec![2, 0xc0]);

    let mut branch = Vec::with_capacity(17);
    branch.push(RlpItem::List(vec![
        RlpItem::Bytes(compact_encode_nibbles(&[1], true)),
        RlpItem::Bytes(vec![2, 0xc0]),
    ]));
    for _ in 1..8 {
        branch.push(RlpItem::Bytes(Vec::new()));
    }
    branch.push(RlpItem::List(vec![
        RlpItem::Bytes(compact_encode_nibbles(&[0], true)),
        RlpItem::Bytes(vec![1, 0xc0]),
    ]));
    for _ in 9..16 {
        branch.push(RlpItem::Bytes(Vec::new()));
    }
    branch.push(RlpItem::Bytes(Vec::new()));

    assert_eq!(
        receipt_trie_root(&[first, second]),
        lzvm_artifacts::eth_block::keccak256(&encode_rlp(&RlpItem::List(branch)))
    );
}

#[test]
fn transaction_trie_build_records_root_preimage() {
    let transaction = legacy_transaction();
    let build =
        transaction_trie_build(std::slice::from_ref(&transaction)).expect("trie should build");

    assert_eq!(
        build.root,
        transaction_trie_root(&[transaction]).expect("root should build")
    );
    assert!(!build.hash_preimages.is_empty());
    assert!(build.hash_preimages.iter().any(|preimage| {
        preimage.hash == build.root
            && lzvm_artifacts::eth_block::keccak256(&preimage.rlp) == build.root
    }));
}

#[test]
fn transaction_trie_build_records_hashed_child_preimages() {
    let first = typed_transaction(1);
    let second = typed_transaction(2);
    let build = transaction_trie_build(&[first, second]).expect("trie should build");

    assert!(
        build.hash_preimages.len() >= 3,
        "expected root plus hashed child preimages"
    );
    for preimage in &build.hash_preimages {
        assert_eq!(
            lzvm_artifacts::eth_block::keccak256(&preimage.rlp),
            preimage.hash
        );
    }
}

#[test]
fn withdrawals_trie_build_records_empty_root_preimage() {
    let build = withdrawals_trie_build(&[]);

    assert_eq!(build.root, empty_trie_root());
    assert_eq!(build.hash_preimages.len(), 1);
    assert_eq!(build.hash_preimages[0].hash, empty_trie_root());
    assert_eq!(build.hash_preimages[0].rlp, vec![0x80]);
}

#[test]
fn receipt_trie_build_records_empty_root_preimage() {
    let build = receipt_trie_build(&[]);

    assert_eq!(build.root, empty_trie_root());
    assert_eq!(build.hash_preimages.len(), 1);
    assert_eq!(build.hash_preimages[0].hash, empty_trie_root());
    assert_eq!(build.hash_preimages[0].rlp, vec![0x80]);
}

fn legacy_transaction() -> RlpItem {
    RlpItem::List(vec![
        RlpItem::Bytes(Vec::new()),
        RlpItem::Bytes(vec![1]),
        RlpItem::Bytes(vec![0x52, 0x08]),
        RlpItem::Bytes(vec![0x11; 20]),
        RlpItem::Bytes(Vec::new()),
        RlpItem::Bytes(Vec::new()),
        RlpItem::Bytes(vec![0x1b]),
        RlpItem::Bytes(vec![1]),
        RlpItem::Bytes(vec![1]),
    ])
}

fn legacy_receipt() -> RlpItem {
    RlpItem::List(vec![
        RlpItem::Bytes(vec![1]),
        RlpItem::Bytes(vec![0x52, 0x08]),
        RlpItem::Bytes(vec![0x11; 256]),
        RlpItem::List(vec![]),
    ])
}

fn typed_transaction(byte: u8) -> RlpItem {
    let mut bytes = vec![1];
    bytes.extend_from_slice(&encode_rlp(&RlpItem::List(vec![RlpItem::Bytes(vec![
        byte;
        40
    ])])));
    RlpItem::Bytes(bytes)
}

fn withdrawal() -> RlpItem {
    RlpItem::List(vec![
        RlpItem::Bytes(Vec::new()),
        RlpItem::Bytes(vec![1]),
        RlpItem::Bytes(vec![0x22; 20]),
        RlpItem::Bytes(vec![0x40]),
    ])
}

fn mainnet_genesis_header_items() -> Vec<RlpItem> {
    vec![
        RlpItem::Bytes(vec![0; 32]),
        RlpItem::Bytes(
            hex32("1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347").to_vec(),
        ),
        RlpItem::Bytes(vec![0; 20]),
        RlpItem::Bytes(
            hex32("d7f8974fb5ac78d9ac099b9ad5018bedc2ce0a72dad1827a1709da30580f0544").to_vec(),
        ),
        RlpItem::Bytes(
            hex32("56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421").to_vec(),
        ),
        RlpItem::Bytes(
            hex32("56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421").to_vec(),
        ),
        RlpItem::Bytes(vec![0; 256]),
        RlpItem::Bytes(vec![0x04, 0x00, 0x00, 0x00, 0x00]),
        RlpItem::Bytes(Vec::new()),
        RlpItem::Bytes(vec![0x13, 0x88]),
        RlpItem::Bytes(Vec::new()),
        RlpItem::Bytes(Vec::new()),
        RlpItem::Bytes(hex_bytes(
            "11bbe8db4e347b4e8c937c1c8370e4b5ed33adb3db69cbdb7a38e1e50b1b82fa",
        )),
        RlpItem::Bytes(vec![0; 32]),
        RlpItem::Bytes(vec![0, 0, 0, 0, 0, 0, 0, 0x42]),
    ]
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
