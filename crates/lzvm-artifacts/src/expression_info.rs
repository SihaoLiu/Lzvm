use std::collections::BTreeSet;
use std::fmt;
use std::path::Path;

use crate::sectioned::{
    encode_sectioned_file, parse_sectioned_file, SectionedError, SectionedFile, SectionedSection,
};

mod binary;

const EXPRESSION_INFO_KIND: [u8; 4] = *b"xinf";
const EXPRESSION_INFO_VERSION: u32 = 5;
const EXPRESSION_INFO_SECTION_ID: u32 = 1;

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
    pub positions: Vec<u32>,
    pub payload: HintPayload,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HintPayload {
    Number {
        value: u64,
    },
    String {
        value: String,
    },
    Temporary {
        id: u32,
        dimension: Option<u32>,
    },
    Commitment {
        id: u32,
        row_offset_index: Option<u32>,
        row_offset: Option<i64>,
        stage: Option<u32>,
        stage_id: Option<u32>,
        dimension: Option<u32>,
        air_group_id: Option<u32>,
        air_id: Option<u32>,
    },
    CustomCommitment {
        id: u32,
        commit_id: Option<u32>,
        row_offset_index: Option<u32>,
        row_offset: Option<i64>,
        stage: Option<u32>,
        stage_id: Option<u32>,
        dimension: Option<u32>,
        air_group_id: Option<u32>,
        air_id: Option<u32>,
    },
    Constant {
        id: u32,
        row_offset_index: Option<u32>,
        row_offset: Option<i64>,
        dimension: Option<u32>,
        air_group_id: Option<u32>,
        air_id: Option<u32>,
    },
    Challenge {
        id: u32,
        stage: Option<u32>,
        stage_id: Option<u32>,
    },
    Public {
        id: u32,
        stage: Option<u32>,
    },
    AirGroupValue {
        id: u32,
        air_group_id: Option<u32>,
        stage: Option<u32>,
        dimension: Option<u32>,
    },
    AirValue {
        id: u32,
        stage: Option<u32>,
        dimension: Option<u32>,
    },
    ProofValue {
        id: u32,
        stage: Option<u32>,
        dimension: Option<u32>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExpressionCode {
    pub expression_id: u32,
    pub stage: u32,
    pub line: String,
    pub temporary_count: u32,
    pub destination: Option<ExpressionDestination>,
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
    pub destination: CodeDestination,
    pub sources: Vec<CodeOperand>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationKind {
    Add,
    Sub,
    Mul,
    Copy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpressionDestination {
    Commitment {
        id: u32,
        stage: Option<u32>,
        stage_id: Option<u32>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodeDestination {
    Temporary { id: u32, dimension: u32 },
    Quotient { id: u32, dimension: u32 },
    FriExpression { id: u32, dimension: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodeOperand {
    Temporary {
        id: u32,
        dimension: u32,
    },
    Number {
        value: u64,
        dimension: u32,
    },
    Evaluation {
        id: u32,
        dimension: u32,
    },
    Challenge {
        id: u32,
        stage: Option<u32>,
        stage_id: Option<u32>,
        dimension: u32,
    },
    Public {
        id: u32,
        dimension: u32,
    },
    Constant {
        id: u32,
        dimension: u32,
    },
    Commitment {
        id: u32,
        prime: Option<i64>,
        dimension: u32,
    },
    BoundaryZerofier {
        id: u32,
        dimension: u32,
    },
    ProofValue {
        id: u32,
        stage: Option<u32>,
        dimension: u32,
    },
    OpeningDenominator {
        id: u32,
        opening: Option<u32>,
        dimension: u32,
    },
    CustomCommitment {
        id: u32,
        commit_id: Option<u32>,
        prime: Option<i64>,
        dimension: u32,
    },
    AirGroupValue {
        id: u32,
        stage: Option<u32>,
        air_group_id: Option<u32>,
        dimension: u32,
    },
    AirValue {
        id: u32,
        stage: Option<u32>,
        air_group_id: Option<u32>,
        dimension: u32,
    },
}

impl ExpressionDestination {
    pub fn commitment(id: u32, stage: Option<u32>, stage_id: Option<u32>) -> Self {
        Self::Commitment {
            id,
            stage,
            stage_id,
        }
    }
}

impl HintPayload {
    pub fn number(value: u64) -> Self {
        Self::Number { value }
    }

    pub fn string(value: impl Into<String>) -> Self {
        Self::String {
            value: value.into(),
        }
    }

    pub fn temporary(id: u32, dimension: Option<u32>) -> Self {
        Self::Temporary { id, dimension }
    }

    pub fn constant(
        id: u32,
        row_offset_index: Option<u32>,
        row_offset: Option<i64>,
        dimension: Option<u32>,
        air_group_id: Option<u32>,
        air_id: Option<u32>,
    ) -> Self {
        Self::Constant {
            id,
            row_offset_index,
            row_offset,
            dimension,
            air_group_id,
            air_id,
        }
    }

    pub fn challenge(id: u32, stage: Option<u32>, stage_id: Option<u32>) -> Self {
        Self::Challenge {
            id,
            stage,
            stage_id,
        }
    }

    pub fn public(id: u32, stage: Option<u32>) -> Self {
        Self::Public { id, stage }
    }

    pub fn air_group_value(
        id: u32,
        air_group_id: Option<u32>,
        stage: Option<u32>,
        dimension: Option<u32>,
    ) -> Self {
        Self::AirGroupValue {
            id,
            air_group_id,
            stage,
            dimension,
        }
    }

    pub fn air_value(id: u32, stage: Option<u32>, dimension: Option<u32>) -> Self {
        Self::AirValue {
            id,
            stage,
            dimension,
        }
    }

    pub fn proof_value(id: u32, stage: Option<u32>, dimension: Option<u32>) -> Self {
        Self::ProofValue {
            id,
            stage,
            dimension,
        }
    }
}

impl CodeDestination {
    pub fn temporary(id: u32, dimension: u32) -> Self {
        Self::Temporary { id, dimension }
    }

    pub fn quotient(id: u32, dimension: u32) -> Self {
        Self::Quotient { id, dimension }
    }

    pub fn fri_expression(id: u32, dimension: u32) -> Self {
        Self::FriExpression { id, dimension }
    }
}

impl CodeOperand {
    pub fn temporary(id: u32, dimension: u32) -> Self {
        Self::Temporary { id, dimension }
    }

    pub fn number(value: u64, dimension: u32) -> Self {
        Self::Number { value, dimension }
    }

    pub fn evaluation(id: u32, dimension: u32) -> Self {
        Self::Evaluation { id, dimension }
    }

    pub fn challenge(id: u32, stage: Option<u32>, stage_id: Option<u32>, dimension: u32) -> Self {
        Self::Challenge {
            id,
            stage,
            stage_id,
            dimension,
        }
    }

    pub fn public(id: u32, dimension: u32) -> Self {
        Self::Public { id, dimension }
    }

    pub fn constant(id: u32, dimension: u32) -> Self {
        Self::Constant { id, dimension }
    }

    pub fn commitment(id: u32, dimension: u32) -> Self {
        Self::Commitment {
            id,
            prime: None,
            dimension,
        }
    }

    pub fn commitment_at(id: u32, prime: Option<i64>, dimension: u32) -> Self {
        Self::Commitment {
            id,
            prime,
            dimension,
        }
    }

    pub fn boundary_zerofier(id: u32, dimension: u32) -> Self {
        Self::BoundaryZerofier { id, dimension }
    }

    pub fn proof_value(id: u32, dimension: u32) -> Self {
        Self::ProofValue {
            id,
            stage: None,
            dimension,
        }
    }

    pub fn proof_value_at(id: u32, stage: Option<u32>, dimension: u32) -> Self {
        Self::ProofValue {
            id,
            stage,
            dimension,
        }
    }

    pub fn x_div_x_sub(id: u32, dimension: u32) -> Self {
        Self::OpeningDenominator {
            id,
            opening: None,
            dimension,
        }
    }

    pub fn opening_denominator(id: u32, opening: Option<u32>, dimension: u32) -> Self {
        Self::OpeningDenominator {
            id,
            opening,
            dimension,
        }
    }

    pub fn custom_commitment(
        id: u32,
        commit_id: Option<u32>,
        prime: Option<i64>,
        dimension: u32,
    ) -> Self {
        Self::CustomCommitment {
            id,
            commit_id,
            prime,
            dimension,
        }
    }

    pub fn air_group_value(
        id: u32,
        stage: Option<u32>,
        air_group_id: Option<u32>,
        dimension: u32,
    ) -> Self {
        Self::AirGroupValue {
            id,
            stage,
            air_group_id,
            dimension,
        }
    }

    pub fn air_value(
        id: u32,
        stage: Option<u32>,
        air_group_id: Option<u32>,
        dimension: u32,
    ) -> Self {
        Self::AirValue {
            id,
            stage,
            air_group_id,
            dimension,
        }
    }
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
    UnknownReferenceKind {
        kind: String,
    },
    InvalidNumber {
        value: String,
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
    InvalidBoundaryTag {
        value: u8,
    },
    InvalidOperandTag {
        value: u8,
    },
    InvalidHintPayloadTag {
        value: u8,
    },
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
            Self::UnknownReferenceKind { kind } => {
                write!(f, "unknown expression-info reference kind: {kind}")
            }
            Self::InvalidNumber { value } => {
                write!(f, "invalid expression-info number: {value}")
            }
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
            Self::InvalidMagic => write!(f, "invalid expression-info file magic"),
            Self::UnsupportedVersion { found, max } => {
                write!(f, "unsupported expression-info file version {found}, max {max}")
            }
            Self::InvalidSectionCount { found } => {
                write!(f, "invalid expression-info section count {found}")
            }
            Self::InvalidSectionId { found } => {
                write!(f, "invalid expression-info section id {found}")
            }
            Self::UnexpectedTrailingBytes { count } => {
                write!(f, "unexpected trailing bytes in expression-info file: {count}")
            }
            Self::UnexpectedEof {
                offset,
                needed,
                available,
            } => write!(
                f,
                "unexpected end of expression-info file at {offset}, needed {needed}, available {available}"
            ),
            Self::LengthOverflow => write!(f, "expression-info length overflow"),
            Self::InvalidUtf8 => write!(f, "expression-info string is not valid utf-8"),
            Self::InvalidFlag { field, value } => {
                write!(f, "invalid expression-info flag for {field}: {value}")
            }
            Self::InvalidOperationTag { value } => {
                write!(f, "invalid expression-info operation tag: {value}")
            }
            Self::InvalidBoundaryTag { value } => {
                write!(f, "invalid expression-info boundary tag: {value}")
            }
            Self::InvalidOperandTag { value } => {
                write!(f, "invalid expression-info operand tag: {value}")
            }
            Self::InvalidHintPayloadTag { value } => {
                write!(f, "invalid expression-info hint payload tag: {value}")
            }
            Self::Io { message } => write!(f, "expression-info io error: {message}"),
        }
    }
}

impl std::error::Error for ExpressionInfoError {}

impl From<SectionedError> for ExpressionInfoError {
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
    read_expression_info_binary_file(path)
}

pub fn read_expression_info_binary_file(
    path: impl AsRef<Path>,
) -> Result<ExpressionInfo, ExpressionInfoError> {
    let bytes = std::fs::read(path).map_err(|error| ExpressionInfoError::Io {
        message: error.to_string(),
    })?;
    parse_expression_info(&bytes)
}

pub fn parse_expression_info(bytes: &[u8]) -> Result<ExpressionInfo, ExpressionInfoError> {
    let file = parse_sectioned_file(bytes, EXPRESSION_INFO_KIND, EXPRESSION_INFO_VERSION)
        .map_err(ExpressionInfoError::from)?;
    if file.version != EXPRESSION_INFO_VERSION {
        return Err(ExpressionInfoError::UnsupportedVersion {
            found: file.version,
            max: EXPRESSION_INFO_VERSION,
        });
    }
    if file.sections.len() != 1 {
        return Err(ExpressionInfoError::InvalidSectionCount {
            found: u32::try_from(file.sections.len()).unwrap_or(u32::MAX),
        });
    }
    let section = &file.sections[0];
    if section.id != EXPRESSION_INFO_SECTION_ID {
        return Err(ExpressionInfoError::InvalidSectionId { found: section.id });
    }
    let value = binary::parse_section(&section.data)?;
    validate_expression_info(&value)?;
    Ok(value)
}

pub fn encode_expression_info(value: &ExpressionInfo) -> Result<Vec<u8>, ExpressionInfoError> {
    validate_expression_info(value)?;
    let section = binary::encode_section(value)?;
    let file = SectionedFile {
        kind: EXPRESSION_INFO_KIND,
        version: EXPRESSION_INFO_VERSION,
        sections: vec![SectionedSection {
            id: EXPRESSION_INFO_SECTION_ID,
            data: section,
        }],
    };
    encode_sectioned_file(&file).map_err(ExpressionInfoError::from)
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

fn validate_expression_info(value: &ExpressionInfo) -> Result<(), ExpressionInfoError> {
    let mut seen = BTreeSet::new();
    for expression in &value.expressions {
        if !seen.insert(expression.expression_id) {
            return Err(ExpressionInfoError::DuplicateExpressionId {
                expression_id: expression.expression_id,
            });
        }
        validate_operations(&expression.operations, expression.temporary_count)?;
    }
    for constraint in &value.constraints {
        if constraint.boundary == BoundaryKind::EveryFrame
            && (constraint.offset_min.is_none() || constraint.offset_max.is_none())
        {
            return Err(ExpressionInfoError::MissingFrameBoundaryOffsets);
        }
        validate_operations(&constraint.operations, constraint.temporary_count)?;
    }
    Ok(())
}

fn validate_operations(
    operations: &[CodeOperation],
    temporary_count: u32,
) -> Result<(), ExpressionInfoError> {
    for operation in operations {
        validate_destination(&operation.destination, temporary_count)?;
        for source in &operation.sources {
            validate_operand(source, temporary_count)?;
        }
    }
    Ok(())
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
            positions: required_u32_array(object, "pos")?,
            payload: parse_hint_payload(object)?,
        });
    }
    Ok(out)
}

fn parse_hint_payload(
    object: &serde_json::Map<String, serde_json::Value>,
) -> Result<HintPayload, ExpressionInfoError> {
    let kind = required_string(object, "op")?;
    match kind.as_str() {
        "number" => Ok(HintPayload::number(required_number(object, "value")?)),
        "string" => Ok(HintPayload::string(required_string(object, "string")?)),
        "tmp" | "exp" => Ok(HintPayload::temporary(
            required_u32(object, "id")?,
            optional_u32(object, "dim")?,
        )),
        "cm" => Ok(HintPayload::Commitment {
            id: required_u32(object, "id")?,
            row_offset_index: optional_u32(object, "rowOffsetIndex")?,
            row_offset: optional_i64(object, "rowOffset")?,
            stage: optional_u32(object, "stage")?,
            stage_id: optional_u32(object, "stageId")?,
            dimension: optional_u32(object, "dim")?,
            air_group_id: optional_u32(object, "airgroupId")?,
            air_id: optional_u32(object, "airId")?,
        }),
        "custom" => Ok(HintPayload::CustomCommitment {
            id: required_u32(object, "id")?,
            commit_id: optional_u32(object, "commitId")?,
            row_offset_index: optional_u32(object, "rowOffsetIndex")?,
            row_offset: optional_i64(object, "rowOffset")?,
            stage: optional_u32(object, "stage")?,
            stage_id: optional_u32(object, "stageId")?,
            dimension: optional_u32(object, "dim")?,
            air_group_id: optional_u32(object, "airgroupId")?,
            air_id: optional_u32(object, "airId")?,
        }),
        "const" => Ok(HintPayload::constant(
            required_u32(object, "id")?,
            optional_u32(object, "rowOffsetIndex")?,
            optional_i64(object, "rowOffset")?,
            optional_u32(object, "dim")?,
            optional_u32(object, "airgroupId")?,
            optional_u32(object, "airId")?,
        )),
        "challenge" => Ok(HintPayload::challenge(
            required_u32(object, "id")?,
            optional_u32(object, "stage")?,
            optional_u32(object, "stageId")?,
        )),
        "public" => Ok(HintPayload::public(
            required_u32(object, "id")?,
            optional_u32(object, "stage")?,
        )),
        "airgroupvalue" => Ok(HintPayload::air_group_value(
            required_u32(object, "id")?,
            optional_u32(object, "airgroupId")?,
            optional_u32(object, "stage")?,
            optional_u32(object, "dim")?,
        )),
        "airvalue" => Ok(HintPayload::air_value(
            required_u32(object, "id")?,
            optional_u32(object, "stage")?,
            optional_u32(object, "dim")?,
        )),
        "proofvalue" | "proofValue" => Ok(HintPayload::proof_value(
            required_u32(object, "id")?,
            optional_u32(object, "stage")?,
            optional_u32(object, "dim")?,
        )),
        _ => Err(ExpressionInfoError::UnknownReferenceKind { kind }),
    }
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
            destination: object
                .get("dest")
                .map(parse_expression_destination)
                .transpose()?,
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
        let destination = parse_destination(required(object, "dest")?, temporary_count)?;
        let sources = parse_operands(required_array(object, "src")?, temporary_count)?;
        validate_destination(&destination, temporary_count)?;
        for source in &sources {
            validate_operand(source, temporary_count)?;
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

fn validate_destination(
    value: &CodeDestination,
    temporary_count: u32,
) -> Result<(), ExpressionInfoError> {
    if let CodeDestination::Temporary { id, .. } = value {
        if *id >= temporary_count {
            return Err(ExpressionInfoError::TemporaryReferenceOutOfBounds {
                temporary_id: *id,
                temporary_count,
            });
        }
    }
    Ok(())
}

fn validate_operand(value: &CodeOperand, temporary_count: u32) -> Result<(), ExpressionInfoError> {
    if let CodeOperand::Temporary { id, .. } = value {
        if *id >= temporary_count {
            return Err(ExpressionInfoError::TemporaryReferenceOutOfBounds {
                temporary_id: *id,
                temporary_count,
            });
        }
    }
    Ok(())
}

fn parse_expression_destination(
    value: &serde_json::Value,
) -> Result<ExpressionDestination, ExpressionInfoError> {
    let object = as_object(value, "dest")?;
    let kind = required_string(object, "op")?;
    match kind.as_str() {
        "cm" => Ok(ExpressionDestination::commitment(
            required_u32(object, "id")?,
            optional_u32(object, "stage")?,
            optional_u32(object, "stageId")?,
        )),
        _ => Err(ExpressionInfoError::UnknownReferenceKind { kind }),
    }
}

fn parse_destination(
    value: &serde_json::Value,
    temporary_count: u32,
) -> Result<CodeDestination, ExpressionInfoError> {
    let object = as_object(value, "dest")?;
    let kind = required_string(object, "type")?;
    let id = required_u32(object, "id")?;
    let dimension = optional_u32(object, "dim")?.unwrap_or(1);
    let destination = match kind.as_str() {
        "tmp" => CodeDestination::temporary(id, dimension),
        "q" => CodeDestination::quotient(id, dimension),
        "f" => CodeDestination::fri_expression(id, dimension),
        _ => return Err(ExpressionInfoError::UnknownReferenceKind { kind }),
    };
    validate_destination(&destination, temporary_count)?;
    Ok(destination)
}

fn parse_operands(
    values: &[serde_json::Value],
    temporary_count: u32,
) -> Result<Vec<CodeOperand>, ExpressionInfoError> {
    let mut operands = Vec::with_capacity(values.len());
    for value in values {
        let operand = parse_operand(value)?;
        validate_operand(&operand, temporary_count)?;
        operands.push(operand);
    }
    Ok(operands)
}

fn parse_operand(value: &serde_json::Value) -> Result<CodeOperand, ExpressionInfoError> {
    let object = as_object(value, "reference")?;
    let kind = required_string(object, "type")?;
    let dimension = optional_u32(object, "dim")?.unwrap_or(1);
    match kind.as_str() {
        "tmp" => Ok(CodeOperand::temporary(
            required_u32(object, "id")?,
            dimension,
        )),
        "number" => Ok(CodeOperand::number(
            required_number(object, "value")?,
            dimension,
        )),
        "eval" => Ok(CodeOperand::evaluation(
            required_u32(object, "id")?,
            dimension,
        )),
        "challenge" => Ok(CodeOperand::challenge(
            required_u32(object, "id")?,
            optional_u32(object, "stage")?,
            optional_u32(object, "stageId")?,
            dimension,
        )),
        "public" => Ok(CodeOperand::public(required_u32(object, "id")?, dimension)),
        "const" => Ok(CodeOperand::constant(
            required_u32(object, "id")?,
            dimension,
        )),
        "cm" => Ok(CodeOperand::commitment_at(
            required_u32(object, "id")?,
            optional_i64(object, "prime")?,
            dimension,
        )),
        "custom" => Ok(CodeOperand::custom_commitment(
            required_u32(object, "id")?,
            optional_u32(object, "commitId")?,
            optional_i64(object, "prime")?,
            dimension,
        )),
        "Zi" => {
            let id = match optional_u32(object, "boundaryId")? {
                Some(id) => id,
                None => required_u32(object, "id")?,
            };
            Ok(CodeOperand::boundary_zerofier(id, dimension))
        }
        "proofvalue" | "proofValue" => Ok(CodeOperand::proof_value_at(
            required_u32(object, "id")?,
            optional_u32(object, "stage")?,
            dimension,
        )),
        "xDivXSub" | "xdivxsub" | "xDivXSubXi" => Ok(CodeOperand::opening_denominator(
            required_u32(object, "id")?,
            optional_u32(object, "opening")?,
            dimension,
        )),
        "airgroupvalue" => Ok(CodeOperand::air_group_value(
            required_u32(object, "id")?,
            optional_u32(object, "stage")?,
            optional_u32(object, "airgroupId")?,
            dimension,
        )),
        "airvalue" => Ok(CodeOperand::air_value(
            required_u32(object, "id")?,
            optional_u32(object, "stage")?,
            optional_u32(object, "airgroupId")?,
            dimension,
        )),
        _ => Err(ExpressionInfoError::UnknownReferenceKind { kind }),
    }
}

fn required_number(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &'static str,
) -> Result<u64, ExpressionInfoError> {
    let value = required(object, field)?;
    if let Some(text) = value.as_str() {
        return text
            .parse::<u64>()
            .map_err(|_| ExpressionInfoError::InvalidNumber {
                value: text.to_owned(),
            });
    }
    value
        .as_u64()
        .ok_or_else(|| ExpressionInfoError::InvalidNumber {
            value: value.to_string(),
        })
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
