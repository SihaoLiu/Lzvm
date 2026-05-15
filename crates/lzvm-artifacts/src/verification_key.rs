use std::fmt;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerificationKeyRoot {
    FieldElements(Vec<u64>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerificationKeyError {
    InvalidBinaryLength { expected: usize, found: usize },
    FieldElementCountMismatch { expected: usize, found: usize },
    Io { message: String },
}

impl fmt::Display for VerificationKeyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidBinaryLength { expected, found } => write!(
                f,
                "invalid verification-key binary length: expected {expected}, found {found}"
            ),
            Self::FieldElementCountMismatch { expected, found } => write!(
                f,
                "verification-key field element count mismatch: expected {expected}, found {found}"
            ),
            Self::Io { message } => write!(f, "verification-key io error: {message}"),
        }
    }
}

impl std::error::Error for VerificationKeyError {}

pub fn parse_verification_key_binary(
    bytes: &[u8],
) -> Result<VerificationKeyRoot, VerificationKeyError> {
    const ROOT_ELEMENTS: usize = 4;
    const ROOT_BYTES: usize = ROOT_ELEMENTS * 8;

    if bytes.len() != ROOT_BYTES {
        return Err(VerificationKeyError::InvalidBinaryLength {
            expected: ROOT_BYTES,
            found: bytes.len(),
        });
    }

    let mut values = Vec::with_capacity(ROOT_ELEMENTS);
    for chunk in bytes.chunks_exact(8) {
        values.push(u64::from_le_bytes(
            chunk.try_into().expect("slice length checked"),
        ));
    }

    Ok(VerificationKeyRoot::FieldElements(values))
}

pub fn read_verification_key_binary_file(
    path: impl AsRef<Path>,
) -> Result<VerificationKeyRoot, VerificationKeyError> {
    let input = std::fs::read(path).map_err(|error| VerificationKeyError::Io {
        message: error.to_string(),
    })?;
    parse_verification_key_binary(&input)
}

pub fn encode_verification_key_binary(
    root: &VerificationKeyRoot,
) -> Result<Vec<u8>, VerificationKeyError> {
    const ROOT_ELEMENTS: usize = 4;

    let VerificationKeyRoot::FieldElements(values) = root;

    if values.len() != ROOT_ELEMENTS {
        return Err(VerificationKeyError::FieldElementCountMismatch {
            expected: ROOT_ELEMENTS,
            found: values.len(),
        });
    }

    let mut out = Vec::with_capacity(ROOT_ELEMENTS * 8);
    for value in values {
        out.extend_from_slice(&value.to_le_bytes());
    }
    Ok(out)
}
