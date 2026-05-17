use crate::eth_block::keccak256;
use crate::rlp::{encode_rlp, RlpItem};

pub fn empty_trie_root() -> [u8; 32] {
    keccak256(&encode_rlp(&RlpItem::Bytes(Vec::new())))
}

pub fn empty_transaction_trie_root() -> [u8; 32] {
    empty_trie_root()
}

pub fn compact_encode_nibbles(nibbles: &[u8], terminator: bool) -> Vec<u8> {
    assert!(nibbles.iter().all(|nibble| *nibble < 16));

    let odd = nibbles.len() % 2 == 1;
    let flag = (if terminator { 2 } else { 0 }) + u8::from(odd);
    let mut output = Vec::with_capacity((nibbles.len() + 2) / 2);
    let mut index = 0;

    if odd {
        output.push((flag << 4) | nibbles[0]);
        index = 1;
    } else {
        output.push(flag << 4);
    }

    while index < nibbles.len() {
        output.push((nibbles[index] << 4) | nibbles[index + 1]);
        index += 2;
    }

    output
}
