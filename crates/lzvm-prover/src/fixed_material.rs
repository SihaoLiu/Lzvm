use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(feature = "cuda")]
use lzvm_accel::{AccelError, CudaDeviceBuffer};
use lzvm_artifacts::fixed::{
    expected_raw_fixed_column_byte_count, parse_fixed_columns, parse_raw_fixed_columns,
    FixedColumnError, FixedColumns,
};
use lzvm_artifacts::setup_info::UnitSetupInfo;
use lzvm_field::{Felt, FieldError};

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
            | Self::NonCanonicalValue { .. } => None,
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
}

pub fn load_fixed_columns_material(
    path: impl AsRef<Path>,
    setup: &UnitSetupInfo,
    group_name: impl Into<String>,
    unit_name: impl Into<String>,
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

    let fixed_columns = match parse_fixed_columns(&raw_bytes) {
        Ok(columns) => columns,
        Err(sectioned_error) => {
            if expected_raw_fixed_column_byte_count(setup).ok() == Some(raw_bytes.len()) {
                parse_raw_fixed_columns(&raw_bytes, setup, group_name, unit_name).map_err(
                    |source| FixedColumnsMaterialError::Read {
                        path: path.clone(),
                        source,
                    },
                )?
            } else {
                return Err(FixedColumnsMaterialError::Read {
                    path: path.clone(),
                    source: sectioned_error,
                });
            }
        }
    };

    let row_major_values = fixed_columns_to_row_major_values(&path, &fixed_columns)?;

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

    Ok(FixedColumnsMaterial {
        fixed_columns,
        row_major_values,
        raw_bytes,
        #[cfg(feature = "cuda")]
        device_buffer,
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
