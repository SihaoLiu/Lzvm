use super::{cuda_status, u64_word_byte_len, AccelError, CudaDeviceBuffer};

unsafe extern "C" {
    fn lzvm_cuda_goldilocks_validate_canonical_words_device(
        values: *const u64,
        word_count: usize,
        found: *mut u32,
    ) -> i32;
    fn lzvm_cuda_goldilocks_begin_validate_canonical_words_device(
        values: *const u64,
        word_count: usize,
        device_found: *mut u32,
    ) -> i32;
}

#[derive(Debug)]
pub struct CudaCanonicalCheck {
    found: CudaDeviceBuffer,
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

pub fn cuda_goldilocks_begin_validate_canonical_words_device(
    values: &CudaDeviceBuffer,
    word_count: usize,
) -> Result<CudaCanonicalCheck, AccelError> {
    let expected_len = u64_word_byte_len(word_count)?;
    if values.len() != expected_len {
        return Err(AccelError::LengthMismatch {
            lhs: values.len(),
            rhs: expected_len,
        });
    }
    let found = CudaDeviceBuffer::zeroed(std::mem::size_of::<u32>())?;
    let code = unsafe {
        lzvm_cuda_goldilocks_begin_validate_canonical_words_device(
            values.as_raw_ptr().cast::<u64>() as *const u64,
            word_count,
            found.as_raw_ptr().cast::<u32>(),
        )
    };
    cuda_status(code)?;
    Ok(CudaCanonicalCheck { found })
}

impl CudaCanonicalCheck {
    pub fn is_canonical(&self) -> Result<bool, AccelError> {
        let bytes = self.found.to_vec()?;
        let found = u32::from_le_bytes(bytes.as_slice().try_into().map_err(|_| {
            AccelError::LengthMismatch {
                lhs: bytes.len(),
                rhs: std::mem::size_of::<u32>(),
            }
        })?);
        Ok(found == 0)
    }
}
