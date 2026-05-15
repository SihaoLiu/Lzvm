use std::fmt;

use lzvm_artifacts::constant_opening_segment::ConstantOpeningUnitSegment;
use lzvm_artifacts::pcs_evaluation_segment::PcsEvaluationUnitSegment;
use lzvm_artifacts::pcs_fri_segment::PcsFriOpeningUnitSegment;
use lzvm_artifacts::verifier_info::VerifierCode;
use lzvm_artifacts::witness_opening_segment::WitnessOpeningUnitSegment;
use lzvm_field::{Ext3, Felt, FieldError, SHIFT};

use crate::verifier_eval::{
    evaluate_verifier_code, VerifierCommitmentColumn, VerifierEvalError, VerifierEvalInputs,
    VerifierOpenedStage,
};
use crate::ProveUnitSchedule;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifierQueryOpenedStage {
    pub stage_index: u32,
    pub values: Vec<Felt>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifierQueryEvalInput {
    pub challenges: Vec<Ext3>,
    pub evaluations: Vec<Ext3>,
    pub constants: Vec<Felt>,
    pub opened_stages: Vec<VerifierQueryOpenedStage>,
    pub commitment_columns: Vec<VerifierCommitmentColumn>,
    pub zi: Vec<Ext3>,
    pub proof_values: Vec<Ext3>,
    pub x_div_x_sub: Vec<Ext3>,
}

#[derive(Debug, Clone, Copy)]
pub struct VerifierQueryEvalInputRequest<'a> {
    pub unit_index: u32,
    pub query_index: usize,
    pub challenges: &'a [Ext3],
    pub proof_values: &'a [Ext3],
    pub constant_unit: &'a ConstantOpeningUnitSegment,
    pub witness_unit: &'a WitnessOpeningUnitSegment,
    pub evaluations: &'a PcsEvaluationUnitSegment,
}

#[derive(Debug, Clone, Copy)]
pub struct VerifierUnitQueryEvalRequest<'a> {
    pub unit_index: u32,
    pub challenges: &'a [Ext3],
    pub proof_values: &'a [Ext3],
    pub constant_unit: &'a ConstantOpeningUnitSegment,
    pub witness_unit: &'a WitnessOpeningUnitSegment,
    pub evaluations: &'a PcsEvaluationUnitSegment,
    pub code: &'a VerifierCode,
    pub publics: &'a [Felt],
}

#[derive(Debug, Clone, Copy)]
pub struct VerifierFriComparisonRequest<'a> {
    pub unit_index: u32,
    pub query_rows: &'a [u64],
    pub query_outputs: &'a [Ext3],
    pub fri: &'a PcsFriOpeningUnitSegment,
}

#[derive(Debug, Clone, Copy)]
pub struct VerifierFriQueryOutputValidationRequest<'a> {
    pub unit_index: u32,
    pub query_rows: &'a [u64],
    pub challenges: &'a [Ext3],
    pub proof_values: &'a [Ext3],
    pub constant_unit: &'a ConstantOpeningUnitSegment,
    pub witness_unit: &'a WitnessOpeningUnitSegment,
    pub evaluations: &'a PcsEvaluationUnitSegment,
    pub code: &'a VerifierCode,
    pub publics: &'a [Felt],
    pub fri: &'a PcsFriOpeningUnitSegment,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerifierQueryEvalError {
    UnitIndexMismatch {
        expected: u32,
        found: u32,
        source: &'static str,
    },
    QueryIndexOutOfRange {
        query_index: usize,
        len: usize,
        source: &'static str,
    },
    QueryRowMismatch {
        constant_row: u64,
        witness_row: u64,
    },
    QueryRowOutOfRange {
        row_index: u64,
        domain_size: u64,
    },
    MissingXiChallenge {
        challenge_count: usize,
    },
    ChallengeIndexOutOfRange {
        index: usize,
        len: usize,
    },
    UnsupportedDomainBits {
        bits: u32,
        domain: &'static str,
    },
    NonCanonicalField {
        source: &'static str,
        value: u64,
    },
    ZeroDenominator {
        opening_index: usize,
    },
    ZeroBoundaryDenominator,
    LengthOverflow,
    Eval(VerifierEvalError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerifierFriComparisonError {
    UnitIndexMismatch {
        expected: u32,
        found: u32,
    },
    MissingFriLayer,
    QueryOutputCountMismatch {
        expected: usize,
        found: usize,
    },
    QueryRowCountMismatch {
        expected: usize,
        found: usize,
    },
    FriQueryCountMismatch {
        expected: usize,
        found: usize,
    },
    UnsupportedDomainBits {
        bits: u32,
    },
    FriValueIndexOutOfRange {
        query_index: usize,
        value_index: usize,
        len: usize,
    },
    FriQueryRowMismatch {
        query_index: usize,
        expected: u64,
        found: u64,
    },
    NonCanonicalField {
        value: u64,
    },
    LengthOverflow,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerifierFriQueryOutputValidationError {
    Query(VerifierQueryEvalError),
    Comparison(VerifierFriComparisonError),
}

impl fmt::Display for VerifierQueryEvalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnitIndexMismatch {
                expected,
                found,
                source,
            } => write!(
                f,
                "verifier query {source} unit index {found} does not match expected {expected}"
            ),
            Self::QueryIndexOutOfRange {
                query_index,
                len,
                source,
            } => write!(
                f,
                "verifier query {source} index {query_index} is outside query count {len}"
            ),
            Self::QueryRowMismatch {
                constant_row,
                witness_row,
            } => write!(
                f,
                "verifier query row mismatch: constant row {constant_row}, witness row {witness_row}"
            ),
            Self::QueryRowOutOfRange {
                row_index,
                domain_size,
            } => write!(
                f,
                "verifier query row {row_index} is outside extended domain size {domain_size}"
            ),
            Self::MissingXiChallenge { challenge_count } => write!(
                f,
                "verifier query challenge count {challenge_count} cannot locate xi challenge"
            ),
            Self::ChallengeIndexOutOfRange { index, len } => write!(
                f,
                "verifier query challenge index {index} is outside challenge count {len}"
            ),
            Self::UnsupportedDomainBits { bits, domain } => {
                write!(f, "unsupported verifier query {domain} domain bits: {bits}")
            }
            Self::NonCanonicalField { source, value } => write!(
                f,
                "verifier query {source} field value is not canonical: {value}"
            ),
            Self::ZeroDenominator { opening_index } => write!(
                f,
                "verifier query x divisor denominator is zero at opening {opening_index}"
            ),
            Self::ZeroBoundaryDenominator => {
                write!(f, "verifier query boundary denominator is zero")
            }
            Self::LengthOverflow => write!(f, "verifier query input length overflow"),
            Self::Eval(error) => write!(f, "verifier query evaluation failed: {error}"),
        }
    }
}

impl fmt::Display for VerifierFriComparisonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnitIndexMismatch { expected, found } => write!(
                f,
                "verifier FRI comparison unit index {found} does not match expected {expected}"
            ),
            Self::MissingFriLayer => write!(f, "verifier FRI comparison has no FRI layers"),
            Self::QueryOutputCountMismatch { expected, found } => write!(
                f,
                "verifier FRI comparison expected {expected} query outputs, found {found}"
            ),
            Self::QueryRowCountMismatch { expected, found } => write!(
                f,
                "verifier FRI comparison expected {expected} query rows, found {found}"
            ),
            Self::FriQueryCountMismatch { expected, found } => write!(
                f,
                "verifier FRI comparison expected {expected} FRI queries, found {found}"
            ),
            Self::UnsupportedDomainBits { bits } => {
                write!(f, "unsupported verifier FRI comparison domain bits: {bits}")
            }
            Self::FriValueIndexOutOfRange {
                query_index,
                value_index,
                len,
            } => write!(
                f,
                "verifier FRI comparison query {query_index} value index {value_index} is outside value count {len}"
            ),
            Self::FriQueryRowMismatch {
                query_index,
                expected,
                found,
            } => write!(
                f,
                "verifier FRI comparison query {query_index} row {found} does not match expected {expected}"
            ),
            Self::NonCanonicalField { value } => write!(
                f,
                "verifier FRI comparison field value is not canonical: {value}"
            ),
            Self::LengthOverflow => write!(f, "verifier FRI comparison length overflow"),
        }
    }
}

impl fmt::Display for VerifierFriQueryOutputValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Query(error) => write!(f, "{error}"),
            Self::Comparison(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for VerifierQueryEvalError {}
impl std::error::Error for VerifierFriComparisonError {}
impl std::error::Error for VerifierFriQueryOutputValidationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Query(error) => Some(error),
            Self::Comparison(error) => Some(error),
        }
    }
}

impl From<VerifierEvalError> for VerifierQueryEvalError {
    fn from(error: VerifierEvalError) -> Self {
        Self::Eval(error)
    }
}

impl From<VerifierQueryEvalError> for VerifierFriQueryOutputValidationError {
    fn from(error: VerifierQueryEvalError) -> Self {
        Self::Query(error)
    }
}

impl From<VerifierFriComparisonError> for VerifierFriQueryOutputValidationError {
    fn from(error: VerifierFriComparisonError) -> Self {
        Self::Comparison(error)
    }
}

pub fn assemble_verifier_query_eval_input(
    schedule: &ProveUnitSchedule,
    request: VerifierQueryEvalInputRequest<'_>,
) -> Result<VerifierQueryEvalInput, VerifierQueryEvalError> {
    expect_unit_index(
        request.unit_index,
        request.constant_unit.unit_index,
        "constant opening",
    )?;
    expect_unit_index(
        request.unit_index,
        request.witness_unit.unit_index,
        "witness opening",
    )?;
    expect_unit_index(
        request.unit_index,
        request.evaluations.unit_index,
        "evaluation",
    )?;

    let constant_query = request
        .constant_unit
        .queries
        .get(request.query_index)
        .ok_or(VerifierQueryEvalError::QueryIndexOutOfRange {
            query_index: request.query_index,
            len: request.constant_unit.queries.len(),
            source: "constant opening",
        })?;
    let witness_query = request
        .witness_unit
        .queries
        .get(request.query_index)
        .ok_or(VerifierQueryEvalError::QueryIndexOutOfRange {
            query_index: request.query_index,
            len: request.witness_unit.queries.len(),
            source: "witness opening",
        })?;
    if constant_query.row_index != witness_query.row_index {
        return Err(VerifierQueryEvalError::QueryRowMismatch {
            constant_row: constant_query.row_index,
            witness_row: witness_query.row_index,
        });
    }
    if constant_query.row_index >= schedule.extended_domain_size {
        return Err(VerifierQueryEvalError::QueryRowOutOfRange {
            row_index: constant_query.row_index,
            domain_size: schedule.extended_domain_size,
        });
    }

    let constants = convert_felts("constant opening", &constant_query.values)?;
    let opened_stages = witness_query
        .stages
        .iter()
        .map(|stage| {
            Ok(VerifierQueryOpenedStage {
                stage_index: stage.stage_index,
                values: convert_felts("witness opening", &stage.values)?,
            })
        })
        .collect::<Result<Vec<_>, VerifierQueryEvalError>>()?;
    let evaluations = request
        .evaluations
        .values
        .iter()
        .map(|value| convert_ext("evaluation", *value))
        .collect::<Result<Vec<_>, VerifierQueryEvalError>>()?;
    let commitment_columns = schedule
        .commitment_columns
        .iter()
        .map(|column| {
            let position = usize::try_from(column.stage_position)
                .map_err(|_| VerifierQueryEvalError::LengthOverflow)?;
            Ok(VerifierCommitmentColumn {
                stage_index: column.stage,
                position,
            })
        })
        .collect::<Result<Vec<_>, VerifierQueryEvalError>>()?;
    let xi_challenge = xi_challenge(schedule, request.challenges)?;
    let x_div_x_sub = compute_x_div_x_sub(schedule, constant_query.row_index, xi_challenge)?;
    let zi = vec![compute_zi(schedule, xi_challenge)?];

    Ok(VerifierQueryEvalInput {
        challenges: request.challenges.to_vec(),
        evaluations,
        constants,
        opened_stages,
        commitment_columns,
        zi,
        proof_values: request.proof_values.to_vec(),
        x_div_x_sub,
    })
}

pub fn evaluate_verifier_unit_queries(
    schedule: &ProveUnitSchedule,
    request: VerifierUnitQueryEvalRequest<'_>,
) -> Result<Vec<Ext3>, VerifierQueryEvalError> {
    let query_count = usize::try_from(schedule.query_count)
        .map_err(|_| VerifierQueryEvalError::LengthOverflow)?;
    let mut values = Vec::with_capacity(query_count);
    for query_index in 0..query_count {
        let input = assemble_verifier_query_eval_input(
            schedule,
            VerifierQueryEvalInputRequest {
                unit_index: request.unit_index,
                query_index,
                challenges: request.challenges,
                proof_values: request.proof_values,
                constant_unit: request.constant_unit,
                witness_unit: request.witness_unit,
                evaluations: request.evaluations,
            },
        )?;
        values.push(input.evaluate(request.code, request.publics)?);
    }
    Ok(values)
}

pub fn verify_query_outputs_against_fri_opening(
    schedule: &ProveUnitSchedule,
    request: VerifierFriComparisonRequest<'_>,
) -> Result<bool, VerifierFriComparisonError> {
    if request.unit_index != request.fri.unit_index {
        return Err(VerifierFriComparisonError::UnitIndexMismatch {
            expected: request.unit_index,
            found: request.fri.unit_index,
        });
    }
    let query_count = usize::try_from(schedule.query_count)
        .map_err(|_| VerifierFriComparisonError::LengthOverflow)?;
    if request.query_outputs.len() != query_count {
        return Err(VerifierFriComparisonError::QueryOutputCountMismatch {
            expected: query_count,
            found: request.query_outputs.len(),
        });
    }
    if request.query_rows.len() != query_count {
        return Err(VerifierFriComparisonError::QueryRowCountMismatch {
            expected: query_count,
            found: request.query_rows.len(),
        });
    }
    let Some(first_layer) = request
        .fri
        .layers
        .iter()
        .find(|layer| layer.layer_index == 0)
    else {
        return Err(VerifierFriComparisonError::MissingFriLayer);
    };
    if first_layer.queries.len() != query_count {
        return Err(VerifierFriComparisonError::FriQueryCountMismatch {
            expected: query_count,
            found: first_layer.queries.len(),
        });
    }
    let Some(layer) = schedule.fri_layers.first() else {
        return Err(VerifierFriComparisonError::MissingFriLayer);
    };
    let domain_size = 1_u64.checked_shl(layer.input_bits).ok_or(
        VerifierFriComparisonError::UnsupportedDomainBits {
            bits: layer.input_bits,
        },
    )?;
    let output_size = 1_u64.checked_shl(layer.output_bits).ok_or(
        VerifierFriComparisonError::UnsupportedDomainBits {
            bits: layer.output_bits,
        },
    )?;

    for (query_index, ((query_row, query_output), query)) in request
        .query_rows
        .iter()
        .zip(request.query_outputs.iter())
        .zip(&first_layer.queries)
        .enumerate()
    {
        let expected_layer_row = query_row % output_size;
        if query.row_index != expected_layer_row {
            return Err(VerifierFriComparisonError::FriQueryRowMismatch {
                query_index,
                expected: expected_layer_row,
                found: query.row_index,
            });
        }
        let value_index = usize::try_from((*query_row % domain_size) / output_size)
            .map_err(|_| VerifierFriComparisonError::LengthOverflow)?;
        let Some(value) = query.values.get(value_index) else {
            return Err(VerifierFriComparisonError::FriValueIndexOutOfRange {
                query_index,
                value_index,
                len: query.values.len(),
            });
        };
        if *query_output != convert_fri_ext(*value)? {
            return Ok(false);
        }
    }
    Ok(true)
}

pub fn validate_verifier_query_outputs_against_fri_opening(
    schedule: &ProveUnitSchedule,
    request: VerifierFriQueryOutputValidationRequest<'_>,
) -> Result<bool, VerifierFriQueryOutputValidationError> {
    let query_outputs = evaluate_verifier_unit_queries(
        schedule,
        VerifierUnitQueryEvalRequest {
            unit_index: request.unit_index,
            challenges: request.challenges,
            proof_values: request.proof_values,
            constant_unit: request.constant_unit,
            witness_unit: request.witness_unit,
            evaluations: request.evaluations,
            code: request.code,
            publics: request.publics,
        },
    )?;
    Ok(verify_query_outputs_against_fri_opening(
        schedule,
        VerifierFriComparisonRequest {
            unit_index: request.unit_index,
            query_rows: request.query_rows,
            query_outputs: &query_outputs,
            fri: request.fri,
        },
    )?)
}

impl VerifierQueryEvalInput {
    pub fn evaluate(
        &self,
        code: &VerifierCode,
        publics: &[Felt],
    ) -> Result<Ext3, VerifierQueryEvalError> {
        let opened_stages = self
            .opened_stages
            .iter()
            .map(|stage| VerifierOpenedStage {
                stage_index: stage.stage_index,
                values: &stage.values,
            })
            .collect::<Vec<_>>();
        let inputs = VerifierEvalInputs {
            challenges: &self.challenges,
            evaluations: &self.evaluations,
            publics,
            constants: &self.constants,
            commitments: &[],
            opened_stages: &opened_stages,
            commitment_columns: &self.commitment_columns,
            zi: &self.zi,
            proof_values: &self.proof_values,
            x_div_x_sub: &self.x_div_x_sub,
        };
        Ok(evaluate_verifier_code(code, &inputs)?)
    }
}

fn expect_unit_index(
    expected: u32,
    found: u32,
    source: &'static str,
) -> Result<(), VerifierQueryEvalError> {
    if expected == found {
        Ok(())
    } else {
        Err(VerifierQueryEvalError::UnitIndexMismatch {
            expected,
            found,
            source,
        })
    }
}

fn convert_felts(
    source: &'static str,
    values: &[u64],
) -> Result<Vec<Felt>, VerifierQueryEvalError> {
    values
        .iter()
        .map(|value| convert_felt(source, *value))
        .collect()
}

fn convert_ext(source: &'static str, values: [u64; 3]) -> Result<Ext3, VerifierQueryEvalError> {
    Ok(Ext3::new(
        convert_felt(source, values[0])?,
        convert_felt(source, values[1])?,
        convert_felt(source, values[2])?,
    ))
}

fn convert_fri_ext(values: [u64; 3]) -> Result<Ext3, VerifierFriComparisonError> {
    Ok(Ext3::new(
        convert_fri_felt(values[0])?,
        convert_fri_felt(values[1])?,
        convert_fri_felt(values[2])?,
    ))
}

fn convert_felt(source: &'static str, value: u64) -> Result<Felt, VerifierQueryEvalError> {
    Felt::from_canonical(value).map_err(|error| match error {
        FieldError::NonCanonical { value } => {
            VerifierQueryEvalError::NonCanonicalField { source, value }
        }
    })
}

fn convert_fri_felt(value: u64) -> Result<Felt, VerifierFriComparisonError> {
    Felt::from_canonical(value).map_err(|error| match error {
        FieldError::NonCanonical { value } => {
            VerifierFriComparisonError::NonCanonicalField { value }
        }
    })
}

fn xi_challenge(
    schedule: &ProveUnitSchedule,
    challenges: &[Ext3],
) -> Result<Ext3, VerifierQueryEvalError> {
    let index = schedule.challenge_count.checked_sub(3).ok_or(
        VerifierQueryEvalError::MissingXiChallenge {
            challenge_count: schedule.challenge_count,
        },
    )?;
    challenges
        .get(index)
        .copied()
        .ok_or(VerifierQueryEvalError::ChallengeIndexOutOfRange {
            index,
            len: challenges.len(),
        })
}

fn compute_x_div_x_sub(
    schedule: &ProveUnitSchedule,
    row_index: u64,
    xi_challenge: Ext3,
) -> Result<Vec<Ext3>, VerifierQueryEvalError> {
    let root_ext = Felt::root_of_unity(schedule.extended_domain_bits as usize).ok_or(
        VerifierQueryEvalError::UnsupportedDomainBits {
            bits: schedule.extended_domain_bits,
            domain: "extended",
        },
    )?;
    let root_base = Felt::root_of_unity(schedule.base_domain_bits as usize).ok_or(
        VerifierQueryEvalError::UnsupportedDomainBits {
            bits: schedule.base_domain_bits,
            domain: "base",
        },
    )?;
    let x = Ext3::new(SHIFT * root_ext.pow(row_index), Felt::ZERO, Felt::ZERO);
    schedule
        .opening_points
        .iter()
        .enumerate()
        .map(|(opening_index, opening_point)| {
            let mut wi = root_base.pow(opening_point.unsigned_abs());
            if *opening_point < 0 {
                wi = wi
                    .inverse()
                    .ok_or(VerifierQueryEvalError::ZeroDenominator { opening_index })?;
            }
            (x - xi_challenge * Ext3::new(wi, Felt::ZERO, Felt::ZERO))
                .inverse()
                .ok_or(VerifierQueryEvalError::ZeroDenominator { opening_index })
        })
        .collect()
}

fn compute_zi(
    schedule: &ProveUnitSchedule,
    xi_challenge: Ext3,
) -> Result<Ext3, VerifierQueryEvalError> {
    (xi_challenge.pow(schedule.base_domain_size) - Ext3::ONE)
        .inverse()
        .ok_or(VerifierQueryEvalError::ZeroBoundaryDenominator)
}
