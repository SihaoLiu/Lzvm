use crate::expression_info::ExpressionInfo;
use crate::global_info::{GlobalInfo, NamedStageValue};
use crate::setup_info::UnitSetupInfo;
use crate::verifier_info::VerifierInfo;
use std::collections::BTreeSet;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MetadataValidationError {
    ConstraintCountMismatch {
        expected: u32,
        found: usize,
    },
    ExpressionStageOutOfRange {
        expression_id: u32,
        stage: u32,
        max_stage: u32,
    },
    ConstraintStageOutOfRange {
        constraint_index: usize,
        stage: u32,
        max_stage: u32,
    },
    VerifierQuotientExpressionMissing {
        expression_id: u32,
    },
    VerifierQueryExpressionMissing {
        expression_id: u32,
    },
    VerifierQuotientStageOutOfRange {
        stage: u32,
        max_stage: u32,
    },
    VerifierQueryStageOutOfRange {
        stage: u32,
        max_stage: u32,
    },
    NoChallengeStages,
    ProofValueCountMismatch {
        expected: u64,
        found: u64,
    },
    ProofValueStageOutOfRange {
        stage: u64,
        declared_stages: usize,
    },
    ProofValueStageCountMismatch {
        stage: u64,
        expected: u64,
        found: u64,
    },
    ProofValueCountOverflow,
}

impl fmt::Display for MetadataValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConstraintCountMismatch { expected, found } => write!(
                f,
                "metadata constraint count mismatch: expected {expected}, found {found}"
            ),
            Self::ExpressionStageOutOfRange {
                expression_id,
                stage,
                max_stage,
            } => write!(
                f,
                "metadata expression {expression_id} uses stage {stage}, above max stage {max_stage}"
            ),
            Self::ConstraintStageOutOfRange {
                constraint_index,
                stage,
                max_stage,
            } => write!(
                f,
                "metadata constraint {constraint_index} uses stage {stage}, outside 1..={max_stage}"
            ),
            Self::VerifierQuotientExpressionMissing { expression_id } => write!(
                f,
                "metadata quotient verifier references missing expression {expression_id}"
            ),
            Self::VerifierQueryExpressionMissing { expression_id } => write!(
                f,
                "metadata query verifier references missing expression {expression_id}"
            ),
            Self::VerifierQuotientStageOutOfRange { stage, max_stage } => write!(
                f,
                "metadata quotient verifier uses stage {stage}, above max stage {max_stage}"
            ),
            Self::VerifierQueryStageOutOfRange { stage, max_stage } => write!(
                f,
                "metadata query verifier uses stage {stage}, above max stage {max_stage}"
            ),
            Self::NoChallengeStages => write!(f, "metadata has no challenge stage counters"),
            Self::ProofValueCountMismatch { expected, found } => write!(
                f,
                "metadata proof value count mismatch: expected {expected}, found {found}"
            ),
            Self::ProofValueStageOutOfRange {
                stage,
                declared_stages,
            } => write!(
                f,
                "metadata proof value uses stage {stage}, but only {declared_stages} proof value stages are declared"
            ),
            Self::ProofValueStageCountMismatch {
                stage,
                expected,
                found,
            } => write!(
                f,
                "metadata proof value stage {stage} count mismatch: expected {expected}, found {found}"
            ),
            Self::ProofValueCountOverflow => write!(f, "metadata proof value count overflow"),
        }
    }
}

impl std::error::Error for MetadataValidationError {}

pub fn validate_unit_metadata(
    setup: &UnitSetupInfo,
    expressions: &ExpressionInfo,
    verifier: &VerifierInfo,
) -> Result<(), MetadataValidationError> {
    if let Some(expected) = setup.n_constraints {
        let found = expressions.constraints.len();
        if expected as usize != found {
            return Err(MetadataValidationError::ConstraintCountMismatch { expected, found });
        }
    }

    let max_expression_stage = setup.n_stages.saturating_add(1);
    let expression_ids = validate_expression_stages(expressions, max_expression_stage)?;
    validate_constraint_stages(expressions, setup.n_stages)?;
    validate_verifier_code(verifier, &expression_ids, max_expression_stage)?;

    Ok(())
}

pub fn validate_global_metadata(global: &GlobalInfo) -> Result<(), MetadataValidationError> {
    if global.num_challenges.is_empty() {
        return Err(MetadataValidationError::NoChallengeStages);
    }

    validate_global_proof_value_counts(global)
}

pub(crate) fn validate_global_proof_value_counts(
    global: &GlobalInfo,
) -> Result<(), MetadataValidationError> {
    let found = proof_value_count(global)?;
    if global.num_proof_values.is_empty() {
        if found != 0 {
            return Err(MetadataValidationError::ProofValueCountMismatch { expected: 0, found });
        }
    } else {
        let expected = global
            .num_proof_values
            .iter()
            .try_fold(0_u64, |count, value| {
                count
                    .checked_add(*value)
                    .ok_or(MetadataValidationError::ProofValueCountOverflow)
            })?;
        if expected != found {
            return Err(MetadataValidationError::ProofValueCountMismatch { expected, found });
        }
        validate_proof_value_stage_counts(global)?;
    }

    Ok(())
}

fn proof_value_count(global: &GlobalInfo) -> Result<u64, MetadataValidationError> {
    global
        .proof_values_map
        .iter()
        .try_fold(0_u64, |count, entry| {
            let dimension = proof_value_dimension(entry)?;
            count
                .checked_add(dimension)
                .ok_or(MetadataValidationError::ProofValueCountOverflow)
        })
}

fn validate_proof_value_stage_counts(global: &GlobalInfo) -> Result<(), MetadataValidationError> {
    let mut found_by_stage = vec![0_u64; global.num_proof_values.len()];
    for entry in &global.proof_values_map {
        let stage_index = usize::try_from(entry.stage)
            .ok()
            .and_then(|stage| stage.checked_sub(1))
            .filter(|stage_index| *stage_index < found_by_stage.len())
            .ok_or(MetadataValidationError::ProofValueStageOutOfRange {
                stage: entry.stage,
                declared_stages: found_by_stage.len(),
            })?;
        let dimension = proof_value_dimension(entry)?;
        found_by_stage[stage_index] = found_by_stage[stage_index]
            .checked_add(dimension)
            .ok_or(MetadataValidationError::ProofValueCountOverflow)?;
    }

    for (stage_index, (expected, found)) in global
        .num_proof_values
        .iter()
        .zip(found_by_stage.iter())
        .enumerate()
    {
        if expected != found {
            return Err(MetadataValidationError::ProofValueStageCountMismatch {
                stage: u64::try_from(stage_index)
                    .ok()
                    .and_then(|stage| stage.checked_add(1))
                    .ok_or(MetadataValidationError::ProofValueCountOverflow)?,
                expected: *expected,
                found: *found,
            });
        }
    }

    Ok(())
}

fn proof_value_dimension(entry: &NamedStageValue) -> Result<u64, MetadataValidationError> {
    entry.lengths.iter().try_fold(1_u64, |dimension, length| {
        if *length == 0 {
            return Err(MetadataValidationError::ProofValueCountOverflow);
        }
        dimension
            .checked_mul(*length)
            .ok_or(MetadataValidationError::ProofValueCountOverflow)
    })
}

fn validate_expression_stages(
    expressions: &ExpressionInfo,
    max_stage: u32,
) -> Result<BTreeSet<u32>, MetadataValidationError> {
    let mut expression_ids = BTreeSet::new();
    for expression in &expressions.expressions {
        expression_ids.insert(expression.expression_id);
        if expression.stage > max_stage {
            return Err(MetadataValidationError::ExpressionStageOutOfRange {
                expression_id: expression.expression_id,
                stage: expression.stage,
                max_stage,
            });
        }
    }
    Ok(expression_ids)
}

fn validate_constraint_stages(
    expressions: &ExpressionInfo,
    max_stage: u32,
) -> Result<(), MetadataValidationError> {
    for (constraint_index, constraint) in expressions.constraints.iter().enumerate() {
        if constraint.stage == 0 || constraint.stage > max_stage {
            return Err(MetadataValidationError::ConstraintStageOutOfRange {
                constraint_index,
                stage: constraint.stage,
                max_stage,
            });
        }
    }
    Ok(())
}

fn validate_verifier_code(
    verifier: &VerifierInfo,
    expression_ids: &BTreeSet<u32>,
    max_stage: u32,
) -> Result<(), MetadataValidationError> {
    if let Some(expression_id) = verifier.quotient.expression_id {
        if !expression_ids.contains(&expression_id) {
            return Err(MetadataValidationError::VerifierQuotientExpressionMissing {
                expression_id,
            });
        }
    }
    if let Some(expression_id) = verifier.query.expression_id {
        if !expression_ids.contains(&expression_id) {
            return Err(MetadataValidationError::VerifierQueryExpressionMissing { expression_id });
        }
    }
    if let Some(stage) = verifier.quotient.stage {
        if stage > max_stage {
            return Err(MetadataValidationError::VerifierQuotientStageOutOfRange {
                stage,
                max_stage,
            });
        }
    }
    if let Some(stage) = verifier.query.stage {
        if stage > max_stage {
            return Err(MetadataValidationError::VerifierQueryStageOutOfRange { stage, max_stage });
        }
    }
    Ok(())
}
