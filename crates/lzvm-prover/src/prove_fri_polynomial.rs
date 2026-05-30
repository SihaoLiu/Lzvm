use std::fmt;
use std::path::{Path, PathBuf};

#[cfg(feature = "cuda")]
use lzvm_accel::{
    cuda_goldilocks_coset_extend_row_major_columns_device,
    cuda_goldilocks_coset_extend_row_major_columns_output_bytes, CudaDeviceBuffer,
};
use lzvm_artifacts::fixed::FixedColumns;
#[cfg(not(feature = "cuda"))]
use lzvm_field::coset_extend_evaluations;
#[cfg(feature = "cuda")]
use lzvm_field::FieldError;
use lzvm_field::{DomainError, Ext3, Felt};

use crate::fixed_material::{load_fixed_columns_material, FixedColumnsMaterialError};
use crate::fri_polynomial::{
    build_fri_domain_points, build_fri_polynomial, derive_opening_xis, FriPolynomialColumnMatrix,
    FriPolynomialError, FriPolynomialInputs, FriPolynomialStageColumns, FriPolynomialZerofierTable,
};
#[cfg(feature = "cuda")]
use crate::gpu_setup::{prepare_gpu_setup, GpuSetupError};
use crate::witness_commitment::{extend_witness_trace_stage_values, WitnessTraceCommitmentError};
use crate::witness_trace::WitnessTraceBuffer;
use crate::{ProveExecutionUnitArtifacts, ProveUnitSchedule, ProveWitnessAuxiliaryInputs};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProvePcsFriPolynomialError {
    MissingFriExpression {
        unit_index: usize,
    },
    FixedColumns {
        unit_index: usize,
        path: PathBuf,
        source: Box<FixedColumnsMaterialError>,
    },
    FixedRowCountTooLarge {
        unit_index: usize,
        path: PathBuf,
        rows: u64,
    },
    FixedRowCountMismatch {
        unit_index: usize,
        path: PathBuf,
        expected: usize,
        found: usize,
    },
    FixedColumnCountMismatch {
        unit_index: usize,
        path: PathBuf,
        expected: usize,
        found: usize,
    },
    FixedColumnValueCountMismatch {
        unit_index: usize,
        path: PathBuf,
        column: String,
        expected: usize,
        found: usize,
    },
    FixedColumnValueCountOverflow {
        unit_index: usize,
        path: PathBuf,
    },
    FixedColumnNonCanonical {
        unit_index: usize,
        path: PathBuf,
        index: usize,
        value: u64,
    },
    FixedExtension {
        unit_index: usize,
        source: DomainError,
    },
    #[cfg(feature = "cuda")]
    FixedExtensionGpuSetup {
        unit_index: usize,
        source: GpuSetupError,
    },
    #[cfg(feature = "cuda")]
    FixedExtensionCuda {
        unit_index: usize,
        source: lzvm_accel::AccelError,
    },
    #[cfg(feature = "cuda")]
    FixedExtensionValue {
        unit_index: usize,
        source: FieldError,
    },
    StageInput {
        unit_index: usize,
        source: WitnessTraceCommitmentError,
    },
    StageIndexTooLarge {
        unit_index: usize,
        stage_index: usize,
    },
    FriPolynomial {
        unit_index: usize,
        source: Box<FriPolynomialError>,
    },
    LengthOverflow {
        unit_index: usize,
    },
}

impl fmt::Display for ProvePcsFriPolynomialError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingFriExpression { unit_index } => write!(
                f,
                "prove PCS FRI polynomial unit {unit_index} is missing expression id"
            ),
            Self::FixedColumns {
                unit_index,
                path,
                source,
            } => write!(
                f,
                "prove PCS FRI polynomial fixed input load failed for unit {unit_index} at {}: {source}",
                path.display()
            ),
            Self::FixedRowCountTooLarge {
                unit_index,
                path,
                rows,
            } => write!(
                f,
                "prove PCS FRI polynomial fixed row count is too large for unit {unit_index} at {}: {rows}",
                path.display()
            ),
            Self::FixedRowCountMismatch {
                unit_index,
                path,
                expected,
                found,
            } => write!(
                f,
                "prove PCS FRI polynomial fixed row count mismatch for unit {unit_index} at {}: expected {expected}, found {found}",
                path.display()
            ),
            Self::FixedColumnCountMismatch {
                unit_index,
                path,
                expected,
                found,
            } => write!(
                f,
                "prove PCS FRI polynomial fixed column count mismatch for unit {unit_index} at {}: expected {expected}, found {found}",
                path.display()
            ),
            Self::FixedColumnValueCountMismatch {
                unit_index,
                path,
                column,
                expected,
                found,
            } => write!(
                f,
                "prove PCS FRI polynomial fixed column {column} mismatch for unit {unit_index} at {}: expected {expected}, found {found}",
                path.display()
            ),
            Self::FixedColumnValueCountOverflow { unit_index, path } => write!(
                f,
                "prove PCS FRI polynomial fixed value count overflow for unit {unit_index} at {}",
                path.display()
            ),
            Self::FixedColumnNonCanonical {
                unit_index,
                path,
                index,
                value,
            } => write!(
                f,
                "prove PCS FRI polynomial fixed value {index} is non-canonical for unit {unit_index} at {}: {value}",
                path.display()
            ),
            Self::FixedExtension { unit_index, source } => write!(
                f,
                "prove PCS FRI polynomial fixed extension failed for unit {unit_index}: {source}"
            ),
            #[cfg(feature = "cuda")]
            Self::FixedExtensionGpuSetup { unit_index, source } => write!(
                f,
                "prove PCS FRI polynomial fixed GPU setup failed for unit {unit_index}: {source}"
            ),
            #[cfg(feature = "cuda")]
            Self::FixedExtensionCuda { unit_index, source } => write!(
                f,
                "prove PCS FRI polynomial fixed cuda extension failed for unit {unit_index}: {source}"
            ),
            #[cfg(feature = "cuda")]
            Self::FixedExtensionValue { unit_index, source } => write!(
                f,
                "prove PCS FRI polynomial fixed extension value failed for unit {unit_index}: {source}"
            ),
            Self::StageInput { unit_index, source } => write!(
                f,
                "prove PCS FRI polynomial stage input failed for unit {unit_index}: {source}"
            ),
            Self::StageIndexTooLarge {
                unit_index,
                stage_index,
            } => write!(
                f,
                "prove PCS FRI polynomial unit {unit_index} stage index does not fit u16: {stage_index}"
            ),
            Self::FriPolynomial { unit_index, source } => write!(
                f,
                "prove PCS FRI polynomial build failed for unit {unit_index}: {source}"
            ),
            Self::LengthOverflow { unit_index } => write!(
                f,
                "prove PCS FRI polynomial length overflow for unit {unit_index}"
            ),
        }
    }
}

impl std::error::Error for ProvePcsFriPolynomialError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::FixedColumns { source, .. } => Some(source),
            Self::FixedExtension { source, .. } => Some(source),
            #[cfg(feature = "cuda")]
            Self::FixedExtensionGpuSetup { source, .. } => Some(source),
            #[cfg(feature = "cuda")]
            Self::FixedExtensionCuda { source, .. } => Some(source),
            #[cfg(feature = "cuda")]
            Self::FixedExtensionValue { source, .. } => Some(source),
            Self::StageInput { source, .. } => Some(source),
            Self::FriPolynomial { source, .. } => Some(source.as_ref()),
            Self::MissingFriExpression { .. }
            | Self::FixedRowCountTooLarge { .. }
            | Self::FixedRowCountMismatch { .. }
            | Self::FixedColumnCountMismatch { .. }
            | Self::FixedColumnValueCountMismatch { .. }
            | Self::FixedColumnValueCountOverflow { .. }
            | Self::FixedColumnNonCanonical { .. }
            | Self::StageIndexTooLarge { .. }
            | Self::LengthOverflow { .. } => None,
        }
    }
}

pub fn build_pcs_fri_polynomial_values(
    unit_index: usize,
    unit: &ProveUnitSchedule,
    plan_unit: &ProveExecutionUnitArtifacts,
    trace: &WitnessTraceBuffer,
    publics: &[Felt],
    auxiliary_inputs: &ProveWitnessAuxiliaryInputs,
    xi_challenge: Ext3,
) -> Result<Vec<Ext3>, ProvePcsFriPolynomialError> {
    let expression_id = plan_unit
        .fri_expression_id
        .ok_or(ProvePcsFriPolynomialError::MissingFriExpression { unit_index })?;
    let domain_size = usize::try_from(unit.extended_domain_size)
        .map_err(|_| ProvePcsFriPolynomialError::LengthOverflow { unit_index })?;
    let fixed_values = read_extended_fixed_columns(unit_index, unit, plan_unit)?;
    let stage_values = extend_witness_trace_stage_values(trace, unit)
        .map_err(|source| ProvePcsFriPolynomialError::StageInput { unit_index, source })?;
    let mut stage_columns = Vec::with_capacity(stage_values.len());
    for stage in &stage_values {
        let stage_index = u16::try_from(stage.stage_index()).map_err(|_| {
            ProvePcsFriPolynomialError::StageIndexTooLarge {
                unit_index,
                stage_index: stage.stage_index(),
            }
        })?;
        stage_columns.push(FriPolynomialStageColumns {
            stage_index,
            column_count: stage.column_count(),
            values: stage.values(),
        });
    }
    let domain_points = build_fri_domain_points(unit.extended_domain_bits)
        .map_err(|source| fri_error(unit_index, source))?;
    let zerofiers = FriPolynomialZerofierTable::build(
        unit.base_domain_bits,
        unit.extended_domain_bits,
        &plan_unit.setup.boundaries,
    )
    .map_err(|source| fri_error(unit_index, source))?;
    let opening_xis = derive_opening_xis(unit.base_domain_bits, &unit.opening_points, xi_challenge)
        .map_err(|source| fri_error(unit_index, source))?;

    build_fri_polynomial(
        &plan_unit.expression_program,
        expression_id,
        FriPolynomialInputs {
            domain_size,
            stage_count: plan_unit.stage_count,
            fixed_columns: FriPolynomialColumnMatrix {
                column_count: plan_unit.fixed_column_count,
                values: &fixed_values,
            },
            stage_columns: &stage_columns,
            custom_fixed_columns: &[],
            opening_point_offsets: &unit.opening_points,
            domain_points: &domain_points,
            zerofier_values: zerofiers.as_matrix(),
            opening_xis: &opening_xis,
            publics,
            unit_values: &auxiliary_inputs.unit_values,
            proof_values: &auxiliary_inputs.proof_values,
            group_values: &auxiliary_inputs.group_values,
            challenges: &auxiliary_inputs.challenges,
            evaluations: &auxiliary_inputs.evaluations,
        },
    )
    .map_err(|source| fri_error(unit_index, source))
}

fn read_extended_fixed_columns(
    unit_index: usize,
    unit: &ProveUnitSchedule,
    plan_unit: &ProveExecutionUnitArtifacts,
) -> Result<Vec<Felt>, ProvePcsFriPolynomialError> {
    let material = load_fixed_columns_material(
        &plan_unit.fixed_columns,
        &plan_unit.setup,
        plan_unit.group_name.clone(),
        plan_unit.unit_name.clone(),
    )
    .map_err(|source| ProvePcsFriPolynomialError::FixedColumns {
        unit_index,
        path: plan_unit.fixed_columns.clone(),
        source: Box::new(source),
    })?;
    let source_rows = usize::try_from(unit.base_domain_size)
        .map_err(|_| ProvePcsFriPolynomialError::LengthOverflow { unit_index })?;
    validate_fixed_columns_shape(
        &material.fixed_columns,
        plan_unit.fixed_column_count,
        source_rows,
        unit_index,
        &plan_unit.fixed_columns,
    )?;
    extend_row_major_columns(
        &material.row_major_values,
        plan_unit.fixed_column_count,
        usize::try_from(unit.base_domain_bits)
            .map_err(|_| ProvePcsFriPolynomialError::LengthOverflow { unit_index })?,
        usize::try_from(unit.extended_domain_bits)
            .map_err(|_| ProvePcsFriPolynomialError::LengthOverflow { unit_index })?,
        unit_index,
    )
}

fn validate_fixed_columns_shape(
    fixed_columns: &FixedColumns,
    fixed_column_count: usize,
    row_count: usize,
    unit_index: usize,
    path: &Path,
) -> Result<(), ProvePcsFriPolynomialError> {
    let found_rows = usize::try_from(fixed_columns.row_count).map_err(|_| {
        ProvePcsFriPolynomialError::FixedRowCountTooLarge {
            unit_index,
            path: path.to_path_buf(),
            rows: fixed_columns.row_count,
        }
    })?;
    if found_rows != row_count {
        return Err(ProvePcsFriPolynomialError::FixedRowCountMismatch {
            unit_index,
            path: path.to_path_buf(),
            expected: row_count,
            found: found_rows,
        });
    }
    if fixed_columns.columns.len() != fixed_column_count {
        return Err(ProvePcsFriPolynomialError::FixedColumnCountMismatch {
            unit_index,
            path: path.to_path_buf(),
            expected: fixed_column_count,
            found: fixed_columns.columns.len(),
        });
    }
    Ok(())
}

fn extend_row_major_columns(
    values: &[Felt],
    column_count: usize,
    source_bits: usize,
    target_bits: usize,
    unit_index: usize,
) -> Result<Vec<Felt>, ProvePcsFriPolynomialError> {
    if column_count == 0 {
        return Ok(Vec::new());
    }
    let source_rows = values
        .len()
        .checked_div(column_count)
        .ok_or(ProvePcsFriPolynomialError::LengthOverflow { unit_index })?;
    if source_rows
        .checked_mul(column_count)
        .ok_or(ProvePcsFriPolynomialError::LengthOverflow { unit_index })?
        != values.len()
    {
        return Err(ProvePcsFriPolynomialError::LengthOverflow { unit_index });
    }

    #[cfg(feature = "cuda")]
    {
        let out_byte_count = cuda_goldilocks_coset_extend_row_major_columns_output_bytes(
            values.len(),
            column_count,
            source_bits,
            target_bits,
        )
        .map_err(|source| ProvePcsFriPolynomialError::FixedExtensionCuda { unit_index, source })?;
        prepare_gpu_setup(target_bits).map_err(|source| {
            ProvePcsFriPolynomialError::FixedExtensionGpuSetup { unit_index, source }
        })?;

        let source_bytes = row_major_felt_bytes(values, unit_index)?;
        let mut source_buffer = CudaDeviceBuffer::new(source_bytes.len()).map_err(|source| {
            ProvePcsFriPolynomialError::FixedExtensionCuda { unit_index, source }
        })?;
        source_buffer.copy_from(&source_bytes).map_err(|source| {
            ProvePcsFriPolynomialError::FixedExtensionCuda { unit_index, source }
        })?;
        let mut output_buffer = CudaDeviceBuffer::new(out_byte_count).map_err(|source| {
            ProvePcsFriPolynomialError::FixedExtensionCuda { unit_index, source }
        })?;

        cuda_goldilocks_coset_extend_row_major_columns_device(
            &source_buffer,
            &mut output_buffer,
            column_count,
            source_bits,
            target_bits,
        )
        .map_err(|source| ProvePcsFriPolynomialError::FixedExtensionCuda { unit_index, source })?;
        let bytes = output_buffer.to_vec().map_err(|source| {
            ProvePcsFriPolynomialError::FixedExtensionCuda { unit_index, source }
        })?;
        row_major_felts_from_bytes(&bytes, unit_index)
    }

    #[cfg(not(feature = "cuda"))]
    {
        let mut extended_columns = Vec::with_capacity(column_count);
        for column in 0..column_count {
            let mut source = Vec::with_capacity(source_rows);
            for row in 0..source_rows {
                source.push(values[row * column_count + column]);
            }
            extended_columns.push(
                coset_extend_evaluations(&source, source_bits, target_bits).map_err(|source| {
                    ProvePcsFriPolynomialError::FixedExtension { unit_index, source }
                })?,
            );
        }

        let extended_rows = extended_columns.first().map_or(0, Vec::len);
        let mut out = Vec::with_capacity(
            extended_rows
                .checked_mul(column_count)
                .ok_or(ProvePcsFriPolynomialError::LengthOverflow { unit_index })?,
        );
        for row in 0..extended_rows {
            for column_values in &extended_columns {
                out.push(column_values[row]);
            }
        }
        Ok(out)
    }
}

#[cfg(feature = "cuda")]
fn row_major_felt_bytes(
    values: &[Felt],
    unit_index: usize,
) -> Result<Vec<u8>, ProvePcsFriPolynomialError> {
    let mut bytes = Vec::with_capacity(
        values
            .len()
            .checked_mul(8)
            .ok_or(ProvePcsFriPolynomialError::LengthOverflow { unit_index })?,
    );
    for value in values {
        bytes.extend_from_slice(&value.to_u64().to_le_bytes());
    }
    Ok(bytes)
}

#[cfg(feature = "cuda")]
fn row_major_felts_from_bytes(
    bytes: &[u8],
    unit_index: usize,
) -> Result<Vec<Felt>, ProvePcsFriPolynomialError> {
    if !bytes.len().is_multiple_of(8) {
        return Err(ProvePcsFriPolynomialError::LengthOverflow { unit_index });
    }
    let mut values = Vec::with_capacity(bytes.len() / 8);
    for chunk in bytes.chunks_exact(8) {
        let word = u64::from_le_bytes(chunk.try_into().expect("chunk length checked"));
        values.push(Felt::from_canonical(word).map_err(|source| {
            ProvePcsFriPolynomialError::FixedExtensionValue { unit_index, source }
        })?);
    }
    Ok(values)
}

fn fri_error(unit_index: usize, source: FriPolynomialError) -> ProvePcsFriPolynomialError {
    ProvePcsFriPolynomialError::FriPolynomial {
        unit_index,
        source: Box::new(source),
    }
}

#[cfg(all(test, feature = "cuda"))]
mod tests {
    use super::{extend_row_major_columns, ProvePcsFriPolynomialError};
    use lzvm_field::{coset_extend_evaluations, Felt};

    fn extend_row_major_columns_with_cuda(
        values: &[Felt],
        column_count: usize,
        source_bits: usize,
        target_bits: usize,
        unit_index: usize,
    ) -> Result<Vec<Felt>, ProvePcsFriPolynomialError> {
        extend_row_major_columns(values, column_count, source_bits, target_bits, unit_index)
    }

    #[test]
    fn cuda_row_major_extension_matches_cpu_reference() {
        let values = vec![
            Felt::from_u64(5),
            Felt::from_u64(9),
            Felt::from_u64(1),
            Felt::from_u64(9),
        ];

        let cuda = extend_row_major_columns_with_cuda(&values, 2, 1, 2, 7)
            .expect("cuda extension should run");
        let column_0 = coset_extend_evaluations(&[Felt::from_u64(5), Felt::from_u64(1)], 1, 2)
            .expect("first column should extend");
        let column_1 = coset_extend_evaluations(&[Felt::from_u64(9), Felt::from_u64(9)], 1, 2)
            .expect("second column should extend");
        let expected = (0..4)
            .flat_map(|row| [column_0[row], column_1[row]])
            .collect::<Vec<_>>();

        assert_eq!(cuda, expected);
    }

    #[test]
    fn cuda_row_major_extension_rejects_source_domain_mismatch_before_allocation() {
        let values = vec![Felt::from_u64(5), Felt::from_u64(9)];

        let error = extend_row_major_columns_with_cuda(&values, 1, 2, 3, 7)
            .expect_err("source row mismatch should be rejected");

        assert!(matches!(
            error,
            ProvePcsFriPolynomialError::FixedExtensionCuda {
                unit_index: 7,
                source: lzvm_accel::AccelError::InvalidDomain { bits: 2, len: 2 }
            }
        ));
    }
}
