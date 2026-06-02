use super::{cuda_status, u64_word_byte_len, AccelError, CudaDeviceBuffer};

unsafe extern "C" {
    fn lzvm_cuda_goldilocks_validate_canonical_words_device(
        values: *const u64,
        word_count: usize,
        found: *mut u32,
    ) -> i32;
}

pub fn cuda_goldilocks_validate_canonical_words_device(
    values: &CudaDeviceBuffer,
    word_count: usize,
) -> Result<bool, AccelError> {
    let expected_len = u64_word_byte_len(word_count)?;
    if values.len() != expected_len {
        return Err(AccelError::LengthMismatch {
            lhs: values.len(),
            rhs: expected_len,
        });
    }
    let mut found = 0_u32;
    let code = unsafe {
        lzvm_cuda_goldilocks_validate_canonical_words_device(
            values.as_raw_ptr().cast::<u64>() as *const u64,
            word_count,
            &mut found,
        )
    };
    cuda_status(code)?;
    Ok(found == 0)
}
