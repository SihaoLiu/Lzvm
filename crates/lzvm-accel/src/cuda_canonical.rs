use super::{cuda_status, u64_word_byte_len, AccelError, CudaDeviceBuffer, CudaStream};

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
    fn lzvm_cuda_goldilocks_begin_validate_canonical_words_device_on_stream(
        values: *const u64,
        word_count: usize,
        device_found: *mut u32,
        stream: *mut std::ffi::c_void,
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

/// Enqueues canonical word validation on `stream` and returns after launch.
///
/// # Safety
///
/// The caller must keep `values` and `stream` alive until the queued stream
/// work has completed, and must not read the returned check result until that
/// work has completed.
pub unsafe fn cuda_goldilocks_begin_validate_canonical_words_device_on_stream(
    values: &CudaDeviceBuffer,
    word_count: usize,
    stream: &CudaStream,
) -> Result<CudaCanonicalCheck, AccelError> {
    let expected_len = u64_word_byte_len(word_count)?;
    if values.len() != expected_len {
        return Err(AccelError::LengthMismatch {
            lhs: values.len(),
            rhs: expected_len,
        });
    }
    let found = unsafe { CudaDeviceBuffer::zeroed_on_stream(std::mem::size_of::<u32>(), stream) }?;
    let code = unsafe {
        lzvm_cuda_goldilocks_begin_validate_canonical_words_device_on_stream(
            values.as_raw_ptr().cast::<u64>() as *const u64,
            word_count,
            found.as_raw_ptr().cast::<u32>(),
            stream.as_raw(),
        )
    };
    cuda_status(code)?;
    Ok(CudaCanonicalCheck { found })
}

impl CudaCanonicalCheck {
    pub fn pending() -> Result<Self, AccelError> {
        Ok(Self {
            found: CudaDeviceBuffer::zeroed(std::mem::size_of::<u32>())?,
        })
    }

    pub(crate) fn as_raw_device_ptr(&self) -> *mut u32 {
        self.found.as_raw_ptr().cast::<u32>()
    }

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

#[cfg(test)]
mod tests {
    use super::{
        cuda_goldilocks_begin_validate_canonical_words_device,
        cuda_goldilocks_begin_validate_canonical_words_device_on_stream,
        cuda_goldilocks_validate_canonical_words_device, CudaDeviceBuffer, CudaStream,
    };

    const GOLDILOCKS_MODULUS: u64 = 0xffff_ffff_0000_0001;

    fn assert_stream_validation_matches_blocking(words: &[u64], expected: bool) {
        let values = CudaDeviceBuffer::from_u64_words(words).expect("values should upload");
        assert_eq!(
            cuda_goldilocks_validate_canonical_words_device(&values, words.len())
                .expect("blocking validation should run"),
            expected
        );
        assert_eq!(
            cuda_goldilocks_begin_validate_canonical_words_device(&values, words.len())
                .expect("default async validation should enqueue")
                .is_canonical()
                .expect("default async validation should finish"),
            expected
        );

        let stream = CudaStream::new().expect("CUDA stream should create");
        let stream_check = unsafe {
            cuda_goldilocks_begin_validate_canonical_words_device_on_stream(
                &values,
                words.len(),
                &stream,
            )
        }
        .expect("stream validation should enqueue");
        stream
            .synchronize()
            .expect("stream validation should finish");
        assert_eq!(
            stream_check
                .is_canonical()
                .expect("stream validation result should download"),
            expected
        );
    }

    #[test]
    fn canonical_validate_on_stream_matches_default_stream() {
        let _guard = crate::CUDA_TEST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        assert_stream_validation_matches_blocking(&[0, 1, GOLDILOCKS_MODULUS - 1], true);
        assert_stream_validation_matches_blocking(&[0, GOLDILOCKS_MODULUS, 7], false);
    }
}
