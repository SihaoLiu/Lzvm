use std::fmt;

use lzvm_artifacts::key_directory::{
    key_directory_catalog_digest, KeyDirectoryCatalog, KeyDirectoryError, KeyUnitKind,
};
use lzvm_artifacts::pcs_plan::PcsFriLayer;
use lzvm_artifacts::verification_key::VerificationKeyRoot;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProveSchedule {
    pub setup_hash: [u8; 32],
    pub unit_count: usize,
    pub total_fixed_bytes: u64,
    pub total_query_count: u64,
    pub max_extended_domain_bits: u32,
    pub units: Vec<ProveUnitSchedule>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProveUnitSchedule {
    pub kind: KeyUnitKind,
    pub group_id: Option<usize>,
    pub unit_id: Option<usize>,
    pub group_name: Option<String>,
    pub unit_name: Option<String>,
    pub base_domain_bits: u32,
    pub extended_domain_bits: u32,
    pub base_domain_size: u64,
    pub extended_domain_size: u64,
    pub blowup_factor: u64,
    pub query_count: u32,
    pub proof_of_work_bits: u32,
    pub merkle_tree_arity: u32,
    pub transcript_arity: Option<u32>,
    pub constant_width: u32,
    pub stage_commit_widths: Vec<u32>,
    pub opening_points: Vec<i64>,
    pub fri_layers: Vec<PcsFriLayer>,
    pub final_layer_bits: u32,
    pub fixed_bytes: u64,
    pub constant_tree_root: Option<VerificationKeyRoot>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProveScheduleError {
    EmptyCatalog,
    LengthOverflow,
    KeyDirectory(KeyDirectoryError),
}

impl fmt::Display for ProveScheduleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyCatalog => write!(f, "prove schedule catalog is empty"),
            Self::LengthOverflow => write!(f, "prove schedule length overflow"),
            Self::KeyDirectory(error) => write!(f, "prove schedule catalog error: {error}"),
        }
    }
}

impl std::error::Error for ProveScheduleError {}

impl From<KeyDirectoryError> for ProveScheduleError {
    fn from(error: KeyDirectoryError) -> Self {
        Self::KeyDirectory(error)
    }
}

pub fn derive_prove_schedule(
    catalog: &KeyDirectoryCatalog,
) -> Result<ProveSchedule, ProveScheduleError> {
    if catalog.units.is_empty() {
        return Err(ProveScheduleError::EmptyCatalog);
    }

    let setup_hash = key_directory_catalog_digest(catalog)?;
    let mut total_fixed_bytes = 0_u64;
    let mut total_query_count = 0_u64;
    let mut max_extended_domain_bits = 0_u32;
    let mut units = Vec::with_capacity(catalog.units.len());
    for unit in &catalog.units {
        total_fixed_bytes = total_fixed_bytes
            .checked_add(unit.actual_fixed_bytes)
            .ok_or(ProveScheduleError::LengthOverflow)?;
        total_query_count = total_query_count
            .checked_add(u64::from(unit.pcs_plan.query_count))
            .ok_or(ProveScheduleError::LengthOverflow)?;
        max_extended_domain_bits = max_extended_domain_bits.max(unit.pcs_plan.extended_domain_bits);

        units.push(ProveUnitSchedule {
            kind: unit.paths.kind,
            group_id: unit.paths.group_id,
            unit_id: unit.paths.unit_id,
            group_name: unit.paths.group_name.clone(),
            unit_name: unit.paths.unit_name.clone(),
            base_domain_bits: unit.pcs_plan.base_domain_bits,
            extended_domain_bits: unit.pcs_plan.extended_domain_bits,
            base_domain_size: unit.pcs_plan.base_domain_size,
            extended_domain_size: unit.pcs_plan.extended_domain_size,
            blowup_factor: unit.pcs_plan.blowup_factor,
            query_count: unit.pcs_plan.query_count,
            proof_of_work_bits: unit.pcs_plan.proof_of_work_bits,
            merkle_tree_arity: unit.pcs_plan.merkle_tree_arity,
            transcript_arity: unit.pcs_plan.transcript_arity,
            constant_width: unit.pcs_plan.constant_width,
            stage_commit_widths: unit.pcs_plan.stage_commit_widths.clone(),
            opening_points: unit.pcs_plan.opening_points.clone(),
            fri_layers: unit.pcs_plan.fri_layers.clone(),
            final_layer_bits: unit.pcs_plan.final_layer_bits,
            fixed_bytes: unit.actual_fixed_bytes,
            constant_tree_root: unit.constant_tree_root.clone(),
        });
    }

    Ok(ProveSchedule {
        setup_hash,
        unit_count: units.len(),
        total_fixed_bytes,
        total_query_count,
        max_extended_domain_bits,
        units,
    })
}
