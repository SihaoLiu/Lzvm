use std::ffi::c_void;
use std::ptr;

use super::{cuda_status, AccelError};

unsafe extern "C" {
    fn lzvm_cuda_alloc_bytes(out: *mut *mut c_void, bytes: usize) -> i32;
    fn lzvm_cuda_free_bytes(ptr: *mut c_void);
    #[cfg(test)]
    fn lzvm_cuda_allocator_clear_cache() -> i32;
    #[cfg(test)]
    fn lzvm_cuda_allocator_stats(out: *mut LzvmCudaAllocatorStats) -> i32;
}

pub(crate) fn alloc_bytes(len: usize) -> Result<*mut c_void, AccelError> {
    let mut ptr = ptr::null_mut();
    let code = unsafe { lzvm_cuda_alloc_bytes(&mut ptr, len) };
    cuda_status(code)?;
    Ok(ptr)
}

pub(crate) fn free_bytes(ptr: *mut c_void) {
    unsafe {
        lzvm_cuda_free_bytes(ptr);
    }
}

#[cfg(test)]
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct LzvmCudaAllocatorStats {
    cuda_malloc_calls: usize,
    cuda_free_calls: usize,
    cuda_device_synchronize_calls: usize,
    cached_blocks: usize,
    cached_bytes: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CudaDeviceBuffer;

    fn clear_allocator_cache() {
        let code = unsafe { lzvm_cuda_allocator_clear_cache() };
        cuda_status(code).expect("allocator cache should clear");
    }

    fn allocator_stats() -> LzvmCudaAllocatorStats {
        let mut stats = LzvmCudaAllocatorStats::default();
        let code = unsafe { lzvm_cuda_allocator_stats(&mut stats) };
        cuda_status(code).expect("allocator stats should load");
        stats
    }

    #[test]
    fn cuda_device_buffer_reuses_freed_same_size_allocation() {
        clear_allocator_cache();

        {
            let _buffer = CudaDeviceBuffer::new(4096).expect("first allocation should succeed");
        }
        let after_first = allocator_stats();
        assert_eq!(after_first.cuda_malloc_calls, 1);
        assert_eq!(after_first.cuda_free_calls, 0);
        assert_eq!(after_first.cuda_device_synchronize_calls, 1);
        assert_eq!(after_first.cached_blocks, 1);

        {
            let _buffer = CudaDeviceBuffer::new(4096).expect("cached allocation should succeed");
        }
        let after_second = allocator_stats();

        assert_eq!(after_second.cuda_malloc_calls, 1);
        assert_eq!(after_second.cuda_free_calls, 0);
        assert_eq!(after_second.cuda_device_synchronize_calls, 2);
        assert_eq!(after_second.cached_blocks, 1);

        clear_allocator_cache();
    }
}
