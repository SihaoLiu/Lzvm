use std::fmt;
use std::path::Path;

use crate::sectioned::{
    encode_sectioned_file, parse_sectioned_file, SectionedError, SectionedFile, SectionedSection,
};
use crate::setup_info::{SetupInfoError, UnitSetupInfo};

const PCS_PLAN_KIND: [u8; 4] = *b"pcsp";
const PCS_PLAN_VERSION: u32 = 2;
const PCS_PLAN_SECTION_ID: u32 = 1;
const STAGE_COMMIT_WIDTH_BYTES: usize = 4;
const OPENING_POINT_BYTES: usize = 8;
const FRI_LAYER_BYTES: usize = 4 + 4 + 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PcsSetupPlan {
    pub base_domain_bits: u32,
    pub extended_domain_bits: u32,
    pub base_domain_size: u64,
    pub extended_domain_size: u64,
    pub blowup_factor: u64,
    pub query_count: u32,
    pub proof_of_work_bits: u32,
    pub merkle_tree_arity: u32,
    pub transcript_arity: Option<u32>,
    pub hash_commits: bool,
    pub constant_width: u32,
    pub stage_commit_widths: Vec<u32>,
    pub opening_points: Vec<i64>,
    pub fri_layers: Vec<PcsFriLayer>,
    pub final_layer_bits: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PcsFriLayer {
    pub input_bits: u32,
    pub output_bits: u32,
    pub folding_factor: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PcsPlanError {
    SetupInfo(SetupInfoError),
    Sectioned(SectionedError),
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
    InvalidFlag {
        field: &'static str,
        value: u8,
    },
    DomainTooLarge {
        bits: u32,
    },
    InvalidDomainBits {
        base_bits: u32,
        extended_bits: u32,
    },
    InvalidDomainSize {
        field: &'static str,
    },
    InvalidBlowupFactor,
    InvalidQueryCount,
    InvalidMerkleTreeArity {
        arity: u32,
    },
    EmptyFriSchedule,
    InvalidFirstFriLayer {
        expected: u32,
        found: u32,
    },
    InvalidFriLayer {
        input_bits: u32,
        output_bits: u32,
    },
    FinalLayerMismatch {
        expected: u32,
        found: u32,
    },
    LengthOverflow,
    Io {
        message: String,
    },
}

impl fmt::Display for PcsPlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SetupInfo(error) => write!(f, "PCS setup plan metadata error: {error}"),
            Self::Sectioned(error) => write!(f, "PCS setup plan container error: {error}"),
            Self::InvalidSectionCount { found } => {
                write!(f, "invalid PCS setup plan section count {found}")
            }
            Self::InvalidSectionId { found } => {
                write!(f, "invalid PCS setup plan section id {found}")
            }
            Self::UnexpectedTrailingBytes { count } => {
                write!(f, "unexpected trailing bytes in PCS setup plan: {count}")
            }
            Self::UnexpectedEof {
                offset,
                needed,
                available,
            } => write!(
                f,
                "unexpected end of PCS setup plan at {offset}, needed {needed}, available {available}"
            ),
            Self::InvalidFlag { field, value } => {
                write!(f, "invalid PCS setup plan flag for {field}: {value}")
            }
            Self::DomainTooLarge { bits } => {
                write!(f, "PCS setup plan domain is too large: {bits}")
            }
            Self::InvalidDomainBits {
                base_bits,
                extended_bits,
            } => write!(
                f,
                "PCS setup plan domain bits are invalid: base {base_bits}, extended {extended_bits}"
            ),
            Self::InvalidDomainSize { field } => {
                write!(f, "PCS setup plan domain size is invalid: {field}")
            }
            Self::InvalidBlowupFactor => write!(f, "PCS setup plan blowup factor is invalid"),
            Self::InvalidQueryCount => write!(f, "PCS setup plan query count is invalid"),
            Self::InvalidMerkleTreeArity { arity } => {
                write!(f, "PCS setup plan merkle-tree arity is invalid: {arity}")
            }
            Self::EmptyFriSchedule => write!(f, "PCS setup plan FRI schedule is empty"),
            Self::InvalidFirstFriLayer { expected, found } => write!(
                f,
                "PCS setup plan first FRI layer mismatch: expected {expected}, found {found}"
            ),
            Self::InvalidFriLayer {
                input_bits,
                output_bits,
            } => write!(
                f,
                "PCS setup plan invalid FRI layer: input {input_bits}, output {output_bits}"
            ),
            Self::FinalLayerMismatch { expected, found } => write!(
                f,
                "PCS setup plan final layer mismatch: expected {expected}, found {found}"
            ),
            Self::LengthOverflow => write!(f, "PCS setup plan length overflow"),
            Self::Io { message } => write!(f, "PCS setup plan io error: {message}"),
        }
    }
}

impl std::error::Error for PcsPlanError {}

impl From<SetupInfoError> for PcsPlanError {
    fn from(error: SetupInfoError) -> Self {
        Self::SetupInfo(error)
    }
}

impl From<SectionedError> for PcsPlanError {
    fn from(error: SectionedError) -> Self {
        Self::Sectioned(error)
    }
}

pub fn read_pcs_setup_plan_file(path: impl AsRef<Path>) -> Result<PcsSetupPlan, PcsPlanError> {
    let bytes = std::fs::read(path).map_err(|error| PcsPlanError::Io {
        message: error.to_string(),
    })?;
    parse_pcs_setup_plan(&bytes)
}

pub fn parse_pcs_setup_plan(bytes: &[u8]) -> Result<PcsSetupPlan, PcsPlanError> {
    let file = parse_sectioned_file(bytes, PCS_PLAN_KIND, PCS_PLAN_VERSION)?;
    if file.version != PCS_PLAN_VERSION {
        return Err(PcsPlanError::Sectioned(
            SectionedError::UnsupportedVersion {
                found: file.version,
                max: PCS_PLAN_VERSION,
            },
        ));
    }
    if file.sections.len() != 1 {
        return Err(PcsPlanError::InvalidSectionCount {
            found: u32::try_from(file.sections.len()).unwrap_or(u32::MAX),
        });
    }
    let section = &file.sections[0];
    if section.id != PCS_PLAN_SECTION_ID {
        return Err(PcsPlanError::InvalidSectionId { found: section.id });
    }
    parse_pcs_setup_plan_section(&section.data)
}

pub fn encode_pcs_setup_plan(value: &PcsSetupPlan) -> Result<Vec<u8>, PcsPlanError> {
    validate_pcs_setup_plan(value)?;
    let file = SectionedFile {
        kind: PCS_PLAN_KIND,
        version: PCS_PLAN_VERSION,
        sections: vec![SectionedSection {
            id: PCS_PLAN_SECTION_ID,
            data: encode_pcs_setup_plan_section(value)?,
        }],
    };
    encode_sectioned_file(&file).map_err(PcsPlanError::Sectioned)
}

pub fn derive_pcs_setup_plan(setup: &UnitSetupInfo) -> Result<PcsSetupPlan, PcsPlanError> {
    if setup.stark.n_queries == 0 {
        return Err(PcsPlanError::InvalidQueryCount);
    }
    if setup.stark.merkle_tree_arity < 2 {
        return Err(PcsPlanError::InvalidMerkleTreeArity {
            arity: setup.stark.merkle_tree_arity,
        });
    }
    if setup.stark.n_bits_ext < setup.stark.n_bits {
        return Err(PcsPlanError::InvalidDomainBits {
            base_bits: setup.stark.n_bits,
            extended_bits: setup.stark.n_bits_ext,
        });
    }

    let Some(first_step) = setup.stark.steps.first() else {
        return Err(PcsPlanError::EmptyFriSchedule);
    };
    if first_step.n_bits != setup.stark.n_bits_ext {
        return Err(PcsPlanError::InvalidFirstFriLayer {
            expected: setup.stark.n_bits_ext,
            found: first_step.n_bits,
        });
    }

    let base_domain_size = domain_size(setup.stark.n_bits)?;
    let extended_domain_size = domain_size(setup.stark.n_bits_ext)?;
    let blowup_factor = domain_size(setup.stark.n_bits_ext - setup.stark.n_bits)?;
    let mut fri_layers = Vec::with_capacity(setup.stark.steps.len().saturating_sub(1));
    for pair in setup.stark.steps.windows(2) {
        let input_bits = pair[0].n_bits;
        let output_bits = pair[1].n_bits;
        if output_bits >= input_bits {
            return Err(PcsPlanError::InvalidFriLayer {
                input_bits,
                output_bits,
            });
        }
        fri_layers.push(PcsFriLayer {
            input_bits,
            output_bits,
            folding_factor: domain_size(input_bits - output_bits)?,
        });
    }

    Ok(PcsSetupPlan {
        base_domain_bits: setup.stark.n_bits,
        extended_domain_bits: setup.stark.n_bits_ext,
        base_domain_size,
        extended_domain_size,
        blowup_factor,
        query_count: setup.stark.n_queries,
        proof_of_work_bits: setup.stark.pow_bits,
        merkle_tree_arity: setup.stark.merkle_tree_arity,
        transcript_arity: setup.stark.transcript_arity,
        hash_commits: setup.stark.hash_commits,
        constant_width: setup.n_constants,
        stage_commit_widths: setup.stage_commit_widths()?,
        opening_points: setup.opening_points.clone(),
        fri_layers,
        final_layer_bits: setup
            .stark
            .steps
            .last()
            .expect("first step was already checked")
            .n_bits,
    })
}

fn domain_size(bits: u32) -> Result<u64, PcsPlanError> {
    1_u64
        .checked_shl(bits)
        .ok_or(PcsPlanError::DomainTooLarge { bits })
}

fn parse_pcs_setup_plan_section(bytes: &[u8]) -> Result<PcsSetupPlan, PcsPlanError> {
    let mut reader = Reader::new(bytes);
    let base_domain_bits = reader.read_u32()?;
    let extended_domain_bits = reader.read_u32()?;
    let base_domain_size = reader.read_u64()?;
    let extended_domain_size = reader.read_u64()?;
    let blowup_factor = reader.read_u64()?;
    let query_count = reader.read_u32()?;
    let proof_of_work_bits = reader.read_u32()?;
    let merkle_tree_arity = reader.read_u32()?;
    let transcript_arity = reader.read_optional_u32("transcript_arity")?;
    let hash_commits = reader.read_bool("hash_commits")?;
    let constant_width = reader.read_u32()?;

    let stage_commit_width_count = u32_to_usize(reader.read_u32()?)?;
    if stage_commit_width_count > reader.remaining_len() / STAGE_COMMIT_WIDTH_BYTES {
        return Err(PcsPlanError::LengthOverflow);
    }
    let mut stage_commit_widths = Vec::with_capacity(stage_commit_width_count);
    for _ in 0..stage_commit_width_count {
        stage_commit_widths.push(reader.read_u32()?);
    }

    let opening_point_count = u32_to_usize(reader.read_u32()?)?;
    if opening_point_count > reader.remaining_len() / OPENING_POINT_BYTES {
        return Err(PcsPlanError::LengthOverflow);
    }
    let mut opening_points = Vec::with_capacity(opening_point_count);
    for _ in 0..opening_point_count {
        opening_points.push(reader.read_i64()?);
    }

    let fri_layer_count = u32_to_usize(reader.read_u32()?)?;
    if fri_layer_count > reader.remaining_len() / FRI_LAYER_BYTES {
        return Err(PcsPlanError::LengthOverflow);
    }
    let mut fri_layers = Vec::with_capacity(fri_layer_count);
    for _ in 0..fri_layer_count {
        fri_layers.push(PcsFriLayer {
            input_bits: reader.read_u32()?,
            output_bits: reader.read_u32()?,
            folding_factor: reader.read_u64()?,
        });
    }
    let final_layer_bits = reader.read_u32()?;

    if reader.position() != bytes.len() {
        return Err(PcsPlanError::UnexpectedTrailingBytes {
            count: bytes.len() - reader.position(),
        });
    }

    let plan = PcsSetupPlan {
        base_domain_bits,
        extended_domain_bits,
        base_domain_size,
        extended_domain_size,
        blowup_factor,
        query_count,
        proof_of_work_bits,
        merkle_tree_arity,
        transcript_arity,
        hash_commits,
        constant_width,
        stage_commit_widths,
        opening_points,
        fri_layers,
        final_layer_bits,
    };
    validate_pcs_setup_plan(&plan)?;
    Ok(plan)
}

fn encode_pcs_setup_plan_section(value: &PcsSetupPlan) -> Result<Vec<u8>, PcsPlanError> {
    let mut section = Vec::new();
    write_u32(&mut section, value.base_domain_bits);
    write_u32(&mut section, value.extended_domain_bits);
    write_u64(&mut section, value.base_domain_size);
    write_u64(&mut section, value.extended_domain_size);
    write_u64(&mut section, value.blowup_factor);
    write_u32(&mut section, value.query_count);
    write_u32(&mut section, value.proof_of_work_bits);
    write_u32(&mut section, value.merkle_tree_arity);
    write_optional_u32(&mut section, value.transcript_arity);
    write_bool(&mut section, value.hash_commits);
    write_u32(&mut section, value.constant_width);
    write_u32(&mut section, usize_to_u32(value.stage_commit_widths.len())?);
    for width in &value.stage_commit_widths {
        write_u32(&mut section, *width);
    }
    write_u32(&mut section, usize_to_u32(value.opening_points.len())?);
    for point in &value.opening_points {
        write_i64(&mut section, *point);
    }
    write_u32(&mut section, usize_to_u32(value.fri_layers.len())?);
    for layer in &value.fri_layers {
        write_u32(&mut section, layer.input_bits);
        write_u32(&mut section, layer.output_bits);
        write_u64(&mut section, layer.folding_factor);
    }
    write_u32(&mut section, value.final_layer_bits);
    Ok(section)
}

fn validate_pcs_setup_plan(value: &PcsSetupPlan) -> Result<(), PcsPlanError> {
    if value.query_count == 0 {
        return Err(PcsPlanError::InvalidQueryCount);
    }
    if value.merkle_tree_arity < 2 {
        return Err(PcsPlanError::InvalidMerkleTreeArity {
            arity: value.merkle_tree_arity,
        });
    }
    if value.extended_domain_bits < value.base_domain_bits {
        return Err(PcsPlanError::InvalidDomainBits {
            base_bits: value.base_domain_bits,
            extended_bits: value.extended_domain_bits,
        });
    }
    if domain_size(value.base_domain_bits)? != value.base_domain_size {
        return Err(PcsPlanError::InvalidDomainSize {
            field: "base_domain_size",
        });
    }
    if domain_size(value.extended_domain_bits)? != value.extended_domain_size {
        return Err(PcsPlanError::InvalidDomainSize {
            field: "extended_domain_size",
        });
    }
    let blowup_bits = value.extended_domain_bits - value.base_domain_bits;
    if domain_size(blowup_bits)? != value.blowup_factor {
        return Err(PcsPlanError::InvalidBlowupFactor);
    }
    let Some(first_layer) = value.fri_layers.first() else {
        return Err(PcsPlanError::EmptyFriSchedule);
    };
    if first_layer.input_bits != value.extended_domain_bits {
        return Err(PcsPlanError::InvalidFirstFriLayer {
            expected: value.extended_domain_bits,
            found: first_layer.input_bits,
        });
    }
    for layer in &value.fri_layers {
        if layer.output_bits >= layer.input_bits {
            return Err(PcsPlanError::InvalidFriLayer {
                input_bits: layer.input_bits,
                output_bits: layer.output_bits,
            });
        }
        let expected = domain_size(layer.input_bits - layer.output_bits)?;
        if layer.folding_factor != expected {
            return Err(PcsPlanError::InvalidFriLayer {
                input_bits: layer.input_bits,
                output_bits: layer.output_bits,
            });
        }
    }
    for pair in value.fri_layers.windows(2) {
        if pair[0].output_bits != pair[1].input_bits {
            return Err(PcsPlanError::InvalidFriLayer {
                input_bits: pair[1].input_bits,
                output_bits: pair[1].output_bits,
            });
        }
    }
    let expected_final_layer = value
        .fri_layers
        .last()
        .expect("first layer was checked")
        .output_bits;
    if value.final_layer_bits != expected_final_layer {
        return Err(PcsPlanError::FinalLayerMismatch {
            expected: expected_final_layer,
            found: value.final_layer_bits,
        });
    }
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

    fn remaining_len(&self) -> usize {
        self.bytes.len() - self.offset
    }

    fn read_u8(&mut self) -> Result<u8, PcsPlanError> {
        let bytes = self.read_exact(1)?;
        Ok(bytes[0])
    }

    fn read_u32(&mut self) -> Result<u32, PcsPlanError> {
        let bytes = self.read_exact(4)?;
        Ok(u32::from_le_bytes(
            bytes.try_into().expect("slice length checked"),
        ))
    }

    fn read_u64(&mut self) -> Result<u64, PcsPlanError> {
        let bytes = self.read_exact(8)?;
        Ok(u64::from_le_bytes(
            bytes.try_into().expect("slice length checked"),
        ))
    }

    fn read_i64(&mut self) -> Result<i64, PcsPlanError> {
        let bytes = self.read_exact(8)?;
        Ok(i64::from_le_bytes(
            bytes.try_into().expect("slice length checked"),
        ))
    }

    fn read_optional_u32(&mut self, field: &'static str) -> Result<Option<u32>, PcsPlanError> {
        match self.read_u8()? {
            0 => Ok(None),
            1 => Ok(Some(self.read_u32()?)),
            value => Err(PcsPlanError::InvalidFlag { field, value }),
        }
    }

    fn read_bool(&mut self, field: &'static str) -> Result<bool, PcsPlanError> {
        match self.read_u8()? {
            0 => Ok(false),
            1 => Ok(true),
            value => Err(PcsPlanError::InvalidFlag { field, value }),
        }
    }

    fn read_exact(&mut self, count: usize) -> Result<&'a [u8], PcsPlanError> {
        let end = self
            .offset
            .checked_add(count)
            .ok_or(PcsPlanError::LengthOverflow)?;
        if end > self.bytes.len() {
            return Err(PcsPlanError::UnexpectedEof {
                offset: self.offset,
                needed: count,
                available: self.bytes.len().saturating_sub(self.offset),
            });
        }
        let out = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(out)
    }
}

fn write_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_i64(out: &mut Vec<u8>, value: i64) {
    out.extend_from_slice(&value.to_le_bytes());
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

fn write_bool(out: &mut Vec<u8>, value: bool) {
    out.push(u8::from(value));
}

fn usize_to_u32(value: usize) -> Result<u32, PcsPlanError> {
    u32::try_from(value).map_err(|_| PcsPlanError::LengthOverflow)
}

fn u32_to_usize(value: u32) -> Result<usize, PcsPlanError> {
    usize::try_from(value).map_err(|_| PcsPlanError::LengthOverflow)
}
