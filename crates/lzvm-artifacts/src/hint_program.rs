use std::fmt;
use std::path::Path;

use crate::expression_info::{
    ExpressionInfo, HintFieldInfo as ExpressionHintFieldInfo, HintInfo as ExpressionHintInfo,
    HintPayload, HintValueInfo,
};
use crate::sectioned::{
    encode_sectioned_file, parse_sectioned_file, SectionedError, SectionedFile, SectionedSection,
};

const HINT_MIN_BYTES: usize = 1 + 4;
const FIELD_MIN_BYTES: usize = 1 + 4;
const VALUE_MIN_BYTES: usize = 1 + 4;
const POSITION_BYTES: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HintProgram {
    pub hints: Vec<Hint>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hint {
    pub name: String,
    pub fields: Vec<HintField>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HintField {
    pub name: String,
    pub values: Vec<HintValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HintValue {
    pub operand: HintOperand,
    pub positions: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HintOperand {
    Number(u64),
    String(String),
    GroupValue {
        group_id: u32,
        id: u32,
    },
    AirGroupValue {
        id: u32,
    },
    AirValue {
        id: u32,
    },
    Challenge {
        id: u32,
    },
    Commitment {
        id: u32,
        row_offset_index: u32,
    },
    Constant {
        id: u32,
        row_offset_index: u32,
    },
    CustomCommitment {
        id: u32,
        row_offset_index: u32,
        commit_id: u32,
    },
    Temporary {
        id: u32,
        dimension: Option<u32>,
    },
    Public {
        id: u32,
    },
    ProofValue {
        id: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HintProgramError {
    Sectioned(SectionedError),
    MissingHintSection {
        section_id: u32,
    },
    UnexpectedTrailingBytes {
        count: usize,
    },
    UnexpectedEof {
        offset: usize,
        needed: usize,
        available: usize,
    },
    MissingStringTerminator {
        offset: usize,
    },
    InvalidUtf8,
    StringContainsNul {
        value: String,
    },
    UnknownOperand {
        op: String,
    },
    InvalidOperandSection {
        op: &'static str,
        section: &'static str,
    },
    MissingOperandField {
        op: &'static str,
        field: &'static str,
    },
    LengthOverflow,
    Io {
        message: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HintSectionKind {
    Regular,
    Global,
}

impl fmt::Display for HintProgramError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sectioned(error) => write!(f, "hint program container error: {error}"),
            Self::MissingHintSection { section_id } => {
                write!(f, "missing hint section {section_id}")
            }
            Self::UnexpectedTrailingBytes { count } => {
                write!(f, "unexpected trailing hint program bytes: {count}")
            }
            Self::UnexpectedEof {
                offset,
                needed,
                available,
            } => write!(
                f,
                "unexpected end of hint program at {offset}, needed {needed}, available {available}"
            ),
            Self::MissingStringTerminator { offset } => {
                write!(f, "missing hint string terminator at offset {offset}")
            }
            Self::InvalidUtf8 => write!(f, "hint string is not valid utf-8"),
            Self::StringContainsNul { value } => {
                write!(f, "hint string contains nul byte: {value}")
            }
            Self::UnknownOperand { op } => write!(f, "unknown hint operand: {op}"),
            Self::InvalidOperandSection { op, section } => {
                write!(f, "hint operand {op} is invalid for {section} section")
            }
            Self::MissingOperandField { op, field } => {
                write!(f, "hint operand {op} is missing field {field}")
            }
            Self::LengthOverflow => write!(f, "hint program length overflow"),
            Self::Io { message } => write!(f, "hint program io error: {message}"),
        }
    }
}

impl std::error::Error for HintProgramError {}

pub fn read_regular_hint_program_file(
    path: impl AsRef<Path>,
) -> Result<HintProgram, HintProgramError> {
    let bytes = read_file(path)?;
    parse_regular_hint_program(&bytes)
}

pub fn read_global_hint_program_file(
    path: impl AsRef<Path>,
) -> Result<HintProgram, HintProgramError> {
    let bytes = read_file(path)?;
    parse_global_hint_program(&bytes)
}

fn read_file(path: impl AsRef<Path>) -> Result<Vec<u8>, HintProgramError> {
    std::fs::read(path).map_err(|error| HintProgramError::Io {
        message: error.to_string(),
    })
}

pub fn parse_regular_hint_program(bytes: &[u8]) -> Result<HintProgram, HintProgramError> {
    parse_hint_program(bytes, 3, HintSectionKind::Regular)
}

pub fn encode_regular_hint_program(program: &HintProgram) -> Result<Vec<u8>, HintProgramError> {
    encode_hint_program(program, 3, HintSectionKind::Regular)
}

pub fn parse_global_hint_program(bytes: &[u8]) -> Result<HintProgram, HintProgramError> {
    parse_hint_program(bytes, 2, HintSectionKind::Global)
}

pub fn encode_global_hint_program(program: &HintProgram) -> Result<Vec<u8>, HintProgramError> {
    encode_hint_program(program, 2, HintSectionKind::Global)
}

pub fn regular_hint_program_from_expression_info(
    info: &ExpressionInfo,
) -> Result<HintProgram, HintProgramError> {
    hint_program_from_expression_info(info, HintSectionKind::Regular)
}

pub fn global_hint_program_from_expression_info(
    info: &ExpressionInfo,
) -> Result<HintProgram, HintProgramError> {
    hint_program_from_expression_info(info, HintSectionKind::Global)
}

fn hint_program_from_expression_info(
    info: &ExpressionInfo,
    kind: HintSectionKind,
) -> Result<HintProgram, HintProgramError> {
    let hints = info
        .hints
        .iter()
        .map(|hint| hint_from_expression_info(hint, kind))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(HintProgram { hints })
}

fn hint_from_expression_info(
    hint: &ExpressionHintInfo,
    kind: HintSectionKind,
) -> Result<Hint, HintProgramError> {
    let fields = hint
        .fields
        .iter()
        .map(|field| hint_field_from_expression_info(field, kind))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Hint {
        name: hint.name.clone(),
        fields,
    })
}

fn hint_field_from_expression_info(
    field: &ExpressionHintFieldInfo,
    kind: HintSectionKind,
) -> Result<HintField, HintProgramError> {
    let values = field
        .values
        .iter()
        .map(|value| hint_value_from_expression_info(value, kind))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(HintField {
        name: field.name.clone(),
        values,
    })
}

fn hint_value_from_expression_info(
    value: &HintValueInfo,
    kind: HintSectionKind,
) -> Result<HintValue, HintProgramError> {
    Ok(HintValue {
        operand: hint_operand_from_payload(&value.payload, kind)?,
        positions: value.positions.clone(),
    })
}

fn hint_operand_from_payload(
    payload: &HintPayload,
    kind: HintSectionKind,
) -> Result<HintOperand, HintProgramError> {
    match payload {
        HintPayload::Number { value } => Ok(HintOperand::Number(*value)),
        HintPayload::String { value } => Ok(HintOperand::String(value.clone())),
        HintPayload::Temporary { id, dimension } => Ok(HintOperand::Temporary {
            id: *id,
            dimension: if kind == HintSectionKind::Regular {
                *dimension
            } else {
                None
            },
        }),
        HintPayload::Commitment {
            id,
            row_offset_index,
            ..
        } if kind == HintSectionKind::Regular => Ok(HintOperand::Commitment {
            id: *id,
            row_offset_index: required_payload_field(*row_offset_index, "cm", "row_offset_index")?,
        }),
        HintPayload::Commitment { .. } => Err(invalid_operand_section("cm", kind)),
        HintPayload::CustomCommitment {
            id,
            commit_id,
            row_offset_index,
            ..
        } if kind == HintSectionKind::Regular => Ok(HintOperand::CustomCommitment {
            id: *id,
            row_offset_index: required_payload_field(
                *row_offset_index,
                "custom",
                "row_offset_index",
            )?,
            commit_id: required_payload_field(*commit_id, "custom", "commit_id")?,
        }),
        HintPayload::CustomCommitment { .. } => Err(invalid_operand_section("custom", kind)),
        HintPayload::Constant {
            id,
            row_offset_index,
            ..
        } if kind == HintSectionKind::Regular => Ok(HintOperand::Constant {
            id: *id,
            row_offset_index: required_payload_field(
                *row_offset_index,
                "const",
                "row_offset_index",
            )?,
        }),
        HintPayload::Constant { .. } => Err(invalid_operand_section("const", kind)),
        HintPayload::Challenge { id, .. } if kind == HintSectionKind::Regular => {
            Ok(HintOperand::Challenge { id: *id })
        }
        HintPayload::Challenge { .. } => Err(invalid_operand_section("challenge", kind)),
        HintPayload::Public { id, .. } => Ok(HintOperand::Public { id: *id }),
        HintPayload::AirGroupValue {
            id, air_group_id, ..
        } if kind == HintSectionKind::Regular => Ok(HintOperand::AirGroupValue { id: *id }),
        HintPayload::AirGroupValue {
            id, air_group_id, ..
        } => Ok(HintOperand::GroupValue {
            group_id: required_payload_field(*air_group_id, "airgroupvalue", "air_group_id")?,
            id: *id,
        }),
        HintPayload::AirValue { id, .. } if kind == HintSectionKind::Regular => {
            Ok(HintOperand::AirValue { id: *id })
        }
        HintPayload::AirValue { .. } => Err(invalid_operand_section("airvalue", kind)),
        HintPayload::ProofValue { id, .. } => Ok(HintOperand::ProofValue { id: *id }),
    }
}

fn required_payload_field<T>(
    value: Option<T>,
    op: &'static str,
    field: &'static str,
) -> Result<T, HintProgramError> {
    value.ok_or(HintProgramError::MissingOperandField { op, field })
}

fn parse_hint_program(
    bytes: &[u8],
    section_id: u32,
    kind: HintSectionKind,
) -> Result<HintProgram, HintProgramError> {
    let file = parse_sectioned_file(bytes, *b"chps", 1).map_err(HintProgramError::Sectioned)?;
    let section = file
        .sections
        .iter()
        .find(|section| section.id == section_id)
        .ok_or(HintProgramError::MissingHintSection { section_id })?;
    parse_hint_section(&section.data, kind)
}

fn encode_hint_program(
    program: &HintProgram,
    section_id: u32,
    kind: HintSectionKind,
) -> Result<Vec<u8>, HintProgramError> {
    let section = encode_hint_section(program, kind)?;
    let file = SectionedFile {
        kind: *b"chps",
        version: 1,
        sections: vec![SectionedSection {
            id: section_id,
            data: section,
        }],
    };
    encode_sectioned_file(&file).map_err(HintProgramError::Sectioned)
}

fn parse_hint_section(
    bytes: &[u8],
    kind: HintSectionKind,
) -> Result<HintProgram, HintProgramError> {
    let mut reader = Reader::new(bytes);
    let hint_count = u32_to_usize(reader.read_u32()?)?;
    if hint_count > reader.remaining_len() / HINT_MIN_BYTES {
        return Err(HintProgramError::LengthOverflow);
    }
    let mut hints = Vec::with_capacity(hint_count);

    for _ in 0..hint_count {
        let name = reader.read_string()?;
        let field_count = u32_to_usize(reader.read_u32()?)?;
        if field_count > reader.remaining_len() / FIELD_MIN_BYTES {
            return Err(HintProgramError::LengthOverflow);
        }
        let mut fields = Vec::with_capacity(field_count);

        for _ in 0..field_count {
            let name = reader.read_string()?;
            let value_count = u32_to_usize(reader.read_u32()?)?;
            if value_count > reader.remaining_len() / VALUE_MIN_BYTES {
                return Err(HintProgramError::LengthOverflow);
            }
            let mut values = Vec::with_capacity(value_count);

            for _ in 0..value_count {
                values.push(read_hint_value(&mut reader, kind)?);
            }

            fields.push(HintField { name, values });
        }

        hints.push(Hint { name, fields });
    }

    if reader.position() != bytes.len() {
        return Err(HintProgramError::UnexpectedTrailingBytes {
            count: bytes.len() - reader.position(),
        });
    }

    Ok(HintProgram { hints })
}

fn read_hint_value(
    reader: &mut Reader<'_>,
    kind: HintSectionKind,
) -> Result<HintValue, HintProgramError> {
    let op = reader.read_string()?;
    let operand = match op.as_str() {
        "number" => HintOperand::Number(reader.read_u64()?),
        "string" => HintOperand::String(reader.read_string()?),
        "airgroupvalue" => match kind {
            HintSectionKind::Global => HintOperand::GroupValue {
                group_id: reader.read_u32()?,
                id: reader.read_u32()?,
            },
            HintSectionKind::Regular => HintOperand::AirGroupValue {
                id: reader.read_u32()?,
            },
        },
        "airvalue" if kind == HintSectionKind::Regular => HintOperand::AirValue {
            id: reader.read_u32()?,
        },
        "challenge" if kind == HintSectionKind::Regular => HintOperand::Challenge {
            id: reader.read_u32()?,
        },
        "cm" if kind == HintSectionKind::Regular => {
            let id = reader.read_u32()?;
            let row_offset_index = reader.read_u32()?;
            HintOperand::Commitment {
                id,
                row_offset_index,
            }
        }
        "const" if kind == HintSectionKind::Regular => {
            let id = reader.read_u32()?;
            let row_offset_index = reader.read_u32()?;
            HintOperand::Constant {
                id,
                row_offset_index,
            }
        }
        "custom" if kind == HintSectionKind::Regular => {
            let id = reader.read_u32()?;
            let row_offset_index = reader.read_u32()?;
            let commit_id = reader.read_u32()?;
            HintOperand::CustomCommitment {
                id,
                row_offset_index,
                commit_id,
            }
        }
        "tmp" => {
            let id = reader.read_u32()?;
            let dimension = match kind {
                HintSectionKind::Regular => Some(reader.read_u32()?),
                HintSectionKind::Global => None,
            };
            HintOperand::Temporary { id, dimension }
        }
        "public" => HintOperand::Public {
            id: reader.read_u32()?,
        },
        "proofvalue" => HintOperand::ProofValue {
            id: reader.read_u32()?,
        },
        _ => return Err(HintProgramError::UnknownOperand { op }),
    };

    let position_count = u32_to_usize(reader.read_u32()?)?;
    if position_count > reader.remaining_len() / POSITION_BYTES {
        return Err(HintProgramError::LengthOverflow);
    }
    let mut positions = Vec::with_capacity(position_count);
    for _ in 0..position_count {
        positions.push(reader.read_u32()?);
    }

    Ok(HintValue { operand, positions })
}

fn encode_hint_section(
    program: &HintProgram,
    kind: HintSectionKind,
) -> Result<Vec<u8>, HintProgramError> {
    let mut out = Vec::new();
    write_u32(
        &mut out,
        u32::try_from(program.hints.len()).map_err(|_| HintProgramError::LengthOverflow)?,
    );

    for hint in &program.hints {
        write_string(&mut out, &hint.name)?;
        write_u32(
            &mut out,
            u32::try_from(hint.fields.len()).map_err(|_| HintProgramError::LengthOverflow)?,
        );

        for field in &hint.fields {
            write_string(&mut out, &field.name)?;
            write_u32(
                &mut out,
                u32::try_from(field.values.len()).map_err(|_| HintProgramError::LengthOverflow)?,
            );

            for value in &field.values {
                write_hint_value(&mut out, value, kind)?;
            }
        }
    }

    Ok(out)
}

fn write_hint_value(
    out: &mut Vec<u8>,
    value: &HintValue,
    kind: HintSectionKind,
) -> Result<(), HintProgramError> {
    match &value.operand {
        HintOperand::Number(number) => {
            write_string(out, "number")?;
            write_u64(out, *number);
        }
        HintOperand::String(string) => {
            write_string(out, "string")?;
            write_string(out, string)?;
        }
        HintOperand::GroupValue { group_id, id } if kind == HintSectionKind::Global => {
            write_string(out, "airgroupvalue")?;
            write_u32(out, *group_id);
            write_u32(out, *id);
        }
        HintOperand::GroupValue { .. } => {
            return Err(invalid_operand_section("airgroupvalue", kind));
        }
        HintOperand::AirGroupValue { id } if kind == HintSectionKind::Regular => {
            write_string(out, "airgroupvalue")?;
            write_u32(out, *id);
        }
        HintOperand::AirGroupValue { .. } => {
            return Err(invalid_operand_section("airgroupvalue", kind));
        }
        HintOperand::AirValue { id } if kind == HintSectionKind::Regular => {
            write_string(out, "airvalue")?;
            write_u32(out, *id);
        }
        HintOperand::AirValue { .. } => {
            return Err(invalid_operand_section("airvalue", kind));
        }
        HintOperand::Challenge { id } if kind == HintSectionKind::Regular => {
            write_string(out, "challenge")?;
            write_u32(out, *id);
        }
        HintOperand::Challenge { .. } => {
            return Err(invalid_operand_section("challenge", kind));
        }
        HintOperand::Commitment {
            id,
            row_offset_index,
        } if kind == HintSectionKind::Regular => {
            write_string(out, "cm")?;
            write_u32(out, *id);
            write_u32(out, *row_offset_index);
        }
        HintOperand::Commitment { .. } => {
            return Err(invalid_operand_section("cm", kind));
        }
        HintOperand::Constant {
            id,
            row_offset_index,
        } if kind == HintSectionKind::Regular => {
            write_string(out, "const")?;
            write_u32(out, *id);
            write_u32(out, *row_offset_index);
        }
        HintOperand::Constant { .. } => {
            return Err(invalid_operand_section("const", kind));
        }
        HintOperand::CustomCommitment {
            id,
            row_offset_index,
            commit_id,
        } if kind == HintSectionKind::Regular => {
            write_string(out, "custom")?;
            write_u32(out, *id);
            write_u32(out, *row_offset_index);
            write_u32(out, *commit_id);
        }
        HintOperand::CustomCommitment { .. } => {
            return Err(invalid_operand_section("custom", kind));
        }
        HintOperand::Temporary { id, dimension } => {
            write_string(out, "tmp")?;
            write_u32(out, *id);
            if kind == HintSectionKind::Regular {
                write_u32(out, dimension.unwrap_or(1));
            }
        }
        HintOperand::Public { id } => {
            write_string(out, "public")?;
            write_u32(out, *id);
        }
        HintOperand::ProofValue { id } => {
            write_string(out, "proofvalue")?;
            write_u32(out, *id);
        }
    }

    write_u32(
        out,
        u32::try_from(value.positions.len()).map_err(|_| HintProgramError::LengthOverflow)?,
    );
    for position in &value.positions {
        write_u32(out, *position);
    }
    Ok(())
}

fn invalid_operand_section(op: &'static str, kind: HintSectionKind) -> HintProgramError {
    HintProgramError::InvalidOperandSection {
        op,
        section: kind.name(),
    }
}

impl HintSectionKind {
    fn name(self) -> &'static str {
        match self {
            Self::Regular => "regular",
            Self::Global => "global",
        }
    }
}

fn write_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_string(out: &mut Vec<u8>, value: &str) -> Result<(), HintProgramError> {
    if value.as_bytes().contains(&0) {
        return Err(HintProgramError::StringContainsNul {
            value: value.to_owned(),
        });
    }
    out.extend_from_slice(value.as_bytes());
    out.push(0);
    Ok(())
}

fn u32_to_usize(value: u32) -> Result<usize, HintProgramError> {
    usize::try_from(value).map_err(|_| HintProgramError::LengthOverflow)
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

    fn read_exact(&mut self, count: usize) -> Result<&'a [u8], HintProgramError> {
        let end = self
            .offset
            .checked_add(count)
            .ok_or(HintProgramError::LengthOverflow)?;
        if end > self.bytes.len() {
            return Err(HintProgramError::UnexpectedEof {
                offset: self.offset,
                needed: count,
                available: self.bytes.len().saturating_sub(self.offset),
            });
        }
        let out = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(out)
    }

    fn read_u32(&mut self) -> Result<u32, HintProgramError> {
        let bytes = self.read_exact(4)?;
        Ok(u32::from_le_bytes(
            bytes.try_into().expect("slice length checked"),
        ))
    }

    fn read_u64(&mut self) -> Result<u64, HintProgramError> {
        let bytes = self.read_exact(8)?;
        Ok(u64::from_le_bytes(
            bytes.try_into().expect("slice length checked"),
        ))
    }

    fn read_string(&mut self) -> Result<String, HintProgramError> {
        let start = self.offset;
        let Some(relative_end) = self.bytes[start..].iter().position(|byte| *byte == 0) else {
            return Err(HintProgramError::MissingStringTerminator { offset: start });
        };
        let end = start
            .checked_add(relative_end)
            .ok_or(HintProgramError::LengthOverflow)?;
        self.offset = end + 1;
        String::from_utf8(self.bytes[start..end].to_vec())
            .map_err(|_| HintProgramError::InvalidUtf8)
    }
}
