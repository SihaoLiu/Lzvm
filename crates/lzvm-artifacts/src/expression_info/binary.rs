use super::{
    BoundaryKind, CodeDestination, CodeOperand, CodeOperation, ConstraintCode, ExpressionCode,
    ExpressionDestination, ExpressionInfo, ExpressionInfoError, HintFieldInfo, HintInfo,
    HintPayload, HintValueInfo, OperationKind,
};

const EXPRESSION_DESTINATION_COMMITMENT_TAG: u8 = 1;

const HINT_NUMBER_TAG: u8 = 1;
const HINT_STRING_TAG: u8 = 2;
const HINT_TEMPORARY_TAG: u8 = 3;
const HINT_COMMITMENT_TAG: u8 = 4;
const HINT_CUSTOM_COMMITMENT_TAG: u8 = 5;
const HINT_CONSTANT_TAG: u8 = 6;
const HINT_CHALLENGE_TAG: u8 = 7;
const HINT_PUBLIC_TAG: u8 = 8;
const HINT_AIR_GROUP_VALUE_TAG: u8 = 9;
const HINT_AIR_VALUE_TAG: u8 = 10;
const HINT_PROOF_VALUE_TAG: u8 = 11;

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
const OPERAND_CONSTANT_AT_TAG: u8 = 14;
const OPERAND_COMMITMENT_ELEMENT_TAG: u8 = 15;

const U32_BYTES: usize = 4;
const TAG_BYTES: usize = 1;
const FLAG_BYTES: usize = 1;
const STRING_MIN_BYTES: usize = U32_BYTES;
const REFERENCE_BODY_BYTES: usize = U32_BYTES + U32_BYTES;
const DESTINATION_MIN_BYTES: usize = TAG_BYTES + REFERENCE_BODY_BYTES;
const OPERAND_MIN_BYTES: usize = TAG_BYTES + REFERENCE_BODY_BYTES;
const HINT_MIN_BYTES: usize = STRING_MIN_BYTES + U32_BYTES;
const HINT_FIELD_MIN_BYTES: usize = STRING_MIN_BYTES + U32_BYTES;
const HINT_VALUE_MIN_BYTES: usize = TAG_BYTES + STRING_MIN_BYTES + U32_BYTES;
const POSITION_BYTES: usize = U32_BYTES;
const EXPRESSION_MIN_BYTES: usize =
    U32_BYTES + U32_BYTES + STRING_MIN_BYTES + U32_BYTES + FLAG_BYTES + U32_BYTES;
const CONSTRAINT_MIN_BYTES: usize = U32_BYTES
    + TAG_BYTES
    + FLAG_BYTES
    + FLAG_BYTES
    + STRING_MIN_BYTES
    + FLAG_BYTES
    + U32_BYTES
    + U32_BYTES;
const OPERATION_MIN_BYTES: usize = TAG_BYTES + DESTINATION_MIN_BYTES + U32_BYTES;

pub(super) fn parse_section(bytes: &[u8]) -> Result<ExpressionInfo, ExpressionInfoError> {
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
    Ok(value)
}
pub(super) fn encode_section(value: &ExpressionInfo) -> Result<Vec<u8>, ExpressionInfoError> {
    let mut out = Vec::new();
    write_hints(&mut out, &value.hints)?;
    write_expressions(&mut out, &value.expressions)?;
    write_constraints(&mut out, &value.constraints)?;
    Ok(out)
}
fn read_hints(reader: &mut Reader<'_>) -> Result<Vec<HintInfo>, ExpressionInfoError> {
    let count = read_bounded_count(reader, HINT_MIN_BYTES)?;
    let mut hints = Vec::with_capacity(count);
    for _ in 0..count {
        let name = reader.read_string()?;
        let field_count = read_bounded_count(reader, HINT_FIELD_MIN_BYTES)?;
        let mut fields = Vec::with_capacity(field_count);
        for _ in 0..field_count {
            let name = reader.read_string()?;
            let value_count = read_bounded_count(reader, HINT_VALUE_MIN_BYTES)?;
            let mut values = Vec::with_capacity(value_count);
            for _ in 0..value_count {
                let payload = reader.read_hint_payload()?;
                let position_count = read_bounded_count(reader, POSITION_BYTES)?;
                let mut positions = Vec::with_capacity(position_count);
                for _ in 0..position_count {
                    positions.push(reader.read_u32()?);
                }
                values.push(HintValueInfo { positions, payload });
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
                write_hint_payload(out, &value.payload)?;
                write_len(out, value.positions.len())?;
                for position in &value.positions {
                    write_u32(out, *position);
                }
            }
        }
    }
    Ok(())
}
fn read_expressions(reader: &mut Reader<'_>) -> Result<Vec<ExpressionCode>, ExpressionInfoError> {
    let count = read_bounded_count(reader, EXPRESSION_MIN_BYTES)?;
    let mut expressions = Vec::with_capacity(count);
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
fn read_constraints(reader: &mut Reader<'_>) -> Result<Vec<ConstraintCode>, ExpressionInfoError> {
    let count = read_bounded_count(reader, CONSTRAINT_MIN_BYTES)?;
    let mut constraints = Vec::with_capacity(count);
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
fn read_operations(reader: &mut Reader<'_>) -> Result<Vec<CodeOperation>, ExpressionInfoError> {
    let count = read_bounded_count(reader, OPERATION_MIN_BYTES)?;
    let mut operations = Vec::with_capacity(count);
    for _ in 0..count {
        let op = read_operation_tag(reader.read_u8()?)?;
        let destination = reader.read_destination()?;
        let source_count = read_bounded_count(reader, OPERAND_MIN_BYTES)?;
        let mut sources = Vec::with_capacity(source_count);
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
fn write_hint_payload(out: &mut Vec<u8>, value: &HintPayload) -> Result<(), ExpressionInfoError> {
    match value {
        HintPayload::Number { value } => {
            out.push(HINT_NUMBER_TAG);
            write_u64(out, *value);
        }
        HintPayload::String { value } => {
            out.push(HINT_STRING_TAG);
            write_string(out, value)?;
        }
        HintPayload::Temporary { id, dimension } => {
            out.push(HINT_TEMPORARY_TAG);
            write_u32(out, *id);
            write_optional_u32(out, *dimension);
        }
        HintPayload::Commitment {
            id,
            row_offset_index,
            row_offset,
            stage,
            stage_id,
            dimension,
            air_group_id,
            air_id,
        } => {
            out.push(HINT_COMMITMENT_TAG);
            write_u32(out, *id);
            write_optional_u32(out, *row_offset_index);
            write_optional_i64(out, *row_offset);
            write_optional_u32(out, *stage);
            write_optional_u32(out, *stage_id);
            write_optional_u32(out, *dimension);
            write_optional_u32(out, *air_group_id);
            write_optional_u32(out, *air_id);
        }
        HintPayload::CustomCommitment {
            id,
            commit_id,
            row_offset_index,
            row_offset,
            stage,
            stage_id,
            dimension,
            air_group_id,
            air_id,
        } => {
            out.push(HINT_CUSTOM_COMMITMENT_TAG);
            write_u32(out, *id);
            write_optional_u32(out, *commit_id);
            write_optional_u32(out, *row_offset_index);
            write_optional_i64(out, *row_offset);
            write_optional_u32(out, *stage);
            write_optional_u32(out, *stage_id);
            write_optional_u32(out, *dimension);
            write_optional_u32(out, *air_group_id);
            write_optional_u32(out, *air_id);
        }
        HintPayload::Constant {
            id,
            row_offset_index,
            row_offset,
            dimension,
            air_group_id,
            air_id,
        } => {
            out.push(HINT_CONSTANT_TAG);
            write_u32(out, *id);
            write_optional_u32(out, *row_offset_index);
            write_optional_i64(out, *row_offset);
            write_optional_u32(out, *dimension);
            write_optional_u32(out, *air_group_id);
            write_optional_u32(out, *air_id);
        }
        HintPayload::Challenge {
            id,
            stage,
            stage_id,
        } => {
            out.push(HINT_CHALLENGE_TAG);
            write_u32(out, *id);
            write_optional_u32(out, *stage);
            write_optional_u32(out, *stage_id);
        }
        HintPayload::Public { id, stage } => {
            out.push(HINT_PUBLIC_TAG);
            write_u32(out, *id);
            write_optional_u32(out, *stage);
        }
        HintPayload::AirGroupValue {
            id,
            air_group_id,
            stage,
            dimension,
        } => {
            out.push(HINT_AIR_GROUP_VALUE_TAG);
            write_u32(out, *id);
            write_optional_u32(out, *air_group_id);
            write_optional_u32(out, *stage);
            write_optional_u32(out, *dimension);
        }
        HintPayload::AirValue {
            id,
            stage,
            dimension,
        } => {
            out.push(HINT_AIR_VALUE_TAG);
            write_u32(out, *id);
            write_optional_u32(out, *stage);
            write_optional_u32(out, *dimension);
        }
        HintPayload::ProofValue {
            id,
            stage,
            dimension,
        } => {
            out.push(HINT_PROOF_VALUE_TAG);
            write_u32(out, *id);
            write_optional_u32(out, *stage);
            write_optional_u32(out, *dimension);
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
        CodeOperand::ConstantAt {
            id,
            prime,
            dimension,
        } => {
            out.push(OPERAND_CONSTANT_AT_TAG);
            write_reference_body(out, *id, *dimension);
            write_optional_i64(out, *prime);
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
        CodeOperand::CommitmentElement {
            id,
            element,
            prime,
            dimension,
        } => {
            out.push(OPERAND_COMMITMENT_ELEMENT_TAG);
            write_reference_body(out, *id, *dimension);
            write_u32(out, *element);
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
fn read_bounded_count(
    reader: &mut Reader<'_>,
    record_min_bytes: usize,
) -> Result<usize, ExpressionInfoError> {
    let count = u32_to_usize(reader.read_u32()?)?;
    if count > reader.remaining_len() / record_min_bytes {
        return Err(ExpressionInfoError::LengthOverflow);
    }
    Ok(count)
}
fn u32_to_usize(value: u32) -> Result<usize, ExpressionInfoError> {
    usize::try_from(value).map_err(|_| ExpressionInfoError::LengthOverflow)
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

    fn remaining_len(&self) -> usize {
        self.bytes.len() - self.offset
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
            OPERAND_CONSTANT_AT_TAG => {
                let (id, dimension) = self.read_reference_body()?;
                let prime = self.read_optional_i64("constant_prime")?;
                Ok(CodeOperand::constant_at(id, prime, dimension))
            }
            OPERAND_COMMITMENT_TAG => {
                let (id, dimension) = self.read_reference_body()?;
                let prime = self.read_optional_i64("commitment_prime")?;
                Ok(CodeOperand::commitment_at(id, prime, dimension))
            }
            OPERAND_COMMITMENT_ELEMENT_TAG => {
                let (id, dimension) = self.read_reference_body()?;
                let element = self.read_u32()?;
                let prime = self.read_optional_i64("commitment_element_prime")?;
                Ok(CodeOperand::commitment_element_at(
                    id, element, prime, dimension,
                ))
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

    fn read_hint_payload(&mut self) -> Result<HintPayload, ExpressionInfoError> {
        let tag = self.read_u8()?;
        match tag {
            HINT_NUMBER_TAG => Ok(HintPayload::number(self.read_u64()?)),
            HINT_STRING_TAG => Ok(HintPayload::string(self.read_string()?)),
            HINT_TEMPORARY_TAG => {
                let id = self.read_u32()?;
                let dimension = self.read_optional_u32("hint_temporary_dimension")?;
                Ok(HintPayload::temporary(id, dimension))
            }
            HINT_COMMITMENT_TAG => {
                let id = self.read_u32()?;
                let row_offset_index =
                    self.read_optional_u32("hint_commitment_row_offset_index")?;
                let row_offset = self.read_optional_i64("hint_commitment_row_offset")?;
                let stage = self.read_optional_u32("hint_commitment_stage")?;
                let stage_id = self.read_optional_u32("hint_commitment_stage_id")?;
                let dimension = self.read_optional_u32("hint_commitment_dimension")?;
                let air_group_id = self.read_optional_u32("hint_commitment_group")?;
                let air_id = self.read_optional_u32("hint_commitment_air")?;
                Ok(HintPayload::Commitment {
                    id,
                    row_offset_index,
                    row_offset,
                    stage,
                    stage_id,
                    dimension,
                    air_group_id,
                    air_id,
                })
            }
            HINT_CUSTOM_COMMITMENT_TAG => {
                let id = self.read_u32()?;
                let commit_id = self.read_optional_u32("hint_custom_commitment_id")?;
                let row_offset_index =
                    self.read_optional_u32("hint_custom_commitment_row_offset_index")?;
                let row_offset = self.read_optional_i64("hint_custom_commitment_row_offset")?;
                let stage = self.read_optional_u32("hint_custom_commitment_stage")?;
                let stage_id = self.read_optional_u32("hint_custom_commitment_stage_id")?;
                let dimension = self.read_optional_u32("hint_custom_commitment_dimension")?;
                let air_group_id = self.read_optional_u32("hint_custom_commitment_group")?;
                let air_id = self.read_optional_u32("hint_custom_commitment_air")?;
                Ok(HintPayload::CustomCommitment {
                    id,
                    commit_id,
                    row_offset_index,
                    row_offset,
                    stage,
                    stage_id,
                    dimension,
                    air_group_id,
                    air_id,
                })
            }
            HINT_CONSTANT_TAG => {
                let id = self.read_u32()?;
                let row_offset_index = self.read_optional_u32("hint_constant_row_offset_index")?;
                let row_offset = self.read_optional_i64("hint_constant_row_offset")?;
                let dimension = self.read_optional_u32("hint_constant_dimension")?;
                let air_group_id = self.read_optional_u32("hint_constant_group")?;
                let air_id = self.read_optional_u32("hint_constant_air")?;
                Ok(HintPayload::constant(
                    id,
                    row_offset_index,
                    row_offset,
                    dimension,
                    air_group_id,
                    air_id,
                ))
            }
            HINT_CHALLENGE_TAG => {
                let id = self.read_u32()?;
                let stage = self.read_optional_u32("hint_challenge_stage")?;
                let stage_id = self.read_optional_u32("hint_challenge_stage_id")?;
                Ok(HintPayload::challenge(id, stage, stage_id))
            }
            HINT_PUBLIC_TAG => {
                let id = self.read_u32()?;
                let stage = self.read_optional_u32("hint_public_stage")?;
                Ok(HintPayload::public(id, stage))
            }
            HINT_AIR_GROUP_VALUE_TAG => {
                let id = self.read_u32()?;
                let air_group_id = self.read_optional_u32("hint_group_value_group")?;
                let stage = self.read_optional_u32("hint_group_value_stage")?;
                let dimension = self.read_optional_u32("hint_group_value_dimension")?;
                Ok(HintPayload::air_group_value(
                    id,
                    air_group_id,
                    stage,
                    dimension,
                ))
            }
            HINT_AIR_VALUE_TAG => {
                let id = self.read_u32()?;
                let stage = self.read_optional_u32("hint_air_value_stage")?;
                let dimension = self.read_optional_u32("hint_air_value_dimension")?;
                Ok(HintPayload::air_value(id, stage, dimension))
            }
            HINT_PROOF_VALUE_TAG => {
                let id = self.read_u32()?;
                let stage = self.read_optional_u32("hint_proof_value_stage")?;
                let dimension = self.read_optional_u32("hint_proof_value_dimension")?;
                Ok(HintPayload::proof_value(id, stage, dimension))
            }
            value => Err(ExpressionInfoError::InvalidHintPayloadTag { value }),
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
        let count = u32_to_usize(count)?;
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
