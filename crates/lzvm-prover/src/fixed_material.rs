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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FixedColumnsMaterialError {
    Read {
        path: PathBuf,
        source: FixedColumnError,
    },
    #[cfg(feature = "cuda")]
    Device { path: PathBuf, source: AccelError },
}

impl fmt::Display for FixedColumnsMaterialError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, source } => write!(
                f,
                "fixed columns material read failed for {}: {source}",
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
        }
    }
}

#[derive(Debug)]
pub struct FixedColumnsMaterial {
    pub fixed_columns: FixedColumns,
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
        raw_bytes,
        #[cfg(feature = "cuda")]
        device_buffer,
    })
}
