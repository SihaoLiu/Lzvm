use std::ffi::c_void;
use std::ptr;

use super::{cuda_allocator, cuda_status, u64_word_byte_len, AccelError};

const SPARSE_U64_WORD_CHUNK: usize = 8 * 1024 * 1024;
const ZISK_MAIN_TRACE_COMPACT_DESCRIPTOR_WORDS: usize = 11;
const ZISK_MAIN_TRACE_WIDE_DESCRIPTOR_WORDS: usize = 14;
const ZISK_MAIN_TRACE_WIDTH_WORDS: usize = 39;

unsafe extern "C" {
    fn lzvm_cuda_copy_h2d_bytes(dst: *mut c_void, src: *const c_void, bytes: usize) -> i32;
    fn lzvm_cuda_copy_d2h_bytes(dst: *mut c_void, src: *const c_void, bytes: usize) -> i32;
    fn lzvm_cuda_copy_h2d_row_slice_words(
        dst: *mut c_void,
        src: *const c_void,
        row_count: usize,
        source_width_words: usize,
        start_word: usize,
        slice_width_words: usize,
    ) -> i32;
    fn lzvm_cuda_copy_d2d_row_slice_words(
        dst: *mut c_void,
        src: *const c_void,
        row_count: usize,
        source_width_words: usize,
        start_word: usize,
        slice_width_words: usize,
    ) -> i32;
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
    fn lzvm_cuda_expand_state_prefix_words_device_to_device(
        dst: *mut c_void,
        src: *const c_void,
        state_count: usize,
        state_width_words: usize,
        prefix_words: usize,
    ) -> i32;
    fn lzvm_cuda_memset_zero_bytes(dst: *mut c_void, bytes: usize) -> i32;
    fn lzvm_cuda_fill_row_major_column_u64(
        dst: *mut u64,
        row_count: usize,
        row_width_words: usize,
        start_row: usize,
        column: usize,
        value: u64,
    ) -> i32;
    fn lzvm_cuda_fill_row_major_suffix_from_row_u64(
        dst: *mut u64,
        row_values: *const u64,
        row_count: usize,
        row_width_words: usize,
        start_row: usize,
    ) -> i32;
    fn lzvm_cuda_scatter_sparse_u64_words(
        dst: *mut u64,
        indices: *const u64,
        values: *const u64,
        sparse_word_count: usize,
    ) -> i32;
    fn lzvm_cuda_scatter_sparse_u32_indices_u64_words(
        dst: *mut u64,
        indices: *const u32,
        values: *const u64,
        sparse_word_count: usize,
    ) -> i32;
    fn lzvm_cuda_expand_zisk_main_trace_descriptors(
        dst: *mut u64,
        descriptors: *const u64,
        descriptor_words: usize,
        descriptor_count: usize,
        row_count: usize,
        row_width_words: usize,
        terminal_pc: u64,
    ) -> i32;
}

#[cfg(not(target_endian = "little"))]
fn u64_words_to_bytes(words: &[u64]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(words.len().saturating_mul(8));
    for word in words {
        bytes.extend_from_slice(&word.to_le_bytes());
    }
    bytes
}

fn zisk_main_trace_descriptor_words_supported(descriptor_words: usize) -> bool {
    descriptor_words == ZISK_MAIN_TRACE_COMPACT_DESCRIPTOR_WORDS
        || descriptor_words == ZISK_MAIN_TRACE_WIDE_DESCRIPTOR_WORDS
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

fn validate_row_major_u64_slice_shape(
    word_count: usize,
    row_count: usize,
    source_width_words: usize,
    start_word: usize,
    slice_width_words: usize,
) -> Result<usize, AccelError> {
    let expected_words =
        row_count
            .checked_mul(source_width_words)
            .ok_or(AccelError::InvalidDomain {
                bits: source_width_words,
                len: row_count,
            })?;
    if word_count != expected_words {
        return Err(AccelError::LengthMismatch {
            lhs: word_count,
            rhs: expected_words,
        });
    }
    if row_count == 0 || slice_width_words == 0 {
        return Ok(0);
    }
    if source_width_words == 0
        || start_word > source_width_words
        || slice_width_words > source_width_words - start_word
    {
        return Err(AccelError::InvalidDomain {
            bits: source_width_words,
            len: slice_width_words,
        });
    }
    row_count
        .checked_mul(slice_width_words)
        .ok_or(AccelError::InvalidDomain {
            bits: slice_width_words,
            len: row_count,
        })
}

fn u32_word_byte_len(word_count: usize) -> Result<usize, AccelError> {
    word_count.checked_mul(4).ok_or(AccelError::InvalidDomain {
        bits: 32,
        len: word_count,
    })
}

#[derive(Debug)]
pub struct CudaDeviceBuffer {
    ptr: *mut c_void,
    len: usize,
}

// Device memory ownership is tied to this handle; shared Rust references do not
// expose host-side mutation, and CUDA kernels/copies synchronize through the
// CUDA runtime wrappers used by this crate.
unsafe impl Send for CudaDeviceBuffer {}
unsafe impl Sync for CudaDeviceBuffer {}

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

    pub fn from_row_major_u64_slice(
        words: &[u64],
        row_count: usize,
        source_width_words: usize,
        start_word: usize,
        slice_width_words: usize,
    ) -> Result<Self, AccelError> {
        let output_words = validate_row_major_u64_slice_shape(
            words.len(),
            row_count,
            source_width_words,
            start_word,
            slice_width_words,
        )?;
        let mut buffer = Self::new(u64_word_byte_len(output_words)?)?;
        buffer.copy_from_row_major_u64_slice(
            words,
            row_count,
            source_width_words,
            start_word,
            slice_width_words,
        )?;
        Ok(buffer)
    }

    pub fn from_device_row_major_u64_slice(
        source: &Self,
        row_count: usize,
        source_width_words: usize,
        start_word: usize,
        slice_width_words: usize,
    ) -> Result<Self, AccelError> {
        if !source.len.is_multiple_of(8) {
            return Err(AccelError::LengthMismatch {
                lhs: source.len,
                rhs: source.len / 8 * 8,
            });
        }
        let output_words = validate_row_major_u64_slice_shape(
            source.len / 8,
            row_count,
            source_width_words,
            start_word,
            slice_width_words,
        )?;
        let mut buffer = Self::new(u64_word_byte_len(output_words)?)?;
        buffer.copy_from_device_row_major_u64_slice(
            source,
            row_count,
            source_width_words,
            start_word,
            slice_width_words,
        )?;
        Ok(buffer)
    }

    pub fn from_row_major_u64_prefix_and_suffix_row(
        prefix_words: &[u64],
        suffix_row: &[u64],
        row_count: usize,
        row_width_words: usize,
        prefix_rows: usize,
    ) -> Result<Self, AccelError> {
        if row_width_words == 0 || suffix_row.len() != row_width_words || prefix_rows > row_count {
            return Err(AccelError::InvalidDomain {
                bits: row_width_words,
                len: suffix_row.len(),
            });
        }
        let expected_prefix_words =
            prefix_rows
                .checked_mul(row_width_words)
                .ok_or(AccelError::InvalidDomain {
                    bits: row_width_words,
                    len: prefix_rows,
                })?;
        if prefix_words.len() != expected_prefix_words {
            return Err(AccelError::LengthMismatch {
                lhs: prefix_words.len(),
                rhs: expected_prefix_words,
            });
        }
        let output_words =
            row_count
                .checked_mul(row_width_words)
                .ok_or(AccelError::InvalidDomain {
                    bits: row_width_words,
                    len: row_count,
                })?;
        let mut buffer = Self::new(u64_word_byte_len(output_words)?)?;
        buffer.copy_prefix_from_u64_words(prefix_words)?;
        if prefix_rows < row_count {
            let suffix_row_device = Self::from_u64_words(suffix_row)?;
            buffer.fill_row_major_suffix_from_row_u64(
                &suffix_row_device,
                row_count,
                row_width_words,
                prefix_rows,
            )?;
        }
        Ok(buffer)
    }

    pub fn from_sparse_u64_words(
        word_count: usize,
        indices: &[u64],
        values: &[u64],
    ) -> Result<Self, AccelError> {
        if indices.len() != values.len() {
            return Err(AccelError::LengthMismatch {
                lhs: indices.len(),
                rhs: values.len(),
            });
        }
        let word_count_u64 = u64::try_from(word_count).map_err(|_| AccelError::InvalidDomain {
            bits: usize::BITS as usize,
            len: word_count,
        })?;
        if indices.iter().any(|index| *index >= word_count_u64) {
            return Err(AccelError::InvalidDomain {
                bits: word_count,
                len: indices.len(),
            });
        }
        let buffer = Self::zeroed(u64_word_byte_len(word_count)?)?;
        if indices.is_empty() {
            return Ok(buffer);
        }
        let chunk_capacity = indices.len().min(SPARSE_U64_WORD_CHUNK);
        let mut values_device = Self::new(u64_word_byte_len(chunk_capacity)?)?;
        if word_count <= u32::MAX as usize {
            let mut indices_device = Self::new(u32_word_byte_len(chunk_capacity)?)?;
            let mut index_chunk = Vec::with_capacity(chunk_capacity);
            let mut offset = 0_usize;
            while offset < indices.len() {
                let end = (offset + chunk_capacity).min(indices.len());
                index_chunk.clear();
                index_chunk.extend(indices[offset..end].iter().map(|index| *index as u32));
                indices_device.copy_prefix_from_u32_words(index_chunk.as_slice())?;
                values_device.copy_prefix_from_u64_words(&values[offset..end])?;
                let code = unsafe {
                    lzvm_cuda_scatter_sparse_u32_indices_u64_words(
                        buffer.ptr.cast(),
                        indices_device.ptr.cast(),
                        values_device.ptr.cast(),
                        end - offset,
                    )
                };
                cuda_status(code)?;
                offset = end;
            }
        } else {
            let mut indices_device = Self::new(u64_word_byte_len(chunk_capacity)?)?;
            let mut offset = 0_usize;
            while offset < indices.len() {
                let end = (offset + chunk_capacity).min(indices.len());
                indices_device.copy_prefix_from_u64_words(&indices[offset..end])?;
                values_device.copy_prefix_from_u64_words(&values[offset..end])?;
                let code = unsafe {
                    lzvm_cuda_scatter_sparse_u64_words(
                        buffer.ptr.cast(),
                        indices_device.ptr.cast(),
                        values_device.ptr.cast(),
                        end - offset,
                    )
                };
                cuda_status(code)?;
                offset = end;
            }
        }
        Ok(buffer)
    }

    pub fn from_zisk_main_trace_descriptors(
        descriptors: &[u64],
        descriptor_words: usize,
        descriptor_count: usize,
        row_count: usize,
        row_width_words: usize,
        terminal_pc: u64,
    ) -> Result<Self, AccelError> {
        if !zisk_main_trace_descriptor_words_supported(descriptor_words)
            || row_width_words != ZISK_MAIN_TRACE_WIDTH_WORDS
            || descriptor_count > row_count
        {
            return Err(AccelError::InvalidDomain {
                bits: descriptor_words,
                len: descriptor_count,
            });
        }
        let expected_descriptor_words =
            descriptor_count
                .checked_mul(descriptor_words)
                .ok_or(AccelError::InvalidDomain {
                    bits: descriptor_words,
                    len: descriptor_count,
                })?;
        if descriptors.len() != expected_descriptor_words {
            return Err(AccelError::LengthMismatch {
                lhs: descriptors.len(),
                rhs: expected_descriptor_words,
            });
        }
        let descriptor_buffer = Self::from_u64_words(descriptors)?;
        Self::from_zisk_main_trace_descriptors_device(
            &descriptor_buffer,
            descriptor_words,
            descriptor_count,
            row_count,
            row_width_words,
            terminal_pc,
        )
    }

    pub fn from_zisk_main_trace_descriptors_device(
        descriptors: &Self,
        descriptor_words: usize,
        descriptor_count: usize,
        row_count: usize,
        row_width_words: usize,
        terminal_pc: u64,
    ) -> Result<Self, AccelError> {
        if !zisk_main_trace_descriptor_words_supported(descriptor_words)
            || row_width_words != ZISK_MAIN_TRACE_WIDTH_WORDS
            || descriptor_count > row_count
        {
            return Err(AccelError::InvalidDomain {
                bits: descriptor_words,
                len: descriptor_count,
            });
        }
        let expected_descriptor_words =
            descriptor_count
                .checked_mul(descriptor_words)
                .ok_or(AccelError::InvalidDomain {
                    bits: descriptor_words,
                    len: descriptor_count,
                })?;
        let expected_descriptor_len = u64_word_byte_len(expected_descriptor_words)?;
        if descriptors.len() != expected_descriptor_len {
            return Err(AccelError::LengthMismatch {
                lhs: descriptors.len(),
                rhs: expected_descriptor_len,
            });
        }
        let output_words =
            row_count
                .checked_mul(row_width_words)
                .ok_or(AccelError::InvalidDomain {
                    bits: row_width_words,
                    len: row_count,
                })?;
        let buffer = Self::new(u64_word_byte_len(output_words)?)?;
        if row_count == 0 {
            return Ok(buffer);
        }
        let code = unsafe {
            lzvm_cuda_expand_zisk_main_trace_descriptors(
                buffer.ptr.cast(),
                descriptors.ptr.cast(),
                descriptor_words,
                descriptor_count,
                row_count,
                row_width_words,
                terminal_pc,
            )
        };
        cuda_status(code)?;
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

    pub fn from_device_state_prefix_u64_words(
        source: &Self,
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
        if source.len != expected_input_len {
            return Err(AccelError::LengthMismatch {
                lhs: source.len,
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
        let code = unsafe {
            lzvm_cuda_expand_state_prefix_words_device_to_device(
                buffer.ptr,
                source.ptr as *const c_void,
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

    pub fn copy_prefix_from_u64_words(&mut self, words: &[u64]) -> Result<(), AccelError> {
        let copy_len = u64_word_byte_len(words.len())?;
        if copy_len > self.len {
            return Err(AccelError::LengthMismatch {
                lhs: self.len,
                rhs: copy_len,
            });
        }
        if copy_len == 0 {
            return Ok(());
        }
        #[cfg(target_endian = "little")]
        {
            let code =
                unsafe { lzvm_cuda_copy_h2d_bytes(self.ptr, words.as_ptr().cast(), copy_len) };
            cuda_status(code)
        }
        #[cfg(not(target_endian = "little"))]
        {
            let bytes = u64_words_to_bytes(words);
            if bytes.len() > self.len {
                return Err(AccelError::LengthMismatch {
                    lhs: self.len,
                    rhs: bytes.len(),
                });
            }
            let code =
                unsafe { lzvm_cuda_copy_h2d_bytes(self.ptr, bytes.as_ptr().cast(), copy_len) };
            cuda_status(code)
        }
    }

    fn copy_prefix_from_u32_words(&mut self, words: &[u32]) -> Result<(), AccelError> {
        let copy_len = u32_word_byte_len(words.len())?;
        if copy_len > self.len {
            return Err(AccelError::LengthMismatch {
                lhs: self.len,
                rhs: copy_len,
            });
        }
        if copy_len == 0 {
            return Ok(());
        }
        #[cfg(target_endian = "little")]
        {
            let input =
                unsafe { std::slice::from_raw_parts(words.as_ptr().cast::<u8>(), copy_len) };
            let code =
                unsafe { lzvm_cuda_copy_h2d_bytes(self.ptr, input.as_ptr().cast(), copy_len) };
            cuda_status(code)
        }
        #[cfg(not(target_endian = "little"))]
        {
            let mut bytes = Vec::with_capacity(copy_len);
            for word in words {
                bytes.extend_from_slice(&word.to_le_bytes());
            }
            let code =
                unsafe { lzvm_cuda_copy_h2d_bytes(self.ptr, bytes.as_ptr().cast(), copy_len) };
            cuda_status(code)
        }
    }

    pub fn fill_row_major_column_u64(
        &mut self,
        row_count: usize,
        row_width_words: usize,
        start_row: usize,
        column: usize,
        value: u64,
    ) -> Result<(), AccelError> {
        let expected_words =
            row_count
                .checked_mul(row_width_words)
                .ok_or(AccelError::InvalidDomain {
                    bits: row_width_words,
                    len: row_count,
                })?;
        let expected_len = u64_word_byte_len(expected_words)?;
        if expected_len != self.len {
            return Err(AccelError::LengthMismatch {
                lhs: self.len,
                rhs: expected_len,
            });
        }
        if start_row > row_count || row_width_words == 0 || column >= row_width_words {
            return Err(AccelError::InvalidDomain {
                bits: row_width_words,
                len: column,
            });
        }
        if start_row == row_count {
            return Ok(());
        }
        let code = unsafe {
            lzvm_cuda_fill_row_major_column_u64(
                self.ptr.cast(),
                row_count,
                row_width_words,
                start_row,
                column,
                value,
            )
        };
        cuda_status(code)
    }

    pub fn fill_row_major_suffix_from_row_u64(
        &mut self,
        row_values: &Self,
        row_count: usize,
        row_width_words: usize,
        start_row: usize,
    ) -> Result<(), AccelError> {
        let expected_words =
            row_count
                .checked_mul(row_width_words)
                .ok_or(AccelError::InvalidDomain {
                    bits: row_width_words,
                    len: row_count,
                })?;
        let expected_len = u64_word_byte_len(expected_words)?;
        if expected_len != self.len {
            return Err(AccelError::LengthMismatch {
                lhs: self.len,
                rhs: expected_len,
            });
        }
        let row_len = u64_word_byte_len(row_width_words)?;
        if row_values.len != row_len {
            return Err(AccelError::LengthMismatch {
                lhs: row_values.len,
                rhs: row_len,
            });
        }
        if start_row > row_count || row_width_words == 0 {
            return Err(AccelError::InvalidDomain {
                bits: row_width_words,
                len: start_row,
            });
        }
        if start_row == row_count {
            return Ok(());
        }
        let code = unsafe {
            lzvm_cuda_fill_row_major_suffix_from_row_u64(
                self.ptr.cast(),
                row_values.ptr.cast(),
                row_count,
                row_width_words,
                start_row,
            )
        };
        cuda_status(code)
    }

    pub fn copy_from_row_major_u64_slice(
        &mut self,
        words: &[u64],
        row_count: usize,
        source_width_words: usize,
        start_word: usize,
        slice_width_words: usize,
    ) -> Result<(), AccelError> {
        let output_words = validate_row_major_u64_slice_shape(
            words.len(),
            row_count,
            source_width_words,
            start_word,
            slice_width_words,
        )?;
        let expected_len = u64_word_byte_len(output_words)?;
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
            let code = unsafe {
                lzvm_cuda_copy_h2d_row_slice_words(
                    self.ptr,
                    words.as_ptr().cast(),
                    row_count,
                    source_width_words,
                    start_word,
                    slice_width_words,
                )
            };
            cuda_status(code)
        }
        #[cfg(not(target_endian = "little"))]
        {
            let mut sliced = Vec::with_capacity(output_words);
            for row in 0..row_count {
                let row_start = row
                    .checked_mul(source_width_words)
                    .and_then(|index| index.checked_add(start_word))
                    .ok_or(AccelError::InvalidDomain {
                        bits: source_width_words,
                        len: row_count,
                    })?;
                let row_end =
                    row_start
                        .checked_add(slice_width_words)
                        .ok_or(AccelError::InvalidDomain {
                            bits: slice_width_words,
                            len: row_count,
                        })?;
                sliced.extend_from_slice(&words[row_start..row_end]);
            }
            self.copy_from_u64_words(&sliced)
        }
    }

    pub fn copy_from_device_row_major_u64_slice(
        &mut self,
        source: &Self,
        row_count: usize,
        source_width_words: usize,
        start_word: usize,
        slice_width_words: usize,
    ) -> Result<(), AccelError> {
        if !source.len.is_multiple_of(8) {
            return Err(AccelError::LengthMismatch {
                lhs: source.len,
                rhs: source.len / 8 * 8,
            });
        }
        let output_words = validate_row_major_u64_slice_shape(
            source.len / 8,
            row_count,
            source_width_words,
            start_word,
            slice_width_words,
        )?;
        let expected_len = u64_word_byte_len(output_words)?;
        if expected_len != self.len {
            return Err(AccelError::LengthMismatch {
                lhs: self.len,
                rhs: expected_len,
            });
        }
        if self.len == 0 {
            return Ok(());
        }
        let code = unsafe {
            lzvm_cuda_copy_d2d_row_slice_words(
                self.ptr,
                source.ptr as *const c_void,
                row_count,
                source_width_words,
                start_word,
                slice_width_words,
            )
        };
        cuda_status(code)
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
