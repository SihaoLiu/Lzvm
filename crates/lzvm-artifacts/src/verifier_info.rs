use std::fmt;
use std::path::Path;

use crate::sectioned::{
    encode_sectioned_file, parse_sectioned_file, SectionedError, SectionedFile, SectionedSection,
};

const VERIFIER_INFO_KIND: [u8; 4] = *b"vinf";
const VERIFIER_INFO_VERSION: u32 = 1;
const VERIFIER_INFO_SECTION_ID: u32 = 1;

const JSON_NULL_TAG: u8 = 0;
const JSON_BOOL_TAG: u8 = 1;
const JSON_U64_TAG: u8 = 2;
const JSON_I64_TAG: u8 = 3;
const JSON_F64_TAG: u8 = 4;
const JSON_STRING_TAG: u8 = 5;
const JSON_ARRAY_TAG: u8 = 6;
const JSON_OBJECT_TAG: u8 = 7;

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
    InvalidMagic,
    UnsupportedVersion {
        found: u32,
        max: u32,
    },
    InvalidSectionCount {
        found: u32,
    },
    InvalidSectionId {
        found: u32,
    },
    UnexpectedTrailingBytes {
        count: usize,
    },
    UnexpectedEof {
        offset: usize,
        needed: usize,
        available: usize,
    },
    LengthOverflow,
    InvalidUtf8,
    InvalidFlag {
        field: &'static str,
        value: u8,
    },
    InvalidOperationTag {
        value: u8,
    },
    InvalidJsonTag {
        value: u8,
    },
    InvalidJsonNumber,
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
            Self::InvalidMagic => write!(f, "invalid verifier-info file magic"),
            Self::UnsupportedVersion { found, max } => {
                write!(f, "unsupported verifier-info file version {found}, max {max}")
            }
            Self::InvalidSectionCount { found } => {
                write!(f, "invalid verifier-info section count {found}")
            }
            Self::InvalidSectionId { found } => {
                write!(f, "invalid verifier-info section id {found}")
            }
            Self::UnexpectedTrailingBytes { count } => {
                write!(f, "unexpected trailing bytes in verifier-info file: {count}")
            }
            Self::UnexpectedEof {
                offset,
                needed,
                available,
            } => write!(
                f,
                "unexpected end of verifier-info file at {offset}, needed {needed}, available {available}"
            ),
            Self::LengthOverflow => write!(f, "verifier-info length overflow"),
            Self::InvalidUtf8 => write!(f, "verifier-info string is not valid utf-8"),
            Self::InvalidFlag { field, value } => {
                write!(f, "invalid verifier-info flag for {field}: {value}")
            }
            Self::InvalidOperationTag { value } => {
                write!(f, "invalid verifier-info operation tag: {value}")
            }
            Self::InvalidJsonTag { value } => {
                write!(f, "invalid verifier-info value tag: {value}")
            }
            Self::InvalidJsonNumber => write!(f, "invalid verifier-info number value"),
            Self::Io { message } => write!(f, "verifier-info io error: {message}"),
        }
    }
}

impl std::error::Error for VerifierInfoError {}

impl From<SectionedError> for VerifierInfoError {
    fn from(value: SectionedError) -> Self {
        match value {
            SectionedError::InvalidKind { .. } => Self::InvalidMagic,
            SectionedError::UnsupportedVersion { found, max } => {
                Self::UnsupportedVersion { found, max }
            }
            SectionedError::UnexpectedTrailingBytes { count } => {
                Self::UnexpectedTrailingBytes { count }
            }
            SectionedError::UnexpectedEof {
                offset,
                needed,
                available,
            } => Self::UnexpectedEof {
                offset,
                needed,
                available,
            },
            SectionedError::LengthOverflow => Self::LengthOverflow,
        }
    }
}

impl VerifierCode {
    pub fn operation_count(&self) -> usize {
        self.operations.len()
    }
}

pub fn read_verifier_info_file(path: impl AsRef<Path>) -> Result<VerifierInfo, VerifierInfoError> {
    read_verifier_info_binary_file(path)
}

pub fn read_verifier_info_binary_file(
    path: impl AsRef<Path>,
) -> Result<VerifierInfo, VerifierInfoError> {
    let bytes = std::fs::read(path).map_err(|error| VerifierInfoError::Io {
        message: error.to_string(),
    })?;
    parse_verifier_info(&bytes)
}

pub fn parse_verifier_info(bytes: &[u8]) -> Result<VerifierInfo, VerifierInfoError> {
    let file = parse_sectioned_file(bytes, VERIFIER_INFO_KIND, VERIFIER_INFO_VERSION)
        .map_err(VerifierInfoError::from)?;
    if file.sections.len() != 1 {
        return Err(VerifierInfoError::InvalidSectionCount {
            found: u32::try_from(file.sections.len()).unwrap_or(u32::MAX),
        });
    }
    let section = &file.sections[0];
    if section.id != VERIFIER_INFO_SECTION_ID {
        return Err(VerifierInfoError::InvalidSectionId { found: section.id });
    }
    parse_verifier_info_section(&section.data)
}

pub fn encode_verifier_info(value: &VerifierInfo) -> Result<Vec<u8>, VerifierInfoError> {
    validate_verifier_info(value)?;
    let section = encode_verifier_info_section(value)?;
    let file = SectionedFile {
        kind: VERIFIER_INFO_KIND,
        version: VERIFIER_INFO_VERSION,
        sections: vec![SectionedSection {
            id: VERIFIER_INFO_SECTION_ID,
            data: section,
        }],
    };
    encode_sectioned_file(&file).map_err(VerifierInfoError::from)
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

fn parse_verifier_info_section(bytes: &[u8]) -> Result<VerifierInfo, VerifierInfoError> {
    let mut reader = Reader::new(bytes);
    let value = VerifierInfo {
        quotient: read_verifier_code(&mut reader)?,
        query: read_verifier_code(&mut reader)?,
    };
    if reader.position() != bytes.len() {
        return Err(VerifierInfoError::UnexpectedTrailingBytes {
            count: bytes.len() - reader.position(),
        });
    }
    validate_verifier_info(&value)?;
    Ok(value)
}

fn encode_verifier_info_section(value: &VerifierInfo) -> Result<Vec<u8>, VerifierInfoError> {
    let mut out = Vec::new();
    write_verifier_code(&mut out, &value.quotient)?;
    write_verifier_code(&mut out, &value.query)?;
    Ok(out)
}

fn validate_verifier_info(value: &VerifierInfo) -> Result<(), VerifierInfoError> {
    validate_verifier_code(&value.quotient, "qVerifier")?;
    validate_verifier_code(&value.query, "queryVerifier")
}

fn validate_verifier_code(
    value: &VerifierCode,
    field: &'static str,
) -> Result<(), VerifierInfoError> {
    if value.operations.is_empty() {
        return Err(VerifierInfoError::EmptyCodeBlock { field });
    }
    for operation in &value.operations {
        validate_temporary_reference(&operation.destination, value.temporary_count)?;
        for source in &operation.sources {
            validate_temporary_reference(source, value.temporary_count)?;
        }
    }
    Ok(())
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

fn read_verifier_code(reader: &mut Reader<'_>) -> Result<VerifierCode, VerifierInfoError> {
    let expression_id = reader.read_optional_u32("expression_id")?;
    let stage = reader.read_optional_u32("stage")?;
    let line = reader.read_string()?;
    let temporary_count = reader.read_u32()?;
    let operation_count = reader.read_u32()?;
    let mut operations = Vec::with_capacity(operation_count as usize);
    for _ in 0..operation_count {
        operations.push(read_verifier_operation(reader)?);
    }
    Ok(VerifierCode {
        expression_id,
        stage,
        line,
        temporary_count,
        operations,
    })
}

fn write_verifier_code(out: &mut Vec<u8>, value: &VerifierCode) -> Result<(), VerifierInfoError> {
    write_optional_u32(out, value.expression_id);
    write_optional_u32(out, value.stage);
    write_string(out, &value.line)?;
    write_u32(out, value.temporary_count);
    write_len(out, value.operations.len())?;
    for operation in &value.operations {
        write_verifier_operation(out, operation)?;
    }
    Ok(())
}

fn read_verifier_operation(
    reader: &mut Reader<'_>,
) -> Result<VerifierOperation, VerifierInfoError> {
    let op = read_operation_tag(reader.read_u8()?)?;
    let destination = reader.read_json_value()?;
    let source_count = reader.read_u32()?;
    let mut sources = Vec::with_capacity(source_count as usize);
    for _ in 0..source_count {
        sources.push(reader.read_json_value()?);
    }
    Ok(VerifierOperation {
        op,
        destination,
        sources,
    })
}

fn write_verifier_operation(
    out: &mut Vec<u8>,
    value: &VerifierOperation,
) -> Result<(), VerifierInfoError> {
    out.push(operation_tag(value.op));
    write_json_value(out, &value.destination)?;
    write_len(out, value.sources.len())?;
    for source in &value.sources {
        write_json_value(out, source)?;
    }
    Ok(())
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

fn operation_tag(value: VerifierOperationKind) -> u8 {
    match value {
        VerifierOperationKind::Add => 1,
        VerifierOperationKind::Sub => 2,
        VerifierOperationKind::Mul => 3,
        VerifierOperationKind::Copy => 4,
    }
}

fn read_operation_tag(value: u8) -> Result<VerifierOperationKind, VerifierInfoError> {
    match value {
        1 => Ok(VerifierOperationKind::Add),
        2 => Ok(VerifierOperationKind::Sub),
        3 => Ok(VerifierOperationKind::Mul),
        4 => Ok(VerifierOperationKind::Copy),
        _ => Err(VerifierInfoError::InvalidOperationTag { value }),
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

fn write_json_value(out: &mut Vec<u8>, value: &serde_json::Value) -> Result<(), VerifierInfoError> {
    match value {
        serde_json::Value::Null => out.push(JSON_NULL_TAG),
        serde_json::Value::Bool(value) => {
            out.push(JSON_BOOL_TAG);
            out.push(u8::from(*value));
        }
        serde_json::Value::Number(value) => {
            if let Some(value) = value.as_u64() {
                out.push(JSON_U64_TAG);
                write_u64(out, value);
            } else if let Some(value) = value.as_i64() {
                out.push(JSON_I64_TAG);
                write_i64(out, value);
            } else if let Some(value) = value.as_f64() {
                out.push(JSON_F64_TAG);
                out.extend_from_slice(&value.to_le_bytes());
            } else {
                return Err(VerifierInfoError::InvalidJsonNumber);
            }
        }
        serde_json::Value::String(value) => {
            out.push(JSON_STRING_TAG);
            write_string(out, value)?;
        }
        serde_json::Value::Array(values) => {
            out.push(JSON_ARRAY_TAG);
            write_len(out, values.len())?;
            for value in values {
                write_json_value(out, value)?;
            }
        }
        serde_json::Value::Object(values) => {
            out.push(JSON_OBJECT_TAG);
            write_len(out, values.len())?;
            for (key, value) in values {
                write_string(out, key)?;
                write_json_value(out, value)?;
            }
        }
    }
    Ok(())
}

fn write_optional_u32(out: &mut Vec<u8>, value: Option<u32>) {
    match value {
        Some(value) => {
            out.push(1);
            write_u32(out, value);
        }
        None => out.push(0),
    }
}

fn write_string(out: &mut Vec<u8>, value: &str) -> Result<(), VerifierInfoError> {
    write_len(out, value.len())?;
    out.extend_from_slice(value.as_bytes());
    Ok(())
}

fn write_len(out: &mut Vec<u8>, value: usize) -> Result<(), VerifierInfoError> {
    let value = u32::try_from(value).map_err(|_| VerifierInfoError::LengthOverflow)?;
    write_u32(out, value);
    Ok(())
}

fn write_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_i64(out: &mut Vec<u8>, value: i64) {
    out.extend_from_slice(&value.to_le_bytes());
}

struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn position(&self) -> usize {
        self.offset
    }

    fn read_json_value(&mut self) -> Result<serde_json::Value, VerifierInfoError> {
        let tag = self.read_u8()?;
        match tag {
            JSON_NULL_TAG => Ok(serde_json::Value::Null),
            JSON_BOOL_TAG => match self.read_u8()? {
                0 => Ok(serde_json::Value::Bool(false)),
                1 => Ok(serde_json::Value::Bool(true)),
                value => Err(VerifierInfoError::InvalidFlag {
                    field: "json_bool",
                    value,
                }),
            },
            JSON_U64_TAG => Ok(serde_json::Value::Number(self.read_u64()?.into())),
            JSON_I64_TAG => Ok(serde_json::Value::Number(self.read_i64()?.into())),
            JSON_F64_TAG => {
                let value = self.read_f64()?;
                let number = serde_json::Number::from_f64(value)
                    .ok_or(VerifierInfoError::InvalidJsonNumber)?;
                Ok(serde_json::Value::Number(number))
            }
            JSON_STRING_TAG => Ok(serde_json::Value::String(self.read_string()?)),
            JSON_ARRAY_TAG => {
                let count = self.read_u32()?;
                let mut values = Vec::with_capacity(count as usize);
                for _ in 0..count {
                    values.push(self.read_json_value()?);
                }
                Ok(serde_json::Value::Array(values))
            }
            JSON_OBJECT_TAG => {
                let count = self.read_u32()?;
                let mut values = serde_json::Map::new();
                for _ in 0..count {
                    let key = self.read_string()?;
                    let value = self.read_json_value()?;
                    values.insert(key, value);
                }
                Ok(serde_json::Value::Object(values))
            }
            value => Err(VerifierInfoError::InvalidJsonTag { value }),
        }
    }

    fn read_optional_u32(&mut self, field: &'static str) -> Result<Option<u32>, VerifierInfoError> {
        match self.read_u8()? {
            0 => Ok(None),
            1 => Ok(Some(self.read_u32()?)),
            value => Err(VerifierInfoError::InvalidFlag { field, value }),
        }
    }

    fn read_string(&mut self) -> Result<String, VerifierInfoError> {
        let count = self.read_u32()?;
        let count = usize::try_from(count).map_err(|_| VerifierInfoError::LengthOverflow)?;
        let bytes = self.read_exact(count)?;
        std::str::from_utf8(bytes)
            .map(str::to_owned)
            .map_err(|_| VerifierInfoError::InvalidUtf8)
    }

    fn read_u8(&mut self) -> Result<u8, VerifierInfoError> {
        Ok(self.read_exact(1)?[0])
    }

    fn read_u32(&mut self) -> Result<u32, VerifierInfoError> {
        let bytes = self.read_exact(4)?;
        Ok(u32::from_le_bytes(
            bytes.try_into().expect("slice length checked"),
        ))
    }

    fn read_u64(&mut self) -> Result<u64, VerifierInfoError> {
        let bytes = self.read_exact(8)?;
        Ok(u64::from_le_bytes(
            bytes.try_into().expect("slice length checked"),
        ))
    }

    fn read_i64(&mut self) -> Result<i64, VerifierInfoError> {
        let bytes = self.read_exact(8)?;
        Ok(i64::from_le_bytes(
            bytes.try_into().expect("slice length checked"),
        ))
    }

    fn read_f64(&mut self) -> Result<f64, VerifierInfoError> {
        let bytes = self.read_exact(8)?;
        Ok(f64::from_le_bytes(
            bytes.try_into().expect("slice length checked"),
        ))
    }

    fn read_exact(&mut self, count: usize) -> Result<&'a [u8], VerifierInfoError> {
        let end = self
            .offset
            .checked_add(count)
            .ok_or(VerifierInfoError::LengthOverflow)?;
        if end > self.bytes.len() {
            return Err(VerifierInfoError::UnexpectedEof {
                offset: self.offset,
                needed: count,
                available: self.bytes.len().saturating_sub(self.offset),
            });
        }
        let out = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(out)
    }
}
