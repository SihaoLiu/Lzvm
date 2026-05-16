use std::fmt;
use std::path::Path;

use crate::constraint_program::{
    encode_regular_constraint_program, parse_regular_constraint_program, ConstraintProgram,
    ConstraintProgramError,
};
use crate::expression_program::{
    encode_expression_program, parse_expression_program, ExpressionProgram, ExpressionProgramError,
};
use crate::hint_program::{
    encode_regular_hint_program, parse_regular_hint_program, HintProgram, HintProgramError,
};
use crate::sectioned::{
    encode_sectioned_file, parse_sectioned_file, SectionedError, SectionedFile,
};

pub use crate::regular_lowering::{
    regular_program_from_expression_info, RegularProgramLoweringError,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegularProgram {
    pub expressions: ExpressionProgram,
    pub constraints: ConstraintProgram,
    pub hints: HintProgram,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegularProgramError {
    Expressions(ExpressionProgramError),
    Constraints(ConstraintProgramError),
    Hints(HintProgramError),
    Sectioned(SectionedError),
    Io { message: String },
}

impl fmt::Display for RegularProgramError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Expressions(error) => write!(f, "{error}"),
            Self::Constraints(error) => write!(f, "{error}"),
            Self::Hints(error) => write!(f, "{error}"),
            Self::Sectioned(error) => write!(f, "regular program container error: {error}"),
            Self::Io { message } => write!(f, "regular program io error: {message}"),
        }
    }
}

impl std::error::Error for RegularProgramError {}

impl From<ExpressionProgramError> for RegularProgramError {
    fn from(error: ExpressionProgramError) -> Self {
        Self::Expressions(error)
    }
}

impl From<ConstraintProgramError> for RegularProgramError {
    fn from(error: ConstraintProgramError) -> Self {
        Self::Constraints(error)
    }
}

impl From<HintProgramError> for RegularProgramError {
    fn from(error: HintProgramError) -> Self {
        Self::Hints(error)
    }
}

impl From<SectionedError> for RegularProgramError {
    fn from(error: SectionedError) -> Self {
        Self::Sectioned(error)
    }
}

pub fn read_regular_program_file(
    path: impl AsRef<Path>,
) -> Result<RegularProgram, RegularProgramError> {
    let bytes = std::fs::read(path).map_err(|error| RegularProgramError::Io {
        message: error.to_string(),
    })?;
    parse_regular_program(&bytes)
}

pub fn parse_regular_program(bytes: &[u8]) -> Result<RegularProgram, RegularProgramError> {
    Ok(RegularProgram {
        expressions: parse_expression_program(bytes)?,
        constraints: parse_regular_constraint_program(bytes)?,
        hints: parse_regular_hint_program(bytes)?,
    })
}

pub fn encode_regular_program(program: &RegularProgram) -> Result<Vec<u8>, RegularProgramError> {
    let expressions = encode_expression_program(&program.expressions)?;
    let constraints = encode_regular_constraint_program(&program.constraints)?;
    let hints = encode_regular_hint_program(&program.hints)?;

    let mut file = parse_sectioned_file(&expressions, *b"chps", 1)?;
    let constraint_file = parse_sectioned_file(&constraints, *b"chps", 1)?;
    let hint_file = parse_sectioned_file(&hints, *b"chps", 1)?;
    file.sections.extend(constraint_file.sections);
    file.sections.extend(hint_file.sections);

    encode_sectioned_file(&SectionedFile {
        kind: *b"chps",
        version: 1,
        sections: file.sections,
    })
    .map_err(Into::into)
}
