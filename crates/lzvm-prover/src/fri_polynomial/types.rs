use std::fmt;

use lzvm_field::{Ext3, Felt};

#[derive(Debug, Clone, Copy, Default)]
pub struct FriPolynomialColumnMatrix<'a> {
    pub column_count: usize,
    pub values: &'a [Felt],
}

#[derive(Debug, Clone, Copy)]
pub struct FriPolynomialStageColumns<'a> {
    pub stage_index: u16,
    pub column_count: usize,
    pub values: &'a [Felt],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FriPolynomialZerofierTable {
    pub column_count: usize,
    pub values: Vec<Felt>,
}

impl FriPolynomialZerofierTable {
    pub fn as_matrix(&self) -> FriPolynomialColumnMatrix<'_> {
        FriPolynomialColumnMatrix {
            column_count: self.column_count,
            values: &self.values,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct FriPolynomialInputs<'a> {
    pub domain_size: usize,
    pub stage_count: u16,
    pub fixed_columns: FriPolynomialColumnMatrix<'a>,
    pub stage_columns: &'a [FriPolynomialStageColumns<'a>],
    pub custom_fixed_columns: &'a [FriPolynomialColumnMatrix<'a>],
    pub opening_point_offsets: &'a [i64],
    pub domain_points: &'a [Felt],
    pub zerofier_values: FriPolynomialColumnMatrix<'a>,
    pub opening_xis: &'a [Ext3],
    pub publics: &'a [Felt],
    pub unit_values: &'a [Felt],
    pub proof_values: &'a [Felt],
    pub group_values: &'a [Ext3],
    pub challenges: &'a [Ext3],
    pub evaluations: &'a [Ext3],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FriPolynomialError {
    EmptyDomain,
    MissingExpression {
        expression_id: u32,
    },
    LengthOverflow,
    OperationSpanOutOfBounds {
        expression_id: u32,
    },
    ArgumentSpanOutOfBounds {
        expression_id: u32,
    },
    ArgumentCountMismatch {
        expression_id: u32,
        consumed: usize,
        declared: usize,
    },
    UnsupportedOperationShape {
        shape: u8,
    },
    UnsupportedOperationKind {
        kind: u16,
    },
    UnsupportedDestinationDimension {
        dimension: u32,
    },
    UnsupportedDomainBits {
        bits: u32,
    },
    UnsupportedBoundary {
        boundary_index: usize,
        name: Option<String>,
    },
    MissingBoundaryOffset {
        boundary_index: usize,
        field: &'static str,
    },
    InvalidBoundaryOffset {
        boundary_index: usize,
        field: &'static str,
        value: i64,
    },
    ZeroZerofierDenominator {
        boundary_index: usize,
        row: usize,
    },
    UnsupportedSourceBuffer {
        buffer: u16,
    },
    MissingStageColumns {
        stage_index: u16,
    },
    MatrixLengthMismatch {
        buffer: &'static str,
        expected: usize,
        found: usize,
    },
    ZeroDenominator {
        opening_index: usize,
    },
    NonCanonicalNumber {
        value: u64,
    },
    SourceIndexOutOfRange {
        buffer: &'static str,
        offset: usize,
        width: usize,
        len: usize,
    },
}

impl fmt::Display for FriPolynomialError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyDomain => write!(f, "FRI polynomial domain is empty"),
            Self::MissingExpression { expression_id } => {
                write!(f, "FRI polynomial expression {expression_id} is missing")
            }
            Self::LengthOverflow => write!(f, "FRI polynomial length overflow"),
            Self::OperationSpanOutOfBounds { expression_id } => write!(
                f,
                "FRI polynomial expression {expression_id} operation span is out of bounds"
            ),
            Self::ArgumentSpanOutOfBounds { expression_id } => write!(
                f,
                "FRI polynomial expression {expression_id} argument span is out of bounds"
            ),
            Self::ArgumentCountMismatch {
                expression_id,
                consumed,
                declared,
            } => write!(
                f,
                "FRI polynomial expression {expression_id} consumed {consumed} arguments, declared {declared}"
            ),
            Self::UnsupportedOperationShape { shape } => {
                write!(f, "unsupported FRI polynomial operation shape: {shape}")
            }
            Self::UnsupportedOperationKind { kind } => {
                write!(f, "unsupported FRI polynomial operation kind: {kind}")
            }
            Self::UnsupportedDestinationDimension { dimension } => write!(
                f,
                "unsupported FRI polynomial destination dimension: {dimension}"
            ),
            Self::UnsupportedDomainBits { bits } => {
                write!(f, "unsupported FRI polynomial domain bits: {bits}")
            }
            Self::UnsupportedBoundary {
                boundary_index,
                name,
            } => write!(
                f,
                "unsupported FRI polynomial boundary {boundary_index}: {name:?}"
            ),
            Self::MissingBoundaryOffset {
                boundary_index,
                field,
            } => write!(
                f,
                "missing FRI polynomial boundary {boundary_index} offset field {field}"
            ),
            Self::InvalidBoundaryOffset {
                boundary_index,
                field,
                value,
            } => write!(
                f,
                "invalid FRI polynomial boundary {boundary_index} offset field {field}: {value}"
            ),
            Self::ZeroZerofierDenominator {
                boundary_index,
                row,
            } => write!(
                f,
                "FRI polynomial zerofier denominator is zero at boundary {boundary_index}, row {row}"
            ),
            Self::UnsupportedSourceBuffer { buffer } => {
                write!(f, "unsupported FRI polynomial source buffer: {buffer}")
            }
            Self::MissingStageColumns { stage_index } => {
                write!(f, "missing FRI polynomial stage columns: {stage_index}")
            }
            Self::MatrixLengthMismatch {
                buffer,
                expected,
                found,
            } => write!(
                f,
                "FRI polynomial {buffer} matrix length mismatch: expected {expected}, found {found}"
            ),
            Self::ZeroDenominator { opening_index } => write!(
                f,
                "FRI polynomial opening denominator {opening_index} is zero"
            ),
            Self::NonCanonicalNumber { value } => {
                write!(f, "non-canonical FRI polynomial number: {value}")
            }
            Self::SourceIndexOutOfRange {
                buffer,
                offset,
                width,
                len,
            } => write!(
                f,
                "FRI polynomial {buffer} offset {offset} with width {width} is outside length {len}"
            ),
        }
    }
}

impl std::error::Error for FriPolynomialError {}
