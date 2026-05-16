mod lexer;
pub mod parser;
mod source;
mod source_graph;
mod source_program;

pub use lexer::{lex_source, LexError, Token, TokenKind};
pub use parser::{
    parse_air_group_value_declarations, parse_column_declarations, parse_commit_declarations,
    parse_container_declarations, parse_include_directives, parse_public_declarations,
    parse_public_table_declarations, parse_use_directives, parse_value_declarations,
    AirGroupValueDeclaration, ColumnDeclaration, ColumnFeature, ColumnInitializer,
    ColumnInitializerKind, ColumnItem, ColumnKind, CommitDeclaration, ContainerDeclaration,
    IncludeDirective, IncludeKind, IncludeVisibility, ParseError, PublicDeclaration,
    PublicTableDeclaration, SourceSpan, UseDirective, ValueDeclaration, ValueDeclarationKind,
};
pub use source::{SourceFile, SourceLoadError, SourceLoader, SourceLoaderConfig};
pub use source_graph::{
    collect_static_include_directives, SourceGraph, SourceGraphEdge, SourceGraphError,
    SourceGraphLoader,
};
pub use source_program::{
    SourceProgram, SourceProgramError, SourceProgramLoader, SourceProgramModule,
};
