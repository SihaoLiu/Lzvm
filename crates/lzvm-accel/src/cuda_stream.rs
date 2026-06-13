use std::ffi::c_void;
use std::ptr;

use super::{cuda_status, AccelError};

unsafe extern "C" {
    fn lzvm_cuda_stream_create(out: *mut *mut c_void) -> i32;
    fn lzvm_cuda_stream_destroy(stream: *mut c_void) -> i32;
    fn lzvm_cuda_stream_synchronize(stream: *mut c_void) -> i32;
}

#[derive(Debug)]
pub struct CudaStream {
    raw: *mut c_void,
}

unsafe impl Send for CudaStream {}
unsafe impl Sync for CudaStream {}

impl CudaStream {
    pub fn new() -> Result<Self, AccelError> {
        let mut raw = ptr::null_mut();
        let code = unsafe { lzvm_cuda_stream_create(&mut raw) };
        cuda_status(code)?;
        if raw.is_null() {
            return Err(AccelError::Cuda { code: -1 });
        }
        Ok(Self { raw })
    }

    pub fn synchronize(&self) -> Result<(), AccelError> {
        let code = unsafe { lzvm_cuda_stream_synchronize(self.raw) };
        cuda_status(code)
    }

    pub(crate) fn as_raw(&self) -> *mut c_void {
        self.raw
    }
}

impl Drop for CudaStream {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            let _ = unsafe { lzvm_cuda_stream_destroy(self.raw) };
            self.raw = ptr::null_mut();
        }
    }
}
