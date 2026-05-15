use std::collections::BTreeSet;
use std::fmt;

pub const PCS_FRI_OPENING_SEGMENT_ID: u32 = 10_004;

const PCS_FRI_OPENING_MAGIC: [u8; 4] = *b"fos0";
const PCS_FRI_OPENING_VERSION: u32 = 1;
const HEADER_BYTES: usize = 4 + 4 + 4;
const UNIT_HEADER_BYTES: usize = 4 + 4 + 4;
const LAYER_HEADER_BYTES: usize = 4 + ROOT_WORDS * WORD_BYTES + 4 + 4;
const QUERY_HEADER_BYTES: usize = 8 + 4 + 4;
const LEVEL_HEADER_BYTES: usize = 4;
const WORD_BYTES: usize = 8;
const ROOT_WORDS: usize = 4;
const EXTENSION_WORDS: usize = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PcsFriOpeningSegment {
    pub units: Vec<PcsFriOpeningUnitSegment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PcsFriOpeningUnitSegment {
    pub unit_index: u32,
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
    DuplicateLayerIndex {
        unit_index: u32,
        layer_index: u32,
    },
    DuplicateQueryRow {
        unit_index: u32,
        layer_index: u32,
        row_index: u64,
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
            Self::DuplicateLayerIndex {
                unit_index,
                layer_index,
            } => write!(
                f,
                "duplicate PCS FRI opening layer index: unit {unit_index}, layer {layer_index}"
            ),
            Self::DuplicateQueryRow {
                unit_index,
                layer_index,
                row_index,
            } => write!(
                f,
                "duplicate PCS FRI opening query row: unit {unit_index}, layer {layer_index}, row {row_index}"
            ),
            Self::LengthOverflow => write!(f, "PCS FRI opening segment length overflow"),
        }
    }
}

impl std::error::Error for PcsFriOpeningSegmentError {}

pub fn encode_pcs_fri_opening_segment(
    value: &PcsFriOpeningSegment,
) -> Result<Vec<u8>, PcsFriOpeningSegmentError> {
    validate_pcs_fri_opening_segment(value)?;
    let unit_count =
        u32::try_from(value.units.len()).map_err(|_| PcsFriOpeningSegmentError::LengthOverflow)?;
    let expected_len = encoded_len(value)?;

    let mut out = Vec::with_capacity(expected_len);
    out.extend_from_slice(&PCS_FRI_OPENING_MAGIC);
    write_u32(&mut out, PCS_FRI_OPENING_VERSION);
    write_u32(&mut out, unit_count);
    for unit in &value.units {
        write_u32(&mut out, unit.unit_index);
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
    if version != PCS_FRI_OPENING_VERSION {
        return Err(PcsFriOpeningSegmentError::UnsupportedVersion { version });
    }
    let unit_count = reader.read_u32()? as usize;
    if unit_count == 0 {
        return Err(PcsFriOpeningSegmentError::EmptyUnits);
    }

    let mut units = Vec::with_capacity(unit_count);
    for _ in 0..unit_count {
        let unit_index = reader.read_u32()?;
        let layer_count = reader.read_u32()? as usize;
        let final_count = reader.read_u32()? as usize;
        let mut final_polynomial = Vec::with_capacity(final_count);
        for _ in 0..final_count {
            final_polynomial.push(reader.read_extension()?);
        }
        let mut layers = Vec::with_capacity(layer_count);
        for _ in 0..layer_count {
            let layer_index = reader.read_u32()?;
            let root = reader.read_digest()?;
            let last_level_count = reader.read_u32()? as usize;
            let query_count = reader.read_u32()? as usize;
            let mut last_level = Vec::with_capacity(last_level_count);
            for _ in 0..last_level_count {
                last_level.push(reader.read_digest()?);
            }
            let mut queries = Vec::with_capacity(query_count);
            for _ in 0..query_count {
                let row_index = reader.read_u64()?;
                let value_count = reader.read_u32()? as usize;
                let level_count = reader.read_u32()? as usize;
                let mut values = Vec::with_capacity(value_count);
                for _ in 0..value_count {
                    values.push(reader.read_extension()?);
                }
                let mut siblings = Vec::with_capacity(level_count);
                for _ in 0..level_count {
                    let sibling_count = reader.read_u32()? as usize;
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
            layers,
            final_polynomial,
        });
    }
    reader.finish()?;

    let out = PcsFriOpeningSegment { units };
    validate_pcs_fri_opening_segment(&out)?;
    Ok(out)
}

fn validate_pcs_fri_opening_segment(
    value: &PcsFriOpeningSegment,
) -> Result<(), PcsFriOpeningSegmentError> {
    if value.units.is_empty() {
        return Err(PcsFriOpeningSegmentError::EmptyUnits);
    }
    let mut seen_units = BTreeSet::new();
    for unit in &value.units {
        if !seen_units.insert(unit.unit_index) {
            return Err(PcsFriOpeningSegmentError::DuplicateUnitIndex {
                unit_index: unit.unit_index,
            });
        }
        if unit.final_polynomial.is_empty() {
            return Err(PcsFriOpeningSegmentError::EmptyFinalPolynomial {
                unit_index: unit.unit_index,
            });
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
            let mut seen_rows = BTreeSet::new();
            for query in &layer.queries {
                if query.values.is_empty() {
                    return Err(PcsFriOpeningSegmentError::EmptyQueryValues {
                        unit_index: unit.unit_index,
                        layer_index: layer.layer_index,
                        row_index: query.row_index,
                    });
                }
                if !seen_rows.insert(query.row_index) {
                    return Err(PcsFriOpeningSegmentError::DuplicateQueryRow {
                        unit_index: unit.unit_index,
                        layer_index: layer.layer_index,
                        row_index: query.row_index,
                    });
                }
            }
        }
    }
    Ok(())
}

fn encoded_len(value: &PcsFriOpeningSegment) -> Result<usize, PcsFriOpeningSegmentError> {
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
        acc.checked_add(UNIT_HEADER_BYTES)
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
        let out = self.bytes[self.offset..end]
            .try_into()
            .expect("slice length checked");
        self.offset = end;
        Ok(out)
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
