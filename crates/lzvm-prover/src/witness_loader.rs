use std::borrow::Cow;
use std::fmt;
use std::os::raw::c_int;
use std::path::{Path, PathBuf};

use libloading::Library;
use lzvm_artifacts::guest_image::GuestImageInfo;

use crate::witness_layout::WitnessTraceLayout;

pub const WITNESS_ABI_VERSION: u32 = 1;
pub const WITNESS_STATUS_OK: c_int = 0;

type WitnessAbiVersionFn = unsafe extern "C" fn() -> u32;
type WitnessComputeFn = unsafe extern "C" fn(*const WitnessCall, *mut WitnessResult) -> c_int;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WitnessCall {
    pub input_ptr: *const u8,
    pub input_len: usize,
    pub output_ptr: *mut u8,
    pub output_len: usize,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WitnessResult {
    pub status: c_int,
    pub produced_len: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WitnessTraceBuffers<'a> {
    input: Cow<'a, [u8]>,
    output: Vec<u8>,
}

impl<'a> WitnessTraceBuffers<'a> {
    pub fn new(
        input: impl Into<Cow<'a, [u8]>>,
        output_len: usize,
    ) -> Result<Self, WitnessCallError> {
        if output_len == 0 {
            return Err(WitnessCallError::EmptyOutputBuffer);
        }
        Ok(Self {
            input: input.into(),
            output: vec![0; output_len],
        })
    }

    pub fn input(&self) -> &[u8] {
        &self.input
    }

    pub fn output(&self) -> &[u8] {
        &self.output
    }

    pub fn output_mut(&mut self) -> &mut [u8] {
        &mut self.output
    }

    fn as_call(&mut self) -> WitnessCall {
        WitnessCall {
            input_ptr: self.input().as_ptr(),
            input_len: self.input.len(),
            output_ptr: self.output.as_mut_ptr(),
            output_len: self.output.len(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WitnessTraceOutput {
    pub produced_len: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WitnessComputeContext<'a> {
    pub guest_image: Option<&'a Path>,
    pub guest_image_info: Option<&'a GuestImageInfo>,
    pub trace_layout: Option<&'a WitnessTraceLayout>,
}

impl WitnessComputeContext<'_> {
    pub fn empty() -> Self {
        Self {
            guest_image: None,
            guest_image_info: None,
            trace_layout: None,
        }
    }
}

impl Default for WitnessComputeContext<'_> {
    fn default() -> Self {
        Self::empty()
    }
}

pub trait WitnessBackend {
    fn compute(
        &self,
        buffers: &mut WitnessTraceBuffers<'_>,
    ) -> Result<WitnessTraceOutput, WitnessCallError>;

    fn compute_with_context(
        &self,
        _context: WitnessComputeContext<'_>,
        buffers: &mut WitnessTraceBuffers<'_>,
    ) -> Result<WitnessTraceOutput, WitnessCallError> {
        self.compute(buffers)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceBytesBackend<'a> {
    trace_bytes: Cow<'a, [u8]>,
}

impl TraceBytesBackend<'static> {
    pub fn new(trace_bytes: Vec<u8>) -> Self {
        Self {
            trace_bytes: Cow::Owned(trace_bytes),
        }
    }
}

impl<'a> TraceBytesBackend<'a> {
    pub fn borrowed(trace_bytes: &'a [u8]) -> Self {
        Self {
            trace_bytes: Cow::Borrowed(trace_bytes),
        }
    }

    pub fn trace_bytes(&self) -> &[u8] {
        &self.trace_bytes
    }
}

impl WitnessBackend for TraceBytesBackend<'_> {
    fn compute(
        &self,
        buffers: &mut WitnessTraceBuffers<'_>,
    ) -> Result<WitnessTraceOutput, WitnessCallError> {
        let produced_len = self.trace_bytes.len();
        let output_len = buffers.output().len();
        if produced_len > output_len {
            return Err(WitnessCallError::OutputOverflow {
                produced_len,
                output_len,
            });
        }
        buffers.output_mut()[..produced_len].copy_from_slice(&self.trace_bytes);
        Ok(WitnessTraceOutput { produced_len })
    }
}

pub struct LoadedWitnessLibrary {
    pub path: PathBuf,
    pub abi_version: u32,
    compute: WitnessComputeFn,
    _library: Library,
}

impl LoadedWitnessLibrary {
    pub fn compute(
        &self,
        buffers: &mut WitnessTraceBuffers<'_>,
    ) -> Result<WitnessTraceOutput, WitnessCallError> {
        let call = buffers.as_call();
        let mut result = WitnessResult::default();
        let return_code = unsafe { self.compute_unchecked(&call, &mut result) };
        if return_code != WITNESS_STATUS_OK {
            return Err(WitnessCallError::NativeReturn { code: return_code });
        }
        if result.status != WITNESS_STATUS_OK {
            return Err(WitnessCallError::NativeStatus {
                status: result.status,
            });
        }
        if result.produced_len > buffers.output.len() {
            return Err(WitnessCallError::OutputOverflow {
                produced_len: result.produced_len,
                output_len: buffers.output.len(),
            });
        }
        Ok(WitnessTraceOutput {
            produced_len: result.produced_len,
        })
    }

    /// # Safety
    ///
    /// The caller must ensure that all pointers in `call` and `result` are valid for the loaded
    /// native library during the call.
    pub unsafe fn compute_unchecked(
        &self,
        call: &WitnessCall,
        result: &mut WitnessResult,
    ) -> c_int {
        (self.compute)(call as *const WitnessCall, result as *mut WitnessResult)
    }
}

impl WitnessBackend for LoadedWitnessLibrary {
    fn compute(
        &self,
        buffers: &mut WitnessTraceBuffers<'_>,
    ) -> Result<WitnessTraceOutput, WitnessCallError> {
        LoadedWitnessLibrary::compute(self, buffers)
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
pub enum WitnessCallError {
    EmptyOutputBuffer,
    Backend {
        message: String,
    },
    NativeReturn {
        code: c_int,
    },
    NativeStatus {
        status: c_int,
    },
    OutputOverflow {
        produced_len: usize,
        output_len: usize,
    },
}

impl fmt::Display for WitnessCallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyOutputBuffer => write!(f, "witness output buffer is empty"),
            Self::Backend { message } => write!(f, "witness backend failed: {message}"),
            Self::NativeReturn { code } => {
                write!(f, "witness native call returned failure code: {code}")
            }
            Self::NativeStatus { status } => {
                write!(f, "witness native result has failure status: {status}")
            }
            Self::OutputOverflow {
                produced_len,
                output_len,
            } => write!(
                f,
                "witness native result exceeds output buffer: produced {produced_len}, output {output_len}"
            ),
        }
    }
}

impl std::error::Error for WitnessCallError {}

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
