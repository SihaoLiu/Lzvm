use crate::{
    parse_air_group_declarations, parse_air_group_value_declarations,
    parse_air_template_declarations, parse_column_declarations, parse_commit_declarations,
    parse_container_declarations, parse_include_directives, parse_pragma_directives,
    parse_public_declarations, parse_public_table_declarations, parse_use_directives,
    parse_value_declarations, AirGroupDeclaration, AirGroupValueDeclaration,
    AirTemplateDeclaration, ColumnDeclaration, CommitDeclaration, ContainerDeclaration,
    IncludeKind, IncludeVisibility, ParseError, PragmaDirective, PublicDeclaration,
    PublicTableDeclaration, SourceFile, SourceGraph, SourceGraphEdge, SourceGraphError,
    SourceGraphLoader, SourceLoaderConfig, UseDirective, ValueDeclaration,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceProgramModule {
    pub source_name: String,
    pub source: SourceFile,
    pub pragmas: Vec<PragmaDirective>,
    pub includes: Vec<crate::IncludeDirective>,
    pub uses: Vec<UseDirective>,
    pub containers: Vec<ContainerDeclaration>,
    pub air_templates: Vec<AirTemplateDeclaration>,
    pub air_groups: Vec<AirGroupDeclaration>,
    pub columns: Vec<ColumnDeclaration>,
    pub values: Vec<ValueDeclaration>,
    pub air_group_values: Vec<AirGroupValueDeclaration>,
    pub commits: Vec<CommitDeclaration>,
    pub publics: Vec<PublicDeclaration>,
    pub public_tables: Vec<PublicTableDeclaration>,
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
    Ok(SourceProgramModule {
        source_name: source.source_name.clone(),
        source: source.clone(),
        pragmas: parse_pragma_directives(source)?,
        includes: parse_include_directives(source)?,
        uses: parse_use_directives(source)?,
        containers: parse_container_declarations(source)?,
        air_templates: parse_air_template_declarations(source)?,
        air_groups: parse_air_group_declarations(source)?,
        columns: parse_column_declarations(source)?,
        values: parse_value_declarations(source)?,
        air_group_values: parse_air_group_value_declarations(source)?,
        commits: parse_commit_declarations(source)?,
        publics: parse_public_declarations(source)?,
        public_tables: parse_public_table_declarations(source)?,
    })
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
