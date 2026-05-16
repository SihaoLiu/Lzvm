use crate::{
    parse_air_group_value_declarations, parse_column_declarations, parse_commit_declarations,
    parse_container_declarations, parse_include_directives, parse_public_declarations,
    parse_public_table_declarations, parse_use_directives, parse_value_declarations,
    AirGroupValueDeclaration, ColumnDeclaration, CommitDeclaration, ContainerDeclaration,
    IncludeKind, IncludeVisibility, ParseError, PublicDeclaration, PublicTableDeclaration,
    SourceFile, SourceGraph, SourceGraphEdge, SourceGraphError, SourceGraphLoader,
    SourceLoaderConfig, UseDirective, ValueDeclaration,
};
use lzvm_artifacts::source_program::{
    read_source_program_archive_file, SourceProgramArchive, SourceProgramArchiveError,
    SourceProgramArchiveIncludeKind, SourceProgramArchiveIncludeVisibility,
};
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
    pub includes: Vec<crate::IncludeDirective>,
    pub uses: Vec<UseDirective>,
    pub containers: Vec<ContainerDeclaration>,
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

fn parse_source_module(source: &SourceFile) -> Result<SourceProgramModule, ParseError> {
    Ok(SourceProgramModule {
        source_name: source.source_name.clone(),
        source: source.clone(),
        includes: parse_include_directives(source)?,
        uses: parse_use_directives(source)?,
        containers: parse_container_declarations(source)?,
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
