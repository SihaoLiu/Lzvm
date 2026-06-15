use std::ffi::c_void;
use std::ptr;

use super::{cuda_status, AccelError};

unsafe extern "C" {
    fn lzvm_cuda_stream_create(out: *mut *mut c_void) -> i32;
    fn lzvm_cuda_stream_destroy(stream: *mut c_void) -> i32;
    fn lzvm_cuda_stream_synchronize(stream: *mut c_void) -> i32;
    fn lzvm_cuda_stream_begin_capture(stream: *mut c_void) -> i32;
    fn lzvm_cuda_stream_end_capture(stream: *mut c_void, graph_out: *mut *mut c_void) -> i32;
    fn lzvm_cuda_graph_destroy(graph: *mut c_void) -> i32;
    fn lzvm_cuda_graph_instantiate(graph: *mut c_void, exec_out: *mut *mut c_void) -> i32;
    fn lzvm_cuda_graph_exec_update(exec: *mut c_void, graph: *mut c_void) -> i32;
    fn lzvm_cuda_graph_exec_destroy(exec: *mut c_void) -> i32;
    fn lzvm_cuda_graph_launch(exec: *mut c_void, stream: *mut c_void) -> i32;
    fn lzvm_cuda_event_create(out: *mut *mut c_void) -> i32;
    fn lzvm_cuda_event_destroy(event: *mut c_void) -> i32;
    fn lzvm_cuda_event_record(event: *mut c_void, stream: *mut c_void) -> i32;
    fn lzvm_cuda_event_synchronize(event: *mut c_void) -> i32;
    fn lzvm_cuda_stream_wait_event(stream: *mut c_void, event: *mut c_void) -> i32;
}

#[derive(Debug)]
pub struct CudaStream {
    raw: *mut c_void,
}

#[derive(Debug)]
pub struct CudaGraphCapture<'a> {
    stream: &'a CudaStream,
    active: bool,
}

#[derive(Debug)]
pub struct CudaGraph {
    raw: *mut c_void,
}

#[derive(Debug)]
pub struct CudaGraphExec {
    raw: *mut c_void,
}

#[derive(Debug)]
pub struct CudaEvent {
    raw: *mut c_void,
}

unsafe impl Send for CudaStream {}
unsafe impl Sync for CudaStream {}
unsafe impl Send for CudaGraph {}
unsafe impl Sync for CudaGraph {}
unsafe impl Send for CudaGraphExec {}
unsafe impl Sync for CudaGraphExec {}
unsafe impl Send for CudaEvent {}
unsafe impl Sync for CudaEvent {}

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

    pub fn begin_capture(&self) -> Result<CudaGraphCapture<'_>, AccelError> {
        let code = unsafe { lzvm_cuda_stream_begin_capture(self.raw) };
        cuda_status(code)?;
        Ok(CudaGraphCapture {
            stream: self,
            active: true,
        })
    }

    pub fn wait_event(&self, event: &CudaEvent) -> Result<(), AccelError> {
        let code = unsafe { lzvm_cuda_stream_wait_event(self.raw, event.raw) };
        cuda_status(code)
    }

    pub(crate) fn as_raw(&self) -> *mut c_void {
        self.raw
    }
}

impl CudaGraphCapture<'_> {
    pub fn end(mut self) -> Result<CudaGraph, AccelError> {
        let mut raw = ptr::null_mut();
        let code = unsafe { lzvm_cuda_stream_end_capture(self.stream.raw, &mut raw) };
        self.active = false;
        cuda_status(code)?;
        if raw.is_null() {
            return Err(AccelError::Cuda { code: -1 });
        }
        Ok(CudaGraph { raw })
    }
}

impl CudaGraph {
    pub fn instantiate(&self) -> Result<CudaGraphExec, AccelError> {
        let mut raw = ptr::null_mut();
        let code = unsafe { lzvm_cuda_graph_instantiate(self.raw, &mut raw) };
        cuda_status(code)?;
        if raw.is_null() {
            return Err(AccelError::Cuda { code: -1 });
        }
        Ok(CudaGraphExec { raw })
    }
}

impl CudaGraphExec {
    pub fn update(&mut self, graph: &CudaGraph) -> Result<(), AccelError> {
        let code = unsafe { lzvm_cuda_graph_exec_update(self.raw, graph.raw) };
        cuda_status(code)
    }

    pub fn launch(&self, stream: &CudaStream) -> Result<(), AccelError> {
        let code = unsafe { lzvm_cuda_graph_launch(self.raw, stream.raw) };
        cuda_status(code)
    }
}

impl CudaEvent {
    pub fn new() -> Result<Self, AccelError> {
        let mut raw = ptr::null_mut();
        let code = unsafe { lzvm_cuda_event_create(&mut raw) };
        cuda_status(code)?;
        if raw.is_null() {
            return Err(AccelError::Cuda { code: -1 });
        }
        Ok(Self { raw })
    }

    pub fn record(&self, stream: &CudaStream) -> Result<(), AccelError> {
        let code = unsafe { lzvm_cuda_event_record(self.raw, stream.raw) };
        cuda_status(code)
    }

    pub fn synchronize(&self) -> Result<(), AccelError> {
        let code = unsafe { lzvm_cuda_event_synchronize(self.raw) };
        cuda_status(code)
    }
}

impl Drop for CudaGraphCapture<'_> {
    fn drop(&mut self) {
        if self.active {
            let mut raw = ptr::null_mut();
            if unsafe { lzvm_cuda_stream_end_capture(self.stream.raw, &mut raw) } == 0
                && !raw.is_null()
            {
                let _ = unsafe { lzvm_cuda_graph_destroy(raw) };
            }
            self.active = false;
        }
    }
}

impl Drop for CudaGraph {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            let _ = unsafe { lzvm_cuda_graph_destroy(self.raw) };
            self.raw = ptr::null_mut();
        }
    }
}

impl Drop for CudaGraphExec {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            let _ = unsafe { lzvm_cuda_graph_exec_destroy(self.raw) };
            self.raw = ptr::null_mut();
        }
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

impl Drop for CudaEvent {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            let _ = unsafe { lzvm_cuda_event_destroy(self.raw) };
            self.raw = ptr::null_mut();
        }
    }
}
