use std::fmt;
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::constant_tree::{ConstantTree, ConstantTreeError};
use crate::pcs_plan::{encode_pcs_setup_plan, PcsPlanError, PcsSetupPlan};
use crate::sectioned::{
    encode_sectioned_file, parse_sectioned_file, SectionedError, SectionedFile, SectionedSection,
};
use crate::verification_key::VerificationKeyRoot;

const PCS_MATERIAL_KIND: [u8; 4] = *b"pcsm";
const PCS_MATERIAL_VERSION: u32 = 1;
const PCS_MATERIAL_SECTION_ID: u32 = 1;
const DIGEST_BYTES: usize = 32;
const ROOT_WORDS: usize = 4;
const ENCODED_BYTES: usize = DIGEST_BYTES * 3 + ROOT_WORDS * 8 + 4 * 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PcsSetupMaterial {
    pub plan_digest: [u8; 32],
    pub fixed_column_digest: [u8; 32],
    pub constant_tree_digest: [u8; 32],
    pub constant_tree_root: [u64; 4],
    pub fixed_byte_count: u64,
    pub constant_tree_byte_count: u64,
    pub leaf_byte_count: u64,
    pub node_byte_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PcsSetupMaterialError {
    Sectioned(SectionedError),
    PcsPlan(PcsPlanError),
    ConstantTree(ConstantTreeError),
    InvalidSectionCount { found: u32 },
    InvalidSectionId { found: u32 },
    InvalidPayloadLength { expected: usize, found: usize },
    InvalidRootWordCount { found: usize },
    LengthOverflow,
    Io { message: String },
}

impl fmt::Display for PcsSetupMaterialError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sectioned(error) => write!(f, "PCS setup material container error: {error}"),
            Self::PcsPlan(error) => write!(f, "PCS setup material plan error: {error}"),
            Self::ConstantTree(error) => {
                write!(f, "PCS setup material constant-tree error: {error}")
            }
            Self::InvalidSectionCount { found } => {
                write!(f, "invalid PCS setup material section count {found}")
            }
            Self::InvalidSectionId { found } => {
                write!(f, "invalid PCS setup material section id {found}")
            }
            Self::InvalidPayloadLength { expected, found } => write!(
                f,
                "invalid PCS setup material payload length: expected {expected}, found {found}"
            ),
            Self::InvalidRootWordCount { found } => {
                write!(f, "invalid PCS setup material root word count {found}")
            }
            Self::LengthOverflow => write!(f, "PCS setup material length overflow"),
            Self::Io { message } => write!(f, "PCS setup material io error: {message}"),
        }
    }
}

impl std::error::Error for PcsSetupMaterialError {}

impl From<SectionedError> for PcsSetupMaterialError {
    fn from(error: SectionedError) -> Self {
        Self::Sectioned(error)
    }
}

impl From<PcsPlanError> for PcsSetupMaterialError {
    fn from(error: PcsPlanError) -> Self {
        Self::PcsPlan(error)
    }
}

impl From<ConstantTreeError> for PcsSetupMaterialError {
    fn from(error: ConstantTreeError) -> Self {
        Self::ConstantTree(error)
    }
}

pub fn build_pcs_setup_material(
    plan: &PcsSetupPlan,
    fixed_columns: &[u8],
    constant_tree: &ConstantTree,
) -> Result<PcsSetupMaterial, PcsSetupMaterialError> {
    let plan_bytes = encode_pcs_setup_plan(plan)?;
    let VerificationKeyRoot::FieldElements(root) = constant_tree.root()? else {
        return Err(PcsSetupMaterialError::InvalidRootWordCount { found: 1 });
    };
    if root.len() != ROOT_WORDS {
        return Err(PcsSetupMaterialError::InvalidRootWordCount { found: root.len() });
    }

    Ok(PcsSetupMaterial {
        plan_digest: Sha256::digest(&plan_bytes).into(),
        fixed_column_digest: Sha256::digest(fixed_columns).into(),
        constant_tree_digest: Sha256::digest(&constant_tree.bytes).into(),
        constant_tree_root: root.try_into().expect("root length checked"),
        fixed_byte_count: u64::try_from(fixed_columns.len())
            .map_err(|_| PcsSetupMaterialError::LengthOverflow)?,
        constant_tree_byte_count: u64::try_from(constant_tree.bytes.len())
            .map_err(|_| PcsSetupMaterialError::LengthOverflow)?,
        leaf_byte_count: u64::try_from(constant_tree.leaf_byte_count)
            .map_err(|_| PcsSetupMaterialError::LengthOverflow)?,
        node_byte_count: u64::try_from(constant_tree.node_byte_count)
            .map_err(|_| PcsSetupMaterialError::LengthOverflow)?,
    })
}

pub fn read_pcs_setup_material_file(
    path: impl AsRef<Path>,
) -> Result<PcsSetupMaterial, PcsSetupMaterialError> {
    let bytes = std::fs::read(path).map_err(|error| PcsSetupMaterialError::Io {
        message: error.to_string(),
    })?;
    parse_pcs_setup_material(&bytes)
}

pub fn parse_pcs_setup_material(bytes: &[u8]) -> Result<PcsSetupMaterial, PcsSetupMaterialError> {
    let file = parse_sectioned_file(bytes, PCS_MATERIAL_KIND, PCS_MATERIAL_VERSION)?;
    if file.sections.len() != 1 {
        return Err(PcsSetupMaterialError::InvalidSectionCount {
            found: u32::try_from(file.sections.len()).unwrap_or(u32::MAX),
        });
    }
    let section = &file.sections[0];
    if section.id != PCS_MATERIAL_SECTION_ID {
        return Err(PcsSetupMaterialError::InvalidSectionId { found: section.id });
    }
    parse_pcs_setup_material_payload(&section.data)
}

pub fn encode_pcs_setup_material(
    value: &PcsSetupMaterial,
) -> Result<Vec<u8>, PcsSetupMaterialError> {
    let file = SectionedFile {
        kind: PCS_MATERIAL_KIND,
        version: PCS_MATERIAL_VERSION,
        sections: vec![SectionedSection {
            id: PCS_MATERIAL_SECTION_ID,
            data: encode_pcs_setup_material_payload(value),
        }],
    };
    encode_sectioned_file(&file).map_err(PcsSetupMaterialError::Sectioned)
}

fn parse_pcs_setup_material_payload(
    bytes: &[u8],
) -> Result<PcsSetupMaterial, PcsSetupMaterialError> {
    if bytes.len() != ENCODED_BYTES {
        return Err(PcsSetupMaterialError::InvalidPayloadLength {
            expected: ENCODED_BYTES,
            found: bytes.len(),
        });
    }
    let mut offset = 0;
    let plan_digest = read_digest(bytes, &mut offset);
    let fixed_column_digest = read_digest(bytes, &mut offset);
    let constant_tree_digest = read_digest(bytes, &mut offset);
    let mut constant_tree_root = [0_u64; ROOT_WORDS];
    for value in &mut constant_tree_root {
        *value = read_u64(bytes, &mut offset);
    }
    Ok(PcsSetupMaterial {
        plan_digest,
        fixed_column_digest,
        constant_tree_digest,
        constant_tree_root,
        fixed_byte_count: read_u64(bytes, &mut offset),
        constant_tree_byte_count: read_u64(bytes, &mut offset),
        leaf_byte_count: read_u64(bytes, &mut offset),
        node_byte_count: read_u64(bytes, &mut offset),
    })
}

fn encode_pcs_setup_material_payload(value: &PcsSetupMaterial) -> Vec<u8> {
    let mut out = Vec::with_capacity(ENCODED_BYTES);
    out.extend_from_slice(&value.plan_digest);
    out.extend_from_slice(&value.fixed_column_digest);
    out.extend_from_slice(&value.constant_tree_digest);
    for word in value.constant_tree_root {
        out.extend_from_slice(&word.to_le_bytes());
    }
    out.extend_from_slice(&value.fixed_byte_count.to_le_bytes());
    out.extend_from_slice(&value.constant_tree_byte_count.to_le_bytes());
    out.extend_from_slice(&value.leaf_byte_count.to_le_bytes());
    out.extend_from_slice(&value.node_byte_count.to_le_bytes());
    out
}

fn read_digest(bytes: &[u8], offset: &mut usize) -> [u8; 32] {
    let end = *offset + DIGEST_BYTES;
    let out = bytes[*offset..end]
        .try_into()
        .expect("payload length checked");
    *offset = end;
    out
}

fn read_u64(bytes: &[u8], offset: &mut usize) -> u64 {
    let end = *offset + 8;
    let out = u64::from_le_bytes(
        bytes[*offset..end]
            .try_into()
            .expect("payload length checked"),
    );
    *offset = end;
    out
}
