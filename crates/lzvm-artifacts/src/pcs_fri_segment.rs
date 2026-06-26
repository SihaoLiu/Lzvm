use std::collections::BTreeSet;
use std::fmt;

use lzvm_field::{Felt, FieldError};

pub const PCS_FRI_OPENING_SEGMENT_ID: u32 = 10_004;

const PCS_FRI_OPENING_MAGIC: [u8; 4] = *b"fos0";
const PCS_FRI_OPENING_V1_VERSION: u32 = 1;
const PCS_FRI_OPENING_V2_VERSION: u32 = 2;
const HEADER_BYTES: usize = 4 + 4 + 4;
const V1_UNIT_HEADER_BYTES: usize = 4 + 4 + 4;
const V2_UNIT_HEADER_BYTES: usize = 4 + 4 + 4 + 4;
const LAYER_HEADER_BYTES: usize = 4 + ROOT_WORDS * WORD_BYTES + 4 + 4;
const QUERY_HEADER_BYTES: usize = 8 + 4 + 4;
const LEVEL_HEADER_BYTES: usize = 4;
const WORD_BYTES: usize = 8;
const ROOT_WORDS: usize = 4;
const EXTENSION_WORDS: usize = 3;
const ROOT_BYTES: usize = ROOT_WORDS * WORD_BYTES;
const EXTENSION_BYTES: usize = EXTENSION_WORDS * WORD_BYTES;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PcsFriOpeningSegment {
    pub units: Vec<PcsFriOpeningUnitSegment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PcsFriOpeningUnitSegment {
    pub unit_index: u32,
    pub trace_instance_index: u32,
    pub layers: Vec<PcsFriOpeningLayerSegment>,
    pub final_polynomial: Vec<[u64; EXTENSION_WORDS]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PcsFriOpeningLayerSegment {
    pub layer_index: u32,
    pub root: [u64; ROOT_WORDS],
    pub last_level: Vec<[u64; ROOT_WORDS]>,
    pub queries: Vec<PcsFriOpeningQuerySegment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PcsFriOpeningQuerySegment {
    pub row_index: u64,
    pub values: Vec<[u64; EXTENSION_WORDS]>,
    pub siblings: Vec<PcsFriOpeningLevelSegment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PcsFriOpeningLevelSegment {
    pub siblings: Vec<[u64; ROOT_WORDS]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PcsFriOpeningSegmentError {
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
    EmptyFinalPolynomial {
        unit_index: u32,
    },
    EmptyLayerQueries {
        unit_index: u32,
        layer_index: u32,
    },
    EmptyQueryValues {
        unit_index: u32,
        layer_index: u32,
        row_index: u64,
    },
    DuplicateUnitIndex {
        unit_index: u32,
    },
    DuplicateUnitIdentity {
        unit_index: u32,
        trace_instance_index: u32,
    },
    DuplicateLayerIndex {
        unit_index: u32,
        layer_index: u32,
    },
    FinalPolynomialValueNonCanonical {
        unit_index: u32,
        value_index: usize,
        word_index: usize,
        source: FieldError,
    },
    LayerRootNonCanonical {
        unit_index: u32,
        layer_index: u32,
        word_index: usize,
        source: FieldError,
    },
    LastLevelRootNonCanonical {
        unit_index: u32,
        layer_index: u32,
        root_index: usize,
        word_index: usize,
        source: FieldError,
    },
    QueryValueNonCanonical {
        unit_index: u32,
        layer_index: u32,
        row_index: u64,
        value_index: usize,
        word_index: usize,
        source: FieldError,
    },
    SiblingRootNonCanonical {
        unit_index: u32,
        layer_index: u32,
        row_index: u64,
        level_index: usize,
        root_index: usize,
        word_index: usize,
        source: FieldError,
    },
    LengthOverflow,
}

impl fmt::Display for PcsFriOpeningSegmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMagic => write!(f, "invalid PCS FRI opening segment magic"),
            Self::UnsupportedVersion { version } => {
                write!(
                    f,
                    "unsupported PCS FRI opening segment version: {version}"
                )
            }
            Self::UnexpectedEof { needed, available } => write!(
                f,
                "truncated PCS FRI opening segment: needed {needed}, available {available}"
            ),
            Self::TrailingBytes { trailing } => {
                write!(f, "trailing PCS FRI opening segment bytes: {trailing}")
            }
            Self::EmptyUnits => write!(f, "PCS FRI opening segment has no units"),
            Self::EmptyFinalPolynomial { unit_index } => write!(
                f,
                "PCS FRI opening unit {unit_index} has no final polynomial"
            ),
            Self::EmptyLayerQueries {
                unit_index,
                layer_index,
            } => write!(
                f,
                "PCS FRI opening unit {unit_index} layer {layer_index} has no queries"
            ),
            Self::EmptyQueryValues {
                unit_index,
                layer_index,
                row_index,
            } => write!(
                f,
                "PCS FRI opening unit {unit_index} layer {layer_index} row {row_index} has no values"
            ),
            Self::DuplicateUnitIndex { unit_index } => {
                write!(f, "duplicate PCS FRI opening unit index: {unit_index}")
            }
            Self::DuplicateUnitIdentity {
                unit_index,
                trace_instance_index,
            } => write!(
                f,
                "duplicate PCS FRI opening unit identity: unit {unit_index}, trace instance {trace_instance_index}"
            ),
            Self::DuplicateLayerIndex {
                unit_index,
                layer_index,
            } => write!(
                f,
                "duplicate PCS FRI opening layer index: unit {unit_index}, layer {layer_index}"
            ),
            Self::FinalPolynomialValueNonCanonical {
                unit_index,
                value_index,
                word_index,
                source,
            } => write!(
                f,
                "PCS FRI opening unit {unit_index} final polynomial value {value_index} word {word_index} is non-canonical: {source}"
            ),
            Self::LayerRootNonCanonical {
                unit_index,
                layer_index,
                word_index,
                source,
            } => write!(
                f,
                "PCS FRI opening unit {unit_index} layer {layer_index} root word {word_index} is non-canonical: {source}"
            ),
            Self::LastLevelRootNonCanonical {
                unit_index,
                layer_index,
                root_index,
                word_index,
                source,
            } => write!(
                f,
                "PCS FRI opening unit {unit_index} layer {layer_index} last level root {root_index} word {word_index} is non-canonical: {source}"
            ),
            Self::QueryValueNonCanonical {
                unit_index,
                layer_index,
                row_index,
                value_index,
                word_index,
                source,
            } => write!(
                f,
                "PCS FRI opening unit {unit_index} layer {layer_index} row {row_index} query value {value_index} word {word_index} is non-canonical: {source}"
            ),
            Self::SiblingRootNonCanonical {
                unit_index,
                layer_index,
                row_index,
                level_index,
                root_index,
                word_index,
                source,
            } => write!(
                f,
                "PCS FRI opening unit {unit_index} layer {layer_index} row {row_index} sibling level {level_index} root {root_index} word {word_index} is non-canonical: {source}"
            ),
            Self::LengthOverflow => write!(f, "PCS FRI opening segment length overflow"),
        }
    }
}

impl std::error::Error for PcsFriOpeningSegmentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::FinalPolynomialValueNonCanonical { source, .. }
            | Self::LayerRootNonCanonical { source, .. }
            | Self::LastLevelRootNonCanonical { source, .. }
            | Self::QueryValueNonCanonical { source, .. }
            | Self::SiblingRootNonCanonical { source, .. } => Some(source),
            Self::InvalidMagic
            | Self::UnsupportedVersion { .. }
            | Self::UnexpectedEof { .. }
            | Self::TrailingBytes { .. }
            | Self::EmptyUnits
            | Self::EmptyFinalPolynomial { .. }
            | Self::EmptyLayerQueries { .. }
            | Self::EmptyQueryValues { .. }
            | Self::DuplicateUnitIndex { .. }
            | Self::DuplicateUnitIdentity { .. }
            | Self::DuplicateLayerIndex { .. }
            | Self::LengthOverflow => None,
        }
    }
}

pub fn encode_pcs_fri_opening_segment(
    value: &PcsFriOpeningSegment,
) -> Result<Vec<u8>, PcsFriOpeningSegmentError> {
    validate_pcs_fri_opening_segment(value)?;
    let unit_count =
        u32::try_from(value.units.len()).map_err(|_| PcsFriOpeningSegmentError::LengthOverflow)?;
    let version = pcs_fri_opening_version(value);
    let expected_len = encoded_len(value, unit_header_bytes(version)?)?;

    let mut out = Vec::with_capacity(expected_len);
    out.extend_from_slice(&PCS_FRI_OPENING_MAGIC);
    write_u32(&mut out, version);
    write_u32(&mut out, unit_count);
    for unit in &value.units {
        write_u32(&mut out, unit.unit_index);
        if version == PCS_FRI_OPENING_V2_VERSION {
            write_u32(&mut out, unit.trace_instance_index);
        }
        write_u32(
            &mut out,
            u32::try_from(unit.layers.len())
                .map_err(|_| PcsFriOpeningSegmentError::LengthOverflow)?,
        );
        write_u32(
            &mut out,
            u32::try_from(unit.final_polynomial.len())
                .map_err(|_| PcsFriOpeningSegmentError::LengthOverflow)?,
        );
        for value in &unit.final_polynomial {
            write_extension(&mut out, *value);
        }
        for layer in &unit.layers {
            write_u32(&mut out, layer.layer_index);
            write_digest(&mut out, layer.root);
            write_u32(
                &mut out,
                u32::try_from(layer.last_level.len())
                    .map_err(|_| PcsFriOpeningSegmentError::LengthOverflow)?,
            );
            write_u32(
                &mut out,
                u32::try_from(layer.queries.len())
                    .map_err(|_| PcsFriOpeningSegmentError::LengthOverflow)?,
            );
            for digest in &layer.last_level {
                write_digest(&mut out, *digest);
            }
            for query in &layer.queries {
                write_u64(&mut out, query.row_index);
                write_u32(
                    &mut out,
                    u32::try_from(query.values.len())
                        .map_err(|_| PcsFriOpeningSegmentError::LengthOverflow)?,
                );
                write_u32(
                    &mut out,
                    u32::try_from(query.siblings.len())
                        .map_err(|_| PcsFriOpeningSegmentError::LengthOverflow)?,
                );
                for value in &query.values {
                    write_extension(&mut out, *value);
                }
                for level in &query.siblings {
                    write_u32(
                        &mut out,
                        u32::try_from(level.siblings.len())
                            .map_err(|_| PcsFriOpeningSegmentError::LengthOverflow)?,
                    );
                    for digest in &level.siblings {
                        write_digest(&mut out, *digest);
                    }
                }
            }
        }
    }
    Ok(out)
}

pub fn parse_pcs_fri_opening_segment(
    bytes: &[u8],
) -> Result<PcsFriOpeningSegment, PcsFriOpeningSegmentError> {
    let mut reader = SegmentReader::new(bytes);
    let magic = reader.read_array::<4>()?;
    if magic != PCS_FRI_OPENING_MAGIC {
        return Err(PcsFriOpeningSegmentError::InvalidMagic);
    }
    let version = reader.read_u32()?;
    if !matches!(
        version,
        PCS_FRI_OPENING_V1_VERSION | PCS_FRI_OPENING_V2_VERSION
    ) {
        return Err(PcsFriOpeningSegmentError::UnsupportedVersion { version });
    }
    let unit_count = usize::try_from(reader.read_u32()?)
        .map_err(|_| PcsFriOpeningSegmentError::LengthOverflow)?;
    if unit_count == 0 {
        return Err(PcsFriOpeningSegmentError::EmptyUnits);
    }
    let unit_header_bytes = unit_header_bytes(version)?;
    reader.require_items(unit_count, unit_header_bytes)?;

    let mut units = Vec::with_capacity(unit_count);
    for _ in 0..unit_count {
        let unit_index = reader.read_u32()?;
        let trace_instance_index = if version == PCS_FRI_OPENING_V2_VERSION {
            reader.read_u32()?
        } else {
            0
        };
        let layer_count = usize::try_from(reader.read_u32()?)
            .map_err(|_| PcsFriOpeningSegmentError::LengthOverflow)?;
        let final_count = usize::try_from(reader.read_u32()?)
            .map_err(|_| PcsFriOpeningSegmentError::LengthOverflow)?;
        reader.require_items(final_count, EXTENSION_BYTES)?;
        let mut final_polynomial = Vec::with_capacity(final_count);
        for _ in 0..final_count {
            final_polynomial.push(reader.read_extension()?);
        }
        reader.require_items(layer_count, LAYER_HEADER_BYTES)?;
        let mut layers = Vec::with_capacity(layer_count);
        for _ in 0..layer_count {
            let layer_index = reader.read_u32()?;
            let root = reader.read_digest()?;
            let last_level_count = usize::try_from(reader.read_u32()?)
                .map_err(|_| PcsFriOpeningSegmentError::LengthOverflow)?;
            let query_count = usize::try_from(reader.read_u32()?)
                .map_err(|_| PcsFriOpeningSegmentError::LengthOverflow)?;
            reader.require_items(last_level_count, ROOT_BYTES)?;
            let mut last_level = Vec::with_capacity(last_level_count);
            for _ in 0..last_level_count {
                last_level.push(reader.read_digest()?);
            }
            reader.require_items(query_count, QUERY_HEADER_BYTES)?;
            let mut queries = Vec::with_capacity(query_count);
            for _ in 0..query_count {
                let row_index = reader.read_u64()?;
                let value_count = usize::try_from(reader.read_u32()?)
                    .map_err(|_| PcsFriOpeningSegmentError::LengthOverflow)?;
                let level_count = usize::try_from(reader.read_u32()?)
                    .map_err(|_| PcsFriOpeningSegmentError::LengthOverflow)?;
                reader.require_items(value_count, EXTENSION_BYTES)?;
                let mut values = Vec::with_capacity(value_count);
                for _ in 0..value_count {
                    values.push(reader.read_extension()?);
                }
                reader.require_items(level_count, LEVEL_HEADER_BYTES)?;
                let mut siblings = Vec::with_capacity(level_count);
                for _ in 0..level_count {
                    let sibling_count = usize::try_from(reader.read_u32()?)
                        .map_err(|_| PcsFriOpeningSegmentError::LengthOverflow)?;
                    reader.require_items(sibling_count, ROOT_BYTES)?;
                    let mut level = Vec::with_capacity(sibling_count);
                    for _ in 0..sibling_count {
                        level.push(reader.read_digest()?);
                    }
                    siblings.push(PcsFriOpeningLevelSegment { siblings: level });
                }
                queries.push(PcsFriOpeningQuerySegment {
                    row_index,
                    values,
                    siblings,
                });
            }
            layers.push(PcsFriOpeningLayerSegment {
                layer_index,
                root,
                last_level,
                queries,
            });
        }
        units.push(PcsFriOpeningUnitSegment {
            unit_index,
            trace_instance_index,
            layers,
            final_polynomial,
        });
    }
    reader.finish()?;

    let out = PcsFriOpeningSegment { units };
    validate_pcs_fri_opening_segment(&out)?;
    Ok(out)
}

fn pcs_fri_opening_version(value: &PcsFriOpeningSegment) -> u32 {
    if value
        .units
        .iter()
        .any(|unit| unit.trace_instance_index != 0)
    {
        PCS_FRI_OPENING_V2_VERSION
    } else {
        PCS_FRI_OPENING_V1_VERSION
    }
}

fn unit_header_bytes(version: u32) -> Result<usize, PcsFriOpeningSegmentError> {
    match version {
        PCS_FRI_OPENING_V1_VERSION => Ok(V1_UNIT_HEADER_BYTES),
        PCS_FRI_OPENING_V2_VERSION => Ok(V2_UNIT_HEADER_BYTES),
        _ => Err(PcsFriOpeningSegmentError::UnsupportedVersion { version }),
    }
}

fn validate_pcs_fri_opening_segment(
    value: &PcsFriOpeningSegment,
) -> Result<(), PcsFriOpeningSegmentError> {
    if value.units.is_empty() {
        return Err(PcsFriOpeningSegmentError::EmptyUnits);
    }
    let mut seen_units = BTreeSet::new();
    for unit in &value.units {
        if !seen_units.insert((unit.unit_index, unit.trace_instance_index)) {
            if unit.trace_instance_index == 0 {
                return Err(PcsFriOpeningSegmentError::DuplicateUnitIndex {
                    unit_index: unit.unit_index,
                });
            }
            return Err(PcsFriOpeningSegmentError::DuplicateUnitIdentity {
                unit_index: unit.unit_index,
                trace_instance_index: unit.trace_instance_index,
            });
        }
        if unit.final_polynomial.is_empty() {
            return Err(PcsFriOpeningSegmentError::EmptyFinalPolynomial {
                unit_index: unit.unit_index,
            });
        }
        for (value_index, value) in unit.final_polynomial.iter().enumerate() {
            for (word_index, word) in value.iter().copied().enumerate() {
                Felt::from_canonical(word).map_err(|source| {
                    PcsFriOpeningSegmentError::FinalPolynomialValueNonCanonical {
                        unit_index: unit.unit_index,
                        value_index,
                        word_index,
                        source,
                    }
                })?;
            }
        }
        let mut seen_layers = BTreeSet::new();
        for layer in &unit.layers {
            if !seen_layers.insert(layer.layer_index) {
                return Err(PcsFriOpeningSegmentError::DuplicateLayerIndex {
                    unit_index: unit.unit_index,
                    layer_index: layer.layer_index,
                });
            }
            if layer.queries.is_empty() {
                return Err(PcsFriOpeningSegmentError::EmptyLayerQueries {
                    unit_index: unit.unit_index,
                    layer_index: layer.layer_index,
                });
            }
            for (word_index, word) in layer.root.iter().copied().enumerate() {
                Felt::from_canonical(word).map_err(|source| {
                    PcsFriOpeningSegmentError::LayerRootNonCanonical {
                        unit_index: unit.unit_index,
                        layer_index: layer.layer_index,
                        word_index,
                        source,
                    }
                })?;
            }
            for (root_index, root) in layer.last_level.iter().enumerate() {
                for (word_index, word) in root.iter().copied().enumerate() {
                    Felt::from_canonical(word).map_err(|source| {
                        PcsFriOpeningSegmentError::LastLevelRootNonCanonical {
                            unit_index: unit.unit_index,
                            layer_index: layer.layer_index,
                            root_index,
                            word_index,
                            source,
                        }
                    })?;
                }
            }
            for query in &layer.queries {
                if query.values.is_empty() {
                    return Err(PcsFriOpeningSegmentError::EmptyQueryValues {
                        unit_index: unit.unit_index,
                        layer_index: layer.layer_index,
                        row_index: query.row_index,
                    });
                }
                for (value_index, value) in query.values.iter().enumerate() {
                    for (word_index, word) in value.iter().copied().enumerate() {
                        Felt::from_canonical(word).map_err(|source| {
                            PcsFriOpeningSegmentError::QueryValueNonCanonical {
                                unit_index: unit.unit_index,
                                layer_index: layer.layer_index,
                                row_index: query.row_index,
                                value_index,
                                word_index,
                                source,
                            }
                        })?;
                    }
                }
                for (level_index, level) in query.siblings.iter().enumerate() {
                    for (root_index, root) in level.siblings.iter().enumerate() {
                        for (word_index, word) in root.iter().copied().enumerate() {
                            Felt::from_canonical(word).map_err(|source| {
                                PcsFriOpeningSegmentError::SiblingRootNonCanonical {
                                    unit_index: unit.unit_index,
                                    layer_index: layer.layer_index,
                                    row_index: query.row_index,
                                    level_index,
                                    root_index,
                                    word_index,
                                    source,
                                }
                            })?;
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn encoded_len(
    value: &PcsFriOpeningSegment,
    unit_header_bytes: usize,
) -> Result<usize, PcsFriOpeningSegmentError> {
    value.units.iter().try_fold(HEADER_BYTES, |acc, unit| {
        let final_bytes = unit
            .final_polynomial
            .len()
            .checked_mul(EXTENSION_WORDS * WORD_BYTES)
            .ok_or(PcsFriOpeningSegmentError::LengthOverflow)?;
        let layer_bytes = unit.layers.iter().try_fold(0_usize, |layer_acc, layer| {
            let last_level_bytes = layer
                .last_level
                .len()
                .checked_mul(ROOT_WORDS * WORD_BYTES)
                .ok_or(PcsFriOpeningSegmentError::LengthOverflow)?;
            let query_bytes = layer.queries.iter().try_fold(0_usize, |query_acc, query| {
                let value_bytes = query
                    .values
                    .len()
                    .checked_mul(EXTENSION_WORDS * WORD_BYTES)
                    .ok_or(PcsFriOpeningSegmentError::LengthOverflow)?;
                let sibling_bytes =
                    query
                        .siblings
                        .iter()
                        .try_fold(0_usize, |level_acc, level| {
                            level
                                .siblings
                                .len()
                                .checked_mul(ROOT_WORDS)
                                .and_then(|words| words.checked_mul(WORD_BYTES))
                                .and_then(|bytes| bytes.checked_add(LEVEL_HEADER_BYTES))
                                .and_then(|bytes| bytes.checked_add(level_acc))
                                .ok_or(PcsFriOpeningSegmentError::LengthOverflow)
                        })?;
                query_acc
                    .checked_add(QUERY_HEADER_BYTES)
                    .and_then(|bytes| bytes.checked_add(value_bytes))
                    .and_then(|bytes| bytes.checked_add(sibling_bytes))
                    .ok_or(PcsFriOpeningSegmentError::LengthOverflow)
            })?;
            layer_acc
                .checked_add(LAYER_HEADER_BYTES)
                .and_then(|bytes| bytes.checked_add(last_level_bytes))
                .and_then(|bytes| bytes.checked_add(query_bytes))
                .ok_or(PcsFriOpeningSegmentError::LengthOverflow)
        })?;
        acc.checked_add(unit_header_bytes)
            .and_then(|bytes| bytes.checked_add(final_bytes))
            .and_then(|bytes| bytes.checked_add(layer_bytes))
            .ok_or(PcsFriOpeningSegmentError::LengthOverflow)
    })
}

fn write_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_digest(out: &mut Vec<u8>, value: [u64; ROOT_WORDS]) {
    for word in value {
        write_u64(out, word);
    }
}

fn write_extension(out: &mut Vec<u8>, value: [u64; EXTENSION_WORDS]) {
    for word in value {
        write_u64(out, word);
    }
}

struct SegmentReader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> SegmentReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn read_u32(&mut self) -> Result<u32, PcsFriOpeningSegmentError> {
        Ok(u32::from_le_bytes(self.read_array::<4>()?))
    }

    fn read_u64(&mut self) -> Result<u64, PcsFriOpeningSegmentError> {
        Ok(u64::from_le_bytes(self.read_array::<8>()?))
    }

    fn read_digest(&mut self) -> Result<[u64; ROOT_WORDS], PcsFriOpeningSegmentError> {
        let mut out = [0_u64; ROOT_WORDS];
        for word in &mut out {
            *word = self.read_u64()?;
        }
        Ok(out)
    }

    fn read_extension(&mut self) -> Result<[u64; EXTENSION_WORDS], PcsFriOpeningSegmentError> {
        let mut out = [0_u64; EXTENSION_WORDS];
        for word in &mut out {
            *word = self.read_u64()?;
        }
        Ok(out)
    }

    fn read_array<const N: usize>(&mut self) -> Result<[u8; N], PcsFriOpeningSegmentError> {
        let end = self
            .offset
            .checked_add(N)
            .ok_or(PcsFriOpeningSegmentError::LengthOverflow)?;
        if end > self.bytes.len() {
            return Err(PcsFriOpeningSegmentError::UnexpectedEof {
                needed: end,
                available: self.bytes.len(),
            });
        }
        let mut out = [0_u8; N];
        out.copy_from_slice(&self.bytes[self.offset..end]);
        self.offset = end;
        Ok(out)
    }

    fn require_items(
        &self,
        count: usize,
        item_bytes: usize,
    ) -> Result<(), PcsFriOpeningSegmentError> {
        let needed_bytes = count
            .checked_mul(item_bytes)
            .ok_or(PcsFriOpeningSegmentError::LengthOverflow)?;
        let needed = self
            .offset
            .checked_add(needed_bytes)
            .ok_or(PcsFriOpeningSegmentError::LengthOverflow)?;
        if needed > self.bytes.len() {
            return Err(PcsFriOpeningSegmentError::UnexpectedEof {
                needed,
                available: self.bytes.len(),
            });
        }
        Ok(())
    }

    fn finish(&self) -> Result<(), PcsFriOpeningSegmentError> {
        if self.offset == self.bytes.len() {
            Ok(())
        } else {
            Err(PcsFriOpeningSegmentError::TrailingBytes {
                trailing: self.bytes.len() - self.offset,
            })
        }
    }
}
