#[cfg(feature = "cuda")]
const LARGE_GUEST_PC_TRACE_MIN_FREE_GPU_BYTES: usize = 1024 * 1024 * 1024;
const GUEST_PC_TRACE_GPU_SIZE_THRESHOLD: u64 = 1_000_000;

pub(super) fn validate_large_guest_pc_gpu(
    instruction_limit: Option<u64>,
) -> Result<(), &'static str> {
    match is_large_guest_pc_trace(instruction_limit) && !lzvm_prover::gpu_setup_available() {
        true => Err("large --guest-pc-trace runs require a CUDA-enabled lzvm-cli build"),
        false => Ok(()),
    }
}

pub(super) fn validate_large_guest_pc_runtime_gpu(
    instruction_limit: Option<u64>,
) -> Result<(), String> {
    if !is_large_guest_pc_trace(instruction_limit) || !lzvm_prover::gpu_setup_available() {
        return Ok(());
    }
    #[cfg(feature = "cuda")]
    {
        let info = lzvm_prover::gpu_memory_info().map_err(large_guest_pc_gpu_memory_query_error)?;
        validate_large_guest_pc_gpu_memory(info, LARGE_GUEST_PC_TRACE_MIN_FREE_GPU_BYTES)
    }
    #[cfg(not(feature = "cuda"))]
    {
        Ok(())
    }
}

fn is_large_guest_pc_trace(instruction_limit: Option<u64>) -> bool {
    instruction_limit.unwrap_or(0) >= GUEST_PC_TRACE_GPU_SIZE_THRESHOLD
}

#[cfg(feature = "cuda")]
fn large_guest_pc_gpu_memory_query_error(error: lzvm_prover::GpuSetupError) -> String {
    if error.is_cuda_out_of_memory() {
        return "large --guest-pc-trace GPU memory preflight failed: CUDA reported out of memory while querying free memory; free GPU memory and retry".to_owned();
    }
    format!("large --guest-pc-trace GPU memory preflight failed: {error}")
}

#[cfg(any(test, feature = "cuda"))]
pub(super) fn validate_large_guest_pc_gpu_memory(
    info: lzvm_prover::GpuMemoryInfo,
    min_free_bytes: usize,
) -> Result<(), String> {
    if info.free_bytes >= min_free_bytes {
        return Ok(());
    }
    Err(format!(
        "large --guest-pc-trace requires at least {} MiB free CUDA memory: free {} MiB of {} MiB",
        bytes_to_mib_ceil(min_free_bytes),
        bytes_to_mib_floor(info.free_bytes),
        bytes_to_mib_floor(info.total_bytes)
    ))
}

#[cfg(any(test, feature = "cuda"))]
fn bytes_to_mib_floor(bytes: usize) -> usize {
    bytes / (1024 * 1024)
}

#[cfg(any(test, feature = "cuda"))]
fn bytes_to_mib_ceil(bytes: usize) -> usize {
    bytes.saturating_add(1024 * 1024 - 1) / (1024 * 1024)
}
