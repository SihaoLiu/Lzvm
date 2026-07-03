use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

use crate::sectioned::{
    encode_sectioned_file, parse_sectioned_file, SectionedError, SectionedFile, SectionedSection,
};

const SETUP_INFO_KIND: [u8; 4] = *b"uinf";
const SETUP_INFO_VERSION: u32 = 3;
const SETUP_INFO_SECTION_ID: u32 = 1;

const U32_BYTES: usize = 4;
const I64_BYTES: usize = 8;
const FLAG_BYTES: usize = 1;
const STRING_MIN_BYTES: usize = 1;
const CONSTANT_COLUMN_MIN_BYTES: usize = STRING_MIN_BYTES + U32_BYTES * 5;
const BOUNDARY_MIN_BYTES: usize = FLAG_BYTES * 3;
const FRI_STEP_BYTES: usize = U32_BYTES;
const COMMITMENT_COLUMN_MIN_BYTES: usize =
    STRING_MIN_BYTES + U32_BYTES * 5 + FLAG_BYTES + U32_BYTES;
const STAGE_VALUE_MIN_BYTES: usize = STRING_MIN_BYTES + U32_BYTES + U32_BYTES;
const EVALUATION_MAP_ENTRY_MIN_BYTES: usize =
    FLAG_BYTES + U32_BYTES + I64_BYTES + U32_BYTES + FLAG_BYTES;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitSetupInfo {
    pub n_stages: u32,
    pub n_constants: u32,
    pub constant_columns: Vec<ConstantColumn>,
    pub n_publics: Option<u32>,
    pub n_constraints: Option<u32>,
    pub q_degree: u32,
    pub opening_points: Vec<i64>,
    pub section_widths: BTreeMap<String, u32>,
    pub challenge_count: usize,
    pub eval_count: usize,
    pub evaluation_map: Vec<EvaluationMapEntry>,
    pub boundaries: Vec<Boundary>,
    pub commitment_columns: Vec<CommitmentColumn>,
    pub unit_value_map: Vec<StageValue>,
    pub group_value_map: Vec<StageValue>,
    pub stark: StarkStruct,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StarkStruct {
    pub n_bits: u32,
    pub n_bits_ext: u32,
    pub n_queries: u32,
    pub steps: Vec<FriStep>,
    pub hash_commits: bool,
    pub last_level_verification: u32,
    pub pow_bits: u32,
    pub merkle_tree_arity: u32,
    pub verification_hash_type: Option<String>,
    pub transcript_arity: Option<u32>,
    pub merkle_tree_custom: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FriStep {
    pub n_bits: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Boundary {
    pub name: Option<String>,
    pub offset_min: Option<i64>,
    pub offset_max: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstantColumn {
    pub name: String,
    pub stage: u32,
    pub dimension: u32,
    pub pols_map_id: u32,
    pub stage_id: u32,
    pub lengths: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitmentColumn {
    pub name: String,
    pub stage: u32,
    pub dimension: u32,
    pub pols_map_id: u32,
    pub stage_id: u32,
    pub stage_position: u32,
    pub intermediate: bool,
    pub lengths: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageValue {
    pub name: String,
    pub stage: u32,
    pub lengths: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvaluationMapEntry {
    pub kind: EvaluationMapKind,
    pub id: u32,
    pub prime: i64,
    pub opening_position: u32,
    pub commit_id: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EvaluationMapKind {
    #[default]
    Constant,
    Commitment,
    Custom,
}

impl Default for EvaluationMapEntry {
    fn default() -> Self {
        Self {
            kind: EvaluationMapKind::Constant,
            id: 0,
            prime: 0,
            opening_position: 0,
            commit_id: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetupInfoError {
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
    MissingSectionWidth {
        name: String,
    },
    InvalidDomainBits {
        n_bits: u32,
        n_bits_ext: u32,
    },
    InvalidFriSteps,
    ConstantColumnCountMismatch {
        expected: u32,
        found: usize,
    },
    InvalidConstantColumn {
        index: usize,
    },
    InvalidCommitmentColumn {
        index: usize,
    },
    InvalidStageValue {
        field: &'static str,
        index: usize,
    },
    InvalidEvaluationMap {
        index: usize,
    },
    Io {
        message: String,
    },
}

impl fmt::Display for SetupInfoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMagic => write!(f, "invalid setup-info file magic"),
            Self::UnsupportedVersion { found, max } => {
                write!(f, "unsupported setup-info file version {found}, max {max}")
            }
            Self::InvalidSectionCount { found } => {
                write!(f, "invalid setup-info section count {found}")
            }
            Self::InvalidSectionId { found } => {
                write!(f, "invalid setup-info section id {found}")
            }
            Self::UnexpectedTrailingBytes { count } => {
                write!(f, "unexpected trailing bytes in setup-info file: {count}")
            }
            Self::UnexpectedEof {
                offset,
                needed,
                available,
            } => write!(
                f,
                "unexpected end of setup-info file at {offset}, needed {needed}, available {available}"
            ),
            Self::InvalidUtf8 => write!(f, "setup-info string is not valid utf-8"),
            Self::MissingStringTerminator { offset } => {
                write!(f, "missing setup-info string terminator at offset {offset}")
            }
            Self::LengthOverflow => write!(f, "setup-info length overflow"),
            Self::StringContainsNul { value } => {
                write!(f, "setup-info string contains nul byte: {value}")
            }
            Self::InvalidFlag { field, value } => {
                write!(f, "invalid setup-info flag for {field}: {value}")
            }
            Self::MissingSectionWidth { name } => {
                write!(f, "missing setup-info section width: {name}")
            }
            Self::InvalidDomainBits { n_bits, n_bits_ext } => write!(
                f,
                "invalid setup-info domain bits: n_bits {n_bits}, n_bits_ext {n_bits_ext}"
            ),
            Self::InvalidFriSteps => write!(f, "invalid setup-info FRI steps"),
            Self::ConstantColumnCountMismatch { expected, found } => write!(
                f,
                "setup-info constant-column count mismatch: expected {expected}, found {found}"
            ),
            Self::InvalidConstantColumn { index } => {
                write!(
                    f,
                    "invalid setup-info constant-column entry at index {index}"
                )
            }
            Self::InvalidCommitmentColumn { index } => {
                write!(
                    f,
                    "invalid setup-info commitment-column entry at index {index}"
                )
            }
            Self::InvalidStageValue { field, index } => {
                write!(
                    f,
                    "invalid setup-info {field} entry at index {index}"
                )
            }
            Self::InvalidEvaluationMap { index } => {
                write!(f, "invalid setup-info evaluation-map entry at index {index}")
            }
            Self::Io { message } => write!(f, "setup-info io error: {message}"),
        }
    }
}

impl std::error::Error for SetupInfoError {}

impl From<SectionedError> for SetupInfoError {
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

impl UnitSetupInfo {
    pub fn stage_commit_widths(&self) -> Result<Vec<u32>, SetupInfoError> {
        let stage_count = self
            .n_stages
            .checked_add(1)
            .ok_or(SetupInfoError::LengthOverflow)?;
        let mut widths = Vec::with_capacity(u32_to_usize(stage_count)?);
        for stage in 1..=stage_count {
            let name = format!("cm{stage}");
            let width = *self
                .section_widths
                .get(&name)
                .ok_or(SetupInfoError::MissingSectionWidth { name })?;
            widths.push(width);
        }
        Ok(widths)
    }
}

pub fn read_unit_setup_info_file(path: impl AsRef<Path>) -> Result<UnitSetupInfo, SetupInfoError> {
    read_unit_setup_info_binary_file(path)
}

pub fn read_unit_setup_info_binary_file(
    path: impl AsRef<Path>,
) -> Result<UnitSetupInfo, SetupInfoError> {
    let bytes = std::fs::read(path).map_err(|error| SetupInfoError::Io {
        message: error.to_string(),
    })?;
    parse_unit_setup_info(&bytes)
}

pub fn parse_unit_setup_info(bytes: &[u8]) -> Result<UnitSetupInfo, SetupInfoError> {
    let file = parse_sectioned_file(bytes, SETUP_INFO_KIND, SETUP_INFO_VERSION)
        .map_err(SetupInfoError::from)?;
    if file.version == 0 {
        return Err(SetupInfoError::UnsupportedVersion {
            found: file.version,
            max: SETUP_INFO_VERSION,
        });
    }

    if file.sections.len() != 1 {
        return Err(SetupInfoError::InvalidSectionCount {
            found: u32::try_from(file.sections.len()).unwrap_or(u32::MAX),
        });
    }

    let section = &file.sections[0];
    if section.id != SETUP_INFO_SECTION_ID {
        return Err(SetupInfoError::InvalidSectionId { found: section.id });
    }

    parse_unit_setup_info_section(&section.data, file.version)
}

pub fn encode_unit_setup_info(value: &UnitSetupInfo) -> Result<Vec<u8>, SetupInfoError> {
    validate_unit_setup_info(value)?;
    let section = encode_unit_setup_info_section(value)?;
    let file = SectionedFile {
        kind: SETUP_INFO_KIND,
        version: SETUP_INFO_VERSION,
        sections: vec![SectionedSection {
            id: SETUP_INFO_SECTION_ID,
            data: section,
        }],
    };
    encode_sectioned_file(&file).map_err(SetupInfoError::from)
}

fn parse_unit_setup_info_section(
    bytes: &[u8],
    version: u32,
) -> Result<UnitSetupInfo, SetupInfoError> {
    let mut reader = Reader::new(bytes);
    let n_stages = reader.read_u32()?;
    let n_constants = reader.read_u32()?;
    let n_publics = reader.read_optional_u32("n_publics")?;
    let n_constraints = reader.read_optional_u32("n_constraints")?;
    let q_degree = reader.read_u32()?;
    let challenge_count = u32_to_usize(reader.read_u32()?)?;
    let eval_count = u32_to_usize(reader.read_u32()?)?;

    let opening_point_count = read_bounded_count(&mut reader, I64_BYTES)?;
    let mut opening_points = Vec::with_capacity(opening_point_count);
    for _ in 0..opening_point_count {
        opening_points.push(reader.read_i64()?);
    }

    let section_width_count = reader.read_u32()?;
    let mut section_widths = BTreeMap::new();
    for _ in 0..section_width_count {
        let name = reader.read_string()?;
        let width = reader.read_u32()?;
        section_widths.insert(name, width);
    }

    let constant_column_count = read_bounded_count(&mut reader, CONSTANT_COLUMN_MIN_BYTES)?;
    let mut constant_columns = Vec::with_capacity(constant_column_count);
    for _ in 0..constant_column_count {
        let name = reader.read_string()?;
        let stage = reader.read_u32()?;
        let dimension = reader.read_u32()?;
        let pols_map_id = reader.read_u32()?;
        let stage_id = reader.read_u32()?;
        let lengths_count = read_bounded_count(&mut reader, U32_BYTES)?;
        let mut lengths = Vec::with_capacity(lengths_count);
        for _ in 0..lengths_count {
            lengths.push(reader.read_u32()?);
        }
        constant_columns.push(ConstantColumn {
            name,
            stage,
            dimension,
            pols_map_id,
            stage_id,
            lengths,
        });
    }

    let boundary_count = read_bounded_count(&mut reader, BOUNDARY_MIN_BYTES)?;
    let mut boundaries = Vec::with_capacity(boundary_count);
    for _ in 0..boundary_count {
        boundaries.push(Boundary {
            name: reader.read_optional_string("boundary_name")?,
            offset_min: reader.read_optional_i64("boundary_offset_min")?,
            offset_max: reader.read_optional_i64("boundary_offset_max")?,
        });
    }

    let step_count;
    let stark = {
        let n_bits = reader.read_u32()?;
        let n_bits_ext = reader.read_u32()?;
        let n_queries = reader.read_u32()?;
        step_count = read_bounded_count(&mut reader, FRI_STEP_BYTES)?;
        let mut steps = Vec::with_capacity(step_count);
        for _ in 0..step_count {
            steps.push(FriStep {
                n_bits: reader.read_u32()?,
            });
        }
        StarkStruct {
            n_bits,
            n_bits_ext,
            n_queries,
            steps,
            hash_commits: reader.read_bool("hash_commits")?,
            last_level_verification: reader.read_u32()?,
            pow_bits: reader.read_u32()?,
            merkle_tree_arity: reader.read_u32()?,
            verification_hash_type: reader.read_optional_string("verification_hash_type")?,
            transcript_arity: reader.read_optional_u32("transcript_arity")?,
            merkle_tree_custom: reader.read_optional_bool("merkle_tree_custom")?,
        }
    };

    let commitment_columns = if reader.position() == bytes.len() {
        Vec::new()
    } else {
        read_commitment_columns(&mut reader)?
    };
    let (unit_value_map, group_value_map) = if reader.position() == bytes.len() {
        (Vec::new(), Vec::new())
    } else {
        (
            read_stage_values(&mut reader)?,
            read_stage_values(&mut reader)?,
        )
    };
    let evaluation_map = if reader.position() == bytes.len() || version < 3 {
        default_evaluation_map(eval_count)
    } else {
        read_evaluation_map(&mut reader)?
    };

    if reader.position() != bytes.len() {
        return Err(SetupInfoError::UnexpectedTrailingBytes {
            count: bytes.len() - reader.position(),
        });
    }

    let info = UnitSetupInfo {
        n_stages,
        n_constants,
        constant_columns,
        n_publics,
        n_constraints,
        q_degree,
        opening_points,
        section_widths,
        challenge_count,
        eval_count,
        evaluation_map,
        boundaries,
        commitment_columns,
        unit_value_map,
        group_value_map,
        stark,
    };
    validate_unit_setup_info(&info)?;
    Ok(info)
}

fn encode_unit_setup_info_section(value: &UnitSetupInfo) -> Result<Vec<u8>, SetupInfoError> {
    let mut section = Vec::new();
    write_u32(&mut section, value.n_stages);
    write_u32(&mut section, value.n_constants);
    write_optional_u32(&mut section, value.n_publics);
    write_optional_u32(&mut section, value.n_constraints);
    write_u32(&mut section, value.q_degree);
    write_u32(&mut section, usize_to_u32(value.challenge_count)?);
    write_u32(&mut section, usize_to_u32(value.eval_count)?);

    write_u32(&mut section, usize_to_u32(value.opening_points.len())?);
    for point in &value.opening_points {
        write_i64(&mut section, *point);
    }

    write_u32(&mut section, usize_to_u32(value.section_widths.len())?);
    for (name, width) in &value.section_widths {
        write_string(&mut section, name)?;
        write_u32(&mut section, *width);
    }

    write_u32(&mut section, usize_to_u32(value.constant_columns.len())?);
    for column in &value.constant_columns {
        write_string(&mut section, &column.name)?;
        write_u32(&mut section, column.stage);
        write_u32(&mut section, column.dimension);
        write_u32(&mut section, column.pols_map_id);
        write_u32(&mut section, column.stage_id);
        write_u32(&mut section, usize_to_u32(column.lengths.len())?);
        for length in &column.lengths {
            write_u32(&mut section, *length);
        }
    }

    write_u32(&mut section, usize_to_u32(value.boundaries.len())?);
    for boundary in &value.boundaries {
        write_optional_string(&mut section, boundary.name.as_deref())?;
        write_optional_i64(&mut section, boundary.offset_min);
        write_optional_i64(&mut section, boundary.offset_max);
    }

    write_u32(&mut section, value.stark.n_bits);
    write_u32(&mut section, value.stark.n_bits_ext);
    write_u32(&mut section, value.stark.n_queries);
    write_u32(&mut section, usize_to_u32(value.stark.steps.len())?);
    for step in &value.stark.steps {
        write_u32(&mut section, step.n_bits);
    }
    write_bool(&mut section, value.stark.hash_commits);
    write_u32(&mut section, value.stark.last_level_verification);
    write_u32(&mut section, value.stark.pow_bits);
    write_u32(&mut section, value.stark.merkle_tree_arity);
    write_optional_string(&mut section, value.stark.verification_hash_type.as_deref())?;
    write_optional_u32(&mut section, value.stark.transcript_arity);
    write_optional_bool(&mut section, value.stark.merkle_tree_custom);

    write_u32(&mut section, usize_to_u32(value.commitment_columns.len())?);
    for column in &value.commitment_columns {
        write_string(&mut section, &column.name)?;
        write_u32(&mut section, column.stage);
        write_u32(&mut section, column.dimension);
        write_u32(&mut section, column.pols_map_id);
        write_u32(&mut section, column.stage_id);
        write_u32(&mut section, column.stage_position);
        write_bool(&mut section, column.intermediate);
        write_u32(&mut section, usize_to_u32(column.lengths.len())?);
        for length in &column.lengths {
            write_u32(&mut section, *length);
        }
    }

    write_stage_values(&mut section, &value.unit_value_map)?;
    write_stage_values(&mut section, &value.group_value_map)?;
    write_evaluation_map(&mut section, &value.evaluation_map)?;

    Ok(section)
}

fn default_evaluation_map(count: usize) -> Vec<EvaluationMapEntry> {
    vec![EvaluationMapEntry::default(); count]
}

fn read_commitment_columns(
    reader: &mut Reader<'_>,
) -> Result<Vec<CommitmentColumn>, SetupInfoError> {
    let commitment_column_count = read_bounded_count(reader, COMMITMENT_COLUMN_MIN_BYTES)?;
    let mut commitment_columns = Vec::with_capacity(commitment_column_count);
    for _ in 0..commitment_column_count {
        let name = reader.read_string()?;
        let stage = reader.read_u32()?;
        let dimension = reader.read_u32()?;
        let pols_map_id = reader.read_u32()?;
        let stage_id = reader.read_u32()?;
        let stage_position = reader.read_u32()?;
        let intermediate = reader.read_bool("commitment_column_intermediate")?;
        let lengths_count = read_bounded_count(reader, U32_BYTES)?;
        let mut lengths = Vec::with_capacity(lengths_count);
        for _ in 0..lengths_count {
            lengths.push(reader.read_u32()?);
        }
        commitment_columns.push(CommitmentColumn {
            name,
            stage,
            dimension,
            pols_map_id,
            stage_id,
            stage_position,
            intermediate,
            lengths,
        });
    }
    Ok(commitment_columns)
}

fn read_stage_values(reader: &mut Reader<'_>) -> Result<Vec<StageValue>, SetupInfoError> {
    let value_count = read_bounded_count(reader, STAGE_VALUE_MIN_BYTES)?;
    let mut values = Vec::with_capacity(value_count);
    for _ in 0..value_count {
        let name = reader.read_string()?;
        let stage = reader.read_u32()?;
        let lengths_count = read_bounded_count(reader, U32_BYTES)?;
        let mut lengths = Vec::with_capacity(lengths_count);
        for _ in 0..lengths_count {
            lengths.push(reader.read_u32()?);
        }
        values.push(StageValue {
            name,
            stage,
            lengths,
        });
    }
    Ok(values)
}

fn read_evaluation_map(reader: &mut Reader<'_>) -> Result<Vec<EvaluationMapEntry>, SetupInfoError> {
    let count = read_bounded_count(reader, EVALUATION_MAP_ENTRY_MIN_BYTES)?;
    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        let kind = match reader.read_u8()? {
            0 => EvaluationMapKind::Constant,
            1 => EvaluationMapKind::Commitment,
            2 => EvaluationMapKind::Custom,
            value => {
                return Err(SetupInfoError::InvalidFlag {
                    field: "evMap",
                    value,
                })
            }
        };
        let entry = EvaluationMapEntry {
            kind,
            id: reader.read_u32()?,
            prime: reader.read_i64()?,
            opening_position: reader.read_u32()?,
            commit_id: reader.read_optional_u32("evMap_commit_id")?,
        };
        entries.push(entry);
    }
    Ok(entries)
}

fn write_stage_values(out: &mut Vec<u8>, values: &[StageValue]) -> Result<(), SetupInfoError> {
    write_u32(out, usize_to_u32(values.len())?);
    for value in values {
        write_string(out, &value.name)?;
        write_u32(out, value.stage);
        write_u32(out, usize_to_u32(value.lengths.len())?);
        for length in &value.lengths {
            write_u32(out, *length);
        }
    }
    Ok(())
}

fn write_evaluation_map(
    out: &mut Vec<u8>,
    values: &[EvaluationMapEntry],
) -> Result<(), SetupInfoError> {
    write_u32(out, usize_to_u32(values.len())?);
    for entry in values {
        out.push(match entry.kind {
            EvaluationMapKind::Constant => 0,
            EvaluationMapKind::Commitment => 1,
            EvaluationMapKind::Custom => 2,
        });
        write_u32(out, entry.id);
        write_i64(out, entry.prime);
        write_u32(out, entry.opening_position);
        write_optional_u32(out, entry.commit_id);
    }
    Ok(())
}

fn validate_constant_columns(
    n_constants: u32,
    columns: &[ConstantColumn],
) -> Result<(), SetupInfoError> {
    if columns.is_empty() {
        return Ok(());
    }
    let expected = n_constants;
    let constant_width = u32_to_usize(n_constants)?;
    let mut seen = vec![false; constant_width];
    for (index, column) in columns.iter().enumerate() {
        let id = u32_to_usize(column.pols_map_id)?;
        let dimension = u32_to_usize(column.dimension)?;
        let end = id
            .checked_add(dimension)
            .ok_or(SetupInfoError::InvalidConstantColumn { index })?;
        if column.stage != 0
            || column.dimension == 0
            || end > seen.len()
            || seen[id..end].iter().any(|occupied| *occupied)
            || column.stage_id != column.pols_map_id
        {
            return Err(SetupInfoError::InvalidConstantColumn { index });
        }
        seen[id..end].fill(true);
    }
    let occupied = seen.iter().filter(|occupied| **occupied).count();
    if occupied != constant_width {
        return Err(SetupInfoError::ConstantColumnCountMismatch {
            expected,
            found: occupied,
        });
    }

    Ok(())
}

fn validate_commitment_columns(info: &UnitSetupInfo) -> Result<(), SetupInfoError> {
    let max_stage = info
        .n_stages
        .checked_add(1)
        .ok_or(SetupInfoError::LengthOverflow)?;
    for (index, column) in info.commitment_columns.iter().enumerate() {
        if column.stage == 0 || column.stage > max_stage || column.dimension == 0 {
            return Err(SetupInfoError::InvalidCommitmentColumn { index });
        }
        let name = format!("cm{}", column.stage);
        let width = *info
            .section_widths
            .get(&name)
            .ok_or(SetupInfoError::MissingSectionWidth { name: name.clone() })?;
        let Some(end) = column.stage_position.checked_add(column.dimension) else {
            return Err(SetupInfoError::InvalidCommitmentColumn { index });
        };
        if end > width {
            return Err(SetupInfoError::InvalidCommitmentColumn { index });
        }
    }
    Ok(())
}

fn validate_stage_values(
    field: &'static str,
    values: &[StageValue],
    max_stage: u32,
) -> Result<(), SetupInfoError> {
    for (index, value) in values.iter().enumerate() {
        if value.stage == 0
            || value.stage > max_stage
            || value.lengths.iter().any(|length| *length == 0)
        {
            return Err(SetupInfoError::InvalidStageValue { field, index });
        }
    }
    Ok(())
}

fn validate_domains(stark: &StarkStruct) -> Result<(), SetupInfoError> {
    if stark.n_bits_ext < stark.n_bits {
        return Err(SetupInfoError::InvalidDomainBits {
            n_bits: stark.n_bits,
            n_bits_ext: stark.n_bits_ext,
        });
    }

    let Some(first_step) = stark.steps.first() else {
        return Err(SetupInfoError::InvalidFriSteps);
    };
    if first_step.n_bits != stark.n_bits_ext {
        return Err(SetupInfoError::InvalidFriSteps);
    }

    for pair in stark.steps.windows(2) {
        if pair[1].n_bits >= pair[0].n_bits {
            return Err(SetupInfoError::InvalidFriSteps);
        }
    }

    Ok(())
}

fn validate_unit_setup_info(info: &UnitSetupInfo) -> Result<(), SetupInfoError> {
    validate_constant_columns(info.n_constants, &info.constant_columns)?;
    validate_commitment_columns(info)?;
    let max_stage = info
        .n_stages
        .checked_add(1)
        .ok_or(SetupInfoError::LengthOverflow)?;
    validate_stage_values("unit-value-map", &info.unit_value_map, max_stage)?;
    validate_stage_values("group-value-map", &info.group_value_map, max_stage)?;
    validate_domains(&info.stark)?;
    info.stage_commit_widths()?;
    Ok(())
}

fn usize_to_u32(value: usize) -> Result<u32, SetupInfoError> {
    u32::try_from(value).map_err(|_| SetupInfoError::LengthOverflow)
}

fn read_bounded_count(
    reader: &mut Reader<'_>,
    record_min_bytes: usize,
) -> Result<usize, SetupInfoError> {
    let count = u32_to_usize(reader.read_u32()?)?;
    reader.require_items(count, record_min_bytes)?;
    Ok(count)
}

fn u32_to_usize(value: u32) -> Result<usize, SetupInfoError> {
    usize::try_from(value).map_err(|_| SetupInfoError::LengthOverflow)
}

fn write_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_i64(out: &mut Vec<u8>, value: i64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_bool(out: &mut Vec<u8>, value: bool) {
    out.push(u8::from(value));
}

fn write_optional_u32(out: &mut Vec<u8>, value: Option<u32>) {
    match value {
        Some(value) => {
            out.push(1);
            write_u32(out, value);
        }
        None => out.push(0),
    }
}

fn write_optional_i64(out: &mut Vec<u8>, value: Option<i64>) {
    match value {
        Some(value) => {
            out.push(1);
            write_i64(out, value);
        }
        None => out.push(0),
    }
}

fn write_optional_bool(out: &mut Vec<u8>, value: Option<bool>) {
    match value {
        Some(value) => {
            out.push(1);
            write_bool(out, value);
        }
        None => out.push(0),
    }
}

fn write_optional_string(out: &mut Vec<u8>, value: Option<&str>) -> Result<(), SetupInfoError> {
    match value {
        Some(value) => {
            out.push(1);
            write_string(out, value)?;
        }
        None => out.push(0),
    }
    Ok(())
}

fn write_string(out: &mut Vec<u8>, value: &str) -> Result<(), SetupInfoError> {
    if value.as_bytes().contains(&0) {
        return Err(SetupInfoError::StringContainsNul {
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

    fn read_exact(&mut self, count: usize) -> Result<&'a [u8], SetupInfoError> {
        let end = self
            .offset
            .checked_add(count)
            .ok_or(SetupInfoError::LengthOverflow)?;
        if end > self.bytes.len() {
            return Err(SetupInfoError::UnexpectedEof {
                offset: self.offset,
                needed: count,
                available: self.bytes.len().saturating_sub(self.offset),
            });
        }
        let out = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(out)
    }

    fn require_items(&self, count: usize, item_bytes: usize) -> Result<(), SetupInfoError> {
        let needed = count
            .checked_mul(item_bytes)
            .ok_or(SetupInfoError::LengthOverflow)?;
        let end = self
            .offset
            .checked_add(needed)
            .ok_or(SetupInfoError::LengthOverflow)?;
        if end > self.bytes.len() {
            return Err(SetupInfoError::UnexpectedEof {
                offset: self.offset,
                needed,
                available: self.bytes.len().saturating_sub(self.offset),
            });
        }
        Ok(())
    }

    fn read_u8(&mut self) -> Result<u8, SetupInfoError> {
        Ok(self.read_exact(1)?[0])
    }

    fn read_u32(&mut self) -> Result<u32, SetupInfoError> {
        let bytes = self.read_exact(4)?;
        Ok(u32::from_le_bytes(
            bytes.try_into().expect("slice length checked"),
        ))
    }

    fn read_i64(&mut self) -> Result<i64, SetupInfoError> {
        let bytes = self.read_exact(8)?;
        Ok(i64::from_le_bytes(
            bytes.try_into().expect("slice length checked"),
        ))
    }

    fn read_bool(&mut self, field: &'static str) -> Result<bool, SetupInfoError> {
        match self.read_u8()? {
            0 => Ok(false),
            1 => Ok(true),
            value => Err(SetupInfoError::InvalidFlag { field, value }),
        }
    }

    fn read_optional_u32(&mut self, field: &'static str) -> Result<Option<u32>, SetupInfoError> {
        match self.read_u8()? {
            0 => Ok(None),
            1 => Ok(Some(self.read_u32()?)),
            value => Err(SetupInfoError::InvalidFlag { field, value }),
        }
    }

    fn read_optional_i64(&mut self, field: &'static str) -> Result<Option<i64>, SetupInfoError> {
        match self.read_u8()? {
            0 => Ok(None),
            1 => Ok(Some(self.read_i64()?)),
            value => Err(SetupInfoError::InvalidFlag { field, value }),
        }
    }

    fn read_optional_bool(&mut self, field: &'static str) -> Result<Option<bool>, SetupInfoError> {
        match self.read_u8()? {
            0 => Ok(None),
            1 => Ok(Some(self.read_bool(field)?)),
            value => Err(SetupInfoError::InvalidFlag { field, value }),
        }
    }

    fn read_optional_string(
        &mut self,
        field: &'static str,
    ) -> Result<Option<String>, SetupInfoError> {
        match self.read_u8()? {
            0 => Ok(None),
            1 => Ok(Some(self.read_string()?)),
            value => Err(SetupInfoError::InvalidFlag { field, value }),
        }
    }

    fn read_string(&mut self) -> Result<String, SetupInfoError> {
        let start = self.offset;
        let Some(relative_end) = self.bytes[start..].iter().position(|byte| *byte == 0) else {
            return Err(SetupInfoError::MissingStringTerminator { offset: start });
        };
        let end = start
            .checked_add(relative_end)
            .ok_or(SetupInfoError::LengthOverflow)?;
        self.offset = end + 1;
        String::from_utf8(self.bytes[start..end].to_vec()).map_err(|_| SetupInfoError::InvalidUtf8)
    }
}
