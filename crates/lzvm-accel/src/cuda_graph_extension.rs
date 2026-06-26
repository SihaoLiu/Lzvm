use super::{
    cuda_goldilocks_begin_coset_extend_row_major_columns_device_on_stream,
    cuda_goldilocks_begin_coset_extend_row_major_columns_strided_device_on_stream, AccelError,
    CudaDeviceBuffer, CudaGraph, CudaGraphExec, CudaRowMajorColumnView, CudaStream,
};

#[derive(Debug)]
pub struct CudaRowMajorCosetExtensionGraphRunner {
    stream: CudaStream,
    graph: Option<CudaGraph>,
    exec: Option<CudaGraphExec>,
    graph_key: Option<CudaRowMajorCosetExtensionGraphKey>,
    column_count: usize,
    source_bits: usize,
    target_bits: usize,
    capture_count: usize,
    launch_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CudaRowMajorCosetExtensionGraphKey {
    values_ptr: usize,
    values_len: usize,
    out_ptr: usize,
    out_len: usize,
    workspace_ptr: usize,
    workspace_len: usize,
}

impl CudaRowMajorCosetExtensionGraphKey {
    fn from_buffers(
        values: &CudaDeviceBuffer,
        out: &CudaDeviceBuffer,
        workspace: &CudaDeviceBuffer,
    ) -> Self {
        Self {
            values_ptr: values.as_raw_ptr() as usize,
            values_len: values.len(),
            out_ptr: out.as_raw_ptr() as usize,
            out_len: out.len(),
            workspace_ptr: workspace.as_raw_ptr() as usize,
            workspace_len: workspace.len(),
        }
    }
}

fn saturating_counter_increment(counter: &mut usize) {
    *counter = counter.saturating_add(1);
}

impl CudaRowMajorCosetExtensionGraphRunner {
    pub fn new(
        column_count: usize,
        source_bits: usize,
        target_bits: usize,
    ) -> Result<Self, AccelError> {
        Ok(Self {
            stream: CudaStream::new()?,
            graph: None,
            exec: None,
            graph_key: None,
            column_count,
            source_bits,
            target_bits,
            capture_count: 0,
            launch_count: 0,
        })
    }

    pub fn run(
        &mut self,
        values: &CudaDeviceBuffer,
        out: &mut CudaDeviceBuffer,
        workspace: &mut CudaDeviceBuffer,
    ) -> Result<(), AccelError> {
        unsafe { self.enqueue(values, out, workspace)? };
        self.synchronize()
    }

    /// Enqueues the captured coset extension graph and returns before it completes.
    ///
    /// # Safety
    ///
    /// The caller must keep `values`, `out`, `workspace`, and this runner alive
    /// and must not read or reuse `out` or `workspace` until `synchronize`
    /// has completed on this runner.
    pub unsafe fn enqueue(
        &mut self,
        values: &CudaDeviceBuffer,
        out: &mut CudaDeviceBuffer,
        workspace: &mut CudaDeviceBuffer,
    ) -> Result<(), AccelError> {
        let graph_key = CudaRowMajorCosetExtensionGraphKey::from_buffers(values, out, workspace);
        if self.exec.is_none() || self.graph_key != Some(graph_key) {
            let graph = self.capture(values, out, workspace)?;
            if let Some(exec) = &mut self.exec {
                exec.update(&graph)?;
            } else {
                self.exec = Some(graph.instantiate()?);
            }
            self.graph = Some(graph);
            self.graph_key = Some(graph_key);
            saturating_counter_increment(&mut self.capture_count);
        }
        self.exec
            .as_ref()
            .expect("graph executable should be initialized")
            .launch(&self.stream)?;
        saturating_counter_increment(&mut self.launch_count);
        Ok(())
    }

    pub fn synchronize(&self) -> Result<(), AccelError> {
        self.stream.synchronize()
    }

    pub fn capture_count(&self) -> usize {
        self.capture_count
    }

    pub fn launch_count(&self) -> usize {
        self.launch_count
    }

    fn capture(
        &self,
        values: &CudaDeviceBuffer,
        out: &mut CudaDeviceBuffer,
        workspace: &mut CudaDeviceBuffer,
    ) -> Result<CudaGraph, AccelError> {
        let capture = self.stream.begin_capture()?;
        unsafe {
            cuda_goldilocks_begin_coset_extend_row_major_columns_device_on_stream(
                values,
                out,
                workspace,
                self.column_count,
                self.source_bits,
                self.target_bits,
                &self.stream,
            )?;
        }
        capture.end()
    }
}

#[derive(Debug)]
pub struct CudaStridedRowMajorCosetExtensionGraphRunner {
    stream: CudaStream,
    graph: Option<CudaGraph>,
    exec: Option<CudaGraphExec>,
    graph_key: Option<CudaRowMajorCosetExtensionGraphKey>,
    view: CudaRowMajorColumnView,
    source_bits: usize,
    target_bits: usize,
    capture_count: usize,
    launch_count: usize,
}

impl CudaStridedRowMajorCosetExtensionGraphRunner {
    pub fn new(
        view: CudaRowMajorColumnView,
        source_bits: usize,
        target_bits: usize,
    ) -> Result<Self, AccelError> {
        Ok(Self {
            stream: CudaStream::new()?,
            graph: None,
            exec: None,
            graph_key: None,
            view,
            source_bits,
            target_bits,
            capture_count: 0,
            launch_count: 0,
        })
    }

    pub fn run(
        &mut self,
        values: &CudaDeviceBuffer,
        out: &mut CudaDeviceBuffer,
        workspace: &mut CudaDeviceBuffer,
    ) -> Result<(), AccelError> {
        unsafe { self.enqueue(values, out, workspace)? };
        self.synchronize()
    }

    /// Enqueues the captured strided coset extension graph and returns before it completes.
    ///
    /// # Safety
    ///
    /// The caller must keep `values`, `out`, `workspace`, and this runner alive
    /// and must not read or reuse `out` or `workspace` until `synchronize`
    /// has completed on this runner.
    pub unsafe fn enqueue(
        &mut self,
        values: &CudaDeviceBuffer,
        out: &mut CudaDeviceBuffer,
        workspace: &mut CudaDeviceBuffer,
    ) -> Result<(), AccelError> {
        let graph_key = CudaRowMajorCosetExtensionGraphKey::from_buffers(values, out, workspace);
        if self.exec.is_none() || self.graph_key != Some(graph_key) {
            let graph = self.capture(values, out, workspace)?;
            if let Some(exec) = &mut self.exec {
                exec.update(&graph)?;
            } else {
                self.exec = Some(graph.instantiate()?);
            }
            self.graph = Some(graph);
            self.graph_key = Some(graph_key);
            saturating_counter_increment(&mut self.capture_count);
        }
        self.exec
            .as_ref()
            .expect("graph executable should be initialized")
            .launch(&self.stream)?;
        saturating_counter_increment(&mut self.launch_count);
        Ok(())
    }

    pub fn synchronize(&self) -> Result<(), AccelError> {
        self.stream.synchronize()
    }

    pub fn capture_count(&self) -> usize {
        self.capture_count
    }

    pub fn launch_count(&self) -> usize {
        self.launch_count
    }

    fn capture(
        &self,
        values: &CudaDeviceBuffer,
        out: &mut CudaDeviceBuffer,
        workspace: &mut CudaDeviceBuffer,
    ) -> Result<CudaGraph, AccelError> {
        let capture = self.stream.begin_capture()?;
        unsafe {
            cuda_goldilocks_begin_coset_extend_row_major_columns_strided_device_on_stream(
                values,
                out,
                workspace,
                self.view,
                self.source_bits,
                self.target_bits,
                &self.stream,
            )?;
        }
        capture.end()
    }
}
