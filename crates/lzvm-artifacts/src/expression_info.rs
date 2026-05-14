use std::collections::BTreeSet;
use std::fmt;
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub struct ExpressionInfo {
    pub hints: Vec<HintInfo>,
    pub expressions: Vec<ExpressionCode>,
    pub constraints: Vec<ConstraintCode>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HintInfo {
    pub name: String,
    pub fields: Vec<HintFieldInfo>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HintFieldInfo {
    pub name: String,
    pub values: Vec<HintValueInfo>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HintValueInfo {
    pub op: String,
    pub positions: Vec<u32>,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExpressionCode {
    pub expression_id: u32,
    pub stage: u32,
    pub line: String,
    pub temporary_count: u32,
    pub destination: Option<serde_json::Value>,
    pub operations: Vec<CodeOperation>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConstraintCode {
    pub stage: u32,
    pub boundary: BoundaryKind,
    pub offset_min: Option<i64>,
    pub offset_max: Option<i64>,
    pub line: String,
    pub intermediate: bool,
    pub temporary_count: u32,
    pub operations: Vec<CodeOperation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundaryKind {
    EveryRow,
    FirstRow,
    LastRow,
    EveryFrame,
    FinalProof,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CodeOperation {
    pub op: OperationKind,
    pub destination: serde_json::Value,
    pub sources: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationKind {
    Add,
    Sub,
    Mul,
    Copy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpressionInfoError {
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
    UnknownBoundary {
        boundary: String,
    },
    DuplicateExpressionId {
        expression_id: u32,
    },
    TemporaryReferenceOutOfBounds {
        temporary_id: u32,
        temporary_count: u32,
    },
    MissingFrameBoundaryOffsets,
    Io {
        message: String,
    },
}

impl fmt::Display for ExpressionInfoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json { message } => write!(f, "expression-info json error: {message}"),
            Self::MissingField { field } => write!(f, "missing expression-info field: {field}"),
            Self::InvalidField { field } => write!(f, "invalid expression-info field: {field}"),
            Self::UnknownOperation { op } => write!(f, "unknown expression-info operation: {op}"),
            Self::UnknownBoundary { boundary } => {
                write!(f, "unknown expression-info boundary: {boundary}")
            }
            Self::DuplicateExpressionId { expression_id } => {
                write!(f, "duplicate expression-info id: {expression_id}")
            }
            Self::TemporaryReferenceOutOfBounds {
                temporary_id,
                temporary_count,
            } => write!(
                f,
                "temporary reference {temporary_id} is out of bounds for count {temporary_count}"
            ),
            Self::MissingFrameBoundaryOffsets => {
                write!(f, "frame boundary is missing offset bounds")
            }
            Self::Io { message } => write!(f, "expression-info io error: {message}"),
        }
    }
}

impl std::error::Error for ExpressionInfoError {}

impl ExpressionCode {
    pub fn operation_count(&self) -> usize {
        self.operations.len()
    }
}

impl ConstraintCode {
    pub fn operation_count(&self) -> usize {
        self.operations.len()
    }
}

pub fn read_expression_info_file(
    path: impl AsRef<Path>,
) -> Result<ExpressionInfo, ExpressionInfoError> {
    let input = std::fs::read_to_string(path).map_err(|error| ExpressionInfoError::Io {
        message: error.to_string(),
    })?;
    parse_expression_info_json(&input)
}

pub fn parse_expression_info_json(input: &str) -> Result<ExpressionInfo, ExpressionInfoError> {
    let value: serde_json::Value =
        serde_json::from_str(input).map_err(|error| ExpressionInfoError::Json {
            message: error.to_string(),
        })?;
    let object = as_object(&value, "$")?;

    let hints = parse_hints(required_array(object, "hintsInfo")?)?;
    let expressions = parse_expressions(required_array(object, "expressionsCode")?)?;
    let constraints = parse_constraints(required_array(object, "constraints")?)?;

    Ok(ExpressionInfo {
        hints,
        expressions,
        constraints,
    })
}

fn parse_hints(values: &[serde_json::Value]) -> Result<Vec<HintInfo>, ExpressionInfoError> {
    let mut hints = Vec::with_capacity(values.len());
    for value in values {
        let object = as_object(value, "hintsInfo")?;
        let fields = parse_hint_fields(required_array(object, "fields")?)?;
        hints.push(HintInfo {
            name: required_string(object, "name")?,
            fields,
        });
    }
    Ok(hints)
}

fn parse_hint_fields(
    values: &[serde_json::Value],
) -> Result<Vec<HintFieldInfo>, ExpressionInfoError> {
    let mut fields = Vec::with_capacity(values.len());
    for value in values {
        let object = as_object(value, "fields")?;
        let values = parse_hint_values(required_array(object, "values")?)?;
        fields.push(HintFieldInfo {
            name: required_string(object, "name")?,
            values,
        });
    }
    Ok(fields)
}

fn parse_hint_values(
    values: &[serde_json::Value],
) -> Result<Vec<HintValueInfo>, ExpressionInfoError> {
    let mut out = Vec::with_capacity(values.len());
    for value in values {
        let object = as_object(value, "values")?;
        out.push(HintValueInfo {
            op: required_string(object, "op")?,
            positions: required_u32_array(object, "pos")?,
            payload: value.clone(),
        });
    }
    Ok(out)
}

fn parse_expressions(
    values: &[serde_json::Value],
) -> Result<Vec<ExpressionCode>, ExpressionInfoError> {
    let mut seen = BTreeSet::new();
    let mut expressions = Vec::with_capacity(values.len());
    for value in values {
        let object = as_object(value, "expressionsCode")?;
        let expression_id = required_u32(object, "expId")?;
        if !seen.insert(expression_id) {
            return Err(ExpressionInfoError::DuplicateExpressionId { expression_id });
        }
        let temporary_count = required_u32(object, "tmpUsed")?;
        let operations = parse_operations(required_array(object, "code")?, temporary_count)?;
        expressions.push(ExpressionCode {
            expression_id,
            stage: optional_u32(object, "stage")?.unwrap_or(0),
            line: optional_string(object, "line")?.unwrap_or_default(),
            temporary_count,
            destination: object.get("dest").cloned(),
            operations,
        });
    }
    Ok(expressions)
}

fn parse_constraints(
    values: &[serde_json::Value],
) -> Result<Vec<ConstraintCode>, ExpressionInfoError> {
    let mut constraints = Vec::with_capacity(values.len());
    for value in values {
        let object = as_object(value, "constraints")?;
        let temporary_count = required_u32(object, "tmpUsed")?;
        let boundary = parse_boundary(&required_string(object, "boundary")?)?;
        let offset_min = optional_i64(object, "offsetMin")?;
        let offset_max = optional_i64(object, "offsetMax")?;
        if boundary == BoundaryKind::EveryFrame && (offset_min.is_none() || offset_max.is_none()) {
            return Err(ExpressionInfoError::MissingFrameBoundaryOffsets);
        }
        constraints.push(ConstraintCode {
            stage: required_u32(object, "stage")?,
            boundary,
            offset_min,
            offset_max,
            line: optional_string(object, "line")?.unwrap_or_default(),
            intermediate: optional_u32(object, "imPol")?.unwrap_or(0) != 0,
            temporary_count,
            operations: parse_operations(required_array(object, "code")?, temporary_count)?,
        });
    }
    Ok(constraints)
}

fn parse_operations(
    values: &[serde_json::Value],
    temporary_count: u32,
) -> Result<Vec<CodeOperation>, ExpressionInfoError> {
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
        operations.push(CodeOperation {
            op,
            destination,
            sources,
        });
    }
    Ok(operations)
}

fn parse_operation(op: &str) -> Result<OperationKind, ExpressionInfoError> {
    match op {
        "add" => Ok(OperationKind::Add),
        "sub" => Ok(OperationKind::Sub),
        "mul" => Ok(OperationKind::Mul),
        "copy" => Ok(OperationKind::Copy),
        _ => Err(ExpressionInfoError::UnknownOperation { op: op.to_owned() }),
    }
}

fn parse_boundary(boundary: &str) -> Result<BoundaryKind, ExpressionInfoError> {
    match boundary {
        "everyRow" => Ok(BoundaryKind::EveryRow),
        "firstRow" => Ok(BoundaryKind::FirstRow),
        "lastRow" => Ok(BoundaryKind::LastRow),
        "everyFrame" => Ok(BoundaryKind::EveryFrame),
        "finalProof" => Ok(BoundaryKind::FinalProof),
        _ => Err(ExpressionInfoError::UnknownBoundary {
            boundary: boundary.to_owned(),
        }),
    }
}

fn validate_temporary_reference(
    value: &serde_json::Value,
    temporary_count: u32,
) -> Result<(), ExpressionInfoError> {
    let Some(object) = value.as_object() else {
        return Err(ExpressionInfoError::InvalidField { field: "reference" });
    };
    if object.get("type").and_then(serde_json::Value::as_str) != Some("tmp") {
        return Ok(());
    }
    let id = value_to_u32(
        object
            .get("id")
            .ok_or(ExpressionInfoError::MissingField { field: "id" })?,
        "id",
    )?;
    if id >= temporary_count {
        return Err(ExpressionInfoError::TemporaryReferenceOutOfBounds {
            temporary_id: id,
            temporary_count,
        });
    }
    Ok(())
}

fn as_object<'a>(
    value: &'a serde_json::Value,
    field: &'static str,
) -> Result<&'a serde_json::Map<String, serde_json::Value>, ExpressionInfoError> {
    value
        .as_object()
        .ok_or(ExpressionInfoError::InvalidField { field })
}

fn required<'a>(
    object: &'a serde_json::Map<String, serde_json::Value>,
    field: &'static str,
) -> Result<&'a serde_json::Value, ExpressionInfoError> {
    object
        .get(field)
        .ok_or(ExpressionInfoError::MissingField { field })
}

fn required_array<'a>(
    object: &'a serde_json::Map<String, serde_json::Value>,
    field: &'static str,
) -> Result<&'a Vec<serde_json::Value>, ExpressionInfoError> {
    required(object, field)?
        .as_array()
        .ok_or(ExpressionInfoError::InvalidField { field })
}

fn required_string(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &'static str,
) -> Result<String, ExpressionInfoError> {
    required(object, field)?
        .as_str()
        .map(str::to_owned)
        .ok_or(ExpressionInfoError::InvalidField { field })
}

fn optional_string(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &'static str,
) -> Result<Option<String>, ExpressionInfoError> {
    object
        .get(field)
        .map(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or(ExpressionInfoError::InvalidField { field })
        })
        .transpose()
}

fn required_u32(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &'static str,
) -> Result<u32, ExpressionInfoError> {
    value_to_u32(required(object, field)?, field)
}

fn optional_u32(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &'static str,
) -> Result<Option<u32>, ExpressionInfoError> {
    object
        .get(field)
        .map(|value| value_to_u32(value, field))
        .transpose()
}

fn value_to_u32(
    value: &serde_json::Value,
    field: &'static str,
) -> Result<u32, ExpressionInfoError> {
    let Some(number) = value.as_u64() else {
        return Err(ExpressionInfoError::InvalidField { field });
    };
    u32::try_from(number).map_err(|_| ExpressionInfoError::InvalidField { field })
}

fn optional_i64(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &'static str,
) -> Result<Option<i64>, ExpressionInfoError> {
    object
        .get(field)
        .map(|value| {
            value
                .as_i64()
                .ok_or(ExpressionInfoError::InvalidField { field })
        })
        .transpose()
}

fn required_u32_array(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &'static str,
) -> Result<Vec<u32>, ExpressionInfoError> {
    let values = required_array(object, field)?;
    let mut out = Vec::with_capacity(values.len());
    for value in values {
        out.push(value_to_u32(value, field)?);
    }
    Ok(out)
}
