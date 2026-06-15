#[cfg(feature = "cuda")]
use std::sync::Mutex;

#[cfg(feature = "cuda")]
use lzvm_accel::{
    cuda_goldilocks_coset_extend_row_major_columns_device,
    cuda_goldilocks_coset_extend_row_major_columns_output_bytes, CudaDeviceBuffer,
    CudaRowMajorCosetExtensionGraphRunner,
};

#[cfg(feature = "cuda")]
static CUDA_TEST_LOCK: Mutex<()> = Mutex::new(());

#[cfg(feature = "cuda")]
#[test]
fn row_major_coset_extension_graph_runner_reuses_exec_for_second_output() {
    let _guard = CUDA_TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let source_bits = 2;
    let target_bits = 3;
    let column_count = 3;
    let source_rows = 1_usize << source_bits;
    let values = (0..source_rows * column_count)
        .map(|index| (index as u64 + 1) * 23)
        .collect::<Vec<_>>();
    let out_byte_count = cuda_goldilocks_coset_extend_row_major_columns_output_bytes(
        values.len(),
        column_count,
        source_bits,
        target_bits,
    )
    .expect("output shape should be valid");

    let source = CudaDeviceBuffer::from_u64_words(&values).expect("source should upload");
    let mut default_out =
        CudaDeviceBuffer::new(out_byte_count).expect("default output should allocate");
    cuda_goldilocks_coset_extend_row_major_columns_device(
        &source,
        &mut default_out,
        column_count,
        source_bits,
        target_bits,
    )
    .expect("default stream extension should run");

    let mut runner =
        CudaRowMajorCosetExtensionGraphRunner::new(column_count, source_bits, target_bits)
            .expect("graph runner should create");
    let mut first_out =
        CudaDeviceBuffer::new(out_byte_count).expect("first output should allocate");
    let mut first_workspace =
        CudaDeviceBuffer::new(out_byte_count).expect("first workspace should allocate");
    runner
        .run(&source, &mut first_out, &mut first_workspace)
        .expect("first graph run should finish");

    let mut second_out =
        CudaDeviceBuffer::new(out_byte_count).expect("second output should allocate");
    let mut second_workspace =
        CudaDeviceBuffer::new(out_byte_count).expect("second workspace should allocate");
    runner
        .run(&source, &mut second_out, &mut second_workspace)
        .expect("second graph run should finish");

    assert_eq!(
        second_out
            .to_u64_words()
            .expect("second output should download"),
        default_out
            .to_u64_words()
            .expect("default output should download")
    );
}
