use std::fmt;

use crate::eth_block::eth_header_hash;
use crate::rlp::RlpItem;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EthPublicHeaderPrefix {
    pub header: EthPublicHeader,
    pub consumed: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EthPublicHeader {
    pub parent_hash: [u8; 32],
    pub ommers_hash: [u8; 32],
    pub beneficiary: [u8; 20],
    pub state_root: [u8; 32],
    pub transactions_root: [u8; 32],
    pub receipts_root: [u8; 32],
    pub withdrawals_root: Option<[u8; 32]>,
    pub logs_bloom: [u8; 256],
    pub difficulty: [u8; 32],
    pub block_number: u64,
    pub gas_limit: u64,
    pub gas_used: u64,
    pub timestamp: u64,
    pub mix_hash: [u8; 32],
    pub nonce: [u8; 8],
    pub base_fee_per_gas: Option<u64>,
    pub blob_gas_used: Option<u64>,
    pub excess_blob_gas: Option<u64>,
    pub parent_beacon_block_root: Option<[u8; 32]>,
    pub requests_hash: Option<[u8; 32]>,
    pub extra_data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EthPublicInputError {
    UnexpectedEof {
        offset: usize,
        needed: usize,
        available: usize,
    },
    LengthOverflow {
        field: &'static str,
        value: u64,
    },
    InvalidFixedBytesLength {
        field: &'static str,
        expected: usize,
        found: usize,
    },
    InvalidOptionTag {
        field: &'static str,
        found: u8,
    },
}

impl fmt::Display for EthPublicInputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedEof {
                offset,
                needed,
                available,
            } => write!(
                f,
                "unexpected end of ETH public input at {offset}, needed {needed}, available {available}"
            ),
            Self::LengthOverflow { field, value } => {
                write!(f, "ETH public input {field} length overflow: {value}")
            }
            Self::InvalidFixedBytesLength {
                field,
                expected,
                found,
            } => write!(
                f,
                "invalid ETH public input {field} length: expected {expected}, found {found}"
            ),
            Self::InvalidOptionTag { field, found } => {
                write!(f, "invalid ETH public input {field} option tag: {found}")
            }
        }
    }
}

impl std::error::Error for EthPublicInputError {}

pub fn parse_eth_public_header_prefix(
    bytes: &[u8],
) -> Result<EthPublicHeaderPrefix, EthPublicInputError> {
    let mut reader = PublicInputReader::new(bytes);
    let header = EthPublicHeader {
        parent_hash: reader.read_fixed_bytes("parent_hash")?,
        ommers_hash: reader.read_fixed_bytes("ommers_hash")?,
        beneficiary: reader.read_fixed_bytes("beneficiary")?,
        state_root: reader.read_fixed_bytes("state_root")?,
        transactions_root: reader.read_fixed_bytes("transactions_root")?,
        receipts_root: reader.read_fixed_bytes("receipts_root")?,
        withdrawals_root: reader.read_optional_fixed_bytes("withdrawals_root")?,
        logs_bloom: reader.read_fixed_bytes("logs_bloom")?,
        difficulty: reader.read_fixed_bytes("difficulty")?,
        block_number: reader.read_u64()?,
        gas_limit: reader.read_u64()?,
        gas_used: reader.read_u64()?,
        timestamp: reader.read_u64()?,
        mix_hash: reader.read_fixed_bytes("mix_hash")?,
        nonce: reader.read_fixed_bytes("nonce")?,
        base_fee_per_gas: reader.read_optional_u64("base_fee_per_gas")?,
        blob_gas_used: reader.read_optional_u64("blob_gas_used")?,
        excess_blob_gas: reader.read_optional_u64("excess_blob_gas")?,
        parent_beacon_block_root: reader.read_optional_fixed_bytes("parent_beacon_block_root")?,
        requests_hash: reader.read_optional_fixed_bytes("requests_hash")?,
        extra_data: reader.read_bytes("extra_data")?,
    };
    Ok(EthPublicHeaderPrefix {
        header,
        consumed: reader.offset(),
    })
}

pub fn eth_public_header_rlp_items(header: &EthPublicHeader) -> Vec<RlpItem> {
    let mut items = vec![
        bytes(&header.parent_hash),
        bytes(&header.ommers_hash),
        bytes(&header.beneficiary),
        bytes(&header.state_root),
        bytes(&header.transactions_root),
        bytes(&header.receipts_root),
        bytes(&header.logs_bloom),
        RlpItem::Bytes(quantity_bytes(&header.difficulty)),
        RlpItem::Bytes(u64_quantity_bytes(header.block_number)),
        RlpItem::Bytes(u64_quantity_bytes(header.gas_limit)),
        RlpItem::Bytes(u64_quantity_bytes(header.gas_used)),
        RlpItem::Bytes(u64_quantity_bytes(header.timestamp)),
        RlpItem::Bytes(header.extra_data.clone()),
        bytes(&header.mix_hash),
        bytes(&header.nonce),
    ];
    if let Some(value) = header.base_fee_per_gas {
        items.push(RlpItem::Bytes(u64_quantity_bytes(value)));
    }
    if let Some(value) = header.withdrawals_root {
        items.push(bytes(&value));
    }
    if let Some(value) = header.blob_gas_used {
        items.push(RlpItem::Bytes(u64_quantity_bytes(value)));
    }
    if let Some(value) = header.excess_blob_gas {
        items.push(RlpItem::Bytes(u64_quantity_bytes(value)));
    }
    if let Some(value) = header.parent_beacon_block_root {
        items.push(bytes(&value));
    }
    if let Some(value) = header.requests_hash {
        items.push(bytes(&value));
    }
    items
}

pub fn eth_public_header_hash(header: &EthPublicHeader) -> [u8; 32] {
    eth_header_hash(&eth_public_header_rlp_items(header))
}

fn bytes(value: &[u8]) -> RlpItem {
    RlpItem::Bytes(value.to_vec())
}

fn quantity_bytes(bytes: &[u8]) -> Vec<u8> {
    let first_nonzero = bytes
        .iter()
        .position(|byte| *byte != 0)
        .unwrap_or(bytes.len());
    bytes[first_nonzero..].to_vec()
}

fn u64_quantity_bytes(value: u64) -> Vec<u8> {
    quantity_bytes(&value.to_be_bytes())
}

struct PublicInputReader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> PublicInputReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn offset(&self) -> usize {
        self.offset
    }

    fn read_fixed_bytes<const N: usize>(
        &mut self,
        field: &'static str,
    ) -> Result<[u8; N], EthPublicInputError> {
        let bytes = self.read_bytes(field)?;
        bytes.try_into().map_err(
            |bytes: Vec<u8>| EthPublicInputError::InvalidFixedBytesLength {
                field,
                expected: N,
                found: bytes.len(),
            },
        )
    }

    fn read_optional_fixed_bytes<const N: usize>(
        &mut self,
        field: &'static str,
    ) -> Result<Option<[u8; N]>, EthPublicInputError> {
        match self.read_u8()? {
            0 => Ok(None),
            1 => self.read_fixed_bytes(field).map(Some),
            found => Err(EthPublicInputError::InvalidOptionTag { field, found }),
        }
    }

    fn read_optional_u64(
        &mut self,
        field: &'static str,
    ) -> Result<Option<u64>, EthPublicInputError> {
        match self.read_u8()? {
            0 => Ok(None),
            1 => self.read_u64().map(Some),
            found => Err(EthPublicInputError::InvalidOptionTag { field, found }),
        }
    }

    fn read_bytes(&mut self, field: &'static str) -> Result<Vec<u8>, EthPublicInputError> {
        let len = self.read_u64()?;
        let len = usize::try_from(len)
            .map_err(|_| EthPublicInputError::LengthOverflow { field, value: len })?;
        let bytes = self.read_exact(len)?;
        Ok(bytes.to_vec())
    }

    fn read_u8(&mut self) -> Result<u8, EthPublicInputError> {
        Ok(self.read_exact(1)?[0])
    }

    fn read_u64(&mut self) -> Result<u64, EthPublicInputError> {
        let bytes = self.read_exact(8)?;
        Ok(u64::from_le_bytes(
            bytes.try_into().expect("slice length checked"),
        ))
    }

    fn read_exact(&mut self, needed: usize) -> Result<&'a [u8], EthPublicInputError> {
        let end = self
            .offset
            .checked_add(needed)
            .ok_or(EthPublicInputError::UnexpectedEof {
                offset: self.offset,
                needed,
                available: self.bytes.len(),
            })?;
        if end > self.bytes.len() {
            return Err(EthPublicInputError::UnexpectedEof {
                offset: self.offset,
                needed,
                available: self.bytes.len(),
            });
        }
        let bytes = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(bytes)
    }
}
