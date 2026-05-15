use std::collections::BTreeSet;
use std::fmt;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicValues {
    pub schema_version: u32,
    pub setup_hash: [u8; 32],
    pub values: Vec<PublicValueEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicValueEntry {
    pub name: String,
    pub elements: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PublicValuesError {
    Json { message: String },
    MissingField { field: &'static str },
    InvalidField { field: &'static str },
    InvalidHash { field: &'static str },
    EmptyName { index: usize },
    DuplicateName { name: String },
    EmptyValue { name: String },
    InvalidElement { name: String },
    Io { message: String },
}

impl fmt::Display for PublicValuesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json { message } => write!(f, "public-values json error: {message}"),
            Self::MissingField { field } => write!(f, "missing public-values field: {field}"),
            Self::InvalidField { field } => write!(f, "invalid public-values field: {field}"),
            Self::InvalidHash { field } => write!(f, "invalid public-values hash: {field}"),
            Self::EmptyName { index } => {
                write!(f, "empty public-values name at index {index}")
            }
            Self::DuplicateName { name } => {
                write!(f, "duplicate public-values name: {name}")
            }
            Self::EmptyValue { name } => write!(f, "empty public-values entry: {name}"),
            Self::InvalidElement { name } => {
                write!(f, "invalid public-values element for {name}")
            }
            Self::Io { message } => write!(f, "public-values io error: {message}"),
        }
    }
}

impl std::error::Error for PublicValuesError {}

pub fn read_public_values_file(path: impl AsRef<Path>) -> Result<PublicValues, PublicValuesError> {
    let input = std::fs::read_to_string(path).map_err(|error| PublicValuesError::Io {
        message: error.to_string(),
    })?;
    parse_public_values_json(&input)
}

pub fn parse_public_values_json(input: &str) -> Result<PublicValues, PublicValuesError> {
    let value: serde_json::Value =
        serde_json::from_str(input).map_err(|error| PublicValuesError::Json {
            message: error.to_string(),
        })?;
    let object = value
        .as_object()
        .ok_or(PublicValuesError::InvalidField { field: "root" })?;
    let schema_version = read_u32(object, "schema_version")?;
    let setup_hash = read_hash(object, "setup_hash")?;
    let values = object
        .get("values")
        .and_then(serde_json::Value::as_array)
        .ok_or(PublicValuesError::MissingField { field: "values" })?;

    let mut entries = Vec::with_capacity(values.len());
    for (index, value) in values.iter().enumerate() {
        let object = value
            .as_object()
            .ok_or(PublicValuesError::InvalidField { field: "values" })?;
        let name = object
            .get("name")
            .and_then(serde_json::Value::as_str)
            .ok_or(PublicValuesError::MissingField { field: "name" })?
            .to_owned();
        if name.is_empty() {
            return Err(PublicValuesError::EmptyName { index });
        }
        let element_values = object
            .get("elements")
            .and_then(serde_json::Value::as_array)
            .ok_or(PublicValuesError::MissingField { field: "elements" })?;
        if element_values.is_empty() {
            return Err(PublicValuesError::EmptyValue { name });
        }
        let mut elements = Vec::with_capacity(element_values.len());
        for element in element_values {
            let Some(text) = element.as_str() else {
                return Err(PublicValuesError::InvalidElement { name });
            };
            elements.push(
                text.parse::<u64>()
                    .map_err(|_| PublicValuesError::InvalidElement { name: name.clone() })?,
            );
        }
        entries.push(PublicValueEntry { name, elements });
    }

    let out = PublicValues {
        schema_version,
        setup_hash,
        values: entries,
    };
    validate_public_values(&out)?;
    Ok(out)
}

pub fn encode_public_values_json(value: &PublicValues) -> Result<String, PublicValuesError> {
    validate_public_values(value)?;

    let mut out = String::new();
    out.push('{');
    out.push_str("\"schema_version\":");
    out.push_str(&value.schema_version.to_string());
    out.push_str(",\"setup_hash\":");
    out.push_str(
        &serde_json::to_string(&encode_hash(&value.setup_hash)).map_err(|error| {
            PublicValuesError::Json {
                message: error.to_string(),
            }
        })?,
    );
    out.push_str(",\"values\":[");
    for (index, entry) in value.values.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        out.push_str("\"name\":");
        out.push_str(&serde_json::to_string(&entry.name).map_err(|error| {
            PublicValuesError::Json {
                message: error.to_string(),
            }
        })?);
        out.push_str(",\"elements\":[");
        for (element_index, element) in entry.elements.iter().enumerate() {
            if element_index > 0 {
                out.push(',');
            }
            out.push_str(
                &serde_json::to_string(&element.to_string()).map_err(|error| {
                    PublicValuesError::Json {
                        message: error.to_string(),
                    }
                })?,
            );
        }
        out.push_str("]}");
    }
    out.push_str("]}");
    Ok(out)
}

fn validate_public_values(value: &PublicValues) -> Result<(), PublicValuesError> {
    let mut names = BTreeSet::new();
    for (index, entry) in value.values.iter().enumerate() {
        if entry.name.is_empty() {
            return Err(PublicValuesError::EmptyName { index });
        }
        if !names.insert(entry.name.clone()) {
            return Err(PublicValuesError::DuplicateName {
                name: entry.name.clone(),
            });
        }
        if entry.elements.is_empty() {
            return Err(PublicValuesError::EmptyValue {
                name: entry.name.clone(),
            });
        }
    }
    Ok(())
}

fn read_u32(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &'static str,
) -> Result<u32, PublicValuesError> {
    let value = object
        .get(field)
        .and_then(serde_json::Value::as_u64)
        .ok_or(PublicValuesError::MissingField { field })?;
    u32::try_from(value).map_err(|_| PublicValuesError::InvalidField { field })
}

fn read_hash(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &'static str,
) -> Result<[u8; 32], PublicValuesError> {
    let text = object
        .get(field)
        .and_then(serde_json::Value::as_str)
        .ok_or(PublicValuesError::MissingField { field })?;
    decode_hash(text).ok_or(PublicValuesError::InvalidHash { field })
}

fn encode_hash(value: &[u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(64);
    for byte in value {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn decode_hash(value: &str) -> Option<[u8; 32]> {
    if value.len() != 64 {
        return None;
    }
    let mut out = [0_u8; 32];
    for (index, chunk) in value.as_bytes().chunks_exact(2).enumerate() {
        out[index] = (hex_value(chunk[0])? << 4) | hex_value(chunk[1])?;
    }
    Some(out)
}

fn hex_value(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}
