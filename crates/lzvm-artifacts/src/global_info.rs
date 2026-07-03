use std::collections::BTreeSet;
use std::fmt;
use std::path::Path;

use crate::sectioned::{
    encode_sectioned_file, parse_sectioned_file, SectionedError, SectionedFile, SectionedSection,
};
use crate::transcript_parameters::is_supported_transcript_arity_u64;

const GLOBAL_INFO_KIND: [u8; 4] = *b"ginf";
const GLOBAL_INFO_VERSION: u32 = 1;
const GLOBAL_INFO_SECTION_ID: u32 = 1;

const U32_BYTES: usize = 4;
const U64_BYTES: usize = 8;
const FLAG_BYTES: usize = 1;
const STRING_MIN_BYTES: usize = 1;
const AIR_GROUP_MIN_BYTES: usize = STRING_MIN_BYTES;
const AIR_SECTION_MIN_BYTES: usize = U32_BYTES;
const AIR_UNIT_MIN_BYTES: usize = STRING_MIN_BYTES + U64_BYTES + FLAG_BYTES;
const AGGREGATION_GROUP_MIN_BYTES: usize = U32_BYTES;
const AGGREGATION_ENTRY_BYTES: usize = U64_BYTES;
const NAMED_STAGE_VALUE_MIN_BYTES: usize = STRING_MIN_BYTES + U64_BYTES + FLAG_BYTES + U32_BYTES;
const PUBLIC_VALUE_MIN_BYTES: usize = STRING_MIN_BYTES + U64_BYTES + U32_BYTES;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobalInfo {
    pub name: String,
    pub air_groups: Vec<String>,
    pub airs: Vec<Vec<GlobalAir>>,
    pub curve: CurveKind,
    pub lattice_size: Option<u64>,
    pub aggregation_types: Vec<Vec<AggregationType>>,
    pub n_publics: u64,
    pub num_challenges: Vec<u64>,
    pub num_proof_values: Vec<u64>,
    pub proof_values_map: Vec<NamedStageValue>,
    pub publics_map: Vec<PublicValue>,
    pub transcript_arity: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobalAir {
    pub name: String,
    pub num_rows: u64,
    pub has_compressor: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AggregationType {
    pub aggregation_type: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedStageValue {
    pub name: String,
    pub stage: u64,
    pub id: Option<u64>,
    pub lengths: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicValue {
    pub name: String,
    pub stage: u64,
    pub lengths: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CurveKind {
    None,
    EcGfp5,
    EcMasFp5,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GlobalInfoError {
    InvalidMagic,
    UnsupportedVersion {
        found: u32,
        max: u32,
    },
    InvalidSectionCount {
        found: u32,
    },
    InvalidSectionId {
        found: u32,
    },
    UnexpectedTrailingBytes {
        count: usize,
    },
    UnexpectedEof {
        offset: usize,
        needed: usize,
        available: usize,
    },
    InvalidUtf8,
    MissingStringTerminator {
        offset: usize,
    },
    LengthOverflow,
    StringContainsNul {
        value: String,
    },
    InvalidFlag {
        field: &'static str,
        value: u8,
    },
    AirGroupCountMismatch {
        air_groups: usize,
        airs: usize,
        aggregation_types: usize,
    },
    EmptyAirGroup {
        airgroup_id: usize,
    },
    InvalidRowCount {
        airgroup_id: usize,
        air_id: usize,
    },
    InvalidStage {
        field: &'static str,
        index: usize,
    },
    InvalidLength {
        field: &'static str,
        index: usize,
    },
    DuplicateValueName {
        field: &'static str,
        name: String,
    },
    PublicCountMismatch {
        expected: u64,
        found: u64,
    },
    InvalidTranscriptArity,
    Io {
        message: String,
    },
}

impl fmt::Display for GlobalInfoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMagic => write!(f, "invalid global-info file magic"),
            Self::UnsupportedVersion { found, max } => {
                write!(f, "unsupported global-info file version {found}, max {max}")
            }
            Self::InvalidSectionCount { found } => {
                write!(f, "invalid global-info section count {found}")
            }
            Self::InvalidSectionId { found } => {
                write!(f, "invalid global-info section id {found}")
            }
            Self::UnexpectedTrailingBytes { count } => {
                write!(f, "unexpected trailing bytes in global-info file: {count}")
            }
            Self::UnexpectedEof {
                offset,
                needed,
                available,
            } => write!(
                f,
                "unexpected end of global-info file at {offset}, needed {needed}, available {available}"
            ),
            Self::InvalidUtf8 => write!(f, "global-info string is not valid utf-8"),
            Self::MissingStringTerminator { offset } => {
                write!(f, "missing global-info string terminator at offset {offset}")
            }
            Self::LengthOverflow => write!(f, "global-info length overflow"),
            Self::StringContainsNul { value } => {
                write!(f, "global-info string contains nul byte: {value}")
            }
            Self::InvalidFlag { field, value } => {
                write!(f, "invalid global-info flag for {field}: {value}")
            }
            Self::AirGroupCountMismatch {
                air_groups,
                airs,
                aggregation_types,
            } => write!(
                f,
                "global-info air group count mismatch: air_groups {air_groups}, airs {airs}, aggregation_types {aggregation_types}"
            ),
            Self::EmptyAirGroup { airgroup_id } => {
                write!(f, "global-info air group {airgroup_id} has no units")
            }
            Self::InvalidRowCount {
                airgroup_id,
                air_id,
            } => write!(
                f,
                "global-info unit row count is invalid at group {airgroup_id}, unit {air_id}"
            ),
            Self::InvalidStage { field, index } => {
                write!(f, "global-info {field} stage is invalid at index {index}")
            }
            Self::InvalidLength { field, index } => {
                write!(f, "global-info {field} length is invalid at index {index}")
            }
            Self::DuplicateValueName { field, name } => {
                write!(f, "global-info {field} has duplicate value name {name}")
            }
            Self::PublicCountMismatch { expected, found } => write!(
                f,
                "global-info public count mismatch: expected {expected}, found {found}"
            ),
            Self::InvalidTranscriptArity => write!(f, "global-info transcript arity is invalid"),
            Self::Io { message } => write!(f, "global-info io error: {message}"),
        }
    }
}

impl std::error::Error for GlobalInfoError {}

impl From<SectionedError> for GlobalInfoError {
    fn from(value: SectionedError) -> Self {
        match value {
            SectionedError::InvalidKind { .. } => Self::InvalidMagic,
            SectionedError::UnsupportedVersion { found, max } => {
                Self::UnsupportedVersion { found, max }
            }
            SectionedError::UnexpectedTrailingBytes { count } => {
                Self::UnexpectedTrailingBytes { count }
            }
            SectionedError::UnexpectedEof {
                offset,
                needed,
                available,
            } => Self::UnexpectedEof {
                offset,
                needed,
                available,
            },
            SectionedError::LengthOverflow => Self::LengthOverflow,
        }
    }
}

impl GlobalInfo {
    pub fn total_air_count(&self) -> usize {
        self.airs.iter().map(Vec::len).sum()
    }

    pub fn stage_one_proof_value_count(&self) -> usize {
        self.proof_values_map
            .iter()
            .filter(|entry| entry.stage == 1)
            .count()
    }
}

pub fn read_global_info_file(path: impl AsRef<Path>) -> Result<GlobalInfo, GlobalInfoError> {
    read_global_info_binary_file(path)
}

pub fn read_global_info_binary_file(path: impl AsRef<Path>) -> Result<GlobalInfo, GlobalInfoError> {
    let bytes = std::fs::read(path).map_err(|error| GlobalInfoError::Io {
        message: error.to_string(),
    })?;
    parse_global_info(&bytes)
}

pub fn parse_global_info(bytes: &[u8]) -> Result<GlobalInfo, GlobalInfoError> {
    let file = parse_sectioned_file(bytes, GLOBAL_INFO_KIND, GLOBAL_INFO_VERSION)
        .map_err(GlobalInfoError::from)?;
    if file.version != GLOBAL_INFO_VERSION {
        return Err(GlobalInfoError::UnsupportedVersion {
            found: file.version,
            max: GLOBAL_INFO_VERSION,
        });
    }

    if file.sections.len() != 1 {
        return Err(GlobalInfoError::InvalidSectionCount {
            found: u32::try_from(file.sections.len()).unwrap_or(u32::MAX),
        });
    }

    let section = &file.sections[0];
    if section.id != GLOBAL_INFO_SECTION_ID {
        return Err(GlobalInfoError::InvalidSectionId { found: section.id });
    }

    parse_global_info_section(&section.data)
}

pub fn encode_global_info(value: &GlobalInfo) -> Result<Vec<u8>, GlobalInfoError> {
    validate_global_info(value)?;
    let section = encode_global_info_section(value)?;
    let file = SectionedFile {
        kind: GLOBAL_INFO_KIND,
        version: GLOBAL_INFO_VERSION,
        sections: vec![SectionedSection {
            id: GLOBAL_INFO_SECTION_ID,
            data: section,
        }],
    };
    encode_sectioned_file(&file).map_err(GlobalInfoError::from)
}

fn parse_global_info_section(bytes: &[u8]) -> Result<GlobalInfo, GlobalInfoError> {
    let mut reader = Reader::new(bytes);
    let name = reader.read_string()?;
    let curve = read_curve_tag(reader.read_u8()?)?;
    let lattice_size = reader.read_optional_u64("lattice_size")?;
    let transcript_arity = reader.read_u64()?;
    let n_publics = reader.read_u64()?;

    let air_group_count = read_bounded_count(&mut reader, AIR_GROUP_MIN_BYTES)?;
    let mut air_groups = Vec::with_capacity(air_group_count);
    for _ in 0..air_group_count {
        air_groups.push(reader.read_string()?);
    }

    let airs_group_count = read_bounded_count(&mut reader, AIR_SECTION_MIN_BYTES)?;
    let mut airs = Vec::with_capacity(airs_group_count);
    for _ in 0..airs_group_count {
        let unit_count = read_bounded_count(&mut reader, AIR_UNIT_MIN_BYTES)?;
        let mut units = Vec::with_capacity(unit_count);
        for _ in 0..unit_count {
            units.push(GlobalAir {
                name: reader.read_string()?,
                num_rows: reader.read_u64()?,
                has_compressor: reader.read_bool("has_compressor")?,
            });
        }
        airs.push(units);
    }

    let aggregation_group_count = read_bounded_count(&mut reader, AGGREGATION_GROUP_MIN_BYTES)?;
    let mut aggregation_types = Vec::with_capacity(aggregation_group_count);
    for _ in 0..aggregation_group_count {
        let entry_count = read_bounded_count(&mut reader, AGGREGATION_ENTRY_BYTES)?;
        let mut entries = Vec::with_capacity(entry_count);
        for _ in 0..entry_count {
            entries.push(AggregationType {
                aggregation_type: reader.read_u64()?,
            });
        }
        aggregation_types.push(entries);
    }

    let num_challenges = read_u64_vec(&mut reader)?;
    let num_proof_values = read_u64_vec(&mut reader)?;
    let proof_values_map = read_named_stage_values(&mut reader)?;
    let publics_map = read_public_values(&mut reader)?;

    if reader.position() != bytes.len() {
        return Err(GlobalInfoError::UnexpectedTrailingBytes {
            count: bytes.len() - reader.position(),
        });
    }

    let info = GlobalInfo {
        name,
        air_groups,
        airs,
        curve,
        lattice_size,
        aggregation_types,
        n_publics,
        num_challenges,
        num_proof_values,
        proof_values_map,
        publics_map,
        transcript_arity,
    };
    validate_global_info(&info)?;
    Ok(info)
}

fn encode_global_info_section(value: &GlobalInfo) -> Result<Vec<u8>, GlobalInfoError> {
    let mut section = Vec::new();
    write_string(&mut section, &value.name)?;
    section.push(curve_tag(&value.curve));
    write_optional_u64(&mut section, value.lattice_size);
    write_u64(&mut section, value.transcript_arity);
    write_u64(&mut section, value.n_publics);

    write_u32(&mut section, usize_to_u32(value.air_groups.len())?);
    for name in &value.air_groups {
        write_string(&mut section, name)?;
    }

    write_u32(&mut section, usize_to_u32(value.airs.len())?);
    for group in &value.airs {
        write_u32(&mut section, usize_to_u32(group.len())?);
        for unit in group {
            write_string(&mut section, &unit.name)?;
            write_u64(&mut section, unit.num_rows);
            write_bool(&mut section, unit.has_compressor);
        }
    }

    write_u32(&mut section, usize_to_u32(value.aggregation_types.len())?);
    for group in &value.aggregation_types {
        write_u32(&mut section, usize_to_u32(group.len())?);
        for entry in group {
            write_u64(&mut section, entry.aggregation_type);
        }
    }

    write_u64_vec(&mut section, &value.num_challenges)?;
    write_u64_vec(&mut section, &value.num_proof_values)?;
    write_named_stage_values(&mut section, &value.proof_values_map)?;
    write_public_values(&mut section, &value.publics_map)?;
    Ok(section)
}

fn validate_air_group_shape(
    air_groups: &[String],
    airs: &[Vec<GlobalAir>],
    aggregation_types: &[Vec<AggregationType>],
) -> Result<(), GlobalInfoError> {
    if air_groups.len() != airs.len() || air_groups.len() != aggregation_types.len() {
        return Err(GlobalInfoError::AirGroupCountMismatch {
            air_groups: air_groups.len(),
            airs: airs.len(),
            aggregation_types: aggregation_types.len(),
        });
    }
    for (airgroup_id, units) in airs.iter().enumerate() {
        if units.is_empty() {
            return Err(GlobalInfoError::EmptyAirGroup { airgroup_id });
        }
    }
    Ok(())
}

fn read_named_stage_values(
    reader: &mut Reader<'_>,
) -> Result<Vec<NamedStageValue>, GlobalInfoError> {
    let value_count = read_bounded_count(reader, NAMED_STAGE_VALUE_MIN_BYTES)?;
    let mut values = Vec::with_capacity(value_count);
    for _ in 0..value_count {
        let name = reader.read_string()?;
        let stage = reader.read_u64()?;
        let id = reader.read_optional_u64("proof_value_id")?;
        let lengths_count = read_bounded_count(reader, U64_BYTES)?;
        let mut lengths = Vec::with_capacity(lengths_count);
        for _ in 0..lengths_count {
            lengths.push(reader.read_u64()?);
        }
        values.push(NamedStageValue {
            name,
            stage,
            id,
            lengths,
        });
    }
    Ok(values)
}

fn read_public_values(reader: &mut Reader<'_>) -> Result<Vec<PublicValue>, GlobalInfoError> {
    let value_count = read_bounded_count(reader, PUBLIC_VALUE_MIN_BYTES)?;
    let mut values = Vec::with_capacity(value_count);
    for _ in 0..value_count {
        let name = reader.read_string()?;
        let stage = reader.read_u64()?;
        let lengths_count = read_bounded_count(reader, U64_BYTES)?;
        let mut lengths = Vec::with_capacity(lengths_count);
        for _ in 0..lengths_count {
            lengths.push(reader.read_u64()?);
        }
        values.push(PublicValue {
            name,
            stage,
            lengths,
        });
    }
    Ok(values)
}

fn write_named_stage_values(
    out: &mut Vec<u8>,
    values: &[NamedStageValue],
) -> Result<(), GlobalInfoError> {
    write_u32(out, usize_to_u32(values.len())?);
    for value in values {
        write_string(out, &value.name)?;
        write_u64(out, value.stage);
        write_optional_u64(out, value.id);
        write_u32(out, usize_to_u32(value.lengths.len())?);
        for length in &value.lengths {
            write_u64(out, *length);
        }
    }
    Ok(())
}

fn write_public_values(out: &mut Vec<u8>, values: &[PublicValue]) -> Result<(), GlobalInfoError> {
    write_u32(out, usize_to_u32(values.len())?);
    for value in values {
        write_string(out, &value.name)?;
        write_u64(out, value.stage);
        write_u32(out, usize_to_u32(value.lengths.len())?);
        for length in &value.lengths {
            write_u64(out, *length);
        }
    }
    Ok(())
}

fn read_u64_vec(reader: &mut Reader<'_>) -> Result<Vec<u64>, GlobalInfoError> {
    let count = read_bounded_count(reader, U64_BYTES)?;
    let mut values = Vec::with_capacity(count);
    for _ in 0..count {
        values.push(reader.read_u64()?);
    }
    Ok(values)
}

fn write_u64_vec(out: &mut Vec<u8>, values: &[u64]) -> Result<(), GlobalInfoError> {
    write_u32(out, usize_to_u32(values.len())?);
    for value in values {
        write_u64(out, *value);
    }
    Ok(())
}

pub(crate) fn validate_global_info(value: &GlobalInfo) -> Result<(), GlobalInfoError> {
    validate_air_group_shape(&value.air_groups, &value.airs, &value.aggregation_types)?;
    validate_unique_value_names("airGroups", value.air_groups.iter().map(String::as_str))?;
    for (airgroup_id, units) in value.airs.iter().enumerate() {
        validate_unique_value_names("airs", units.iter().map(|unit| unit.name.as_str()))?;
        for (air_id, unit) in units.iter().enumerate() {
            if unit.num_rows == 0 {
                return Err(GlobalInfoError::InvalidRowCount {
                    airgroup_id,
                    air_id,
                });
            }
        }
    }
    for (index, entry) in value.proof_values_map.iter().enumerate() {
        if entry.stage == 0 {
            return Err(GlobalInfoError::InvalidStage {
                field: "proofValuesMap",
                index,
            });
        }
        validate_nonzero_lengths("proofValuesMap", index, &entry.lengths)?;
    }
    validate_unique_value_names(
        "proofValuesMap",
        value
            .proof_values_map
            .iter()
            .map(|entry| entry.name.as_str()),
    )?;
    for (index, entry) in value.publics_map.iter().enumerate() {
        if entry.stage == 0 {
            return Err(GlobalInfoError::InvalidStage {
                field: "publicsMap",
                index,
            });
        }
        validate_nonzero_lengths("publicsMap", index, &entry.lengths)?;
    }
    validate_unique_value_names(
        "publicsMap",
        value.publics_map.iter().map(|entry| entry.name.as_str()),
    )?;
    validate_disjoint_value_names(
        "globalValues",
        value
            .proof_values_map
            .iter()
            .map(|entry| entry.name.as_str()),
        value.publics_map.iter().map(|entry| entry.name.as_str()),
    )?;
    let public_count = global_public_count(&value.publics_map)?;
    if value.n_publics != public_count {
        return Err(GlobalInfoError::PublicCountMismatch {
            expected: value.n_publics,
            found: public_count,
        });
    }
    if !is_supported_transcript_arity_u64(value.transcript_arity) {
        return Err(GlobalInfoError::InvalidTranscriptArity);
    }
    Ok(())
}

fn validate_disjoint_value_names<'a>(
    field: &'static str,
    left: impl IntoIterator<Item = &'a str>,
    right: impl IntoIterator<Item = &'a str>,
) -> Result<(), GlobalInfoError> {
    let seen = left.into_iter().collect::<BTreeSet<_>>();
    for name in right {
        if seen.contains(name) {
            return Err(GlobalInfoError::DuplicateValueName {
                field,
                name: name.to_owned(),
            });
        }
    }
    Ok(())
}

fn validate_unique_value_names<'a>(
    field: &'static str,
    names: impl IntoIterator<Item = &'a str>,
) -> Result<(), GlobalInfoError> {
    let mut seen = BTreeSet::new();
    for name in names {
        if !seen.insert(name) {
            return Err(GlobalInfoError::DuplicateValueName {
                field,
                name: name.to_owned(),
            });
        }
    }
    Ok(())
}

fn validate_nonzero_lengths(
    field: &'static str,
    index: usize,
    lengths: &[u64],
) -> Result<(), GlobalInfoError> {
    if lengths.iter().any(|length| *length == 0) {
        return Err(GlobalInfoError::InvalidLength { field, index });
    }
    Ok(())
}

fn global_public_count(publics: &[PublicValue]) -> Result<u64, GlobalInfoError> {
    publics.iter().try_fold(0_u64, |count, entry| {
        let dimension = entry.lengths.iter().try_fold(1_u64, |dimension, length| {
            dimension
                .checked_mul(*length)
                .ok_or(GlobalInfoError::LengthOverflow)
        })?;
        count
            .checked_add(dimension)
            .ok_or(GlobalInfoError::LengthOverflow)
    })
}

fn curve_tag(curve: &CurveKind) -> u8 {
    match curve {
        CurveKind::None => 0,
        CurveKind::EcGfp5 => 1,
        CurveKind::EcMasFp5 => 2,
    }
}

fn read_curve_tag(tag: u8) -> Result<CurveKind, GlobalInfoError> {
    match tag {
        0 => Ok(CurveKind::None),
        1 => Ok(CurveKind::EcGfp5),
        2 => Ok(CurveKind::EcMasFp5),
        value => Err(GlobalInfoError::InvalidFlag {
            field: "curve",
            value,
        }),
    }
}

fn usize_to_u32(value: usize) -> Result<u32, GlobalInfoError> {
    u32::try_from(value).map_err(|_| GlobalInfoError::LengthOverflow)
}

fn read_bounded_count(
    reader: &mut Reader<'_>,
    record_min_bytes: usize,
) -> Result<usize, GlobalInfoError> {
    let count = u32_to_usize(reader.read_u32()?)?;
    reader.require_items(count, record_min_bytes)?;
    Ok(count)
}

fn u32_to_usize(value: u32) -> Result<usize, GlobalInfoError> {
    usize::try_from(value).map_err(|_| GlobalInfoError::LengthOverflow)
}

fn write_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_bool(out: &mut Vec<u8>, value: bool) {
    out.push(u8::from(value));
}

fn write_optional_u64(out: &mut Vec<u8>, value: Option<u64>) {
    match value {
        Some(value) => {
            out.push(1);
            write_u64(out, value);
        }
        None => out.push(0),
    }
}

fn write_string(out: &mut Vec<u8>, value: &str) -> Result<(), GlobalInfoError> {
    if value.as_bytes().contains(&0) {
        return Err(GlobalInfoError::StringContainsNul {
            value: value.to_owned(),
        });
    }
    out.extend_from_slice(value.as_bytes());
    out.push(0);
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

    fn position(&self) -> usize {
        self.offset
    }

    fn read_exact(&mut self, count: usize) -> Result<&'a [u8], GlobalInfoError> {
        let end = self
            .offset
            .checked_add(count)
            .ok_or(GlobalInfoError::LengthOverflow)?;
        if end > self.bytes.len() {
            return Err(GlobalInfoError::UnexpectedEof {
                offset: self.offset,
                needed: count,
                available: self.bytes.len().saturating_sub(self.offset),
            });
        }
        let out = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(out)
    }

    fn require_items(&self, count: usize, item_bytes: usize) -> Result<(), GlobalInfoError> {
        let needed = count
            .checked_mul(item_bytes)
            .ok_or(GlobalInfoError::LengthOverflow)?;
        let end = self
            .offset
            .checked_add(needed)
            .ok_or(GlobalInfoError::LengthOverflow)?;
        if end > self.bytes.len() {
            return Err(GlobalInfoError::UnexpectedEof {
                offset: self.offset,
                needed,
                available: self.bytes.len().saturating_sub(self.offset),
            });
        }
        Ok(())
    }

    fn read_u8(&mut self) -> Result<u8, GlobalInfoError> {
        Ok(self.read_exact(1)?[0])
    }

    fn read_u32(&mut self) -> Result<u32, GlobalInfoError> {
        let bytes = self.read_exact(4)?;
        Ok(u32::from_le_bytes(
            bytes.try_into().expect("slice length checked"),
        ))
    }

    fn read_u64(&mut self) -> Result<u64, GlobalInfoError> {
        let bytes = self.read_exact(8)?;
        Ok(u64::from_le_bytes(
            bytes.try_into().expect("slice length checked"),
        ))
    }

    fn read_bool(&mut self, field: &'static str) -> Result<bool, GlobalInfoError> {
        match self.read_u8()? {
            0 => Ok(false),
            1 => Ok(true),
            value => Err(GlobalInfoError::InvalidFlag { field, value }),
        }
    }

    fn read_optional_u64(&mut self, field: &'static str) -> Result<Option<u64>, GlobalInfoError> {
        match self.read_u8()? {
            0 => Ok(None),
            1 => Ok(Some(self.read_u64()?)),
            value => Err(GlobalInfoError::InvalidFlag { field, value }),
        }
    }

    fn read_string(&mut self) -> Result<String, GlobalInfoError> {
        let start = self.offset;
        let Some(relative_end) = self.bytes[start..].iter().position(|byte| *byte == 0) else {
            return Err(GlobalInfoError::MissingStringTerminator { offset: start });
        };
        let end = start
            .checked_add(relative_end)
            .ok_or(GlobalInfoError::LengthOverflow)?;
        self.offset = end + 1;
        String::from_utf8(self.bytes[start..end].to_vec()).map_err(|_| GlobalInfoError::InvalidUtf8)
    }
}
