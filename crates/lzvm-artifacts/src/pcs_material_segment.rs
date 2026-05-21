use std::collections::BTreeSet;
use std::fmt;

use lzvm_field::{Felt, FieldError};

pub const PCS_MATERIAL_MANIFEST_SEGMENT_ID: u32 = 10_000;

const PCS_MATERIAL_MANIFEST_MAGIC: [u8; 4] = *b"pms0";
const PCS_MATERIAL_MANIFEST_VERSION: u32 = 1;
const HEADER_BYTES: usize = 4 + 4 + 4;
const DIGEST_BYTES: usize = 32;
const ROOT_WORDS: usize = 4;
const UNIT_BYTES: usize = 4 + DIGEST_BYTES * 3 + ROOT_WORDS * 8 + 4 * 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PcsMaterialManifestSegment {
    pub units: Vec<PcsMaterialManifestUnit>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PcsMaterialManifestUnit {
    pub unit_index: u32,
    pub plan_digest: [u8; 32],
    pub fixed_column_digest: [u8; 32],
    pub constant_tree_digest: [u8; 32],
    pub constant_tree_root: [u64; ROOT_WORDS],
    pub fixed_byte_count: u64,
    pub constant_tree_byte_count: u64,
    pub leaf_byte_count: u64,
    pub node_byte_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PcsMaterialManifestSegmentError {
    InvalidMagic,
    UnsupportedVersion {
        version: u32,
    },
    UnexpectedEof {
        needed: usize,
        available: usize,
    },
    TrailingBytes {
        trailing: usize,
    },
    EmptyUnits,
    DuplicateUnitIndex {
        unit_index: u32,
    },
    ConstantTreeRootNonCanonical {
        unit_index: u32,
        word_index: usize,
        source: FieldError,
    },
    LengthOverflow,
}

impl fmt::Display for PcsMaterialManifestSegmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMagic => write!(f, "invalid PCS material manifest segment magic"),
            Self::UnsupportedVersion { version } => {
                write!(
                    f,
                    "unsupported PCS material manifest segment version: {version}"
                )
            }
            Self::UnexpectedEof { needed, available } => write!(
                f,
                "truncated PCS material manifest segment: needed {needed}, available {available}"
            ),
            Self::TrailingBytes { trailing } => {
                write!(
                    f,
                    "trailing PCS material manifest segment bytes: {trailing}"
                )
            }
            Self::EmptyUnits => write!(f, "PCS material manifest segment has no units"),
            Self::DuplicateUnitIndex { unit_index } => write!(
                f,
                "duplicate PCS material manifest unit index: {unit_index}"
            ),
            Self::ConstantTreeRootNonCanonical {
                unit_index,
                word_index,
                source,
            } => write!(
                f,
                "PCS material manifest unit {unit_index} constant tree root word {word_index} is non-canonical: {source}"
            ),
            Self::LengthOverflow => write!(f, "PCS material manifest segment length overflow"),
        }
    }
}

impl std::error::Error for PcsMaterialManifestSegmentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ConstantTreeRootNonCanonical { source, .. } => Some(source),
            Self::InvalidMagic
            | Self::UnsupportedVersion { .. }
            | Self::UnexpectedEof { .. }
            | Self::TrailingBytes { .. }
            | Self::EmptyUnits
            | Self::DuplicateUnitIndex { .. }
            | Self::LengthOverflow => None,
        }
    }
}

pub fn encode_pcs_material_manifest_segment(
    value: &PcsMaterialManifestSegment,
) -> Result<Vec<u8>, PcsMaterialManifestSegmentError> {
    validate_pcs_material_manifest_segment(value)?;
    let unit_count = u32::try_from(value.units.len())
        .map_err(|_| PcsMaterialManifestSegmentError::LengthOverflow)?;
    let expected_len = value
        .units
        .len()
        .checked_mul(UNIT_BYTES)
        .and_then(|bytes| bytes.checked_add(HEADER_BYTES))
        .ok_or(PcsMaterialManifestSegmentError::LengthOverflow)?;

    let mut out = Vec::with_capacity(expected_len);
    out.extend_from_slice(&PCS_MATERIAL_MANIFEST_MAGIC);
    write_u32(&mut out, PCS_MATERIAL_MANIFEST_VERSION);
    write_u32(&mut out, unit_count);
    for unit in &value.units {
        write_u32(&mut out, unit.unit_index);
        out.extend_from_slice(&unit.plan_digest);
        out.extend_from_slice(&unit.fixed_column_digest);
        out.extend_from_slice(&unit.constant_tree_digest);
        for word in unit.constant_tree_root {
            write_u64(&mut out, word);
        }
        write_u64(&mut out, unit.fixed_byte_count);
        write_u64(&mut out, unit.constant_tree_byte_count);
        write_u64(&mut out, unit.leaf_byte_count);
        write_u64(&mut out, unit.node_byte_count);
    }
    Ok(out)
}

pub fn parse_pcs_material_manifest_segment(
    bytes: &[u8],
) -> Result<PcsMaterialManifestSegment, PcsMaterialManifestSegmentError> {
    let mut reader = SegmentReader::new(bytes);
    let magic = reader.read_array::<4>()?;
    if magic != PCS_MATERIAL_MANIFEST_MAGIC {
        return Err(PcsMaterialManifestSegmentError::InvalidMagic);
    }
    let version = reader.read_u32()?;
    if version != PCS_MATERIAL_MANIFEST_VERSION {
        return Err(PcsMaterialManifestSegmentError::UnsupportedVersion { version });
    }
    let unit_count = usize::try_from(reader.read_u32()?)
        .map_err(|_| PcsMaterialManifestSegmentError::LengthOverflow)?;
    if unit_count == 0 {
        return Err(PcsMaterialManifestSegmentError::EmptyUnits);
    }
    if unit_count > reader.remaining_len() / UNIT_BYTES {
        return Err(PcsMaterialManifestSegmentError::LengthOverflow);
    }

    let mut units = Vec::with_capacity(unit_count);
    for _ in 0..unit_count {
        let unit_index = reader.read_u32()?;
        let plan_digest = reader.read_array::<DIGEST_BYTES>()?;
        let fixed_column_digest = reader.read_array::<DIGEST_BYTES>()?;
        let constant_tree_digest = reader.read_array::<DIGEST_BYTES>()?;
        let mut constant_tree_root = [0_u64; ROOT_WORDS];
        for word in &mut constant_tree_root {
            *word = reader.read_u64()?;
        }
        units.push(PcsMaterialManifestUnit {
            unit_index,
            plan_digest,
            fixed_column_digest,
            constant_tree_digest,
            constant_tree_root,
            fixed_byte_count: reader.read_u64()?,
            constant_tree_byte_count: reader.read_u64()?,
            leaf_byte_count: reader.read_u64()?,
            node_byte_count: reader.read_u64()?,
        });
    }
    reader.finish()?;

    let out = PcsMaterialManifestSegment { units };
    validate_pcs_material_manifest_segment(&out)?;
    Ok(out)
}

fn validate_pcs_material_manifest_segment(
    value: &PcsMaterialManifestSegment,
) -> Result<(), PcsMaterialManifestSegmentError> {
    if value.units.is_empty() {
        return Err(PcsMaterialManifestSegmentError::EmptyUnits);
    }
    let mut seen = BTreeSet::new();
    for unit in &value.units {
        if !seen.insert(unit.unit_index) {
            return Err(PcsMaterialManifestSegmentError::DuplicateUnitIndex {
                unit_index: unit.unit_index,
            });
        }
        for (word_index, word) in unit.constant_tree_root.iter().copied().enumerate() {
            Felt::from_canonical(word).map_err(|source| {
                PcsMaterialManifestSegmentError::ConstantTreeRootNonCanonical {
                    unit_index: unit.unit_index,
                    word_index,
                    source,
                }
            })?;
        }
    }
    Ok(())
}

fn write_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

struct SegmentReader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> SegmentReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn read_u32(&mut self) -> Result<u32, PcsMaterialManifestSegmentError> {
        Ok(u32::from_le_bytes(self.read_array::<4>()?))
    }

    fn read_u64(&mut self) -> Result<u64, PcsMaterialManifestSegmentError> {
        Ok(u64::from_le_bytes(self.read_array::<8>()?))
    }

    fn remaining_len(&self) -> usize {
        self.bytes.len() - self.offset
    }

    fn read_array<const N: usize>(&mut self) -> Result<[u8; N], PcsMaterialManifestSegmentError> {
        let end = self
            .offset
            .checked_add(N)
            .ok_or(PcsMaterialManifestSegmentError::LengthOverflow)?;
        if end > self.bytes.len() {
            return Err(PcsMaterialManifestSegmentError::UnexpectedEof {
                needed: end,
                available: self.bytes.len(),
            });
        }
        let out = self.bytes[self.offset..end]
            .try_into()
            .expect("slice length checked");
        self.offset = end;
        Ok(out)
    }

    fn finish(&self) -> Result<(), PcsMaterialManifestSegmentError> {
        if self.offset == self.bytes.len() {
            Ok(())
        } else {
            Err(PcsMaterialManifestSegmentError::TrailingBytes {
                trailing: self.bytes.len() - self.offset,
            })
        }
    }
}
