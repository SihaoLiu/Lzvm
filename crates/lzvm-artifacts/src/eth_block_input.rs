use std::collections::BTreeSet;
use std::fmt;

use sha2::{Digest, Sha256};

use crate::eth_block::{
    decode_eth_header_rlp, decode_eth_receipts_rlp, decode_eth_transactions_rlp,
    decode_eth_withdrawals_rlp, eth_header_hash, eth_ommers_hash,
    eth_receipts_cumulative_gas_is_nondecreasing, eth_receipts_cumulative_gas_used,
    eth_receipts_logs_bloom, keccak256, parse_eth_block_rlp, EthBlockError, EthReceiptError,
    EthTransactionError, EthWithdrawalError,
};
use crate::eth_trie::{
    receipt_trie_build, transaction_trie_build, withdrawals_trie_build, IndexedTrieBuild,
    TrieHashPreimage,
};
use crate::rlp::{parse_rlp, RlpItem};
use crate::sectioned::{
    encode_sectioned_file, parse_sectioned_file, SectionedError, SectionedFile, SectionedSection,
};

const ETH_BLOCK_INPUT_KIND: [u8; 4] = *b"ethi";
const ETH_BLOCK_INPUT_VERSION: u32 = 1;
const METADATA_SECTION_ID: u32 = 1;
const BLOCK_RLP_SECTION_ID: u32 = 2;
const TRANSACTION_PREIMAGES_SECTION_ID: u32 = 3;
const WITHDRAWAL_PREIMAGES_SECTION_ID: u32 = 4;
const RECEIPT_PREIMAGES_SECTION_ID: u32 = 5;
const RECEIPTS_RLP_SECTION_ID: u32 = 6;
const HASH_BYTES: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EthBlockInput {
    pub block_rlp: Vec<u8>,
    pub block_hash: [u8; 32],
    pub parent_hash: [u8; 32],
    pub beneficiary: [u8; 20],
    pub state_root: [u8; 32],
    pub receipts_root: [u8; 32],
    pub logs_bloom: [u8; 256],
    pub difficulty: [u8; 32],
    pub block_number: u64,
    pub timestamp: u64,
    pub extra_data: Vec<u8>,
    pub gas_limit: u64,
    pub gas_used: u64,
    pub base_fee_per_gas: Option<[u8; 32]>,
    pub mix_hash: [u8; 32],
    pub nonce: [u8; 8],
    pub ommers_hash: [u8; 32],
    pub transactions_root: [u8; 32],
    pub withdrawals_root: Option<[u8; 32]>,
    pub transactions: IndexedTrieBuild,
    pub receipts_rlp: Option<Vec<u8>>,
    pub receipts: Option<IndexedTrieBuild>,
    pub withdrawals: Option<IndexedTrieBuild>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EthBlockInputTrie {
    Transactions,
    Receipts,
    Withdrawals,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EthBlockInputError {
    Block(EthBlockError),
    Transaction(EthTransactionError),
    Receipt(EthReceiptError),
    Withdrawal(EthWithdrawalError),
    Sectioned(SectionedError),
    MissingMetadata,
    MissingBlockRlp,
    MissingTransactionPreimages,
    ExpectedReceiptsList,
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
    ParentHashMismatch,
    BeneficiaryMismatch,
    StateRootMismatch,
    ReceiptsRootMismatch,
    ReceiptCountMismatch,
    LogsBloomMismatch,
    DifficultyMismatch,
    ExtraDataMismatch,
    ExtraDataOverflow {
        max_bytes: usize,
        found: usize,
    },
    DifficultyOverflow {
        max_bytes: usize,
        found: usize,
    },
    BlockNumberMismatch,
    TimestampMismatch,
    GasLimitMismatch,
    GasUsedMismatch,
    BaseFeePerGasMismatch,
    BaseFeePerGasOverflow {
        max_bytes: usize,
        found: usize,
    },
    InvalidBaseFeePerGasFlag {
        found: u32,
    },
    MixHashMismatch,
    NonceMismatch,
    WithdrawalsRootMismatch,
    InvalidPreimageSection,
    PreimageHashMismatch {
        trie: EthBlockInputTrie,
        index: usize,
    },
    MissingRootPreimage {
        trie: EthBlockInputTrie,
    },
    MissingChildPreimage {
        trie: EthBlockInputTrie,
        index: usize,
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
            Self::Receipt(error) => write!(f, "{error}"),
            Self::Withdrawal(error) => write!(f, "{error}"),
            Self::Sectioned(error) => write!(f, "ETH block input container error: {error}"),
            Self::MissingMetadata => write!(f, "missing ETH block input metadata"),
            Self::MissingBlockRlp => write!(f, "missing ETH block input block RLP"),
            Self::MissingTransactionPreimages => {
                write!(f, "missing ETH block input transaction preimages")
            }
            Self::ExpectedReceiptsList => write!(f, "expected ETH receipts list"),
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
            Self::ParentHashMismatch => write!(f, "ETH block input parent hash mismatch"),
            Self::BeneficiaryMismatch => write!(f, "ETH block input beneficiary mismatch"),
            Self::StateRootMismatch => write!(f, "ETH block input state root mismatch"),
            Self::ReceiptsRootMismatch => write!(f, "ETH block input receipts root mismatch"),
            Self::ReceiptCountMismatch => write!(f, "ETH block input receipt count mismatch"),
            Self::LogsBloomMismatch => write!(f, "ETH block input logs bloom mismatch"),
            Self::DifficultyMismatch => write!(f, "ETH block input difficulty mismatch"),
            Self::ExtraDataMismatch => write!(f, "ETH block input extra data mismatch"),
            Self::ExtraDataOverflow { max_bytes, found } => write!(
                f,
                "ETH block input extra data exceeds {max_bytes} bytes, found {found}"
            ),
            Self::DifficultyOverflow { max_bytes, found } => write!(
                f,
                "ETH block input difficulty exceeds {max_bytes} bytes, found {found}"
            ),
            Self::BlockNumberMismatch => write!(f, "ETH block input block number mismatch"),
            Self::TimestampMismatch => write!(f, "ETH block input timestamp mismatch"),
            Self::GasLimitMismatch => write!(f, "ETH block input gas limit mismatch"),
            Self::GasUsedMismatch => write!(f, "ETH block input gas used mismatch"),
            Self::BaseFeePerGasMismatch => write!(f, "ETH block input base fee mismatch"),
            Self::BaseFeePerGasOverflow { max_bytes, found } => write!(
                f,
                "ETH block input base fee exceeds {max_bytes} bytes, found {found}"
            ),
            Self::InvalidBaseFeePerGasFlag { found } => {
                write!(f, "invalid ETH block input base fee flag: {found}")
            }
            Self::MixHashMismatch => write!(f, "ETH block input mix hash mismatch"),
            Self::NonceMismatch => write!(f, "ETH block input nonce mismatch"),
            Self::WithdrawalsRootMismatch => write!(f, "ETH block withdrawals root mismatch"),
            Self::InvalidPreimageSection => write!(f, "invalid ETH trie preimage section"),
            Self::PreimageHashMismatch { trie, index } => {
                write!(f, "ETH block input {trie} preimage hash mismatch at {index}")
            }
            Self::MissingRootPreimage { trie } => {
                write!(f, "ETH block input {trie} root preimage missing")
            }
            Self::MissingChildPreimage { trie, index } => {
                write!(f, "ETH block input {trie} child preimage missing at {index}")
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
            Self::Receipts => "receipts",
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

impl From<EthReceiptError> for EthBlockInputError {
    fn from(error: EthReceiptError) -> Self {
        Self::Receipt(error)
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
        parent_hash: header.parent_hash,
        beneficiary: header.beneficiary,
        state_root: header.state_root,
        receipts_root: header.receipts_root,
        logs_bloom: header.logs_bloom,
        difficulty: difficulty_to_u256_be(&header.difficulty)?,
        block_number: header.number,
        timestamp: header.timestamp,
        extra_data: checked_extra_data(&header.extra_data)?.to_vec(),
        gas_limit: header.gas_limit,
        gas_used: header.gas_used,
        base_fee_per_gas: header
            .base_fee_per_gas
            .as_deref()
            .map(base_fee_to_u256_be)
            .transpose()?,
        mix_hash: header.mix_hash,
        nonce: header.nonce,
        ommers_hash,
        transactions_root: header.transactions_root,
        withdrawals_root: header.withdrawals_root,
        transactions,
        receipts_rlp: None,
        receipts: None,
        withdrawals,
    })
}

pub fn build_eth_block_input_with_receipts(
    block_rlp: &[u8],
    receipts_rlp: &[u8],
) -> Result<EthBlockInput, EthBlockInputError> {
    let mut input = build_eth_block_input(block_rlp)?;
    let receipts = parse_eth_receipts_rlp(receipts_rlp)?;
    let decoded_receipts = decode_eth_receipts_rlp(&receipts)?;
    validate_receipts_against_block(
        block_rlp,
        &decoded_receipts,
        input.logs_bloom,
        input.gas_used,
    )?;
    let build = receipt_trie_build(&receipts);
    if build.root != input.receipts_root {
        return Err(EthBlockInputError::ReceiptsRootMismatch);
    }
    input.receipts_rlp = Some(receipts_rlp.to_vec());
    input.receipts = Some(build);
    Ok(input)
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
    if let Some(receipts) = &value.receipts {
        sections.push(SectionedSection {
            id: RECEIPT_PREIMAGES_SECTION_ID,
            data: encode_preimages(&receipts.hash_preimages)?,
        });
    }
    if let Some(receipts_rlp) = &value.receipts_rlp {
        sections.push(SectionedSection {
            id: RECEIPTS_RLP_SECTION_ID,
            data: receipts_rlp.clone(),
        });
    }

    encode_sectioned_file(&SectionedFile {
        kind: ETH_BLOCK_INPUT_KIND,
        version: ETH_BLOCK_INPUT_VERSION,
        sections,
    })
    .map_err(EthBlockInputError::Sectioned)
}

pub fn eth_block_input_bytes_digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

pub fn parse_eth_block_input(bytes: &[u8]) -> Result<EthBlockInput, EthBlockInputError> {
    let file = parse_sectioned_file(bytes, ETH_BLOCK_INPUT_KIND, ETH_BLOCK_INPUT_VERSION)
        .map_err(EthBlockInputError::Sectioned)?;
    let mut metadata = None;
    let mut block_rlp = None;
    let mut transaction_preimages = None;
    let mut withdrawal_preimages = None;
    let mut receipt_preimages = None;
    let mut receipts_rlp = None;

    for section in file.sections {
        let target = match section.id {
            METADATA_SECTION_ID => &mut metadata,
            BLOCK_RLP_SECTION_ID => &mut block_rlp,
            TRANSACTION_PREIMAGES_SECTION_ID => &mut transaction_preimages,
            WITHDRAWAL_PREIMAGES_SECTION_ID => &mut withdrawal_preimages,
            RECEIPT_PREIMAGES_SECTION_ID => &mut receipt_preimages,
            RECEIPTS_RLP_SECTION_ID => &mut receipts_rlp,
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
    let validated_input = validate_metadata(&metadata, &block_rlp)?;
    let transaction_hash_preimages = parse_validated_preimages(
        EthBlockInputTrie::Transactions,
        metadata.transactions_root,
        &transaction_preimages,
    )?;
    let transactions = IndexedTrieBuild {
        root: metadata.transactions_root,
        hash_preimages: transaction_hash_preimages,
    };
    let receipts = match receipt_preimages {
        Some(preimages) => {
            let hash_preimages = parse_validated_preimages(
                EthBlockInputTrie::Receipts,
                metadata.receipts_root,
                &preimages,
            )?;
            Some(IndexedTrieBuild {
                root: metadata.receipts_root,
                hash_preimages,
            })
        }
        None => None,
    };
    let receipts_rlp = match receipts_rlp {
        Some(bytes) => {
            let parsed_receipts = parse_eth_receipts_rlp(&bytes)?;
            let decoded_receipts = decode_eth_receipts_rlp(&parsed_receipts)?;
            validate_receipts_against_block(
                &block_rlp,
                &decoded_receipts,
                validated_input.logs_bloom,
                validated_input.gas_used,
            )?;
            let build = receipt_trie_build(&parsed_receipts);
            if build.root != metadata.receipts_root {
                return Err(EthBlockInputError::ReceiptsRootMismatch);
            }
            Some(bytes)
        }
        None => None,
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
        parent_hash: metadata.parent_hash,
        beneficiary: metadata.beneficiary,
        state_root: metadata.state_root,
        receipts_root: metadata.receipts_root,
        logs_bloom: metadata.logs_bloom.unwrap_or(validated_input.logs_bloom),
        difficulty: metadata.difficulty.unwrap_or(validated_input.difficulty),
        block_number: metadata.block_number,
        timestamp: metadata.timestamp,
        extra_data: metadata
            .extra_data
            .unwrap_or_else(|| validated_input.extra_data.clone()),
        gas_limit: metadata.gas_limit,
        gas_used: metadata.gas_used,
        base_fee_per_gas: metadata.base_fee_per_gas,
        mix_hash: metadata.mix_hash,
        nonce: metadata.nonce,
        ommers_hash: metadata.ommers_hash,
        transactions_root: metadata.transactions_root,
        withdrawals_root: metadata.withdrawals_root,
        transactions,
        receipts_rlp,
        receipts,
        withdrawals,
    })
}

fn parse_eth_receipts_rlp(receipts_rlp: &[u8]) -> Result<Vec<RlpItem>, EthBlockInputError> {
    match parse_rlp(receipts_rlp).map_err(EthBlockError::Rlp)? {
        RlpItem::List(receipts) => Ok(receipts),
        RlpItem::Bytes(_) => Err(EthBlockInputError::ExpectedReceiptsList),
    }
}

fn validate_receipts_against_block(
    block_rlp: &[u8],
    receipts: &[crate::eth_block::EthReceiptRlp],
    logs_bloom: [u8; 256],
    gas_used: u64,
) -> Result<(), EthBlockInputError> {
    if let Some(receipts_logs_bloom) = eth_receipts_logs_bloom(receipts) {
        if receipts_logs_bloom != logs_bloom {
            return Err(EthBlockInputError::LogsBloomMismatch);
        }
    }
    let transaction_count = parse_eth_block_rlp(block_rlp)?.transactions.len();
    if receipts.len() != transaction_count {
        return Err(EthBlockInputError::ReceiptCountMismatch);
    }
    if let Some(false) = eth_receipts_cumulative_gas_is_nondecreasing(receipts) {
        return Err(EthBlockInputError::GasUsedMismatch);
    }
    if let Some(receipts_gas_used) = eth_receipts_cumulative_gas_used(receipts) {
        if receipts_gas_used != gas_used {
            return Err(EthBlockInputError::GasUsedMismatch);
        }
    }
    Ok(())
}

struct Metadata {
    block_hash: [u8; 32],
    parent_hash: [u8; 32],
    beneficiary: [u8; 20],
    state_root: [u8; 32],
    receipts_root: [u8; 32],
    logs_bloom: Option<[u8; 256]>,
    difficulty: Option<[u8; 32]>,
    block_number: u64,
    timestamp: u64,
    extra_data: Option<Vec<u8>>,
    gas_limit: u64,
    gas_used: u64,
    base_fee_per_gas: Option<[u8; 32]>,
    mix_hash: [u8; 32],
    nonce: [u8; 8],
    ommers_hash: [u8; 32],
    transactions_root: [u8; 32],
    withdrawals_root: Option<[u8; 32]>,
}

fn encode_metadata(value: &EthBlockInput) -> Vec<u8> {
    let mut out = Vec::with_capacity(HASH_BYTES * 8 + 20 + 8 * 5 + 4);
    out.extend_from_slice(&value.block_hash);
    out.extend_from_slice(&value.parent_hash);
    out.extend_from_slice(&value.beneficiary);
    out.extend_from_slice(&value.state_root);
    out.extend_from_slice(&value.receipts_root);
    out.extend_from_slice(&value.block_number.to_le_bytes());
    out.extend_from_slice(&value.timestamp.to_le_bytes());
    out.extend_from_slice(&value.gas_limit.to_le_bytes());
    out.extend_from_slice(&value.gas_used.to_le_bytes());
    out.extend_from_slice(&value.mix_hash);
    out.extend_from_slice(&value.nonce);
    out.extend_from_slice(&value.ommers_hash);
    out.extend_from_slice(&value.transactions_root);
    match value.withdrawals_root {
        Some(root) => {
            out.extend_from_slice(&1_u32.to_le_bytes());
            out.extend_from_slice(&root);
        }
        None => out.extend_from_slice(&0_u32.to_le_bytes()),
    }
    match value.base_fee_per_gas {
        Some(value) => {
            out.extend_from_slice(&1_u32.to_le_bytes());
            out.extend_from_slice(&value);
        }
        None => out.extend_from_slice(&0_u32.to_le_bytes()),
    }
    out.extend_from_slice(&value.difficulty);
    out.extend_from_slice(&value.logs_bloom);
    let extra_data_len =
        u32::try_from(value.extra_data.len()).expect("extra data length should fit u32");
    out.extend_from_slice(&extra_data_len.to_le_bytes());
    out.extend_from_slice(&value.extra_data);
    out
}

fn parse_metadata(bytes: &[u8]) -> Result<Metadata, EthBlockInputError> {
    let mut reader = Reader::new(bytes);
    let block_hash = reader.read_hash()?;
    let parent_hash = reader.read_hash()?;
    let beneficiary = reader.read_20_bytes()?;
    let state_root = reader.read_hash()?;
    let receipts_root = reader.read_hash()?;
    let block_number = reader.read_u64()?;
    let timestamp = reader.read_u64()?;
    let gas_limit = reader.read_u64()?;
    let gas_used = reader.read_u64()?;
    let mix_hash = reader.read_hash()?;
    let nonce = reader.read_8_bytes()?;
    let ommers_hash = reader.read_hash()?;
    let transactions_root = reader.read_hash()?;
    let withdrawals_flag = reader.read_u32()?;
    let withdrawals_root = match withdrawals_flag {
        0 => None,
        1 => Some(reader.read_hash()?),
        found => return Err(EthBlockInputError::InvalidWithdrawalFlag { found }),
    };
    let base_fee_per_gas = if reader.is_finished() {
        None
    } else {
        match reader.read_u32()? {
            0 => None,
            1 => Some(reader.read_hash()?),
            found => return Err(EthBlockInputError::InvalidBaseFeePerGasFlag { found }),
        }
    };
    let difficulty = if reader.is_finished() {
        None
    } else {
        Some(reader.read_hash()?)
    };
    let logs_bloom = if reader.is_finished() {
        None
    } else {
        Some(reader.read_256_bytes()?)
    };
    let extra_data = if reader.is_finished() {
        None
    } else {
        let len =
            usize::try_from(reader.read_u32()?).map_err(|_| EthBlockInputError::LengthOverflow)?;
        Some(reader.read_exact(len)?.to_vec())
    };
    reader.finish()?;

    Ok(Metadata {
        block_hash,
        parent_hash,
        beneficiary,
        state_root,
        receipts_root,
        logs_bloom,
        difficulty,
        block_number,
        timestamp,
        extra_data,
        gas_limit,
        gas_used,
        base_fee_per_gas,
        mix_hash,
        nonce,
        ommers_hash,
        transactions_root,
        withdrawals_root,
    })
}

fn validate_metadata(
    metadata: &Metadata,
    block_rlp: &[u8],
) -> Result<EthBlockInput, EthBlockInputError> {
    let input = build_eth_block_input(block_rlp)?;
    if metadata.block_hash != input.block_hash {
        return Err(EthBlockInputError::BlockHashMismatch);
    }
    if metadata.parent_hash != input.parent_hash {
        return Err(EthBlockInputError::ParentHashMismatch);
    }
    if metadata.beneficiary != input.beneficiary {
        return Err(EthBlockInputError::BeneficiaryMismatch);
    }
    if metadata.state_root != input.state_root {
        return Err(EthBlockInputError::StateRootMismatch);
    }
    if metadata.receipts_root != input.receipts_root {
        return Err(EthBlockInputError::ReceiptsRootMismatch);
    }
    if let Some(logs_bloom) = metadata.logs_bloom {
        if logs_bloom != input.logs_bloom {
            return Err(EthBlockInputError::LogsBloomMismatch);
        }
    }
    if let Some(difficulty) = metadata.difficulty {
        if difficulty != input.difficulty {
            return Err(EthBlockInputError::DifficultyMismatch);
        }
    }
    if metadata.block_number != input.block_number {
        return Err(EthBlockInputError::BlockNumberMismatch);
    }
    if metadata.timestamp != input.timestamp {
        return Err(EthBlockInputError::TimestampMismatch);
    }
    if let Some(extra_data) = &metadata.extra_data {
        if extra_data != &input.extra_data {
            return Err(EthBlockInputError::ExtraDataMismatch);
        }
    }
    if metadata.gas_limit != input.gas_limit {
        return Err(EthBlockInputError::GasLimitMismatch);
    }
    if metadata.gas_used != input.gas_used {
        return Err(EthBlockInputError::GasUsedMismatch);
    }
    if metadata.base_fee_per_gas != input.base_fee_per_gas {
        return Err(EthBlockInputError::BaseFeePerGasMismatch);
    }
    if metadata.mix_hash != input.mix_hash {
        return Err(EthBlockInputError::MixHashMismatch);
    }
    if metadata.nonce != input.nonce {
        return Err(EthBlockInputError::NonceMismatch);
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
    Ok(input)
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
    let hashes = preimages
        .iter()
        .map(|preimage| preimage.hash)
        .collect::<BTreeSet<_>>();
    let mut root_found = false;
    for (index, preimage) in preimages.iter().enumerate() {
        if keccak256(&preimage.rlp) != preimage.hash {
            return Err(EthBlockInputError::PreimageHashMismatch { trie, index });
        }
        root_found |= preimage.hash == root;
        validate_preimage_child_refs(trie, index, &preimage.rlp, &hashes)?;
    }
    if !root_found {
        return Err(EthBlockInputError::MissingRootPreimage { trie });
    }
    Ok(())
}

fn validate_preimage_child_refs(
    trie: EthBlockInputTrie,
    index: usize,
    rlp: &[u8],
    hashes: &BTreeSet<[u8; 32]>,
) -> Result<(), EthBlockInputError> {
    let item = parse_rlp(rlp).map_err(EthBlockError::from)?;
    validate_node_child_refs(trie, index, &item, hashes)
}

fn validate_node_child_refs(
    trie: EthBlockInputTrie,
    index: usize,
    item: &RlpItem,
    hashes: &BTreeSet<[u8; 32]>,
) -> Result<(), EthBlockInputError> {
    let RlpItem::List(items) = item else {
        return Ok(());
    };

    match items.as_slice() {
        [RlpItem::Bytes(path), child] => {
            if !compact_path_has_terminator(path)? {
                validate_child_ref(trie, index, child, hashes)?;
            }
            Ok(())
        }
        items if items.len() == 17 => {
            for child in &items[..16] {
                validate_child_ref(trie, index, child, hashes)?;
            }
            Ok(())
        }
        _ => Err(EthBlockInputError::InvalidPreimageSection),
    }
}

fn validate_child_ref(
    trie: EthBlockInputTrie,
    index: usize,
    item: &RlpItem,
    hashes: &BTreeSet<[u8; 32]>,
) -> Result<(), EthBlockInputError> {
    match item {
        RlpItem::Bytes(bytes) if bytes.is_empty() => Ok(()),
        RlpItem::Bytes(bytes) if bytes.len() == HASH_BYTES => {
            let hash: [u8; 32] = bytes.as_slice().try_into().expect("length checked");
            if hashes.contains(&hash) {
                Ok(())
            } else {
                Err(EthBlockInputError::MissingChildPreimage { trie, index })
            }
        }
        RlpItem::Bytes(_) => Err(EthBlockInputError::InvalidPreimageSection),
        RlpItem::List(_) => validate_node_child_refs(trie, index, item, hashes),
    }
}

fn compact_path_has_terminator(path: &[u8]) -> Result<bool, EthBlockInputError> {
    let Some(first) = path.first() else {
        return Err(EthBlockInputError::InvalidPreimageSection);
    };
    let flag = first >> 4;
    if flag > 3 {
        return Err(EthBlockInputError::InvalidPreimageSection);
    }
    Ok(flag & 2 != 0)
}

fn difficulty_to_u256_be(bytes: &[u8]) -> Result<[u8; 32], EthBlockInputError> {
    quantity_to_u256_be(bytes, |found| EthBlockInputError::DifficultyOverflow {
        max_bytes: 32,
        found,
    })
}

fn base_fee_to_u256_be(bytes: &[u8]) -> Result<[u8; 32], EthBlockInputError> {
    quantity_to_u256_be(bytes, |found| EthBlockInputError::BaseFeePerGasOverflow {
        max_bytes: 32,
        found,
    })
}

fn checked_extra_data(bytes: &[u8]) -> Result<&[u8], EthBlockInputError> {
    if bytes.len() > 32 {
        return Err(EthBlockInputError::ExtraDataOverflow {
            max_bytes: 32,
            found: bytes.len(),
        });
    }
    Ok(bytes)
}

fn quantity_to_u256_be(
    bytes: &[u8],
    overflow: impl FnOnce(usize) -> EthBlockInputError,
) -> Result<[u8; 32], EthBlockInputError> {
    if bytes.len() > 32 {
        return Err(overflow(bytes.len()));
    }
    let mut out = [0_u8; 32];
    out[32 - bytes.len()..].copy_from_slice(bytes);
    Ok(out)
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

    fn is_finished(&self) -> bool {
        self.offset == self.bytes.len()
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

    fn read_20_bytes(&mut self) -> Result<[u8; 20], EthBlockInputError> {
        Ok(self
            .read_exact(20)?
            .try_into()
            .expect("slice length checked"))
    }

    fn read_256_bytes(&mut self) -> Result<[u8; 256], EthBlockInputError> {
        Ok(self
            .read_exact(256)?
            .try_into()
            .expect("slice length checked"))
    }

    fn read_8_bytes(&mut self) -> Result<[u8; 8], EthBlockInputError> {
        Ok(self
            .read_exact(8)?
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
