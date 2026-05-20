use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::PathBuf;

use lzvm_artifacts::fixed::{
    encode_fixed_columns, encode_raw_fixed_columns, raw_fixed_column_layout,
    read_fixed_columns_file, FixedColumn, FixedColumnError, FixedColumns,
};
use lzvm_artifacts::global_info::GlobalInfo;
use lzvm_artifacts::key_directory::{read_key_directory_layout, KeyDirectoryError, KeyUnitKind};
use lzvm_artifacts::setup_info::{read_unit_setup_info_binary_file, SetupInfoError, UnitSetupInfo};
use lzvm_pil::{
    evaluate_fixed_file_template_value_expression_with_values, ColumnInitializer,
    ColumnInitializerKind, ColumnItem, ColumnKind, ConstantDeclaration, FixedFileTemplateValue,
    LexError, ParseError, SourceLoaderConfig, SourceProgram, SourceProgramError,
    SourceProgramLoader, SourceSpan,
};

use crate::{
    publish_staging_bytes,
    source_fixed_expression::source_fixed_column_expression_values,
    source_fixed_expression::SourceFixedConstantValues,
    source_fixed_sequence::{
        canonical_fixed_value, parse_literal_sequence, parse_literal_sequence_values,
    },
    write_staging_bytes, SetupError,
};

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
pub struct SourceFixedColumnsDirectoryWriteRequest {
    pub working_dir: PathBuf,
    pub include_paths: Vec<PathBuf>,
    pub include_path_first: bool,
    pub main_file: PathBuf,
    pub setup_dir: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFixedColumnsDirectoryWriteReport {
    pub setup_dir: PathBuf,
    pub unit_count: usize,
    pub bytes_written: u64,
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
    ExpressionParse {
        source_name: String,
        source_span: SourceSpan,
        source: ParseError,
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
    UnsupportedExpression {
        source_name: String,
        source_span: SourceSpan,
        expression: String,
    },
    IntegerOutOfRange {
        source_name: String,
        source_span: SourceSpan,
        expression: String,
    },
    DomainSizeOverflow {
        n_bits: u32,
    },
    Setup(SetupError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceFixedColumnsDirectoryWriteError {
    KeyDirectory(KeyDirectoryError),
    FixedColumns(SourceFixedColumnsWriteError),
    MissingUnitPath {
        role: &'static str,
        unit: KeyUnitKind,
    },
    SourceAirUnitMismatch {
        message: String,
    },
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
            Self::ExpressionParse {
                source_name,
                source_span,
                source,
            } => write!(
                f,
                "source fixed-column expression parse failed in {source_name} at {}..{}: {source}",
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
            Self::UnsupportedExpression {
                source_name,
                source_span,
                expression,
            } => write!(
                f,
                "unsupported fixed-column expression {expression} in {source_name} at {}..{}",
                source_span.start, source_span.end
            ),
            Self::IntegerOutOfRange {
                source_name,
                source_span,
                expression,
            } => write!(
                f,
                "fixed-column expression {expression} is out of range in {source_name} at {}..{}",
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

impl fmt::Display for SourceFixedColumnsDirectoryWriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::KeyDirectory(error) => write!(f, "{error}"),
            Self::FixedColumns(error) => write!(f, "{error}"),
            Self::MissingUnitPath { role, unit } => {
                write!(f, "missing source fixed-column {role} for {unit}")
            }
            Self::SourceAirUnitMismatch { message } => {
                write!(f, "source fixed-column AIR unit mismatch: {message}")
            }
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
            Self::ExpressionParse { source, .. } => Some(source),
            Self::Setup(error) => Some(error),
            Self::UnsupportedInitializer { .. }
            | Self::UnsupportedColumnShape { .. }
            | Self::UnexpectedSequenceToken { .. }
            | Self::InvalidLiteral { .. }
            | Self::UnsupportedExpression { .. }
            | Self::IntegerOutOfRange { .. }
            | Self::DomainSizeOverflow { .. } => None,
        }
    }
}

impl std::error::Error for SourceFixedColumnsDirectoryWriteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::KeyDirectory(error) => Some(error),
            Self::FixedColumns(error) => Some(error),
            Self::MissingUnitPath { .. } | Self::SourceAirUnitMismatch { .. } => None,
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

impl From<KeyDirectoryError> for SourceFixedColumnsDirectoryWriteError {
    fn from(error: KeyDirectoryError) -> Self {
        Self::KeyDirectory(error)
    }
}

impl From<SourceFixedColumnsWriteError> for SourceFixedColumnsDirectoryWriteError {
    fn from(error: SourceFixedColumnsWriteError) -> Self {
        Self::FixedColumns(error)
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
    write_fixed_columns_from_source_program(
        &program,
        &setup,
        &request.group_name,
        &request.unit_name,
        request.output_path.clone(),
    )
}

pub fn write_fixed_columns_from_source_directory(
    request: &SourceFixedColumnsDirectoryWriteRequest,
) -> Result<SourceFixedColumnsDirectoryWriteReport, SourceFixedColumnsDirectoryWriteError> {
    let layout = read_key_directory_layout(&request.setup_dir)?;
    let mut loader = SourceProgramLoader::new(SourceLoaderConfig {
        working_dir: request.working_dir.clone(),
        include_paths: request.include_paths.clone(),
        include_path_first: request.include_path_first,
    });
    let program = loader
        .load_main(&request.main_file)
        .map_err(SourceFixedColumnsWriteError::SourceProgram)?;
    validate_source_air_units(&program, &layout.global_info)?;

    let mut bytes_written = 0_u64;
    for unit in &layout.units {
        let setup_info_path =
            unit.setup_info()
                .ok_or(SourceFixedColumnsDirectoryWriteError::MissingUnitPath {
                    role: "setup metadata path",
                    unit: unit.kind,
                })?;
        let setup = read_unit_setup_info_binary_file(&setup_info_path)
            .map_err(SourceFixedColumnsWriteError::from)?;
        let group_name = unit.group_name.as_deref().unwrap_or("raw");
        let unit_name = unit.unit_name.as_deref().unwrap_or("unit");
        let report = write_fixed_columns_from_source_program(
            &program,
            &setup,
            group_name,
            unit_name,
            unit.fixed_columns.clone(),
        )?;
        bytes_written = bytes_written.saturating_add(report.bytes_written);
    }

    Ok(SourceFixedColumnsDirectoryWriteReport {
        setup_dir: request.setup_dir.clone(),
        unit_count: layout.units.len(),
        bytes_written,
    })
}

fn validate_source_air_units(
    program: &SourceProgram,
    global_info: &GlobalInfo,
) -> Result<(), SourceFixedColumnsDirectoryWriteError> {
    let expected_unit_count = global_info.airs.iter().map(Vec::len).sum::<usize>();
    let source_units = program
        .air_units()
        .into_iter()
        .filter(|unit| !unit.virtual_instance)
        .collect::<Vec<_>>();
    if source_units.len() != expected_unit_count {
        return Err(
            SourceFixedColumnsDirectoryWriteError::SourceAirUnitMismatch {
                message: format!(
                    "expected {expected_unit_count} units, found {}",
                    source_units.len()
                ),
            },
        );
    }

    for unit in source_units {
        let group_id = usize::try_from(unit.group_id).map_err(|_| {
            SourceFixedColumnsDirectoryWriteError::SourceAirUnitMismatch {
                message: format!("source group id is negative for {}", unit.group_name),
            }
        })?;
        let Some(group_name) = global_info.air_groups.get(group_id) else {
            return Err(
                SourceFixedColumnsDirectoryWriteError::SourceAirUnitMismatch {
                    message: format!(
                        "source group {}:{} is outside setup metadata",
                        unit.group_id, unit.group_name
                    ),
                },
            );
        };
        if group_name != &unit.group_name {
            return Err(
                SourceFixedColumnsDirectoryWriteError::SourceAirUnitMismatch {
                    message: format!(
                        "source group {}:{} does not match setup group {group_name}",
                        unit.group_id, unit.group_name
                    ),
                },
            );
        }

        let unit_id = usize::try_from(unit.unit_id).map_err(|_| {
            SourceFixedColumnsDirectoryWriteError::SourceAirUnitMismatch {
                message: format!("source unit id is negative for {}", unit.unit_name),
            }
        })?;
        let Some(expected_unit) = global_info
            .airs
            .get(group_id)
            .and_then(|group| group.get(unit_id))
        else {
            return Err(
                SourceFixedColumnsDirectoryWriteError::SourceAirUnitMismatch {
                    message: format!(
                        "source unit {}:{}:{} is outside setup metadata",
                        unit.group_id, unit.unit_id, unit.unit_name
                    ),
                },
            );
        };
        if expected_unit.name != unit.unit_name {
            return Err(
                SourceFixedColumnsDirectoryWriteError::SourceAirUnitMismatch {
                    message: format!(
                        "source unit {}:{}:{} does not match setup unit {}",
                        unit.group_id, unit.unit_id, unit.unit_name, expected_unit.name
                    ),
                },
            );
        }
    }

    Ok(())
}

fn write_fixed_columns_from_source_program(
    program: &SourceProgram,
    setup: &UnitSetupInfo,
    group_name: &str,
    unit_name: &str,
    output_path: PathBuf,
) -> Result<SourceFixedColumnsWriteReport, SourceFixedColumnsWriteError> {
    let columns = fixed_columns_from_source_program(program, setup, group_name, unit_name)?;
    let bytes = encode_fixed_columns(&columns)?;
    encode_raw_fixed_columns(&columns, setup)?;
    let staging_path = write_staging_bytes(
        &output_path,
        &bytes,
        "write source fixed columns staging file",
    )?;
    read_fixed_columns_file(&staging_path)?;
    let bytes_written =
        publish_staging_bytes(&staging_path, &output_path, "publish source fixed columns")?;

    Ok(SourceFixedColumnsWriteReport {
        output_path,
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
    let row_count_usize = usize::try_from(row_count).map_err(|_| {
        SourceFixedColumnsWriteError::DomainSizeOverflow {
            n_bits: setup.stark.n_bits,
        }
    })?;
    let expected_columns = raw_fixed_column_layout(setup, group_name, unit_name)?
        .columns
        .into_iter()
        .map(|column| column.name)
        .collect::<BTreeSet<_>>();
    let mut declarations = Vec::<(String, ColumnItem, ColumnInitializer, Vec<u32>, String)>::new();
    let mut column_values = BTreeMap::new();
    let constant_values = source_fixed_constant_values(program)?;

    for module in &program.modules {
        for declaration in &module.columns {
            if declaration.kind != ColumnKind::Fixed {
                continue;
            }
            if !declaration
                .items
                .iter()
                .any(|item| expected_columns.contains(&item.name))
            {
                continue;
            }
            let Some(initializer) = declaration.initializer.as_ref() else {
                continue;
            };
            let Some(item) = declaration.items.first() else {
                continue;
            };
            if !expected_columns.contains(&item.name) {
                continue;
            }
            if declaration.items.len() != 1 || item.template {
                return Err(SourceFixedColumnsWriteError::UnsupportedColumnShape {
                    source_name: declaration.source_name.clone(),
                    column: item.name.clone(),
                });
            }
            let dimensions = source_fixed_column_dimensions(
                &declaration.source_name,
                &module.source.contents,
                item,
                &constant_values,
            )?;
            declarations.push((
                declaration.source_name.clone(),
                item.clone(),
                initializer.clone(),
                dimensions,
                module.source.contents.clone(),
            ));
        }
    }

    let mut resolved_values = vec![None::<Vec<u64>>; declarations.len()];
    loop {
        let mut progressed = false;
        for (index, (source_name, item, initializer, _, source)) in declarations.iter().enumerate()
        {
            if resolved_values[index].is_some() {
                continue;
            }
            let Some(values) = source_fixed_column_values_from_initializer(
                source_name,
                source,
                &item.name,
                initializer,
                row_count_usize,
                &constant_values,
                &column_values,
            )?
            else {
                continue;
            };
            column_values.insert(item.name.clone(), values.clone());
            resolved_values[index] = Some(values);
            progressed = true;
        }
        if !progressed {
            break;
        }
    }

    for ((source_name, item, _, _, _), values) in declarations.iter().zip(&resolved_values) {
        if values.is_none() {
            return Err(SourceFixedColumnsWriteError::UnsupportedInitializer {
                source_name: source_name.clone(),
                column: item.name.clone(),
            });
        }
    }

    let mut columns = Vec::with_capacity(declarations.len());
    for (index, (_, item, _, dimensions, _)) in declarations.into_iter().enumerate() {
        let values = resolved_values[index]
            .take()
            .expect("resolved fixed column values should exist");
        columns.push(FixedColumn {
            name: item.name,
            dimensions,
            values,
        });
    }

    Ok(FixedColumns {
        group_name: group_name.to_owned(),
        unit_name: unit_name.to_owned(),
        row_count,
        columns,
    })
}

fn source_fixed_column_values_from_initializer(
    source_name: &str,
    source: &str,
    column_name: &str,
    initializer: &ColumnInitializer,
    row_count: usize,
    constant_values: &SourceFixedConstantValues,
    column_values: &BTreeMap<String, Vec<u64>>,
) -> Result<Option<Vec<u64>>, SourceFixedColumnsWriteError> {
    match initializer.kind {
        ColumnInitializerKind::Sequence => {
            let source = &source[initializer.span.start..initializer.span.end];
            parse_literal_sequence(
                source_name,
                initializer.span,
                source,
                row_count,
                constant_values,
            )
            .map(Some)
        }
        ColumnInitializerKind::Expression => source_fixed_column_expression_values(
            source_name,
            source,
            column_name,
            initializer,
            row_count,
            constant_values,
            column_values,
        ),
    }
}

fn source_fixed_column_dimensions(
    source_name: &str,
    source: &str,
    item: &lzvm_pil::ColumnItem,
    constant_values: &SourceFixedConstantValues,
) -> Result<Vec<u32>, SourceFixedColumnsWriteError> {
    if item.array_dims.is_empty() {
        return Ok(vec![1]);
    }

    item.array_dim_expressions
        .iter()
        .zip(&item.array_dims)
        .map(|(expression, span)| {
            let Some(expression) = expression else {
                return Err(SourceFixedColumnsWriteError::UnsupportedColumnShape {
                    source_name: source_name.to_owned(),
                    column: item.name.clone(),
                });
            };
            let expression_text = source_fixed_dimension_expression_text(source, *span);
            let Some(FixedFileTemplateValue::Integer(value)) =
                evaluate_fixed_file_template_value_expression_with_values(
                    expression,
                    &constant_values.scalars,
                )
            else {
                return Err(SourceFixedColumnsWriteError::UnsupportedExpression {
                    source_name: source_name.to_owned(),
                    source_span: *span,
                    expression: expression_text,
                });
            };
            u32::try_from(value).map_err(|_| SourceFixedColumnsWriteError::IntegerOutOfRange {
                source_name: source_name.to_owned(),
                source_span: *span,
                expression: expression_text,
            })
        })
        .collect()
}

fn source_fixed_dimension_expression_text(source: &str, span: SourceSpan) -> String {
    if span.start < span.end
        && source.as_bytes().get(span.start) == Some(&b'[')
        && source.as_bytes().get(span.end.saturating_sub(1)) == Some(&b']')
    {
        return source[span.start + 1..span.end - 1].to_owned();
    }
    source
        .get(span.start..span.end)
        .unwrap_or_default()
        .to_owned()
}

fn source_fixed_constant_values(
    program: &SourceProgram,
) -> Result<SourceFixedConstantValues, SourceFixedColumnsWriteError> {
    let declarations = program
        .modules
        .iter()
        .flat_map(|module| module.constants.iter())
        .collect::<Vec<_>>();
    let mut values = BTreeMap::new();
    let mut resolved = vec![false; declarations.len()];

    loop {
        let mut progressed = false;
        for (index, declaration) in declarations.iter().enumerate() {
            if resolved[index] {
                continue;
            }
            if source_fixed_constant_value(declaration, &mut values).is_some() {
                resolved[index] = true;
                progressed = true;
            }
        }
        if !progressed {
            break;
        }
    }

    let mut constant_values = SourceFixedConstantValues {
        scalars: values,
        arrays: BTreeMap::new(),
    };

    for module in &program.modules {
        for declaration in &module.constants {
            if let Some(values) = source_fixed_constant_array_value(
                declaration,
                &module.source.contents,
                &constant_values,
            )? {
                constant_values
                    .arrays
                    .insert(declaration.name.clone(), values);
            }
        }
    }

    Ok(constant_values)
}

fn source_fixed_constant_value(
    declaration: &ConstantDeclaration,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
) -> Option<()> {
    if !declaration.array_dims.is_empty() || values.contains_key(&declaration.name) {
        return Some(());
    }
    let expression = declaration.initializer_expression.as_ref()?;
    let value = evaluate_fixed_file_template_value_expression_with_values(expression, values)?;
    values.insert(declaration.name.clone(), value);
    Some(())
}

fn source_fixed_constant_array_value(
    declaration: &ConstantDeclaration,
    source: &str,
    values: &SourceFixedConstantValues,
) -> Result<Option<Vec<u64>>, SourceFixedColumnsWriteError> {
    if declaration.array_dims.len() != 1
        || declaration.array_dim_expressions.len() != 1
        || values.arrays.contains_key(&declaration.name)
    {
        return Ok(None);
    }
    let Some(dimension_expression) = declaration.array_dim_expressions[0].as_ref() else {
        return Ok(None);
    };
    let Some(initializer_span) = declaration.initializer else {
        return Ok(None);
    };
    let Some(FixedFileTemplateValue::Integer(length)) =
        evaluate_fixed_file_template_value_expression_with_values(
            dimension_expression,
            &values.scalars,
        )
    else {
        return Ok(None);
    };
    let length =
        usize::try_from(length).map_err(|_| SourceFixedColumnsWriteError::IntegerOutOfRange {
            source_name: declaration.source_name.clone(),
            source_span: declaration.array_dims[0],
            expression: source_fixed_dimension_expression_text(source, declaration.array_dims[0]),
        })?;
    let initializer = source
        .get(initializer_span.start..initializer_span.end)
        .unwrap_or_default();
    parse_literal_sequence_values(
        &declaration.source_name,
        initializer_span,
        initializer,
        length,
        values,
    )?
    .into_iter()
    .map(|value| canonical_fixed_value(value, &declaration.source_name, initializer_span))
    .collect::<Result<Vec<_>, _>>()
    .map(Some)
}
