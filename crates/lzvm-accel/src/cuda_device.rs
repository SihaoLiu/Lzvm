use std::time::Instant;

use super::{cuda_status, AccelError};

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct LzvmCudaMemoryInfo {
    free_bytes: usize,
    total_bytes: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CudaMemoryInfo {
    pub free_bytes: usize,
    pub total_bytes: usize,
}

unsafe extern "C" {
    fn lzvm_cuda_memory_info(out: *mut LzvmCudaMemoryInfo) -> i32;
    fn lzvm_cuda_synchronize() -> i32;
}

pub fn cuda_memory_info() -> Result<CudaMemoryInfo, AccelError> {
    let started = Instant::now();
    let mut raw = LzvmCudaMemoryInfo::default();
    let code = unsafe { lzvm_cuda_memory_info(&mut raw) };
    super::cuda_setup::record_cuda_memory_info_duration(started.elapsed());
    cuda_status(code)?;
    Ok(CudaMemoryInfo {
        free_bytes: raw.free_bytes,
        total_bytes: raw.total_bytes,
    })
}

pub fn cuda_device_synchronize() -> Result<(), AccelError> {
    let code = unsafe { lzvm_cuda_synchronize() };
    cuda_status(code)
}
