mod lexer;
pub mod parser;
mod source;
mod source_graph;

pub use lexer::{lex_source, LexError, Token, TokenKind};
pub use parser::{
    parse_container_declarations, parse_include_directives, ContainerDeclaration, IncludeDirective,
    IncludeKind, IncludeVisibility, ParseError, SourceSpan, UseDirective,
};
pub use source::{SourceFile, SourceLoadError, SourceLoader, SourceLoaderConfig};
pub use source_graph::{
    collect_static_include_directives, SourceGraph, SourceGraphEdge, SourceGraphError,
    SourceGraphLoader,
};
