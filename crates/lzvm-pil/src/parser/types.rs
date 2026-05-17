use std::fmt;

use crate::LexError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceSpan {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PragmaDirective {
    pub value: String,
    pub source_name: String,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IncludeKind {
    Include,
    Require,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IncludeVisibility {
    Public,
    Private,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncludeDirective {
    pub kind: IncludeKind,
    pub visibility: IncludeVisibility,
    pub file: String,
    pub source_name: String,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UseDirective {
    pub name: String,
    pub alias: Option<String>,
    pub source_name: String,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContainerDeclaration {
    pub name: String,
    pub alias: Option<String>,
    pub body: Option<SourceSpan>,
    pub source_name: String,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AirTemplateDeclaration {
    pub name: String,
    pub params: Option<SourceSpan>,
    pub parameters: Vec<FunctionParameter>,
    pub body: SourceSpan,
    pub source_name: String,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AirGroupDeclaration {
    pub name: String,
    pub body: SourceSpan,
    pub source_name: String,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AirInstanceDeclaration {
    pub air_group: String,
    pub template: String,
    pub alias: Option<String>,
    pub virtual_instance: bool,
    pub args: SourceSpan,
    pub args_expressions: Option<Vec<CallArgument>>,
    pub source_name: String,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FunctionVisibility {
    Public,
    Private,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionDeclaration {
    pub name: String,
    pub visibility: Option<FunctionVisibility>,
    pub params: SourceSpan,
    pub parameters: Vec<FunctionParameter>,
    pub return_type: Option<SourceSpan>,
    pub body: SourceSpan,
    pub statements: Vec<FunctionStatement>,
    pub source_name: String,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionParameter {
    pub is_const: bool,
    pub by_reference: bool,
    pub type_name: String,
    pub name: String,
    pub array_dims: Vec<SourceSpan>,
    pub default_value: Option<SourceSpan>,
    pub default_expression: Option<Expression>,
    pub source_name: String,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FunctionStatementKind {
    Return,
    If,
    ElseIf,
    Else,
    For,
    While,
    Do,
    Switch,
    Break,
    Continue,
    Declaration,
    Block,
    Expression,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FunctionStatementDeclaration {
    Constant(ConstantDeclaration),
    Variable(VariableDeclaration),
    Column(ColumnDeclaration),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionStatement {
    pub kind: FunctionStatementKind,
    pub declaration: Option<FunctionStatementDeclaration>,
    pub header: Option<SourceSpan>,
    pub body: Option<SourceSpan>,
    pub value: Option<SourceSpan>,
    pub header_expression: Option<Expression>,
    pub value_expression: Option<Expression>,
    pub source_name: String,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expression {
    pub kind: ExpressionKind,
    pub source_name: String,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallArgument {
    pub name: Option<String>,
    pub value: Expression,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOperator {
    Plus,
    Minus,
    Not,
    Increment,
    Decrement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOperator {
    Power,
    Multiply,
    Divide,
    Modulo,
    Backslash,
    Add,
    Subtract,
    ShiftLeft,
    ShiftRight,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    EqualEqual,
    NotEqual,
    TripleEqual,
    ConstrainedAssign,
    BitAnd,
    BitXor,
    BitOr,
    LogicalAnd,
    LogicalOr,
    Assign,
    PlusAssign,
    MinusAssign,
    StarAssign,
    Range,
    RangeFill,
    RangeMulFill,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpressionKind {
    Integer(String),
    HexInteger(String),
    StringLiteral(String),
    TemplateLiteral(String),
    Name(String),
    PositionalParam(String),
    Group(Box<Expression>),
    Unary {
        op: UnaryOperator,
        expr: Box<Expression>,
    },
    Binary {
        op: BinaryOperator,
        left: Box<Expression>,
        right: Box<Expression>,
    },
    Call {
        callee: Box<Expression>,
        args: Vec<CallArgument>,
    },
    Index {
        target: Box<Expression>,
        index: Box<Expression>,
    },
    RowOffset {
        target: Box<Expression>,
        offset: Box<Expression>,
        prior: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnKind {
    Witness,
    Fixed,
    Custom,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnFeature {
    pub name: String,
    pub args: SourceSpan,
    pub args_expressions: Option<Vec<Expression>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnItem {
    pub name: String,
    pub template: bool,
    pub array_dims: Vec<SourceSpan>,
    pub array_dim_expressions: Vec<Option<Expression>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnInitializerKind {
    Expression,
    Sequence,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnInitializer {
    pub kind: ColumnInitializerKind,
    pub span: SourceSpan,
    pub expression: Option<Expression>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnDeclaration {
    pub kind: ColumnKind,
    pub commit: Option<String>,
    pub features: Vec<ColumnFeature>,
    pub items: Vec<ColumnItem>,
    pub initializer: Option<ColumnInitializer>,
    pub source_name: String,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstantDeclarationKind {
    Constant,
    Const,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstantDeclaration {
    pub kind: ConstantDeclarationKind,
    pub type_name: Option<String>,
    pub name: String,
    pub array_dims: Vec<SourceSpan>,
    pub array_dim_expressions: Vec<Option<Expression>>,
    pub initializer: Option<SourceSpan>,
    pub initializer_expression: Option<Expression>,
    pub source_name: String,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableDeclaration {
    pub type_name: String,
    pub name: String,
    pub array_dims: Vec<SourceSpan>,
    pub array_dim_expressions: Vec<Option<Expression>>,
    pub initializer: Option<SourceSpan>,
    pub initializer_expression: Option<Expression>,
    pub source_name: String,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueDeclarationKind {
    Challenge,
    ProofValue,
    AirValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueDeclaration {
    pub kind: ValueDeclarationKind,
    pub stage: u32,
    pub items: Vec<ColumnItem>,
    pub source_name: String,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AirGroupValueDeclaration {
    pub stage: u32,
    pub default_value: Option<SourceSpan>,
    pub default_expression: Option<Expression>,
    pub aggregate_type: Option<String>,
    pub items: Vec<ColumnItem>,
    pub source_name: String,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitDeclaration {
    pub stage: u32,
    pub publics: Vec<String>,
    pub name: String,
    pub source_name: String,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicDeclaration {
    pub items: Vec<ColumnItem>,
    pub initializer: Option<SourceSpan>,
    pub initializer_expression: Option<Expression>,
    pub source_name: String,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicTableDeclaration {
    pub aggregate_type: String,
    pub aggregate_function: String,
    pub name: String,
    pub args: Option<SourceSpan>,
    pub args_expressions: Option<Vec<Expression>>,
    pub cols: SourceSpan,
    pub cols_expression: Option<Expression>,
    pub rows: SourceSpan,
    pub rows_expression: Option<Expression>,
    pub source_name: String,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    Lex {
        source_name: String,
        error: LexError,
    },
    ExpectedPath {
        source_name: String,
        start: usize,
    },
    TemplatePath {
        source_name: String,
        start: usize,
        end: usize,
    },
    ExpectedTerminator {
        source_name: String,
        start: usize,
    },
    ExpectedName {
        source_name: String,
        start: usize,
    },
    ExpectedAlias {
        source_name: String,
        start: usize,
    },
    ExpectedNumber {
        source_name: String,
        start: usize,
    },
    DuplicateProperty {
        source_name: String,
        start: usize,
        name: &'static str,
    },
    ExpectedCloseBrace {
        source_name: String,
        start: usize,
    },
    ExpectedCloseParen {
        source_name: String,
        start: usize,
    },
    ExpectedCloseBracket {
        source_name: String,
        start: usize,
    },
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Lex { source_name, error } => write!(f, "{source_name}: {error}"),
            Self::ExpectedPath { source_name, start } => {
                write!(f, "{source_name}: expected include path at {start}")
            }
            Self::TemplatePath {
                source_name, start, ..
            } => write!(f, "{source_name}: template include path at {start}"),
            Self::ExpectedTerminator { source_name, start } => {
                write!(f, "{source_name}: expected statement terminator at {start}")
            }
            Self::ExpectedName { source_name, start } => {
                write!(f, "{source_name}: expected name at {start}")
            }
            Self::ExpectedAlias { source_name, start } => {
                write!(f, "{source_name}: expected alias at {start}")
            }
            Self::ExpectedNumber { source_name, start } => {
                write!(f, "{source_name}: expected number at {start}")
            }
            Self::DuplicateProperty {
                source_name,
                start,
                name,
            } => write!(f, "{source_name}: duplicate {name} property at {start}"),
            Self::ExpectedCloseBrace { source_name, start } => {
                write!(f, "{source_name}: expected closing brace at {start}")
            }
            Self::ExpectedCloseParen { source_name, start } => {
                write!(f, "{source_name}: expected closing parenthesis at {start}")
            }
            Self::ExpectedCloseBracket { source_name, start } => {
                write!(f, "{source_name}: expected closing bracket at {start}")
            }
        }
    }
}

impl std::error::Error for ParseError {}
