mod lexer;
pub mod parser;
mod source;
mod source_graph;
mod source_program;

pub use lexer::{lex_source, LexError, Token, TokenKind};
pub use parser::{
    evaluate_fixed_file_template_value_expression,
    evaluate_fixed_file_template_value_expression_with_values, parse_air_group_declarations,
    parse_air_group_value_declarations, parse_air_instance_declarations,
    parse_air_template_declarations, parse_column_declarations, parse_commit_declarations,
    parse_constant_declarations, parse_container_declarations, parse_expression,
    parse_fixed_file_pragmas, parse_function_declarations, parse_include_directives,
    parse_pragma_directives, parse_public_declarations, parse_public_table_declarations,
    parse_use_directives, parse_value_declarations, parse_variable_declarations,
    resolve_fixed_file_pragma_path, resolve_fixed_file_pragma_path_with_values,
    AirGroupDeclaration, AirGroupValueDeclaration, AirInstanceDeclaration, AirTemplateDeclaration,
    BinaryOperator, CallArgument, ColumnDeclaration, ColumnFeature, ColumnInitializer,
    ColumnInitializerKind, ColumnItem, ColumnKind, CommitDeclaration, ConstantDeclaration,
    ConstantDeclarationKind, ContainerDeclaration, Expression, ExpressionKind, FixedFilePragma,
    FixedFilePragmaKind, FixedFileTemplateContext, FixedFileTemplateValue, FunctionDeclaration,
    FunctionParameter, FunctionStatement, FunctionStatementDeclaration, FunctionStatementKind,
    FunctionVisibility, IncludeDirective, IncludeKind, IncludeVisibility, ParseError,
    PragmaDirective, PragmaTextValue, PublicDeclaration, PublicTableDeclaration, SourceSpan,
    UnaryOperator, UseDirective, ValueDeclaration, ValueDeclarationKind, VariableDeclaration,
};
pub use source::{SourceFile, SourceLoadError, SourceLoader, SourceLoaderConfig};
pub use source_graph::{
    collect_static_include_directives, SourceGraph, SourceGraphEdge, SourceGraphError,
    SourceGraphLoader,
};
pub use source_program::{
    build_source_program_archive, AirTemplateFixedFilePragma, SourceProgram, SourceProgramAirUnit,
    SourceProgramArchiveBuildError, SourceProgramArchiveLoadError, SourceProgramArchiveLoader,
    SourceProgramError, SourceProgramLoader, SourceProgramModule,
    SourceProgramResolvedFixedFilePragma,
};
