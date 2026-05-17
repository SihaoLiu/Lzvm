use crate::rlp::{parse_rlp, RlpError, RlpItem};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EthBlockRlp {
    pub header: Vec<RlpItem>,
    pub transactions: Vec<RlpItem>,
    pub ommers: Vec<RlpItem>,
    pub withdrawals: Option<Vec<RlpItem>>,
    pub extra_body_fields: Vec<RlpItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EthBlockError {
    Rlp(RlpError),
    ExpectedBlockList,
    BodyFieldCount { found: usize },
    ExpectedHeaderList,
    ExpectedTransactionsList,
    ExpectedOmmersList,
    ExpectedWithdrawalsList,
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
        }
    }
}

impl std::error::Error for EthBlockError {}

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

fn take_list(item: RlpItem, error: EthBlockError) -> Result<Vec<RlpItem>, EthBlockError> {
    match item {
        RlpItem::List(items) => Ok(items),
        RlpItem::Bytes(_) => Err(error),
    }
}
