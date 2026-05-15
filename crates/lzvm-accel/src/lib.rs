use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccelError {
    LengthMismatch { lhs: usize, rhs: usize },
    CudaUnavailable,
    Cuda { code: i32 },
}

impl fmt::Display for AccelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LengthMismatch { lhs, rhs } => {
                write!(f, "length mismatch: lhs {lhs}, rhs {rhs}")
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
