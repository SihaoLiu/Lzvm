use std::collections::BTreeSet;
use std::fmt;
use std::path::Path;

use crate::sectioned::{
    encode_sectioned_file, parse_sectioned_file, SectionedError, SectionedFile, SectionedSection,
};

mod binary;

const EXPRESSION_INFO_KIND: [u8; 4] = *b"xinf";
const EXPRESSION_INFO_VERSION: u32 = 8;
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
    CommitmentElement {
        id: u32,
        element: u32,
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
    ConstantAt {
        id: u32,
        prime: Option<i64>,
        dimension: u32,
    },
    Commitment {
        id: u32,
        prime: Option<i64>,
        dimension: u32,
    },
    CommitmentElement {
        id: u32,
        element: u32,
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

    pub fn commitment_element(
        id: u32,
        element: u32,
        row_offset_index: Option<u32>,
        row_offset: Option<i64>,
        dimension: Option<u32>,
    ) -> Self {
        Self::CommitmentElement {
            id,
            element,
            row_offset_index,
            row_offset,
            stage: None,
            stage_id: None,
            dimension,
            air_group_id: None,
            air_id: None,
        }
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

    pub fn constant_at(id: u32, prime: Option<i64>, dimension: u32) -> Self {
        Self::ConstantAt {
            id,
            prime,
            dimension,
        }
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

    pub fn commitment_element_at(
        id: u32,
        element: u32,
        prime: Option<i64>,
        dimension: u32,
    ) -> Self {
        Self::CommitmentElement {
            id,
            element,
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
    DuplicateExpressionId {
        expression_id: u32,
    },
    TemporaryReferenceOutOfBounds {
        temporary_id: u32,
        temporary_count: u32,
    },
    TemporaryReadBeforeWrite {
        temporary_id: u32,
        dimension: u32,
        operation_index: usize,
    },
    ZeroDestinationDimension {
        destination_id: u32,
    },
    ZeroOperandDimension {
        source_index: usize,
    },
    ZeroHintPayloadDimension {
        hint_index: usize,
        field_index: usize,
        value_index: usize,
    },
    InvalidOperationSourceCount {
        operation: OperationKind,
        operation_index: usize,
        expected: usize,
        found: usize,
    },
    EmptyCodeBlock {
        item: &'static str,
        index: usize,
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
            Self::TemporaryReadBeforeWrite {
                temporary_id,
                dimension,
                operation_index,
            } => write!(
                f,
                "temporary reference {temporary_id} with dimension {dimension} is read before write at operation {operation_index}"
            ),
            Self::ZeroDestinationDimension { destination_id } => write!(
                f,
                "expression-info destination dimension is zero for destination {destination_id}"
            ),
            Self::ZeroOperandDimension { source_index } => write!(
                f,
                "expression-info source dimension is zero at source {source_index}"
            ),
            Self::ZeroHintPayloadDimension {
                hint_index,
                field_index,
                value_index,
            } => write!(
                f,
                "expression-info hint dimension is zero at hint {hint_index}, field {field_index}, value {value_index}"
            ),
            Self::InvalidOperationSourceCount {
                operation,
                operation_index,
                expected,
                found,
            } => write!(
                f,
                "expression-info operation {operation:?} at index {operation_index} expected {expected} sources, found {found}"
            ),
            Self::EmptyCodeBlock { item, index } => {
                write!(f, "expression-info {item} {index} has no operations")
            }
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

fn validate_expression_info(value: &ExpressionInfo) -> Result<(), ExpressionInfoError> {
    validate_hints(&value.hints)?;
    let mut seen = BTreeSet::new();
    for (index, expression) in value.expressions.iter().enumerate() {
        if !seen.insert(expression.expression_id) {
            return Err(ExpressionInfoError::DuplicateExpressionId {
                expression_id: expression.expression_id,
            });
        }
        if expression.operations.is_empty() {
            return Err(ExpressionInfoError::EmptyCodeBlock {
                item: "expression",
                index,
            });
        }
        validate_operations(&expression.operations, expression.temporary_count)?;
    }
    for (index, constraint) in value.constraints.iter().enumerate() {
        if constraint.boundary == BoundaryKind::EveryFrame
            && (constraint.offset_min.is_none() || constraint.offset_max.is_none())
        {
            return Err(ExpressionInfoError::MissingFrameBoundaryOffsets);
        }
        if constraint.operations.is_empty() {
            return Err(ExpressionInfoError::EmptyCodeBlock {
                item: "constraint",
                index,
            });
        }
        validate_operations(&constraint.operations, constraint.temporary_count)?;
    }
    Ok(())
}

fn validate_hints(hints: &[HintInfo]) -> Result<(), ExpressionInfoError> {
    for (hint_index, hint) in hints.iter().enumerate() {
        for (field_index, field) in hint.fields.iter().enumerate() {
            for (value_index, value) in field.values.iter().enumerate() {
                if hint_payload_dimension(&value.payload).is_some_and(|dimension| dimension == 0) {
                    return Err(ExpressionInfoError::ZeroHintPayloadDimension {
                        hint_index,
                        field_index,
                        value_index,
                    });
                }
            }
        }
    }
    Ok(())
}

fn hint_payload_dimension(value: &HintPayload) -> Option<u32> {
    match value {
        HintPayload::Temporary { dimension, .. }
        | HintPayload::Commitment { dimension, .. }
        | HintPayload::CommitmentElement { dimension, .. }
        | HintPayload::CustomCommitment { dimension, .. }
        | HintPayload::Constant { dimension, .. }
        | HintPayload::AirGroupValue { dimension, .. }
        | HintPayload::AirValue { dimension, .. }
        | HintPayload::ProofValue { dimension, .. } => *dimension,
        HintPayload::Number { .. }
        | HintPayload::String { .. }
        | HintPayload::Challenge { .. }
        | HintPayload::Public { .. } => None,
    }
}

fn validate_operations(
    operations: &[CodeOperation],
    temporary_count: u32,
) -> Result<(), ExpressionInfoError> {
    let mut defined = BTreeSet::new();
    for (operation_index, operation) in operations.iter().enumerate() {
        validate_destination(&operation.destination, temporary_count)?;
        validate_operation_source_count(operation, operation_index)?;
        for (source_index, source) in operation.sources.iter().enumerate() {
            validate_operand(source, temporary_count, source_index)?;
            if let CodeOperand::Temporary { id, dimension } = source {
                if !defined.contains(&(*id, *dimension)) {
                    return Err(ExpressionInfoError::TemporaryReadBeforeWrite {
                        temporary_id: *id,
                        dimension: *dimension,
                        operation_index,
                    });
                }
            }
        }
        if let CodeDestination::Temporary { id, dimension } = &operation.destination {
            defined.insert((*id, *dimension));
        }
    }
    Ok(())
}

fn validate_operation_source_count(
    operation: &CodeOperation,
    operation_index: usize,
) -> Result<(), ExpressionInfoError> {
    let expected = match operation.op {
        OperationKind::Copy => 1,
        OperationKind::Add | OperationKind::Sub | OperationKind::Mul => 2,
    };
    let found = operation.sources.len();
    if found != expected {
        return Err(ExpressionInfoError::InvalidOperationSourceCount {
            operation: operation.op,
            operation_index,
            expected,
            found,
        });
    }
    Ok(())
}

fn validate_destination(
    value: &CodeDestination,
    temporary_count: u32,
) -> Result<(), ExpressionInfoError> {
    let (destination_id, destination_dimension) = destination_id_and_dimension(value);
    if destination_dimension == 0 {
        return Err(ExpressionInfoError::ZeroDestinationDimension { destination_id });
    }

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

fn validate_operand(
    value: &CodeOperand,
    temporary_count: u32,
    source_index: usize,
) -> Result<(), ExpressionInfoError> {
    if operand_dimension(value) == 0 {
        return Err(ExpressionInfoError::ZeroOperandDimension { source_index });
    }

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

fn destination_id_and_dimension(value: &CodeDestination) -> (u32, u32) {
    match value {
        CodeDestination::Temporary { id, dimension }
        | CodeDestination::Quotient { id, dimension }
        | CodeDestination::FriExpression { id, dimension } => (*id, *dimension),
    }
}

fn operand_dimension(value: &CodeOperand) -> u32 {
    match value {
        CodeOperand::Temporary { dimension, .. }
        | CodeOperand::Number { dimension, .. }
        | CodeOperand::Evaluation { dimension, .. }
        | CodeOperand::Challenge { dimension, .. }
        | CodeOperand::Public { dimension, .. }
        | CodeOperand::Constant { dimension, .. }
        | CodeOperand::ConstantAt { dimension, .. }
        | CodeOperand::Commitment { dimension, .. }
        | CodeOperand::CommitmentElement { dimension, .. }
        | CodeOperand::BoundaryZerofier { dimension, .. }
        | CodeOperand::ProofValue { dimension, .. }
        | CodeOperand::OpeningDenominator { dimension, .. }
        | CodeOperand::CustomCommitment { dimension, .. }
        | CodeOperand::AirGroupValue { dimension, .. }
        | CodeOperand::AirValue { dimension, .. } => *dimension,
    }
}
