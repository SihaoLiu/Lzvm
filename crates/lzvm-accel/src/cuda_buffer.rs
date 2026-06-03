use std::ffi::c_void;
use std::ptr;

use super::{cuda_allocator, cuda_status, u64_word_byte_len, AccelError};

unsafe extern "C" {
    fn lzvm_cuda_copy_h2d_bytes(dst: *mut c_void, src: *const c_void, bytes: usize) -> i32;
    fn lzvm_cuda_copy_d2h_bytes(dst: *mut c_void, src: *const c_void, bytes: usize) -> i32;
    fn lzvm_cuda_copy_d2h_state_prefix_words(
        dst: *mut c_void,
        src: *const c_void,
        state_count: usize,
        state_width_words: usize,
        prefix_words: usize,
    ) -> i32;
    fn lzvm_cuda_expand_state_prefix_words(
        dst: *mut c_void,
        src: *const c_void,
        state_count: usize,
        state_width_words: usize,
        prefix_words: usize,
    ) -> i32;
    fn lzvm_cuda_memset_zero_bytes(dst: *mut c_void, bytes: usize) -> i32;
}

#[cfg(not(target_endian = "little"))]
fn u64_words_to_bytes(words: &[u64]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(words.len().saturating_mul(8));
    for word in words {
        bytes.extend_from_slice(&word.to_le_bytes());
    }
    bytes
}

#[cfg(not(target_endian = "little"))]
fn bytes_to_u64_words(bytes: &[u8]) -> Result<Vec<u64>, AccelError> {
    if !bytes.len().is_multiple_of(8) {
        return Err(AccelError::LengthMismatch {
            lhs: bytes.len(),
            rhs: bytes.len() / 8 * 8,
        });
    }

    Ok(bytes
        .chunks_exact(8)
        .map(|chunk| {
            let mut word = [0_u8; 8];
            word.copy_from_slice(chunk);
            u64::from_le_bytes(word)
        })
        .collect::<Vec<_>>())
}

#[derive(Debug)]
pub struct CudaDeviceBuffer {
    ptr: *mut c_void,
    len: usize,
}

impl CudaDeviceBuffer {
    pub fn new(len: usize) -> Result<Self, AccelError> {
        let ptr = cuda_allocator::alloc_bytes(len)?;
        Ok(Self { ptr, len })
    }

    pub fn zeroed(len: usize) -> Result<Self, AccelError> {
        let buffer = Self::new(len)?;
        if len > 0 {
            let code = unsafe { lzvm_cuda_memset_zero_bytes(buffer.ptr, len) };
            cuda_status(code)?;
        }
        Ok(buffer)
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn as_raw_ptr(&self) -> *mut c_void {
        self.ptr
    }

    pub fn from_u64_words(words: &[u64]) -> Result<Self, AccelError> {
        let mut buffer = Self::new(u64_word_byte_len(words.len())?)?;
        buffer.copy_from_u64_words(words)?;
        Ok(buffer)
    }

    pub fn from_state_prefix_u64_words(
        words: &[u64],
        state_count: usize,
        state_width_words: usize,
        prefix_words: usize,
    ) -> Result<Self, AccelError> {
        if state_count > 0 && state_width_words == 0 {
            return Err(AccelError::InvalidDomain {
                bits: state_width_words,
                len: prefix_words,
            });
        }
        if prefix_words > state_width_words {
            return Err(AccelError::InvalidDomain {
                bits: state_width_words,
                len: prefix_words,
            });
        }
        let expected_words =
            state_count
                .checked_mul(prefix_words)
                .ok_or(AccelError::InvalidDomain {
                    bits: prefix_words,
                    len: state_count,
                })?;
        let expected_input_len = u64_word_byte_len(expected_words)?;
        let input_len = u64_word_byte_len(words.len())?;
        if input_len != expected_input_len {
            return Err(AccelError::LengthMismatch {
                lhs: input_len,
                rhs: expected_input_len,
            });
        }
        let output_words =
            state_count
                .checked_mul(state_width_words)
                .ok_or(AccelError::InvalidDomain {
                    bits: state_width_words,
                    len: state_count,
                })?;
        let buffer = Self::new(u64_word_byte_len(output_words)?)?;
        if state_count == 0 {
            return Ok(buffer);
        }
        #[cfg(target_endian = "little")]
        let src = words.as_ptr().cast();
        #[cfg(not(target_endian = "little"))]
        let src_bytes = u64_words_to_bytes(words);
        #[cfg(not(target_endian = "little"))]
        let src = src_bytes.as_ptr().cast();
        let code = unsafe {
            lzvm_cuda_expand_state_prefix_words(
                buffer.ptr,
                src,
                state_count,
                state_width_words,
                prefix_words,
            )
        };
        cuda_status(code)?;
        Ok(buffer)
    }

    pub fn to_u64_words(&self) -> Result<Vec<u64>, AccelError> {
        if !self.len.is_multiple_of(8) {
            return Err(AccelError::LengthMismatch {
                lhs: self.len,
                rhs: self.len / 8 * 8,
            });
        }
        #[cfg(target_endian = "little")]
        {
            let mut output = vec![0_u64; self.len / 8];
            if output.is_empty() {
                return Ok(output);
            }
            let code = unsafe {
                lzvm_cuda_copy_d2h_bytes(
                    output.as_mut_ptr().cast(),
                    self.ptr as *const c_void,
                    self.len,
                )
            };
            cuda_status(code)?;
            Ok(output)
        }
        #[cfg(not(target_endian = "little"))]
        {
            let bytes = self.to_vec()?;
            bytes_to_u64_words(&bytes)
        }
    }

    pub fn copy_from_u64_words(&mut self, words: &[u64]) -> Result<(), AccelError> {
        let expected_len = u64_word_byte_len(words.len())?;
        if expected_len != self.len {
            return Err(AccelError::LengthMismatch {
                lhs: self.len,
                rhs: expected_len,
            });
        }
        if self.len == 0 {
            return Ok(());
        }
        #[cfg(target_endian = "little")]
        {
            let code =
                unsafe { lzvm_cuda_copy_h2d_bytes(self.ptr, words.as_ptr().cast(), self.len) };
            cuda_status(code)
        }
        #[cfg(not(target_endian = "little"))]
        {
            let bytes = u64_words_to_bytes(words);
            self.copy_from(&bytes)
        }
    }

    pub fn to_state_prefix_u64_words(
        &self,
        state_count: usize,
        state_width_words: usize,
        prefix_words: usize,
    ) -> Result<Vec<u64>, AccelError> {
        if prefix_words > state_width_words {
            return Err(AccelError::InvalidDomain {
                bits: state_width_words,
                len: prefix_words,
            });
        }
        let expected_len = state_count
            .checked_mul(state_width_words)
            .and_then(|word_count| word_count.checked_mul(8))
            .ok_or(AccelError::InvalidDomain {
                bits: state_width_words,
                len: state_count,
            })?;
        if self.len != expected_len {
            return Err(AccelError::LengthMismatch {
                lhs: expected_len,
                rhs: self.len,
            });
        }
        let output_words =
            state_count
                .checked_mul(prefix_words)
                .ok_or(AccelError::InvalidDomain {
                    bits: prefix_words,
                    len: state_count,
                })?;
        let mut output = vec![0_u64; output_words];
        if output.is_empty() {
            return Ok(output);
        }
        let code = unsafe {
            lzvm_cuda_copy_d2h_state_prefix_words(
                output.as_mut_ptr().cast(),
                self.ptr as *const c_void,
                state_count,
                state_width_words,
                prefix_words,
            )
        };
        cuda_status(code)?;
        Ok(output)
    }

    pub fn copy_from(&mut self, input: &[u8]) -> Result<(), AccelError> {
        if input.len() != self.len {
            return Err(AccelError::LengthMismatch {
                lhs: self.len,
                rhs: input.len(),
            });
        }
        if self.len == 0 {
            return Ok(());
        }
        let code = unsafe { lzvm_cuda_copy_h2d_bytes(self.ptr, input.as_ptr().cast(), self.len) };
        cuda_status(code)
    }

    pub fn copy_to(&self, output: &mut [u8]) -> Result<(), AccelError> {
        if output.len() != self.len {
            return Err(AccelError::LengthMismatch {
                lhs: self.len,
                rhs: output.len(),
            });
        }
        if self.len == 0 {
            return Ok(());
        }
        let code = unsafe {
            lzvm_cuda_copy_d2h_bytes(
                output.as_mut_ptr().cast(),
                self.ptr as *const c_void,
                self.len,
            )
        };
        cuda_status(code)
    }

    pub fn copy_range_to(&self, byte_offset: usize, output: &mut [u8]) -> Result<(), AccelError> {
        let end = byte_offset
            .checked_add(output.len())
            .ok_or(AccelError::LengthMismatch {
                lhs: self.len,
                rhs: output.len(),
            })?;
        if end > self.len {
            return Err(AccelError::LengthMismatch {
                lhs: self.len.saturating_sub(byte_offset.min(self.len)),
                rhs: output.len(),
            });
        }
        if output.is_empty() {
            return Ok(());
        }
        let source = unsafe { (self.ptr as *const u8).add(byte_offset).cast() };
        let code =
            unsafe { lzvm_cuda_copy_d2h_bytes(output.as_mut_ptr().cast(), source, output.len()) };
        cuda_status(code)
    }

    pub fn to_vec(&self) -> Result<Vec<u8>, AccelError> {
        let mut output = vec![0_u8; self.len];
        self.copy_to(&mut output)?;
        Ok(output)
    }
}

impl Drop for CudaDeviceBuffer {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            cuda_allocator::free_bytes(self.ptr);
            self.ptr = ptr::null_mut();
        }
    }
}
