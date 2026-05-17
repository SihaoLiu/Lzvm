use std::fmt;

use crate::eth_block::{
    decode_eth_header_rlp, decode_eth_transactions_rlp, decode_eth_withdrawals_rlp,
    eth_header_hash, eth_ommers_hash, keccak256, parse_eth_block_rlp, EthBlockError,
    EthTransactionError, EthWithdrawalError,
};
use crate::eth_trie::{
    transaction_trie_build, withdrawals_trie_build, IndexedTrieBuild, TrieHashPreimage,
};
use crate::sectioned::{
    encode_sectioned_file, parse_sectioned_file, SectionedError, SectionedFile, SectionedSection,
};

const ETH_BLOCK_INPUT_KIND: [u8; 4] = *b"ethi";
const ETH_BLOCK_INPUT_VERSION: u32 = 1;
const METADATA_SECTION_ID: u32 = 1;
const BLOCK_RLP_SECTION_ID: u32 = 2;
const TRANSACTION_PREIMAGES_SECTION_ID: u32 = 3;
const WITHDRAWAL_PREIMAGES_SECTION_ID: u32 = 4;
const HASH_BYTES: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EthBlockInput {
    pub block_rlp: Vec<u8>,
    pub block_hash: [u8; 32],
    pub block_number: u64,
    pub timestamp: u64,
    pub ommers_hash: [u8; 32],
    pub transactions_root: [u8; 32],
    pub withdrawals_root: Option<[u8; 32]>,
    pub transactions: IndexedTrieBuild,
    pub withdrawals: Option<IndexedTrieBuild>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EthBlockInputTrie {
    Transactions,
    Withdrawals,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EthBlockInputError {
    Block(EthBlockError),
    Transaction(EthTransactionError),
    Withdrawal(EthWithdrawalError),
    Sectioned(SectionedError),
    MissingMetadata,
    MissingBlockRlp,
    MissingTransactionPreimages,
    DuplicateSection {
        id: u32,
    },
    InvalidMetadataLength {
        expected_min: usize,
        found: usize,
    },
    InvalidWithdrawalFlag {
        found: u32,
    },
    TransactionsRootMismatch,
    OmmersHashMismatch,
    MissingWithdrawalsRoot,
    UnexpectedWithdrawalsRoot,
    BlockHashMismatch,
    BlockNumberMismatch,
    TimestampMismatch,
    WithdrawalsRootMismatch,
    InvalidPreimageSection,
    PreimageHashMismatch {
        trie: EthBlockInputTrie,
        index: usize,
    },
    MissingRootPreimage {
        trie: EthBlockInputTrie,
    },
    UnexpectedTrailingBytes {
        count: usize,
    },
    UnexpectedEof {
        offset: usize,
        needed: usize,
        available: usize,
    },
    LengthOverflow,
}

impl fmt::Display for EthBlockInputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Block(error) => write!(f, "{error}"),
            Self::Transaction(error) => write!(f, "{error}"),
            Self::Withdrawal(error) => write!(f, "{error}"),
            Self::Sectioned(error) => write!(f, "ETH block input container error: {error}"),
            Self::MissingMetadata => write!(f, "missing ETH block input metadata"),
            Self::MissingBlockRlp => write!(f, "missing ETH block input block RLP"),
            Self::MissingTransactionPreimages => {
                write!(f, "missing ETH block input transaction preimages")
            }
            Self::DuplicateSection { id } => write!(f, "duplicate ETH block input section: {id}"),
            Self::InvalidMetadataLength {
                expected_min,
                found,
            } => write!(
                f,
                "invalid ETH block input metadata length: expected at least {expected_min}, found {found}"
            ),
            Self::InvalidWithdrawalFlag { found } => {
                write!(f, "invalid ETH block input withdrawal flag: {found}")
            }
            Self::TransactionsRootMismatch => write!(f, "ETH block transactions root mismatch"),
            Self::OmmersHashMismatch => write!(f, "ETH block ommers hash mismatch"),
            Self::MissingWithdrawalsRoot => {
                write!(f, "ETH block withdrawals body present without withdrawals root")
            }
            Self::UnexpectedWithdrawalsRoot => {
                write!(f, "ETH block input has withdrawals root without withdrawals data")
            }
            Self::BlockHashMismatch => write!(f, "ETH block input block hash mismatch"),
            Self::BlockNumberMismatch => write!(f, "ETH block input block number mismatch"),
            Self::TimestampMismatch => write!(f, "ETH block input timestamp mismatch"),
            Self::WithdrawalsRootMismatch => write!(f, "ETH block withdrawals root mismatch"),
            Self::InvalidPreimageSection => write!(f, "invalid ETH trie preimage section"),
            Self::PreimageHashMismatch { trie, index } => {
                write!(f, "ETH block input {trie} preimage hash mismatch at {index}")
            }
            Self::MissingRootPreimage { trie } => {
                write!(f, "ETH block input {trie} root preimage missing")
            }
            Self::UnexpectedTrailingBytes { count } => {
                write!(f, "unexpected trailing bytes in ETH block input: {count}")
            }
            Self::UnexpectedEof {
                offset,
                needed,
                available,
            } => write!(
                f,
                "unexpected end of ETH block input at {offset}, needed {needed}, available {available}"
            ),
            Self::LengthOverflow => write!(f, "ETH block input length overflow"),
        }
    }
}

impl fmt::Display for EthBlockInputTrie {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Transactions => "transactions",
            Self::Withdrawals => "withdrawals",
        };
        f.write_str(name)
    }
}

impl std::error::Error for EthBlockInputError {}

impl From<EthBlockError> for EthBlockInputError {
    fn from(error: EthBlockError) -> Self {
        Self::Block(error)
    }
}

impl From<EthTransactionError> for EthBlockInputError {
    fn from(error: EthTransactionError) -> Self {
        Self::Transaction(error)
    }
}

impl From<EthWithdrawalError> for EthBlockInputError {
    fn from(error: EthWithdrawalError) -> Self {
        Self::Withdrawal(error)
    }
}

pub fn build_eth_block_input(block_rlp: &[u8]) -> Result<EthBlockInput, EthBlockInputError> {
    let block = parse_eth_block_rlp(block_rlp)?;
    let header = decode_eth_header_rlp(&block.header)?;
    let block_hash = eth_header_hash(&block.header);
    let transactions = transaction_trie_build(&block.transactions)?;
    if transactions.root != header.transactions_root {
        return Err(EthBlockInputError::TransactionsRootMismatch);
    }
    let ommers_hash = eth_ommers_hash(&block.ommers);
    if ommers_hash != header.ommers_hash {
        return Err(EthBlockInputError::OmmersHashMismatch);
    }
    decode_eth_transactions_rlp(&block.transactions)?;

    let withdrawals = match (&block.withdrawals, header.withdrawals_root) {
        (Some(withdrawals), Some(root)) => {
            decode_eth_withdrawals_rlp(withdrawals)?;
            let build = withdrawals_trie_build(withdrawals);
            if build.root != root {
                return Err(EthBlockInputError::WithdrawalsRootMismatch);
            }
            Some(build)
        }
        (Some(_), None) => return Err(EthBlockInputError::MissingWithdrawalsRoot),
        (None, Some(_)) => return Err(EthBlockInputError::UnexpectedWithdrawalsRoot),
        (None, None) => None,
    };

    Ok(EthBlockInput {
        block_rlp: block_rlp.to_vec(),
        block_hash,
        block_number: header.number,
        timestamp: header.timestamp,
        ommers_hash,
        transactions_root: header.transactions_root,
        withdrawals_root: header.withdrawals_root,
        transactions,
        withdrawals,
    })
}

pub fn encode_eth_block_input(value: &EthBlockInput) -> Result<Vec<u8>, EthBlockInputError> {
    let metadata = encode_metadata(value);
    let mut sections = vec![
        SectionedSection {
            id: METADATA_SECTION_ID,
            data: metadata,
        },
        SectionedSection {
            id: BLOCK_RLP_SECTION_ID,
            data: value.block_rlp.clone(),
        },
        SectionedSection {
            id: TRANSACTION_PREIMAGES_SECTION_ID,
            data: encode_preimages(&value.transactions.hash_preimages)?,
        },
    ];
    if let Some(withdrawals) = &value.withdrawals {
        sections.push(SectionedSection {
            id: WITHDRAWAL_PREIMAGES_SECTION_ID,
            data: encode_preimages(&withdrawals.hash_preimages)?,
        });
    }

    encode_sectioned_file(&SectionedFile {
        kind: ETH_BLOCK_INPUT_KIND,
        version: ETH_BLOCK_INPUT_VERSION,
        sections,
    })
    .map_err(EthBlockInputError::Sectioned)
}

pub fn parse_eth_block_input(bytes: &[u8]) -> Result<EthBlockInput, EthBlockInputError> {
    let file = parse_sectioned_file(bytes, ETH_BLOCK_INPUT_KIND, ETH_BLOCK_INPUT_VERSION)
        .map_err(EthBlockInputError::Sectioned)?;
    let mut metadata = None;
    let mut block_rlp = None;
    let mut transaction_preimages = None;
    let mut withdrawal_preimages = None;

    for section in file.sections {
        let target = match section.id {
            METADATA_SECTION_ID => &mut metadata,
            BLOCK_RLP_SECTION_ID => &mut block_rlp,
            TRANSACTION_PREIMAGES_SECTION_ID => &mut transaction_preimages,
            WITHDRAWAL_PREIMAGES_SECTION_ID => &mut withdrawal_preimages,
            _ => continue,
        };
        if target.replace(section.data).is_some() {
            return Err(EthBlockInputError::DuplicateSection { id: section.id });
        }
    }

    let metadata = metadata.ok_or(EthBlockInputError::MissingMetadata)?;
    let block_rlp = block_rlp.ok_or(EthBlockInputError::MissingBlockRlp)?;
    let transaction_preimages =
        transaction_preimages.ok_or(EthBlockInputError::MissingTransactionPreimages)?;
    let metadata = parse_metadata(&metadata)?;
    validate_metadata(&metadata, &block_rlp)?;
    let transaction_hash_preimages = parse_validated_preimages(
        EthBlockInputTrie::Transactions,
        metadata.transactions_root,
        &transaction_preimages,
    )?;
    let transactions = IndexedTrieBuild {
        root: metadata.transactions_root,
        hash_preimages: transaction_hash_preimages,
    };
    let withdrawals = match (metadata.withdrawals_root, withdrawal_preimages) {
        (Some(root), Some(preimages)) => {
            let hash_preimages =
                parse_validated_preimages(EthBlockInputTrie::Withdrawals, root, &preimages)?;
            Some(IndexedTrieBuild {
                root,
                hash_preimages,
            })
        }
        (Some(_), None) => return Err(EthBlockInputError::UnexpectedWithdrawalsRoot),
        (None, Some(_)) => return Err(EthBlockInputError::UnexpectedWithdrawalsRoot),
        (None, None) => None,
    };

    Ok(EthBlockInput {
        block_rlp,
        block_hash: metadata.block_hash,
        block_number: metadata.block_number,
        timestamp: metadata.timestamp,
        ommers_hash: metadata.ommers_hash,
        transactions_root: metadata.transactions_root,
        withdrawals_root: metadata.withdrawals_root,
        transactions,
        withdrawals,
    })
}

struct Metadata {
    block_hash: [u8; 32],
    block_number: u64,
    timestamp: u64,
    ommers_hash: [u8; 32],
    transactions_root: [u8; 32],
    withdrawals_root: Option<[u8; 32]>,
}

fn encode_metadata(value: &EthBlockInput) -> Vec<u8> {
    let mut out = Vec::with_capacity(HASH_BYTES * 4 + 8 + 8 + 4);
    out.extend_from_slice(&value.block_hash);
    out.extend_from_slice(&value.block_number.to_le_bytes());
    out.extend_from_slice(&value.timestamp.to_le_bytes());
    out.extend_from_slice(&value.ommers_hash);
    out.extend_from_slice(&value.transactions_root);
    match value.withdrawals_root {
        Some(root) => {
            out.extend_from_slice(&1_u32.to_le_bytes());
            out.extend_from_slice(&root);
        }
        None => out.extend_from_slice(&0_u32.to_le_bytes()),
    }
    out
}

fn parse_metadata(bytes: &[u8]) -> Result<Metadata, EthBlockInputError> {
    let mut reader = Reader::new(bytes);
    let block_hash = reader.read_hash()?;
    let block_number = reader.read_u64()?;
    let timestamp = reader.read_u64()?;
    let ommers_hash = reader.read_hash()?;
    let transactions_root = reader.read_hash()?;
    let withdrawals_flag = reader.read_u32()?;
    let withdrawals_root = match withdrawals_flag {
        0 => None,
        1 => Some(reader.read_hash()?),
        found => return Err(EthBlockInputError::InvalidWithdrawalFlag { found }),
    };
    reader.finish()?;

    Ok(Metadata {
        block_hash,
        block_number,
        timestamp,
        ommers_hash,
        transactions_root,
        withdrawals_root,
    })
}

fn validate_metadata(metadata: &Metadata, block_rlp: &[u8]) -> Result<(), EthBlockInputError> {
    let input = build_eth_block_input(block_rlp)?;
    if metadata.block_hash != input.block_hash {
        return Err(EthBlockInputError::BlockHashMismatch);
    }
    if metadata.block_number != input.block_number {
        return Err(EthBlockInputError::BlockNumberMismatch);
    }
    if metadata.timestamp != input.timestamp {
        return Err(EthBlockInputError::TimestampMismatch);
    }
    if metadata.ommers_hash != input.ommers_hash {
        return Err(EthBlockInputError::OmmersHashMismatch);
    }
    if metadata.transactions_root != input.transactions_root {
        return Err(EthBlockInputError::TransactionsRootMismatch);
    }
    if metadata.withdrawals_root != input.withdrawals_root {
        return Err(EthBlockInputError::WithdrawalsRootMismatch);
    }
    Ok(())
}

fn encode_preimages(preimages: &[TrieHashPreimage]) -> Result<Vec<u8>, EthBlockInputError> {
    let count = u32::try_from(preimages.len()).map_err(|_| EthBlockInputError::LengthOverflow)?;
    let mut out = Vec::new();
    out.extend_from_slice(&count.to_le_bytes());
    for preimage in preimages {
        out.extend_from_slice(&preimage.hash);
        let len =
            u64::try_from(preimage.rlp.len()).map_err(|_| EthBlockInputError::LengthOverflow)?;
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&preimage.rlp);
    }
    Ok(out)
}

fn parse_preimages(bytes: &[u8]) -> Result<Vec<TrieHashPreimage>, EthBlockInputError> {
    let mut reader = Reader::new(bytes);
    let count = reader.read_u32()?;
    let count = usize::try_from(count).map_err(|_| EthBlockInputError::LengthOverflow)?;
    if count == 0 {
        return Err(EthBlockInputError::InvalidPreimageSection);
    }

    let mut preimages = Vec::with_capacity(count);
    for _ in 0..count {
        let hash = reader.read_hash()?;
        let len = reader.read_u64()?;
        let len = usize::try_from(len).map_err(|_| EthBlockInputError::LengthOverflow)?;
        let rlp = reader.read_exact(len)?.to_vec();
        preimages.push(TrieHashPreimage { hash, rlp });
    }
    reader.finish()?;
    Ok(preimages)
}

fn parse_validated_preimages(
    trie: EthBlockInputTrie,
    root: [u8; 32],
    bytes: &[u8],
) -> Result<Vec<TrieHashPreimage>, EthBlockInputError> {
    let preimages = parse_preimages(bytes)?;
    validate_preimages(trie, root, &preimages)?;
    Ok(preimages)
}

fn validate_preimages(
    trie: EthBlockInputTrie,
    root: [u8; 32],
    preimages: &[TrieHashPreimage],
) -> Result<(), EthBlockInputError> {
    let mut root_found = false;
    for (index, preimage) in preimages.iter().enumerate() {
        if keccak256(&preimage.rlp) != preimage.hash {
            return Err(EthBlockInputError::PreimageHashMismatch { trie, index });
        }
        root_found |= preimage.hash == root;
    }
    if !root_found {
        return Err(EthBlockInputError::MissingRootPreimage { trie });
    }
    Ok(())
}

struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn finish(&self) -> Result<(), EthBlockInputError> {
        if self.offset != self.bytes.len() {
            return Err(EthBlockInputError::UnexpectedTrailingBytes {
                count: self.bytes.len() - self.offset,
            });
        }
        Ok(())
    }

    fn read_exact(&mut self, count: usize) -> Result<&'a [u8], EthBlockInputError> {
        let end = self
            .offset
            .checked_add(count)
            .ok_or(EthBlockInputError::LengthOverflow)?;
        if end > self.bytes.len() {
            return Err(EthBlockInputError::UnexpectedEof {
                offset: self.offset,
                needed: count,
                available: self.bytes.len().saturating_sub(self.offset),
            });
        }
        let out = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(out)
    }

    fn read_hash(&mut self) -> Result<[u8; 32], EthBlockInputError> {
        Ok(self
            .read_exact(HASH_BYTES)?
            .try_into()
            .expect("slice length checked"))
    }

    fn read_u32(&mut self) -> Result<u32, EthBlockInputError> {
        Ok(u32::from_le_bytes(
            self.read_exact(4)?
                .try_into()
                .expect("slice length checked"),
        ))
    }

    fn read_u64(&mut self) -> Result<u64, EthBlockInputError> {
        Ok(u64::from_le_bytes(
            self.read_exact(8)?
                .try_into()
                .expect("slice length checked"),
        ))
    }
}
