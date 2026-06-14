use std::ffi::c_void;
use std::ptr;

use super::{cuda_status, AccelError};

unsafe extern "C" {
    fn lzvm_cuda_alloc_bytes(out: *mut *mut c_void, bytes: usize) -> i32;
    fn lzvm_cuda_free_bytes(ptr: *mut c_void);
    #[cfg(test)]
    fn lzvm_cuda_allocator_clear_cache() -> i32;
    fn lzvm_cuda_allocator_stats(out: *mut CudaAllocatorStats) -> i32;
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

#[repr(C)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct CudaAllocatorStats {
    pub cuda_malloc_calls: usize,
    pub cuda_malloc_bytes: usize,
    pub cuda_malloc_wait_ns: usize,
    pub cuda_malloc_max_wait_ns: usize,
    pub cuda_host_register_calls: usize,
    pub cuda_host_register_bytes: usize,
    pub cuda_host_register_wait_ns: usize,
    pub cuda_host_register_max_wait_ns: usize,
    pub cuda_host_unregister_calls: usize,
    pub cuda_host_unregister_wait_ns: usize,
    pub cuda_host_unregister_max_wait_ns: usize,
    pub cuda_copy_h2d_calls: usize,
    pub cuda_copy_h2d_bytes: usize,
    pub cuda_copy_h2d_wait_ns: usize,
    pub cuda_copy_h2d_max_wait_ns: usize,
    pub cuda_copy_d2h_calls: usize,
    pub cuda_copy_d2h_bytes: usize,
    pub cuda_copy_d2h_wait_ns: usize,
    pub cuda_copy_d2h_max_wait_ns: usize,
    pub cuda_copy_d2h_hot_bytes: usize,
    pub cuda_copy_d2h_hot_count: usize,
    pub cuda_copy_d2h_hot_wait_ns: usize,
    pub cuda_copy_d2d_calls: usize,
    pub cuda_copy_d2d_bytes: usize,
    pub cuda_copy_d2d_wait_ns: usize,
    pub cuda_copy_d2d_max_wait_ns: usize,
    pub cuda_free_calls: usize,
    pub cuda_device_synchronize_calls: usize,
    pub cached_blocks: usize,
    pub cached_bytes: usize,
    pub cuda_event_query_calls: usize,
    pub cuda_event_query_ready_count: usize,
    pub cuda_event_query_not_ready_count: usize,
    pub cuda_event_synchronize_calls: usize,
    pub cuda_event_synchronize_bytes: usize,
    pub cuda_event_synchronize_max_bytes: usize,
    pub cuda_event_synchronize_wait_ns: usize,
    pub cuda_event_synchronize_max_wait_ns: usize,
    pub cuda_event_synchronize_hot_bytes: usize,
    pub cuda_event_synchronize_hot_count: usize,
    pub cuda_event_synchronize_hot_wait_ns: usize,
    pub cached_reuse_count: usize,
    pub pending_reuse_count: usize,
    pub no_wait_bypass_count: usize,
    pub no_wait_bypass_bytes: usize,
}

pub fn cuda_allocator_stats() -> Result<CudaAllocatorStats, AccelError> {
    let mut stats = CudaAllocatorStats::default();
    let code = unsafe { lzvm_cuda_allocator_stats(&mut stats) };
    cuda_status(code)?;
    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CudaDeviceBuffer;

    fn clear_allocator_cache() {
        let code = unsafe { lzvm_cuda_allocator_clear_cache() };
        cuda_status(code).expect("allocator cache should clear");
    }

    #[test]
    fn cuda_device_buffer_reuses_freed_same_size_allocation_without_device_synchronizing() {
        let _guard = crate::CUDA_TEST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        clear_allocator_cache();

        {
            let _buffer = CudaDeviceBuffer::new(4096).expect("first allocation should succeed");
        }
        let after_first = cuda_allocator_stats().expect("allocator stats should load");
        assert_eq!(after_first.cuda_malloc_calls, 1);
        assert_eq!(after_first.cuda_malloc_bytes, 4096);
        assert_eq!(after_first.cuda_free_calls, 0);
        assert_eq!(after_first.cuda_device_synchronize_calls, 0);
        assert_eq!(after_first.cached_blocks, 1);

        {
            let _buffer = CudaDeviceBuffer::new(4096).expect("cached allocation should succeed");
        }
        let after_second = cuda_allocator_stats().expect("allocator stats should load");

        assert_eq!(after_second.cuda_malloc_calls, 1);
        assert_eq!(after_second.cuda_malloc_bytes, 4096);
        assert_eq!(after_second.cuda_free_calls, 0);
        assert_eq!(after_second.cuda_device_synchronize_calls, 0);
        assert_eq!(after_second.cached_reuse_count, 1);
        assert_eq!(after_second.cached_blocks, 1);

        clear_allocator_cache();
    }
}
