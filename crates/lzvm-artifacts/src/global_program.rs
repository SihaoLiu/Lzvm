use std::fmt;
use std::path::Path;

use crate::constraint_program::{
    encode_global_constraint_program, parse_global_constraint_program, ConstraintProgramError,
    GlobalConstraintProgram,
};
use crate::hint_program::{
    encode_global_hint_program, parse_global_hint_program, HintProgram, HintProgramError,
};
use crate::sectioned::{
    encode_sectioned_file, parse_sectioned_file, SectionedError, SectionedFile,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobalProgram {
    pub constraints: GlobalConstraintProgram,
    pub hints: HintProgram,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GlobalProgramError {
    Constraints(ConstraintProgramError),
    Hints(HintProgramError),
    Sectioned(SectionedError),
    Io { message: String },
}

impl fmt::Display for GlobalProgramError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Constraints(error) => write!(f, "{error}"),
            Self::Hints(error) => write!(f, "{error}"),
            Self::Sectioned(error) => write!(f, "global program container error: {error}"),
            Self::Io { message } => write!(f, "global program io error: {message}"),
        }
    }
}

impl std::error::Error for GlobalProgramError {}

impl From<ConstraintProgramError> for GlobalProgramError {
    fn from(error: ConstraintProgramError) -> Self {
        Self::Constraints(error)
    }
}

impl From<HintProgramError> for GlobalProgramError {
    fn from(error: HintProgramError) -> Self {
        Self::Hints(error)
    }
}

impl From<SectionedError> for GlobalProgramError {
    fn from(error: SectionedError) -> Self {
        Self::Sectioned(error)
    }
}

pub fn read_global_program_file(
    path: impl AsRef<Path>,
) -> Result<GlobalProgram, GlobalProgramError> {
    let bytes = std::fs::read(path).map_err(|error| GlobalProgramError::Io {
        message: error.to_string(),
    })?;
    parse_global_program(&bytes)
}

pub fn parse_global_program(bytes: &[u8]) -> Result<GlobalProgram, GlobalProgramError> {
    Ok(GlobalProgram {
        constraints: parse_global_constraint_program(bytes)?,
        hints: parse_global_hint_program(bytes)?,
    })
}

pub fn encode_global_program(program: &GlobalProgram) -> Result<Vec<u8>, GlobalProgramError> {
    let constraints = encode_global_constraint_program(&program.constraints)?;
    let hints = encode_global_hint_program(&program.hints)?;

    let mut file = parse_sectioned_file(&constraints, *b"chps", 1)?;
    let hint_file = parse_sectioned_file(&hints, *b"chps", 1)?;
    file.sections.extend(hint_file.sections);

    encode_sectioned_file(&SectionedFile {
        kind: *b"chps",
        version: 1,
        sections: file.sections,
    })
    .map_err(Into::into)
}
