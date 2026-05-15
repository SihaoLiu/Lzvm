use std::fmt;

use crate::setup_info::{SetupInfoError, UnitSetupInfo};

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
    DomainTooLarge { bits: u32 },
    InvalidDomainBits { base_bits: u32, extended_bits: u32 },
    InvalidQueryCount,
    InvalidMerkleTreeArity { arity: u32 },
    EmptyFriSchedule,
    InvalidFirstFriLayer { expected: u32, found: u32 },
    InvalidFriLayer { input_bits: u32, output_bits: u32 },
}

impl fmt::Display for PcsPlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SetupInfo(error) => write!(f, "PCS setup plan metadata error: {error}"),
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
        }
    }
}

impl std::error::Error for PcsPlanError {}

impl From<SetupInfoError> for PcsPlanError {
    fn from(error: SetupInfoError) -> Self {
        Self::SetupInfo(error)
    }
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
