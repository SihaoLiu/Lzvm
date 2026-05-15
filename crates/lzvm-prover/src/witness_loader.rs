use std::fmt;
use std::os::raw::c_int;
use std::path::{Path, PathBuf};

use libloading::Library;

pub const WITNESS_ABI_VERSION: u32 = 1;

type WitnessAbiVersionFn = unsafe extern "C" fn() -> u32;
type WitnessComputeFn = unsafe extern "C" fn() -> c_int;

pub struct LoadedWitnessLibrary {
    pub path: PathBuf,
    pub abi_version: u32,
    compute: WitnessComputeFn,
    _library: Library,
}

impl LoadedWitnessLibrary {
    /// # Safety
    ///
    /// This only belongs in loader smoke tests until the runtime passes a real witness context.
    pub unsafe fn call_compute_for_smoke(&self) -> c_int {
        (self.compute)()
    }
}

impl fmt::Debug for LoadedWitnessLibrary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LoadedWitnessLibrary")
            .field("path", &self.path)
            .field("abi_version", &self.abi_version)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WitnessLoadError {
    Open {
        path: PathBuf,
        message: String,
    },
    MissingAbiVersion {
        path: PathBuf,
        message: String,
    },
    MissingCompute {
        path: PathBuf,
        message: String,
    },
    UnsupportedAbiVersion {
        path: PathBuf,
        expected: u32,
        found: u32,
    },
}

impl fmt::Display for WitnessLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Open { path, message } => {
                write!(f, "witness library open failed: {}: {message}", path.display())
            }
            Self::MissingAbiVersion { path, message } => write!(
                f,
                "witness library ABI version symbol is missing: {}: {message}",
                path.display()
            ),
            Self::MissingCompute { path, message } => write!(
                f,
                "witness library compute symbol is missing: {}: {message}",
                path.display()
            ),
            Self::UnsupportedAbiVersion {
                path,
                expected,
                found,
            } => write!(
                f,
                "witness library ABI version is unsupported: {}: expected {expected}, found {found}",
                path.display()
            ),
        }
    }
}

impl std::error::Error for WitnessLoadError {}

pub fn load_witness_library(
    path: impl AsRef<Path>,
) -> Result<LoadedWitnessLibrary, WitnessLoadError> {
    let path = path.as_ref().to_path_buf();
    let library = unsafe { Library::new(&path) }.map_err(|error| WitnessLoadError::Open {
        path: path.clone(),
        message: error.to_string(),
    })?;

    let abi_version = {
        let symbol = unsafe { library.get::<WitnessAbiVersionFn>(b"lzvm_witness_abi_version\0") }
            .map_err(|error| WitnessLoadError::MissingAbiVersion {
            path: path.clone(),
            message: error.to_string(),
        })?;
        unsafe { symbol() }
    };

    if abi_version != WITNESS_ABI_VERSION {
        return Err(WitnessLoadError::UnsupportedAbiVersion {
            path,
            expected: WITNESS_ABI_VERSION,
            found: abi_version,
        });
    }

    let compute = {
        let symbol = unsafe { library.get::<WitnessComputeFn>(b"lzvm_witness_compute\0") }
            .map_err(|error| WitnessLoadError::MissingCompute {
                path: path.clone(),
                message: error.to_string(),
            })?;
        *symbol
    };

    Ok(LoadedWitnessLibrary {
        path,
        abi_version,
        compute,
        _library: library,
    })
}
