use std::collections::BTreeSet;
use std::fmt;
use std::path::Path;

use crate::sectioned::{
    encode_sectioned_file, parse_sectioned_file, SectionedError, SectionedFile, SectionedSection,
};

const EXPRESSION_INFO_KIND: [u8; 4] = *b"xinf";
const EXPRESSION_INFO_VERSION: u32 = 4;
const EXPRESSION_INFO_SECTION_ID: u32 = 1;

const JSON_NULL_TAG: u8 = 0;
const JSON_BOOL_TAG: u8 = 1;
const JSON_U64_TAG: u8 = 2;
const JSON_I64_TAG: u8 = 3;
const JSON_F64_TAG: u8 = 4;
const JSON_STRING_TAG: u8 = 5;
const JSON_ARRAY_TAG: u8 = 6;
const JSON_OBJECT_TAG: u8 = 7;

const EXPRESSION_DESTINATION_COMMITMENT_TAG: u8 = 1;

const DESTINATION_TEMPORARY_TAG: u8 = 1;
const DESTINATION_QUOTIENT_TAG: u8 = 2;
const DESTINATION_FRI_TAG: u8 = 3;

const OPERAND_TEMPORARY_TAG: u8 = 1;
const OPERAND_NUMBER_TAG: u8 = 2;
const OPERAND_EVALUATION_TAG: u8 = 3;
const OPERAND_CHALLENGE_TAG: u8 = 4;
const OPERAND_PUBLIC_TAG: u8 = 5;
const OPERAND_CONSTANT_TAG: u8 = 6;
const OPERAND_COMMITMENT_TAG: u8 = 7;
const OPERAND_BOUNDARY_TAG: u8 = 8;
const OPERAND_PROOF_VALUE_TAG: u8 = 9;
const OPERAND_OPENING_DENOMINATOR_TAG: u8 = 10;
const OPERAND_CUSTOM_COMMITMENT_TAG: u8 = 11;
const OPERAND_AIR_GROUP_VALUE_TAG: u8 = 12;
const OPERAND_AIR_VALUE_TAG: u8 = 13;

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
    InvalidJsonTag {
        value: u8,
    },
    InvalidJsonNumber,
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
            Self::InvalidJsonTag { value } => {
                write!(f, "invalid expression-info value tag: {value}")
            }
            Self::InvalidJsonNumber => write!(f, "invalid expression-info number value"),
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
    parse_expression_info_section(&section.data)
}

pub fn encode_expression_info(value: &ExpressionInfo) -> Result<Vec<u8>, ExpressionInfoError> {
    validate_expression_info(value)?;
    let section = encode_expression_info_section(value)?;
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

fn parse_expression_info_section(bytes: &[u8]) -> Result<ExpressionInfo, ExpressionInfoError> {
    let mut reader = Reader::new(bytes);
    let value = ExpressionInfo {
        hints: read_hints(&mut reader)?,
        expressions: read_expressions(&mut reader)?,
        constraints: read_constraints(&mut reader)?,
    };
    if reader.position() != bytes.len() {
        return Err(ExpressionInfoError::UnexpectedTrailingBytes {
            count: bytes.len() - reader.position(),
        });
    }
    validate_expression_info(&value)?;
    Ok(value)
}

fn encode_expression_info_section(value: &ExpressionInfo) -> Result<Vec<u8>, ExpressionInfoError> {
    let mut out = Vec::new();
    write_hints(&mut out, &value.hints)?;
    write_expressions(&mut out, &value.expressions)?;
    write_constraints(&mut out, &value.constraints)?;
    Ok(out)
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
            op: required_string(object, "op")?,
            positions: required_u32_array(object, "pos")?,
            payload: value.clone(),
        });
    }
    Ok(out)
}

fn read_hints(reader: &mut Reader<'_>) -> Result<Vec<HintInfo>, ExpressionInfoError> {
    let count = reader.read_u32()?;
    let mut hints = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let name = reader.read_string()?;
        let field_count = reader.read_u32()?;
        let mut fields = Vec::with_capacity(field_count as usize);
        for _ in 0..field_count {
            let name = reader.read_string()?;
            let value_count = reader.read_u32()?;
            let mut values = Vec::with_capacity(value_count as usize);
            for _ in 0..value_count {
                let op = reader.read_string()?;
                let position_count = reader.read_u32()?;
                let mut positions = Vec::with_capacity(position_count as usize);
                for _ in 0..position_count {
                    positions.push(reader.read_u32()?);
                }
                let payload = reader.read_json_value()?;
                values.push(HintValueInfo {
                    op,
                    positions,
                    payload,
                });
            }
            fields.push(HintFieldInfo { name, values });
        }
        hints.push(HintInfo { name, fields });
    }
    Ok(hints)
}

fn write_hints(out: &mut Vec<u8>, values: &[HintInfo]) -> Result<(), ExpressionInfoError> {
    write_len(out, values.len())?;
    for hint in values {
        write_string(out, &hint.name)?;
        write_len(out, hint.fields.len())?;
        for field in &hint.fields {
            write_string(out, &field.name)?;
            write_len(out, field.values.len())?;
            for value in &field.values {
                write_string(out, &value.op)?;
                write_len(out, value.positions.len())?;
                for position in &value.positions {
                    write_u32(out, *position);
                }
                write_json_value(out, &value.payload)?;
            }
        }
    }
    Ok(())
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

fn read_expressions(reader: &mut Reader<'_>) -> Result<Vec<ExpressionCode>, ExpressionInfoError> {
    let count = reader.read_u32()?;
    let mut expressions = Vec::with_capacity(count as usize);
    for _ in 0..count {
        expressions.push(ExpressionCode {
            expression_id: reader.read_u32()?,
            stage: reader.read_u32()?,
            line: reader.read_string()?,
            temporary_count: reader.read_u32()?,
            destination: reader.read_optional_expression_destination("expression_destination")?,
            operations: read_operations(reader)?,
        });
    }
    Ok(expressions)
}

fn write_expressions(
    out: &mut Vec<u8>,
    values: &[ExpressionCode],
) -> Result<(), ExpressionInfoError> {
    write_len(out, values.len())?;
    for value in values {
        write_u32(out, value.expression_id);
        write_u32(out, value.stage);
        write_string(out, &value.line)?;
        write_u32(out, value.temporary_count);
        write_optional_expression_destination(out, value.destination.as_ref());
        write_operations(out, &value.operations)?;
    }
    Ok(())
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

fn read_constraints(reader: &mut Reader<'_>) -> Result<Vec<ConstraintCode>, ExpressionInfoError> {
    let count = reader.read_u32()?;
    let mut constraints = Vec::with_capacity(count as usize);
    for _ in 0..count {
        constraints.push(ConstraintCode {
            stage: reader.read_u32()?,
            boundary: read_boundary_tag(reader.read_u8()?)?,
            offset_min: reader.read_optional_i64("offset_min")?,
            offset_max: reader.read_optional_i64("offset_max")?,
            line: reader.read_string()?,
            intermediate: reader.read_bool("intermediate")?,
            temporary_count: reader.read_u32()?,
            operations: read_operations(reader)?,
        });
    }
    Ok(constraints)
}

fn write_constraints(
    out: &mut Vec<u8>,
    values: &[ConstraintCode],
) -> Result<(), ExpressionInfoError> {
    write_len(out, values.len())?;
    for value in values {
        write_u32(out, value.stage);
        out.push(boundary_tag(value.boundary));
        write_optional_i64(out, value.offset_min);
        write_optional_i64(out, value.offset_max);
        write_string(out, &value.line)?;
        out.push(u8::from(value.intermediate));
        write_u32(out, value.temporary_count);
        write_operations(out, &value.operations)?;
    }
    Ok(())
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

fn read_operations(reader: &mut Reader<'_>) -> Result<Vec<CodeOperation>, ExpressionInfoError> {
    let count = reader.read_u32()?;
    let mut operations = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let op = read_operation_tag(reader.read_u8()?)?;
        let destination = reader.read_destination()?;
        let source_count = reader.read_u32()?;
        let mut sources = Vec::with_capacity(source_count as usize);
        for _ in 0..source_count {
            sources.push(reader.read_operand()?);
        }
        operations.push(CodeOperation {
            op,
            destination,
            sources,
        });
    }
    Ok(operations)
}

fn write_operations(
    out: &mut Vec<u8>,
    values: &[CodeOperation],
) -> Result<(), ExpressionInfoError> {
    write_len(out, values.len())?;
    for value in values {
        out.push(operation_tag(value.op));
        write_destination(out, &value.destination);
        write_len(out, value.sources.len())?;
        for source in &value.sources {
            write_operand(out, source);
        }
    }
    Ok(())
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

fn operation_tag(value: OperationKind) -> u8 {
    match value {
        OperationKind::Add => 1,
        OperationKind::Sub => 2,
        OperationKind::Mul => 3,
        OperationKind::Copy => 4,
    }
}

fn read_operation_tag(value: u8) -> Result<OperationKind, ExpressionInfoError> {
    match value {
        1 => Ok(OperationKind::Add),
        2 => Ok(OperationKind::Sub),
        3 => Ok(OperationKind::Mul),
        4 => Ok(OperationKind::Copy),
        _ => Err(ExpressionInfoError::InvalidOperationTag { value }),
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

fn boundary_tag(value: BoundaryKind) -> u8 {
    match value {
        BoundaryKind::EveryRow => 1,
        BoundaryKind::FirstRow => 2,
        BoundaryKind::LastRow => 3,
        BoundaryKind::EveryFrame => 4,
        BoundaryKind::FinalProof => 5,
    }
}

fn read_boundary_tag(value: u8) -> Result<BoundaryKind, ExpressionInfoError> {
    match value {
        1 => Ok(BoundaryKind::EveryRow),
        2 => Ok(BoundaryKind::FirstRow),
        3 => Ok(BoundaryKind::LastRow),
        4 => Ok(BoundaryKind::EveryFrame),
        5 => Ok(BoundaryKind::FinalProof),
        _ => Err(ExpressionInfoError::InvalidBoundaryTag { value }),
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

fn write_json_value(
    out: &mut Vec<u8>,
    value: &serde_json::Value,
) -> Result<(), ExpressionInfoError> {
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
                return Err(ExpressionInfoError::InvalidJsonNumber);
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

fn write_optional_expression_destination(out: &mut Vec<u8>, value: Option<&ExpressionDestination>) {
    match value {
        Some(value) => {
            out.push(1);
            write_expression_destination(out, value);
        }
        None => out.push(0),
    }
}

fn write_expression_destination(out: &mut Vec<u8>, value: &ExpressionDestination) {
    match value {
        ExpressionDestination::Commitment {
            id,
            stage,
            stage_id,
        } => {
            out.push(EXPRESSION_DESTINATION_COMMITMENT_TAG);
            write_u32(out, *id);
            write_optional_u32(out, *stage);
            write_optional_u32(out, *stage_id);
        }
    }
}

fn write_optional_i64(out: &mut Vec<u8>, value: Option<i64>) {
    match value {
        Some(value) => {
            out.push(1);
            write_i64(out, value);
        }
        None => out.push(0),
    }
}

fn write_destination(out: &mut Vec<u8>, value: &CodeDestination) {
    match value {
        CodeDestination::Temporary { id, dimension } => {
            out.push(DESTINATION_TEMPORARY_TAG);
            write_reference_body(out, *id, *dimension);
        }
        CodeDestination::Quotient { id, dimension } => {
            out.push(DESTINATION_QUOTIENT_TAG);
            write_reference_body(out, *id, *dimension);
        }
        CodeDestination::FriExpression { id, dimension } => {
            out.push(DESTINATION_FRI_TAG);
            write_reference_body(out, *id, *dimension);
        }
    }
}

fn write_operand(out: &mut Vec<u8>, value: &CodeOperand) {
    match value {
        CodeOperand::Temporary { id, dimension } => {
            out.push(OPERAND_TEMPORARY_TAG);
            write_reference_body(out, *id, *dimension);
        }
        CodeOperand::Number { value, dimension } => {
            out.push(OPERAND_NUMBER_TAG);
            write_u64(out, *value);
            write_u32(out, *dimension);
        }
        CodeOperand::Evaluation { id, dimension } => {
            out.push(OPERAND_EVALUATION_TAG);
            write_reference_body(out, *id, *dimension);
        }
        CodeOperand::Challenge {
            id,
            stage,
            stage_id,
            dimension,
        } => {
            out.push(OPERAND_CHALLENGE_TAG);
            write_reference_body(out, *id, *dimension);
            write_optional_u32(out, *stage);
            write_optional_u32(out, *stage_id);
        }
        CodeOperand::Public { id, dimension } => {
            out.push(OPERAND_PUBLIC_TAG);
            write_reference_body(out, *id, *dimension);
        }
        CodeOperand::Constant { id, dimension } => {
            out.push(OPERAND_CONSTANT_TAG);
            write_reference_body(out, *id, *dimension);
        }
        CodeOperand::Commitment {
            id,
            prime,
            dimension,
        } => {
            out.push(OPERAND_COMMITMENT_TAG);
            write_reference_body(out, *id, *dimension);
            write_optional_i64(out, *prime);
        }
        CodeOperand::BoundaryZerofier { id, dimension } => {
            out.push(OPERAND_BOUNDARY_TAG);
            write_reference_body(out, *id, *dimension);
        }
        CodeOperand::ProofValue {
            id,
            stage,
            dimension,
        } => {
            out.push(OPERAND_PROOF_VALUE_TAG);
            write_reference_body(out, *id, *dimension);
            write_optional_u32(out, *stage);
        }
        CodeOperand::OpeningDenominator {
            id,
            opening,
            dimension,
        } => {
            out.push(OPERAND_OPENING_DENOMINATOR_TAG);
            write_reference_body(out, *id, *dimension);
            write_optional_u32(out, *opening);
        }
        CodeOperand::CustomCommitment {
            id,
            commit_id,
            prime,
            dimension,
        } => {
            out.push(OPERAND_CUSTOM_COMMITMENT_TAG);
            write_reference_body(out, *id, *dimension);
            write_optional_u32(out, *commit_id);
            write_optional_i64(out, *prime);
        }
        CodeOperand::AirGroupValue {
            id,
            stage,
            air_group_id,
            dimension,
        } => {
            out.push(OPERAND_AIR_GROUP_VALUE_TAG);
            write_reference_body(out, *id, *dimension);
            write_optional_u32(out, *stage);
            write_optional_u32(out, *air_group_id);
        }
        CodeOperand::AirValue {
            id,
            stage,
            air_group_id,
            dimension,
        } => {
            out.push(OPERAND_AIR_VALUE_TAG);
            write_reference_body(out, *id, *dimension);
            write_optional_u32(out, *stage);
            write_optional_u32(out, *air_group_id);
        }
    }
}

fn write_reference_body(out: &mut Vec<u8>, id: u32, dimension: u32) {
    write_u32(out, id);
    write_u32(out, dimension);
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

fn write_string(out: &mut Vec<u8>, value: &str) -> Result<(), ExpressionInfoError> {
    write_len(out, value.len())?;
    out.extend_from_slice(value.as_bytes());
    Ok(())
}

fn write_len(out: &mut Vec<u8>, value: usize) -> Result<(), ExpressionInfoError> {
    let value = u32::try_from(value).map_err(|_| ExpressionInfoError::LengthOverflow)?;
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

    fn read_optional_expression_destination(
        &mut self,
        field: &'static str,
    ) -> Result<Option<ExpressionDestination>, ExpressionInfoError> {
        match self.read_u8()? {
            0 => Ok(None),
            1 => Ok(Some(self.read_expression_destination()?)),
            value => Err(ExpressionInfoError::InvalidFlag { field, value }),
        }
    }

    fn read_expression_destination(
        &mut self,
    ) -> Result<ExpressionDestination, ExpressionInfoError> {
        let tag = self.read_u8()?;
        match tag {
            EXPRESSION_DESTINATION_COMMITMENT_TAG => {
                let id = self.read_u32()?;
                let stage = self.read_optional_u32("expression_destination_stage")?;
                let stage_id = self.read_optional_u32("expression_destination_stage_id")?;
                Ok(ExpressionDestination::commitment(id, stage, stage_id))
            }
            value => Err(ExpressionInfoError::InvalidOperandTag { value }),
        }
    }

    fn read_destination(&mut self) -> Result<CodeDestination, ExpressionInfoError> {
        let tag = self.read_u8()?;
        let (id, dimension) = self.read_reference_body()?;
        match tag {
            DESTINATION_TEMPORARY_TAG => Ok(CodeDestination::temporary(id, dimension)),
            DESTINATION_QUOTIENT_TAG => Ok(CodeDestination::quotient(id, dimension)),
            DESTINATION_FRI_TAG => Ok(CodeDestination::fri_expression(id, dimension)),
            value => Err(ExpressionInfoError::InvalidOperandTag { value }),
        }
    }

    fn read_operand(&mut self) -> Result<CodeOperand, ExpressionInfoError> {
        let tag = self.read_u8()?;
        match tag {
            OPERAND_TEMPORARY_TAG => {
                let (id, dimension) = self.read_reference_body()?;
                Ok(CodeOperand::temporary(id, dimension))
            }
            OPERAND_NUMBER_TAG => Ok(CodeOperand::number(self.read_u64()?, self.read_u32()?)),
            OPERAND_EVALUATION_TAG => {
                let (id, dimension) = self.read_reference_body()?;
                Ok(CodeOperand::evaluation(id, dimension))
            }
            OPERAND_CHALLENGE_TAG => {
                let (id, dimension) = self.read_reference_body()?;
                let stage = self.read_optional_u32("challenge_stage")?;
                let stage_id = self.read_optional_u32("challenge_stage_id")?;
                Ok(CodeOperand::challenge(id, stage, stage_id, dimension))
            }
            OPERAND_PUBLIC_TAG => {
                let (id, dimension) = self.read_reference_body()?;
                Ok(CodeOperand::public(id, dimension))
            }
            OPERAND_CONSTANT_TAG => {
                let (id, dimension) = self.read_reference_body()?;
                Ok(CodeOperand::constant(id, dimension))
            }
            OPERAND_COMMITMENT_TAG => {
                let (id, dimension) = self.read_reference_body()?;
                let prime = self.read_optional_i64("commitment_prime")?;
                Ok(CodeOperand::commitment_at(id, prime, dimension))
            }
            OPERAND_BOUNDARY_TAG => {
                let (id, dimension) = self.read_reference_body()?;
                Ok(CodeOperand::boundary_zerofier(id, dimension))
            }
            OPERAND_PROOF_VALUE_TAG => {
                let (id, dimension) = self.read_reference_body()?;
                let stage = self.read_optional_u32("proof_value_stage")?;
                Ok(CodeOperand::proof_value_at(id, stage, dimension))
            }
            OPERAND_OPENING_DENOMINATOR_TAG => {
                let (id, dimension) = self.read_reference_body()?;
                let opening = self.read_optional_u32("opening_denominator_opening")?;
                Ok(CodeOperand::opening_denominator(id, opening, dimension))
            }
            OPERAND_CUSTOM_COMMITMENT_TAG => {
                let (id, dimension) = self.read_reference_body()?;
                let commit_id = self.read_optional_u32("custom_commitment_id")?;
                let prime = self.read_optional_i64("custom_commitment_prime")?;
                Ok(CodeOperand::custom_commitment(
                    id, commit_id, prime, dimension,
                ))
            }
            OPERAND_AIR_GROUP_VALUE_TAG => {
                let (id, dimension) = self.read_reference_body()?;
                let stage = self.read_optional_u32("air_group_value_stage")?;
                let air_group_id = self.read_optional_u32("air_group_value_group")?;
                Ok(CodeOperand::air_group_value(
                    id,
                    stage,
                    air_group_id,
                    dimension,
                ))
            }
            OPERAND_AIR_VALUE_TAG => {
                let (id, dimension) = self.read_reference_body()?;
                let stage = self.read_optional_u32("air_value_stage")?;
                let air_group_id = self.read_optional_u32("air_value_group")?;
                Ok(CodeOperand::air_value(id, stage, air_group_id, dimension))
            }
            value => Err(ExpressionInfoError::InvalidOperandTag { value }),
        }
    }

    fn read_reference_body(&mut self) -> Result<(u32, u32), ExpressionInfoError> {
        Ok((self.read_u32()?, self.read_u32()?))
    }

    fn read_json_value(&mut self) -> Result<serde_json::Value, ExpressionInfoError> {
        let tag = self.read_u8()?;
        match tag {
            JSON_NULL_TAG => Ok(serde_json::Value::Null),
            JSON_BOOL_TAG => match self.read_u8()? {
                0 => Ok(serde_json::Value::Bool(false)),
                1 => Ok(serde_json::Value::Bool(true)),
                value => Err(ExpressionInfoError::InvalidFlag {
                    field: "json_bool",
                    value,
                }),
            },
            JSON_U64_TAG => Ok(serde_json::Value::Number(self.read_u64()?.into())),
            JSON_I64_TAG => Ok(serde_json::Value::Number(self.read_i64()?.into())),
            JSON_F64_TAG => {
                let value = self.read_f64()?;
                let number = serde_json::Number::from_f64(value)
                    .ok_or(ExpressionInfoError::InvalidJsonNumber)?;
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
            value => Err(ExpressionInfoError::InvalidJsonTag { value }),
        }
    }

    fn read_optional_i64(
        &mut self,
        field: &'static str,
    ) -> Result<Option<i64>, ExpressionInfoError> {
        match self.read_u8()? {
            0 => Ok(None),
            1 => Ok(Some(self.read_i64()?)),
            value => Err(ExpressionInfoError::InvalidFlag { field, value }),
        }
    }

    fn read_optional_u32(
        &mut self,
        field: &'static str,
    ) -> Result<Option<u32>, ExpressionInfoError> {
        match self.read_u8()? {
            0 => Ok(None),
            1 => Ok(Some(self.read_u32()?)),
            value => Err(ExpressionInfoError::InvalidFlag { field, value }),
        }
    }

    fn read_bool(&mut self, field: &'static str) -> Result<bool, ExpressionInfoError> {
        match self.read_u8()? {
            0 => Ok(false),
            1 => Ok(true),
            value => Err(ExpressionInfoError::InvalidFlag { field, value }),
        }
    }

    fn read_string(&mut self) -> Result<String, ExpressionInfoError> {
        let count = self.read_u32()?;
        let count = usize::try_from(count).map_err(|_| ExpressionInfoError::LengthOverflow)?;
        let bytes = self.read_exact(count)?;
        std::str::from_utf8(bytes)
            .map(str::to_owned)
            .map_err(|_| ExpressionInfoError::InvalidUtf8)
    }

    fn read_u8(&mut self) -> Result<u8, ExpressionInfoError> {
        Ok(self.read_exact(1)?[0])
    }

    fn read_u32(&mut self) -> Result<u32, ExpressionInfoError> {
        let bytes = self.read_exact(4)?;
        Ok(u32::from_le_bytes(
            bytes.try_into().expect("slice length checked"),
        ))
    }

    fn read_u64(&mut self) -> Result<u64, ExpressionInfoError> {
        let bytes = self.read_exact(8)?;
        Ok(u64::from_le_bytes(
            bytes.try_into().expect("slice length checked"),
        ))
    }

    fn read_i64(&mut self) -> Result<i64, ExpressionInfoError> {
        let bytes = self.read_exact(8)?;
        Ok(i64::from_le_bytes(
            bytes.try_into().expect("slice length checked"),
        ))
    }

    fn read_f64(&mut self) -> Result<f64, ExpressionInfoError> {
        let bytes = self.read_exact(8)?;
        Ok(f64::from_le_bytes(
            bytes.try_into().expect("slice length checked"),
        ))
    }

    fn read_exact(&mut self, count: usize) -> Result<&'a [u8], ExpressionInfoError> {
        let end = self
            .offset
            .checked_add(count)
            .ok_or(ExpressionInfoError::LengthOverflow)?;
        if end > self.bytes.len() {
            return Err(ExpressionInfoError::UnexpectedEof {
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
