use std::fmt;
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub struct VerifierInfo {
    pub quotient: VerifierCode,
    pub query: VerifierCode,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VerifierCode {
    pub expression_id: Option<u32>,
    pub stage: Option<u32>,
    pub line: String,
    pub temporary_count: u32,
    pub operations: Vec<VerifierOperation>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VerifierOperation {
    pub op: VerifierOperationKind,
    pub destination: serde_json::Value,
    pub sources: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerifierOperationKind {
    Add,
    Sub,
    Mul,
    Copy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerifierInfoError {
    Json {
        message: String,
    },
    MissingField {
        field: &'static str,
    },
    InvalidField {
        field: &'static str,
    },
    UnknownOperation {
        op: String,
    },
    TemporaryReferenceOutOfBounds {
        temporary_id: u32,
        temporary_count: u32,
    },
    EmptyCodeBlock {
        field: &'static str,
    },
    Io {
        message: String,
    },
}

impl fmt::Display for VerifierInfoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json { message } => write!(f, "verifier-info json error: {message}"),
            Self::MissingField { field } => write!(f, "missing verifier-info field: {field}"),
            Self::InvalidField { field } => write!(f, "invalid verifier-info field: {field}"),
            Self::UnknownOperation { op } => write!(f, "unknown verifier-info operation: {op}"),
            Self::TemporaryReferenceOutOfBounds {
                temporary_id,
                temporary_count,
            } => write!(
                f,
                "temporary reference {temporary_id} is out of bounds for count {temporary_count}"
            ),
            Self::EmptyCodeBlock { field } => write!(f, "empty verifier-info code block: {field}"),
            Self::Io { message } => write!(f, "verifier-info io error: {message}"),
        }
    }
}

impl std::error::Error for VerifierInfoError {}

impl VerifierCode {
    pub fn operation_count(&self) -> usize {
        self.operations.len()
    }
}

pub fn read_verifier_info_file(path: impl AsRef<Path>) -> Result<VerifierInfo, VerifierInfoError> {
    let input = std::fs::read_to_string(path).map_err(|error| VerifierInfoError::Io {
        message: error.to_string(),
    })?;
    parse_verifier_info_json(&input)
}

pub fn parse_verifier_info_json(input: &str) -> Result<VerifierInfo, VerifierInfoError> {
    let value: serde_json::Value =
        serde_json::from_str(input).map_err(|error| VerifierInfoError::Json {
            message: error.to_string(),
        })?;
    let object = as_object(&value, "$")?;

    Ok(VerifierInfo {
        quotient: parse_verifier_code(required(object, "qVerifier")?, "qVerifier")?,
        query: parse_verifier_code(required(object, "queryVerifier")?, "queryVerifier")?,
    })
}

fn parse_verifier_code(
    value: &serde_json::Value,
    field: &'static str,
) -> Result<VerifierCode, VerifierInfoError> {
    let object = as_object(value, field)?;
    let temporary_count = required_u32(object, "tmpUsed")?;
    let operations = parse_operations(required_array(object, "code")?, temporary_count)?;
    if operations.is_empty() {
        return Err(VerifierInfoError::EmptyCodeBlock { field });
    }

    Ok(VerifierCode {
        expression_id: optional_u32(object, "expId")?,
        stage: optional_u32(object, "stage")?,
        line: optional_string(object, "line")?.unwrap_or_default(),
        temporary_count,
        operations,
    })
}

fn parse_operations(
    values: &[serde_json::Value],
    temporary_count: u32,
) -> Result<Vec<VerifierOperation>, VerifierInfoError> {
    let mut operations = Vec::with_capacity(values.len());
    for value in values {
        let object = as_object(value, "code")?;
        let op = parse_operation(&required_string(object, "op")?)?;
        let destination = required(object, "dest")?.clone();
        let sources = required_array(object, "src")?.to_vec();
        validate_temporary_reference(&destination, temporary_count)?;
        for source in &sources {
            validate_temporary_reference(source, temporary_count)?;
        }
        operations.push(VerifierOperation {
            op,
            destination,
            sources,
        });
    }
    Ok(operations)
}

fn parse_operation(op: &str) -> Result<VerifierOperationKind, VerifierInfoError> {
    match op {
        "add" => Ok(VerifierOperationKind::Add),
        "sub" => Ok(VerifierOperationKind::Sub),
        "mul" => Ok(VerifierOperationKind::Mul),
        "copy" => Ok(VerifierOperationKind::Copy),
        _ => Err(VerifierInfoError::UnknownOperation { op: op.to_owned() }),
    }
}

fn validate_temporary_reference(
    value: &serde_json::Value,
    temporary_count: u32,
) -> Result<(), VerifierInfoError> {
    let Some(object) = value.as_object() else {
        return Err(VerifierInfoError::InvalidField { field: "reference" });
    };
    if object.get("type").and_then(serde_json::Value::as_str) != Some("tmp") {
        return Ok(());
    }
    let id = value_to_u32(
        object
            .get("id")
            .ok_or(VerifierInfoError::MissingField { field: "id" })?,
        "id",
    )?;
    if id >= temporary_count {
        return Err(VerifierInfoError::TemporaryReferenceOutOfBounds {
            temporary_id: id,
            temporary_count,
        });
    }
    Ok(())
}

fn as_object<'a>(
    value: &'a serde_json::Value,
    field: &'static str,
) -> Result<&'a serde_json::Map<String, serde_json::Value>, VerifierInfoError> {
    value
        .as_object()
        .ok_or(VerifierInfoError::InvalidField { field })
}

fn required<'a>(
    object: &'a serde_json::Map<String, serde_json::Value>,
    field: &'static str,
) -> Result<&'a serde_json::Value, VerifierInfoError> {
    object
        .get(field)
        .ok_or(VerifierInfoError::MissingField { field })
}

fn required_array<'a>(
    object: &'a serde_json::Map<String, serde_json::Value>,
    field: &'static str,
) -> Result<&'a Vec<serde_json::Value>, VerifierInfoError> {
    required(object, field)?
        .as_array()
        .ok_or(VerifierInfoError::InvalidField { field })
}

fn required_string(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &'static str,
) -> Result<String, VerifierInfoError> {
    required(object, field)?
        .as_str()
        .map(str::to_owned)
        .ok_or(VerifierInfoError::InvalidField { field })
}

fn required_u32(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &'static str,
) -> Result<u32, VerifierInfoError> {
    value_to_u32(required(object, field)?, field)
}

fn optional_u32(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &'static str,
) -> Result<Option<u32>, VerifierInfoError> {
    object
        .get(field)
        .map(|value| value_to_u32(value, field))
        .transpose()
}

fn value_to_u32(value: &serde_json::Value, field: &'static str) -> Result<u32, VerifierInfoError> {
    let Some(number) = value.as_u64() else {
        return Err(VerifierInfoError::InvalidField { field });
    };
    u32::try_from(number).map_err(|_| VerifierInfoError::InvalidField { field })
}

fn optional_string(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &'static str,
) -> Result<Option<String>, VerifierInfoError> {
    object
        .get(field)
        .map(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or(VerifierInfoError::InvalidField { field })
        })
        .transpose()
}
