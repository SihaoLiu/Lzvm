use lzvm_artifacts::eth_trie::{empty_transaction_trie_root, empty_trie_root};
use lzvm_artifacts::rlp::RlpItem;

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
