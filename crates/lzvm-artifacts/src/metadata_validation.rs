use crate::expression_info::ExpressionInfo;
use crate::global_info::GlobalInfo;
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
        found: usize,
    },
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

    if !global.num_proof_values.is_empty() {
        let expected = global.num_proof_values.iter().sum::<u64>();
        let found = global.proof_values_map.len();
        if expected != found as u64 {
            return Err(MetadataValidationError::ProofValueCountMismatch { expected, found });
        }
    }

    Ok(())
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
