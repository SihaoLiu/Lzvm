use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerificationKeyRoot {
    FieldElements(Vec<u64>),
    DecimalScalar(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerificationKeyError {
    Json { message: String },
    UnsupportedJsonShape,
    InvalidBinaryLength { expected: usize, found: usize },
    ScalarHasNoBinaryEncoding,
    FieldElementCountMismatch { expected: usize, found: usize },
}

impl fmt::Display for VerificationKeyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json { message } => write!(f, "verification-key json error: {message}"),
            Self::UnsupportedJsonShape => write!(f, "unsupported verification-key json shape"),
            Self::InvalidBinaryLength { expected, found } => write!(
                f,
                "invalid verification-key binary length: expected {expected}, found {found}"
            ),
            Self::ScalarHasNoBinaryEncoding => {
                write!(
                    f,
                    "scalar verification-key roots have no fixed binary encoding"
                )
            }
            Self::FieldElementCountMismatch { expected, found } => write!(
                f,
                "verification-key field element count mismatch: expected {expected}, found {found}"
            ),
        }
    }
}

impl std::error::Error for VerificationKeyError {}

pub fn parse_verification_key_json(
    input: &str,
) -> Result<VerificationKeyRoot, VerificationKeyError> {
    let value: serde_json::Value =
        serde_json::from_str(input).map_err(|error| VerificationKeyError::Json {
            message: error.to_string(),
        })?;

    match value {
        serde_json::Value::Array(values) => {
            let mut out = Vec::with_capacity(values.len());
            for value in values {
                let Some(number) = value.as_u64() else {
                    return Err(VerificationKeyError::UnsupportedJsonShape);
                };
                out.push(number);
            }
            Ok(VerificationKeyRoot::FieldElements(out))
        }
        serde_json::Value::String(value) => Ok(VerificationKeyRoot::DecimalScalar(value)),
        _ => Err(VerificationKeyError::UnsupportedJsonShape),
    }
}

pub fn encode_verification_key_json(
    root: &VerificationKeyRoot,
) -> Result<String, VerificationKeyError> {
    match root {
        VerificationKeyRoot::FieldElements(values) => {
            serde_json::to_string(values).map_err(|error| VerificationKeyError::Json {
                message: error.to_string(),
            })
        }
        VerificationKeyRoot::DecimalScalar(value) => {
            serde_json::to_string(value).map_err(|error| VerificationKeyError::Json {
                message: error.to_string(),
            })
        }
    }
}

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

pub fn encode_verification_key_binary(
    root: &VerificationKeyRoot,
) -> Result<Vec<u8>, VerificationKeyError> {
    const ROOT_ELEMENTS: usize = 4;

    let VerificationKeyRoot::FieldElements(values) = root else {
        return Err(VerificationKeyError::ScalarHasNoBinaryEncoding);
    };

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
