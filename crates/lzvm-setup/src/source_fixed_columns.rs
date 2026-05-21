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
    lex_source, BinaryOperator, ColumnInitializer, ColumnInitializerKind, ColumnItem, ColumnKind,
    ConstantDeclaration, Expression, ExpressionKind, FixedFileTemplateValue, FunctionStatement,
    FunctionStatementKind, LexError, ParseError, SourceLoaderConfig, SourceProgram,
    SourceProgramError, SourceProgramLoader, SourceProgramModule, SourceSpan, Token, UnaryOperator,
};

use crate::{
    publish_staging_bytes,
    source_control_body_cache::SourceControlBodyCache,
    source_fixed_expression::SourceFixedConstantValues,
    source_fixed_expression::{
        evaluate_source_fixed_template_value_expression_with_parts,
        source_fixed_column_expression_values, SourceFixedExpressionValuesRequest,
    },
    source_fixed_sequence::{
        canonical_fixed_value, parse_literal_sequence, parse_literal_sequence_values,
    },
    source_key_directory::SourceKeyDirectoryMetadataError,
    source_scope::{
        concrete_template_names, declaration_in_function_body, declaration_in_inactive_template,
    },
    source_static_values::{
        evaluate_source_static_expression, source_declaration_constant_values_from_cache,
        source_declaration_in_static_false_branch, source_template_constant_value_cache,
        static_value_integer, SourceStaticValueLookup, SourceTemplateConstantValueCache,
    },
    source_template_for::source_static_for_loop_with_lookup,
    source_template_if::source_static_if_body_statements_with_lookup,
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
                if !source_fixed_expected_column_matches(&item.name, &expected_columns) {
                    continue;
                }
                if item.template {
                    return Err(SourceFixedColumnsWriteError::UnsupportedColumnShape {
                        source_name: declaration.source_name.clone(),
                        column: item.name.clone(),
                    });
                }
                let dimensions = source_fixed_column_dimensions(
                    program,
                    &declaration.source_name,
                    &module.source.contents,
                    item,
                    &declaration_values,
                )?;
                logical_dimensions
                    .entry(item.name.clone())
                    .or_insert_with(|| dimensions.clone());
                let physical_columns =
                    source_fixed_physical_columns(&item.name, &dimensions, &expected_columns);
                for (column_name, column_dimensions) in physical_columns {
                    if !seen_declarations.insert(column_name.clone()) {
                        continue;
                    }
                    let mut column_item = item.clone();
                    column_item.name = column_name;
                    let initializer = if column_item.name == item.name {
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

fn source_fixed_values_from_template_assignments(
    program: &SourceProgram,
    expected_columns: &BTreeSet<String>,
    logical_dimensions: &BTreeMap<String, Vec<u32>>,
    row_count: usize,
    constant_values: &SourceFixedConstantValues,
) -> Result<BTreeMap<String, Vec<u64>>, SourceFixedColumnsWriteError> {
    let mut partial_values = BTreeMap::<String, Vec<Option<u64>>>::new();
    let active_templates = concrete_template_names(program);
    for module in &program.modules {
        let tokens = lex_source(&module.source.contents).map_err(|source| {
            SourceFixedColumnsWriteError::Lex {
                source_name: module.source_name.clone(),
                source_span: SourceSpan {
                    start: 0,
                    end: module.source.contents.len(),
                },
                source,
            }
        })?;
        let mut body_cache = SourceControlBodyCache::default();
        for template in &module.air_templates {
            if !active_templates.contains(&template.name) {
                continue;
            }
            let assignment_values = SourceFixedAssignmentValues::base(constant_values);
            let context = SourceFixedTemplateAssignmentContext {
                program,
                module,
                tokens: &tokens,
                expected_columns,
                logical_dimensions,
                row_count,
            };
            for statement in &template.statements {
                collect_source_fixed_template_assignment(
                    &context,
                    statement,
                    &assignment_values,
                    &mut body_cache,
                    &mut partial_values,
                )?;
            }
        }
    }

    Ok(partial_values
        .into_iter()
        .filter_map(|(name, values)| {
            values
                .into_iter()
                .collect::<Option<Vec<_>>>()
                .map(|values| (name, values))
        })
        .collect())
}

struct SourceFixedTemplateAssignmentContext<'a> {
    program: &'a SourceProgram,
    module: &'a SourceProgramModule,
    tokens: &'a [Token],
    expected_columns: &'a BTreeSet<String>,
    logical_dimensions: &'a BTreeMap<String, Vec<u32>>,
    row_count: usize,
}

struct SourceFixedAssignmentValues<'a> {
    base_scalars: &'a BTreeMap<String, FixedFileTemplateValue>,
    overlays: Vec<(String, FixedFileTemplateValue)>,
    arrays: &'a BTreeMap<String, Vec<u64>>,
}

impl<'a> SourceFixedAssignmentValues<'a> {
    fn base(constant_values: &'a SourceFixedConstantValues) -> Self {
        Self {
            base_scalars: &constant_values.scalars,
            overlays: Vec::new(),
            arrays: &constant_values.arrays,
        }
    }

    fn with_loop_value(
        base: &SourceFixedAssignmentValues<'a>,
        variable_name: &str,
        value: &FixedFileTemplateValue,
    ) -> Self {
        let mut overlays = base.overlays.clone();
        overlays.push((variable_name.to_owned(), value.clone()));
        Self {
            base_scalars: base.base_scalars,
            overlays,
            arrays: base.arrays,
        }
    }

    fn scalar_value(&self, name: &str) -> Option<FixedFileTemplateValue> {
        self.source_static_value(name).cloned()
    }

    fn fixed_constant_values(&self) -> SourceFixedConstantValues {
        let mut scalars = self.base_scalars.clone();
        for (name, value) in &self.overlays {
            scalars.insert(name.clone(), value.clone());
        }
        SourceFixedConstantValues {
            scalars,
            arrays: self.arrays.clone(),
        }
    }
}

impl SourceStaticValueLookup for SourceFixedAssignmentValues<'_> {
    fn source_static_value(&self, name: &str) -> Option<&FixedFileTemplateValue> {
        self.overlays
            .iter()
            .rev()
            .find_map(|(candidate, value)| (candidate == name).then_some(value))
            .or_else(|| self.base_scalars.get(name))
    }

    fn source_static_integer_values(&self) -> BTreeMap<String, i128> {
        let mut values = self
            .base_scalars
            .iter()
            .filter_map(|(name, value)| match value {
                FixedFileTemplateValue::Integer(value) => Some((name.clone(), *value)),
                _ => None,
            })
            .collect::<BTreeMap<_, _>>();
        for (name, value) in &self.overlays {
            match value {
                FixedFileTemplateValue::Integer(value) => {
                    values.insert(name.clone(), *value);
                }
                _ => {
                    values.remove(name);
                }
            }
        }
        values
    }
}

fn collect_source_fixed_template_assignment(
    context: &SourceFixedTemplateAssignmentContext<'_>,
    statement: &FunctionStatement,
    assignment_values: &SourceFixedAssignmentValues<'_>,
    body_cache: &mut SourceControlBodyCache,
    partial_values: &mut BTreeMap<String, Vec<Option<u64>>>,
) -> Result<(), SourceFixedColumnsWriteError> {
    if statement.kind == FunctionStatementKind::If {
        match source_static_if_body_statements_with_lookup(
            context.program,
            context.module,
            context.tokens,
            statement,
            assignment_values,
            body_cache,
        ) {
            Ok(Some(body_statements)) => {
                for body_statement in body_statements.iter() {
                    collect_source_fixed_template_assignment(
                        context,
                        body_statement,
                        assignment_values,
                        body_cache,
                        partial_values,
                    )?;
                }
            }
            Ok(None) | Err(SourceKeyDirectoryMetadataError::UnsupportedSourceProgram { .. }) => {}
            Err(error) => {
                return Err(source_fixed_template_assignment_error(statement, error));
            }
        }
        return Ok(());
    }
    if statement.kind == FunctionStatementKind::For {
        match source_static_for_loop_with_lookup(
            context.program,
            context.module,
            context.tokens,
            statement,
            assignment_values,
            body_cache,
        ) {
            Ok(Some(loop_info)) => {
                for iteration_value in &loop_info.iteration_values {
                    let iteration_assignment_values = SourceFixedAssignmentValues::with_loop_value(
                        assignment_values,
                        &loop_info.variable_name,
                        iteration_value,
                    );
                    for body_statement in loop_info.body_statements.iter() {
                        collect_source_fixed_template_assignment(
                            context,
                            body_statement,
                            &iteration_assignment_values,
                            body_cache,
                            partial_values,
                        )?;
                    }
                }
            }
            Ok(None) | Err(SourceKeyDirectoryMetadataError::UnsupportedSourceProgram { .. }) => {}
            Err(error) => {
                return Err(source_fixed_template_assignment_error(statement, error));
            }
        }
        return Ok(());
    }
    if statement.kind != FunctionStatementKind::Expression {
        return Ok(());
    }
    let Some(expression) = statement.value_expression.as_ref() else {
        return Ok(());
    };
    let ExpressionKind::Binary { op, left, right } =
        &strip_source_fixed_group_expression(expression).kind
    else {
        return Ok(());
    };
    if *op != BinaryOperator::Assign {
        return Ok(());
    }
    if collect_source_fixed_element_sequence_assignment(
        context,
        left,
        right,
        assignment_values,
        partial_values,
    )? {
        return Ok(());
    }
    let Some((column_name, row)) = source_fixed_index_assignment_target(
        &context.module.source_name,
        left,
        context.expected_columns,
        context.logical_dimensions,
        context.row_count,
        assignment_values,
    )?
    else {
        return Ok(());
    };
    let Some(value) = evaluate_source_fixed_assignment_value_expression(right, assignment_values)
        .as_ref()
        .and_then(source_fixed_assignment_integer)
    else {
        return Ok(());
    };
    let value = canonical_fixed_value(
        value,
        &context.module.source_name,
        SourceSpan {
            start: right.start,
            end: right.end,
        },
    )?;
    let values = partial_values
        .entry(column_name.clone())
        .or_insert_with(|| vec![None; context.row_count]);
    match values[row] {
        Some(existing) if existing != value => {
            Err(SourceFixedColumnsWriteError::UnsupportedInitializer {
                source_name: context.module.source_name.clone(),
                column: column_name,
            })
        }
        Some(_) => Ok(()),
        None => {
            values[row] = Some(value);
            Ok(())
        }
    }
}

fn collect_source_fixed_element_sequence_assignment(
    context: &SourceFixedTemplateAssignmentContext<'_>,
    left: &Expression,
    right: &Expression,
    assignment_values: &SourceFixedAssignmentValues<'_>,
    partial_values: &mut BTreeMap<String, Vec<Option<u64>>>,
) -> Result<bool, SourceFixedColumnsWriteError> {
    let Some(column_name) = source_fixed_element_assignment_target(
        left,
        context.expected_columns,
        context.logical_dimensions,
        assignment_values,
    ) else {
        return Ok(false);
    };
    let Some(source) = context.module.source.contents.get(right.start..right.end) else {
        return Ok(false);
    };
    if !source.trim_start().starts_with('[') {
        return Ok(false);
    }
    let constant_values = assignment_values.fixed_constant_values();
    let values = parse_literal_sequence(
        context.program,
        &context.module.source_name,
        SourceSpan {
            start: right.start,
            end: right.end,
        },
        source,
        context.row_count,
        &constant_values,
    )?;
    merge_source_fixed_complete_values(
        &context.module.source_name,
        &column_name,
        context.row_count,
        values,
        partial_values,
    )?;
    Ok(true)
}

fn source_fixed_element_assignment_target(
    expression: &Expression,
    expected_columns: &BTreeSet<String>,
    logical_dimensions: &BTreeMap<String, Vec<u32>>,
    values: &SourceFixedAssignmentValues<'_>,
) -> Option<String> {
    source_fixed_physical_assignment_column_name(
        expression,
        expected_columns,
        logical_dimensions,
        values,
    )
}

fn merge_source_fixed_complete_values(
    source_name: &str,
    column_name: &str,
    row_count: usize,
    values: Vec<u64>,
    partial_values: &mut BTreeMap<String, Vec<Option<u64>>>,
) -> Result<(), SourceFixedColumnsWriteError> {
    if values.len() != row_count {
        return Err(SourceFixedColumnsWriteError::UnsupportedInitializer {
            source_name: source_name.to_owned(),
            column: column_name.to_owned(),
        });
    }
    let partial = partial_values
        .entry(column_name.to_owned())
        .or_insert_with(|| vec![None; row_count]);
    for (row, value) in values.into_iter().enumerate() {
        match partial[row] {
            Some(existing) if existing != value => {
                return Err(SourceFixedColumnsWriteError::UnsupportedInitializer {
                    source_name: source_name.to_owned(),
                    column: column_name.to_owned(),
                });
            }
            Some(_) => {}
            None => partial[row] = Some(value),
        }
    }
    Ok(())
}

fn evaluate_source_fixed_assignment_value_expression(
    expression: &Expression,
    values: &SourceFixedAssignmentValues<'_>,
) -> Option<FixedFileTemplateValue> {
    if values.overlays.is_empty() {
        return evaluate_source_fixed_template_value_expression_with_parts(
            expression,
            values.base_scalars,
            values.arrays,
        );
    }

    match &expression.kind {
        ExpressionKind::Integer(value) | ExpressionKind::HexInteger(value) => {
            parse_source_fixed_assignment_integer(value).map(FixedFileTemplateValue::Integer)
        }
        ExpressionKind::StringLiteral(value) | ExpressionKind::TemplateLiteral(value) => {
            Some(FixedFileTemplateValue::String(value.clone()))
        }
        ExpressionKind::Name(name) => values.scalar_value(name),
        ExpressionKind::Group(inner) => {
            evaluate_source_fixed_assignment_value_expression(inner, values)
        }
        ExpressionKind::Unary { op, expr } => {
            let value = evaluate_source_fixed_assignment_value_expression(expr, values)?;
            match op {
                UnaryOperator::Plus => {
                    source_fixed_assignment_integer(&value).map(FixedFileTemplateValue::Integer)
                }
                UnaryOperator::Minus => source_fixed_assignment_integer(&value)
                    .and_then(i128::checked_neg)
                    .map(FixedFileTemplateValue::Integer),
                UnaryOperator::Not => Some(FixedFileTemplateValue::Boolean(
                    !source_fixed_assignment_truthy(&value),
                )),
                UnaryOperator::Increment | UnaryOperator::Decrement => None,
            }
        }
        ExpressionKind::Binary { op, left, right } => {
            let left = evaluate_source_fixed_assignment_value_expression(left, values)?;
            if *op == BinaryOperator::LogicalAnd {
                if source_fixed_assignment_truthy(&left) {
                    return evaluate_source_fixed_assignment_value_expression(right, values);
                }
                return Some(left);
            }
            if *op == BinaryOperator::LogicalOr {
                if source_fixed_assignment_truthy(&left) {
                    return Some(left);
                }
                return evaluate_source_fixed_assignment_value_expression(right, values);
            }
            let right = evaluate_source_fixed_assignment_value_expression(right, values)?;
            evaluate_source_fixed_assignment_binary(*op, left, right)
        }
        ExpressionKind::Index { target, index } => {
            let ExpressionKind::Name(array_name) =
                &strip_source_fixed_group_expression(target).kind
            else {
                return None;
            };
            let array_values = values.arrays.get(array_name)?;
            let index = evaluate_source_fixed_assignment_value_expression(index, values)?;
            let index = usize::try_from(source_fixed_assignment_integer(&index)?).ok()?;
            array_values
                .get(index)
                .copied()
                .map(|value| FixedFileTemplateValue::Integer(i128::from(value)))
        }
        ExpressionKind::Call { .. }
        | ExpressionKind::Array(_)
        | ExpressionKind::RowOffset { .. }
        | ExpressionKind::PositionalParam(_) => None,
    }
}

fn evaluate_source_fixed_assignment_binary(
    op: BinaryOperator,
    left: FixedFileTemplateValue,
    right: FixedFileTemplateValue,
) -> Option<FixedFileTemplateValue> {
    match op {
        BinaryOperator::Add => match (left, right) {
            (FixedFileTemplateValue::Integer(left), FixedFileTemplateValue::Integer(right)) => {
                left.checked_add(right).map(FixedFileTemplateValue::Integer)
            }
            (left, right) => Some(FixedFileTemplateValue::String(format!(
                "{}{}",
                source_fixed_assignment_string(left),
                source_fixed_assignment_string(right)
            ))),
        },
        BinaryOperator::Subtract => {
            source_fixed_assignment_integer_op(left, right, i128::checked_sub)
        }
        BinaryOperator::Multiply => {
            source_fixed_assignment_integer_op(left, right, i128::checked_mul)
        }
        BinaryOperator::Divide | BinaryOperator::Backslash => {
            let left = source_fixed_assignment_integer(&left)?;
            let right = source_fixed_assignment_integer(&right)?;
            (right != 0).then(|| FixedFileTemplateValue::Integer(left / right))
        }
        BinaryOperator::Modulo => {
            let left = source_fixed_assignment_integer(&left)?;
            let right = source_fixed_assignment_integer(&right)?;
            (right != 0).then(|| FixedFileTemplateValue::Integer(left % right))
        }
        BinaryOperator::Power => {
            let left = source_fixed_assignment_integer(&left)?;
            let right = u32::try_from(source_fixed_assignment_integer(&right)?).ok()?;
            left.checked_pow(right).map(FixedFileTemplateValue::Integer)
        }
        BinaryOperator::ShiftLeft => source_fixed_assignment_shift(left, right, true),
        BinaryOperator::ShiftRight => source_fixed_assignment_shift(left, right, false),
        BinaryOperator::BitAnd => {
            source_fixed_assignment_bitwise(left, right, |left, right| left & right)
        }
        BinaryOperator::BitXor => {
            source_fixed_assignment_bitwise(left, right, |left, right| left ^ right)
        }
        BinaryOperator::BitOr => {
            source_fixed_assignment_bitwise(left, right, |left, right| left | right)
        }
        BinaryOperator::Less => {
            source_fixed_assignment_cmp(left, right, |left, right| left < right)
        }
        BinaryOperator::LessEqual => {
            source_fixed_assignment_cmp(left, right, |left, right| left <= right)
        }
        BinaryOperator::Greater => {
            source_fixed_assignment_cmp(left, right, |left, right| left > right)
        }
        BinaryOperator::GreaterEqual => {
            source_fixed_assignment_cmp(left, right, |left, right| left >= right)
        }
        BinaryOperator::EqualEqual | BinaryOperator::TripleEqual => Some(
            FixedFileTemplateValue::Boolean(source_fixed_assignment_value_eq(&left, &right)),
        ),
        BinaryOperator::NotEqual => Some(FixedFileTemplateValue::Boolean(
            !source_fixed_assignment_value_eq(&left, &right),
        )),
        _ => None,
    }
}

fn source_fixed_assignment_integer_op(
    left: FixedFileTemplateValue,
    right: FixedFileTemplateValue,
    op: impl Fn(i128, i128) -> Option<i128>,
) -> Option<FixedFileTemplateValue> {
    let left = source_fixed_assignment_integer(&left)?;
    let right = source_fixed_assignment_integer(&right)?;
    op(left, right).map(FixedFileTemplateValue::Integer)
}

fn source_fixed_assignment_shift(
    left: FixedFileTemplateValue,
    right: FixedFileTemplateValue,
    left_shift: bool,
) -> Option<FixedFileTemplateValue> {
    let left = source_fixed_assignment_integer(&left)?;
    let right = u32::try_from(source_fixed_assignment_integer(&right)?).ok()?;
    if left_shift {
        left.checked_shl(right).map(FixedFileTemplateValue::Integer)
    } else {
        left.checked_shr(right).map(FixedFileTemplateValue::Integer)
    }
}

fn source_fixed_assignment_bitwise(
    left: FixedFileTemplateValue,
    right: FixedFileTemplateValue,
    op: impl Fn(i128, i128) -> i128,
) -> Option<FixedFileTemplateValue> {
    let left = source_fixed_assignment_integer(&left)?;
    let right = source_fixed_assignment_integer(&right)?;
    Some(FixedFileTemplateValue::Integer(op(left, right)))
}

fn source_fixed_assignment_cmp(
    left: FixedFileTemplateValue,
    right: FixedFileTemplateValue,
    op: impl Fn(i128, i128) -> bool,
) -> Option<FixedFileTemplateValue> {
    let left = source_fixed_assignment_integer(&left)?;
    let right = source_fixed_assignment_integer(&right)?;
    Some(FixedFileTemplateValue::Boolean(op(left, right)))
}

fn source_fixed_assignment_value_eq(
    left: &FixedFileTemplateValue,
    right: &FixedFileTemplateValue,
) -> bool {
    match (left, right) {
        (FixedFileTemplateValue::Integer(left), FixedFileTemplateValue::Integer(right)) => {
            left == right
        }
        (FixedFileTemplateValue::Boolean(left), FixedFileTemplateValue::Boolean(right)) => {
            left == right
        }
        (FixedFileTemplateValue::String(left), FixedFileTemplateValue::String(right)) => {
            left == right
        }
        _ => false,
    }
}

fn source_fixed_assignment_integer(value: &FixedFileTemplateValue) -> Option<i128> {
    match value {
        FixedFileTemplateValue::Integer(value) => Some(*value),
        FixedFileTemplateValue::Boolean(value) => Some(if *value { 1 } else { 0 }),
        _ => None,
    }
}

fn source_fixed_assignment_truthy(value: &FixedFileTemplateValue) -> bool {
    match value {
        FixedFileTemplateValue::Integer(value) => *value != 0,
        FixedFileTemplateValue::Boolean(value) => *value,
        FixedFileTemplateValue::String(value) => !value.is_empty(),
    }
}

fn source_fixed_assignment_string(value: FixedFileTemplateValue) -> String {
    match value {
        FixedFileTemplateValue::Integer(value) => value.to_string(),
        FixedFileTemplateValue::Boolean(value) => value.to_string(),
        FixedFileTemplateValue::String(value) => value,
    }
}

fn parse_source_fixed_assignment_integer(value: &str) -> Option<i128> {
    let value = value.trim().replace('_', "");
    if let Some(hex) = value
        .strip_prefix("-0x")
        .or_else(|| value.strip_prefix("-0X"))
    {
        return i128::from_str_radix(hex, 16)
            .ok()
            .and_then(i128::checked_neg);
    }
    if let Some(hex) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        return i128::from_str_radix(hex, 16).ok();
    }
    value.parse::<i128>().ok()
}

fn source_fixed_template_assignment_error(
    statement: &FunctionStatement,
    error: SourceKeyDirectoryMetadataError,
) -> SourceFixedColumnsWriteError {
    let source_span = SourceSpan {
        start: statement.start,
        end: statement.end,
    };
    match error {
        SourceKeyDirectoryMetadataError::SourceProgram(error) => {
            SourceFixedColumnsWriteError::SourceProgram(error)
        }
        SourceKeyDirectoryMetadataError::SetupInfo(error) => {
            SourceFixedColumnsWriteError::SetupInfo(error)
        }
        SourceKeyDirectoryMetadataError::Setup(error) => SourceFixedColumnsWriteError::Setup(error),
        SourceKeyDirectoryMetadataError::Parse(error) => {
            SourceFixedColumnsWriteError::ExpressionParse {
                source_name: statement.source_name.clone(),
                source_span,
                source: error,
            }
        }
        SourceKeyDirectoryMetadataError::Lex {
            source_name,
            source,
        } => SourceFixedColumnsWriteError::Lex {
            source_name,
            source_span,
            source,
        },
        other => SourceFixedColumnsWriteError::UnsupportedExpression {
            source_name: statement.source_name.clone(),
            source_span,
            expression: other.to_string(),
        },
    }
}

fn source_fixed_index_assignment_target(
    source_name: &str,
    expression: &Expression,
    expected_columns: &BTreeSet<String>,
    logical_dimensions: &BTreeMap<String, Vec<u32>>,
    row_count: usize,
    constant_values: &SourceFixedAssignmentValues<'_>,
) -> Result<Option<(String, usize)>, SourceFixedColumnsWriteError> {
    let ExpressionKind::Index { target, index } =
        &strip_source_fixed_group_expression(expression).kind
    else {
        return Ok(None);
    };
    let Some(column_name) = source_fixed_index_assignment_column_name(
        target,
        expected_columns,
        logical_dimensions,
        constant_values,
    ) else {
        return Ok(None);
    };
    let Some(row_value) = evaluate_source_fixed_assignment_value_expression(index, constant_values)
    else {
        return Ok(None);
    };
    let Some(row) = source_fixed_assignment_integer(&row_value) else {
        return Ok(None);
    };
    let row =
        usize::try_from(row).map_err(|_| SourceFixedColumnsWriteError::IntegerOutOfRange {
            source_name: source_name.to_owned(),
            source_span: SourceSpan {
                start: index.start,
                end: index.end,
            },
            expression: row.to_string(),
        })?;
    if row >= row_count {
        return Err(SourceFixedColumnsWriteError::IntegerOutOfRange {
            source_name: source_name.to_owned(),
            source_span: SourceSpan {
                start: index.start,
                end: index.end,
            },
            expression: row.to_string(),
        });
    }
    Ok(Some((column_name, row)))
}

fn source_fixed_index_assignment_column_name(
    expression: &Expression,
    expected_columns: &BTreeSet<String>,
    logical_dimensions: &BTreeMap<String, Vec<u32>>,
    values: &SourceFixedAssignmentValues<'_>,
) -> Option<String> {
    source_fixed_physical_assignment_column_name(
        expression,
        expected_columns,
        logical_dimensions,
        values,
    )
}

fn source_fixed_physical_assignment_column_name(
    expression: &Expression,
    expected_columns: &BTreeSet<String>,
    logical_dimensions: &BTreeMap<String, Vec<u32>>,
    values: &SourceFixedAssignmentValues<'_>,
) -> Option<String> {
    let (column_name, indices) = source_fixed_assignment_index_path(expression, values)?;
    if indices.is_empty() {
        return expected_columns
            .contains(&column_name)
            .then_some(column_name);
    }
    if let Some(dimensions) = logical_dimensions.get(&column_name) {
        let index = source_fixed_linear_element_index(&indices, dimensions)?;
        let physical_name = format!("{column_name}[{index}]");
        return expected_columns
            .contains(&physical_name)
            .then_some(physical_name);
    }
    if indices.len() == 1 {
        let physical_name = format!("{}[{}]", column_name, indices[0]);
        return expected_columns
            .contains(&physical_name)
            .then_some(physical_name);
    }
    None
}

fn source_fixed_assignment_index_path(
    expression: &Expression,
    values: &SourceFixedAssignmentValues<'_>,
) -> Option<(String, Vec<u32>)> {
    match &strip_source_fixed_group_expression(expression).kind {
        ExpressionKind::Name(column_name) => Some((column_name.clone(), Vec::new())),
        ExpressionKind::Index { target, index } => {
            let (column_name, mut indices) = source_fixed_assignment_index_path(target, values)?;
            let index = evaluate_source_fixed_assignment_value_expression(index, values)?;
            let index = source_fixed_assignment_integer(&index)?;
            let index = u32::try_from(index).ok()?;
            indices.push(index);
            Some((column_name, indices))
        }
        _ => None,
    }
}

fn source_fixed_linear_element_index(indices: &[u32], dimensions: &[u32]) -> Option<u32> {
    if indices.len() != dimensions.len() {
        return None;
    }
    indices
        .iter()
        .zip(dimensions)
        .try_fold(0_u32, |acc, (index, dimension)| {
            if index >= dimension {
                return None;
            }
            acc.checked_mul(*dimension)?.checked_add(*index)
        })
}

fn strip_source_fixed_group_expression(expression: &Expression) -> &Expression {
    match &expression.kind {
        ExpressionKind::Group(inner) => strip_source_fixed_group_expression(inner),
        _ => expression,
    }
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
