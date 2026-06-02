use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(feature = "cuda")]
use lzvm_accel::{AccelError, CudaDeviceBuffer};
use lzvm_artifacts::fixed::{
    expected_raw_fixed_column_byte_count, parse_fixed_columns, parse_raw_fixed_columns,
    raw_fixed_column_layout, FixedColumnError, FixedColumns, RawFixedColumnLayout,
};
use lzvm_artifacts::setup_info::UnitSetupInfo;
use lzvm_field::{Felt, FieldError};
use sha2::{Digest, Sha256};

use crate::ProveExecutionUnitArtifacts;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FixedColumnsMaterialError {
    Read {
        path: PathBuf,
        source: FixedColumnError,
    },
    ValueCountOverflow {
        path: PathBuf,
    },
    ValueCountMismatch {
        path: PathBuf,
        column: String,
        expected: usize,
        found: usize,
    },
    NonCanonicalValue {
        path: PathBuf,
        index: usize,
        value: u64,
    },
    DigestMismatch {
        path: PathBuf,
        expected: [u8; 32],
        found: [u8; 32],
    },
    #[cfg(feature = "cuda")]
    Device {
        path: PathBuf,
        source: AccelError,
    },
}

impl fmt::Display for FixedColumnsMaterialError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, source } => write!(
                f,
                "fixed columns material read failed for {}: {source}",
                path.display()
            ),
            Self::ValueCountOverflow { path } => write!(
                f,
                "fixed columns material value count overflow for {}",
                path.display()
            ),
            Self::ValueCountMismatch {
                path,
                column,
                expected,
                found,
            } => write!(
                f,
                "fixed columns material value count mismatch for {}: {column}: expected {expected}, found {found}",
                path.display()
            ),
            Self::NonCanonicalValue { path, index, value } => write!(
                f,
                "fixed columns material value is non-canonical for {}: index {index}: {value}",
                path.display()
            ),
            Self::DigestMismatch {
                path,
                expected,
                found,
            } => write!(
                f,
                "fixed columns material digest mismatch for {}: expected {expected:?}, found {found:?}",
                path.display()
            ),
            #[cfg(feature = "cuda")]
            Self::Device { path, source } => write!(
                f,
                "fixed columns material device staging failed for {}: {source}",
                path.display()
            ),
        }
    }
}

impl std::error::Error for FixedColumnsMaterialError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Read { source, .. } => Some(source),
            #[cfg(feature = "cuda")]
            Self::Device { source, .. } => Some(source),
            Self::ValueCountOverflow { .. }
            | Self::ValueCountMismatch { .. }
            | Self::NonCanonicalValue { .. }
            | Self::DigestMismatch { .. } => None,
        }
    }
}

#[derive(Debug)]
pub struct FixedColumnsMaterial {
    pub fixed_columns: FixedColumns,
    pub row_major_values: Vec<Felt>,
    pub raw_bytes: Vec<u8>,
    #[cfg(feature = "cuda")]
    pub device_buffer: Option<CudaDeviceBuffer>,
    #[cfg(feature = "cuda")]
    pub device_buffer_is_row_major: bool,
}

#[cfg(feature = "cuda")]
impl FixedColumnsMaterial {
    pub fn row_major_device_buffer(&self) -> Option<&CudaDeviceBuffer> {
        self.device_buffer
            .as_ref()
            .filter(|_| self.device_buffer_is_row_major)
    }
}

pub fn load_fixed_columns_material(
    path: impl AsRef<Path>,
    setup: &UnitSetupInfo,
    group_name: impl Into<String>,
    unit_name: impl Into<String>,
) -> Result<FixedColumnsMaterial, FixedColumnsMaterialError> {
    load_fixed_columns_material_inner(path, setup, group_name, unit_name, None)
}

pub fn load_fixed_columns_material_with_digest(
    path: impl AsRef<Path>,
    setup: &UnitSetupInfo,
    group_name: impl Into<String>,
    unit_name: impl Into<String>,
    expected_digest: [u8; 32],
) -> Result<FixedColumnsMaterial, FixedColumnsMaterialError> {
    load_fixed_columns_material_inner(path, setup, group_name, unit_name, Some(expected_digest))
}

pub fn load_execution_unit_fixed_columns_material(
    plan_unit: &ProveExecutionUnitArtifacts,
) -> Result<FixedColumnsMaterial, FixedColumnsMaterialError> {
    if let Some(expected_digest) = plan_unit.pcs_material_fixed_column_digest {
        load_fixed_columns_material_with_digest(
            &plan_unit.fixed_columns,
            &plan_unit.setup,
            plan_unit.group_name.clone(),
            plan_unit.unit_name.clone(),
            expected_digest,
        )
    } else {
        load_fixed_columns_material(
            &plan_unit.fixed_columns,
            &plan_unit.setup,
            plan_unit.group_name.clone(),
            plan_unit.unit_name.clone(),
        )
    }
}

fn load_fixed_columns_material_inner(
    path: impl AsRef<Path>,
    setup: &UnitSetupInfo,
    group_name: impl Into<String>,
    unit_name: impl Into<String>,
    expected_digest: Option<[u8; 32]>,
) -> Result<FixedColumnsMaterial, FixedColumnsMaterialError> {
    let path = path.as_ref().to_path_buf();
    let group_name = group_name.into();
    let unit_name = unit_name.into();
    let raw_bytes = fs::read(&path).map_err(|error| FixedColumnsMaterialError::Read {
        path: path.clone(),
        source: FixedColumnError::Io {
            message: error.to_string(),
        },
    })?;
    if let Some(expected) = expected_digest {
        let found: [u8; 32] = Sha256::digest(&raw_bytes).into();
        if found != expected {
            return Err(FixedColumnsMaterialError::DigestMismatch {
                path,
                expected,
                found,
            });
        }
    }

    let (fixed_columns, row_major_values, raw_bytes_are_row_major) =
        match parse_fixed_columns(&raw_bytes) {
            Ok(columns) => {
                let row_major_values = fixed_columns_to_row_major_values(&path, &columns)?;
                (columns, row_major_values, false)
            }
            Err(sectioned_error) => {
                if expected_raw_fixed_column_byte_count(setup).ok() == Some(raw_bytes.len()) {
                    let raw_layout =
                        raw_fixed_column_layout(setup, group_name.clone(), unit_name.clone())
                            .map_err(|source| FixedColumnsMaterialError::Read {
                                path: path.clone(),
                                source,
                            })?;
                    let columns = parse_raw_fixed_columns(&raw_bytes, setup, group_name, unit_name)
                        .map_err(|source| FixedColumnsMaterialError::Read {
                            path: path.clone(),
                            source,
                        })?;
                    let raw_bytes_are_row_major =
                        raw_layout_columns_match_physical_order(&raw_layout);
                    let row_major_values = if raw_bytes_are_row_major {
                        raw_fixed_bytes_to_row_major_values(&path, &raw_bytes)?
                    } else {
                        fixed_columns_to_row_major_values(&path, &columns)?
                    };
                    (columns, row_major_values, raw_bytes_are_row_major)
                } else {
                    return Err(FixedColumnsMaterialError::Read {
                        path: path.clone(),
                        source: sectioned_error,
                    });
                }
            }
        };

    #[cfg(feature = "cuda")]
    let device_buffer = {
        let mut buffer = CudaDeviceBuffer::new(raw_bytes.len()).map_err(|source| {
            FixedColumnsMaterialError::Device {
                path: path.clone(),
                source,
            }
        })?;
        buffer
            .copy_from(&raw_bytes)
            .map_err(|source| FixedColumnsMaterialError::Device {
                path: path.clone(),
                source,
            })?;
        Some(buffer)
    };

    #[cfg(not(feature = "cuda"))]
    let _ = raw_bytes_are_row_major;

    Ok(FixedColumnsMaterial {
        fixed_columns,
        row_major_values,
        raw_bytes,
        #[cfg(feature = "cuda")]
        device_buffer,
        #[cfg(feature = "cuda")]
        device_buffer_is_row_major: raw_bytes_are_row_major,
    })
}

fn fixed_columns_to_row_major_values(
    path: &Path,
    fixed_columns: &FixedColumns,
) -> Result<Vec<Felt>, FixedColumnsMaterialError> {
    let row_count = usize::try_from(fixed_columns.row_count).map_err(|_| {
        FixedColumnsMaterialError::ValueCountOverflow {
            path: path.to_path_buf(),
        }
    })?;
    let column_count = fixed_columns.columns.len();
    let value_count = row_count.checked_mul(column_count).ok_or(
        FixedColumnsMaterialError::ValueCountOverflow {
            path: path.to_path_buf(),
        },
    )?;
    let mut values = vec![Felt::ZERO; value_count];
    for (column_index, column) in fixed_columns.columns.iter().enumerate() {
        if column.values.len() != row_count {
            return Err(FixedColumnsMaterialError::ValueCountMismatch {
                path: path.to_path_buf(),
                column: column.name.clone(),
                expected: row_count,
                found: column.values.len(),
            });
        }
        for (row, value) in column.values.iter().copied().enumerate() {
            let index = row
                .checked_mul(column_count)
                .and_then(|offset| offset.checked_add(column_index))
                .ok_or(FixedColumnsMaterialError::ValueCountOverflow {
                    path: path.to_path_buf(),
                })?;
            values[index] = Felt::from_canonical(value).map_err(|error| match error {
                FieldError::NonCanonical { value } => {
                    FixedColumnsMaterialError::NonCanonicalValue {
                        path: path.to_path_buf(),
                        index,
                        value,
                    }
                }
            })?;
        }
    }
    Ok(values)
}

fn raw_layout_columns_match_physical_order(layout: &RawFixedColumnLayout) -> bool {
    let Ok(column_count) = usize::try_from(layout.column_count) else {
        return false;
    };
    layout.columns.len() == column_count
        && layout
            .columns
            .iter()
            .enumerate()
            .all(|(position, column)| usize::try_from(column.index).ok() == Some(position))
}

fn raw_fixed_bytes_to_row_major_values(
    path: &Path,
    raw_bytes: &[u8],
) -> Result<Vec<Felt>, FixedColumnsMaterialError> {
    if !raw_bytes.len().is_multiple_of(8) {
        return Err(FixedColumnsMaterialError::ValueCountOverflow {
            path: path.to_path_buf(),
        });
    }
    let mut values = Vec::with_capacity(raw_bytes.len() / 8);
    for chunk in raw_bytes.chunks_exact(8) {
        let value = u64::from_le_bytes(chunk.try_into().expect("chunk length checked"));
        debug_assert!(Felt::from_canonical(value).is_ok());
        values.push(Felt::from_u64(value));
    }
    Ok(values)
}
