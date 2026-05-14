use std::fmt;
use std::path::Path;

use crate::sectioned::{
    encode_sectioned_file, parse_sectioned_file, SectionedError, SectionedFile, SectionedSection,
};

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
    GroupValue { group_id: u32, id: u32 },
    Temporary { id: u32 },
    Public { id: u32 },
    ProofValue { id: u32 },
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
    LengthOverflow,
    Io {
        message: String,
    },
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
    parse_hint_program(bytes, 3)
}

pub fn encode_regular_hint_program(program: &HintProgram) -> Result<Vec<u8>, HintProgramError> {
    encode_hint_program(program, 3)
}

pub fn parse_global_hint_program(bytes: &[u8]) -> Result<HintProgram, HintProgramError> {
    parse_hint_program(bytes, 2)
}

pub fn encode_global_hint_program(program: &HintProgram) -> Result<Vec<u8>, HintProgramError> {
    encode_hint_program(program, 2)
}

fn parse_hint_program(bytes: &[u8], section_id: u32) -> Result<HintProgram, HintProgramError> {
    let file = parse_sectioned_file(bytes, *b"chps", 1).map_err(HintProgramError::Sectioned)?;
    let section = file
        .sections
        .iter()
        .find(|section| section.id == section_id)
        .ok_or(HintProgramError::MissingHintSection { section_id })?;
    parse_hint_section(&section.data)
}

fn encode_hint_program(
    program: &HintProgram,
    section_id: u32,
) -> Result<Vec<u8>, HintProgramError> {
    let section = encode_hint_section(program)?;
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

fn parse_hint_section(bytes: &[u8]) -> Result<HintProgram, HintProgramError> {
    let mut reader = Reader::new(bytes);
    let hint_count = reader.read_u32()?;
    let mut hints = Vec::with_capacity(hint_count as usize);

    for _ in 0..hint_count {
        let name = reader.read_string()?;
        let field_count = reader.read_u32()?;
        let mut fields = Vec::with_capacity(field_count as usize);

        for _ in 0..field_count {
            let name = reader.read_string()?;
            let value_count = reader.read_u32()?;
            let mut values = Vec::with_capacity(value_count as usize);

            for _ in 0..value_count {
                values.push(read_hint_value(&mut reader)?);
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

fn read_hint_value(reader: &mut Reader<'_>) -> Result<HintValue, HintProgramError> {
    let op = reader.read_string()?;
    let operand = match op.as_str() {
        "number" => HintOperand::Number(reader.read_u64()?),
        "string" => HintOperand::String(reader.read_string()?),
        "airgroupvalue" => HintOperand::GroupValue {
            group_id: reader.read_u32()?,
            id: reader.read_u32()?,
        },
        "tmp" => HintOperand::Temporary {
            id: reader.read_u32()?,
        },
        "public" => HintOperand::Public {
            id: reader.read_u32()?,
        },
        "proofvalue" => HintOperand::ProofValue {
            id: reader.read_u32()?,
        },
        _ => return Err(HintProgramError::UnknownOperand { op }),
    };

    let position_count = reader.read_u32()?;
    let mut positions = Vec::with_capacity(position_count as usize);
    for _ in 0..position_count {
        positions.push(reader.read_u32()?);
    }

    Ok(HintValue { operand, positions })
}

fn encode_hint_section(program: &HintProgram) -> Result<Vec<u8>, HintProgramError> {
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
                write_hint_value(&mut out, value)?;
            }
        }
    }

    Ok(out)
}

fn write_hint_value(out: &mut Vec<u8>, value: &HintValue) -> Result<(), HintProgramError> {
    match &value.operand {
        HintOperand::Number(number) => {
            write_string(out, "number")?;
            write_u64(out, *number);
        }
        HintOperand::String(string) => {
            write_string(out, "string")?;
            write_string(out, string)?;
        }
        HintOperand::GroupValue { group_id, id } => {
            write_string(out, "airgroupvalue")?;
            write_u32(out, *group_id);
            write_u32(out, *id);
        }
        HintOperand::Temporary { id } => {
            write_string(out, "tmp")?;
            write_u32(out, *id);
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
