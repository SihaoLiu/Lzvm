use std::fmt;
use std::path::Path;

use crate::sectioned::{
    encode_sectioned_file, parse_sectioned_file, SectionedError, SectionedFile, SectionedSection,
};
use crate::setup_info::UnitSetupInfo;

const FIELD_WORD_BYTES: u64 = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixedColumns {
    pub group_name: String,
    pub unit_name: String,
    pub row_count: u64,
    pub columns: Vec<FixedColumn>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixedColumn {
    pub name: String,
    pub dimensions: Vec<u32>,
    pub values: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FixedColumnError {
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
    InvalidSectionSize {
        expected: u64,
        available: usize,
    },
    UnexpectedTrailingBytes {
        count: usize,
    },
    UnexpectedEof {
        offset: usize,
        needed: usize,
        available: usize,
    },
    InvalidUtf8,
    MissingStringTerminator {
        offset: usize,
    },
    LengthOverflow,
    StringContainsNul {
        value: String,
    },
    ColumnValueCountMismatch {
        column: String,
        expected: u64,
        found: usize,
    },
    RawDomainTooLarge {
        n_bits: u32,
    },
    InvalidRawByteLength {
        expected: usize,
        found: usize,
    },
    RawColumnIndexOutOfBounds {
        column: String,
        index: u32,
        width: u32,
    },
    Io {
        message: String,
    },
}

impl fmt::Display for FixedColumnError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMagic => write!(f, "invalid fixed-column file magic"),
            Self::UnsupportedVersion { found, max } => {
                write!(f, "unsupported fixed-column file version {found}, max {max}")
            }
            Self::InvalidSectionCount { found } => {
                write!(f, "invalid fixed-column section count {found}")
            }
            Self::InvalidSectionId { found } => {
                write!(f, "invalid fixed-column section id {found}")
            }
            Self::InvalidSectionSize {
                expected,
                available,
            } => write!(
                f,
                "invalid fixed-column section size {expected}, available {available}"
            ),
            Self::UnexpectedTrailingBytes { count } => {
                write!(f, "unexpected trailing bytes in fixed-column file: {count}")
            }
            Self::UnexpectedEof {
                offset,
                needed,
                available,
            } => write!(
                f,
                "unexpected end of fixed-column file at {offset}, needed {needed}, available {available}"
            ),
            Self::InvalidUtf8 => write!(f, "fixed-column string is not valid utf-8"),
            Self::MissingStringTerminator { offset } => {
                write!(f, "missing string terminator at offset {offset}")
            }
            Self::LengthOverflow => write!(f, "fixed-column length overflow"),
            Self::StringContainsNul { value } => {
                write!(f, "fixed-column string contains nul byte: {value}")
            }
            Self::ColumnValueCountMismatch {
                column,
                expected,
                found,
            } => write!(
                f,
                "fixed-column value count mismatch for {column}: expected {expected}, found {found}"
            ),
            Self::RawDomainTooLarge { n_bits } => {
                write!(f, "raw fixed-column domain is too large: {n_bits}")
            }
            Self::InvalidRawByteLength { expected, found } => write!(
                f,
                "invalid raw fixed-column byte length: expected {expected}, found {found}"
            ),
            Self::RawColumnIndexOutOfBounds {
                column,
                index,
                width,
            } => write!(
                f,
                "raw fixed-column {column} index {index} is outside row width {width}"
            ),
            Self::Io { message } => write!(f, "fixed-column file io error: {message}"),
        }
    }
}

impl std::error::Error for FixedColumnError {}

impl From<SectionedError> for FixedColumnError {
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

pub fn read_fixed_columns_file(path: impl AsRef<Path>) -> Result<FixedColumns, FixedColumnError> {
    let bytes = std::fs::read(path).map_err(|error| FixedColumnError::Io {
        message: error.to_string(),
    })?;
    parse_fixed_columns(&bytes)
}

pub fn read_raw_fixed_columns_file(
    path: impl AsRef<Path>,
    setup: &UnitSetupInfo,
    group_name: impl Into<String>,
    unit_name: impl Into<String>,
) -> Result<FixedColumns, FixedColumnError> {
    let bytes = std::fs::read(path).map_err(|error| FixedColumnError::Io {
        message: error.to_string(),
    })?;
    parse_raw_fixed_columns(&bytes, setup, group_name, unit_name)
}

pub fn read_fixed_columns_file_for_setup(
    path: impl AsRef<Path>,
    setup: &UnitSetupInfo,
    group_name: impl Into<String>,
    unit_name: impl Into<String>,
) -> Result<FixedColumns, FixedColumnError> {
    let bytes = std::fs::read(path).map_err(|error| FixedColumnError::Io {
        message: error.to_string(),
    })?;
    match parse_fixed_columns(&bytes) {
        Ok(columns) => Ok(columns),
        Err(sectioned_error) => {
            if expected_raw_fixed_column_byte_count(setup).ok() == Some(bytes.len()) {
                parse_raw_fixed_columns(&bytes, setup, group_name, unit_name)
            } else {
                Err(sectioned_error)
            }
        }
    }
}

pub fn encode_fixed_columns(value: &FixedColumns) -> Result<Vec<u8>, FixedColumnError> {
    let section = encode_fixed_columns_section(value)?;
    let file = SectionedFile {
        kind: *b"cnst",
        version: 1,
        sections: vec![SectionedSection {
            id: 1,
            data: section,
        }],
    };
    encode_sectioned_file(&file).map_err(FixedColumnError::from)
}

fn encode_fixed_columns_section(value: &FixedColumns) -> Result<Vec<u8>, FixedColumnError> {
    let mut section = Vec::new();
    write_string(&mut section, &value.group_name)?;
    write_string(&mut section, &value.unit_name)?;
    write_u64(&mut section, value.row_count);
    let column_count =
        u32::try_from(value.columns.len()).map_err(|_| FixedColumnError::LengthOverflow)?;
    write_u32(&mut section, column_count);

    for column in &value.columns {
        if column.values.len()
            != usize::try_from(value.row_count).map_err(|_| FixedColumnError::LengthOverflow)?
        {
            return Err(FixedColumnError::ColumnValueCountMismatch {
                column: column.name.clone(),
                expected: value.row_count,
                found: column.values.len(),
            });
        }

        write_string(&mut section, &column.name)?;
        let dimension_count =
            u32::try_from(column.dimensions.len()).map_err(|_| FixedColumnError::LengthOverflow)?;
        write_u32(&mut section, dimension_count);
        for dimension in &column.dimensions {
            write_u32(&mut section, *dimension);
        }
        for entry in &column.values {
            write_u64(&mut section, *entry);
        }
    }

    Ok(section)
}

pub fn parse_fixed_columns(bytes: &[u8]) -> Result<FixedColumns, FixedColumnError> {
    let file = parse_sectioned_file(bytes, *b"cnst", 1).map_err(FixedColumnError::from)?;
    if file.sections.len() != 1 {
        return Err(FixedColumnError::InvalidSectionCount {
            found: u32::try_from(file.sections.len()).unwrap_or(u32::MAX),
        });
    }

    let section = &file.sections[0];
    if section.id != 1 {
        return Err(FixedColumnError::InvalidSectionId { found: section.id });
    }

    parse_fixed_columns_section(&section.data)
}

pub fn parse_raw_fixed_columns(
    bytes: &[u8],
    setup: &UnitSetupInfo,
    group_name: impl Into<String>,
    unit_name: impl Into<String>,
) -> Result<FixedColumns, FixedColumnError> {
    let expected = expected_raw_fixed_column_byte_count(setup)?;
    if bytes.len() != expected {
        return Err(FixedColumnError::InvalidRawByteLength {
            expected,
            found: bytes.len(),
        });
    }

    let row_count = raw_row_count(setup)?;
    let width = setup.n_constants;
    let row_count_usize =
        usize::try_from(row_count).map_err(|_| FixedColumnError::LengthOverflow)?;
    let width_usize = usize::try_from(width).map_err(|_| FixedColumnError::LengthOverflow)?;
    let column_specs = constant_column_specs(setup);
    let mut columns = Vec::with_capacity(column_specs.len());

    for spec in column_specs {
        if spec.index >= width {
            return Err(FixedColumnError::RawColumnIndexOutOfBounds {
                column: spec.name,
                index: spec.index,
                width,
            });
        }
        let index = usize::try_from(spec.index).map_err(|_| FixedColumnError::LengthOverflow)?;
        let mut values = Vec::with_capacity(row_count_usize);
        for row in 0..row_count_usize {
            let word_index = row
                .checked_mul(width_usize)
                .and_then(|offset| offset.checked_add(index))
                .ok_or(FixedColumnError::LengthOverflow)?;
            let byte_index = word_index
                .checked_mul(FIELD_WORD_BYTES as usize)
                .ok_or(FixedColumnError::LengthOverflow)?;
            let word = &bytes[byte_index..byte_index + FIELD_WORD_BYTES as usize];
            values.push(u64::from_le_bytes(
                word.try_into().expect("slice length checked"),
            ));
        }
        columns.push(FixedColumn {
            name: spec.name,
            dimensions: spec.dimensions,
            values,
        });
    }

    Ok(FixedColumns {
        group_name: group_name.into(),
        unit_name: unit_name.into(),
        row_count,
        columns,
    })
}

pub fn expected_raw_fixed_column_byte_count(
    setup: &UnitSetupInfo,
) -> Result<usize, FixedColumnError> {
    let words = raw_row_count(setup)?
        .checked_mul(u64::from(setup.n_constants))
        .ok_or(FixedColumnError::LengthOverflow)?;
    let bytes = words
        .checked_mul(FIELD_WORD_BYTES)
        .ok_or(FixedColumnError::LengthOverflow)?;
    usize::try_from(bytes).map_err(|_| FixedColumnError::LengthOverflow)
}

fn parse_fixed_columns_section(bytes: &[u8]) -> Result<FixedColumns, FixedColumnError> {
    let mut reader = Reader::new(bytes);
    let section_end = bytes.len();
    let group_name = reader.read_string()?;
    let unit_name = reader.read_string()?;
    let row_count = reader.read_u64()?;
    let column_count = reader.read_u32()?;
    let rows = usize::try_from(row_count).map_err(|_| FixedColumnError::LengthOverflow)?;
    let mut columns = Vec::with_capacity(column_count as usize);

    for _ in 0..column_count {
        let name = reader.read_string()?;
        let dimension_count = reader.read_u32()?;
        let mut dimensions = Vec::with_capacity(dimension_count as usize);
        for _ in 0..dimension_count {
            dimensions.push(reader.read_u32()?);
        }

        let mut values = Vec::with_capacity(rows);
        for _ in 0..rows {
            values.push(reader.read_u64()?);
        }

        columns.push(FixedColumn {
            name,
            dimensions,
            values,
        });
    }

    if reader.position() != section_end {
        if reader.position() > section_end {
            return Err(FixedColumnError::InvalidSectionSize {
                expected: section_end as u64,
                available: reader.position(),
            });
        }
        return Err(FixedColumnError::UnexpectedTrailingBytes {
            count: section_end - reader.position(),
        });
    }

    if section_end != bytes.len() {
        return Err(FixedColumnError::UnexpectedTrailingBytes {
            count: bytes.len() - section_end,
        });
    }

    Ok(FixedColumns {
        group_name,
        unit_name,
        row_count,
        columns,
    })
}

struct RawColumnSpec {
    name: String,
    index: u32,
    dimensions: Vec<u32>,
}

fn constant_column_specs(setup: &UnitSetupInfo) -> Vec<RawColumnSpec> {
    if setup.constant_columns.is_empty() {
        return (0..setup.n_constants)
            .map(|index| RawColumnSpec {
                name: format!("const_{index}"),
                index,
                dimensions: Vec::new(),
            })
            .collect();
    }

    setup
        .constant_columns
        .iter()
        .map(|column| {
            let dimensions = if column.lengths.is_empty() {
                vec![column.dimension]
            } else {
                column.lengths.clone()
            };
            RawColumnSpec {
                name: column.name.clone(),
                index: column.pols_map_id,
                dimensions,
            }
        })
        .collect()
}

fn raw_row_count(setup: &UnitSetupInfo) -> Result<u64, FixedColumnError> {
    1_u64
        .checked_shl(setup.stark.n_bits)
        .ok_or(FixedColumnError::RawDomainTooLarge {
            n_bits: setup.stark.n_bits,
        })
}

fn write_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_string(out: &mut Vec<u8>, value: &str) -> Result<(), FixedColumnError> {
    if value.as_bytes().contains(&0) {
        return Err(FixedColumnError::StringContainsNul {
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

    fn read_exact(&mut self, count: usize) -> Result<&'a [u8], FixedColumnError> {
        let end = self
            .offset
            .checked_add(count)
            .ok_or(FixedColumnError::LengthOverflow)?;
        if end > self.bytes.len() {
            return Err(FixedColumnError::UnexpectedEof {
                offset: self.offset,
                needed: count,
                available: self.bytes.len().saturating_sub(self.offset),
            });
        }
        let out = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(out)
    }

    fn read_u32(&mut self) -> Result<u32, FixedColumnError> {
        let bytes = self.read_exact(4)?;
        Ok(u32::from_le_bytes(
            bytes.try_into().expect("slice length checked"),
        ))
    }

    fn read_u64(&mut self) -> Result<u64, FixedColumnError> {
        let bytes = self.read_exact(8)?;
        Ok(u64::from_le_bytes(
            bytes.try_into().expect("slice length checked"),
        ))
    }

    fn read_string(&mut self) -> Result<String, FixedColumnError> {
        let start = self.offset;
        let Some(relative_end) = self.bytes[start..].iter().position(|byte| *byte == 0) else {
            return Err(FixedColumnError::MissingStringTerminator { offset: start });
        };
        let end = start
            .checked_add(relative_end)
            .ok_or(FixedColumnError::LengthOverflow)?;
        self.offset = end + 1;
        String::from_utf8(self.bytes[start..end].to_vec())
            .map_err(|_| FixedColumnError::InvalidUtf8)
    }
}
