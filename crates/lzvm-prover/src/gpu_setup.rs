use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GpuSetupError {
    #[cfg(feature = "cuda")]
    Accel(lzvm_accel::AccelError),
    #[cfg(not(feature = "cuda"))]
    CudaUnavailable,
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

pub fn gpu_setup_available() -> bool {
    cfg!(feature = "cuda")
}

#[cfg(feature = "cuda")]
pub fn prepare_gpu_setup(max_extended_domain_bits: usize) -> Result<(), GpuSetupError> {
    lzvm_accel::cuda_setup_init(max_extended_domain_bits).map_err(GpuSetupError::Accel)
}

#[cfg(not(feature = "cuda"))]
pub fn prepare_gpu_setup(_max_extended_domain_bits: usize) -> Result<(), GpuSetupError> {
    Err(GpuSetupError::CudaUnavailable)
}
