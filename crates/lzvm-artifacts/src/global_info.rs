use std::fmt;
use std::path::Path;

use crate::sectioned::{
    encode_sectioned_file, parse_sectioned_file, SectionedError, SectionedFile, SectionedSection,
};

const GLOBAL_INFO_KIND: [u8; 4] = *b"ginf";
const GLOBAL_INFO_VERSION: u32 = 1;
const GLOBAL_INFO_SECTION_ID: u32 = 1;

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
    Json {
        message: String,
    },
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
    MissingField {
        field: &'static str,
    },
    InvalidField {
        field: &'static str,
    },
    UnknownCurve {
        curve: String,
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
    PublicCountMismatch {
        expected: u64,
        found: usize,
    },
    InvalidTranscriptArity,
    Io {
        message: String,
    },
}

impl fmt::Display for GlobalInfoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json { message } => write!(f, "global-info json error: {message}"),
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
            Self::MissingField { field } => write!(f, "missing global-info field: {field}"),
            Self::InvalidField { field } => write!(f, "invalid global-info field: {field}"),
            Self::UnknownCurve { curve } => write!(f, "unknown global-info curve: {curve}"),
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

#[cfg(feature = "json")]
pub fn parse_global_info_json(input: &str) -> Result<GlobalInfo, GlobalInfoError> {
    let value: serde_json::Value =
        serde_json::from_str(input).map_err(|error| GlobalInfoError::Json {
            message: error.to_string(),
        })?;
    let object = as_object(&value, "$")?;

    let name = required_string(object, "name")?;
    let air_groups = required_string_array(object, "air_groups")?;
    let airs = parse_airs(required_array(object, "airs")?)?;
    let aggregation_types = parse_aggregation_types(required_array(object, "aggTypes")?)?;
    validate_air_group_shape(&air_groups, &airs, &aggregation_types)?;

    let proof_values_map =
        parse_named_stage_values(optional_array(object, "proofValuesMap")?, "proofValuesMap")?;
    let publics_map = parse_public_values(optional_array(object, "publicsMap")?)?;
    let n_publics = required_u64(object, "nPublics")?;
    if n_publics != publics_map.len() as u64 {
        return Err(GlobalInfoError::PublicCountMismatch {
            expected: n_publics,
            found: publics_map.len(),
        });
    }

    let transcript_arity = required_u64(object, "transcriptArity")?;
    if transcript_arity == 0 {
        return Err(GlobalInfoError::InvalidTranscriptArity);
    }

    let info = GlobalInfo {
        name,
        air_groups,
        airs,
        curve: parse_curve(&required_string(object, "curve")?)?,
        lattice_size: optional_u64(object, "latticeSize")?,
        aggregation_types,
        n_publics,
        num_challenges: required_u64_array(object, "numChallenges")?,
        num_proof_values: optional_u64_array(object, "numProofValues")?.unwrap_or_default(),
        proof_values_map,
        publics_map,
        transcript_arity,
    };
    validate_global_info(&info)?;
    Ok(info)
}

fn parse_global_info_section(bytes: &[u8]) -> Result<GlobalInfo, GlobalInfoError> {
    let mut reader = Reader::new(bytes);
    let name = reader.read_string()?;
    let curve = read_curve_tag(reader.read_u8()?)?;
    let lattice_size = reader.read_optional_u64("lattice_size")?;
    let transcript_arity = reader.read_u64()?;
    let n_publics = reader.read_u64()?;

    let air_group_count = reader.read_u32()?;
    let mut air_groups = Vec::with_capacity(air_group_count as usize);
    for _ in 0..air_group_count {
        air_groups.push(reader.read_string()?);
    }

    let airs_group_count = reader.read_u32()?;
    let mut airs = Vec::with_capacity(airs_group_count as usize);
    for _ in 0..airs_group_count {
        let unit_count = reader.read_u32()?;
        let mut units = Vec::with_capacity(unit_count as usize);
        for _ in 0..unit_count {
            units.push(GlobalAir {
                name: reader.read_string()?,
                num_rows: reader.read_u64()?,
                has_compressor: reader.read_bool("has_compressor")?,
            });
        }
        airs.push(units);
    }

    let aggregation_group_count = reader.read_u32()?;
    let mut aggregation_types = Vec::with_capacity(aggregation_group_count as usize);
    for _ in 0..aggregation_group_count {
        let entry_count = reader.read_u32()?;
        let mut entries = Vec::with_capacity(entry_count as usize);
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

#[cfg(feature = "json")]
fn parse_airs(values: &[serde_json::Value]) -> Result<Vec<Vec<GlobalAir>>, GlobalInfoError> {
    let mut out = Vec::with_capacity(values.len());
    for (airgroup_id, group) in values.iter().enumerate() {
        let units = group
            .as_array()
            .ok_or(GlobalInfoError::InvalidField { field: "airs" })?;
        let mut parsed_units = Vec::with_capacity(units.len());
        for (air_id, unit) in units.iter().enumerate() {
            let object = as_object(unit, "airs")?;
            let num_rows = required_u64(object, "num_rows")?;
            if num_rows == 0 {
                return Err(GlobalInfoError::InvalidRowCount {
                    airgroup_id,
                    air_id,
                });
            }
            parsed_units.push(GlobalAir {
                name: required_string(object, "name")?,
                num_rows,
                has_compressor: optional_bool(object, "hasCompressor")?.unwrap_or(false),
            });
        }
        out.push(parsed_units);
    }
    Ok(out)
}

#[cfg(feature = "json")]
fn parse_aggregation_types(
    values: &[serde_json::Value],
) -> Result<Vec<Vec<AggregationType>>, GlobalInfoError> {
    let mut out = Vec::with_capacity(values.len());
    for group in values {
        let entries = group
            .as_array()
            .ok_or(GlobalInfoError::InvalidField { field: "aggTypes" })?;
        let mut parsed_entries = Vec::with_capacity(entries.len());
        for entry in entries {
            let object = as_object(entry, "aggTypes")?;
            parsed_entries.push(AggregationType {
                aggregation_type: required_u64(object, "aggType")?,
            });
        }
        out.push(parsed_entries);
    }
    Ok(out)
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

#[cfg(feature = "json")]
fn parse_named_stage_values(
    values: Option<&Vec<serde_json::Value>>,
    field: &'static str,
) -> Result<Vec<NamedStageValue>, GlobalInfoError> {
    let Some(values) = values else {
        return Ok(Vec::new());
    };
    let mut out = Vec::with_capacity(values.len());
    for (index, value) in values.iter().enumerate() {
        let object = as_object(value, field)?;
        let stage = optional_u64(object, "stage")?.unwrap_or(0);
        if stage == 0 {
            return Err(GlobalInfoError::InvalidStage { field, index });
        }
        out.push(NamedStageValue {
            name: required_string(object, "name")?,
            stage,
            id: optional_u64(object, "id")?,
            lengths: optional_u64_array(object, "lengths")?.unwrap_or_default(),
        });
    }
    Ok(out)
}

#[cfg(feature = "json")]
fn parse_public_values(
    values: Option<&Vec<serde_json::Value>>,
) -> Result<Vec<PublicValue>, GlobalInfoError> {
    let Some(values) = values else {
        return Ok(Vec::new());
    };
    let mut out = Vec::with_capacity(values.len());
    for (index, value) in values.iter().enumerate() {
        let object = as_object(value, "publicsMap")?;
        let stage = optional_u64(object, "stage")?.unwrap_or(0);
        if stage == 0 {
            return Err(GlobalInfoError::InvalidStage {
                field: "publicsMap",
                index,
            });
        }
        out.push(PublicValue {
            name: required_string(object, "name")?,
            stage,
            lengths: optional_u64_array(object, "lengths")?.unwrap_or_default(),
        });
    }
    Ok(out)
}

fn read_named_stage_values(
    reader: &mut Reader<'_>,
) -> Result<Vec<NamedStageValue>, GlobalInfoError> {
    let value_count = reader.read_u32()?;
    let mut values = Vec::with_capacity(value_count as usize);
    for index in 0..value_count {
        let lengths_count = {
            let name = reader.read_string()?;
            let stage = reader.read_u64()?;
            let id = reader.read_optional_u64("proof_value_id")?;
            let lengths_count = reader.read_u32()?;
            values.push(NamedStageValue {
                name,
                stage,
                id,
                lengths: Vec::with_capacity(lengths_count as usize),
            });
            lengths_count
        };
        for _ in 0..lengths_count {
            values[index as usize].lengths.push(reader.read_u64()?);
        }
    }
    Ok(values)
}

fn read_public_values(reader: &mut Reader<'_>) -> Result<Vec<PublicValue>, GlobalInfoError> {
    let value_count = reader.read_u32()?;
    let mut values = Vec::with_capacity(value_count as usize);
    for index in 0..value_count {
        let lengths_count = {
            let name = reader.read_string()?;
            let stage = reader.read_u64()?;
            let lengths_count = reader.read_u32()?;
            values.push(PublicValue {
                name,
                stage,
                lengths: Vec::with_capacity(lengths_count as usize),
            });
            lengths_count
        };
        for _ in 0..lengths_count {
            values[index as usize].lengths.push(reader.read_u64()?);
        }
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
    let count = reader.read_u32()?;
    let mut values = Vec::with_capacity(count as usize);
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

fn validate_global_info(value: &GlobalInfo) -> Result<(), GlobalInfoError> {
    validate_air_group_shape(&value.air_groups, &value.airs, &value.aggregation_types)?;
    for (airgroup_id, units) in value.airs.iter().enumerate() {
        for (air_id, unit) in units.iter().enumerate() {
            if unit.num_rows == 0 {
                return Err(GlobalInfoError::InvalidRowCount {
                    airgroup_id,
                    air_id,
                });
            }
        }
    }
    if value.n_publics != value.publics_map.len() as u64 {
        return Err(GlobalInfoError::PublicCountMismatch {
            expected: value.n_publics,
            found: value.publics_map.len(),
        });
    }
    if value.transcript_arity == 0 {
        return Err(GlobalInfoError::InvalidTranscriptArity);
    }
    for (index, entry) in value.proof_values_map.iter().enumerate() {
        if entry.stage == 0 {
            return Err(GlobalInfoError::InvalidStage {
                field: "proofValuesMap",
                index,
            });
        }
    }
    for (index, entry) in value.publics_map.iter().enumerate() {
        if entry.stage == 0 {
            return Err(GlobalInfoError::InvalidStage {
                field: "publicsMap",
                index,
            });
        }
    }
    Ok(())
}

#[cfg(feature = "json")]
fn parse_curve(curve: &str) -> Result<CurveKind, GlobalInfoError> {
    match curve {
        "None" => Ok(CurveKind::None),
        "EcGFp5" => Ok(CurveKind::EcGfp5),
        "EcMasFp5" => Ok(CurveKind::EcMasFp5),
        _ => Err(GlobalInfoError::UnknownCurve {
            curve: curve.to_owned(),
        }),
    }
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

#[cfg(feature = "json")]
fn as_object<'a>(
    value: &'a serde_json::Value,
    field: &'static str,
) -> Result<&'a serde_json::Map<String, serde_json::Value>, GlobalInfoError> {
    value
        .as_object()
        .ok_or(GlobalInfoError::InvalidField { field })
}

#[cfg(feature = "json")]
fn required<'a>(
    object: &'a serde_json::Map<String, serde_json::Value>,
    field: &'static str,
) -> Result<&'a serde_json::Value, GlobalInfoError> {
    object
        .get(field)
        .ok_or(GlobalInfoError::MissingField { field })
}

#[cfg(feature = "json")]
fn required_array<'a>(
    object: &'a serde_json::Map<String, serde_json::Value>,
    field: &'static str,
) -> Result<&'a Vec<serde_json::Value>, GlobalInfoError> {
    required(object, field)?
        .as_array()
        .ok_or(GlobalInfoError::InvalidField { field })
}

#[cfg(feature = "json")]
fn optional_array<'a>(
    object: &'a serde_json::Map<String, serde_json::Value>,
    field: &'static str,
) -> Result<Option<&'a Vec<serde_json::Value>>, GlobalInfoError> {
    object
        .get(field)
        .map(|value| {
            value
                .as_array()
                .ok_or(GlobalInfoError::InvalidField { field })
        })
        .transpose()
}

#[cfg(feature = "json")]
fn required_string(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &'static str,
) -> Result<String, GlobalInfoError> {
    required(object, field)?
        .as_str()
        .map(str::to_owned)
        .ok_or(GlobalInfoError::InvalidField { field })
}

#[cfg(feature = "json")]
fn required_string_array(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &'static str,
) -> Result<Vec<String>, GlobalInfoError> {
    let values = required_array(object, field)?;
    let mut out = Vec::with_capacity(values.len());
    for value in values {
        out.push(
            value
                .as_str()
                .map(str::to_owned)
                .ok_or(GlobalInfoError::InvalidField { field })?,
        );
    }
    Ok(out)
}

#[cfg(feature = "json")]
fn required_u64(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &'static str,
) -> Result<u64, GlobalInfoError> {
    value_to_u64(required(object, field)?, field)
}

#[cfg(feature = "json")]
fn optional_u64(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &'static str,
) -> Result<Option<u64>, GlobalInfoError> {
    object
        .get(field)
        .map(|value| value_to_u64(value, field))
        .transpose()
}

#[cfg(feature = "json")]
fn value_to_u64(value: &serde_json::Value, field: &'static str) -> Result<u64, GlobalInfoError> {
    value
        .as_u64()
        .ok_or(GlobalInfoError::InvalidField { field })
}

#[cfg(feature = "json")]
fn optional_bool(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &'static str,
) -> Result<Option<bool>, GlobalInfoError> {
    object
        .get(field)
        .map(|value| {
            value
                .as_bool()
                .ok_or(GlobalInfoError::InvalidField { field })
        })
        .transpose()
}

#[cfg(feature = "json")]
fn required_u64_array(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &'static str,
) -> Result<Vec<u64>, GlobalInfoError> {
    let values = required_array(object, field)?;
    let mut out = Vec::with_capacity(values.len());
    for value in values {
        out.push(value_to_u64(value, field)?);
    }
    Ok(out)
}

#[cfg(feature = "json")]
fn optional_u64_array(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &'static str,
) -> Result<Option<Vec<u64>>, GlobalInfoError> {
    let Some(values) = optional_array(object, field)? else {
        return Ok(None);
    };
    let mut out = Vec::with_capacity(values.len());
    for value in values {
        out.push(value_to_u64(value, field)?);
    }
    Ok(Some(out))
}

fn usize_to_u32(value: usize) -> Result<u32, GlobalInfoError> {
    u32::try_from(value).map_err(|_| GlobalInfoError::LengthOverflow)
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
