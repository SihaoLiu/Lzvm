use super::{
    cuda_goldilocks_begin_coset_extend_row_major_columns_device_on_stream, AccelError,
    CudaDeviceBuffer, CudaGraph, CudaGraphExec, CudaStream,
};

#[derive(Debug)]
pub struct CudaRowMajorCosetExtensionGraphRunner {
    stream: CudaStream,
    graph: Option<CudaGraph>,
    exec: Option<CudaGraphExec>,
    column_count: usize,
    source_bits: usize,
    target_bits: usize,
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
            column_count,
            source_bits,
            target_bits,
        })
    }

    pub fn run(
        &mut self,
        values: &CudaDeviceBuffer,
        out: &mut CudaDeviceBuffer,
        workspace: &mut CudaDeviceBuffer,
    ) -> Result<(), AccelError> {
        let graph = self.capture(values, out, workspace)?;
        if let Some(exec) = &mut self.exec {
            exec.update(&graph)?;
        } else {
            self.exec = Some(graph.instantiate()?);
        }
        self.graph = Some(graph);
        self.exec
            .as_ref()
            .expect("graph executable should be initialized")
            .launch(&self.stream)?;
        self.stream.synchronize()
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
