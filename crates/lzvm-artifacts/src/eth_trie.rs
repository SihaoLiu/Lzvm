use crate::eth_block::{
    decode_eth_transaction_rlp, keccak256, EthTransactionError, EthTransactionRlp,
};
use crate::rlp::{encode_rlp, RlpItem};

pub fn empty_trie_root() -> [u8; 32] {
    keccak256(&encode_rlp(&RlpItem::Bytes(Vec::new())))
}

pub fn empty_transaction_trie_root() -> [u8; 32] {
    empty_trie_root()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexedTrieBuild {
    pub root: [u8; 32],
    pub hash_preimages: Vec<TrieHashPreimage>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrieHashPreimage {
    pub hash: [u8; 32],
    pub rlp: Vec<u8>,
}

pub fn transaction_trie_root(transactions: &[RlpItem]) -> Result<[u8; 32], EthTransactionError> {
    Ok(transaction_trie_build(transactions)?.root)
}

pub fn transaction_trie_build(
    transactions: &[RlpItem],
) -> Result<IndexedTrieBuild, EthTransactionError> {
    let values = transactions
        .iter()
        .map(transaction_value_bytes)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(indexed_trie_build(&values))
}

pub fn receipt_trie_root(receipts: &[RlpItem]) -> [u8; 32] {
    receipt_trie_build(receipts).root
}

pub fn receipt_trie_build(receipts: &[RlpItem]) -> IndexedTrieBuild {
    let values = receipts.iter().map(receipt_value_bytes).collect::<Vec<_>>();
    indexed_trie_build(&values)
}

pub fn withdrawals_trie_root(withdrawals: &[RlpItem]) -> [u8; 32] {
    withdrawals_trie_build(withdrawals).root
}

pub fn withdrawals_trie_build(withdrawals: &[RlpItem]) -> IndexedTrieBuild {
    indexed_item_trie_build(withdrawals)
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct TrieEntry {
    path: Vec<u8>,
    value: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TrieNode {
    Leaf {
        path: Vec<u8>,
        value: Vec<u8>,
    },
    Extension {
        path: Vec<u8>,
        child: Box<TrieNode>,
    },
    Branch {
        children: Vec<Option<Box<TrieNode>>>,
        value: Option<Vec<u8>>,
    },
}

fn transaction_value_bytes(transaction: &RlpItem) -> Result<Vec<u8>, EthTransactionError> {
    match decode_eth_transaction_rlp(transaction)? {
        EthTransactionRlp::Legacy(_) => Ok(encode_rlp(transaction)),
        EthTransactionRlp::Typed {
            transaction_type,
            payload,
        } => {
            let mut bytes = Vec::with_capacity(1 + payload.len());
            bytes.push(transaction_type);
            bytes.extend_from_slice(&payload);
            Ok(bytes)
        }
    }
}

fn receipt_value_bytes(receipt: &RlpItem) -> Vec<u8> {
    match receipt {
        RlpItem::List(_) => encode_rlp(receipt),
        RlpItem::Bytes(bytes) => bytes.clone(),
    }
}

fn indexed_item_trie_build(items: &[RlpItem]) -> IndexedTrieBuild {
    let values = items.iter().map(encode_rlp).collect::<Vec<_>>();
    indexed_trie_build(&values)
}

fn indexed_trie_build(values: &[Vec<u8>]) -> IndexedTrieBuild {
    if values.is_empty() {
        let rlp = encode_rlp(&RlpItem::Bytes(Vec::new()));
        let root = keccak256(&rlp);
        return IndexedTrieBuild {
            root,
            hash_preimages: vec![TrieHashPreimage { hash: root, rlp }],
        };
    }

    let mut entries = Vec::with_capacity(values.len());
    for (index, value) in values.iter().enumerate() {
        entries.push(TrieEntry {
            path: bytes_to_nibbles(&encode_transaction_index(index)),
            value: value.clone(),
        });
    }
    entries.sort_by(|lhs, rhs| lhs.path.cmp(&rhs.path));

    let mut hash_preimages = Vec::new();
    let root_rlp = encode_node(&build_node(&entries, 0), &mut hash_preimages);
    let root = keccak256(&root_rlp);
    hash_preimages.insert(
        0,
        TrieHashPreimage {
            hash: root,
            rlp: root_rlp,
        },
    );

    IndexedTrieBuild {
        root,
        hash_preimages,
    }
}

fn encode_transaction_index(mut index: usize) -> Vec<u8> {
    if index == 0 {
        return encode_rlp(&RlpItem::Bytes(Vec::new()));
    }

    let mut bytes = Vec::new();
    while index > 0 {
        bytes.push((index & 0xff) as u8);
        index >>= 8;
    }
    bytes.reverse();

    encode_rlp(&RlpItem::Bytes(bytes))
}

fn bytes_to_nibbles(bytes: &[u8]) -> Vec<u8> {
    bytes
        .iter()
        .flat_map(|byte| [byte >> 4, byte & 0x0f])
        .collect()
}

fn build_node(entries: &[TrieEntry], depth: usize) -> TrieNode {
    assert!(!entries.is_empty());

    if entries.len() == 1 {
        return TrieNode::Leaf {
            path: entries[0].path[depth..].to_vec(),
            value: entries[0].value.clone(),
        };
    }

    let shared_prefix_len = shared_prefix_len(entries, depth);
    if shared_prefix_len > 0 {
        return TrieNode::Extension {
            path: entries[0].path[depth..depth + shared_prefix_len].to_vec(),
            child: Box::new(build_node(entries, depth + shared_prefix_len)),
        };
    }

    let mut children = vec![None; 16];
    let mut value = None;
    let mut index = 0;
    while index < entries.len() {
        if entries[index].path.len() == depth {
            value = Some(entries[index].value.clone());
            index += 1;
            continue;
        }

        let child_index = usize::from(entries[index].path[depth]);
        let child_start = index;
        while index < entries.len()
            && entries[index].path.len() > depth
            && usize::from(entries[index].path[depth]) == child_index
        {
            index += 1;
        }
        children[child_index] = Some(Box::new(build_node(
            &entries[child_start..index],
            depth + 1,
        )));
    }

    TrieNode::Branch { children, value }
}

fn shared_prefix_len(entries: &[TrieEntry], depth: usize) -> usize {
    let mut len = 0;
    'prefix: while depth + len < entries[0].path.len() {
        let nibble = entries[0].path[depth + len];
        for entry in &entries[1..] {
            if depth + len >= entry.path.len() || entry.path[depth + len] != nibble {
                break 'prefix;
            }
        }
        len += 1;
    }
    len
}

fn encode_node(node: &TrieNode, hash_preimages: &mut Vec<TrieHashPreimage>) -> Vec<u8> {
    encode_rlp(&node_to_rlp(node, hash_preimages))
}

fn node_to_rlp(node: &TrieNode, hash_preimages: &mut Vec<TrieHashPreimage>) -> RlpItem {
    match node {
        TrieNode::Leaf { path, value } => RlpItem::List(vec![
            RlpItem::Bytes(compact_encode_nibbles(path, true)),
            RlpItem::Bytes(value.clone()),
        ]),
        TrieNode::Extension { path, child } => RlpItem::List(vec![
            RlpItem::Bytes(compact_encode_nibbles(path, false)),
            child_reference(child, hash_preimages),
        ]),
        TrieNode::Branch { children, value } => {
            let mut items = Vec::with_capacity(17);
            for child in children {
                items.push(match child {
                    Some(child) => child_reference(child, hash_preimages),
                    None => RlpItem::Bytes(Vec::new()),
                });
            }
            items.push(RlpItem::Bytes(value.clone().unwrap_or_default()));
            RlpItem::List(items)
        }
    }
}

fn child_reference(child: &TrieNode, hash_preimages: &mut Vec<TrieHashPreimage>) -> RlpItem {
    let child_item = node_to_rlp(child, hash_preimages);
    let child_bytes = encode_rlp(&child_item);
    if child_bytes.len() < 32 {
        child_item
    } else {
        let hash = keccak256(&child_bytes);
        hash_preimages.push(TrieHashPreimage {
            hash,
            rlp: child_bytes,
        });
        RlpItem::Bytes(hash.to_vec())
    }
}
