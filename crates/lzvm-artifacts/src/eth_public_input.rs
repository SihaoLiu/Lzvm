use std::fmt;

use crate::eth_block::{eth_header_hash, eth_ommers_hash};
use crate::eth_trie::{transaction_trie_root, withdrawals_trie_root};
use crate::rlp::{encode_rlp, RlpItem};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EthPublicHeaderPrefix {
    pub header: EthPublicHeader,
    pub consumed: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EthPublicTransactionsPrefix {
    pub header: EthPublicHeader,
    pub transactions: Vec<RlpItem>,
    pub consumed: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EthPublicBlockPrefix {
    pub header: EthPublicHeader,
    pub transactions: Vec<RlpItem>,
    pub ommers: Vec<RlpItem>,
    pub withdrawals: Option<Vec<RlpItem>>,
    pub consumed: usize,
}

impl EthPublicBlockPrefix {
    pub fn block_rlp(&self) -> Vec<u8> {
        let mut fields = vec![
            RlpItem::List(eth_public_header_rlp_items(&self.header)),
            RlpItem::List(self.transactions.clone()),
            RlpItem::List(self.ommers.clone()),
        ];
        if let Some(withdrawals) = &self.withdrawals {
            fields.push(RlpItem::List(withdrawals.clone()));
        }
        encode_rlp(&RlpItem::List(fields))
    }

    pub fn transactions_root(&self) -> [u8; 32] {
        transaction_trie_root(&self.transactions)
            .expect("ETH public input transactions should encode as valid transaction RLP")
    }

    pub fn transactions_root_matches(&self) -> bool {
        self.transactions_root() == self.header.transactions_root
    }

    pub fn legacy_transaction_count(&self) -> usize {
        legacy_transaction_count(&self.transactions)
    }

    pub fn typed_transaction_count(&self) -> usize {
        typed_transaction_count(&self.transactions)
    }

    pub fn ommers_hash(&self) -> [u8; 32] {
        eth_ommers_hash(&self.ommers)
    }

    pub fn ommers_hash_matches(&self) -> bool {
        self.ommers_hash() == self.header.ommers_hash
    }

    pub fn withdrawals_root(&self) -> Option<[u8; 32]> {
        self.withdrawals.as_deref().map(withdrawals_trie_root)
    }

    pub fn withdrawals_root_matches(&self) -> bool {
        self.withdrawals_root() == self.header.withdrawals_root
    }
}

impl EthPublicTransactionsPrefix {
    pub fn transactions_root(&self) -> [u8; 32] {
        transaction_trie_root(&self.transactions)
            .expect("ETH public input transactions should encode as valid transaction RLP")
    }

    pub fn transactions_root_matches(&self) -> bool {
        self.transactions_root() == self.header.transactions_root
    }

    pub fn legacy_transaction_count(&self) -> usize {
        legacy_transaction_count(&self.transactions)
    }

    pub fn typed_transaction_count(&self) -> usize {
        typed_transaction_count(&self.transactions)
    }
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
    InvalidTransactionVariant {
        index: usize,
        found: u32,
    },
    InvalidParity {
        field: &'static str,
        found: u64,
    },
    NumericOverflow {
        field: &'static str,
    },
    TrailingBytes {
        consumed: usize,
        total: usize,
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
            Self::InvalidTransactionVariant { index, found } => write!(
                f,
                "invalid ETH public input transaction {index} type: {found}"
            ),
            Self::InvalidParity { field, found } => {
                write!(f, "invalid ETH public input {field} parity: {found}")
            }
            Self::NumericOverflow { field } => {
                write!(f, "ETH public input {field} numeric overflow")
            }
            Self::TrailingBytes { consumed, total } => {
                write!(
                    f,
                    "unexpected trailing bytes in ETH public input: {}",
                    total - consumed
                )
            }
        }
    }
}

impl std::error::Error for EthPublicInputError {}

pub fn parse_eth_public_header_prefix(
    bytes: &[u8],
) -> Result<EthPublicHeaderPrefix, EthPublicInputError> {
    let mut reader = PublicInputReader::new(bytes);
    let header = read_eth_public_header(&mut reader)?;
    Ok(EthPublicHeaderPrefix {
        header,
        consumed: reader.offset(),
    })
}

pub fn parse_eth_public_transactions_prefix(
    bytes: &[u8],
) -> Result<EthPublicTransactionsPrefix, EthPublicInputError> {
    let mut reader = PublicInputReader::new(bytes);
    let header = read_eth_public_header(&mut reader)?;
    let transactions = reader.read_transactions()?;
    Ok(EthPublicTransactionsPrefix {
        header,
        transactions,
        consumed: reader.offset(),
    })
}

pub fn parse_eth_public_block_prefix(
    bytes: &[u8],
) -> Result<EthPublicBlockPrefix, EthPublicInputError> {
    let mut reader = PublicInputReader::new(bytes);
    let header = read_eth_public_header(&mut reader)?;
    let transactions = reader.read_transactions()?;
    let ommers = reader.read_ommers()?;
    let withdrawals = reader.read_withdrawals()?;
    Ok(EthPublicBlockPrefix {
        header,
        transactions,
        ommers,
        withdrawals,
        consumed: reader.offset(),
    })
}

pub fn parse_eth_public_block(bytes: &[u8]) -> Result<EthPublicBlockPrefix, EthPublicInputError> {
    let block = parse_eth_public_block_prefix(bytes)?;
    if block.consumed != bytes.len() {
        return Err(EthPublicInputError::TrailingBytes {
            consumed: block.consumed,
            total: bytes.len(),
        });
    }
    Ok(block)
}

fn read_eth_public_header(
    reader: &mut PublicInputReader<'_>,
) -> Result<EthPublicHeader, EthPublicInputError> {
    Ok(EthPublicHeader {
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

fn u128_quantity_bytes(value: u128) -> Vec<u8> {
    quantity_bytes(&value.to_be_bytes())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct EthPublicSignature {
    y_parity: u64,
    r: Vec<u8>,
    s: Vec<u8>,
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
        let len = self.len_to_usize(field, len)?;
        let bytes = self.read_exact(len)?;
        Ok(bytes.to_vec())
    }

    fn read_len(&mut self, field: &'static str) -> Result<usize, EthPublicInputError> {
        let len = self.read_u64()?;
        self.len_to_usize(field, len)
    }

    fn read_u8(&mut self) -> Result<u8, EthPublicInputError> {
        Ok(self.read_exact(1)?[0])
    }

    fn read_u32(&mut self) -> Result<u32, EthPublicInputError> {
        let bytes = self.read_exact(4)?;
        Ok(u32::from_le_bytes(
            bytes.try_into().expect("slice length checked"),
        ))
    }

    fn read_u64(&mut self) -> Result<u64, EthPublicInputError> {
        let bytes = self.read_exact(8)?;
        Ok(u64::from_le_bytes(
            bytes.try_into().expect("slice length checked"),
        ))
    }

    fn read_u128(&mut self) -> Result<u128, EthPublicInputError> {
        let bytes = self.read_exact(16)?;
        Ok(u128::from_le_bytes(
            bytes.try_into().expect("slice length checked"),
        ))
    }

    fn read_signed_transaction(&mut self, index: usize) -> Result<RlpItem, EthPublicInputError> {
        let signature = self.read_signature()?;
        match self.read_u32()? {
            0 => self.read_legacy_transaction(&signature),
            1 => self.read_eip2930_transaction(&signature),
            2 => self.read_eip1559_transaction(&signature),
            3 => self.read_eip4844_transaction(&signature),
            4 => self.read_eip7702_transaction(&signature),
            found => Err(EthPublicInputError::InvalidTransactionVariant { index, found }),
        }
    }

    fn read_transactions(&mut self) -> Result<Vec<RlpItem>, EthPublicInputError> {
        let transaction_count = self.read_len("transactions")?;
        let mut transactions = Vec::with_capacity(transaction_count);
        for index in 0..transaction_count {
            transactions.push(self.read_signed_transaction(index)?);
        }
        Ok(transactions)
    }

    fn read_ommers(&mut self) -> Result<Vec<RlpItem>, EthPublicInputError> {
        let ommer_count = self.read_len("ommers")?;
        let mut ommers = Vec::with_capacity(ommer_count);
        for _ in 0..ommer_count {
            let header = read_eth_public_header(self)?;
            ommers.push(RlpItem::List(eth_public_header_rlp_items(&header)));
        }
        Ok(ommers)
    }

    fn read_withdrawals(&mut self) -> Result<Option<Vec<RlpItem>>, EthPublicInputError> {
        match self.read_u8()? {
            0 => Ok(None),
            1 => {
                let withdrawal_count = self.read_len("withdrawals")?;
                let mut withdrawals = Vec::with_capacity(withdrawal_count);
                for _ in 0..withdrawal_count {
                    let index = self.read_quantity_u64("withdrawal_index")?;
                    let validator_index = self.read_quantity_u64("withdrawal_validator_index")?;
                    let address = self.read_address("withdrawal_address")?.to_vec();
                    let amount = self.read_quantity_u64("withdrawal_amount")?;
                    withdrawals.push(RlpItem::List(vec![
                        RlpItem::Bytes(u64_quantity_bytes(index)),
                        RlpItem::Bytes(u64_quantity_bytes(validator_index)),
                        RlpItem::Bytes(address),
                        RlpItem::Bytes(u64_quantity_bytes(amount)),
                    ]));
                }
                Ok(Some(withdrawals))
            }
            found => Err(EthPublicInputError::InvalidOptionTag {
                field: "withdrawals",
                found,
            }),
        }
    }

    fn read_signature(&mut self) -> Result<EthPublicSignature, EthPublicInputError> {
        let r = self.read_quantity_u256("signature_r")?;
        let s = self.read_quantity_u256("signature_s")?;
        let y_parity = self.read_quantity_u64("signature_y_parity")?;
        validate_parity("signature_y_parity", y_parity)?;
        Ok(EthPublicSignature { y_parity, r, s })
    }

    fn read_legacy_transaction(
        &mut self,
        signature: &EthPublicSignature,
    ) -> Result<RlpItem, EthPublicInputError> {
        let chain_id = self.read_optional_quantity_u64("legacy_chain_id")?;
        let nonce = self.read_u64()?;
        let gas_price = self.read_u128()?;
        let gas_limit = self.read_u64()?;
        let to = self.read_transaction_kind("legacy_to")?;
        let value = self.read_quantity_u256("legacy_value")?;
        let input = self.read_bytes("legacy_input")?;
        let v = legacy_v(chain_id, signature.y_parity)?;
        Ok(RlpItem::List(vec![
            RlpItem::Bytes(u64_quantity_bytes(nonce)),
            RlpItem::Bytes(u128_quantity_bytes(gas_price)),
            RlpItem::Bytes(u64_quantity_bytes(gas_limit)),
            to,
            RlpItem::Bytes(value),
            RlpItem::Bytes(input),
            RlpItem::Bytes(u128_quantity_bytes(v)),
            RlpItem::Bytes(signature.r.clone()),
            RlpItem::Bytes(signature.s.clone()),
        ]))
    }

    fn read_eip2930_transaction(
        &mut self,
        signature: &EthPublicSignature,
    ) -> Result<RlpItem, EthPublicInputError> {
        let chain_id = self.read_u64()?;
        let nonce = self.read_u64()?;
        let gas_price = self.read_u128()?;
        let gas_limit = self.read_u64()?;
        let to = self.read_transaction_kind("eip2930_to")?;
        let value = self.read_quantity_u256("eip2930_value")?;
        let access_list = self.read_access_list()?;
        let input = self.read_bytes("eip2930_input")?;
        typed_transaction(
            1,
            vec![
                RlpItem::Bytes(u64_quantity_bytes(chain_id)),
                RlpItem::Bytes(u64_quantity_bytes(nonce)),
                RlpItem::Bytes(u128_quantity_bytes(gas_price)),
                RlpItem::Bytes(u64_quantity_bytes(gas_limit)),
                to,
                RlpItem::Bytes(value),
                RlpItem::Bytes(input),
                access_list,
            ],
            signature,
        )
    }

    fn read_eip1559_transaction(
        &mut self,
        signature: &EthPublicSignature,
    ) -> Result<RlpItem, EthPublicInputError> {
        let chain_id = self.read_u64()?;
        let nonce = self.read_u64()?;
        let gas_limit = self.read_u64()?;
        let max_fee_per_gas = self.read_u128()?;
        let max_priority_fee_per_gas = self.read_u128()?;
        let to = self.read_transaction_kind("eip1559_to")?;
        let value = self.read_quantity_u256("eip1559_value")?;
        let access_list = self.read_access_list()?;
        let input = self.read_bytes("eip1559_input")?;
        typed_transaction(
            2,
            vec![
                RlpItem::Bytes(u64_quantity_bytes(chain_id)),
                RlpItem::Bytes(u64_quantity_bytes(nonce)),
                RlpItem::Bytes(u128_quantity_bytes(max_priority_fee_per_gas)),
                RlpItem::Bytes(u128_quantity_bytes(max_fee_per_gas)),
                RlpItem::Bytes(u64_quantity_bytes(gas_limit)),
                to,
                RlpItem::Bytes(value),
                RlpItem::Bytes(input),
                access_list,
            ],
            signature,
        )
    }

    fn read_eip4844_transaction(
        &mut self,
        signature: &EthPublicSignature,
    ) -> Result<RlpItem, EthPublicInputError> {
        let chain_id = self.read_quantity_u64("eip4844_chain_id")?;
        let nonce = self.read_quantity_u64("eip4844_nonce")?;
        let gas_limit = self.read_quantity_u64("eip4844_gas_limit")?;
        let max_fee_per_gas = self.read_quantity_u128("eip4844_max_fee_per_gas")?;
        let max_priority_fee_per_gas =
            self.read_quantity_u128("eip4844_max_priority_fee_per_gas")?;
        let to = RlpItem::Bytes(self.read_address("eip4844_to")?.to_vec());
        let value = self.read_quantity_u256("eip4844_value")?;
        let access_list = self.read_access_list()?;
        let blob_versioned_hashes = self.read_fixed_bytes_list("eip4844_blob_versioned_hashes")?;
        let max_fee_per_blob_gas = self.read_quantity_u128("eip4844_max_fee_per_blob_gas")?;
        let input = self.read_bytes("eip4844_input")?;
        typed_transaction(
            3,
            vec![
                RlpItem::Bytes(u64_quantity_bytes(chain_id)),
                RlpItem::Bytes(u64_quantity_bytes(nonce)),
                RlpItem::Bytes(u128_quantity_bytes(max_priority_fee_per_gas)),
                RlpItem::Bytes(u128_quantity_bytes(max_fee_per_gas)),
                RlpItem::Bytes(u64_quantity_bytes(gas_limit)),
                to,
                RlpItem::Bytes(value),
                RlpItem::Bytes(input),
                access_list,
                RlpItem::Bytes(u128_quantity_bytes(max_fee_per_blob_gas)),
                blob_versioned_hashes,
            ],
            signature,
        )
    }

    fn read_eip7702_transaction(
        &mut self,
        signature: &EthPublicSignature,
    ) -> Result<RlpItem, EthPublicInputError> {
        let chain_id = self.read_u64()?;
        let nonce = self.read_u64()?;
        let gas_limit = self.read_u64()?;
        let max_fee_per_gas = self.read_u128()?;
        let max_priority_fee_per_gas = self.read_u128()?;
        let to = RlpItem::Bytes(self.read_address("eip7702_to")?.to_vec());
        let value = self.read_quantity_u256("eip7702_value")?;
        let access_list = self.read_access_list()?;
        let authorization_list = self.read_authorization_list()?;
        let input = self.read_bytes("eip7702_input")?;
        typed_transaction(
            4,
            vec![
                RlpItem::Bytes(u64_quantity_bytes(chain_id)),
                RlpItem::Bytes(u64_quantity_bytes(nonce)),
                RlpItem::Bytes(u128_quantity_bytes(max_priority_fee_per_gas)),
                RlpItem::Bytes(u128_quantity_bytes(max_fee_per_gas)),
                RlpItem::Bytes(u64_quantity_bytes(gas_limit)),
                to,
                RlpItem::Bytes(value),
                RlpItem::Bytes(input),
                access_list,
                authorization_list,
            ],
            signature,
        )
    }

    fn read_transaction_kind(
        &mut self,
        field: &'static str,
    ) -> Result<RlpItem, EthPublicInputError> {
        match self.read_u8()? {
            0 => Ok(RlpItem::Bytes(Vec::new())),
            1 => Ok(RlpItem::Bytes(self.read_address(field)?.to_vec())),
            found => Err(EthPublicInputError::InvalidOptionTag { field, found }),
        }
    }

    fn read_access_list(&mut self) -> Result<RlpItem, EthPublicInputError> {
        let count = self.read_len("access_list")?;
        let mut entries = Vec::with_capacity(count);
        for _ in 0..count {
            let address = RlpItem::Bytes(self.read_address("access_list_address")?.to_vec());
            let storage_key_count = self.read_len("access_list_storage_keys")?;
            let mut storage_keys = Vec::with_capacity(storage_key_count);
            for _ in 0..storage_key_count {
                storage_keys.push(RlpItem::Bytes(
                    self.read_fixed_bytes::<32>("access_list_storage_key")?
                        .to_vec(),
                ));
            }
            entries.push(RlpItem::List(vec![address, RlpItem::List(storage_keys)]));
        }
        Ok(RlpItem::List(entries))
    }

    fn read_fixed_bytes_list(
        &mut self,
        field: &'static str,
    ) -> Result<RlpItem, EthPublicInputError> {
        let count = self.read_len(field)?;
        let mut entries = Vec::with_capacity(count);
        for _ in 0..count {
            entries.push(RlpItem::Bytes(self.read_fixed_bytes::<32>(field)?.to_vec()));
        }
        Ok(RlpItem::List(entries))
    }

    fn read_authorization_list(&mut self) -> Result<RlpItem, EthPublicInputError> {
        let count = self.read_len("authorization_list")?;
        let mut entries = Vec::with_capacity(count);
        for _ in 0..count {
            let chain_id = self.read_quantity_u256("authorization_chain_id")?;
            let address = self.read_address("authorization_address")?.to_vec();
            let nonce = self.read_quantity_u64("authorization_nonce")?;
            let y_parity = self.read_quantity_u8("authorization_y_parity")?;
            let r = self.read_quantity_u256("authorization_r")?;
            let s = self.read_quantity_u256("authorization_s")?;
            validate_parity("authorization_y_parity", u64::from(y_parity))?;
            entries.push(RlpItem::List(vec![
                RlpItem::Bytes(chain_id),
                RlpItem::Bytes(address),
                RlpItem::Bytes(u64_quantity_bytes(nonce)),
                RlpItem::Bytes(u64_quantity_bytes(u64::from(y_parity))),
                RlpItem::Bytes(r),
                RlpItem::Bytes(s),
            ]));
        }
        Ok(RlpItem::List(entries))
    }

    fn read_address(&mut self, field: &'static str) -> Result<[u8; 20], EthPublicInputError> {
        self.read_fixed_bytes(field)
    }

    fn read_optional_quantity_u64(
        &mut self,
        field: &'static str,
    ) -> Result<Option<u64>, EthPublicInputError> {
        match self.read_u8()? {
            0 => Ok(None),
            1 => self.read_quantity_u64(field).map(Some),
            found => Err(EthPublicInputError::InvalidOptionTag { field, found }),
        }
    }

    fn read_quantity_u8(&mut self, field: &'static str) -> Result<u8, EthPublicInputError> {
        let bytes = self.read_quantity_bytes(field, 1)?;
        Ok(bytes.first().copied().unwrap_or(0))
    }

    fn read_quantity_u64(&mut self, field: &'static str) -> Result<u64, EthPublicInputError> {
        let bytes = self.read_quantity_bytes(field, 8)?;
        let mut value = 0_u64;
        for byte in bytes {
            value = value
                .checked_mul(256)
                .and_then(|value| value.checked_add(u64::from(byte)))
                .ok_or(EthPublicInputError::NumericOverflow { field })?;
        }
        Ok(value)
    }

    fn read_quantity_u128(&mut self, field: &'static str) -> Result<u128, EthPublicInputError> {
        let bytes = self.read_quantity_bytes(field, 16)?;
        let mut value = 0_u128;
        for byte in bytes {
            value = value
                .checked_mul(256)
                .and_then(|value| value.checked_add(u128::from(byte)))
                .ok_or(EthPublicInputError::NumericOverflow { field })?;
        }
        Ok(value)
    }

    fn read_quantity_u256(&mut self, field: &'static str) -> Result<Vec<u8>, EthPublicInputError> {
        self.read_quantity_bytes(field, 32)
    }

    fn read_quantity_bytes(
        &mut self,
        field: &'static str,
        expected: usize,
    ) -> Result<Vec<u8>, EthPublicInputError> {
        let bytes = self.read_bytes(field)?;
        if bytes.len() != expected {
            return Err(EthPublicInputError::InvalidFixedBytesLength {
                field,
                expected,
                found: bytes.len(),
            });
        }
        Ok(quantity_bytes(&bytes))
    }

    fn len_to_usize(&self, field: &'static str, len: u64) -> Result<usize, EthPublicInputError> {
        usize::try_from(len).map_err(|_| EthPublicInputError::LengthOverflow { field, value: len })
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

fn legacy_transaction_count(transactions: &[RlpItem]) -> usize {
    transactions
        .iter()
        .filter(|transaction| matches!(transaction, RlpItem::List(_)))
        .count()
}

fn typed_transaction_count(transactions: &[RlpItem]) -> usize {
    transactions.len() - legacy_transaction_count(transactions)
}

fn typed_transaction(
    transaction_type: u8,
    mut payload: Vec<RlpItem>,
    signature: &EthPublicSignature,
) -> Result<RlpItem, EthPublicInputError> {
    payload.push(RlpItem::Bytes(u64_quantity_bytes(signature.y_parity)));
    payload.push(RlpItem::Bytes(signature.r.clone()));
    payload.push(RlpItem::Bytes(signature.s.clone()));
    let mut encoded = vec![transaction_type];
    encoded.extend_from_slice(&encode_rlp(&RlpItem::List(payload)));
    Ok(RlpItem::Bytes(encoded))
}

fn legacy_v(chain_id: Option<u64>, y_parity: u64) -> Result<u128, EthPublicInputError> {
    validate_parity("legacy_signature_y_parity", y_parity)?;
    match chain_id {
        Some(chain_id) => u128::from(chain_id)
            .checked_mul(2)
            .and_then(|value| value.checked_add(35))
            .and_then(|value| value.checked_add(u128::from(y_parity)))
            .ok_or(EthPublicInputError::NumericOverflow { field: "legacy_v" }),
        None => 27_u128
            .checked_add(u128::from(y_parity))
            .ok_or(EthPublicInputError::NumericOverflow { field: "legacy_v" }),
    }
}

fn validate_parity(field: &'static str, value: u64) -> Result<(), EthPublicInputError> {
    if value <= 1 {
        return Ok(());
    }
    Err(EthPublicInputError::InvalidParity {
        field,
        found: value,
    })
}
