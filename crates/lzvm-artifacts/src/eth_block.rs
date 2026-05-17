use crate::rlp::{encode_rlp, parse_rlp, RlpError, RlpItem};
use sha3::{Digest, Keccak256};
use std::fmt;

const BASE_HEADER_FIELD_COUNT: usize = 15;
const BASE_FEE_FIELD_INDEX: usize = 15;
const WITHDRAWALS_ROOT_FIELD_INDEX: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EthBlockRlp {
    pub header: Vec<RlpItem>,
    pub transactions: Vec<RlpItem>,
    pub ommers: Vec<RlpItem>,
    pub withdrawals: Option<Vec<RlpItem>>,
    pub extra_body_fields: Vec<RlpItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EthHeaderRlp {
    pub parent_hash: [u8; 32],
    pub ommers_hash: [u8; 32],
    pub beneficiary: [u8; 20],
    pub state_root: [u8; 32],
    pub transactions_root: [u8; 32],
    pub receipts_root: [u8; 32],
    pub logs_bloom: [u8; 256],
    pub difficulty: Vec<u8>,
    pub number: u64,
    pub gas_limit: u64,
    pub gas_used: u64,
    pub timestamp: u64,
    pub extra_data: Vec<u8>,
    pub mix_hash: [u8; 32],
    pub nonce: [u8; 8],
    pub base_fee_per_gas: Option<Vec<u8>>,
    pub withdrawals_root: Option<[u8; 32]>,
    pub extra_header_fields: Vec<RlpItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EthTransactionRlp {
    Legacy(Vec<RlpItem>),
    Typed {
        transaction_type: u8,
        payload: Vec<u8>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeaderField {
    ParentHash,
    OmmersHash,
    Beneficiary,
    StateRoot,
    TransactionsRoot,
    ReceiptsRoot,
    LogsBloom,
    Difficulty,
    Number,
    GasLimit,
    GasUsed,
    Timestamp,
    ExtraData,
    MixHash,
    Nonce,
    BaseFeePerGas,
    WithdrawalsRoot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EthTransactionError {
    EmptyTypedTransaction,
    InvalidTransactionType { found: u8 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EthBlockError {
    Rlp(RlpError),
    ExpectedBlockList,
    BodyFieldCount {
        found: usize,
    },
    ExpectedHeaderList,
    ExpectedTransactionsList,
    ExpectedOmmersList,
    ExpectedWithdrawalsList,
    HeaderFieldCount {
        found: usize,
    },
    ExpectedHeaderFieldBytes {
        field: HeaderField,
    },
    HeaderFieldLength {
        field: HeaderField,
        expected: usize,
        found: usize,
    },
    NonCanonicalQuantity {
        field: HeaderField,
    },
    QuantityOverflow {
        field: HeaderField,
        max_bytes: usize,
        found: usize,
    },
}

impl fmt::Display for EthBlockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Rlp(error) => write!(f, "{error}"),
            Self::ExpectedBlockList => write!(f, "expected RLP block body list"),
            Self::BodyFieldCount { found } => {
                write!(f, "expected at least 3 block body fields, found {found}")
            }
            Self::ExpectedHeaderList => write!(f, "expected block header list"),
            Self::ExpectedTransactionsList => write!(f, "expected transactions list"),
            Self::ExpectedOmmersList => write!(f, "expected ommers list"),
            Self::ExpectedWithdrawalsList => write!(f, "expected withdrawals list"),
            Self::HeaderFieldCount { found } => {
                write!(f, "expected at least 15 header fields, found {found}")
            }
            Self::ExpectedHeaderFieldBytes { field } => {
                write!(f, "expected header field {field} to be bytes")
            }
            Self::HeaderFieldLength {
                field,
                expected,
                found,
            } => write!(
                f,
                "expected header field {field} to have length {expected}, found {found}"
            ),
            Self::NonCanonicalQuantity { field } => {
                write!(f, "non-canonical quantity in header field {field}")
            }
            Self::QuantityOverflow {
                field,
                max_bytes,
                found,
            } => write!(
                f,
                "header field {field} quantity exceeds {max_bytes} bytes, found {found}"
            ),
        }
    }
}

impl std::error::Error for EthBlockError {}

impl fmt::Display for EthTransactionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyTypedTransaction => write!(f, "empty typed transaction envelope"),
            Self::InvalidTransactionType { found } => {
                write!(f, "invalid transaction type byte: 0x{found:02x}")
            }
        }
    }
}

impl std::error::Error for EthTransactionError {}

impl fmt::Display for HeaderField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::ParentHash => "parent_hash",
            Self::OmmersHash => "ommers_hash",
            Self::Beneficiary => "beneficiary",
            Self::StateRoot => "state_root",
            Self::TransactionsRoot => "transactions_root",
            Self::ReceiptsRoot => "receipts_root",
            Self::LogsBloom => "logs_bloom",
            Self::Difficulty => "difficulty",
            Self::Number => "number",
            Self::GasLimit => "gas_limit",
            Self::GasUsed => "gas_used",
            Self::Timestamp => "timestamp",
            Self::ExtraData => "extra_data",
            Self::MixHash => "mix_hash",
            Self::Nonce => "nonce",
            Self::BaseFeePerGas => "base_fee_per_gas",
            Self::WithdrawalsRoot => "withdrawals_root",
        };
        f.write_str(name)
    }
}

impl From<RlpError> for EthBlockError {
    fn from(error: RlpError) -> Self {
        Self::Rlp(error)
    }
}

pub fn parse_eth_block_rlp(bytes: &[u8]) -> Result<EthBlockRlp, EthBlockError> {
    let fields = match parse_rlp(bytes)? {
        RlpItem::List(fields) => fields,
        RlpItem::Bytes(_) => return Err(EthBlockError::ExpectedBlockList),
    };

    if fields.len() < 3 {
        return Err(EthBlockError::BodyFieldCount {
            found: fields.len(),
        });
    }

    let mut fields = fields.into_iter();
    let header = take_list(
        fields.next().expect("block body field count checked"),
        EthBlockError::ExpectedHeaderList,
    )?;
    let transactions = take_list(
        fields.next().expect("block body field count checked"),
        EthBlockError::ExpectedTransactionsList,
    )?;
    let ommers = take_list(
        fields.next().expect("block body field count checked"),
        EthBlockError::ExpectedOmmersList,
    )?;
    let withdrawals = fields
        .next()
        .map(|item| take_list(item, EthBlockError::ExpectedWithdrawalsList))
        .transpose()?;
    let extra_body_fields = fields.collect();

    Ok(EthBlockRlp {
        header,
        transactions,
        ommers,
        withdrawals,
        extra_body_fields,
    })
}

pub fn decode_eth_header_rlp(header: &[RlpItem]) -> Result<EthHeaderRlp, EthBlockError> {
    if header.len() < BASE_HEADER_FIELD_COUNT {
        return Err(EthBlockError::HeaderFieldCount {
            found: header.len(),
        });
    }

    let base_fee_per_gas = if header.len() > BASE_FEE_FIELD_INDEX {
        Some(quantity_bytes(
            &header[BASE_FEE_FIELD_INDEX],
            HeaderField::BaseFeePerGas,
        )?)
    } else {
        None
    };
    let withdrawals_root = if header.len() > WITHDRAWALS_ROOT_FIELD_INDEX {
        Some(fixed_bytes::<32>(
            &header[WITHDRAWALS_ROOT_FIELD_INDEX],
            HeaderField::WithdrawalsRoot,
        )?)
    } else {
        None
    };
    let extra_header_fields = header
        .get(WITHDRAWALS_ROOT_FIELD_INDEX + 1..)
        .unwrap_or_default()
        .to_vec();

    Ok(EthHeaderRlp {
        parent_hash: fixed_bytes::<32>(&header[0], HeaderField::ParentHash)?,
        ommers_hash: fixed_bytes::<32>(&header[1], HeaderField::OmmersHash)?,
        beneficiary: fixed_bytes::<20>(&header[2], HeaderField::Beneficiary)?,
        state_root: fixed_bytes::<32>(&header[3], HeaderField::StateRoot)?,
        transactions_root: fixed_bytes::<32>(&header[4], HeaderField::TransactionsRoot)?,
        receipts_root: fixed_bytes::<32>(&header[5], HeaderField::ReceiptsRoot)?,
        logs_bloom: fixed_bytes::<256>(&header[6], HeaderField::LogsBloom)?,
        difficulty: quantity_bytes(&header[7], HeaderField::Difficulty)?,
        number: quantity_u64(&header[8], HeaderField::Number)?,
        gas_limit: quantity_u64(&header[9], HeaderField::GasLimit)?,
        gas_used: quantity_u64(&header[10], HeaderField::GasUsed)?,
        timestamp: quantity_u64(&header[11], HeaderField::Timestamp)?,
        extra_data: bytes(&header[12], HeaderField::ExtraData)?.to_vec(),
        mix_hash: fixed_bytes::<32>(&header[13], HeaderField::MixHash)?,
        nonce: fixed_bytes::<8>(&header[14], HeaderField::Nonce)?,
        base_fee_per_gas,
        withdrawals_root,
        extra_header_fields,
    })
}

pub fn decode_eth_transaction_rlp(
    transaction: &RlpItem,
) -> Result<EthTransactionRlp, EthTransactionError> {
    match transaction {
        RlpItem::List(fields) => Ok(EthTransactionRlp::Legacy(fields.clone())),
        RlpItem::Bytes(bytes) => {
            let Some((&transaction_type, payload)) = bytes.split_first() else {
                return Err(EthTransactionError::EmptyTypedTransaction);
            };
            if transaction_type > 0x7f {
                return Err(EthTransactionError::InvalidTransactionType {
                    found: transaction_type,
                });
            }
            Ok(EthTransactionRlp::Typed {
                transaction_type,
                payload: payload.to_vec(),
            })
        }
    }
}

pub fn decode_eth_transactions_rlp(
    transactions: &[RlpItem],
) -> Result<Vec<EthTransactionRlp>, EthTransactionError> {
    transactions
        .iter()
        .map(decode_eth_transaction_rlp)
        .collect()
}

pub fn keccak256(bytes: &[u8]) -> [u8; 32] {
    Keccak256::digest(bytes).into()
}

pub fn eth_header_hash(header: &[RlpItem]) -> [u8; 32] {
    keccak256(&encode_rlp(&RlpItem::List(header.to_vec())))
}

pub fn eth_ommers_hash(ommers: &[RlpItem]) -> [u8; 32] {
    keccak256(&encode_rlp(&RlpItem::List(ommers.to_vec())))
}

fn take_list(item: RlpItem, error: EthBlockError) -> Result<Vec<RlpItem>, EthBlockError> {
    match item {
        RlpItem::List(items) => Ok(items),
        RlpItem::Bytes(_) => Err(error),
    }
}

fn fixed_bytes<const N: usize>(
    item: &RlpItem,
    field: HeaderField,
) -> Result<[u8; N], EthBlockError> {
    let bytes = bytes(item, field)?;
    bytes
        .try_into()
        .map_err(|_| EthBlockError::HeaderFieldLength {
            field,
            expected: N,
            found: bytes.len(),
        })
}

fn quantity_bytes(item: &RlpItem, field: HeaderField) -> Result<Vec<u8>, EthBlockError> {
    let bytes = bytes(item, field)?;
    if bytes.first() == Some(&0) {
        return Err(EthBlockError::NonCanonicalQuantity { field });
    }
    Ok(bytes.to_vec())
}

fn quantity_u64(item: &RlpItem, field: HeaderField) -> Result<u64, EthBlockError> {
    let bytes = quantity_bytes(item, field)?;
    if bytes.len() > 8 {
        return Err(EthBlockError::QuantityOverflow {
            field,
            max_bytes: 8,
            found: bytes.len(),
        });
    }

    let mut value = 0_u64;
    for byte in bytes {
        value = (value << 8) | u64::from(byte);
    }
    Ok(value)
}

fn bytes(item: &RlpItem, field: HeaderField) -> Result<&[u8], EthBlockError> {
    match item {
        RlpItem::Bytes(bytes) => Ok(bytes),
        RlpItem::List(_) => Err(EthBlockError::ExpectedHeaderFieldBytes { field }),
    }
}
