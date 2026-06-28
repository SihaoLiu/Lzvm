use std::fmt;

#[cfg(feature = "cuda")]
use lzvm_accel::cuda_goldilocks_intt;
use lzvm_artifacts::pcs_fri_segment::{PcsFriOpeningLayerSegment, PcsFriOpeningUnitSegment};
#[cfg(not(feature = "cuda"))]
use lzvm_field::intt_in_place;
use lzvm_field::{DomainError, Ext3, Felt, FieldError, SHIFT};

use super::errors::PcsFriOpeningFoldError;
use super::requests::PcsFriOpeningFoldRequest;
use crate::ProveUnitSchedule;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PcsFriFoldError {
    InvalidLayerBits {
        current_bits: u32,
        prev_bits: u32,
    },
    InvalidExtensionBits {
        n_bits_ext: u32,
        prev_bits: u32,
    },
    ValueLengthMismatch {
        expected: usize,
        found: usize,
    },
    UnsupportedRoot {
        bits: u32,
    },
    ZeroEvaluationPoint,
    Domain(DomainError),
    Field(FieldError),
    #[cfg(feature = "cuda")]
    Accel(lzvm_accel::AccelError),
    LengthOverflow,
}

impl fmt::Display for PcsFriFoldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLayerBits {
                current_bits,
                prev_bits,
            } => write!(
                f,
                "PCS FRI fold layer bits are invalid: current {current_bits}, previous {prev_bits}"
            ),
            Self::InvalidExtensionBits {
                n_bits_ext,
                prev_bits,
            } => write!(
                f,
                "PCS FRI fold extension bits are invalid: extended {n_bits_ext}, previous {prev_bits}"
            ),
            Self::ValueLengthMismatch { expected, found } => write!(
                f,
                "PCS FRI fold value length mismatch: expected {expected}, found {found}"
            ),
            Self::UnsupportedRoot { bits } => {
                write!(f, "PCS FRI fold root is unsupported for bits {bits}")
            }
            Self::ZeroEvaluationPoint => write!(f, "PCS FRI fold evaluation point is zero"),
            Self::Domain(error) => write!(f, "PCS FRI fold domain error: {error}"),
            Self::Field(error) => write!(f, "PCS FRI fold field error: {error}"),
            #[cfg(feature = "cuda")]
            Self::Accel(error) => write!(f, "PCS FRI fold cuda error: {error}"),
            Self::LengthOverflow => write!(f, "PCS FRI fold length overflow"),
        }
    }
}

impl std::error::Error for PcsFriFoldError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Domain(error) => Some(error),
            Self::Field(error) => Some(error),
            #[cfg(feature = "cuda")]
            Self::Accel(error) => Some(error),
            Self::InvalidLayerBits { .. }
            | Self::InvalidExtensionBits { .. }
            | Self::ValueLengthMismatch { .. }
            | Self::UnsupportedRoot { .. }
            | Self::ZeroEvaluationPoint
            | Self::LengthOverflow => None,
        }
    }
}

impl From<DomainError> for PcsFriFoldError {
    fn from(error: DomainError) -> Self {
        Self::Domain(error)
    }
}

impl From<FieldError> for PcsFriFoldError {
    fn from(error: FieldError) -> Self {
        Self::Field(error)
    }
}

pub fn verify_fri_fold(
    n_bits_ext: u32,
    current_bits: u32,
    prev_bits: u32,
    challenge: Ext3,
    index: u64,
    values: &[Ext3],
) -> Result<Ext3, PcsFriFoldError> {
    let fold_bits = validate_fri_fold_shape(n_bits_ext, current_bits, prev_bits, values.len())?;
    let (c0, c1, c2) = extension_fold_value_columns(values);
    evaluate_fri_fold_columns(
        n_bits_ext, prev_bits, fold_bits, challenge, index, c0, c1, c2,
    )
}

pub fn verify_fri_opening_folds(
    schedule: &ProveUnitSchedule,
    request: PcsFriOpeningFoldRequest<'_>,
) -> Result<bool, PcsFriOpeningFoldError> {
    if request.unit_index != request.fri.unit_index {
        return Err(PcsFriOpeningFoldError::UnitIndexMismatch {
            expected: request.unit_index,
            found: request.fri.unit_index,
        });
    }

    let query_count = usize::try_from(schedule.query_count)
        .map_err(|_| PcsFriOpeningFoldError::LengthOverflow)?;
    if request.query_rows.len() != query_count {
        return Err(PcsFriOpeningFoldError::QueryRowCountMismatch {
            expected: query_count,
            found: request.query_rows.len(),
        });
    }
    if request.fri.layers.len() != schedule.fri_layers.len() {
        return Err(PcsFriOpeningFoldError::LayerCountMismatch {
            expected: schedule.fri_layers.len(),
            found: request.fri.layers.len(),
        });
    }

    let layers = ordered_opening_layers(request.fri, schedule.fri_layers.len())?;
    for layer in &layers {
        if layer.queries.len() != query_count {
            return Err(PcsFriOpeningFoldError::LayerQueryCountMismatch {
                layer_index: layer.layer_index,
                expected: query_count,
                found: layer.queries.len(),
            });
        }
    }

    for (query_index, query_row) in request.query_rows.iter().enumerate() {
        for (layer_index, (layer_plan, opening_layer)) in
            schedule.fri_layers.iter().zip(layers.iter()).enumerate()
        {
            let output_size = domain_size(layer_plan.output_bits)?;
            let expected_row = query_row % output_size;
            let query = &opening_layer.queries[query_index];
            if query.row_index != expected_row {
                return Err(PcsFriOpeningFoldError::LayerQueryRowMismatch {
                    layer_index: opening_layer.layer_index,
                    query_index,
                    expected: expected_row,
                    found: query.row_index,
                });
            }

            let (c0, c1, c2) = convert_fold_value_columns(&query.values)?;
            let layer_challenge_start = request
                .challenges
                .len()
                .checked_sub(schedule.fri_layers.len())
                .and_then(|index| index.checked_sub(1))
                .ok_or(PcsFriOpeningFoldError::LengthOverflow)?;
            let challenge_index = layer_challenge_start
                .checked_add(layer_index)
                .ok_or(PcsFriOpeningFoldError::LengthOverflow)?;
            let challenge = *request.challenges.get(challenge_index).ok_or(
                PcsFriOpeningFoldError::MissingChallenge {
                    index: challenge_index,
                    len: request.challenges.len(),
                },
            )?;
            let folded = verify_fri_fold_columns(
                schedule.extended_domain_bits,
                layer_plan.output_bits,
                layer_plan.input_bits,
                challenge,
                expected_row,
                c0,
                c1,
                c2,
            )
            .map_err(PcsFriOpeningFoldError::Fold)?;

            let target = if let Some(next_plan) = schedule.fri_layers.get(layer_index + 1) {
                let next_output_size = domain_size(next_plan.output_bits)?;
                let value_index = usize::try_from(expected_row / next_output_size)
                    .map_err(|_| PcsFriOpeningFoldError::LengthOverflow)?;
                let next_layer = layers[layer_index + 1];
                let next_query = &next_layer.queries[query_index];
                let value = next_query.values.get(value_index).ok_or(
                    PcsFriOpeningFoldError::LayerValueIndexOutOfRange {
                        layer_index: next_layer.layer_index,
                        query_index,
                        value_index,
                        len: next_query.values.len(),
                    },
                )?;
                convert_ext(*value)?
            } else {
                let final_index = usize::try_from(expected_row)
                    .map_err(|_| PcsFriOpeningFoldError::LengthOverflow)?;
                let value = request.fri.final_polynomial.get(final_index).ok_or(
                    PcsFriOpeningFoldError::FinalIndexOutOfRange {
                        query_index,
                        index: final_index,
                        len: request.fri.final_polynomial.len(),
                    },
                )?;
                convert_ext(*value)?
            };

            if folded != target {
                return Ok(false);
            }
        }
    }

    Ok(true)
}

fn extension_fold_value_columns(values: &[Ext3]) -> (Vec<Felt>, Vec<Felt>, Vec<Felt>) {
    let mut c0 = Vec::with_capacity(values.len());
    let mut c1 = Vec::with_capacity(values.len());
    let mut c2 = Vec::with_capacity(values.len());
    for value in values {
        c0.push(value.c0);
        c1.push(value.c1);
        c2.push(value.c2);
    }
    (c0, c1, c2)
}

fn convert_fold_value_columns(
    values: &[[u64; 3]],
) -> Result<(Vec<Felt>, Vec<Felt>, Vec<Felt>), PcsFriOpeningFoldError> {
    let mut c0 = Vec::with_capacity(values.len());
    let mut c1 = Vec::with_capacity(values.len());
    let mut c2 = Vec::with_capacity(values.len());
    for value in values {
        c0.push(convert_felt(value[0])?);
        c1.push(convert_felt(value[1])?);
        c2.push(convert_felt(value[2])?);
    }
    Ok((c0, c1, c2))
}

fn verify_fri_fold_columns(
    n_bits_ext: u32,
    current_bits: u32,
    prev_bits: u32,
    challenge: Ext3,
    index: u64,
    c0: Vec<Felt>,
    c1: Vec<Felt>,
    c2: Vec<Felt>,
) -> Result<Ext3, PcsFriFoldError> {
    let fold_bits = validate_fri_fold_shape(n_bits_ext, current_bits, prev_bits, c0.len())?;
    evaluate_fri_fold_columns(
        n_bits_ext, prev_bits, fold_bits, challenge, index, c0, c1, c2,
    )
}

fn validate_fri_fold_shape(
    n_bits_ext: u32,
    current_bits: u32,
    prev_bits: u32,
    value_len: usize,
) -> Result<u32, PcsFriFoldError> {
    if prev_bits <= current_bits {
        return Err(PcsFriFoldError::InvalidLayerBits {
            current_bits,
            prev_bits,
        });
    }
    if n_bits_ext < prev_bits {
        return Err(PcsFriFoldError::InvalidExtensionBits {
            n_bits_ext,
            prev_bits,
        });
    }

    let fold_bits = prev_bits - current_bits;
    let expected_len = 1_usize
        .checked_shl(fold_bits)
        .ok_or(PcsFriFoldError::LengthOverflow)?;
    if value_len != expected_len {
        return Err(PcsFriFoldError::ValueLengthMismatch {
            expected: expected_len,
            found: value_len,
        });
    }
    Ok(fold_bits)
}

fn evaluate_fri_fold_columns(
    n_bits_ext: u32,
    prev_bits: u32,
    fold_bits: u32,
    challenge: Ext3,
    index: u64,
    c0: Vec<Felt>,
    c1: Vec<Felt>,
    c2: Vec<Felt>,
) -> Result<Ext3, PcsFriFoldError> {
    let coefficients = interpolate_fold_columns(c0, c1, c2, fold_bits as usize)?;
    let shift = fold_shift(n_bits_ext, prev_bits);
    let root = Felt::root_of_unity(prev_bits as usize)
        .ok_or(PcsFriFoldError::UnsupportedRoot { bits: prev_bits })?;
    let point = shift * root.pow(index);
    let inverse = point
        .inverse()
        .ok_or(PcsFriFoldError::ZeroEvaluationPoint)?;
    Ok(evaluate_extension_polynomial(
        &coefficients,
        scale_extension(challenge, inverse),
    ))
}

fn interpolate_fold_columns(
    c0: Vec<Felt>,
    c1: Vec<Felt>,
    c2: Vec<Felt>,
    bits: usize,
) -> Result<Vec<Ext3>, PcsFriFoldError> {
    let c0 = interpolate_fold_column_owned(c0, bits)?;
    let c1 = interpolate_fold_column_owned(c1, bits)?;
    let c2 = interpolate_fold_column_owned(c2, bits)?;
    Ok(c0
        .into_iter()
        .zip(c1)
        .zip(c2)
        .map(|((c0, c1), c2)| Ext3::new(c0, c1, c2))
        .collect())
}

#[cfg(feature = "cuda")]
fn interpolate_fold_column_owned(
    values: Vec<Felt>,
    bits: usize,
) -> Result<Vec<Felt>, PcsFriFoldError> {
    let raw = values
        .iter()
        .map(|value| value.to_u64())
        .collect::<Vec<_>>();
    let transformed = cuda_goldilocks_intt(&raw, bits).map_err(PcsFriFoldError::Accel)?;
    transformed
        .into_iter()
        .map(Felt::from_canonical)
        .collect::<Result<Vec<_>, _>>()
        .map_err(PcsFriFoldError::Field)
}

#[cfg(not(feature = "cuda"))]
fn interpolate_fold_column_owned(
    mut values: Vec<Felt>,
    bits: usize,
) -> Result<Vec<Felt>, PcsFriFoldError> {
    intt_in_place(&mut values, bits).map_err(PcsFriFoldError::Domain)?;
    Ok(values)
}

fn fold_shift(n_bits_ext: u32, prev_bits: u32) -> Felt {
    let mut shift = SHIFT;
    for _ in 0..(n_bits_ext - prev_bits) {
        shift = shift * shift;
    }
    shift
}

fn evaluate_extension_polynomial(coefficients: &[Ext3], point: Ext3) -> Ext3 {
    coefficients
        .iter()
        .rev()
        .fold(Ext3::ZERO, |acc, coefficient| acc * point + *coefficient)
}

fn scale_extension(value: Ext3, scalar: Felt) -> Ext3 {
    Ext3::new(value.c0 * scalar, value.c1 * scalar, value.c2 * scalar)
}

fn ordered_opening_layers(
    fri: &PcsFriOpeningUnitSegment,
    expected_count: usize,
) -> Result<Vec<&PcsFriOpeningLayerSegment>, PcsFriOpeningFoldError> {
    let mut layers = Vec::with_capacity(expected_count);
    for layer_index in 0..expected_count {
        let layer_index_u32 =
            u32::try_from(layer_index).map_err(|_| PcsFriOpeningFoldError::LengthOverflow)?;
        let layer = fri
            .layers
            .iter()
            .find(|layer| layer.layer_index == layer_index_u32)
            .ok_or(PcsFriOpeningFoldError::MissingLayer {
                layer_index: layer_index_u32,
            })?;
        layers.push(layer);
    }
    Ok(layers)
}

fn domain_size(bits: u32) -> Result<u64, PcsFriOpeningFoldError> {
    1_u64
        .checked_shl(bits)
        .ok_or(PcsFriOpeningFoldError::UnsupportedDomainBits { bits })
}

fn convert_ext(values: [u64; 3]) -> Result<Ext3, PcsFriOpeningFoldError> {
    Ok(Ext3::new(
        convert_felt(values[0])?,
        convert_felt(values[1])?,
        convert_felt(values[2])?,
    ))
}

fn convert_felt(value: u64) -> Result<Felt, PcsFriOpeningFoldError> {
    Felt::from_canonical(value).map_err(|error| match error {
        FieldError::NonCanonical { value } => PcsFriOpeningFoldError::NonCanonicalField { value },
    })
}
