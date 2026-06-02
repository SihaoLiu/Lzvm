#[cfg(feature = "cuda")]
use std::ffi::c_void;
use std::fmt;
#[cfg(feature = "cuda")]
use std::ptr;

#[cfg(feature = "cuda")]
mod cuda_allocator;
#[cfg(feature = "cuda")]
mod cuda_canonical;
#[cfg(feature = "cuda")]
mod cuda_regular_constraints;
#[cfg(feature = "cuda")]
mod cuda_setup;
#[cfg(feature = "cuda")]
pub use cuda_canonical::cuda_goldilocks_validate_canonical_words_device;
#[cfg(feature = "cuda")]
pub use cuda_regular_constraints::{
    cuda_regular_constraints_base, CudaRegularConstraintEntry, CudaRegularConstraintInputs,
    CudaRegularConstraintResult, CudaRegularStage,
};
#[cfg(feature = "cuda")]
pub use cuda_setup::cuda_setup_init;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccelError {
    LengthMismatch { lhs: usize, rhs: usize },
    InvalidDomain { bits: usize, len: usize },
    CudaUnavailable,
    Cuda { code: i32 },
}

impl fmt::Display for AccelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LengthMismatch { lhs, rhs } => {
                write!(f, "length mismatch: lhs {lhs}, rhs {rhs}")
            }
            Self::InvalidDomain { bits, len } => {
                write!(f, "invalid field domain: bits {bits}, len {len}")
            }
            Self::CudaUnavailable => write!(f, "cuda backend is not enabled"),
            Self::Cuda { code } if *code < 0 => write!(f, "invalid cuda input: {code}"),
            Self::Cuda { code } => write!(f, "cuda backend error: {code}"),
        }
    }
}

impl std::error::Error for AccelError {}

#[cfg(feature = "cuda")]
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
    fn lzvm_cuda_goldilocks_add(lhs: *const u64, rhs: *const u64, out: *mut u64, len: usize)
        -> i32;
    fn lzvm_cuda_goldilocks_mul(lhs: *const u64, rhs: *const u64, out: *mut u64, len: usize)
        -> i32;
    fn lzvm_cuda_goldilocks_butterfly(
        even: *const u64,
        odd: *const u64,
        twiddle: *const u64,
        out_even: *mut u64,
        out_odd: *mut u64,
        len: usize,
    ) -> i32;
    fn lzvm_cuda_goldilocks_ntt(
        values: *const u64,
        out: *mut u64,
        len: usize,
        bits: usize,
        root: u64,
    ) -> i32;
    fn lzvm_cuda_goldilocks_intt(
        values: *const u64,
        out: *mut u64,
        len: usize,
        bits: usize,
        root: u64,
    ) -> i32;
    fn lzvm_cuda_goldilocks_coset_extend(
        values: *const u64,
        out: *mut u64,
        source_len: usize,
        source_bits: usize,
        target_len: usize,
        target_bits: usize,
        source_root_inverse: u64,
        target_root: u64,
        shift: u64,
    ) -> i32;
    fn lzvm_cuda_goldilocks_coset_extend_row_major_columns(
        values: *const u64,
        out: *mut u64,
        source_len: usize,
        source_bits: usize,
        target_len: usize,
        target_bits: usize,
        column_count: usize,
        source_root_inverse: u64,
        target_root: u64,
        shift: u64,
    ) -> i32;
    #[link_name = "lzvm_cuda_goldilocks_coset_extend_row_major_columns_device"]
    fn lzvm_cuda_goldilocks_coset_extend_row_major_columns_device_raw(
        values: *const u64,
        out: *mut u64,
        source_len: usize,
        source_bits: usize,
        target_len: usize,
        target_bits: usize,
        column_count: usize,
        source_root_inverse: u64,
        target_root: u64,
        shift: u64,
    ) -> i32;
    #[link_name = "lzvm_cuda_goldilocks_coset_extend_device"]
    fn lzvm_cuda_goldilocks_coset_extend_device_raw(
        values: *const u64,
        out: *mut u64,
        source_len: usize,
        source_bits: usize,
        target_len: usize,
        target_bits: usize,
        source_root_inverse: u64,
        target_root: u64,
        shift: u64,
    ) -> i32;
    fn lzvm_cuda_poseidon2_width4(values: *const u64, out: *mut u64, state_count: usize) -> i32;
    #[link_name = "lzvm_cuda_poseidon2_width4_device"]
    fn lzvm_cuda_poseidon2_width4_device_raw(
        values: *const u64,
        out: *mut u64,
        state_count: usize,
    ) -> i32;
    fn lzvm_cuda_poseidon2_width4_find_nonce(
        challenge: *const u64,
        start: u64,
        count: usize,
        target: u64,
        out: *mut u64,
        found: *mut u32,
    ) -> i32;
    fn lzvm_cuda_poseidon2_width8(values: *const u64, out: *mut u64, state_count: usize) -> i32;
    #[link_name = "lzvm_cuda_poseidon2_width8_device"]
    fn lzvm_cuda_poseidon2_width8_device_raw(
        values: *const u64,
        out: *mut u64,
        state_count: usize,
    ) -> i32;
    #[link_name = "lzvm_cuda_poseidon2_width8_merkle_parent_device"]
    fn lzvm_cuda_poseidon2_width8_merkle_parent_device_raw(
        values: *const u64,
        out: *mut u64,
        child_state_count: usize,
    ) -> i32;
    #[link_name = "lzvm_cuda_poseidon2_width8_merkle_root_device"]
    fn lzvm_cuda_poseidon2_width8_merkle_root_device_raw(
        values: *const u64,
        out: *mut u64,
        child_state_count: usize,
    ) -> i32;
    #[link_name = "lzvm_cuda_poseidon2_width8_linear_round_device"]
    fn lzvm_cuda_poseidon2_width8_linear_round_device_raw(
        current_states: *const u64,
        row_values: *const u64,
        out: *mut u64,
        row_count: usize,
        chunk_len: usize,
    ) -> i32;
    #[link_name = "lzvm_cuda_poseidon2_width8_linear_round_row_major_device"]
    fn lzvm_cuda_poseidon2_width8_linear_round_row_major_device_raw(
        current_states: *const u64,
        row_values: *const u64,
        out: *mut u64,
        row_count: usize,
        column_count: usize,
        offset: usize,
        chunk_len: usize,
    ) -> i32;
    fn lzvm_cuda_poseidon2_width16(values: *const u64, out: *mut u64, state_count: usize) -> i32;
    #[link_name = "lzvm_cuda_poseidon2_width16_device"]
    fn lzvm_cuda_poseidon2_width16_device_raw(
        values: *const u64,
        out: *mut u64,
        state_count: usize,
    ) -> i32;
    #[link_name = "lzvm_cuda_poseidon2_width16_merkle_parent_device"]
    fn lzvm_cuda_poseidon2_width16_merkle_parent_device_raw(
        values: *const u64,
        out: *mut u64,
        child_state_count: usize,
    ) -> i32;
    #[link_name = "lzvm_cuda_poseidon2_width16_merkle_root_device"]
    fn lzvm_cuda_poseidon2_width16_merkle_root_device_raw(
        values: *const u64,
        out: *mut u64,
        child_state_count: usize,
    ) -> i32;
    #[link_name = "lzvm_cuda_poseidon2_width16_linear_round_device"]
    fn lzvm_cuda_poseidon2_width16_linear_round_device_raw(
        current_states: *const u64,
        row_values: *const u64,
        out: *mut u64,
        row_count: usize,
        chunk_len: usize,
    ) -> i32;
    #[link_name = "lzvm_cuda_poseidon2_width16_linear_round_row_major_device"]
    fn lzvm_cuda_poseidon2_width16_linear_round_row_major_device_raw(
        current_states: *const u64,
        row_values: *const u64,
        out: *mut u64,
        row_count: usize,
        column_count: usize,
        offset: usize,
        chunk_len: usize,
    ) -> i32;
    fn lzvm_cuda_keccak256_fixed(
        input: *const u8,
        message_len: usize,
        out: *mut u8,
        message_count: usize,
    ) -> i32;
}

#[cfg(feature = "cuda")]
const ROOTS_OF_UNITY: [u64; 33] = [
    1,
    18_446_744_069_414_584_320,
    281_474_976_710_656,
    16_777_216,
    4096,
    64,
    8,
    2_198_989_700_608,
    4_404_853_092_538_523_347,
    6_434_636_298_004_421_797,
    4_255_134_452_441_852_017,
    9_113_133_275_150_391_358,
    4_355_325_209_153_869_931,
    4_308_460_244_895_131_701,
    7_126_024_226_993_609_386,
    1_873_558_160_482_552_414,
    8_167_150_655_112_846_419,
    5_718_075_921_287_398_682,
    3_411_401_055_030_829_696,
    8_982_441_859_486_529_725,
    1_971_462_654_193_939_361,
    6_553_637_399_136_210_105,
    8_124_823_329_697_072_476,
    5_936_499_541_590_631_774,
    2_709_866_199_236_980_323,
    8_877_499_657_461_974_390,
    3_757_607_247_483_852_735,
    4_969_973_714_567_017_225,
    2_147_253_751_702_802_259,
    2_530_564_950_562_219_707,
    1_905_180_297_017_055_339,
    3_524_815_499_551_269_279,
    7_277_203_076_849_721_926,
];

#[cfg(feature = "cuda")]
const SHIFT: u64 = 7;

#[cfg(feature = "cuda")]
fn pow_mod(mut base: u64, mut exponent: u64) -> u64 {
    const MODULUS: u64 = 0xffff_ffff_0000_0001;

    let mut result = 1_u64;
    while exponent > 0 {
        if exponent & 1 == 1 {
            result = ((result as u128 * base as u128) % MODULUS as u128) as u64;
        }
        base = ((base as u128 * base as u128) % MODULUS as u128) as u64;
        exponent >>= 1;
    }
    result
}

#[cfg(feature = "cuda")]
fn ensure_cuda_setup(max_bits_ext: usize) -> Result<(), AccelError> {
    cuda_setup_init(max_bits_ext)
}

#[cfg(feature = "cuda")]
fn cuda_status(code: i32) -> Result<(), AccelError> {
    if code == 0 {
        Ok(())
    } else {
        Err(AccelError::Cuda { code })
    }
}

#[cfg(all(feature = "cuda", not(target_endian = "little")))]
fn u64_words_to_bytes(words: &[u64]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(words.len().saturating_mul(8));
    for word in words {
        bytes.extend_from_slice(&word.to_le_bytes());
    }
    bytes
}

#[cfg(all(feature = "cuda", not(target_endian = "little")))]
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

#[cfg(feature = "cuda")]
fn u64_word_byte_len(word_count: usize) -> Result<usize, AccelError> {
    word_count.checked_mul(8).ok_or(AccelError::InvalidDomain {
        bits: 64,
        len: word_count,
    })
}

#[cfg(feature = "cuda")]
fn coset_extend_domain(
    len: usize,
    source_bits: usize,
    target_bits: usize,
) -> Result<(usize, usize, u64, u64), AccelError> {
    if target_bits < source_bits {
        return Err(AccelError::InvalidDomain {
            bits: target_bits,
            len,
        });
    }
    let Some(source_root) = ROOTS_OF_UNITY.get(source_bits).copied() else {
        return Err(AccelError::InvalidDomain {
            bits: source_bits,
            len,
        });
    };
    let Some(target_root) = ROOTS_OF_UNITY.get(target_bits).copied() else {
        return Err(AccelError::InvalidDomain {
            bits: target_bits,
            len,
        });
    };
    let source_len = 1_usize
        .checked_shl(
            u32::try_from(source_bits).map_err(|_| AccelError::InvalidDomain {
                bits: source_bits,
                len,
            })?,
        )
        .ok_or(AccelError::InvalidDomain {
            bits: source_bits,
            len,
        })?;
    let target_len = 1_usize
        .checked_shl(
            u32::try_from(target_bits).map_err(|_| AccelError::InvalidDomain {
                bits: target_bits,
                len,
            })?,
        )
        .ok_or(AccelError::InvalidDomain {
            bits: target_bits,
            len,
        })?;

    Ok((source_len, target_len, source_root, target_root))
}

#[cfg(feature = "cuda")]
#[derive(Debug)]
pub struct CudaDeviceBuffer {
    ptr: *mut c_void,
    len: usize,
}

#[cfg(feature = "cuda")]
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

    pub fn to_vec(&self) -> Result<Vec<u8>, AccelError> {
        let mut output = vec![0_u8; self.len];
        self.copy_to(&mut output)?;
        Ok(output)
    }
}

#[cfg(feature = "cuda")]
impl Drop for CudaDeviceBuffer {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            cuda_allocator::free_bytes(self.ptr);
            self.ptr = ptr::null_mut();
        }
    }
}

#[cfg(feature = "cuda")]
type CudaBinaryOp = unsafe extern "C" fn(*const u64, *const u64, *mut u64, usize) -> i32;

#[cfg(feature = "cuda")]
fn run_cuda_binary_op(
    lhs: &[u64],
    rhs: &[u64],
    operation: CudaBinaryOp,
) -> Result<Vec<u64>, AccelError> {
    if lhs.len() != rhs.len() {
        return Err(AccelError::LengthMismatch {
            lhs: lhs.len(),
            rhs: rhs.len(),
        });
    }

    let mut out = vec![0_u64; lhs.len()];
    let code = if lhs.is_empty() {
        0
    } else {
        unsafe { operation(lhs.as_ptr(), rhs.as_ptr(), out.as_mut_ptr(), lhs.len()) }
    };
    if code == 0 {
        Ok(out)
    } else {
        Err(AccelError::Cuda { code })
    }
}

#[cfg(feature = "cuda")]
pub fn cuda_goldilocks_add(lhs: &[u64], rhs: &[u64]) -> Result<Vec<u64>, AccelError> {
    run_cuda_binary_op(lhs, rhs, lzvm_cuda_goldilocks_add)
}

#[cfg(feature = "cuda")]
pub fn cuda_goldilocks_mul(lhs: &[u64], rhs: &[u64]) -> Result<Vec<u64>, AccelError> {
    run_cuda_binary_op(lhs, rhs, lzvm_cuda_goldilocks_mul)
}

#[cfg(feature = "cuda")]
pub fn cuda_goldilocks_butterfly(
    even: &[u64],
    odd: &[u64],
    twiddle: &[u64],
) -> Result<(Vec<u64>, Vec<u64>), AccelError> {
    if even.len() != odd.len() {
        return Err(AccelError::LengthMismatch {
            lhs: even.len(),
            rhs: odd.len(),
        });
    }
    if even.len() != twiddle.len() {
        return Err(AccelError::LengthMismatch {
            lhs: even.len(),
            rhs: twiddle.len(),
        });
    }

    let mut out_even = vec![0_u64; even.len()];
    let mut out_odd = vec![0_u64; even.len()];
    let code = if even.is_empty() {
        0
    } else {
        unsafe {
            lzvm_cuda_goldilocks_butterfly(
                even.as_ptr(),
                odd.as_ptr(),
                twiddle.as_ptr(),
                out_even.as_mut_ptr(),
                out_odd.as_mut_ptr(),
                even.len(),
            )
        }
    };
    if code == 0 {
        Ok((out_even, out_odd))
    } else {
        Err(AccelError::Cuda { code })
    }
}

#[cfg(feature = "cuda")]
pub fn cuda_goldilocks_ntt(values: &[u64], bits: usize) -> Result<Vec<u64>, AccelError> {
    let Some(root) = ROOTS_OF_UNITY.get(bits).copied() else {
        return Err(AccelError::InvalidDomain {
            bits,
            len: values.len(),
        });
    };
    let expected_len = 1_usize
        .checked_shl(u32::try_from(bits).map_err(|_| AccelError::InvalidDomain {
            bits,
            len: values.len(),
        })?)
        .ok_or(AccelError::InvalidDomain {
            bits,
            len: values.len(),
        })?;
    if values.len() != expected_len {
        return Err(AccelError::InvalidDomain {
            bits,
            len: values.len(),
        });
    }
    ensure_cuda_setup(bits)?;

    let mut out = vec![0_u64; values.len()];
    let code = unsafe {
        lzvm_cuda_goldilocks_ntt(values.as_ptr(), out.as_mut_ptr(), values.len(), bits, root)
    };
    if code == 0 {
        Ok(out)
    } else {
        Err(AccelError::Cuda { code })
    }
}

#[cfg(feature = "cuda")]
pub fn cuda_goldilocks_intt(values: &[u64], bits: usize) -> Result<Vec<u64>, AccelError> {
    let Some(root) = ROOTS_OF_UNITY.get(bits).copied() else {
        return Err(AccelError::InvalidDomain {
            bits,
            len: values.len(),
        });
    };
    let expected_len = 1_usize
        .checked_shl(u32::try_from(bits).map_err(|_| AccelError::InvalidDomain {
            bits,
            len: values.len(),
        })?)
        .ok_or(AccelError::InvalidDomain {
            bits,
            len: values.len(),
        })?;
    if values.len() != expected_len {
        return Err(AccelError::InvalidDomain {
            bits,
            len: values.len(),
        });
    }
    ensure_cuda_setup(bits)?;

    let mut out = vec![0_u64; values.len()];
    let code = unsafe {
        lzvm_cuda_goldilocks_intt(values.as_ptr(), out.as_mut_ptr(), values.len(), bits, root)
    };
    if code == 0 {
        Ok(out)
    } else {
        Err(AccelError::Cuda { code })
    }
}

#[cfg(feature = "cuda")]
pub fn cuda_goldilocks_coset_extend(
    values: &[u64],
    source_bits: usize,
    target_bits: usize,
) -> Result<Vec<u64>, AccelError> {
    let (source_len, target_len, source_root, target_root) =
        coset_extend_domain(values.len(), source_bits, target_bits)?;
    if values.len() != source_len {
        return Err(AccelError::InvalidDomain {
            bits: source_bits,
            len: values.len(),
        });
    }
    ensure_cuda_setup(target_bits)?;

    let mut out = vec![0_u64; target_len];
    let code = unsafe {
        lzvm_cuda_goldilocks_coset_extend(
            values.as_ptr(),
            out.as_mut_ptr(),
            source_len,
            source_bits,
            target_len,
            target_bits,
            pow_mod(source_root, 0xffff_ffff_0000_0001 - 2),
            target_root,
            SHIFT,
        )
    };
    if code == 0 {
        Ok(out)
    } else {
        Err(AccelError::Cuda { code })
    }
}

#[cfg(feature = "cuda")]
pub fn cuda_goldilocks_coset_extend_row_major_columns(
    values: &[u64],
    column_count: usize,
    source_bits: usize,
    target_bits: usize,
) -> Result<Vec<u64>, AccelError> {
    if column_count == 0 {
        if values.is_empty() {
            return Ok(Vec::new());
        }
        return Err(AccelError::InvalidDomain {
            bits: source_bits,
            len: values.len(),
        });
    }
    if !values.len().is_multiple_of(column_count) {
        return Err(AccelError::InvalidDomain {
            bits: source_bits,
            len: values.len(),
        });
    }

    let source_rows = values.len() / column_count;
    let (source_len, target_len, source_root, target_root) =
        coset_extend_domain(source_rows, source_bits, target_bits)?;
    if source_rows != source_len {
        return Err(AccelError::InvalidDomain {
            bits: source_bits,
            len: values.len(),
        });
    }
    ensure_cuda_setup(target_bits)?;

    let out_len = target_len
        .checked_mul(column_count)
        .ok_or(AccelError::InvalidDomain {
            bits: target_bits,
            len: values.len(),
        })?;
    let mut out = vec![0_u64; out_len];
    let code = unsafe {
        lzvm_cuda_goldilocks_coset_extend_row_major_columns(
            values.as_ptr(),
            out.as_mut_ptr(),
            source_len,
            source_bits,
            target_len,
            target_bits,
            column_count,
            pow_mod(source_root, 0xffff_ffff_0000_0001 - 2),
            target_root,
            SHIFT,
        )
    };
    if code == 0 {
        Ok(out)
    } else {
        Err(AccelError::Cuda { code })
    }
}

#[cfg(feature = "cuda")]
pub fn cuda_goldilocks_coset_extend_row_major_columns_output_bytes(
    value_count: usize,
    column_count: usize,
    source_bits: usize,
    target_bits: usize,
) -> Result<usize, AccelError> {
    if column_count == 0 {
        if value_count == 0 {
            return Ok(0);
        }
        return Err(AccelError::InvalidDomain {
            bits: source_bits,
            len: value_count,
        });
    }
    if !value_count.is_multiple_of(column_count) {
        return Err(AccelError::InvalidDomain {
            bits: source_bits,
            len: value_count,
        });
    }

    let source_rows = value_count / column_count;
    let (source_len, target_len, _, _) =
        coset_extend_domain(source_rows, source_bits, target_bits)?;
    if source_rows != source_len {
        return Err(AccelError::InvalidDomain {
            bits: source_bits,
            len: value_count,
        });
    }
    let target_words = target_len
        .checked_mul(column_count)
        .ok_or(AccelError::InvalidDomain {
            bits: target_bits,
            len: value_count,
        })?;
    target_words
        .checked_mul(8)
        .ok_or(AccelError::InvalidDomain {
            bits: target_bits,
            len: target_words,
        })
}

#[cfg(feature = "cuda")]
pub fn cuda_goldilocks_coset_extend_row_major_columns_device(
    values: &CudaDeviceBuffer,
    out: &mut CudaDeviceBuffer,
    column_count: usize,
    source_bits: usize,
    target_bits: usize,
) -> Result<(), AccelError> {
    if column_count == 0 {
        if values.is_empty() && out.is_empty() {
            return Ok(());
        }
        return Err(AccelError::InvalidDomain {
            bits: source_bits,
            len: values.len(),
        });
    }
    if !values.len().is_multiple_of(8) {
        return Err(AccelError::LengthMismatch {
            lhs: values.len(),
            rhs: values.len() / 8 * 8,
        });
    }
    if !out.len().is_multiple_of(8) {
        return Err(AccelError::LengthMismatch {
            lhs: out.len(),
            rhs: out.len() / 8 * 8,
        });
    }

    let source_words = values.len() / 8;
    if !source_words.is_multiple_of(column_count) {
        return Err(AccelError::InvalidDomain {
            bits: source_bits,
            len: source_words,
        });
    }
    let source_rows = source_words / column_count;
    let (source_len, target_len, source_root, target_root) =
        coset_extend_domain(source_rows, source_bits, target_bits)?;
    if source_rows != source_len {
        return Err(AccelError::InvalidDomain {
            bits: source_bits,
            len: source_words,
        });
    }
    let target_words = target_len
        .checked_mul(column_count)
        .ok_or(AccelError::InvalidDomain {
            bits: target_bits,
            len: source_words,
        })?;
    let target_bytes = target_words
        .checked_mul(8)
        .ok_or(AccelError::InvalidDomain {
            bits: target_bits,
            len: target_words,
        })?;
    if out.len() != target_bytes {
        return Err(AccelError::LengthMismatch {
            lhs: target_bytes,
            rhs: out.len(),
        });
    }
    ensure_cuda_setup(target_bits)?;

    let code = unsafe {
        lzvm_cuda_goldilocks_coset_extend_row_major_columns_device_raw(
            values.as_raw_ptr() as *const u64,
            out.as_raw_ptr() as *mut u64,
            source_len,
            source_bits,
            target_len,
            target_bits,
            column_count,
            pow_mod(source_root, 0xffff_ffff_0000_0001 - 2),
            target_root,
            SHIFT,
        )
    };
    cuda_status(code)
}

#[cfg(feature = "cuda")]
pub fn cuda_goldilocks_coset_extend_device(
    values: &CudaDeviceBuffer,
    out: &mut CudaDeviceBuffer,
    source_bits: usize,
    target_bits: usize,
) -> Result<(), AccelError> {
    let (source_len, target_len, source_root, target_root) =
        coset_extend_domain(values.len(), source_bits, target_bits)?;
    let source_bytes = source_len.checked_mul(8).ok_or(AccelError::InvalidDomain {
        bits: source_bits,
        len: values.len(),
    })?;
    let target_bytes = target_len.checked_mul(8).ok_or(AccelError::InvalidDomain {
        bits: target_bits,
        len: out.len(),
    })?;
    if values.len() != source_bytes {
        return Err(AccelError::LengthMismatch {
            lhs: source_bytes,
            rhs: values.len(),
        });
    }
    if out.len() != target_bytes {
        return Err(AccelError::LengthMismatch {
            lhs: target_bytes,
            rhs: out.len(),
        });
    }
    ensure_cuda_setup(target_bits)?;

    let code = unsafe {
        lzvm_cuda_goldilocks_coset_extend_device_raw(
            values.as_raw_ptr() as *const u64,
            out.as_raw_ptr() as *mut u64,
            source_len,
            source_bits,
            target_len,
            target_bits,
            pow_mod(source_root, 0xffff_ffff_0000_0001 - 2),
            target_root,
            SHIFT,
        )
    };
    cuda_status(code)
}

#[cfg(feature = "cuda")]
pub fn cuda_poseidon2_width4(values: &[u64]) -> Result<Vec<u64>, AccelError> {
    const WIDTH: usize = 4;

    if !values.len().is_multiple_of(WIDTH) {
        return Err(AccelError::InvalidDomain {
            bits: 2,
            len: values.len(),
        });
    }
    let mut out = vec![0_u64; values.len()];
    let code = if values.is_empty() {
        0
    } else {
        unsafe {
            lzvm_cuda_poseidon2_width4(values.as_ptr(), out.as_mut_ptr(), values.len() / WIDTH)
        }
    };
    if code == 0 {
        Ok(out)
    } else {
        Err(AccelError::Cuda { code })
    }
}

#[cfg(feature = "cuda")]
type CudaPoseidon2DeviceOp = unsafe extern "C" fn(*const u64, *mut u64, usize) -> i32;

#[cfg(feature = "cuda")]
fn run_cuda_poseidon2_device_op(
    values: &CudaDeviceBuffer,
    out: &mut CudaDeviceBuffer,
    width: usize,
    bits: usize,
    operation: CudaPoseidon2DeviceOp,
) -> Result<(), AccelError> {
    if !values.len().is_multiple_of(8) {
        return Err(AccelError::LengthMismatch {
            lhs: values.len(),
            rhs: values.len() / 8 * 8,
        });
    }
    if values.len() != out.len() {
        return Err(AccelError::LengthMismatch {
            lhs: values.len(),
            rhs: out.len(),
        });
    }

    let word_count = values.len() / 8;
    if !word_count.is_multiple_of(width) {
        return Err(AccelError::InvalidDomain {
            bits,
            len: word_count,
        });
    }
    if word_count == 0 {
        return Ok(());
    }

    let code = unsafe {
        operation(
            values.as_raw_ptr() as *const u64,
            out.as_raw_ptr() as *mut u64,
            word_count / width,
        )
    };
    cuda_status(code)
}

#[cfg(feature = "cuda")]
type CudaPoseidon2MerkleParentDeviceOp = unsafe extern "C" fn(*const u64, *mut u64, usize) -> i32;

#[cfg(feature = "cuda")]
fn run_cuda_poseidon2_merkle_parent_device_op(
    values: &CudaDeviceBuffer,
    out: &mut CudaDeviceBuffer,
    width: usize,
    arity: usize,
    bits: usize,
    operation: CudaPoseidon2MerkleParentDeviceOp,
) -> Result<(), AccelError> {
    if !values.len().is_multiple_of(8) {
        return Err(AccelError::LengthMismatch {
            lhs: values.len(),
            rhs: values.len() / 8 * 8,
        });
    }

    let child_word_count = values.len() / 8;
    if !child_word_count.is_multiple_of(width) {
        return Err(AccelError::InvalidDomain {
            bits,
            len: child_word_count,
        });
    }

    let child_state_count = child_word_count / width;
    let parent_state_count = child_state_count.div_ceil(arity);
    let expected_out_bytes = parent_state_count
        .checked_mul(width)
        .and_then(|word_count| word_count.checked_mul(8))
        .ok_or(AccelError::InvalidDomain {
            bits,
            len: child_word_count,
        })?;

    if out.len() != expected_out_bytes {
        return Err(AccelError::LengthMismatch {
            lhs: expected_out_bytes,
            rhs: out.len(),
        });
    }
    if child_state_count == 0 {
        return Ok(());
    }

    let code = unsafe {
        operation(
            values.as_raw_ptr() as *const u64,
            out.as_raw_ptr() as *mut u64,
            child_state_count,
        )
    };
    cuda_status(code)
}

#[cfg(feature = "cuda")]
fn run_cuda_poseidon2_merkle_root_device_op(
    values: &CudaDeviceBuffer,
    width: usize,
    bits: usize,
    operation: CudaPoseidon2MerkleParentDeviceOp,
) -> Result<[u64; 4], AccelError> {
    if !values.len().is_multiple_of(8) {
        return Err(AccelError::LengthMismatch {
            lhs: values.len(),
            rhs: values.len() / 8 * 8,
        });
    }

    let child_word_count = values.len() / 8;
    if !child_word_count.is_multiple_of(width) {
        return Err(AccelError::InvalidDomain {
            bits,
            len: child_word_count,
        });
    }

    let child_state_count = child_word_count / width;
    if child_state_count == 0 {
        return Ok([0; 4]);
    }

    let out = CudaDeviceBuffer::new(width.checked_mul(8).ok_or(AccelError::InvalidDomain {
        bits,
        len: child_word_count,
    })?)?;
    let code = unsafe {
        operation(
            values.as_raw_ptr() as *const u64,
            out.as_raw_ptr() as *mut u64,
            child_state_count,
        )
    };
    cuda_status(code)?;

    let root = out.to_u64_words()?;
    Ok([root[0], root[1], root[2], root[3]])
}

#[cfg(feature = "cuda")]
type CudaPoseidon2LinearRoundDeviceOp =
    unsafe extern "C" fn(*const u64, *const u64, *mut u64, usize, usize) -> i32;

#[cfg(feature = "cuda")]
type CudaPoseidon2LinearRoundRowMajorDeviceOp =
    unsafe extern "C" fn(*const u64, *const u64, *mut u64, usize, usize, usize, usize) -> i32;

#[cfg(feature = "cuda")]
struct CudaLinearRoundRowMajorParams {
    width: usize,
    rate: usize,
    column_count: usize,
    offset: usize,
    chunk_len: usize,
}

#[cfg(feature = "cuda")]
fn run_cuda_poseidon2_linear_round_device_op(
    current_states: &CudaDeviceBuffer,
    row_values: &CudaDeviceBuffer,
    out: &mut CudaDeviceBuffer,
    width: usize,
    rate: usize,
    chunk_len: usize,
    operation: CudaPoseidon2LinearRoundDeviceOp,
) -> Result<(), AccelError> {
    if !current_states.len().is_multiple_of(8) {
        return Err(AccelError::LengthMismatch {
            lhs: current_states.len(),
            rhs: current_states.len() / 8 * 8,
        });
    }
    if !row_values.len().is_multiple_of(8) {
        return Err(AccelError::LengthMismatch {
            lhs: row_values.len(),
            rhs: row_values.len() / 8 * 8,
        });
    }
    if current_states.len() != out.len() {
        return Err(AccelError::LengthMismatch {
            lhs: current_states.len(),
            rhs: out.len(),
        });
    }
    if chunk_len == 0 || chunk_len > rate {
        return Err(AccelError::InvalidDomain {
            bits: width,
            len: chunk_len,
        });
    }

    let current_word_count = current_states.len() / 8;
    if !current_word_count.is_multiple_of(width) {
        return Err(AccelError::InvalidDomain {
            bits: width,
            len: current_word_count,
        });
    }
    let row_count = current_word_count / width;
    let expected_row_bytes = row_count
        .checked_mul(chunk_len)
        .and_then(|word_count| word_count.checked_mul(8))
        .ok_or(AccelError::InvalidDomain {
            bits: width,
            len: current_word_count,
        })?;
    if row_values.len() != expected_row_bytes {
        return Err(AccelError::LengthMismatch {
            lhs: expected_row_bytes,
            rhs: row_values.len(),
        });
    }
    if row_count == 0 {
        return Ok(());
    }

    let code = unsafe {
        operation(
            current_states.as_raw_ptr() as *const u64,
            row_values.as_raw_ptr() as *const u64,
            out.as_raw_ptr() as *mut u64,
            row_count,
            chunk_len,
        )
    };
    cuda_status(code)
}

#[cfg(feature = "cuda")]
fn run_cuda_poseidon2_linear_round_row_major_device_op(
    current_states: &CudaDeviceBuffer,
    row_values: &CudaDeviceBuffer,
    out: &mut CudaDeviceBuffer,
    params: CudaLinearRoundRowMajorParams,
    operation: CudaPoseidon2LinearRoundRowMajorDeviceOp,
) -> Result<(), AccelError> {
    if !current_states.len().is_multiple_of(8) {
        return Err(AccelError::LengthMismatch {
            lhs: current_states.len(),
            rhs: current_states.len() / 8 * 8,
        });
    }
    if !row_values.len().is_multiple_of(8) {
        return Err(AccelError::LengthMismatch {
            lhs: row_values.len(),
            rhs: row_values.len() / 8 * 8,
        });
    }
    if current_states.len() != out.len() {
        return Err(AccelError::LengthMismatch {
            lhs: current_states.len(),
            rhs: out.len(),
        });
    }
    if params.chunk_len == 0 || params.chunk_len > params.rate {
        return Err(AccelError::InvalidDomain {
            bits: params.width,
            len: params.chunk_len,
        });
    }
    if params
        .offset
        .checked_add(params.chunk_len)
        .is_none_or(|end| end > params.column_count)
    {
        return Err(AccelError::InvalidDomain {
            bits: params.width,
            len: params.column_count,
        });
    }

    let current_word_count = current_states.len() / 8;
    if !current_word_count.is_multiple_of(params.width) {
        return Err(AccelError::InvalidDomain {
            bits: params.width,
            len: current_word_count,
        });
    }
    let row_count = current_word_count / params.width;
    let expected_row_bytes = row_count
        .checked_mul(params.column_count)
        .and_then(|word_count| word_count.checked_mul(8))
        .ok_or(AccelError::InvalidDomain {
            bits: params.width,
            len: current_word_count,
        })?;
    if row_values.len() != expected_row_bytes {
        return Err(AccelError::LengthMismatch {
            lhs: expected_row_bytes,
            rhs: row_values.len(),
        });
    }
    if row_count == 0 {
        return Ok(());
    }

    let code = unsafe {
        operation(
            current_states.as_raw_ptr() as *const u64,
            row_values.as_raw_ptr() as *const u64,
            out.as_raw_ptr() as *mut u64,
            row_count,
            params.column_count,
            params.offset,
            params.chunk_len,
        )
    };
    cuda_status(code)
}

#[cfg(feature = "cuda")]
pub fn cuda_poseidon2_width4_device(
    values: &CudaDeviceBuffer,
    out: &mut CudaDeviceBuffer,
) -> Result<(), AccelError> {
    run_cuda_poseidon2_device_op(values, out, 4, 2, lzvm_cuda_poseidon2_width4_device_raw)
}

#[cfg(feature = "cuda")]
pub fn cuda_poseidon2_width4_find_nonce(
    challenge: [u64; 3],
    start: u64,
    count: usize,
    target: u64,
) -> Result<Option<u64>, AccelError> {
    if count == 0 {
        return Ok(None);
    }
    let count_u64 = u64::try_from(count).map_err(|_| AccelError::InvalidDomain {
        bits: 2,
        len: count,
    })?;
    start
        .checked_add(count_u64 - 1)
        .ok_or(AccelError::InvalidDomain {
            bits: 2,
            len: count,
        })?;

    let mut out = 0_u64;
    let mut found = 0_u32;
    let code = unsafe {
        lzvm_cuda_poseidon2_width4_find_nonce(
            challenge.as_ptr(),
            start,
            count,
            target,
            &mut out,
            &mut found,
        )
    };
    if code == 0 {
        Ok((found != 0).then_some(out))
    } else {
        Err(AccelError::Cuda { code })
    }
}

#[cfg(feature = "cuda")]
pub fn cuda_poseidon2_width8(values: &[u64]) -> Result<Vec<u64>, AccelError> {
    const WIDTH: usize = 8;

    if !values.len().is_multiple_of(WIDTH) {
        return Err(AccelError::InvalidDomain {
            bits: 3,
            len: values.len(),
        });
    }
    let mut out = vec![0_u64; values.len()];
    let code = if values.is_empty() {
        0
    } else {
        unsafe {
            lzvm_cuda_poseidon2_width8(values.as_ptr(), out.as_mut_ptr(), values.len() / WIDTH)
        }
    };
    if code == 0 {
        Ok(out)
    } else {
        Err(AccelError::Cuda { code })
    }
}

#[cfg(feature = "cuda")]
pub fn cuda_poseidon2_width8_device(
    values: &CudaDeviceBuffer,
    out: &mut CudaDeviceBuffer,
) -> Result<(), AccelError> {
    run_cuda_poseidon2_device_op(values, out, 8, 3, lzvm_cuda_poseidon2_width8_device_raw)
}

#[cfg(feature = "cuda")]
pub fn cuda_poseidon2_width8_merkle_parent_device(
    values: &CudaDeviceBuffer,
    out: &mut CudaDeviceBuffer,
) -> Result<(), AccelError> {
    run_cuda_poseidon2_merkle_parent_device_op(
        values,
        out,
        8,
        2,
        3,
        lzvm_cuda_poseidon2_width8_merkle_parent_device_raw,
    )
}

#[cfg(feature = "cuda")]
pub fn cuda_poseidon2_width8_merkle_root_device(
    values: &CudaDeviceBuffer,
) -> Result<[u64; 4], AccelError> {
    run_cuda_poseidon2_merkle_root_device_op(
        values,
        8,
        3,
        lzvm_cuda_poseidon2_width8_merkle_root_device_raw,
    )
}

#[cfg(feature = "cuda")]
pub fn cuda_poseidon2_width16(values: &[u64]) -> Result<Vec<u64>, AccelError> {
    const WIDTH: usize = 16;

    if !values.len().is_multiple_of(WIDTH) {
        return Err(AccelError::InvalidDomain {
            bits: 4,
            len: values.len(),
        });
    }
    let mut out = vec![0_u64; values.len()];
    let code = if values.is_empty() {
        0
    } else {
        unsafe {
            lzvm_cuda_poseidon2_width16(values.as_ptr(), out.as_mut_ptr(), values.len() / WIDTH)
        }
    };
    if code == 0 {
        Ok(out)
    } else {
        Err(AccelError::Cuda { code })
    }
}

#[cfg(feature = "cuda")]
pub fn cuda_poseidon2_width16_device(
    values: &CudaDeviceBuffer,
    out: &mut CudaDeviceBuffer,
) -> Result<(), AccelError> {
    run_cuda_poseidon2_device_op(values, out, 16, 4, lzvm_cuda_poseidon2_width16_device_raw)
}

#[cfg(feature = "cuda")]
pub fn cuda_poseidon2_width16_merkle_parent_device(
    values: &CudaDeviceBuffer,
    out: &mut CudaDeviceBuffer,
) -> Result<(), AccelError> {
    run_cuda_poseidon2_merkle_parent_device_op(
        values,
        out,
        16,
        4,
        4,
        lzvm_cuda_poseidon2_width16_merkle_parent_device_raw,
    )
}

#[cfg(feature = "cuda")]
pub fn cuda_poseidon2_width16_merkle_root_device(
    values: &CudaDeviceBuffer,
) -> Result<[u64; 4], AccelError> {
    run_cuda_poseidon2_merkle_root_device_op(
        values,
        16,
        4,
        lzvm_cuda_poseidon2_width16_merkle_root_device_raw,
    )
}

#[cfg(feature = "cuda")]
pub fn cuda_poseidon2_width8_linear_round_device(
    current_states: &CudaDeviceBuffer,
    row_values: &CudaDeviceBuffer,
    out: &mut CudaDeviceBuffer,
    chunk_len: usize,
) -> Result<(), AccelError> {
    run_cuda_poseidon2_linear_round_device_op(
        current_states,
        row_values,
        out,
        8,
        4,
        chunk_len,
        lzvm_cuda_poseidon2_width8_linear_round_device_raw,
    )
}

#[cfg(feature = "cuda")]
pub fn cuda_poseidon2_width8_linear_round_row_major_device(
    current_states: &CudaDeviceBuffer,
    row_values: &CudaDeviceBuffer,
    out: &mut CudaDeviceBuffer,
    column_count: usize,
    offset: usize,
    chunk_len: usize,
) -> Result<(), AccelError> {
    run_cuda_poseidon2_linear_round_row_major_device_op(
        current_states,
        row_values,
        out,
        CudaLinearRoundRowMajorParams {
            width: 8,
            rate: 4,
            column_count,
            offset,
            chunk_len,
        },
        lzvm_cuda_poseidon2_width8_linear_round_row_major_device_raw,
    )
}

#[cfg(feature = "cuda")]
pub fn cuda_poseidon2_width16_linear_round_device(
    current_states: &CudaDeviceBuffer,
    row_values: &CudaDeviceBuffer,
    out: &mut CudaDeviceBuffer,
    chunk_len: usize,
) -> Result<(), AccelError> {
    run_cuda_poseidon2_linear_round_device_op(
        current_states,
        row_values,
        out,
        16,
        12,
        chunk_len,
        lzvm_cuda_poseidon2_width16_linear_round_device_raw,
    )
}

#[cfg(feature = "cuda")]
pub fn cuda_poseidon2_width16_linear_round_row_major_device(
    current_states: &CudaDeviceBuffer,
    row_values: &CudaDeviceBuffer,
    out: &mut CudaDeviceBuffer,
    column_count: usize,
    offset: usize,
    chunk_len: usize,
) -> Result<(), AccelError> {
    run_cuda_poseidon2_linear_round_row_major_device_op(
        current_states,
        row_values,
        out,
        CudaLinearRoundRowMajorParams {
            width: 16,
            rate: 12,
            column_count,
            offset,
            chunk_len,
        },
        lzvm_cuda_poseidon2_width16_linear_round_row_major_device_raw,
    )
}

#[cfg(feature = "cuda")]
pub fn cuda_keccak256_fixed(input: &[u8], message_len: usize) -> Result<Vec<[u8; 32]>, AccelError> {
    if message_len == 0 || !input.len().is_multiple_of(message_len) {
        return Err(AccelError::InvalidDomain {
            bits: 0,
            len: input.len(),
        });
    }

    let message_count = input.len() / message_len;
    let output_len = message_count
        .checked_mul(32)
        .ok_or(AccelError::InvalidDomain {
            bits: 0,
            len: input.len(),
        })?;
    let mut out = vec![0_u8; output_len];
    let code = if message_count == 0 {
        0
    } else {
        unsafe {
            lzvm_cuda_keccak256_fixed(input.as_ptr(), message_len, out.as_mut_ptr(), message_count)
        }
    };
    if code == 0 {
        Ok(out
            .chunks_exact(32)
            .map(|chunk| {
                let mut digest = [0_u8; 32];
                digest.copy_from_slice(chunk);
                digest
            })
            .collect())
    } else {
        Err(AccelError::Cuda { code })
    }
}

#[cfg(not(feature = "cuda"))]
pub fn cuda_setup_init(_max_bits_ext: usize) -> Result<(), AccelError> {
    Err(AccelError::CudaUnavailable)
}

#[cfg(not(feature = "cuda"))]
pub fn cuda_goldilocks_add(_lhs: &[u64], _rhs: &[u64]) -> Result<Vec<u64>, AccelError> {
    Err(AccelError::CudaUnavailable)
}

#[cfg(not(feature = "cuda"))]
pub fn cuda_goldilocks_mul(_lhs: &[u64], _rhs: &[u64]) -> Result<Vec<u64>, AccelError> {
    Err(AccelError::CudaUnavailable)
}

#[cfg(not(feature = "cuda"))]
pub fn cuda_goldilocks_butterfly(
    _even: &[u64],
    _odd: &[u64],
    _twiddle: &[u64],
) -> Result<(Vec<u64>, Vec<u64>), AccelError> {
    Err(AccelError::CudaUnavailable)
}

#[cfg(not(feature = "cuda"))]
pub fn cuda_goldilocks_ntt(_values: &[u64], _bits: usize) -> Result<Vec<u64>, AccelError> {
    Err(AccelError::CudaUnavailable)
}

#[cfg(not(feature = "cuda"))]
pub fn cuda_goldilocks_intt(_values: &[u64], _bits: usize) -> Result<Vec<u64>, AccelError> {
    Err(AccelError::CudaUnavailable)
}

#[cfg(not(feature = "cuda"))]
pub fn cuda_goldilocks_coset_extend(
    _values: &[u64],
    _source_bits: usize,
    _target_bits: usize,
) -> Result<Vec<u64>, AccelError> {
    Err(AccelError::CudaUnavailable)
}

#[cfg(not(feature = "cuda"))]
pub fn cuda_poseidon2_width4(_values: &[u64]) -> Result<Vec<u64>, AccelError> {
    Err(AccelError::CudaUnavailable)
}

#[cfg(not(feature = "cuda"))]
pub fn cuda_poseidon2_width4_find_nonce(
    _challenge: [u64; 3],
    _start: u64,
    _count: usize,
    _target: u64,
) -> Result<Option<u64>, AccelError> {
    Err(AccelError::CudaUnavailable)
}

#[cfg(not(feature = "cuda"))]
pub fn cuda_poseidon2_width8(_values: &[u64]) -> Result<Vec<u64>, AccelError> {
    Err(AccelError::CudaUnavailable)
}

#[cfg(not(feature = "cuda"))]
pub fn cuda_poseidon2_width16(_values: &[u64]) -> Result<Vec<u64>, AccelError> {
    Err(AccelError::CudaUnavailable)
}

#[cfg(not(feature = "cuda"))]
pub fn cuda_keccak256_fixed(
    _input: &[u8],
    _message_len: usize,
) -> Result<Vec<[u8; 32]>, AccelError> {
    Err(AccelError::CudaUnavailable)
}
