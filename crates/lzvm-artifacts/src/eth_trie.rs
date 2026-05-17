use crate::eth_block::keccak256;
use crate::rlp::{encode_rlp, RlpItem};

pub fn empty_trie_root() -> [u8; 32] {
    keccak256(&encode_rlp(&RlpItem::Bytes(Vec::new())))
}

pub fn empty_transaction_trie_root() -> [u8; 32] {
    empty_trie_root()
}
