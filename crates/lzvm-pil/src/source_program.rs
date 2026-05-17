use crate::{
    parse_air_group_declarations, parse_air_group_value_declarations,
    parse_air_instance_declarations, parse_air_template_declarations, parse_column_declarations,
    parse_commit_declarations, parse_constant_declarations, parse_container_declarations,
    parse_fixed_file_pragmas, parse_function_declarations, parse_include_directives,
    parse_pragma_directives, parse_public_declarations, parse_public_table_declarations,
    parse_use_directives, parse_value_declarations, parse_variable_declarations,
    resolve_fixed_file_pragma_path_with_values, AirGroupDeclaration, AirGroupValueDeclaration,
    AirInstanceDeclaration, AirTemplateDeclaration, CallArgument, ColumnDeclaration,
    CommitDeclaration, ConstantDeclaration, ContainerDeclaration, Expression, ExpressionKind,
    FixedFilePragma, FixedFilePragmaKind, FixedFileTemplateContext, FixedFileTemplateValue,
    FunctionDeclaration, IncludeKind, IncludeVisibility, ParseError, PragmaDirective,
    PublicDeclaration, PublicTableDeclaration, SourceFile, SourceGraph, SourceGraphEdge,
    SourceGraphError, SourceGraphLoader, SourceLoaderConfig, UnaryOperator, UseDirective,
    ValueDeclaration, VariableDeclaration,
};
use lzvm_artifacts::source_program::{
    read_source_program_archive_file, SourceProgramArchive, SourceProgramArchiveEdge,
    SourceProgramArchiveError, SourceProgramArchiveIncludeKind,
    SourceProgramArchiveIncludeVisibility, SourceProgramArchiveSource,
};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceProgram {
    pub graph: SourceGraph,
    pub modules: Vec<SourceProgramModule>,
}

impl SourceProgram {
    pub fn air_units(&self) -> Vec<SourceProgramAirUnit> {
        self.air_unit_entries()
            .into_iter()
            .map(|entry| entry.unit)
            .collect()
    }

    fn air_unit_entries(&self) -> Vec<SourceProgramAirUnitEntry<'_>> {
        let group_ids = self.air_group_ids();
        let mut unit_counts = BTreeMap::<String, i128>::new();
        let mut virtual_unit_counts = BTreeMap::<String, i128>::new();
        let mut entries = Vec::new();

        for module in &self.modules {
            for instance in &module.air_instances {
                let group_id = group_ids.get(&instance.air_group).copied().unwrap_or(-1);
                let unit_id = if instance.virtual_instance {
                    let count = virtual_unit_counts
                        .entry(instance.air_group.clone())
                        .or_default();
                    let unit_id = VIRTUAL_UNIT_ID_BASE + *count;
                    *count += 1;
                    unit_id
                } else {
                    let count = unit_counts.entry(instance.air_group.clone()).or_default();
                    let unit_id = *count;
                    *count += 1;
                    unit_id
                };
                entries.push(SourceProgramAirUnitEntry {
                    instance,
                    unit: SourceProgramAirUnit {
                        source_name: instance.source_name.clone(),
                        group_name: instance.air_group.clone(),
                        group_id,
                        unit_id,
                        unit_name: instance
                            .alias
                            .clone()
                            .unwrap_or_else(|| instance.template.clone()),
                        template_name: instance.template.clone(),
                        virtual_instance: instance.virtual_instance,
                        start: instance.start,
                        end: instance.end,
                    },
                });
            }
        }

        entries
    }

    pub fn resolved_fixed_file_pragmas(
        &self,
    ) -> Result<Vec<SourceProgramResolvedFixedFilePragma>, ParseError> {
        let sources_by_name = self
            .modules
            .iter()
            .map(|module| (module.source_name.as_str(), &module.source))
            .collect::<BTreeMap<_, _>>();
        let mut templates_by_name = BTreeMap::<&str, &AirTemplateDeclaration>::new();
        let mut pragmas_by_template = BTreeMap::<&str, Vec<&AirTemplateFixedFilePragma>>::new();
        for module in &self.modules {
            for template in &module.air_templates {
                templates_by_name
                    .entry(template.name.as_str())
                    .or_insert(template);
            }
            for pragma in &module.air_template_fixed_file_pragmas {
                pragmas_by_template
                    .entry(pragma.template_name.as_str())
                    .or_default()
                    .push(pragma);
            }
        }

        let mut resolved = Vec::new();
        for entry in self.air_unit_entries() {
            let unit = entry.unit;
            let Some(pragmas) = pragmas_by_template.get(unit.template_name.as_str()) else {
                continue;
            };
            let context = FixedFileTemplateContext {
                group_name: unit.group_name.clone(),
                group_id: unit.group_id,
                unit_id: unit.unit_id,
                unit_name: unit.unit_name.clone(),
                template_name: unit.template_name.clone(),
            };
            let values = fixed_file_template_values(
                templates_by_name.get(unit.template_name.as_str()).copied(),
                entry.instance,
            );
            for scoped in pragmas {
                let Some(source) = sources_by_name.get(scoped.pragma.source_name.as_str()) else {
                    continue;
                };
                let path = resolve_fixed_file_pragma_path_with_values(
                    source,
                    &scoped.pragma,
                    &context,
                    &values,
                )?;
                resolved.push(SourceProgramResolvedFixedFilePragma {
                    source_name: scoped.pragma.source_name.clone(),
                    kind: scoped.pragma.kind,
                    path,
                    column: scoped.pragma.column,
                    group_name: unit.group_name.clone(),
                    group_id: unit.group_id,
                    unit_id: unit.unit_id,
                    unit_name: unit.unit_name.clone(),
                    template_name: unit.template_name.clone(),
                    virtual_instance: unit.virtual_instance,
                    start: scoped.pragma.start,
                    end: scoped.pragma.end,
                });
            }
        }

        Ok(resolved)
    }

    fn air_group_ids(&self) -> BTreeMap<String, i128> {
        let mut group_ids = BTreeMap::new();
        let mut next_group_id = 0_i128;
        for module in &self.modules {
            for group in &module.air_groups {
                if let std::collections::btree_map::Entry::Vacant(entry) =
                    group_ids.entry(group.name.clone())
                {
                    entry.insert(next_group_id);
                    next_group_id += 1;
                }
            }
        }
        group_ids
    }
}

const VIRTUAL_UNIT_ID_BASE: i128 = 10_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceProgramAirUnit {
    pub source_name: String,
    pub group_name: String,
    pub group_id: i128,
    pub unit_id: i128,
    pub unit_name: String,
    pub template_name: String,
    pub virtual_instance: bool,
    pub start: usize,
    pub end: usize,
}

struct SourceProgramAirUnitEntry<'a> {
    unit: SourceProgramAirUnit,
    instance: &'a AirInstanceDeclaration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceProgramResolvedFixedFilePragma {
    pub source_name: String,
    pub kind: FixedFilePragmaKind,
    pub path: Option<String>,
    pub column: Option<u32>,
    pub group_name: String,
    pub group_id: i128,
    pub unit_id: i128,
    pub unit_name: String,
    pub template_name: String,
    pub virtual_instance: bool,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceProgramModule {
    pub source_name: String,
    pub source: SourceFile,
    pub pragmas: Vec<PragmaDirective>,
    pub fixed_file_pragmas: Vec<FixedFilePragma>,
    pub air_template_fixed_file_pragmas: Vec<AirTemplateFixedFilePragma>,
    pub includes: Vec<crate::IncludeDirective>,
    pub uses: Vec<UseDirective>,
    pub containers: Vec<ContainerDeclaration>,
    pub constants: Vec<ConstantDeclaration>,
    pub variables: Vec<VariableDeclaration>,
    pub air_templates: Vec<AirTemplateDeclaration>,
    pub air_groups: Vec<AirGroupDeclaration>,
    pub air_instances: Vec<AirInstanceDeclaration>,
    pub functions: Vec<FunctionDeclaration>,
    pub columns: Vec<ColumnDeclaration>,
    pub values: Vec<ValueDeclaration>,
    pub air_group_values: Vec<AirGroupValueDeclaration>,
    pub commits: Vec<CommitDeclaration>,
    pub publics: Vec<PublicDeclaration>,
    pub public_tables: Vec<PublicTableDeclaration>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AirTemplateFixedFilePragma {
    pub template_name: String,
    pub pragma: FixedFilePragma,
}

fn fixed_file_template_values(
    template: Option<&AirTemplateDeclaration>,
    instance: &AirInstanceDeclaration,
) -> BTreeMap<String, FixedFileTemplateValue> {
    let Some(template) = template else {
        return BTreeMap::new();
    };
    let mut values = BTreeMap::new();
    for parameter in &template.parameters {
        if let Some(value) = parameter
            .default_expression
            .as_ref()
            .and_then(fixed_file_template_value_from_expression)
        {
            values.insert(parameter.name.clone(), value);
        }
    }
    if let Some(arguments) = instance.args_expressions.as_ref() {
        apply_fixed_file_template_call_arguments(template, arguments, &mut values);
    }
    values
}

fn apply_fixed_file_template_call_arguments(
    template: &AirTemplateDeclaration,
    arguments: &[CallArgument],
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
) {
    let mut positional_index = 0;
    for argument in arguments {
        let Some(value) = fixed_file_template_value_from_expression(&argument.value) else {
            continue;
        };
        if let Some(name) = argument.name.as_ref() {
            values.insert(name.clone(), value);
            continue;
        }
        if let Some(parameter) = template.parameters.get(positional_index) {
            values.insert(parameter.name.clone(), value);
        }
        positional_index += 1;
    }
}

fn fixed_file_template_value_from_expression(
    expression: &Expression,
) -> Option<FixedFileTemplateValue> {
    match &expression.kind {
        ExpressionKind::Integer(value) => value
            .parse::<i128>()
            .ok()
            .map(FixedFileTemplateValue::Integer),
        ExpressionKind::HexInteger(value) => value
            .strip_prefix("0x")
            .and_then(|digits| i128::from_str_radix(digits, 16).ok())
            .map(FixedFileTemplateValue::Integer),
        ExpressionKind::StringLiteral(value) | ExpressionKind::TemplateLiteral(value) => {
            Some(FixedFileTemplateValue::String(value.clone()))
        }
        ExpressionKind::Group(inner) => fixed_file_template_value_from_expression(inner),
        ExpressionKind::Unary { op, expr } => {
            let value = fixed_file_template_value_from_expression(expr)?;
            match (op, value) {
                (UnaryOperator::Plus, FixedFileTemplateValue::Integer(value)) => {
                    Some(FixedFileTemplateValue::Integer(value))
                }
                (UnaryOperator::Minus, FixedFileTemplateValue::Integer(value)) => {
                    value.checked_neg().map(FixedFileTemplateValue::Integer)
                }
                (UnaryOperator::Not, FixedFileTemplateValue::Integer(value)) => {
                    Some(FixedFileTemplateValue::Boolean(value == 0))
                }
                (UnaryOperator::Not, FixedFileTemplateValue::Boolean(value)) => {
                    Some(FixedFileTemplateValue::Boolean(!value))
                }
                _ => None,
            }
        }
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceProgramError {
    Graph(SourceGraphError),
    Parse(ParseError),
}

impl std::fmt::Display for SourceProgramError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Graph(error) => write!(f, "{error}"),
            Self::Parse(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for SourceProgramError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceProgramArchiveLoadError {
    Archive(SourceProgramArchiveError),
    InvalidSourceIndex { index: u32 },
    Parse(ParseError),
}

impl std::fmt::Display for SourceProgramArchiveLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Archive(error) => write!(f, "{error}"),
            Self::InvalidSourceIndex { index } => {
                write!(
                    f,
                    "source program archive references missing source index {index}"
                )
            }
            Self::Parse(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for SourceProgramArchiveLoadError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceProgramArchiveBuildError {
    SourceCountOverflow,
    MissingSourceIndex { source_name: String },
}

impl std::fmt::Display for SourceProgramArchiveBuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SourceCountOverflow => {
                write!(f, "source program has too many sources to archive")
            }
            Self::MissingSourceIndex { source_name } => {
                write!(
                    f,
                    "source program archive edge references missing source {source_name}"
                )
            }
        }
    }
}

impl std::error::Error for SourceProgramArchiveBuildError {}

pub struct SourceProgramLoader {
    graph_loader: SourceGraphLoader,
}

impl SourceProgramLoader {
    pub fn new(config: SourceLoaderConfig) -> Self {
        Self {
            graph_loader: SourceGraphLoader::new(config),
        }
    }

    pub fn load_main(
        &mut self,
        file_name: impl AsRef<Path>,
    ) -> Result<SourceProgram, SourceProgramError> {
        let graph = self
            .graph_loader
            .load_main(file_name)
            .map_err(SourceProgramError::Graph)?;
        let modules = graph
            .sources
            .iter()
            .map(parse_source_module)
            .collect::<Result<Vec<_>, _>>()
            .map_err(SourceProgramError::Parse)?;

        Ok(SourceProgram { graph, modules })
    }
}

pub struct SourceProgramArchiveLoader;

impl SourceProgramArchiveLoader {
    pub fn load(path: impl AsRef<Path>) -> Result<SourceProgram, SourceProgramArchiveLoadError> {
        let archive = read_source_program_archive_file(path)
            .map_err(SourceProgramArchiveLoadError::Archive)?;
        build_source_program_from_archive(&archive)
    }
}

pub fn build_source_program_archive(
    program: &SourceProgram,
) -> Result<SourceProgramArchive, SourceProgramArchiveBuildError> {
    let sources = program
        .graph
        .sources
        .iter()
        .map(|source| SourceProgramArchiveSource {
            source_name: source.source_name.clone(),
            contents: source.contents.clone(),
        })
        .collect::<Vec<_>>();
    let mut source_indexes = BTreeMap::new();
    for (index, source) in program.graph.sources.iter().enumerate() {
        let index = u32::try_from(index)
            .map_err(|_| SourceProgramArchiveBuildError::SourceCountOverflow)?;
        source_indexes.insert(source.source_name.clone(), index);
    }

    let mut edges = Vec::with_capacity(program.graph.edges.len());
    for edge in &program.graph.edges {
        let from_index = source_indexes.get(&edge.from).copied().ok_or_else(|| {
            SourceProgramArchiveBuildError::MissingSourceIndex {
                source_name: edge.from.clone(),
            }
        })?;
        let to_index = source_indexes.get(&edge.to).copied().ok_or_else(|| {
            SourceProgramArchiveBuildError::MissingSourceIndex {
                source_name: edge.to.clone(),
            }
        })?;
        edges.push(SourceProgramArchiveEdge {
            from_index,
            to_index,
            request: edge.request.clone(),
            kind: source_program_archive_include_kind(edge.kind),
            visibility: source_program_archive_include_visibility(edge.visibility),
        });
    }

    Ok(SourceProgramArchive { sources, edges })
}

fn parse_source_module(source: &SourceFile) -> Result<SourceProgramModule, ParseError> {
    let fixed_file_pragmas = parse_fixed_file_pragmas(source)?;
    let air_templates = parse_air_template_declarations(source)?;
    let air_template_fixed_file_pragmas =
        collect_air_template_fixed_file_pragmas(&air_templates, &fixed_file_pragmas);

    Ok(SourceProgramModule {
        source_name: source.source_name.clone(),
        source: source.clone(),
        pragmas: parse_pragma_directives(source)?,
        fixed_file_pragmas,
        air_template_fixed_file_pragmas,
        includes: parse_include_directives(source)?,
        uses: parse_use_directives(source)?,
        containers: parse_container_declarations(source)?,
        constants: parse_constant_declarations(source)?,
        variables: parse_variable_declarations(source)?,
        air_templates,
        air_groups: parse_air_group_declarations(source)?,
        air_instances: parse_air_instance_declarations(source)?,
        functions: parse_function_declarations(source)?,
        columns: parse_column_declarations(source)?,
        values: parse_value_declarations(source)?,
        air_group_values: parse_air_group_value_declarations(source)?,
        commits: parse_commit_declarations(source)?,
        publics: parse_public_declarations(source)?,
        public_tables: parse_public_table_declarations(source)?,
    })
}

fn collect_air_template_fixed_file_pragmas(
    air_templates: &[AirTemplateDeclaration],
    pragmas: &[FixedFilePragma],
) -> Vec<AirTemplateFixedFilePragma> {
    let mut scoped = Vec::new();
    for template in air_templates {
        for pragma in pragmas {
            if pragma.start >= template.body.start && pragma.end <= template.body.end {
                scoped.push(AirTemplateFixedFilePragma {
                    template_name: template.name.clone(),
                    pragma: pragma.clone(),
                });
            }
        }
    }
    scoped
}

fn build_source_program_from_archive(
    archive: &SourceProgramArchive,
) -> Result<SourceProgram, SourceProgramArchiveLoadError> {
    let mut sources = Vec::with_capacity(archive.sources.len());
    for source in &archive.sources {
        sources.push(SourceFile {
            contents: source.contents.clone(),
            file_dir: Path::new(".").to_path_buf(),
            full_path: Path::new(&source.source_name).to_path_buf(),
            source_name: source.source_name.clone(),
        });
    }

    let mut edges = Vec::with_capacity(archive.edges.len());
    for edge in &archive.edges {
        let from = archive_source_name(archive, edge.from_index)?;
        let to = archive_source_name(archive, edge.to_index)?;
        edges.push(SourceGraphEdge {
            from,
            to,
            request: edge.request.clone(),
            kind: archive_include_kind(edge.kind),
            visibility: archive_include_visibility(edge.visibility),
        });
    }

    let modules = sources
        .iter()
        .map(parse_source_module)
        .collect::<Result<Vec<_>, _>>()
        .map_err(SourceProgramArchiveLoadError::Parse)?;

    Ok(SourceProgram {
        graph: SourceGraph { sources, edges },
        modules,
    })
}

fn archive_source_name(
    archive: &SourceProgramArchive,
    index: u32,
) -> Result<String, SourceProgramArchiveLoadError> {
    let index = usize::try_from(index)
        .map_err(|_| SourceProgramArchiveLoadError::InvalidSourceIndex { index })?;
    let source =
        archive
            .sources
            .get(index)
            .ok_or(SourceProgramArchiveLoadError::InvalidSourceIndex {
                index: u32::try_from(index).unwrap_or(u32::MAX),
            })?;
    Ok(source.source_name.clone())
}

fn archive_include_kind(kind: SourceProgramArchiveIncludeKind) -> IncludeKind {
    match kind {
        SourceProgramArchiveIncludeKind::Include => IncludeKind::Include,
        SourceProgramArchiveIncludeKind::Require => IncludeKind::Require,
    }
}

fn archive_include_visibility(
    visibility: SourceProgramArchiveIncludeVisibility,
) -> IncludeVisibility {
    match visibility {
        SourceProgramArchiveIncludeVisibility::Public => IncludeVisibility::Public,
        SourceProgramArchiveIncludeVisibility::Private => IncludeVisibility::Private,
    }
}

fn source_program_archive_include_kind(kind: IncludeKind) -> SourceProgramArchiveIncludeKind {
    match kind {
        IncludeKind::Include => SourceProgramArchiveIncludeKind::Include,
        IncludeKind::Require => SourceProgramArchiveIncludeKind::Require,
    }
}

fn source_program_archive_include_visibility(
    visibility: IncludeVisibility,
) -> SourceProgramArchiveIncludeVisibility {
    match visibility {
        IncludeVisibility::Public => SourceProgramArchiveIncludeVisibility::Public,
        IncludeVisibility::Private => SourceProgramArchiveIncludeVisibility::Private,
    }
}
