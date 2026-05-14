use std::fmt;
use std::path::Path;

use crate::sectioned::{
    encode_sectioned_file, parse_sectioned_file, SectionedError, SectionedFile, SectionedSection,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstraintProgram {
    pub entries: Vec<ConstraintEntry>,
    pub ops: Vec<u8>,
    pub args: Vec<u16>,
    pub numbers: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstraintEntry {
    pub stage: u32,
    pub destination_dimension: u32,
    pub destination_id: u32,
    pub first_row: u32,
    pub last_row: u32,
    pub temp1_count: u32,
    pub temp3_count: u32,
    pub ops_count: u32,
    pub ops_offset: u32,
    pub args_count: u32,
    pub args_offset: u32,
    pub intermediate: bool,
    pub source_line: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobalConstraintProgram {
    pub entries: Vec<GlobalConstraintEntry>,
    pub ops: Vec<u8>,
    pub args: Vec<u16>,
    pub numbers: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobalConstraintEntry {
    pub destination_dimension: u32,
    pub destination_id: u32,
    pub temp1_count: u32,
    pub temp3_count: u32,
    pub ops_count: u32,
    pub ops_offset: u32,
    pub args_count: u32,
    pub args_offset: u32,
    pub source_line: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConstraintProgramError {
    Sectioned(SectionedError),
    MissingConstraintSection {
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
    LengthOverflow,
    Io {
        message: String,
    },
    OperationSpanOutOfBounds {
        constraint_index: usize,
    },
    ArgumentSpanOutOfBounds {
        constraint_index: usize,
    },
}

impl fmt::Display for ConstraintProgramError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sectioned(error) => write!(f, "constraint program container error: {error}"),
            Self::MissingConstraintSection { section_id } => {
                write!(f, "missing constraint section {section_id}")
            }
            Self::UnexpectedTrailingBytes { count } => {
                write!(f, "unexpected trailing constraint program bytes: {count}")
            }
            Self::UnexpectedEof {
                offset,
                needed,
                available,
            } => write!(
                f,
                "unexpected end of constraint program at {offset}, needed {needed}, available {available}"
            ),
            Self::MissingStringTerminator { offset } => {
                write!(f, "missing constraint string terminator at offset {offset}")
            }
            Self::InvalidUtf8 => write!(f, "constraint string is not valid utf-8"),
            Self::StringContainsNul { value } => {
                write!(f, "constraint string contains nul byte: {value}")
            }
            Self::LengthOverflow => write!(f, "constraint program length overflow"),
            Self::Io { message } => write!(f, "constraint program io error: {message}"),
            Self::OperationSpanOutOfBounds { constraint_index } => write!(
                f,
                "operation span is out of bounds for constraint {constraint_index}"
            ),
            Self::ArgumentSpanOutOfBounds { constraint_index } => write!(
                f,
                "argument span is out of bounds for constraint {constraint_index}"
            ),
        }
    }
}

impl std::error::Error for ConstraintProgramError {}

pub fn read_regular_constraint_program_file(
    path: impl AsRef<Path>,
) -> Result<ConstraintProgram, ConstraintProgramError> {
    let bytes = read_file(path)?;
    parse_regular_constraint_program(&bytes)
}

pub fn read_global_constraint_program_file(
    path: impl AsRef<Path>,
) -> Result<GlobalConstraintProgram, ConstraintProgramError> {
    let bytes = read_file(path)?;
    parse_global_constraint_program(&bytes)
}

fn read_file(path: impl AsRef<Path>) -> Result<Vec<u8>, ConstraintProgramError> {
    std::fs::read(path).map_err(|error| ConstraintProgramError::Io {
        message: error.to_string(),
    })
}

pub fn parse_regular_constraint_program(
    bytes: &[u8],
) -> Result<ConstraintProgram, ConstraintProgramError> {
    let section = find_section(bytes, 2)?;
    parse_regular_section(&section)
}

pub fn encode_regular_constraint_program(
    program: &ConstraintProgram,
) -> Result<Vec<u8>, ConstraintProgramError> {
    validate_regular_spans(program)?;
    let section = encode_regular_section(program)?;
    wrap_section(2, section)
}

pub fn parse_global_constraint_program(
    bytes: &[u8],
) -> Result<GlobalConstraintProgram, ConstraintProgramError> {
    let section = find_section(bytes, 1)?;
    parse_global_section(&section)
}

pub fn encode_global_constraint_program(
    program: &GlobalConstraintProgram,
) -> Result<Vec<u8>, ConstraintProgramError> {
    validate_global_spans(program)?;
    let section = encode_global_section(program)?;
    wrap_section(1, section)
}

fn find_section(bytes: &[u8], section_id: u32) -> Result<Vec<u8>, ConstraintProgramError> {
    let file =
        parse_sectioned_file(bytes, *b"chps", 1).map_err(ConstraintProgramError::Sectioned)?;
    file.sections
        .into_iter()
        .find(|section| section.id == section_id)
        .map(|section| section.data)
        .ok_or(ConstraintProgramError::MissingConstraintSection { section_id })
}

fn wrap_section(section_id: u32, data: Vec<u8>) -> Result<Vec<u8>, ConstraintProgramError> {
    let file = SectionedFile {
        kind: *b"chps",
        version: 1,
        sections: vec![SectionedSection {
            id: section_id,
            data,
        }],
    };
    encode_sectioned_file(&file).map_err(ConstraintProgramError::Sectioned)
}

fn parse_regular_section(bytes: &[u8]) -> Result<ConstraintProgram, ConstraintProgramError> {
    let mut reader = Reader::new(bytes);
    let ops_len = reader.read_u32()?;
    let args_len = reader.read_u32()?;
    let numbers_len = reader.read_u32()?;
    let entry_count = reader.read_u32()?;

    let mut entries = Vec::with_capacity(entry_count as usize);
    for _ in 0..entry_count {
        entries.push(ConstraintEntry {
            stage: reader.read_u32()?,
            destination_dimension: reader.read_u32()?,
            destination_id: reader.read_u32()?,
            first_row: reader.read_u32()?,
            last_row: reader.read_u32()?,
            temp1_count: reader.read_u32()?,
            temp3_count: reader.read_u32()?,
            ops_count: reader.read_u32()?,
            ops_offset: reader.read_u32()?,
            args_count: reader.read_u32()?,
            args_offset: reader.read_u32()?,
            intermediate: reader.read_u32()? != 0,
            source_line: reader.read_string()?,
        });
    }

    let (ops, args, numbers) = read_buffers(&mut reader, ops_len, args_len, numbers_len)?;
    expect_reader_done(&reader, bytes.len())?;

    let program = ConstraintProgram {
        entries,
        ops,
        args,
        numbers,
    };
    validate_regular_spans(&program)?;
    Ok(program)
}

fn parse_global_section(bytes: &[u8]) -> Result<GlobalConstraintProgram, ConstraintProgramError> {
    let mut reader = Reader::new(bytes);
    let ops_len = reader.read_u32()?;
    let args_len = reader.read_u32()?;
    let numbers_len = reader.read_u32()?;
    let entry_count = reader.read_u32()?;

    let mut entries = Vec::with_capacity(entry_count as usize);
    for _ in 0..entry_count {
        entries.push(GlobalConstraintEntry {
            destination_dimension: reader.read_u32()?,
            destination_id: reader.read_u32()?,
            temp1_count: reader.read_u32()?,
            temp3_count: reader.read_u32()?,
            ops_count: reader.read_u32()?,
            ops_offset: reader.read_u32()?,
            args_count: reader.read_u32()?,
            args_offset: reader.read_u32()?,
            source_line: reader.read_string()?,
        });
    }

    let (ops, args, numbers) = read_buffers(&mut reader, ops_len, args_len, numbers_len)?;
    expect_reader_done(&reader, bytes.len())?;

    let program = GlobalConstraintProgram {
        entries,
        ops,
        args,
        numbers,
    };
    validate_global_spans(&program)?;
    Ok(program)
}

fn encode_regular_section(program: &ConstraintProgram) -> Result<Vec<u8>, ConstraintProgramError> {
    let mut out = Vec::new();
    write_buffer_header(
        &mut out,
        program.ops.len(),
        program.args.len(),
        program.numbers.len(),
    )?;
    write_u32(
        &mut out,
        u32::try_from(program.entries.len()).map_err(|_| ConstraintProgramError::LengthOverflow)?,
    );

    for entry in &program.entries {
        write_u32(&mut out, entry.stage);
        write_u32(&mut out, entry.destination_dimension);
        write_u32(&mut out, entry.destination_id);
        write_u32(&mut out, entry.first_row);
        write_u32(&mut out, entry.last_row);
        write_u32(&mut out, entry.temp1_count);
        write_u32(&mut out, entry.temp3_count);
        write_u32(&mut out, entry.ops_count);
        write_u32(&mut out, entry.ops_offset);
        write_u32(&mut out, entry.args_count);
        write_u32(&mut out, entry.args_offset);
        write_u32(&mut out, u32::from(entry.intermediate));
        write_string(&mut out, &entry.source_line)?;
    }

    write_buffers(&mut out, &program.ops, &program.args, &program.numbers);
    Ok(out)
}

fn encode_global_section(
    program: &GlobalConstraintProgram,
) -> Result<Vec<u8>, ConstraintProgramError> {
    let mut out = Vec::new();
    write_buffer_header(
        &mut out,
        program.ops.len(),
        program.args.len(),
        program.numbers.len(),
    )?;
    write_u32(
        &mut out,
        u32::try_from(program.entries.len()).map_err(|_| ConstraintProgramError::LengthOverflow)?,
    );

    for entry in &program.entries {
        write_u32(&mut out, entry.destination_dimension);
        write_u32(&mut out, entry.destination_id);
        write_u32(&mut out, entry.temp1_count);
        write_u32(&mut out, entry.temp3_count);
        write_u32(&mut out, entry.ops_count);
        write_u32(&mut out, entry.ops_offset);
        write_u32(&mut out, entry.args_count);
        write_u32(&mut out, entry.args_offset);
        write_string(&mut out, &entry.source_line)?;
    }

    write_buffers(&mut out, &program.ops, &program.args, &program.numbers);
    Ok(out)
}

fn read_buffers(
    reader: &mut Reader<'_>,
    ops_len: u32,
    args_len: u32,
    numbers_len: u32,
) -> Result<(Vec<u8>, Vec<u16>, Vec<u64>), ConstraintProgramError> {
    let ops_count = usize::try_from(ops_len).map_err(|_| ConstraintProgramError::LengthOverflow)?;
    let args_count =
        usize::try_from(args_len).map_err(|_| ConstraintProgramError::LengthOverflow)?;
    let numbers_count =
        usize::try_from(numbers_len).map_err(|_| ConstraintProgramError::LengthOverflow)?;

    let ops = reader.read_exact(ops_count)?.to_vec();
    let mut args = Vec::with_capacity(args_count);
    for _ in 0..args_count {
        args.push(reader.read_u16()?);
    }
    let mut numbers = Vec::with_capacity(numbers_count);
    for _ in 0..numbers_count {
        numbers.push(reader.read_u64()?);
    }
    Ok((ops, args, numbers))
}

fn write_buffer_header(
    out: &mut Vec<u8>,
    ops_len: usize,
    args_len: usize,
    numbers_len: usize,
) -> Result<(), ConstraintProgramError> {
    write_u32(
        out,
        u32::try_from(ops_len).map_err(|_| ConstraintProgramError::LengthOverflow)?,
    );
    write_u32(
        out,
        u32::try_from(args_len).map_err(|_| ConstraintProgramError::LengthOverflow)?,
    );
    write_u32(
        out,
        u32::try_from(numbers_len).map_err(|_| ConstraintProgramError::LengthOverflow)?,
    );
    Ok(())
}

fn write_buffers(out: &mut Vec<u8>, ops: &[u8], args: &[u16], numbers: &[u64]) {
    out.extend_from_slice(ops);
    for value in args {
        write_u16(out, *value);
    }
    for value in numbers {
        write_u64(out, *value);
    }
}

fn validate_regular_spans(program: &ConstraintProgram) -> Result<(), ConstraintProgramError> {
    for (index, entry) in program.entries.iter().enumerate() {
        validate_span(
            index,
            entry.ops_offset,
            entry.ops_count,
            program.ops.len(),
            true,
        )?;
        validate_span(
            index,
            entry.args_offset,
            entry.args_count,
            program.args.len(),
            false,
        )?;
    }
    Ok(())
}

fn validate_global_spans(program: &GlobalConstraintProgram) -> Result<(), ConstraintProgramError> {
    for (index, entry) in program.entries.iter().enumerate() {
        validate_span(
            index,
            entry.ops_offset,
            entry.ops_count,
            program.ops.len(),
            true,
        )?;
        validate_span(
            index,
            entry.args_offset,
            entry.args_count,
            program.args.len(),
            false,
        )?;
    }
    Ok(())
}

fn validate_span(
    constraint_index: usize,
    offset: u32,
    count: u32,
    len: usize,
    operation: bool,
) -> Result<(), ConstraintProgramError> {
    let offset = usize::try_from(offset).map_err(|_| ConstraintProgramError::LengthOverflow)?;
    let count = usize::try_from(count).map_err(|_| ConstraintProgramError::LengthOverflow)?;
    let end = offset
        .checked_add(count)
        .ok_or(ConstraintProgramError::LengthOverflow)?;
    if end > len {
        if operation {
            Err(ConstraintProgramError::OperationSpanOutOfBounds { constraint_index })
        } else {
            Err(ConstraintProgramError::ArgumentSpanOutOfBounds { constraint_index })
        }
    } else {
        Ok(())
    }
}

fn expect_reader_done(reader: &Reader<'_>, expected: usize) -> Result<(), ConstraintProgramError> {
    if reader.position() == expected {
        Ok(())
    } else {
        Err(ConstraintProgramError::UnexpectedTrailingBytes {
            count: expected - reader.position(),
        })
    }
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

fn write_string(out: &mut Vec<u8>, value: &str) -> Result<(), ConstraintProgramError> {
    if value.as_bytes().contains(&0) {
        return Err(ConstraintProgramError::StringContainsNul {
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

    fn read_exact(&mut self, count: usize) -> Result<&'a [u8], ConstraintProgramError> {
        let end = self
            .offset
            .checked_add(count)
            .ok_or(ConstraintProgramError::LengthOverflow)?;
        if end > self.bytes.len() {
            return Err(ConstraintProgramError::UnexpectedEof {
                offset: self.offset,
                needed: count,
                available: self.bytes.len().saturating_sub(self.offset),
            });
        }
        let out = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(out)
    }

    fn read_u16(&mut self) -> Result<u16, ConstraintProgramError> {
        let bytes = self.read_exact(2)?;
        Ok(u16::from_le_bytes(
            bytes.try_into().expect("slice length checked"),
        ))
    }

    fn read_u32(&mut self) -> Result<u32, ConstraintProgramError> {
        let bytes = self.read_exact(4)?;
        Ok(u32::from_le_bytes(
            bytes.try_into().expect("slice length checked"),
        ))
    }

    fn read_u64(&mut self) -> Result<u64, ConstraintProgramError> {
        let bytes = self.read_exact(8)?;
        Ok(u64::from_le_bytes(
            bytes.try_into().expect("slice length checked"),
        ))
    }

    fn read_string(&mut self) -> Result<String, ConstraintProgramError> {
        let start = self.offset;
        let Some(relative_end) = self.bytes[start..].iter().position(|byte| *byte == 0) else {
            return Err(ConstraintProgramError::MissingStringTerminator { offset: start });
        };
        let end = start
            .checked_add(relative_end)
            .ok_or(ConstraintProgramError::LengthOverflow)?;
        self.offset = end + 1;
        String::from_utf8(self.bytes[start..end].to_vec())
            .map_err(|_| ConstraintProgramError::InvalidUtf8)
    }
}
