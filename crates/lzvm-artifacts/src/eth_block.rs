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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EthReceiptRlp {
    Legacy {
        status_or_post_state: Vec<u8>,
        cumulative_gas_used: u64,
        logs_bloom: Box<[u8; 256]>,
        logs: Vec<EthLogRlp>,
    },
    Typed {
        receipt_type: u8,
        payload: Vec<u8>,
        status_or_post_state: Vec<u8>,
        cumulative_gas_used: u64,
        logs_bloom: Box<[u8; 256]>,
        logs: Vec<EthLogRlp>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct EthReceiptBodyRlp {
    status_or_post_state: Vec<u8>,
    cumulative_gas_used: u64,
    logs_bloom: [u8; 256],
    logs: Vec<EthLogRlp>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EthLogRlp {
    pub address: [u8; 20],
    pub topics: Vec<[u8; 32]>,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EthWithdrawalRlp {
    pub index: u64,
    pub validator_index: u64,
    pub address: [u8; 20],
    pub amount: u64,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WithdrawalField {
    Index,
    ValidatorIndex,
    Address,
    Amount,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceiptField {
    StatusOrPostState,
    CumulativeGasUsed,
    LogsBloom,
    Logs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogField {
    Address,
    Topic,
    Data,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EthTransactionError {
    Rlp(RlpError),
    EmptyTypedTransaction,
    InvalidTransactionType { found: u8 },
    ExpectedTransactionList,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EthReceiptError {
    Log(EthLogError),
    Rlp(RlpError),
    EmptyTypedReceipt,
    InvalidReceiptType {
        found: u8,
    },
    ExpectedReceiptList,
    ReceiptFieldCount {
        found: usize,
    },
    ExpectedReceiptFieldBytes {
        field: ReceiptField,
    },
    ReceiptFieldLength {
        field: ReceiptField,
        expected: usize,
        found: usize,
    },
    ExpectedLogsList,
    NonCanonicalReceiptQuantity {
        field: ReceiptField,
    },
    ReceiptQuantityOverflow {
        field: ReceiptField,
        max_bytes: usize,
        found: usize,
    },
    LogsBloomMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EthLogError {
    ExpectedLogList,
    LogFieldCount {
        found: usize,
    },
    ExpectedLogFieldBytes {
        field: LogField,
    },
    LogFieldLength {
        field: LogField,
        expected: usize,
        found: usize,
    },
    ExpectedTopicsList,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EthWithdrawalError {
    ExpectedWithdrawalList,
    WithdrawalFieldCount {
        found: usize,
    },
    ExpectedWithdrawalFieldBytes {
        field: WithdrawalField,
    },
    WithdrawalFieldLength {
        field: WithdrawalField,
        expected: usize,
        found: usize,
    },
    NonCanonicalWithdrawalQuantity {
        field: WithdrawalField,
    },
    WithdrawalQuantityOverflow {
        field: WithdrawalField,
        max_bytes: usize,
        found: usize,
    },
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
            Self::Rlp(error) => write!(f, "typed transaction RLP error: {error}"),
            Self::EmptyTypedTransaction => write!(f, "empty typed transaction envelope"),
            Self::InvalidTransactionType { found } => {
                write!(f, "invalid transaction type byte: 0x{found:02x}")
            }
            Self::ExpectedTransactionList => write!(f, "expected transaction list"),
        }
    }
}

impl std::error::Error for EthTransactionError {}

impl fmt::Display for EthReceiptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Log(error) => write!(f, "{error}"),
            Self::Rlp(error) => write!(f, "typed receipt RLP error: {error}"),
            Self::EmptyTypedReceipt => write!(f, "empty typed receipt envelope"),
            Self::InvalidReceiptType { found } => {
                write!(f, "invalid receipt type byte: 0x{found:02x}")
            }
            Self::ExpectedReceiptList => write!(f, "expected receipt list"),
            Self::ReceiptFieldCount { found } => {
                write!(f, "expected 4 receipt fields, found {found}")
            }
            Self::ExpectedReceiptFieldBytes { field } => {
                write!(f, "expected receipt field {field} to be bytes")
            }
            Self::ReceiptFieldLength {
                field,
                expected,
                found,
            } => write!(
                f,
                "expected receipt field {field} to have length {expected}, found {found}"
            ),
            Self::ExpectedLogsList => write!(f, "expected receipt logs list"),
            Self::NonCanonicalReceiptQuantity { field } => {
                write!(f, "non-canonical quantity in receipt field {field}")
            }
            Self::ReceiptQuantityOverflow {
                field,
                max_bytes,
                found,
            } => write!(
                f,
                "receipt field {field} quantity exceeds {max_bytes} bytes, found {found}"
            ),
            Self::LogsBloomMismatch => write!(f, "receipt logs bloom mismatch"),
        }
    }
}

impl std::error::Error for EthReceiptError {}

impl fmt::Display for EthLogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ExpectedLogList => write!(f, "expected log list"),
            Self::LogFieldCount { found } => write!(f, "expected 3 log fields, found {found}"),
            Self::ExpectedLogFieldBytes { field } => {
                write!(f, "expected log field {field} to be bytes")
            }
            Self::LogFieldLength {
                field,
                expected,
                found,
            } => write!(
                f,
                "expected log field {field} to have length {expected}, found {found}"
            ),
            Self::ExpectedTopicsList => write!(f, "expected log topics list"),
        }
    }
}

impl std::error::Error for EthLogError {}

impl fmt::Display for EthWithdrawalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ExpectedWithdrawalList => write!(f, "expected withdrawal list"),
            Self::WithdrawalFieldCount { found } => {
                write!(f, "expected 4 withdrawal fields, found {found}")
            }
            Self::ExpectedWithdrawalFieldBytes { field } => {
                write!(f, "expected withdrawal field {field} to be bytes")
            }
            Self::WithdrawalFieldLength {
                field,
                expected,
                found,
            } => write!(
                f,
                "expected withdrawal field {field} to have length {expected}, found {found}"
            ),
            Self::NonCanonicalWithdrawalQuantity { field } => {
                write!(f, "non-canonical quantity in withdrawal field {field}")
            }
            Self::WithdrawalQuantityOverflow {
                field,
                max_bytes,
                found,
            } => write!(
                f,
                "withdrawal field {field} quantity exceeds {max_bytes} bytes, found {found}"
            ),
        }
    }
}

impl std::error::Error for EthWithdrawalError {}

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

impl fmt::Display for WithdrawalField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Index => "index",
            Self::ValidatorIndex => "validator_index",
            Self::Address => "address",
            Self::Amount => "amount",
        };
        f.write_str(name)
    }
}

impl fmt::Display for ReceiptField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::StatusOrPostState => "status_or_post_state",
            Self::CumulativeGasUsed => "cumulative_gas_used",
            Self::LogsBloom => "logs_bloom",
            Self::Logs => "logs",
        };
        f.write_str(name)
    }
}

impl fmt::Display for LogField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Address => "address",
            Self::Topic => "topic",
            Self::Data => "data",
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
            if !matches!(
                parse_rlp(payload).map_err(EthTransactionError::Rlp)?,
                RlpItem::List(_)
            ) {
                return Err(EthTransactionError::ExpectedTransactionList);
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

pub fn decode_eth_log_rlp(log: &RlpItem) -> Result<EthLogRlp, EthLogError> {
    let fields = match log {
        RlpItem::List(fields) => fields,
        RlpItem::Bytes(_) => return Err(EthLogError::ExpectedLogList),
    };
    if fields.len() != 3 {
        return Err(EthLogError::LogFieldCount {
            found: fields.len(),
        });
    }
    let topics = log_topics(&fields[1])?
        .iter()
        .map(|topic| log_fixed_bytes::<32>(topic, LogField::Topic))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(EthLogRlp {
        address: log_fixed_bytes::<20>(&fields[0], LogField::Address)?,
        topics,
        data: log_bytes(&fields[2], LogField::Data)?.to_vec(),
    })
}

pub fn decode_eth_logs_rlp(logs: &[RlpItem]) -> Result<Vec<EthLogRlp>, EthLogError> {
    logs.iter().map(decode_eth_log_rlp).collect()
}

pub fn eth_logs_bloom(logs: &[EthLogRlp]) -> [u8; 256] {
    let mut bloom = [0_u8; 256];
    for log in logs {
        add_bloom_value(&mut bloom, &log.address);
        for topic in &log.topics {
            add_bloom_value(&mut bloom, topic);
        }
    }
    bloom
}

pub fn eth_receipts_logs_bloom(receipts: &[EthReceiptRlp]) -> Option<[u8; 256]> {
    let mut bloom = [0_u8; 256];
    for receipt in receipts {
        match receipt {
            EthReceiptRlp::Legacy { logs_bloom, .. } | EthReceiptRlp::Typed { logs_bloom, .. } => {
                for (target, source) in bloom.iter_mut().zip(logs_bloom.iter()) {
                    *target |= *source;
                }
            }
        }
    }
    Some(bloom)
}

pub fn eth_receipts_cumulative_gas_used(receipts: &[EthReceiptRlp]) -> Option<u64> {
    let mut gas_used = 0;
    for receipt in receipts {
        match receipt {
            EthReceiptRlp::Legacy {
                cumulative_gas_used,
                ..
            }
            | EthReceiptRlp::Typed {
                cumulative_gas_used,
                ..
            } => gas_used = *cumulative_gas_used,
        }
    }
    Some(gas_used)
}

pub fn eth_receipts_cumulative_gas_is_nondecreasing(receipts: &[EthReceiptRlp]) -> Option<bool> {
    let mut previous_gas_used = 0;
    for receipt in receipts {
        match receipt {
            EthReceiptRlp::Legacy {
                cumulative_gas_used,
                ..
            }
            | EthReceiptRlp::Typed {
                cumulative_gas_used,
                ..
            } => {
                if *cumulative_gas_used < previous_gas_used {
                    return Some(false);
                }
                previous_gas_used = *cumulative_gas_used;
            }
        }
    }
    Some(true)
}

pub fn decode_eth_receipt_rlp(receipt: &RlpItem) -> Result<EthReceiptRlp, EthReceiptError> {
    match receipt {
        RlpItem::List(fields) => decode_legacy_eth_receipt(fields),
        RlpItem::Bytes(bytes) => {
            let Some((&receipt_type, payload)) = bytes.split_first() else {
                return Err(EthReceiptError::EmptyTypedReceipt);
            };
            if receipt_type > 0x7f {
                return Err(EthReceiptError::InvalidReceiptType {
                    found: receipt_type,
                });
            }
            let fields = match parse_rlp(payload).map_err(EthReceiptError::Rlp)? {
                RlpItem::List(fields) => fields,
                RlpItem::Bytes(_) => return Err(EthReceiptError::ExpectedReceiptList),
            };
            let body = decode_eth_receipt_body(&fields)?;
            Ok(EthReceiptRlp::Typed {
                receipt_type,
                payload: payload.to_vec(),
                status_or_post_state: body.status_or_post_state,
                cumulative_gas_used: body.cumulative_gas_used,
                logs_bloom: Box::new(body.logs_bloom),
                logs: body.logs,
            })
        }
    }
}

pub fn decode_eth_receipts_rlp(
    receipts: &[RlpItem],
) -> Result<Vec<EthReceiptRlp>, EthReceiptError> {
    receipts.iter().map(decode_eth_receipt_rlp).collect()
}

fn decode_legacy_eth_receipt(fields: &[RlpItem]) -> Result<EthReceiptRlp, EthReceiptError> {
    let body = decode_eth_receipt_body(fields)?;
    Ok(EthReceiptRlp::Legacy {
        status_or_post_state: body.status_or_post_state,
        cumulative_gas_used: body.cumulative_gas_used,
        logs_bloom: Box::new(body.logs_bloom),
        logs: body.logs,
    })
}

fn decode_eth_receipt_body(fields: &[RlpItem]) -> Result<EthReceiptBodyRlp, EthReceiptError> {
    if fields.len() != 4 {
        return Err(EthReceiptError::ReceiptFieldCount {
            found: fields.len(),
        });
    }
    let logs_bloom = receipt_fixed_bytes::<256>(&fields[2], ReceiptField::LogsBloom)?;
    let logs = decode_eth_logs_rlp(receipt_logs(&fields[3])?).map_err(EthReceiptError::Log)?;
    if logs_bloom != eth_logs_bloom(&logs) {
        return Err(EthReceiptError::LogsBloomMismatch);
    }
    Ok(EthReceiptBodyRlp {
        status_or_post_state: receipt_bytes(&fields[0], ReceiptField::StatusOrPostState)?.to_vec(),
        cumulative_gas_used: receipt_quantity_u64(&fields[1], ReceiptField::CumulativeGasUsed)?,
        logs_bloom,
        logs,
    })
}

pub fn decode_eth_withdrawal_rlp(
    withdrawal: &RlpItem,
) -> Result<EthWithdrawalRlp, EthWithdrawalError> {
    let fields = match withdrawal {
        RlpItem::List(fields) => fields,
        RlpItem::Bytes(_) => return Err(EthWithdrawalError::ExpectedWithdrawalList),
    };
    if fields.len() != 4 {
        return Err(EthWithdrawalError::WithdrawalFieldCount {
            found: fields.len(),
        });
    }

    Ok(EthWithdrawalRlp {
        index: withdrawal_quantity_u64(&fields[0], WithdrawalField::Index)?,
        validator_index: withdrawal_quantity_u64(&fields[1], WithdrawalField::ValidatorIndex)?,
        address: withdrawal_fixed_bytes::<20>(&fields[2], WithdrawalField::Address)?,
        amount: withdrawal_quantity_u64(&fields[3], WithdrawalField::Amount)?,
    })
}

pub fn decode_eth_withdrawals_rlp(
    withdrawals: &[RlpItem],
) -> Result<Vec<EthWithdrawalRlp>, EthWithdrawalError> {
    withdrawals.iter().map(decode_eth_withdrawal_rlp).collect()
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

fn add_bloom_value(bloom: &mut [u8; 256], value: &[u8]) {
    let hash = keccak256(value);
    for index in 0..3 {
        let bit = (((hash[2 * index] as usize) << 8) | hash[2 * index + 1] as usize) & 2047;
        bloom[255 - bit / 8] |= 1 << (bit % 8);
    }
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

fn log_fixed_bytes<const N: usize>(
    item: &RlpItem,
    field: LogField,
) -> Result<[u8; N], EthLogError> {
    let bytes = log_bytes(item, field)?;
    bytes.try_into().map_err(|_| EthLogError::LogFieldLength {
        field,
        expected: N,
        found: bytes.len(),
    })
}

fn log_bytes(item: &RlpItem, field: LogField) -> Result<&[u8], EthLogError> {
    match item {
        RlpItem::Bytes(bytes) => Ok(bytes),
        RlpItem::List(_) => Err(EthLogError::ExpectedLogFieldBytes { field }),
    }
}

fn log_topics(item: &RlpItem) -> Result<&[RlpItem], EthLogError> {
    match item {
        RlpItem::List(topics) => Ok(topics),
        RlpItem::Bytes(_) => Err(EthLogError::ExpectedTopicsList),
    }
}

fn receipt_fixed_bytes<const N: usize>(
    item: &RlpItem,
    field: ReceiptField,
) -> Result<[u8; N], EthReceiptError> {
    let bytes = receipt_bytes(item, field)?;
    bytes
        .try_into()
        .map_err(|_| EthReceiptError::ReceiptFieldLength {
            field,
            expected: N,
            found: bytes.len(),
        })
}

fn receipt_quantity_bytes(item: &RlpItem, field: ReceiptField) -> Result<Vec<u8>, EthReceiptError> {
    let bytes = receipt_bytes(item, field)?;
    if bytes.first() == Some(&0) {
        return Err(EthReceiptError::NonCanonicalReceiptQuantity { field });
    }
    Ok(bytes.to_vec())
}

fn receipt_quantity_u64(item: &RlpItem, field: ReceiptField) -> Result<u64, EthReceiptError> {
    let bytes = receipt_quantity_bytes(item, field)?;
    if bytes.len() > 8 {
        return Err(EthReceiptError::ReceiptQuantityOverflow {
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

fn receipt_bytes(item: &RlpItem, field: ReceiptField) -> Result<&[u8], EthReceiptError> {
    match item {
        RlpItem::Bytes(bytes) => Ok(bytes),
        RlpItem::List(_) => Err(EthReceiptError::ExpectedReceiptFieldBytes { field }),
    }
}

fn receipt_logs(item: &RlpItem) -> Result<&[RlpItem], EthReceiptError> {
    match item {
        RlpItem::List(logs) => Ok(logs),
        RlpItem::Bytes(_) => Err(EthReceiptError::ExpectedLogsList),
    }
}

fn withdrawal_fixed_bytes<const N: usize>(
    item: &RlpItem,
    field: WithdrawalField,
) -> Result<[u8; N], EthWithdrawalError> {
    let bytes = withdrawal_bytes(item, field)?;
    bytes
        .try_into()
        .map_err(|_| EthWithdrawalError::WithdrawalFieldLength {
            field,
            expected: N,
            found: bytes.len(),
        })
}

fn withdrawal_quantity_bytes(
    item: &RlpItem,
    field: WithdrawalField,
) -> Result<Vec<u8>, EthWithdrawalError> {
    let bytes = withdrawal_bytes(item, field)?;
    if bytes.first() == Some(&0) {
        return Err(EthWithdrawalError::NonCanonicalWithdrawalQuantity { field });
    }
    Ok(bytes.to_vec())
}

fn withdrawal_quantity_u64(
    item: &RlpItem,
    field: WithdrawalField,
) -> Result<u64, EthWithdrawalError> {
    let bytes = withdrawal_quantity_bytes(item, field)?;
    if bytes.len() > 8 {
        return Err(EthWithdrawalError::WithdrawalQuantityOverflow {
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

fn withdrawal_bytes(item: &RlpItem, field: WithdrawalField) -> Result<&[u8], EthWithdrawalError> {
    match item {
        RlpItem::Bytes(bytes) => Ok(bytes),
        RlpItem::List(_) => Err(EthWithdrawalError::ExpectedWithdrawalFieldBytes { field }),
    }
}
