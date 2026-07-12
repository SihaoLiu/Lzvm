use std::fmt;

pub const GUEST_PC_TRACE_GPU_SIZE_THRESHOLD: u64 = 1_000_000;
pub const LARGE_GUEST_PC_TRACE_MIN_FREE_GPU_BYTES: usize = 1024 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GpuSetupError {
    #[cfg(feature = "cuda")]
    Accel(lzvm_accel::AccelError),
    #[cfg(not(feature = "cuda"))]
    CudaUnavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuMemoryInfo {
    pub free_bytes: usize,
    pub total_bytes: usize,
}

impl fmt::Display for GpuSetupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            #[cfg(feature = "cuda")]
            Self::Accel(error) => write!(f, "prover GPU setup failed: {error}"),
            #[cfg(not(feature = "cuda"))]
            Self::CudaUnavailable => write!(f, "prover GPU setup is unavailable"),
        }
    }
}

impl std::error::Error for GpuSetupError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            #[cfg(feature = "cuda")]
            Self::Accel(error) => Some(error),
            #[cfg(not(feature = "cuda"))]
            Self::CudaUnavailable => None,
        }
    }
}

impl GpuSetupError {
    pub fn cuda_error_code(&self) -> Option<i32> {
        match self {
            #[cfg(feature = "cuda")]
            Self::Accel(lzvm_accel::AccelError::Cuda { code }) => Some(*code),
            #[cfg(feature = "cuda")]
            Self::Accel(_) => None,
            #[cfg(not(feature = "cuda"))]
            Self::CudaUnavailable => None,
        }
    }

    pub fn is_cuda_out_of_memory(&self) -> bool {
        self.cuda_error_code() == Some(2)
    }
}

pub fn gpu_setup_available() -> bool {
    cfg!(feature = "cuda")
}

#[cfg(feature = "cuda")]
pub fn gpu_memory_info() -> Result<GpuMemoryInfo, GpuSetupError> {
    lzvm_accel::cuda_memory_info()
        .map(|info| GpuMemoryInfo {
            free_bytes: info.free_bytes,
            total_bytes: info.total_bytes,
        })
        .map_err(GpuSetupError::Accel)
}

#[cfg(not(feature = "cuda"))]
pub fn gpu_memory_info() -> Result<GpuMemoryInfo, GpuSetupError> {
    Err(GpuSetupError::CudaUnavailable)
}

#[cfg(feature = "cuda")]
pub fn prepare_gpu_setup(max_extended_domain_bits: usize) -> Result<(), GpuSetupError> {
    lzvm_accel::cuda_setup_init(max_extended_domain_bits).map_err(GpuSetupError::Accel)
}

#[cfg(not(feature = "cuda"))]
pub fn prepare_gpu_setup(_max_extended_domain_bits: usize) -> Result<(), GpuSetupError> {
    Err(GpuSetupError::CudaUnavailable)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "cuda")]
    #[test]
    fn classifies_cuda_out_of_memory_error_code() {
        let error = GpuSetupError::Accel(lzvm_accel::AccelError::Cuda { code: 2 });

        assert_eq!(error.cuda_error_code(), Some(2));
        assert!(error.is_cuda_out_of_memory());
    }

    #[cfg(feature = "cuda")]
    #[test]
    fn ignores_non_memory_cuda_error_codes() {
        let error = GpuSetupError::Accel(lzvm_accel::AccelError::Cuda { code: 700 });

        assert_eq!(error.cuda_error_code(), Some(700));
        assert!(!error.is_cuda_out_of_memory());
    }

    #[cfg(not(feature = "cuda"))]
    #[test]
    fn unavailable_setup_has_no_cuda_error_code() {
        let error = GpuSetupError::CudaUnavailable;

        assert_eq!(error.cuda_error_code(), None);
        assert!(!error.is_cuda_out_of_memory());
    }
}
