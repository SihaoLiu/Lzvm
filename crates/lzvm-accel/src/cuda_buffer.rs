use std::ffi::c_void;
use std::ptr;

use super::{cuda_allocator, cuda_status, u64_word_byte_len, AccelError, CudaStream};

const SPARSE_U64_WORD_CHUNK: usize = 8 * 1024 * 1024;
const ZISK_MAIN_TRACE_COMPACT_DESCRIPTOR_WORDS: usize = 11;
const ZISK_MAIN_TRACE_SPARSE_DESCRIPTOR_WORDS: usize = 9;
const ZISK_MAIN_TRACE_WIDE_DESCRIPTOR_WORDS: usize = 14;
const ZISK_MAIN_TRACE_WIDTH_WORDS: usize = 39;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainTraceDeviceLayout {
    Legacy,
    WithStoreAddress,
}

impl MainTraceDeviceLayout {
    fn raw(self) -> u32 {
        match self {
            Self::Legacy => 0,
            Self::WithStoreAddress => 1,
        }
    }
}

unsafe extern "C" {
    fn lzvm_cuda_pinned_host_alloc(out: *mut *mut c_void, bytes: usize) -> i32;
    safe fn lzvm_cuda_pinned_host_alloc_copy_from(
        out: *mut *mut c_void,
        src: *const c_void,
        bytes: usize,
    ) -> i32;
    fn lzvm_cuda_pinned_host_free(ptr: *mut c_void);
    fn lzvm_cuda_copy_h2d_bytes(dst: *mut c_void, src: *const c_void, bytes: usize) -> i32;
    fn lzvm_cuda_copy_h2d_pinned_bytes(dst: *mut c_void, src: *const c_void, bytes: usize) -> i32;
    fn lzvm_cuda_copy_h2d_bytes_on_stream(
        dst: *mut c_void,
        src: *const c_void,
        bytes: usize,
        stream: *mut c_void,
    ) -> i32;
    fn lzvm_cuda_copy_d2h_bytes(dst: *mut c_void, src: *const c_void, bytes: usize) -> i32;
    fn lzvm_cuda_copy_d2h_bytes_on_stream(
        dst: *mut c_void,
        src: *const c_void,
        bytes: usize,
        stream: *mut c_void,
    ) -> i32;
    fn lzvm_cuda_copy_d2h_bytes_on_default_stream(
        dst: *mut c_void,
        src: *const c_void,
        bytes: usize,
    ) -> i32;
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
    fn lzvm_cuda_copy_d2d_row_slice_words_on_stream(
        dst: *mut c_void,
        src: *const c_void,
        row_count: usize,
        source_width_words: usize,
        start_word: usize,
        slice_width_words: usize,
        stream: *mut c_void,
    ) -> i32;
    fn lzvm_cuda_copy_d2d_selected_row_major_rows(
        dst: *mut c_void,
        src: *const c_void,
        rows: *const u64,
        selected_row_count: usize,
        source_row_count: usize,
        row_width_words: usize,
    ) -> i32;
    fn lzvm_cuda_copy_d2d_row_major_rows(
        dst: *mut c_void,
        sources: *const usize,
        rows: *const u64,
        selected_row_count: usize,
        row_width_words: usize,
    ) -> i32;
    fn lzvm_cuda_copy_d2d_row_major_concat_words(
        dst: *mut c_void,
        left: *const c_void,
        right: *const c_void,
        row_count: usize,
        left_width_words: usize,
        right_width_words: usize,
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
    fn lzvm_cuda_expand_state_prefix_words_device_to_device_on_stream(
        dst: *mut c_void,
        src: *const c_void,
        state_count: usize,
        state_width_words: usize,
        prefix_words: usize,
        stream: *mut c_void,
    ) -> i32;
    fn lzvm_cuda_memset_zero_bytes(dst: *mut c_void, bytes: usize) -> i32;
    fn lzvm_cuda_memset_zero_bytes_on_stream(
        dst: *mut c_void,
        bytes: usize,
        stream: *mut c_void,
    ) -> i32;
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
    fn lzvm_cuda_expand_zisk_main_trace_descriptors_on_stream(
        dst: *mut u64,
        descriptors: *const u64,
        descriptor_words: usize,
        descriptor_count: usize,
        row_count: usize,
        row_width_words: usize,
        terminal_pc: u64,
        stream: *mut c_void,
    ) -> i32;
    fn lzvm_cuda_expand_zisk_main_trace_descriptor_selected_row_major_u64_slice(
        dst: *mut u64,
        descriptors: *const u64,
        descriptor_words: usize,
        descriptor_count: usize,
        row_count: usize,
        row_width_words: usize,
        terminal_pc: u64,
        rows: *const u64,
        selected_row_count: usize,
        start_word: usize,
        slice_width_words: usize,
    ) -> i32;
    fn lzvm_cuda_expand_main_trace_descriptor_selected_row_major_u64_slice_layout(
        dst: *mut u64,
        descriptors: *const u64,
        descriptor_words: usize,
        descriptor_count: usize,
        row_count: usize,
        row_width_words: usize,
        terminal_pc: u64,
        rows: *const u64,
        selected_row_count: usize,
        start_word: usize,
        slice_width_words: usize,
        layout_kind: u32,
    ) -> i32;
    fn lzvm_cuda_expand_sparse_zisk_main_trace_descriptors(
        dst: *mut u64,
        descriptors: *const u64,
        high_words: *const u64,
        high_word_count: usize,
        descriptor_count: usize,
        row_count: usize,
        row_width_words: usize,
        terminal_pc: u64,
    ) -> i32;
    fn lzvm_cuda_expand_main_trace_descriptors_layout(
        dst: *mut u64,
        descriptors: *const u64,
        descriptor_words: usize,
        descriptor_count: usize,
        row_count: usize,
        row_width_words: usize,
        terminal_pc: u64,
        layout_kind: u32,
    ) -> i32;
    fn lzvm_cuda_expand_main_trace_descriptors_layout_on_stream(
        dst: *mut u64,
        descriptors: *const u64,
        descriptor_words: usize,
        descriptor_count: usize,
        row_count: usize,
        row_width_words: usize,
        terminal_pc: u64,
        layout_kind: u32,
        stream: *mut c_void,
    ) -> i32;
    fn lzvm_cuda_expand_sparse_main_trace_descriptors_layout(
        dst: *mut u64,
        descriptors: *const u64,
        high_words: *const u64,
        high_word_count: usize,
        descriptor_count: usize,
        row_count: usize,
        row_width_words: usize,
        terminal_pc: u64,
        layout_kind: u32,
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

fn main_trace_descriptor_words_supported(descriptor_words: usize) -> bool {
    descriptor_words == ZISK_MAIN_TRACE_COMPACT_DESCRIPTOR_WORDS
        || descriptor_words == ZISK_MAIN_TRACE_WIDE_DESCRIPTOR_WORDS
}

fn validate_main_trace_descriptor_device_shape(
    descriptor_byte_len: usize,
    descriptor_words: usize,
    descriptor_count: usize,
    row_count: usize,
    row_width_words: usize,
) -> Result<usize, AccelError> {
    if !main_trace_descriptor_words_supported(descriptor_words)
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
    if descriptor_byte_len != expected_descriptor_len {
        return Err(AccelError::LengthMismatch {
            lhs: descriptor_byte_len,
            rhs: expected_descriptor_len,
        });
    }
    let output_words = row_count
        .checked_mul(row_width_words)
        .ok_or(AccelError::InvalidDomain {
            bits: row_width_words,
            len: row_count,
        })?;
    u64_word_byte_len(output_words)
}

fn validate_zisk_main_trace_descriptor_device_shape(
    descriptor_byte_len: usize,
    descriptor_words: usize,
    descriptor_count: usize,
    row_count: usize,
    row_width_words: usize,
) -> Result<usize, AccelError> {
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
    if descriptor_byte_len != expected_descriptor_len {
        return Err(AccelError::LengthMismatch {
            lhs: descriptor_byte_len,
            rhs: expected_descriptor_len,
        });
    }
    let output_words = row_count
        .checked_mul(row_width_words)
        .ok_or(AccelError::InvalidDomain {
            bits: row_width_words,
            len: row_count,
        })?;
    u64_word_byte_len(output_words)
}

fn validate_sparse_zisk_main_trace_descriptors(
    descriptors: &[u64],
    high_words: &[u64],
    descriptor_count: usize,
    row_count: usize,
    row_width_words: usize,
) -> Result<usize, AccelError> {
    if row_width_words != ZISK_MAIN_TRACE_WIDTH_WORDS || descriptor_count > row_count {
        return Err(AccelError::InvalidDomain {
            bits: row_width_words,
            len: row_count,
        });
    }
    let expected_descriptor_words = descriptor_count
        .checked_mul(ZISK_MAIN_TRACE_SPARSE_DESCRIPTOR_WORDS)
        .ok_or(AccelError::InvalidDomain {
            bits: ZISK_MAIN_TRACE_SPARSE_DESCRIPTOR_WORDS,
            len: descriptor_count,
        })?;
    if descriptors.len() != expected_descriptor_words {
        return Err(AccelError::LengthMismatch {
            lhs: descriptors.len(),
            rhs: expected_descriptor_words,
        });
    }
    let mut required_high_words = 0usize;
    for descriptor in descriptors.chunks_exact(ZISK_MAIN_TRACE_SPARSE_DESCRIPTOR_WORDS) {
        let high_mask = descriptor[7] >> 32;
        if high_mask & !0x7f != 0 {
            return Err(AccelError::InvalidDomain {
                bits: high_mask as usize,
                len: descriptor_count,
            });
        }
        let high_words_for_row = (high_mask.count_ones() as usize).div_ceil(2);
        let high_offset =
            usize::try_from(descriptor[8]).map_err(|_| AccelError::InvalidDomain {
                bits: ZISK_MAIN_TRACE_SPARSE_DESCRIPTOR_WORDS,
                len: descriptor_count,
            })?;
        let row_required_high_words =
            high_offset
                .checked_add(high_words_for_row)
                .ok_or(AccelError::InvalidDomain {
                    bits: ZISK_MAIN_TRACE_SPARSE_DESCRIPTOR_WORDS,
                    len: descriptor_count,
                })?;
        required_high_words = required_high_words.max(row_required_high_words);
    }
    if high_words.len() != required_high_words {
        return Err(AccelError::LengthMismatch {
            lhs: high_words.len(),
            rhs: required_high_words,
        });
    }
    let output_words = row_count
        .checked_mul(row_width_words)
        .ok_or(AccelError::InvalidDomain {
            bits: row_width_words,
            len: row_count,
        })?;
    u64_word_byte_len(output_words)
}

fn validate_sparse_main_trace_descriptors_layout(
    descriptors: &[u64],
    high_words: &[u64],
    descriptor_count: usize,
    row_count: usize,
    row_width_words: usize,
) -> Result<usize, AccelError> {
    if row_width_words != ZISK_MAIN_TRACE_WIDTH_WORDS || descriptor_count > row_count {
        return Err(AccelError::InvalidDomain {
            bits: row_width_words,
            len: row_count,
        });
    }
    let expected_descriptor_words = descriptor_count
        .checked_mul(ZISK_MAIN_TRACE_SPARSE_DESCRIPTOR_WORDS)
        .ok_or(AccelError::InvalidDomain {
            bits: ZISK_MAIN_TRACE_SPARSE_DESCRIPTOR_WORDS,
            len: descriptor_count,
        })?;
    if descriptors.len() != expected_descriptor_words {
        return Err(AccelError::LengthMismatch {
            lhs: descriptors.len(),
            rhs: expected_descriptor_words,
        });
    }
    let mut required_high_words = 0usize;
    for descriptor in descriptors.chunks_exact(ZISK_MAIN_TRACE_SPARSE_DESCRIPTOR_WORDS) {
        let high_mask = descriptor[7] >> 32;
        if high_mask & !0x7f != 0 {
            return Err(AccelError::InvalidDomain {
                bits: high_mask as usize,
                len: descriptor_count,
            });
        }
        let high_words_for_row = (high_mask.count_ones() as usize).div_ceil(2);
        let high_offset =
            usize::try_from(descriptor[8]).map_err(|_| AccelError::InvalidDomain {
                bits: ZISK_MAIN_TRACE_SPARSE_DESCRIPTOR_WORDS,
                len: descriptor_count,
            })?;
        let row_required_high_words =
            high_offset
                .checked_add(high_words_for_row)
                .ok_or(AccelError::InvalidDomain {
                    bits: ZISK_MAIN_TRACE_SPARSE_DESCRIPTOR_WORDS,
                    len: descriptor_count,
                })?;
        required_high_words = required_high_words.max(row_required_high_words);
    }
    if high_words.len() != required_high_words {
        return Err(AccelError::LengthMismatch {
            lhs: high_words.len(),
            rhs: required_high_words,
        });
    }
    let output_words = row_count
        .checked_mul(row_width_words)
        .ok_or(AccelError::InvalidDomain {
            bits: row_width_words,
            len: row_count,
        })?;
    u64_word_byte_len(output_words)
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

fn validate_selected_row_major_u64_rows_shape(
    word_count: usize,
    row_count: usize,
    row_width_words: usize,
    rows: &[usize],
) -> Result<usize, AccelError> {
    let expected_words =
        row_count
            .checked_mul(row_width_words)
            .ok_or(AccelError::InvalidDomain {
                bits: row_width_words,
                len: row_count,
            })?;
    if word_count != expected_words {
        return Err(AccelError::LengthMismatch {
            lhs: word_count,
            rhs: expected_words,
        });
    }
    if rows.is_empty() {
        return Ok(0);
    }
    if row_width_words == 0 {
        return Err(AccelError::InvalidDomain {
            bits: row_width_words,
            len: rows.len(),
        });
    }
    for &row in rows {
        if row >= row_count {
            return Err(AccelError::InvalidDomain {
                bits: row_count,
                len: row,
            });
        }
    }
    rows.len()
        .checked_mul(row_width_words)
        .ok_or(AccelError::InvalidDomain {
            bits: row_width_words,
            len: rows.len(),
        })
}

#[allow(clippy::too_many_arguments)]
fn validate_zisk_main_trace_descriptor_selected_row_major_u64_slice_shape(
    descriptor_byte_len: usize,
    descriptor_words: usize,
    descriptor_count: usize,
    row_count: usize,
    row_width_words: usize,
    rows: &[usize],
    start_word: usize,
    slice_width_words: usize,
) -> Result<usize, AccelError> {
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
    if descriptor_byte_len != expected_descriptor_len {
        return Err(AccelError::LengthMismatch {
            lhs: descriptor_byte_len,
            rhs: expected_descriptor_len,
        });
    }
    if rows.is_empty() || slice_width_words == 0 {
        return Ok(0);
    }
    if start_word > row_width_words || slice_width_words > row_width_words - start_word {
        return Err(AccelError::InvalidDomain {
            bits: row_width_words,
            len: slice_width_words,
        });
    }
    for &row in rows {
        if row >= row_count {
            return Err(AccelError::InvalidDomain {
                bits: row_count,
                len: row,
            });
        }
    }
    rows.len()
        .checked_mul(slice_width_words)
        .ok_or(AccelError::InvalidDomain {
            bits: slice_width_words,
            len: rows.len(),
        })
}

fn validate_main_trace_descriptor_selected_row_major_u64_slice_shape(
    descriptor_byte_len: usize,
    descriptor_words: usize,
    descriptor_count: usize,
    row_count: usize,
    row_width_words: usize,
    rows: &[usize],
    start_word: usize,
    slice_width_words: usize,
) -> Result<usize, AccelError> {
    if !main_trace_descriptor_words_supported(descriptor_words)
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
    if descriptor_byte_len != expected_descriptor_len {
        return Err(AccelError::LengthMismatch {
            lhs: descriptor_byte_len,
            rhs: expected_descriptor_len,
        });
    }
    if rows.is_empty() || slice_width_words == 0 {
        return Ok(0);
    }
    if start_word > row_width_words || slice_width_words > row_width_words - start_word {
        return Err(AccelError::InvalidDomain {
            bits: start_word,
            len: slice_width_words,
        });
    }
    for row in rows {
        if *row >= row_count {
            return Err(AccelError::InvalidDomain {
                bits: row_count,
                len: rows.len(),
            });
        }
    }
    rows.len()
        .checked_mul(slice_width_words)
        .ok_or(AccelError::InvalidDomain {
            bits: slice_width_words,
            len: rows.len(),
        })
}

fn validate_device_row_major_u64_rows_shape(
    sources: &[(&CudaDeviceBuffer, usize, usize)],
    row_width_words: usize,
) -> Result<usize, AccelError> {
    if sources.is_empty() {
        return Ok(0);
    }
    if row_width_words == 0 {
        return Err(AccelError::InvalidDomain {
            bits: row_width_words,
            len: sources.len(),
        });
    }
    for (source, row_count, row_index) in sources {
        if !source.len.is_multiple_of(8) {
            return Err(AccelError::LengthMismatch {
                lhs: source.len,
                rhs: source.len / 8 * 8,
            });
        }
        let expected_words =
            row_count
                .checked_mul(row_width_words)
                .ok_or(AccelError::InvalidDomain {
                    bits: row_width_words,
                    len: *row_count,
                })?;
        let source_words = source.len / 8;
        if source_words != expected_words {
            return Err(AccelError::LengthMismatch {
                lhs: source_words,
                rhs: expected_words,
            });
        }
        if row_index >= row_count {
            return Err(AccelError::InvalidDomain {
                bits: *row_count,
                len: *row_index,
            });
        }
    }
    sources
        .len()
        .checked_mul(row_width_words)
        .ok_or(AccelError::InvalidDomain {
            bits: row_width_words,
            len: sources.len(),
        })
}

fn validate_device_row_major_u64_row_concat_shape(
    left_word_count: usize,
    left_row_count: usize,
    left_width_words: usize,
    right_word_count: usize,
    right_row_count: usize,
    right_width_words: usize,
) -> Result<usize, AccelError> {
    if left_row_count != right_row_count {
        return Err(AccelError::LengthMismatch {
            lhs: left_row_count,
            rhs: right_row_count,
        });
    }
    let expected_left_words =
        left_row_count
            .checked_mul(left_width_words)
            .ok_or(AccelError::InvalidDomain {
                bits: left_width_words,
                len: left_row_count,
            })?;
    if left_word_count != expected_left_words {
        return Err(AccelError::LengthMismatch {
            lhs: left_word_count,
            rhs: expected_left_words,
        });
    }
    let expected_right_words =
        right_row_count
            .checked_mul(right_width_words)
            .ok_or(AccelError::InvalidDomain {
                bits: right_width_words,
                len: right_row_count,
            })?;
    if right_word_count != expected_right_words {
        return Err(AccelError::LengthMismatch {
            lhs: right_word_count,
            rhs: expected_right_words,
        });
    }
    let output_width =
        left_width_words
            .checked_add(right_width_words)
            .ok_or(AccelError::InvalidDomain {
                bits: left_width_words,
                len: right_width_words,
            })?;
    left_row_count
        .checked_mul(output_width)
        .ok_or(AccelError::InvalidDomain {
            bits: output_width,
            len: left_row_count,
        })
}

fn validate_device_state_prefix_shape(
    source_len: usize,
    state_count: usize,
    state_width_words: usize,
    prefix_words: usize,
) -> Result<usize, AccelError> {
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
    if source_len != expected_input_len {
        return Err(AccelError::LengthMismatch {
            lhs: source_len,
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
    u64_word_byte_len(output_words)
}

fn u32_word_byte_len(word_count: usize) -> Result<usize, AccelError> {
    word_count.checked_mul(4).ok_or(AccelError::InvalidDomain {
        bits: 32,
        len: word_count,
    })
}

#[derive(Debug)]
pub struct CudaPinnedHostBuffer {
    ptr: *mut c_void,
    len: usize,
}

unsafe impl Send for CudaPinnedHostBuffer {}

impl CudaPinnedHostBuffer {
    pub fn new(len: usize) -> Result<Self, AccelError> {
        let mut ptr = ptr::null_mut();
        let code = unsafe { lzvm_cuda_pinned_host_alloc(&mut ptr, len) };
        cuda_status(code)?;
        if len > 0 && ptr.is_null() {
            return Err(AccelError::Cuda { code: -1 });
        }
        Ok(Self { ptr, len })
    }

    #[cfg(target_endian = "little")]
    pub fn from_u64_words(words: &[u64]) -> Result<Self, AccelError> {
        let len = u64_word_byte_len(words.len())?;
        let mut ptr = ptr::null_mut();
        let code = lzvm_cuda_pinned_host_alloc_copy_from(&mut ptr, words.as_ptr().cast(), len);
        cuda_status(code)?;
        if len > 0 && ptr.is_null() {
            return Err(AccelError::Cuda { code: -1 });
        }
        Ok(Self { ptr, len })
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    fn as_mut_raw_ptr(&mut self) -> *mut c_void {
        self.ptr
    }

    /// Returns the host bytes after the caller has synchronized the stream
    /// that last wrote this buffer.
    ///
    /// # Safety
    ///
    /// The caller must ensure no CUDA operation is still writing this memory.
    pub unsafe fn as_bytes(&self) -> &[u8] {
        if self.len == 0 {
            return &[];
        }
        unsafe { std::slice::from_raw_parts(self.ptr.cast(), self.len) }
    }

    /// Returns mutable host bytes after the caller has ensured CUDA is not
    /// reading or writing this memory.
    ///
    /// # Safety
    ///
    /// The caller must ensure no CUDA operation is still using this memory.
    pub unsafe fn as_mut_bytes(&mut self) -> &mut [u8] {
        if self.len == 0 {
            return &mut [];
        }
        unsafe { std::slice::from_raw_parts_mut(self.ptr.cast(), self.len) }
    }
}

impl Drop for CudaPinnedHostBuffer {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe { lzvm_cuda_pinned_host_free(self.ptr) };
            self.ptr = ptr::null_mut();
        }
    }
}

#[derive(Debug)]
pub struct CudaDeviceBuffer {
    ptr: *mut c_void,
    len: usize,
}

#[derive(Debug)]
pub struct CudaPendingTraceDescriptorExpansion {
    descriptor_buffer: CudaDeviceBuffer,
    output_buffer: CudaDeviceBuffer,
}

impl CudaPendingTraceDescriptorExpansion {
    pub fn output(&self) -> &CudaDeviceBuffer {
        &self.output_buffer
    }

    pub fn into_output(self) -> CudaDeviceBuffer {
        self.output_buffer
    }

    pub fn into_parts(self) -> (CudaDeviceBuffer, CudaDeviceBuffer) {
        (self.descriptor_buffer, self.output_buffer)
    }

    pub fn descriptor_buffer_len(&self) -> usize {
        self.descriptor_buffer.len()
    }
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

    /// Enqueues zero-initialization on `stream` and returns after launch.
    ///
    /// # Safety
    ///
    /// The caller must keep the returned buffer and `stream` alive until the
    /// queued work has completed, and must not read or reuse the buffer until
    /// that work has completed.
    pub unsafe fn zeroed_on_stream(len: usize, stream: &CudaStream) -> Result<Self, AccelError> {
        let buffer = Self::new(len)?;
        if len > 0 {
            let code =
                unsafe { lzvm_cuda_memset_zero_bytes_on_stream(buffer.ptr, len, stream.as_raw()) };
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

    #[track_caller]
    pub fn from_u64_words(words: &[u64]) -> Result<Self, AccelError> {
        let mut buffer = Self::new(u64_word_byte_len(words.len())?)?;
        buffer.copy_from_u64_words(words)?;
        Ok(buffer)
    }

    #[track_caller]
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

    pub fn from_device_selected_row_major_u64_rows(
        source: &Self,
        row_count: usize,
        row_width_words: usize,
        rows: &[usize],
    ) -> Result<Self, AccelError> {
        if !source.len.is_multiple_of(8) {
            return Err(AccelError::LengthMismatch {
                lhs: source.len,
                rhs: source.len / 8 * 8,
            });
        }
        let output_words = validate_selected_row_major_u64_rows_shape(
            source.len / 8,
            row_count,
            row_width_words,
            rows,
        )?;
        let mut buffer = Self::new(u64_word_byte_len(output_words)?)?;
        buffer.copy_from_device_selected_row_major_u64_rows(
            source,
            row_count,
            row_width_words,
            rows,
        )?;
        Ok(buffer)
    }

    pub fn from_device_row_major_u64_rows(
        sources: &[(&Self, usize, usize)],
        row_width_words: usize,
    ) -> Result<Self, AccelError> {
        let output_words = validate_device_row_major_u64_rows_shape(sources, row_width_words)?;
        let mut buffer = Self::new(u64_word_byte_len(output_words)?)?;
        buffer.copy_from_device_row_major_u64_rows(sources, row_width_words)?;
        Ok(buffer)
    }

    pub fn from_device_row_major_u64_row_concat(
        left: &Self,
        left_row_count: usize,
        left_width_words: usize,
        right: &Self,
        right_row_count: usize,
        right_width_words: usize,
    ) -> Result<Self, AccelError> {
        if !left.len.is_multiple_of(8) {
            return Err(AccelError::LengthMismatch {
                lhs: left.len,
                rhs: left.len / 8 * 8,
            });
        }
        if !right.len.is_multiple_of(8) {
            return Err(AccelError::LengthMismatch {
                lhs: right.len,
                rhs: right.len / 8 * 8,
            });
        }
        let output_words = validate_device_row_major_u64_row_concat_shape(
            left.len / 8,
            left_row_count,
            left_width_words,
            right.len / 8,
            right_row_count,
            right_width_words,
        )?;
        let mut buffer = Self::new(u64_word_byte_len(output_words)?)?;
        buffer.copy_from_device_row_major_u64_row_concat(
            left,
            left_row_count,
            left_width_words,
            right,
            right_row_count,
            right_width_words,
        )?;
        Ok(buffer)
    }

    /// Enqueues a device-to-device row-major slice copy on `stream`.
    ///
    /// # Safety
    ///
    /// The caller must keep `source`, the returned buffer, and `stream` alive
    /// until the queued copy has completed, and must not read or reuse the
    /// returned buffer until that work has completed.
    pub unsafe fn from_device_row_major_u64_slice_on_stream(
        source: &Self,
        row_count: usize,
        source_width_words: usize,
        start_word: usize,
        slice_width_words: usize,
        stream: &CudaStream,
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
        unsafe {
            buffer.copy_from_device_row_major_u64_slice_on_stream(
                source,
                row_count,
                source_width_words,
                start_word,
                slice_width_words,
                stream,
            )?;
        }
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

    #[allow(clippy::too_many_arguments)]
    pub fn from_zisk_main_trace_descriptor_selected_row_major_u64_slice(
        descriptors: &[u64],
        descriptor_words: usize,
        descriptor_count: usize,
        row_count: usize,
        row_width_words: usize,
        terminal_pc: u64,
        rows: &[usize],
        start_word: usize,
        slice_width_words: usize,
    ) -> Result<Self, AccelError> {
        let descriptor_byte_len = u64_word_byte_len(descriptors.len())?;
        validate_zisk_main_trace_descriptor_selected_row_major_u64_slice_shape(
            descriptor_byte_len,
            descriptor_words,
            descriptor_count,
            row_count,
            row_width_words,
            rows,
            start_word,
            slice_width_words,
        )?;
        let descriptor_buffer = Self::from_u64_words(descriptors)?;
        Self::from_zisk_main_trace_descriptors_device_selected_row_major_u64_slice(
            &descriptor_buffer,
            descriptor_words,
            descriptor_count,
            row_count,
            row_width_words,
            terminal_pc,
            rows,
            start_word,
            slice_width_words,
        )
    }

    pub fn from_sparse_zisk_main_trace_descriptors(
        descriptors: &[u64],
        high_words: &[u64],
        descriptor_count: usize,
        row_count: usize,
        row_width_words: usize,
        terminal_pc: u64,
    ) -> Result<Self, AccelError> {
        let output_byte_len = validate_sparse_zisk_main_trace_descriptors(
            descriptors,
            high_words,
            descriptor_count,
            row_count,
            row_width_words,
        )?;
        let descriptor_buffer = Self::from_u64_words(descriptors)?;
        let high_buffer = Self::from_u64_words(high_words)?;
        Self::from_sparse_zisk_main_trace_descriptors_device(
            &descriptor_buffer,
            &high_buffer,
            high_words.len(),
            descriptor_count,
            row_count,
            row_width_words,
            terminal_pc,
            output_byte_len,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn from_main_trace_descriptors_with_layout(
        descriptors: &[u64],
        descriptor_words: usize,
        descriptor_count: usize,
        row_count: usize,
        row_width_words: usize,
        terminal_pc: u64,
        layout: MainTraceDeviceLayout,
    ) -> Result<Self, AccelError> {
        let descriptor_byte_len = u64_word_byte_len(descriptors.len())?;
        validate_main_trace_descriptor_device_shape(
            descriptor_byte_len,
            descriptor_words,
            descriptor_count,
            row_count,
            row_width_words,
        )?;
        let descriptor_buffer = Self::from_u64_words(descriptors)?;
        Self::from_main_trace_descriptors_device_with_layout(
            &descriptor_buffer,
            descriptor_words,
            descriptor_count,
            row_count,
            row_width_words,
            terminal_pc,
            layout,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn from_sparse_main_trace_descriptors_with_layout(
        descriptors: &[u64],
        high_words: &[u64],
        descriptor_count: usize,
        row_count: usize,
        row_width_words: usize,
        terminal_pc: u64,
        layout: MainTraceDeviceLayout,
    ) -> Result<Self, AccelError> {
        let output_byte_len = validate_sparse_main_trace_descriptors_layout(
            descriptors,
            high_words,
            descriptor_count,
            row_count,
            row_width_words,
        )?;
        let descriptor_buffer = Self::from_u64_words(descriptors)?;
        let high_buffer = Self::from_u64_words(high_words)?;
        let buffer = Self::new(output_byte_len)?;
        if row_count == 0 {
            return Ok(buffer);
        }
        let code = unsafe {
            lzvm_cuda_expand_sparse_main_trace_descriptors_layout(
                buffer.ptr.cast(),
                descriptor_buffer.ptr.cast(),
                high_buffer.ptr.cast(),
                high_words.len(),
                descriptor_count,
                row_count,
                row_width_words,
                terminal_pc,
                layout.raw(),
            )
        };
        cuda_status(code)?;
        Ok(buffer)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn from_main_trace_descriptor_selected_row_major_u64_slice_with_layout(
        descriptors: &[u64],
        descriptor_words: usize,
        descriptor_count: usize,
        row_count: usize,
        row_width_words: usize,
        terminal_pc: u64,
        rows: &[usize],
        start_word: usize,
        slice_width_words: usize,
        layout: MainTraceDeviceLayout,
    ) -> Result<Self, AccelError> {
        let descriptor_byte_len = u64_word_byte_len(descriptors.len())?;
        validate_main_trace_descriptor_selected_row_major_u64_slice_shape(
            descriptor_byte_len,
            descriptor_words,
            descriptor_count,
            row_count,
            row_width_words,
            rows,
            start_word,
            slice_width_words,
        )?;
        let descriptor_buffer = Self::from_u64_words(descriptors)?;
        Self::from_main_trace_descriptors_device_selected_row_major_u64_slice_with_layout(
            &descriptor_buffer,
            descriptor_words,
            descriptor_count,
            row_count,
            row_width_words,
            terminal_pc,
            rows,
            start_word,
            slice_width_words,
            layout,
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
        let output_byte_len = validate_zisk_main_trace_descriptor_device_shape(
            descriptors.len(),
            descriptor_words,
            descriptor_count,
            row_count,
            row_width_words,
        )?;
        let buffer = Self::new(output_byte_len)?;
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

    #[allow(clippy::too_many_arguments)]
    pub fn from_main_trace_descriptors_device_with_layout(
        descriptors: &Self,
        descriptor_words: usize,
        descriptor_count: usize,
        row_count: usize,
        row_width_words: usize,
        terminal_pc: u64,
        layout: MainTraceDeviceLayout,
    ) -> Result<Self, AccelError> {
        let output_byte_len = validate_main_trace_descriptor_device_shape(
            descriptors.len(),
            descriptor_words,
            descriptor_count,
            row_count,
            row_width_words,
        )?;
        let buffer = Self::new(output_byte_len)?;
        if row_count == 0 {
            return Ok(buffer);
        }
        let code = unsafe {
            lzvm_cuda_expand_main_trace_descriptors_layout(
                buffer.ptr.cast(),
                descriptors.ptr.cast(),
                descriptor_words,
                descriptor_count,
                row_count,
                row_width_words,
                terminal_pc,
                layout.raw(),
            )
        };
        cuda_status(code)?;
        Ok(buffer)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn from_main_trace_descriptors_device_selected_row_major_u64_slice_with_layout(
        descriptors: &Self,
        descriptor_words: usize,
        descriptor_count: usize,
        row_count: usize,
        row_width_words: usize,
        terminal_pc: u64,
        rows: &[usize],
        start_word: usize,
        slice_width_words: usize,
        layout: MainTraceDeviceLayout,
    ) -> Result<Self, AccelError> {
        let output_words = validate_main_trace_descriptor_selected_row_major_u64_slice_shape(
            descriptors.len,
            descriptor_words,
            descriptor_count,
            row_count,
            row_width_words,
            rows,
            start_word,
            slice_width_words,
        )?;
        let buffer = Self::new(u64_word_byte_len(output_words)?)?;
        if output_words == 0 {
            return Ok(buffer);
        }
        let row_indices = rows
            .iter()
            .copied()
            .map(u64::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| AccelError::InvalidDomain {
                bits: row_count,
                len: rows.len(),
            })?;
        let row_buffer = Self::from_u64_words(&row_indices)?;
        let code = unsafe {
            lzvm_cuda_expand_main_trace_descriptor_selected_row_major_u64_slice_layout(
                buffer.ptr.cast(),
                descriptors.ptr.cast(),
                descriptor_words,
                descriptor_count,
                row_count,
                row_width_words,
                terminal_pc,
                row_buffer.ptr.cast(),
                rows.len(),
                start_word,
                slice_width_words,
                layout.raw(),
            )
        };
        cuda_status(code)?;
        Ok(buffer)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn from_zisk_main_trace_descriptors_device_selected_row_major_u64_slice(
        descriptors: &Self,
        descriptor_words: usize,
        descriptor_count: usize,
        row_count: usize,
        row_width_words: usize,
        terminal_pc: u64,
        rows: &[usize],
        start_word: usize,
        slice_width_words: usize,
    ) -> Result<Self, AccelError> {
        let output_words = validate_zisk_main_trace_descriptor_selected_row_major_u64_slice_shape(
            descriptors.len,
            descriptor_words,
            descriptor_count,
            row_count,
            row_width_words,
            rows,
            start_word,
            slice_width_words,
        )?;
        let buffer = Self::new(u64_word_byte_len(output_words)?)?;
        if output_words == 0 {
            return Ok(buffer);
        }
        let row_indices = rows
            .iter()
            .copied()
            .map(u64::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| AccelError::InvalidDomain {
                bits: row_count,
                len: rows.len(),
            })?;
        let row_buffer = Self::from_u64_words(&row_indices)?;
        let code = unsafe {
            lzvm_cuda_expand_zisk_main_trace_descriptor_selected_row_major_u64_slice(
                buffer.ptr.cast(),
                descriptors.ptr.cast(),
                descriptor_words,
                descriptor_count,
                row_count,
                row_width_words,
                terminal_pc,
                row_buffer.ptr.cast(),
                rows.len(),
                start_word,
                slice_width_words,
            )
        };
        cuda_status(code)?;
        Ok(buffer)
    }

    #[allow(clippy::too_many_arguments)]
    fn from_sparse_zisk_main_trace_descriptors_device(
        descriptors: &Self,
        high_words: &Self,
        high_word_count: usize,
        descriptor_count: usize,
        row_count: usize,
        row_width_words: usize,
        terminal_pc: u64,
        output_byte_len: usize,
    ) -> Result<Self, AccelError> {
        let buffer = Self::new(output_byte_len)?;
        if row_count == 0 {
            return Ok(buffer);
        }
        let code = unsafe {
            lzvm_cuda_expand_sparse_zisk_main_trace_descriptors(
                buffer.ptr.cast(),
                descriptors.ptr.cast(),
                high_words.ptr.cast(),
                high_word_count,
                descriptor_count,
                row_count,
                row_width_words,
                terminal_pc,
            )
        };
        cuda_status(code)?;
        Ok(buffer)
    }

    /// Enqueues descriptor expansion on `stream`.
    ///
    /// # Safety
    ///
    /// The caller must keep `descriptors` and the returned buffer alive until
    /// the stream has completed the queued kernel, and must not read the
    /// returned buffer before synchronizing the stream or an event recorded
    /// after this call.
    pub unsafe fn from_zisk_main_trace_descriptors_device_on_stream(
        descriptors: &Self,
        descriptor_words: usize,
        descriptor_count: usize,
        row_count: usize,
        row_width_words: usize,
        terminal_pc: u64,
        stream: &CudaStream,
    ) -> Result<Self, AccelError> {
        let output_byte_len = validate_zisk_main_trace_descriptor_device_shape(
            descriptors.len(),
            descriptor_words,
            descriptor_count,
            row_count,
            row_width_words,
        )?;
        let buffer = Self::new(output_byte_len)?;
        if row_count == 0 {
            return Ok(buffer);
        }
        let code = unsafe {
            lzvm_cuda_expand_zisk_main_trace_descriptors_on_stream(
                buffer.ptr.cast(),
                descriptors.ptr.cast(),
                descriptor_words,
                descriptor_count,
                row_count,
                row_width_words,
                terminal_pc,
                stream.as_raw(),
            )
        };
        cuda_status(code)?;
        Ok(buffer)
    }

    #[allow(clippy::too_many_arguments)]
    pub unsafe fn from_main_trace_descriptors_device_on_stream_with_layout(
        descriptors: &Self,
        descriptor_words: usize,
        descriptor_count: usize,
        row_count: usize,
        row_width_words: usize,
        terminal_pc: u64,
        layout: MainTraceDeviceLayout,
        stream: &CudaStream,
    ) -> Result<Self, AccelError> {
        let output_byte_len = validate_main_trace_descriptor_device_shape(
            descriptors.len(),
            descriptor_words,
            descriptor_count,
            row_count,
            row_width_words,
        )?;
        let buffer = Self::new(output_byte_len)?;
        if row_count == 0 {
            return Ok(buffer);
        }
        let code = unsafe {
            lzvm_cuda_expand_main_trace_descriptors_layout_on_stream(
                buffer.ptr.cast(),
                descriptors.ptr.cast(),
                descriptor_words,
                descriptor_count,
                row_count,
                row_width_words,
                terminal_pc,
                layout.raw(),
                stream.as_raw(),
            )
        };
        cuda_status(code)?;
        Ok(buffer)
    }

    /// Enqueues host descriptor upload followed by descriptor expansion on `stream`.
    ///
    /// # Safety
    ///
    /// The caller must keep `descriptors`, the returned pending object, and
    /// `stream` alive until the stream has completed the queued copy and
    /// kernel, and must not read the pending output before synchronizing the
    /// stream or an event recorded after this call.
    pub unsafe fn begin_trace_descriptor_expansion_on_stream(
        descriptors: &[u64],
        descriptor_words: usize,
        descriptor_count: usize,
        row_count: usize,
        row_width_words: usize,
        terminal_pc: u64,
        stream: &CudaStream,
    ) -> Result<CudaPendingTraceDescriptorExpansion, AccelError> {
        let descriptor_byte_len = u64_word_byte_len(descriptors.len())?;
        validate_zisk_main_trace_descriptor_device_shape(
            descriptor_byte_len,
            descriptor_words,
            descriptor_count,
            row_count,
            row_width_words,
        )?;
        let mut descriptor_buffer = Self::new(descriptor_byte_len)?;
        unsafe {
            descriptor_buffer.copy_from_u64_words_on_stream(descriptors, stream)?;
        }
        let output_buffer = unsafe {
            Self::from_zisk_main_trace_descriptors_device_on_stream(
                &descriptor_buffer,
                descriptor_words,
                descriptor_count,
                row_count,
                row_width_words,
                terminal_pc,
                stream,
            )?
        };
        Ok(CudaPendingTraceDescriptorExpansion {
            descriptor_buffer,
            output_buffer,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub unsafe fn begin_trace_descriptor_expansion_on_stream_with_layout(
        descriptors: &[u64],
        descriptor_words: usize,
        descriptor_count: usize,
        row_count: usize,
        row_width_words: usize,
        terminal_pc: u64,
        layout: MainTraceDeviceLayout,
        stream: &CudaStream,
    ) -> Result<CudaPendingTraceDescriptorExpansion, AccelError> {
        let descriptor_byte_len = u64_word_byte_len(descriptors.len())?;
        validate_main_trace_descriptor_device_shape(
            descriptor_byte_len,
            descriptor_words,
            descriptor_count,
            row_count,
            row_width_words,
        )?;
        let mut descriptor_buffer = Self::new(descriptor_byte_len)?;
        unsafe {
            descriptor_buffer.copy_from_u64_words_on_stream(descriptors, stream)?;
        }
        let output_buffer = unsafe {
            Self::from_main_trace_descriptors_device_on_stream_with_layout(
                &descriptor_buffer,
                descriptor_words,
                descriptor_count,
                row_count,
                row_width_words,
                terminal_pc,
                layout,
                stream,
            )?
        };
        Ok(CudaPendingTraceDescriptorExpansion {
            descriptor_buffer,
            output_buffer,
        })
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
        let output_len = validate_device_state_prefix_shape(
            source.len,
            state_count,
            state_width_words,
            prefix_words,
        )?;
        let buffer = Self::new(output_len)?;
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

    /// Enqueues device-to-device state-prefix expansion on `stream`.
    ///
    /// # Safety
    ///
    /// The caller must keep `source`, the returned buffer, and `stream` alive
    /// until the queued work has completed, and must not read or reuse the
    /// returned buffer until that work has completed.
    pub unsafe fn from_device_state_prefix_u64_words_on_stream(
        source: &Self,
        state_count: usize,
        state_width_words: usize,
        prefix_words: usize,
        stream: &CudaStream,
    ) -> Result<Self, AccelError> {
        let output_len = validate_device_state_prefix_shape(
            source.len,
            state_count,
            state_width_words,
            prefix_words,
        )?;
        let buffer = Self::new(output_len)?;
        if state_count == 0 {
            return Ok(buffer);
        }
        let code = unsafe {
            lzvm_cuda_expand_state_prefix_words_device_to_device_on_stream(
                buffer.ptr,
                source.ptr as *const c_void,
                state_count,
                state_width_words,
                prefix_words,
                stream.as_raw(),
            )
        };
        cuda_status(code)?;
        Ok(buffer)
    }

    #[track_caller]
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
            super::cuda_copy_sites::record_d2h_copy_site_timing("to_u64_words", self.len, || {
                let code = unsafe {
                    lzvm_cuda_copy_d2h_bytes(
                        output.as_mut_ptr().cast(),
                        self.ptr as *const c_void,
                        self.len,
                    )
                };
                cuda_status(code)
            })?;
            Ok(output)
        }
        #[cfg(not(target_endian = "little"))]
        {
            let bytes = self.to_vec()?;
            bytes_to_u64_words(&bytes)
        }
    }

    #[track_caller]
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
            super::cuda_copy_sites::record_h2d_copy_site_timing(
                "copy_from_u64_words",
                self.len,
                || {
                    let code = unsafe {
                        lzvm_cuda_copy_h2d_bytes(self.ptr, words.as_ptr().cast(), self.len)
                    };
                    cuda_status(code)
                },
            )
        }
        #[cfg(not(target_endian = "little"))]
        {
            let bytes = u64_words_to_bytes(words);
            self.copy_from(&bytes)
        }
    }

    /// Enqueues a host-to-device upload on `stream`.
    ///
    /// # Safety
    ///
    /// The caller must keep `words` alive until the stream has completed the
    /// copy or an ordering event recorded after this call has completed, and
    /// must keep this buffer alive until the queued copy has completed.
    #[track_caller]
    pub unsafe fn copy_from_u64_words_on_stream(
        &mut self,
        words: &[u64],
        stream: &CudaStream,
    ) -> Result<(), AccelError> {
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
            super::cuda_copy_sites::record_h2d_copy_site_timing(
                "copy_from_u64_words_on_stream",
                self.len,
                || {
                    let code = unsafe {
                        lzvm_cuda_copy_h2d_bytes_on_stream(
                            self.ptr,
                            words.as_ptr().cast(),
                            self.len,
                            stream.as_raw(),
                        )
                    };
                    cuda_status(code)
                },
            )
        }
        #[cfg(not(target_endian = "little"))]
        {
            self.copy_from_u64_words(words)
        }
    }

    #[track_caller]
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
            super::cuda_copy_sites::record_h2d_copy_site_timing(
                "copy_prefix_from_u64_words",
                copy_len,
                || {
                    let code = unsafe {
                        lzvm_cuda_copy_h2d_bytes(self.ptr, words.as_ptr().cast(), copy_len)
                    };
                    cuda_status(code)
                },
            )
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
            super::cuda_copy_sites::record_h2d_copy_site_timing(
                "copy_prefix_from_u64_words",
                copy_len,
                || {
                    let code = unsafe {
                        lzvm_cuda_copy_h2d_bytes(self.ptr, bytes.as_ptr().cast(), copy_len)
                    };
                    cuda_status(code)
                },
            )
        }
    }

    #[track_caller]
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
            super::cuda_copy_sites::record_h2d_copy_site_timing(
                "copy_prefix_from_u32_words",
                copy_len,
                || {
                    let code = unsafe {
                        lzvm_cuda_copy_h2d_bytes(self.ptr, input.as_ptr().cast(), copy_len)
                    };
                    cuda_status(code)
                },
            )
        }
        #[cfg(not(target_endian = "little"))]
        {
            let mut bytes = Vec::with_capacity(copy_len);
            for word in words {
                bytes.extend_from_slice(&word.to_le_bytes());
            }
            super::cuda_copy_sites::record_h2d_copy_site_timing(
                "copy_prefix_from_u32_words",
                copy_len,
                || {
                    let code = unsafe {
                        lzvm_cuda_copy_h2d_bytes(self.ptr, bytes.as_ptr().cast(), copy_len)
                    };
                    cuda_status(code)
                },
            )
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

    #[track_caller]
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
            super::cuda_copy_sites::record_h2d_copy_site_timing(
                "copy_from_row_major_u64_slice",
                self.len,
                || {
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
                },
            )
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

    pub fn copy_from_device_selected_row_major_u64_rows(
        &mut self,
        source: &Self,
        row_count: usize,
        row_width_words: usize,
        rows: &[usize],
    ) -> Result<(), AccelError> {
        if !source.len.is_multiple_of(8) {
            return Err(AccelError::LengthMismatch {
                lhs: source.len,
                rhs: source.len / 8 * 8,
            });
        }
        let output_words = validate_selected_row_major_u64_rows_shape(
            source.len / 8,
            row_count,
            row_width_words,
            rows,
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
        let row_indices = rows
            .iter()
            .copied()
            .map(u64::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| AccelError::InvalidDomain {
                bits: row_count,
                len: rows.len(),
            })?;
        let row_buffer = CudaDeviceBuffer::from_u64_words(&row_indices)?;
        let code = unsafe {
            lzvm_cuda_copy_d2d_selected_row_major_rows(
                self.ptr,
                source.ptr as *const c_void,
                row_buffer.ptr as *const u64,
                rows.len(),
                row_count,
                row_width_words,
            )
        };
        cuda_status(code)
    }

    pub fn copy_from_device_row_major_u64_rows(
        &mut self,
        sources: &[(&Self, usize, usize)],
        row_width_words: usize,
    ) -> Result<(), AccelError> {
        let output_words = validate_device_row_major_u64_rows_shape(sources, row_width_words)?;
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
        let source_ptrs = sources
            .iter()
            .map(|(source, _, _)| source.ptr as usize as u64)
            .collect::<Vec<_>>();
        let row_indices = sources
            .iter()
            .map(|(_, _, row_index)| u64::try_from(*row_index))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| AccelError::InvalidDomain {
                bits: row_width_words,
                len: sources.len(),
            })?;
        let source_ptr_buffer = CudaDeviceBuffer::from_u64_words(&source_ptrs)?;
        let row_buffer = CudaDeviceBuffer::from_u64_words(&row_indices)?;
        let code = unsafe {
            lzvm_cuda_copy_d2d_row_major_rows(
                self.ptr,
                source_ptr_buffer.ptr as *const usize,
                row_buffer.ptr as *const u64,
                sources.len(),
                row_width_words,
            )
        };
        cuda_status(code)
    }

    pub fn copy_from_device_row_major_u64_row_concat(
        &mut self,
        left: &Self,
        left_row_count: usize,
        left_width_words: usize,
        right: &Self,
        right_row_count: usize,
        right_width_words: usize,
    ) -> Result<(), AccelError> {
        if !left.len.is_multiple_of(8) {
            return Err(AccelError::LengthMismatch {
                lhs: left.len,
                rhs: left.len / 8 * 8,
            });
        }
        if !right.len.is_multiple_of(8) {
            return Err(AccelError::LengthMismatch {
                lhs: right.len,
                rhs: right.len / 8 * 8,
            });
        }
        let output_words = validate_device_row_major_u64_row_concat_shape(
            left.len / 8,
            left_row_count,
            left_width_words,
            right.len / 8,
            right_row_count,
            right_width_words,
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
            lzvm_cuda_copy_d2d_row_major_concat_words(
                self.ptr,
                left.ptr as *const c_void,
                right.ptr as *const c_void,
                left_row_count,
                left_width_words,
                right_width_words,
            )
        };
        cuda_status(code)
    }

    /// Enqueues a device-to-device row-major slice copy into this buffer.
    ///
    /// # Safety
    ///
    /// The caller must keep `source`, this buffer, and `stream` alive until the
    /// queued copy has completed, and must not read or reuse this buffer until
    /// that work has completed.
    pub unsafe fn copy_from_device_row_major_u64_slice_on_stream(
        &mut self,
        source: &Self,
        row_count: usize,
        source_width_words: usize,
        start_word: usize,
        slice_width_words: usize,
        stream: &CudaStream,
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
            lzvm_cuda_copy_d2d_row_slice_words_on_stream(
                self.ptr,
                source.ptr as *const c_void,
                row_count,
                source_width_words,
                start_word,
                slice_width_words,
                stream.as_raw(),
            )
        };
        cuda_status(code)
    }

    #[track_caller]
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
        super::cuda_copy_sites::record_d2h_copy_site_timing(
            "to_state_prefix_u64_words",
            u64_word_byte_len(output_words)?,
            || {
                let code = unsafe {
                    lzvm_cuda_copy_d2h_state_prefix_words(
                        output.as_mut_ptr().cast(),
                        self.ptr as *const c_void,
                        state_count,
                        state_width_words,
                        prefix_words,
                    )
                };
                cuda_status(code)
            },
        )?;
        Ok(output)
    }

    #[track_caller]
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
        super::cuda_copy_sites::record_h2d_copy_site_timing("copy_from", self.len, || {
            let code =
                unsafe { lzvm_cuda_copy_h2d_bytes(self.ptr, input.as_ptr().cast(), self.len) };
            cuda_status(code)
        })
    }

    #[track_caller]
    pub fn copy_from_pinned(&mut self, input: &CudaPinnedHostBuffer) -> Result<(), AccelError> {
        if input.len() != self.len {
            return Err(AccelError::LengthMismatch {
                lhs: self.len,
                rhs: input.len(),
            });
        }
        if self.len == 0 {
            return Ok(());
        }
        super::cuda_copy_sites::record_h2d_copy_site_timing("copy_from_pinned", self.len, || {
            let code = unsafe {
                lzvm_cuda_copy_h2d_pinned_bytes(self.ptr, input.ptr.cast_const(), self.len)
            };
            cuda_status(code)
        })
    }

    #[track_caller]
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
        super::cuda_copy_sites::record_d2h_copy_site_timing("copy_to", self.len, || {
            let code = unsafe {
                lzvm_cuda_copy_d2h_bytes(
                    output.as_mut_ptr().cast(),
                    self.ptr as *const c_void,
                    self.len,
                )
            };
            cuda_status(code)
        })
    }

    /// Enqueues a device-to-page-locked-host copy on `stream`.
    ///
    /// # Safety
    ///
    /// The caller must keep `self`, `output`, and `stream` alive until the copy
    /// has completed, and must not read `output` until that stream has been
    /// synchronized or a later event proves completion.
    #[track_caller]
    pub unsafe fn copy_to_pinned_on_stream(
        &self,
        output: &mut CudaPinnedHostBuffer,
        stream: &CudaStream,
    ) -> Result<(), AccelError> {
        if output.len() != self.len {
            return Err(AccelError::LengthMismatch {
                lhs: self.len,
                rhs: output.len(),
            });
        }
        unsafe { self.copy_range_to_pinned_on_stream(0, output, stream) }
    }

    /// Enqueues a device-to-page-locked-host copy on the legacy default stream.
    ///
    /// # Safety
    ///
    /// The caller must keep `self` and `output` alive until the copy has
    /// completed, and must not read `output` until the device or default stream
    /// has been synchronized.
    #[track_caller]
    pub unsafe fn copy_to_pinned_on_default_stream(
        &self,
        output: &mut CudaPinnedHostBuffer,
    ) -> Result<(), AccelError> {
        if output.len() != self.len {
            return Err(AccelError::LengthMismatch {
                lhs: self.len,
                rhs: output.len(),
            });
        }
        unsafe { self.copy_range_to_pinned_on_default_stream(0, output) }
    }

    #[track_caller]
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
        super::cuda_copy_sites::record_d2h_copy_site_timing("copy_range_to", output.len(), || {
            let code = unsafe {
                lzvm_cuda_copy_d2h_bytes(output.as_mut_ptr().cast(), source, output.len())
            };
            cuda_status(code)
        })
    }

    /// Enqueues a device range to page-locked host memory on the legacy default
    /// stream.
    ///
    /// # Safety
    ///
    /// The caller must keep `self` and `output` alive until the copy has
    /// completed, and must not read `output` until the device or default stream
    /// has been synchronized.
    #[track_caller]
    pub unsafe fn copy_range_to_pinned_on_default_stream(
        &self,
        byte_offset: usize,
        output: &mut CudaPinnedHostBuffer,
    ) -> Result<(), AccelError> {
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
        super::cuda_copy_sites::record_d2h_copy_site_timing(
            "copy_range_to_pinned_on_default_stream",
            output.len(),
            || {
                let code = unsafe {
                    lzvm_cuda_copy_d2h_bytes_on_default_stream(
                        output.as_mut_raw_ptr(),
                        source,
                        output.len(),
                    )
                };
                cuda_status(code)
            },
        )
    }

    /// Enqueues a device range to page-locked host memory on `stream`.
    ///
    /// # Safety
    ///
    /// The caller must keep `self`, `output`, and `stream` alive until the copy
    /// has completed, and must not read `output` until that stream has been
    /// synchronized or a later event proves completion.
    #[track_caller]
    pub unsafe fn copy_range_to_pinned_on_stream(
        &self,
        byte_offset: usize,
        output: &mut CudaPinnedHostBuffer,
        stream: &CudaStream,
    ) -> Result<(), AccelError> {
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
        super::cuda_copy_sites::record_d2h_copy_site_timing(
            "copy_range_to_pinned_on_stream",
            output.len(),
            || {
                let code = unsafe {
                    lzvm_cuda_copy_d2h_bytes_on_stream(
                        output.as_mut_raw_ptr(),
                        source,
                        output.len(),
                        stream.as_raw(),
                    )
                };
                cuda_status(code)
            },
        )
    }

    #[track_caller]
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

#[cfg(test)]
mod tests {
    use super::{AccelError, CudaDeviceBuffer, CudaPinnedHostBuffer, CudaStream};
    use crate::cuda_allocator;

    #[test]
    fn stream_buffer_initialization_on_stream_matches_blocking() {
        let _guard = crate::CUDA_TEST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let stream = CudaStream::new().expect("CUDA stream should create");
        let stream_zeroed =
            unsafe { CudaDeviceBuffer::zeroed_on_stream(32 * std::mem::size_of::<u64>(), &stream) }
                .expect("stream zeroed buffer should enqueue");
        stream
            .synchronize()
            .expect("stream zeroed buffer should finish");
        assert_eq!(
            stream_zeroed
                .to_u64_words()
                .expect("stream zeroed buffer should download"),
            vec![0_u64; 32]
        );

        let state_count = 5;
        let prefix_words = 4;
        let state_width_words = 8;
        let compact_words = (0..state_count * prefix_words)
            .map(|index| (index as u64 + 11) * 17)
            .collect::<Vec<_>>();
        let compact =
            CudaDeviceBuffer::from_u64_words(&compact_words).expect("compact source should upload");
        let blocking = CudaDeviceBuffer::from_device_state_prefix_u64_words(
            &compact,
            state_count,
            state_width_words,
            prefix_words,
        )
        .expect("blocking prefix expansion should run");
        let stream = CudaStream::new().expect("CUDA stream should create");
        let streamed = unsafe {
            CudaDeviceBuffer::from_device_state_prefix_u64_words_on_stream(
                &compact,
                state_count,
                state_width_words,
                prefix_words,
                &stream,
            )
        }
        .expect("stream prefix expansion should enqueue");
        stream
            .synchronize()
            .expect("stream prefix expansion should finish");
        assert_eq!(
            streamed
                .to_u64_words()
                .expect("stream prefix output should download"),
            blocking
                .to_u64_words()
                .expect("blocking prefix output should download")
        );

        let error = unsafe {
            CudaDeviceBuffer::from_device_state_prefix_u64_words_on_stream(
                &compact,
                state_count,
                prefix_words - 1,
                prefix_words,
                &stream,
            )
        }
        .expect_err("invalid stream prefix expansion should be rejected");
        assert!(matches!(error, AccelError::InvalidDomain { .. }));
    }

    #[test]
    fn stream_row_slice_on_stream_matches_blocking() {
        let _guard = crate::CUDA_TEST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let row_count = 6;
        let source_width_words = 9;
        let start_word = 2;
        let slice_width_words = 4;
        let values = (0..row_count * source_width_words)
            .map(|index| (index as u64 + 5) * 23)
            .collect::<Vec<_>>();
        let source = CudaDeviceBuffer::from_u64_words(&values).expect("source should upload");
        let blocking = CudaDeviceBuffer::from_device_row_major_u64_slice(
            &source,
            row_count,
            source_width_words,
            start_word,
            slice_width_words,
        )
        .expect("blocking row slice should copy");
        let stream = CudaStream::new().expect("CUDA stream should create");
        let streamed = unsafe {
            CudaDeviceBuffer::from_device_row_major_u64_slice_on_stream(
                &source,
                row_count,
                source_width_words,
                start_word,
                slice_width_words,
                &stream,
            )
        }
        .expect("stream row slice should enqueue");
        stream
            .synchronize()
            .expect("stream row slice should finish");

        assert_eq!(
            streamed
                .to_u64_words()
                .expect("stream row slice should download"),
            blocking
                .to_u64_words()
                .expect("blocking row slice should download")
        );
    }

    #[test]
    fn device_row_major_concat_combines_rows_on_device() {
        let _guard = crate::CUDA_TEST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let row_count = 4;
        let left_width = 3;
        let right_width = 2;
        let left_words = (0..row_count * left_width)
            .map(|index| 100 + index as u64)
            .collect::<Vec<_>>();
        let right_words = (0..row_count * right_width)
            .map(|index| 900 + index as u64)
            .collect::<Vec<_>>();
        let left = CudaDeviceBuffer::from_u64_words(&left_words).expect("left rows should upload");
        let right =
            CudaDeviceBuffer::from_u64_words(&right_words).expect("right rows should upload");

        let combined = CudaDeviceBuffer::from_device_row_major_u64_row_concat(
            &left,
            row_count,
            left_width,
            &right,
            row_count,
            right_width,
        )
        .expect("device row concat should run");

        let mut expected = Vec::new();
        for row in 0..row_count {
            expected.extend_from_slice(&left_words[row * left_width..(row + 1) * left_width]);
            expected.extend_from_slice(&right_words[row * right_width..(row + 1) * right_width]);
        }
        assert_eq!(
            combined
                .to_u64_words()
                .expect("combined rows should download"),
            expected
        );

        let error = CudaDeviceBuffer::from_device_row_major_u64_row_concat(
            &left,
            row_count,
            left_width,
            &right,
            row_count - 1,
            right_width,
        )
        .expect_err("mismatched row counts should be rejected");
        assert!(matches!(error, AccelError::LengthMismatch { .. }));
    }

    #[test]
    fn async_device_to_pinned_host_copy_on_stream_round_trips_words() {
        let _guard = crate::CUDA_TEST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let words = vec![
            0x0102_0304_0506_0708,
            0x1112_1314_1516_1718,
            0x2122_2324_2526_2728,
            0x3132_3334_3536_3738,
        ];
        let source = CudaDeviceBuffer::from_u64_words(&words).expect("source should upload");
        let stream = CudaStream::new().expect("CUDA stream should create");
        let mut output = CudaPinnedHostBuffer::new(words.len() * std::mem::size_of::<u64>())
            .expect("pinned host buffer should allocate");

        unsafe {
            source
                .copy_to_pinned_on_stream(&mut output, &stream)
                .expect("D2H copy should enqueue");
        }
        stream.synchronize().expect("D2H copy should finish");

        let round_trip = unsafe { output.as_bytes() }
            .chunks_exact(std::mem::size_of::<u64>())
            .map(|chunk| {
                let mut word = [0_u8; std::mem::size_of::<u64>()];
                word.copy_from_slice(chunk);
                u64::from_le_bytes(word)
            })
            .collect::<Vec<_>>();
        assert_eq!(round_trip, words);

        let mut too_short = CudaPinnedHostBuffer::new(output.len() - 1)
            .expect("short pinned host buffer should allocate");
        let error = unsafe { source.copy_to_pinned_on_stream(&mut too_short, &stream) }
            .expect_err("short pinned host buffer should be rejected");
        assert!(matches!(error, AccelError::LengthMismatch { .. }));

        let empty_source = CudaDeviceBuffer::new(0).expect("empty device buffer should allocate");
        let mut empty_output =
            CudaPinnedHostBuffer::new(0).expect("empty pinned host buffer should allocate");
        unsafe {
            empty_source
                .copy_to_pinned_on_stream(&mut empty_output, &stream)
                .expect("empty D2H copy should be accepted");
        }
        assert!(unsafe { empty_output.as_bytes() }.is_empty());
    }

    #[test]
    fn pinned_host_to_device_copy_round_trips_without_registering_host_range() {
        let _guard = crate::CUDA_TEST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        cuda_allocator::cuda_allocator_clear_cache().expect("allocator cache should clear");
        let words = (0..200_000_u64)
            .map(|value| value.wrapping_mul(0x1_0000_0001))
            .collect::<Vec<_>>();
        let byte_len = words.len() * std::mem::size_of::<u64>();
        let pinned = CudaPinnedHostBuffer::from_u64_words(&words)
            .expect("pinned host words should allocate");
        let mut device = CudaDeviceBuffer::new(byte_len).expect("device buffer should allocate");

        device
            .copy_from_pinned(&pinned)
            .expect("pinned H2D copy should run");

        assert_eq!(
            device.to_u64_words().expect("device words should download"),
            words
        );
        let stats = cuda_allocator::cuda_allocator_stats().expect("allocator stats should load");
        assert_eq!(
            stats.cuda_host_register_calls, 0,
            "pinned H2D copy should not register an already pinned source"
        );
        assert_eq!(stats.cuda_copy_h2d_calls, 1);
        assert_eq!(stats.cuda_copy_h2d_bytes, byte_len);

        let mut too_short =
            CudaPinnedHostBuffer::new(byte_len - 1).expect("short pinned buffer should allocate");
        let error = device
            .copy_from_pinned(&too_short)
            .expect_err("short pinned source should be rejected");
        assert!(matches!(error, AccelError::LengthMismatch { .. }));
        unsafe { too_short.as_mut_bytes() }.fill(0);

        cuda_allocator::cuda_allocator_clear_cache().expect("allocator cache should clear");
    }
}
