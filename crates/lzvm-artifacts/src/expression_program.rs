use std::fmt;
use std::path::Path;

use crate::sectioned::{
    encode_sectioned_file, parse_sectioned_file, SectionedError, SectionedFile, SectionedSection,
};

const ENTRY_MIN_BYTES: usize = 10 * 4 + 1;
const ARG_BYTES: usize = 2;
const NUMBER_BYTES: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpressionProgram {
    pub max_tmp1: u32,
    pub max_tmp3: u32,
    pub max_args: u32,
    pub max_ops: u32,
    pub entries: Vec<ExpressionEntry>,
    pub ops: Vec<u8>,
    pub args: Vec<u16>,
    pub numbers: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpressionEntry {
    pub expression_id: u32,
    pub destination_dimension: u32,
    pub destination_id: u32,
    pub stage: u32,
    pub temp1_count: u32,
    pub temp3_count: u32,
    pub ops_count: u32,
    pub ops_offset: u32,
    pub args_count: u32,
    pub args_offset: u32,
    pub source_line: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpressionProgramError {
    Sectioned(SectionedError),
    MissingExpressionSection,
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
    LengthOverflow,
    Io {
        message: String,
    },
    OperationSpanOutOfBounds {
        expression_id: u32,
    },
    ArgumentSpanOutOfBounds {
        expression_id: u32,
    },
}

impl fmt::Display for ExpressionProgramError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sectioned(error) => write!(f, "expression program container error: {error}"),
            Self::MissingExpressionSection => write!(f, "missing expression program section"),
            Self::UnexpectedTrailingBytes { count } => {
                write!(f, "unexpected trailing expression program bytes: {count}")
            }
            Self::UnexpectedEof {
                offset,
                needed,
                available,
            } => write!(
                f,
                "unexpected end of expression program at {offset}, needed {needed}, available {available}"
            ),
            Self::MissingStringTerminator { offset } => {
                write!(f, "missing expression string terminator at offset {offset}")
            }
            Self::InvalidUtf8 => write!(f, "expression string is not valid utf-8"),
            Self::StringContainsNul { value } => {
                write!(f, "expression string contains nul byte: {value}")
            }
            Self::LengthOverflow => write!(f, "expression program length overflow"),
            Self::Io { message } => write!(f, "expression program io error: {message}"),
            Self::OperationSpanOutOfBounds { expression_id } => {
                write!(f, "operation span is out of bounds for expression {expression_id}")
            }
            Self::ArgumentSpanOutOfBounds { expression_id } => {
                write!(f, "argument span is out of bounds for expression {expression_id}")
            }
        }
    }
}

impl std::error::Error for ExpressionProgramError {}

pub fn read_expression_program_file(
    path: impl AsRef<Path>,
) -> Result<ExpressionProgram, ExpressionProgramError> {
    let bytes = std::fs::read(path).map_err(|error| ExpressionProgramError::Io {
        message: error.to_string(),
    })?;
    parse_expression_program(&bytes)
}

pub fn parse_expression_program(bytes: &[u8]) -> Result<ExpressionProgram, ExpressionProgramError> {
    let file =
        parse_sectioned_file(bytes, *b"chps", 1).map_err(ExpressionProgramError::Sectioned)?;
    let section = file
        .sections
        .iter()
        .find(|section| section.id == 1)
        .ok_or(ExpressionProgramError::MissingExpressionSection)?;

    parse_expression_section(&section.data)
}

pub fn encode_expression_program(
    program: &ExpressionProgram,
) -> Result<Vec<u8>, ExpressionProgramError> {
    validate_spans(program)?;
    let section = encode_expression_section(program)?;
    let file = SectionedFile {
        kind: *b"chps",
        version: 1,
        sections: vec![SectionedSection {
            id: 1,
            data: section,
        }],
    };
    encode_sectioned_file(&file).map_err(ExpressionProgramError::Sectioned)
}

fn parse_expression_section(bytes: &[u8]) -> Result<ExpressionProgram, ExpressionProgramError> {
    let mut reader = Reader::new(bytes);
    let max_tmp1 = reader.read_u32()?;
    let max_tmp3 = reader.read_u32()?;
    let max_args = reader.read_u32()?;
    let max_ops = reader.read_u32()?;
    let ops_len = reader.read_u32()?;
    let args_len = reader.read_u32()?;
    let numbers_len = reader.read_u32()?;
    let entry_count = u32_to_usize(reader.read_u32()?)?;

    if entry_count > reader.remaining_len() / ENTRY_MIN_BYTES {
        return Err(ExpressionProgramError::LengthOverflow);
    }
    let mut entries = Vec::with_capacity(entry_count);
    for _ in 0..entry_count {
        entries.push(ExpressionEntry {
            expression_id: reader.read_u32()?,
            destination_dimension: reader.read_u32()?,
            destination_id: reader.read_u32()?,
            stage: reader.read_u32()?,
            temp1_count: reader.read_u32()?,
            temp3_count: reader.read_u32()?,
            ops_count: reader.read_u32()?,
            ops_offset: reader.read_u32()?,
            args_count: reader.read_u32()?,
            args_offset: reader.read_u32()?,
            source_line: reader.read_string()?,
        });
    }

    let ops_count = usize::try_from(ops_len).map_err(|_| ExpressionProgramError::LengthOverflow)?;
    let args_count =
        usize::try_from(args_len).map_err(|_| ExpressionProgramError::LengthOverflow)?;
    let numbers_count =
        usize::try_from(numbers_len).map_err(|_| ExpressionProgramError::LengthOverflow)?;

    let ops = reader.read_exact(ops_count)?.to_vec();

    if args_count > reader.remaining_len() / ARG_BYTES {
        return Err(ExpressionProgramError::LengthOverflow);
    }
    let mut args = Vec::with_capacity(args_count);
    for _ in 0..args_count {
        args.push(reader.read_u16()?);
    }

    if numbers_count > reader.remaining_len() / NUMBER_BYTES {
        return Err(ExpressionProgramError::LengthOverflow);
    }
    let mut numbers = Vec::with_capacity(numbers_count);
    for _ in 0..numbers_count {
        numbers.push(reader.read_u64()?);
    }

    if reader.position() != bytes.len() {
        return Err(ExpressionProgramError::UnexpectedTrailingBytes {
            count: bytes.len() - reader.position(),
        });
    }

    let program = ExpressionProgram {
        max_tmp1,
        max_tmp3,
        max_args,
        max_ops,
        entries,
        ops,
        args,
        numbers,
    };
    validate_spans(&program)?;
    Ok(program)
}

fn encode_expression_section(
    program: &ExpressionProgram,
) -> Result<Vec<u8>, ExpressionProgramError> {
    let mut out = Vec::new();
    write_u32(&mut out, program.max_tmp1);
    write_u32(&mut out, program.max_tmp3);
    write_u32(&mut out, program.max_args);
    write_u32(&mut out, program.max_ops);
    write_u32(
        &mut out,
        u32::try_from(program.ops.len()).map_err(|_| ExpressionProgramError::LengthOverflow)?,
    );
    write_u32(
        &mut out,
        u32::try_from(program.args.len()).map_err(|_| ExpressionProgramError::LengthOverflow)?,
    );
    write_u32(
        &mut out,
        u32::try_from(program.numbers.len()).map_err(|_| ExpressionProgramError::LengthOverflow)?,
    );
    write_u32(
        &mut out,
        u32::try_from(program.entries.len()).map_err(|_| ExpressionProgramError::LengthOverflow)?,
    );

    for entry in &program.entries {
        write_u32(&mut out, entry.expression_id);
        write_u32(&mut out, entry.destination_dimension);
        write_u32(&mut out, entry.destination_id);
        write_u32(&mut out, entry.stage);
        write_u32(&mut out, entry.temp1_count);
        write_u32(&mut out, entry.temp3_count);
        write_u32(&mut out, entry.ops_count);
        write_u32(&mut out, entry.ops_offset);
        write_u32(&mut out, entry.args_count);
        write_u32(&mut out, entry.args_offset);
        write_string(&mut out, &entry.source_line)?;
    }

    out.extend_from_slice(&program.ops);
    for value in &program.args {
        write_u16(&mut out, *value);
    }
    for value in &program.numbers {
        write_u64(&mut out, *value);
    }

    Ok(out)
}

fn validate_spans(program: &ExpressionProgram) -> Result<(), ExpressionProgramError> {
    for entry in &program.entries {
        let ops_offset = usize::try_from(entry.ops_offset)
            .map_err(|_| ExpressionProgramError::LengthOverflow)?;
        let ops_count =
            usize::try_from(entry.ops_count).map_err(|_| ExpressionProgramError::LengthOverflow)?;
        let ops_end = ops_offset
            .checked_add(ops_count)
            .ok_or(ExpressionProgramError::LengthOverflow)?;
        if ops_end > program.ops.len() {
            return Err(ExpressionProgramError::OperationSpanOutOfBounds {
                expression_id: entry.expression_id,
            });
        }

        let args_offset = usize::try_from(entry.args_offset)
            .map_err(|_| ExpressionProgramError::LengthOverflow)?;
        let args_count = usize::try_from(entry.args_count)
            .map_err(|_| ExpressionProgramError::LengthOverflow)?;
        let args_end = args_offset
            .checked_add(args_count)
            .ok_or(ExpressionProgramError::LengthOverflow)?;
        if args_end > program.args.len() {
            return Err(ExpressionProgramError::ArgumentSpanOutOfBounds {
                expression_id: entry.expression_id,
            });
        }
    }
    Ok(())
}

fn write_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_string(out: &mut Vec<u8>, value: &str) -> Result<(), ExpressionProgramError> {
    if value.as_bytes().contains(&0) {
        return Err(ExpressionProgramError::StringContainsNul {
            value: value.to_owned(),
        });
    }
    out.extend_from_slice(value.as_bytes());
    out.push(0);
    Ok(())
}

fn u32_to_usize(value: u32) -> Result<usize, ExpressionProgramError> {
    usize::try_from(value).map_err(|_| ExpressionProgramError::LengthOverflow)
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

    fn read_exact(&mut self, count: usize) -> Result<&'a [u8], ExpressionProgramError> {
        let end = self
            .offset
            .checked_add(count)
            .ok_or(ExpressionProgramError::LengthOverflow)?;
        if end > self.bytes.len() {
            return Err(ExpressionProgramError::UnexpectedEof {
                offset: self.offset,
                needed: count,
                available: self.bytes.len().saturating_sub(self.offset),
            });
        }
        let out = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(out)
    }

    fn read_u16(&mut self) -> Result<u16, ExpressionProgramError> {
        let bytes = self.read_exact(2)?;
        Ok(u16::from_le_bytes(
            bytes.try_into().expect("slice length checked"),
        ))
    }

    fn read_u32(&mut self) -> Result<u32, ExpressionProgramError> {
        let bytes = self.read_exact(4)?;
        Ok(u32::from_le_bytes(
            bytes.try_into().expect("slice length checked"),
        ))
    }

    fn read_u64(&mut self) -> Result<u64, ExpressionProgramError> {
        let bytes = self.read_exact(8)?;
        Ok(u64::from_le_bytes(
            bytes.try_into().expect("slice length checked"),
        ))
    }

    fn read_string(&mut self) -> Result<String, ExpressionProgramError> {
        let start = self.offset;
        let Some(relative_end) = self.bytes[start..].iter().position(|byte| *byte == 0) else {
            return Err(ExpressionProgramError::MissingStringTerminator { offset: start });
        };
        let end = start
            .checked_add(relative_end)
            .ok_or(ExpressionProgramError::LengthOverflow)?;
        self.offset = end + 1;
        String::from_utf8(self.bytes[start..end].to_vec())
            .map_err(|_| ExpressionProgramError::InvalidUtf8)
    }
}
