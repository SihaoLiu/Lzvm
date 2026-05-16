use crate::{
    parse_air_group_value_declarations, parse_column_declarations, parse_commit_declarations,
    parse_container_declarations, parse_include_directives, parse_public_declarations,
    parse_public_table_declarations, parse_use_directives, parse_value_declarations,
    AirGroupValueDeclaration, ColumnDeclaration, CommitDeclaration, ContainerDeclaration,
    ParseError, PublicDeclaration, PublicTableDeclaration, SourceFile, SourceGraph,
    SourceGraphError, SourceGraphLoader, SourceLoaderConfig, UseDirective, ValueDeclaration,
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

fn parse_source_module(source: &SourceFile) -> Result<SourceProgramModule, ParseError> {
    Ok(SourceProgramModule {
        source_name: source.source_name.clone(),
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
