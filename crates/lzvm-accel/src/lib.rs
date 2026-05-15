use std::fmt;

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
