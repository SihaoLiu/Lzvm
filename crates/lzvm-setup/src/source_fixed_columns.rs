use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::PathBuf;

use lzvm_artifacts::fixed::{
    encode_fixed_columns, encode_raw_fixed_columns, raw_fixed_column_layout,
    read_fixed_columns_file, read_fixed_columns_file_for_setup, FixedColumn, FixedColumnError,
    FixedColumns,
};
use lzvm_artifacts::global_info::GlobalInfo;
use lzvm_artifacts::key_directory::{read_key_directory_layout, KeyDirectoryError, KeyUnitKind};
use lzvm_artifacts::setup_info::{read_unit_setup_info_binary_file, SetupInfoError, UnitSetupInfo};
use lzvm_field::Felt;
use lzvm_pil::{
    AirInstanceDeclaration, ColumnInitializer, ColumnInitializerKind, ColumnItem, ColumnKind,
    ConstantDeclaration, FixedFileTemplateValue, LexError, ParseError, SourceLoaderConfig,
    SourceProgram, SourceProgramError, SourceProgramLoader, SourceSpan,
};

use crate::{
    publish_staging_bytes,
    source_fixed_assignments::source_fixed_values_from_template_assignments,
    source_fixed_expression::SourceFixedConstantValues,
    source_fixed_expression::{
        source_fixed_column_expression_values, SourceFixedExpressionValuesRequest,
    },
    source_fixed_sequence::{
        canonical_fixed_value, parse_literal_sequence, parse_literal_sequence_values,
    },
    source_key_directory::source_item_name,
    source_metadata_template::{
        source_metadata_declaration_template, source_metadata_template_values,
    },
    source_scope::{
        concrete_template_names, declaration_in_function_body, declaration_in_inactive_template,
    },
    source_static_values::{
        evaluate_source_static_expression, source_declaration_constant_values_from_cache,
        source_declaration_in_static_false_branch, source_template_constant_value_cache,
        static_value_integer, SourceTemplateConstantValueCache,
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

#[derive(Debug, Clone)]
struct SourceFixedColumnDeclaration {
    source_name: String,
    source: String,
    item: ColumnItem,
    initializer: Option<ColumnInitializer>,
    dimensions: Vec<u32>,
    constant_values: SourceFixedConstantValues,
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
        SourceFixedColumnsOutputFormat::Portable,
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
            SourceFixedColumnsOutputFormat::Raw,
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
    output_format: SourceFixedColumnsOutputFormat,
) -> Result<SourceFixedColumnsWriteReport, SourceFixedColumnsWriteError> {
    let columns = fixed_columns_from_source_program(program, setup, group_name, unit_name)?;
    let bytes = match output_format {
        SourceFixedColumnsOutputFormat::Portable => {
            let bytes = encode_fixed_columns(&columns)?;
            encode_raw_fixed_columns(&columns, setup)?;
            bytes
        }
        SourceFixedColumnsOutputFormat::Raw => encode_raw_fixed_columns(&columns, setup)?,
    };
    let staging_path = write_staging_bytes(
        &output_path,
        &bytes,
        "write source fixed columns staging file",
    )?;
    match output_format {
        SourceFixedColumnsOutputFormat::Portable => {
            read_fixed_columns_file(&staging_path)?;
        }
        SourceFixedColumnsOutputFormat::Raw => {
            read_fixed_columns_file_for_setup(&staging_path, setup, group_name, unit_name)?;
        }
    }
    let bytes_written =
        publish_staging_bytes(&staging_path, &output_path, "publish source fixed columns")?;

    Ok(SourceFixedColumnsWriteReport {
        output_path,
        bytes_written,
        column_count: columns.columns.len(),
        row_count: columns.row_count,
    })
}

#[derive(Clone, Copy)]
enum SourceFixedColumnsOutputFormat {
    Portable,
    Raw,
}

fn source_fixed_unit_instance<'a>(
    program: &'a SourceProgram,
    group_name: &str,
    unit_name: &str,
) -> Option<&'a AirInstanceDeclaration> {
    let units = program
        .air_units()
        .into_iter()
        .filter(|unit| !unit.virtual_instance)
        .collect::<Vec<_>>();
    let instances = program
        .modules
        .iter()
        .flat_map(|module| module.air_instances.iter())
        .filter(|instance| !instance.virtual_instance)
        .collect::<Vec<_>>();
    units
        .into_iter()
        .zip(instances)
        .find_map(|(unit, instance)| {
            (unit.group_name == group_name && unit.unit_name == unit_name).then_some(instance)
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
    let mut declarations = Vec::<SourceFixedColumnDeclaration>::new();
    let constant_values = source_fixed_constant_values(program, setup, row_count)?;
    let template_values = source_template_constant_value_cache(program, &constant_values.scalars);
    let active_templates = concrete_template_names(program);
    let unit_instance = source_fixed_unit_instance(program, group_name, unit_name);
    let mut logical_dimensions = BTreeMap::new();
    let mut seen_declarations = BTreeSet::new();

    for module in &program.modules {
        for declaration in &module.columns {
            if declaration.kind != ColumnKind::Fixed {
                continue;
            }
            if declaration_in_function_body(module, declaration.start, declaration.end)
                || declaration_in_inactive_template(
                    module,
                    declaration.start,
                    declaration.end,
                    &active_templates,
                )
            {
                continue;
            }
            let declaration_values = source_fixed_declaration_constant_values(
                module,
                declaration.start,
                declaration.end,
                &constant_values,
                &template_values,
            );
            let declaration_values = if let Some(template) =
                source_metadata_declaration_template(module, declaration.start, declaration.end)
            {
                if let Some(instance) = unit_instance {
                    if template.name != instance.template {
                        continue;
                    }
                    SourceFixedConstantValues {
                        scalars: source_metadata_template_values(
                            program,
                            module,
                            template,
                            Some(instance),
                            &constant_values.scalars,
                            &template_values,
                        ),
                        arrays: declaration_values.arrays,
                    }
                } else {
                    SourceFixedConstantValues {
                        scalars: source_metadata_template_values(
                            program,
                            module,
                            template,
                            None,
                            &constant_values.scalars,
                            &template_values,
                        ),
                        arrays: declaration_values.arrays,
                    }
                }
            } else {
                declaration_values
            };
            if source_declaration_in_static_false_branch(
                program,
                module,
                declaration.start,
                declaration.end,
                &declaration_values.scalars,
            ) {
                continue;
            }
            for item in &declaration.items {
                let item_name = source_fixed_item_name(
                    program,
                    &declaration.source_name,
                    item,
                    &declaration_values,
                )?;
                if !source_fixed_expected_column_matches(&item_name, &expected_columns) {
                    continue;
                }
                let dimensions = source_fixed_column_dimensions(
                    program,
                    &declaration.source_name,
                    &module.source.contents,
                    item,
                    &declaration_values,
                )?;
                logical_dimensions
                    .entry(item_name.clone())
                    .or_insert_with(|| dimensions.clone());
                let physical_columns =
                    source_fixed_physical_columns(&item_name, &dimensions, &expected_columns);
                for (column_name, column_dimensions) in physical_columns {
                    if !seen_declarations.insert(column_name.clone()) {
                        continue;
                    }
                    let mut column_item = item.clone();
                    column_item.name = column_name;
                    let initializer = if column_item.name == item_name {
                        declaration.initializer.clone()
                    } else {
                        None
                    };
                    declarations.push(SourceFixedColumnDeclaration {
                        source_name: declaration.source_name.clone(),
                        source: module.source.contents.clone(),
                        item: column_item,
                        initializer,
                        dimensions: column_dimensions,
                        constant_values: declaration_values.clone(),
                    });
                }
            }
        }
    }

    let mut column_values = source_fixed_values_from_template_assignments(
        program,
        &expected_columns,
        &logical_dimensions,
        row_count_usize,
        &constant_values,
        unit_instance,
    )?;
    let mut resolved_values = vec![None::<Vec<u64>>; declarations.len()];
    loop {
        let mut progressed = false;
        for (index, declaration) in declarations.iter().enumerate() {
            if resolved_values[index].is_some() {
                continue;
            }
            let values = if declaration.initializer.is_some() {
                source_fixed_column_values_from_initializer(
                    program,
                    declaration,
                    row_count_usize,
                    &column_values,
                )?
            } else {
                column_values.get(&declaration.item.name).cloned()
            };
            let Some(values) = values else {
                continue;
            };
            column_values.insert(declaration.item.name.clone(), values.clone());
            resolved_values[index] = Some(values);
            progressed = true;
        }
        if !progressed {
            break;
        }
    }

    for (declaration, values) in declarations.iter().zip(&resolved_values) {
        if values.is_none() {
            return Err(SourceFixedColumnsWriteError::UnsupportedInitializer {
                source_name: declaration.source_name.clone(),
                column: declaration.item.name.clone(),
            });
        }
    }

    let mut columns = Vec::with_capacity(declarations.len());
    for (index, declaration) in declarations.into_iter().enumerate() {
        let values = resolved_values[index]
            .take()
            .expect("resolved fixed column values should exist");
        columns.push(FixedColumn {
            name: declaration.item.name,
            dimensions: declaration.dimensions,
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

fn source_fixed_declaration_constant_values(
    module: &lzvm_pil::SourceProgramModule,
    start: usize,
    end: usize,
    base_values: &SourceFixedConstantValues,
    template_values: &SourceTemplateConstantValueCache,
) -> SourceFixedConstantValues {
    SourceFixedConstantValues {
        scalars: source_declaration_constant_values_from_cache(
            module,
            start,
            end,
            &base_values.scalars,
            template_values,
        )
        .clone(),
        arrays: base_values.arrays.clone(),
    }
}

fn source_fixed_item_name(
    program: &SourceProgram,
    source_name: &str,
    item: &ColumnItem,
    constant_values: &SourceFixedConstantValues,
) -> Result<String, SourceFixedColumnsWriteError> {
    source_item_name(
        program,
        item,
        "source fixed-column",
        &constant_values.scalars,
    )
    .map_err(|_| SourceFixedColumnsWriteError::UnsupportedColumnShape {
        source_name: source_name.to_owned(),
        column: item.name.clone(),
    })
}

fn source_fixed_column_values_from_initializer(
    program: &SourceProgram,
    declaration: &SourceFixedColumnDeclaration,
    row_count: usize,
    column_values: &BTreeMap<String, Vec<u64>>,
) -> Result<Option<Vec<u64>>, SourceFixedColumnsWriteError> {
    let Some(initializer) = declaration.initializer.as_ref() else {
        return Ok(None);
    };
    match initializer.kind {
        ColumnInitializerKind::Sequence => {
            let source = &declaration.source[initializer.span.start..initializer.span.end];
            parse_literal_sequence(
                program,
                &declaration.source_name,
                initializer.span,
                source,
                row_count,
                &declaration.constant_values,
            )
            .map(Some)
        }
        ColumnInitializerKind::Expression => {
            source_fixed_column_expression_values(&SourceFixedExpressionValuesRequest {
                program,
                source_name: &declaration.source_name,
                source: &declaration.source,
                column_name: &declaration.item.name,
                initializer,
                row_count,
                constant_values: &declaration.constant_values,
                column_values,
            })
        }
    }
}

fn source_fixed_column_dimensions(
    program: &SourceProgram,
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
            let Some(value) =
                evaluate_source_static_expression(program, expression, &constant_values.scalars)
                    .as_ref()
                    .and_then(static_value_integer)
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

fn source_fixed_physical_columns(
    name: &str,
    dimensions: &[u32],
    expected_columns: &BTreeSet<String>,
) -> Vec<(String, Vec<u32>)> {
    if expected_columns.contains(name) {
        return vec![(name.to_owned(), dimensions.to_vec())];
    }
    let element_count = dimensions
        .iter()
        .try_fold(1_u32, |acc, dimension| acc.checked_mul(*dimension));
    let Some(element_count) = element_count else {
        return Vec::new();
    };
    let mut columns = Vec::new();
    for element in 0..element_count {
        let column_name = format!("{name}[{element}]");
        if expected_columns.contains(&column_name) {
            columns.push((column_name, vec![1]));
        }
    }
    columns
}

fn source_fixed_expected_column_matches(name: &str, expected_columns: &BTreeSet<String>) -> bool {
    expected_columns.contains(name)
        || expected_columns
            .iter()
            .any(|column| column.starts_with(name) && column[name.len()..].starts_with('['))
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
    setup: &UnitSetupInfo,
    row_count: u64,
) -> Result<SourceFixedConstantValues, SourceFixedColumnsWriteError> {
    let declarations = program
        .modules
        .iter()
        .flat_map(|module| module.constants.iter())
        .collect::<Vec<_>>();
    let mut values = source_fixed_domain_constant_values(setup, row_count);
    let mut resolved = vec![false; declarations.len()];

    loop {
        let mut progressed = false;
        for (index, declaration) in declarations.iter().enumerate() {
            if resolved[index] {
                continue;
            }
            if source_fixed_constant_value(program, declaration, &mut values).is_some() {
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
                program,
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

fn source_fixed_domain_constant_values(
    setup: &UnitSetupInfo,
    row_count: u64,
) -> BTreeMap<String, FixedFileTemplateValue> {
    let mut values = BTreeMap::from([
        (
            "BITS".to_owned(),
            FixedFileTemplateValue::Integer(i128::from(setup.stark.n_bits)),
        ),
        (
            "N".to_owned(),
            FixedFileTemplateValue::Integer(i128::from(row_count)),
        ),
    ]);
    if let Some(root) = Felt::root_of_unity(setup.stark.n_bits as usize) {
        values.insert(
            "omega".to_owned(),
            FixedFileTemplateValue::Integer(i128::from(root.to_u64())),
        );
    }
    values
}

fn source_fixed_constant_value(
    program: &SourceProgram,
    declaration: &ConstantDeclaration,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
) -> Option<()> {
    if !declaration.array_dims.is_empty() || values.contains_key(&declaration.name) {
        return Some(());
    }
    let expression = declaration.initializer_expression.as_ref()?;
    let value = evaluate_source_static_expression(program, expression, values)?;
    values.insert(declaration.name.clone(), value);
    Some(())
}

fn source_fixed_constant_array_value(
    program: &SourceProgram,
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
    let Some(length) =
        evaluate_source_static_expression(program, dimension_expression, &values.scalars)
            .as_ref()
            .and_then(static_value_integer)
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
    if !initializer.trim_start().starts_with('[') {
        return Ok(None);
    }
    let values = match parse_literal_sequence_values(
        program,
        &declaration.source_name,
        initializer_span,
        initializer,
        length,
        values,
    ) {
        Ok(values) => values,
        Err(SourceFixedColumnsWriteError::UnsupportedExpression { .. }) => return Ok(None),
        Err(error) => return Err(error),
    };
    values
        .into_iter()
        .map(|value| canonical_fixed_value(value, &declaration.source_name, initializer_span))
        .collect::<Result<Vec<_>, _>>()
        .map(Some)
}
