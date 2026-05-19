use std::fmt;
use std::path::PathBuf;

use lzvm_artifacts::fixed::{
    encode_fixed_columns, encode_raw_fixed_columns, read_fixed_columns_file, FixedColumn,
    FixedColumnError, FixedColumns,
};
use lzvm_artifacts::setup_info::{read_unit_setup_info_binary_file, SetupInfoError, UnitSetupInfo};
use lzvm_pil::{
    lex_source, ColumnInitializerKind, ColumnKind, LexError, SourceLoaderConfig, SourceProgram,
    SourceProgramError, SourceProgramLoader, SourceSpan, Token, TokenKind,
};

use crate::{publish_staging_bytes, write_staging_bytes, SetupError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFixedColumnsWriteRequest {
    pub working_dir: PathBuf,
    pub include_paths: Vec<PathBuf>,
    pub include_path_first: bool,
    pub main_file: PathBuf,
    pub setup_info_path: PathBuf,
    pub group_name: String,
    pub unit_name: String,
    pub output_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFixedColumnsWriteReport {
    pub output_path: PathBuf,
    pub bytes_written: u64,
    pub column_count: usize,
    pub row_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceFixedColumnsWriteError {
    SourceProgram(SourceProgramError),
    SetupInfo(SetupInfoError),
    FixedColumns(FixedColumnError),
    Lex {
        source_name: String,
        source_span: SourceSpan,
        source: LexError,
    },
    UnsupportedInitializer {
        source_name: String,
        column: String,
    },
    UnsupportedColumnShape {
        source_name: String,
        column: String,
    },
    UnexpectedSequenceToken {
        source_name: String,
        source_span: SourceSpan,
        token: String,
    },
    InvalidLiteral {
        source_name: String,
        source_span: SourceSpan,
        literal: String,
    },
    DomainSizeOverflow {
        n_bits: u32,
    },
    Setup(SetupError),
}

impl fmt::Display for SourceFixedColumnsWriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SourceProgram(error) => write!(f, "{error}"),
            Self::SetupInfo(error) => write!(f, "{error}"),
            Self::FixedColumns(error) => write!(f, "{error}"),
            Self::Lex {
                source_name,
                source_span,
                source,
            } => write!(
                f,
                "source fixed-column lexing failed in {source_name} at {}..{}: {source}",
                source_span.start, source_span.end
            ),
            Self::UnsupportedInitializer {
                source_name,
                column,
            } => write!(
                f,
                "unsupported fixed-column initializer for {column} in {source_name}"
            ),
            Self::UnsupportedColumnShape {
                source_name,
                column,
            } => write!(
                f,
                "unsupported fixed-column declaration shape for {column} in {source_name}"
            ),
            Self::UnexpectedSequenceToken {
                source_name,
                source_span,
                token,
            } => write!(
                f,
                "unexpected fixed-column sequence token {token} in {source_name} at {}..{}",
                source_span.start, source_span.end
            ),
            Self::InvalidLiteral {
                source_name,
                source_span,
                literal,
            } => write!(
                f,
                "invalid fixed-column literal {literal} in {source_name} at {}..{}",
                source_span.start, source_span.end
            ),
            Self::DomainSizeOverflow { n_bits } => {
                write!(
                    f,
                    "source fixed-column domain size overflows for n_bits {n_bits}"
                )
            }
            Self::Setup(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for SourceFixedColumnsWriteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::SourceProgram(error) => Some(error),
            Self::SetupInfo(error) => Some(error),
            Self::FixedColumns(error) => Some(error),
            Self::Lex { source, .. } => Some(source),
            Self::Setup(error) => Some(error),
            Self::UnsupportedInitializer { .. }
            | Self::UnsupportedColumnShape { .. }
            | Self::UnexpectedSequenceToken { .. }
            | Self::InvalidLiteral { .. }
            | Self::DomainSizeOverflow { .. } => None,
        }
    }
}

impl From<SetupInfoError> for SourceFixedColumnsWriteError {
    fn from(error: SetupInfoError) -> Self {
        Self::SetupInfo(error)
    }
}

impl From<FixedColumnError> for SourceFixedColumnsWriteError {
    fn from(error: FixedColumnError) -> Self {
        Self::FixedColumns(error)
    }
}

impl From<SetupError> for SourceFixedColumnsWriteError {
    fn from(error: SetupError) -> Self {
        Self::Setup(error)
    }
}

pub fn write_fixed_columns_from_source_file(
    request: &SourceFixedColumnsWriteRequest,
) -> Result<SourceFixedColumnsWriteReport, SourceFixedColumnsWriteError> {
    let setup = read_unit_setup_info_binary_file(&request.setup_info_path)?;
    let mut loader = SourceProgramLoader::new(SourceLoaderConfig {
        working_dir: request.working_dir.clone(),
        include_paths: request.include_paths.clone(),
        include_path_first: request.include_path_first,
    });
    let program = loader
        .load_main(&request.main_file)
        .map_err(SourceFixedColumnsWriteError::SourceProgram)?;
    let columns = fixed_columns_from_source_program(
        &program,
        &setup,
        &request.group_name,
        &request.unit_name,
    )?;
    let bytes = encode_fixed_columns(&columns)?;
    encode_raw_fixed_columns(&columns, &setup)?;
    let staging_path = write_staging_bytes(
        &request.output_path,
        &bytes,
        "write source fixed columns staging file",
    )?;
    read_fixed_columns_file(&staging_path)?;
    let bytes_written = publish_staging_bytes(
        &staging_path,
        &request.output_path,
        "publish source fixed columns",
    )?;

    Ok(SourceFixedColumnsWriteReport {
        output_path: request.output_path.clone(),
        bytes_written,
        column_count: columns.columns.len(),
        row_count: columns.row_count,
    })
}

fn fixed_columns_from_source_program(
    program: &SourceProgram,
    setup: &UnitSetupInfo,
    group_name: &str,
    unit_name: &str,
) -> Result<FixedColumns, SourceFixedColumnsWriteError> {
    let row_count = 1_u64.checked_shl(setup.stark.n_bits).ok_or(
        SourceFixedColumnsWriteError::DomainSizeOverflow {
            n_bits: setup.stark.n_bits,
        },
    )?;
    let mut columns = Vec::new();

    for module in &program.modules {
        for declaration in &module.columns {
            if declaration.kind != ColumnKind::Fixed {
                continue;
            }
            let Some(initializer) = declaration.initializer.as_ref() else {
                continue;
            };
            let Some(item) = declaration.items.first() else {
                continue;
            };
            if declaration.items.len() != 1 || item.template || !item.array_dims.is_empty() {
                return Err(SourceFixedColumnsWriteError::UnsupportedColumnShape {
                    source_name: declaration.source_name.clone(),
                    column: item.name.clone(),
                });
            }
            if initializer.kind != ColumnInitializerKind::Sequence {
                return Err(SourceFixedColumnsWriteError::UnsupportedInitializer {
                    source_name: declaration.source_name.clone(),
                    column: item.name.clone(),
                });
            }
            let source = &module.source.contents[initializer.span.start..initializer.span.end];
            let values =
                parse_literal_sequence(&declaration.source_name, initializer.span, source)?;
            columns.push(FixedColumn {
                name: item.name.clone(),
                dimensions: vec![1],
                values,
            });
        }
    }

    Ok(FixedColumns {
        group_name: group_name.to_owned(),
        unit_name: unit_name.to_owned(),
        row_count,
        columns,
    })
}

fn parse_literal_sequence(
    source_name: &str,
    source_span: SourceSpan,
    source: &str,
) -> Result<Vec<u64>, SourceFixedColumnsWriteError> {
    let tokens = lex_source(source).map_err(|source| SourceFixedColumnsWriteError::Lex {
        source_name: source_name.to_owned(),
        source_span,
        source,
    })?;
    let mut cursor = 0_usize;
    expect_token(
        &tokens,
        &mut cursor,
        TokenKind::LBracket,
        source_name,
        source_span,
    )?;
    let mut values = Vec::new();
    if tokens
        .get(cursor)
        .is_some_and(|token| token.kind == TokenKind::RBracket)
    {
        cursor += 1;
        expect_end(&tokens, cursor, source_name, source_span)?;
        return Ok(values);
    }
    loop {
        let token = tokens.get(cursor).ok_or_else(|| {
            SourceFixedColumnsWriteError::UnexpectedSequenceToken {
                source_name: source_name.to_owned(),
                source_span,
                token: "<end>".to_owned(),
            }
        })?;
        values.push(parse_literal_token(token, source_name, source_span)?);
        cursor += 1;

        match tokens.get(cursor).map(|token| token.kind) {
            Some(TokenKind::Comma) => cursor += 1,
            Some(TokenKind::RBracket) => {
                cursor += 1;
                break;
            }
            Some(_) => {
                let token = tokens[cursor].lexeme.clone();
                return Err(SourceFixedColumnsWriteError::UnexpectedSequenceToken {
                    source_name: source_name.to_owned(),
                    source_span,
                    token,
                });
            }
            None => {
                return Err(SourceFixedColumnsWriteError::UnexpectedSequenceToken {
                    source_name: source_name.to_owned(),
                    source_span,
                    token: "<end>".to_owned(),
                });
            }
        }
    }
    expect_end(&tokens, cursor, source_name, source_span)?;
    Ok(values)
}

fn expect_token(
    tokens: &[Token],
    cursor: &mut usize,
    kind: TokenKind,
    source_name: &str,
    source_span: SourceSpan,
) -> Result<(), SourceFixedColumnsWriteError> {
    match tokens.get(*cursor) {
        Some(token) if token.kind == kind => {
            *cursor += 1;
            Ok(())
        }
        Some(token) => Err(SourceFixedColumnsWriteError::UnexpectedSequenceToken {
            source_name: source_name.to_owned(),
            source_span,
            token: token.lexeme.clone(),
        }),
        None => Err(SourceFixedColumnsWriteError::UnexpectedSequenceToken {
            source_name: source_name.to_owned(),
            source_span,
            token: "<end>".to_owned(),
        }),
    }
}

fn expect_end(
    tokens: &[Token],
    cursor: usize,
    source_name: &str,
    source_span: SourceSpan,
) -> Result<(), SourceFixedColumnsWriteError> {
    if let Some(token) = tokens.get(cursor) {
        return Err(SourceFixedColumnsWriteError::UnexpectedSequenceToken {
            source_name: source_name.to_owned(),
            source_span,
            token: token.lexeme.clone(),
        });
    }
    Ok(())
}

fn parse_literal_token(
    token: &Token,
    source_name: &str,
    source_span: SourceSpan,
) -> Result<u64, SourceFixedColumnsWriteError> {
    match token.kind {
        TokenKind::Integer => token.lexeme.parse::<u64>(),
        TokenKind::HexInteger => u64::from_str_radix(
            token
                .lexeme
                .strip_prefix("0x")
                .or_else(|| token.lexeme.strip_prefix("0X"))
                .unwrap_or(&token.lexeme),
            16,
        ),
        _ => {
            return Err(SourceFixedColumnsWriteError::UnexpectedSequenceToken {
                source_name: source_name.to_owned(),
                source_span,
                token: token.lexeme.clone(),
            });
        }
    }
    .map_err(|_| SourceFixedColumnsWriteError::InvalidLiteral {
        source_name: source_name.to_owned(),
        source_span,
        literal: token.lexeme.clone(),
    })
}
