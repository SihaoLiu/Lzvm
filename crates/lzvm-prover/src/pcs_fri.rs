use std::fmt;

use lzvm_artifacts::pcs_fri_segment::{
    PcsFriOpeningLayerSegment, PcsFriOpeningLevelSegment, PcsFriOpeningQuerySegment,
    PcsFriOpeningUnitSegment,
};
use lzvm_artifacts::setup_info::StageValue;
use lzvm_field::{intt_in_place, DomainError, Ext3, Felt, FieldError, PoseidonTranscript, SHIFT};

use crate::merkle_hash::{
    linear_hash, parent_hash, root_from_digest_level, MerkleHashError, HASH_WORDS,
};
use crate::pcs_transcript::{absorb_commit_values, PcsTranscriptError};
use crate::ProveUnitSchedule;

#[derive(Debug, Clone, Copy)]
pub struct PcsFriOpeningFoldRequest<'a> {
    pub unit_index: u32,
    pub query_rows: &'a [u64],
    pub challenges: &'a [Ext3],
    pub fri: &'a PcsFriOpeningUnitSegment,
}

#[derive(Debug, Clone, Copy)]
pub struct PcsFriOpeningBuildRequest<'a> {
    pub unit_index: u32,
    pub query_rows: &'a [u64],
    pub challenges: &'a [Ext3],
    pub polynomial: &'a [Ext3],
}

#[derive(Debug, Clone, Copy)]
pub struct PcsFriTranscriptCommitmentRequest<'a> {
    pub arity: usize,
    pub hash_values: bool,
    pub constant_root: [Felt; 4],
    pub public_values: &'a [Felt],
    pub witness_roots: &'a [[Felt; 4]],
    pub root_challenge_draws: &'a [usize],
    pub unit_value_map: &'a [StageValue],
    pub unit_values: &'a [Felt],
    pub evaluation_values: &'a [Ext3],
    pub evaluation_challenge_draws: usize,
    pub polynomial: &'a [Ext3],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PcsFriTranscriptCommitments {
    pub challenges: Vec<Ext3>,
    pub layer_roots: Vec<[Felt; 4]>,
    pub final_polynomial: Vec<Ext3>,
    pub final_query_challenge: Ext3,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PcsFriFoldError {
    InvalidLayerBits { current_bits: u32, prev_bits: u32 },
    InvalidExtensionBits { n_bits_ext: u32, prev_bits: u32 },
    ValueLengthMismatch { expected: usize, found: usize },
    UnsupportedRoot { bits: u32 },
    ZeroEvaluationPoint,
    Domain(DomainError),
    Field(FieldError),
    LengthOverflow,
}

impl fmt::Display for PcsFriFoldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLayerBits {
                current_bits,
                prev_bits,
            } => write!(
                f,
                "PCS FRI fold layer bits are invalid: current {current_bits}, previous {prev_bits}"
            ),
            Self::InvalidExtensionBits {
                n_bits_ext,
                prev_bits,
            } => write!(
                f,
                "PCS FRI fold extension bits are invalid: extended {n_bits_ext}, previous {prev_bits}"
            ),
            Self::ValueLengthMismatch { expected, found } => write!(
                f,
                "PCS FRI fold value length mismatch: expected {expected}, found {found}"
            ),
            Self::UnsupportedRoot { bits } => {
                write!(f, "PCS FRI fold root is unsupported for bits {bits}")
            }
            Self::ZeroEvaluationPoint => write!(f, "PCS FRI fold evaluation point is zero"),
            Self::Domain(error) => write!(f, "PCS FRI fold domain error: {error}"),
            Self::Field(error) => write!(f, "PCS FRI fold field error: {error}"),
            Self::LengthOverflow => write!(f, "PCS FRI fold length overflow"),
        }
    }
}

impl std::error::Error for PcsFriFoldError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Domain(error) => Some(error),
            Self::Field(error) => Some(error),
            Self::InvalidLayerBits { .. }
            | Self::InvalidExtensionBits { .. }
            | Self::ValueLengthMismatch { .. }
            | Self::UnsupportedRoot { .. }
            | Self::ZeroEvaluationPoint
            | Self::LengthOverflow => None,
        }
    }
}

impl From<DomainError> for PcsFriFoldError {
    fn from(error: DomainError) -> Self {
        Self::Domain(error)
    }
}

impl From<FieldError> for PcsFriFoldError {
    fn from(error: FieldError) -> Self {
        Self::Field(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PcsFriOpeningBuildError {
    EmptyFriLayers,
    QueryRowCountMismatch {
        expected: usize,
        found: usize,
    },
    InvalidLayerBits {
        layer_index: usize,
        input_bits: u32,
        output_bits: u32,
    },
    LayerInputMismatch {
        layer_index: usize,
        expected: u32,
        found: u32,
    },
    FoldingFactorMismatch {
        layer_index: usize,
        expected: usize,
        found: usize,
    },
    PolynomialLengthMismatch {
        layer_index: usize,
        expected: usize,
        found: usize,
    },
    FinalLayerMismatch {
        expected: u32,
        found: u32,
    },
    MissingChallenge {
        index: usize,
        len: usize,
    },
    UnsupportedDomainBits {
        bits: u32,
    },
    Merkle(PcsFriMerkleError),
    Fold(PcsFriFoldError),
    LengthOverflow,
}

impl fmt::Display for PcsFriOpeningBuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyFriLayers => write!(f, "PCS FRI opening build has no layers"),
            Self::QueryRowCountMismatch { expected, found } => write!(
                f,
                "PCS FRI opening build expected {expected} query rows, found {found}"
            ),
            Self::InvalidLayerBits {
                layer_index,
                input_bits,
                output_bits,
            } => write!(
                f,
                "PCS FRI opening build layer {layer_index} bits are invalid: input {input_bits}, output {output_bits}"
            ),
            Self::LayerInputMismatch {
                layer_index,
                expected,
                found,
            } => write!(
                f,
                "PCS FRI opening build layer {layer_index} input bits {found} do not match expected {expected}"
            ),
            Self::FoldingFactorMismatch {
                layer_index,
                expected,
                found,
            } => write!(
                f,
                "PCS FRI opening build layer {layer_index} folding factor {found} does not match expected {expected}"
            ),
            Self::PolynomialLengthMismatch {
                layer_index,
                expected,
                found,
            } => write!(
                f,
                "PCS FRI opening build layer {layer_index} expected polynomial length {expected}, found {found}"
            ),
            Self::FinalLayerMismatch { expected, found } => write!(
                f,
                "PCS FRI opening build final layer bits {found} do not match expected {expected}"
            ),
            Self::MissingChallenge { index, len } => write!(
                f,
                "PCS FRI opening build challenge index {index} is outside challenge count {len}"
            ),
            Self::UnsupportedDomainBits { bits } => write!(
                f,
                "PCS FRI opening build domain bits are unsupported: {bits}"
            ),
            Self::Merkle(error) => write!(f, "PCS FRI opening build Merkle error: {error}"),
            Self::Fold(error) => write!(f, "PCS FRI opening build fold error: {error}"),
            Self::LengthOverflow => write!(f, "PCS FRI opening build length overflow"),
        }
    }
}

impl std::error::Error for PcsFriOpeningBuildError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Merkle(error) => Some(error),
            Self::Fold(error) => Some(error),
            Self::EmptyFriLayers
            | Self::QueryRowCountMismatch { .. }
            | Self::InvalidLayerBits { .. }
            | Self::LayerInputMismatch { .. }
            | Self::FoldingFactorMismatch { .. }
            | Self::PolynomialLengthMismatch { .. }
            | Self::FinalLayerMismatch { .. }
            | Self::MissingChallenge { .. }
            | Self::UnsupportedDomainBits { .. }
            | Self::LengthOverflow => None,
        }
    }
}

impl From<PcsFriMerkleError> for PcsFriOpeningBuildError {
    fn from(error: PcsFriMerkleError) -> Self {
        Self::Merkle(error)
    }
}

impl From<PcsFriFoldError> for PcsFriOpeningBuildError {
    fn from(error: PcsFriFoldError) -> Self {
        Self::Fold(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PcsFriTranscriptCommitmentError {
    Transcript(PcsTranscriptError),
    Opening(PcsFriOpeningBuildError),
}

impl fmt::Display for PcsFriTranscriptCommitmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transcript(error) => {
                write!(f, "PCS FRI transcript commitment failed: {error}")
            }
            Self::Opening(error) => write!(f, "PCS FRI transcript opening failed: {error}"),
        }
    }
}

impl std::error::Error for PcsFriTranscriptCommitmentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Transcript(error) => Some(error),
            Self::Opening(error) => Some(error),
        }
    }
}

impl From<PcsTranscriptError> for PcsFriTranscriptCommitmentError {
    fn from(error: PcsTranscriptError) -> Self {
        Self::Transcript(error)
    }
}

impl From<PcsFriOpeningBuildError> for PcsFriTranscriptCommitmentError {
    fn from(error: PcsFriOpeningBuildError) -> Self {
        Self::Opening(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PcsFriMerkleError {
    UnsupportedArity { arity: usize },
    EmptyValues,
    EmptyLastLevel,
    InvalidSiblingCount { expected: usize, found: usize },
    LastLevelIndexOutOfRange { index: u64, node_count: usize },
    LengthOverflow,
}

impl fmt::Display for PcsFriMerkleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedArity { arity } => {
                write!(f, "PCS FRI Merkle arity is unsupported: {arity}")
            }
            Self::EmptyValues => write!(f, "PCS FRI Merkle query has no values"),
            Self::EmptyLastLevel => write!(f, "PCS FRI Merkle last level is empty"),
            Self::InvalidSiblingCount { expected, found } => write!(
                f,
                "PCS FRI Merkle sibling count mismatch: expected {expected}, found {found}"
            ),
            Self::LastLevelIndexOutOfRange { index, node_count } => write!(
                f,
                "PCS FRI Merkle last-level index {index} is outside node count {node_count}"
            ),
            Self::LengthOverflow => write!(f, "PCS FRI Merkle length overflow"),
        }
    }
}

impl std::error::Error for PcsFriMerkleError {}

impl From<MerkleHashError> for PcsFriMerkleError {
    fn from(error: MerkleHashError) -> Self {
        match error {
            MerkleHashError::UnsupportedArity { arity } => Self::UnsupportedArity { arity },
            MerkleHashError::InvalidChildCount { expected, found } => {
                Self::InvalidSiblingCount { expected, found }
            }
            MerkleHashError::LengthOverflow => Self::LengthOverflow,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PcsFriOpeningFoldError {
    UnitIndexMismatch {
        expected: u32,
        found: u32,
    },
    QueryRowCountMismatch {
        expected: usize,
        found: usize,
    },
    LayerCountMismatch {
        expected: usize,
        found: usize,
    },
    MissingLayer {
        layer_index: u32,
    },
    LayerQueryCountMismatch {
        layer_index: u32,
        expected: usize,
        found: usize,
    },
    LayerQueryRowMismatch {
        layer_index: u32,
        query_index: usize,
        expected: u64,
        found: u64,
    },
    MissingChallenge {
        index: usize,
        len: usize,
    },
    UnsupportedDomainBits {
        bits: u32,
    },
    LayerValueIndexOutOfRange {
        layer_index: u32,
        query_index: usize,
        value_index: usize,
        len: usize,
    },
    FinalIndexOutOfRange {
        query_index: usize,
        index: usize,
        len: usize,
    },
    NonCanonicalField {
        value: u64,
    },
    Fold(PcsFriFoldError),
    LengthOverflow,
}

impl fmt::Display for PcsFriOpeningFoldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnitIndexMismatch { expected, found } => write!(
                f,
                "PCS FRI opening fold unit index {found} does not match expected {expected}"
            ),
            Self::QueryRowCountMismatch { expected, found } => write!(
                f,
                "PCS FRI opening fold expected {expected} query rows, found {found}"
            ),
            Self::LayerCountMismatch { expected, found } => write!(
                f,
                "PCS FRI opening fold expected {expected} layers, found {found}"
            ),
            Self::MissingLayer { layer_index } => {
                write!(f, "PCS FRI opening fold is missing layer {layer_index}")
            }
            Self::LayerQueryCountMismatch {
                layer_index,
                expected,
                found,
            } => write!(
                f,
                "PCS FRI opening fold layer {layer_index} expected {expected} queries, found {found}"
            ),
            Self::LayerQueryRowMismatch {
                layer_index,
                query_index,
                expected,
                found,
            } => write!(
                f,
                "PCS FRI opening fold layer {layer_index} query {query_index} row {found} does not match expected {expected}"
            ),
            Self::MissingChallenge { index, len } => write!(
                f,
                "PCS FRI opening fold challenge index {index} is outside challenge count {len}"
            ),
            Self::UnsupportedDomainBits { bits } => {
                write!(f, "unsupported PCS FRI opening fold domain bits: {bits}")
            }
            Self::LayerValueIndexOutOfRange {
                layer_index,
                query_index,
                value_index,
                len,
            } => write!(
                f,
                "PCS FRI opening fold layer {layer_index} query {query_index} value index {value_index} is outside value count {len}"
            ),
            Self::FinalIndexOutOfRange {
                query_index,
                index,
                len,
            } => write!(
                f,
                "PCS FRI opening fold query {query_index} final index {index} is outside final polynomial length {len}"
            ),
            Self::NonCanonicalField { value } => write!(
                f,
                "PCS FRI opening fold field value is not canonical: {value}"
            ),
            Self::Fold(error) => write!(f, "PCS FRI opening fold evaluation failed: {error}"),
            Self::LengthOverflow => write!(f, "PCS FRI opening fold length overflow"),
        }
    }
}

impl std::error::Error for PcsFriOpeningFoldError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Fold(error) => Some(error),
            Self::UnitIndexMismatch { .. }
            | Self::QueryRowCountMismatch { .. }
            | Self::LayerCountMismatch { .. }
            | Self::MissingLayer { .. }
            | Self::LayerQueryCountMismatch { .. }
            | Self::LayerQueryRowMismatch { .. }
            | Self::MissingChallenge { .. }
            | Self::UnsupportedDomainBits { .. }
            | Self::LayerValueIndexOutOfRange { .. }
            | Self::FinalIndexOutOfRange { .. }
            | Self::NonCanonicalField { .. }
            | Self::LengthOverflow => None,
        }
    }
}

impl From<PcsFriFoldError> for PcsFriOpeningFoldError {
    fn from(error: PcsFriFoldError) -> Self {
        Self::Fold(error)
    }
}

pub fn verify_fri_fold(
    n_bits_ext: u32,
    current_bits: u32,
    prev_bits: u32,
    challenge: Ext3,
    index: u64,
    values: &[Ext3],
) -> Result<Ext3, PcsFriFoldError> {
    if prev_bits <= current_bits {
        return Err(PcsFriFoldError::InvalidLayerBits {
            current_bits,
            prev_bits,
        });
    }
    if n_bits_ext < prev_bits {
        return Err(PcsFriFoldError::InvalidExtensionBits {
            n_bits_ext,
            prev_bits,
        });
    }

    let fold_bits = prev_bits - current_bits;
    let expected_len = 1_usize
        .checked_shl(fold_bits)
        .ok_or(PcsFriFoldError::LengthOverflow)?;
    if values.len() != expected_len {
        return Err(PcsFriFoldError::ValueLengthMismatch {
            expected: expected_len,
            found: values.len(),
        });
    }

    let coefficients = interpolate_fold_values(values, fold_bits as usize)?;
    let shift = fold_shift(n_bits_ext, prev_bits);
    let root = Felt::root_of_unity(prev_bits as usize)
        .ok_or(PcsFriFoldError::UnsupportedRoot { bits: prev_bits })?;
    let point = shift * root.pow(index);
    let inverse = point
        .inverse()
        .ok_or(PcsFriFoldError::ZeroEvaluationPoint)?;
    Ok(evaluate_extension_polynomial(
        &coefficients,
        scale_extension(challenge, inverse),
    ))
}

pub fn verify_fri_query_path(
    root: [Felt; HASH_WORDS],
    last_level: &[[Felt; HASH_WORDS]],
    arity: usize,
    row_index: u64,
    values: &[Ext3],
    siblings: &[Vec<[Felt; HASH_WORDS]>],
) -> Result<bool, PcsFriMerkleError> {
    if values.is_empty() {
        return Err(PcsFriMerkleError::EmptyValues);
    }
    let flattened_values = flatten_extension_values(values)?;
    let mut digest = linear_hash(&flattened_values, arity)?;
    let mut path_index = row_index;
    let arity_u64 = u64::try_from(arity).map_err(|_| PcsFriMerkleError::LengthOverflow)?;
    let expected_siblings = arity
        .checked_sub(1)
        .ok_or(PcsFriMerkleError::LengthOverflow)?;

    for level in siblings {
        if level.len() != expected_siblings {
            return Err(PcsFriMerkleError::InvalidSiblingCount {
                expected: expected_siblings,
                found: level.len(),
            });
        }

        let child_slot = usize::try_from(path_index % arity_u64)
            .map_err(|_| PcsFriMerkleError::LengthOverflow)?;
        let mut children = vec![[Felt::ZERO; HASH_WORDS]; arity];
        let mut sibling_index = 0;
        for (slot, child) in children.iter_mut().enumerate() {
            if slot == child_slot {
                *child = digest;
            } else {
                *child = level[sibling_index];
                sibling_index += 1;
            }
        }
        digest = parent_hash(&children, arity)?;
        path_index /= arity_u64;
    }

    if last_level.is_empty() {
        Ok(digest == root)
    } else {
        let index = usize::try_from(path_index).map_err(|_| PcsFriMerkleError::LengthOverflow)?;
        let target = last_level
            .get(index)
            .ok_or(PcsFriMerkleError::LastLevelIndexOutOfRange {
                index: path_index,
                node_count: last_level.len(),
            })?;
        Ok(digest == *target)
    }
}

pub fn verify_fri_last_level_root(
    root: [Felt; HASH_WORDS],
    arity: usize,
    last_level: &[[Felt; HASH_WORDS]],
) -> Result<bool, PcsFriMerkleError> {
    if last_level.is_empty() {
        return Err(PcsFriMerkleError::EmptyLastLevel);
    }
    Ok(root_from_digest_level(last_level, arity)? == root)
}

pub fn build_pcs_fri_transcript_commitments(
    schedule: &ProveUnitSchedule,
    request: PcsFriTranscriptCommitmentRequest<'_>,
) -> Result<PcsFriTranscriptCommitments, PcsFriTranscriptCommitmentError> {
    if schedule.fri_layers.is_empty() {
        return Err(PcsFriOpeningBuildError::EmptyFriLayers.into());
    }

    let (mut transcript, mut challenges) = build_fri_transcript_prefix(request)?;
    challenges.push(Ext3::ZERO);

    let arity = usize::try_from(schedule.merkle_tree_arity)
        .map_err(|_| PcsFriOpeningBuildError::LengthOverflow)?;
    let mut current = request.polynomial.to_vec();
    let mut current_bits = schedule.fri_layers[0].input_bits;
    let expected_initial_len = build_domain_size(current_bits)?;
    if current.len() != expected_initial_len {
        return Err(PcsFriOpeningBuildError::PolynomialLengthMismatch {
            layer_index: 0,
            expected: expected_initial_len,
            found: current.len(),
        }
        .into());
    }

    let mut layer_roots = Vec::with_capacity(schedule.fri_layers.len());
    for (layer_index, layer) in schedule.fri_layers.iter().enumerate() {
        if layer.input_bits != current_bits {
            return Err(PcsFriOpeningBuildError::LayerInputMismatch {
                layer_index,
                expected: current_bits,
                found: layer.input_bits,
            }
            .into());
        }
        if layer.output_bits >= layer.input_bits {
            return Err(PcsFriOpeningBuildError::InvalidLayerBits {
                layer_index,
                input_bits: layer.input_bits,
                output_bits: layer.output_bits,
            }
            .into());
        }

        let output_size = build_domain_size(layer.output_bits)?;
        let folding_factor = usize::try_from(layer.folding_factor)
            .map_err(|_| PcsFriOpeningBuildError::LengthOverflow)?;
        let expected_folding_factor = build_domain_size(layer.input_bits - layer.output_bits)?;
        if folding_factor != expected_folding_factor {
            return Err(PcsFriOpeningBuildError::FoldingFactorMismatch {
                layer_index,
                expected: expected_folding_factor,
                found: folding_factor,
            }
            .into());
        }

        let grouped_values =
            group_fri_layer_values(layer_index, &current, output_size, folding_factor)?;
        let tree = build_fri_layer_tree(&grouped_values, arity, schedule.last_level_verification)?;
        layer_roots.push(tree.root);
        transcript.put(&tree.root);
        let challenge = transcript.get_field();
        challenges.push(challenge);

        let mut next = Vec::with_capacity(output_size);
        for (row_index, values) in grouped_values.iter().enumerate() {
            next.push(
                verify_fri_fold(
                    schedule.extended_domain_bits,
                    layer.output_bits,
                    layer.input_bits,
                    challenge,
                    u64::try_from(row_index)
                        .map_err(|_| PcsFriOpeningBuildError::LengthOverflow)?,
                    values,
                )
                .map_err(PcsFriOpeningBuildError::from)?,
            );
        }
        current = next;
        current_bits = layer.output_bits;
    }

    if current_bits != schedule.final_layer_bits {
        return Err(PcsFriOpeningBuildError::FinalLayerMismatch {
            expected: schedule.final_layer_bits,
            found: current_bits,
        }
        .into());
    }

    let final_values = flatten_extension_values_for_transcript(&current);
    absorb_commit_values(
        &mut transcript,
        request.arity,
        request.hash_values,
        &final_values,
    )?;
    let final_query_challenge = transcript.get_field();
    challenges.push(final_query_challenge);

    Ok(PcsFriTranscriptCommitments {
        challenges,
        layer_roots,
        final_polynomial: current,
        final_query_challenge,
    })
}

pub fn build_pcs_fri_opening_unit(
    schedule: &ProveUnitSchedule,
    request: PcsFriOpeningBuildRequest<'_>,
) -> Result<PcsFriOpeningUnitSegment, PcsFriOpeningBuildError> {
    if schedule.fri_layers.is_empty() {
        return Err(PcsFriOpeningBuildError::EmptyFriLayers);
    }

    let query_count = usize::try_from(schedule.query_count)
        .map_err(|_| PcsFriOpeningBuildError::LengthOverflow)?;
    if request.query_rows.len() != query_count {
        return Err(PcsFriOpeningBuildError::QueryRowCountMismatch {
            expected: query_count,
            found: request.query_rows.len(),
        });
    }

    let arity = usize::try_from(schedule.merkle_tree_arity)
        .map_err(|_| PcsFriOpeningBuildError::LengthOverflow)?;
    let mut current = request.polynomial.to_vec();
    let mut current_bits = schedule.fri_layers[0].input_bits;
    let expected_initial_len = build_domain_size(current_bits)?;
    if current.len() != expected_initial_len {
        return Err(PcsFriOpeningBuildError::PolynomialLengthMismatch {
            layer_index: 0,
            expected: expected_initial_len,
            found: current.len(),
        });
    }

    let mut layers = Vec::with_capacity(schedule.fri_layers.len());
    for (layer_index, layer) in schedule.fri_layers.iter().enumerate() {
        if layer.input_bits != current_bits {
            return Err(PcsFriOpeningBuildError::LayerInputMismatch {
                layer_index,
                expected: current_bits,
                found: layer.input_bits,
            });
        }
        if layer.output_bits >= layer.input_bits {
            return Err(PcsFriOpeningBuildError::InvalidLayerBits {
                layer_index,
                input_bits: layer.input_bits,
                output_bits: layer.output_bits,
            });
        }

        let output_size = build_domain_size(layer.output_bits)?;
        let folding_factor = usize::try_from(layer.folding_factor)
            .map_err(|_| PcsFriOpeningBuildError::LengthOverflow)?;
        let expected_folding_factor = build_domain_size(layer.input_bits - layer.output_bits)?;
        if folding_factor != expected_folding_factor {
            return Err(PcsFriOpeningBuildError::FoldingFactorMismatch {
                layer_index,
                expected: expected_folding_factor,
                found: folding_factor,
            });
        }

        let grouped_values =
            group_fri_layer_values(layer_index, &current, output_size, folding_factor)?;
        let tree = build_fri_layer_tree(&grouped_values, arity, schedule.last_level_verification)?;
        let layer_index_u32 =
            u32::try_from(layer_index).map_err(|_| PcsFriOpeningBuildError::LengthOverflow)?;
        let output_size_u64 =
            u64::try_from(output_size).map_err(|_| PcsFriOpeningBuildError::LengthOverflow)?;
        let queries = request
            .query_rows
            .iter()
            .map(|query_row| {
                let row_index = *query_row % output_size_u64;
                let row_index_usize = usize::try_from(row_index)
                    .map_err(|_| PcsFriOpeningBuildError::LengthOverflow)?;
                let values = grouped_values[row_index_usize]
                    .iter()
                    .map(|value| value.to_u64s())
                    .collect();
                let siblings = tree.query_siblings(row_index_usize)?;
                Ok(PcsFriOpeningQuerySegment {
                    row_index,
                    values,
                    siblings,
                })
            })
            .collect::<Result<Vec<_>, PcsFriOpeningBuildError>>()?;

        layers.push(PcsFriOpeningLayerSegment {
            layer_index: layer_index_u32,
            root: digest_to_u64s(tree.root),
            last_level: tree
                .last_level
                .iter()
                .copied()
                .map(digest_to_u64s)
                .collect(),
            queries,
        });

        let challenge_index = schedule
            .challenge_count
            .checked_add(layer_index)
            .and_then(|index| index.checked_add(1))
            .ok_or(PcsFriOpeningBuildError::LengthOverflow)?;
        let challenge = *request.challenges.get(challenge_index).ok_or(
            PcsFriOpeningBuildError::MissingChallenge {
                index: challenge_index,
                len: request.challenges.len(),
            },
        )?;
        let mut next = Vec::with_capacity(output_size);
        for (row_index, values) in grouped_values.iter().enumerate() {
            next.push(verify_fri_fold(
                schedule.extended_domain_bits,
                layer.output_bits,
                layer.input_bits,
                challenge,
                u64::try_from(row_index).map_err(|_| PcsFriOpeningBuildError::LengthOverflow)?,
                values,
            )?);
        }
        current = next;
        current_bits = layer.output_bits;
    }

    if current_bits != schedule.final_layer_bits {
        return Err(PcsFriOpeningBuildError::FinalLayerMismatch {
            expected: schedule.final_layer_bits,
            found: current_bits,
        });
    }

    Ok(PcsFriOpeningUnitSegment {
        unit_index: request.unit_index,
        layers,
        final_polynomial: current.iter().map(|value| value.to_u64s()).collect(),
    })
}

pub fn verify_fri_opening_folds(
    schedule: &ProveUnitSchedule,
    request: PcsFriOpeningFoldRequest<'_>,
) -> Result<bool, PcsFriOpeningFoldError> {
    if request.unit_index != request.fri.unit_index {
        return Err(PcsFriOpeningFoldError::UnitIndexMismatch {
            expected: request.unit_index,
            found: request.fri.unit_index,
        });
    }

    let query_count = usize::try_from(schedule.query_count)
        .map_err(|_| PcsFriOpeningFoldError::LengthOverflow)?;
    if request.query_rows.len() != query_count {
        return Err(PcsFriOpeningFoldError::QueryRowCountMismatch {
            expected: query_count,
            found: request.query_rows.len(),
        });
    }
    if request.fri.layers.len() != schedule.fri_layers.len() {
        return Err(PcsFriOpeningFoldError::LayerCountMismatch {
            expected: schedule.fri_layers.len(),
            found: request.fri.layers.len(),
        });
    }

    let layers = ordered_opening_layers(request.fri, schedule.fri_layers.len())?;
    for layer in &layers {
        if layer.queries.len() != query_count {
            return Err(PcsFriOpeningFoldError::LayerQueryCountMismatch {
                layer_index: layer.layer_index,
                expected: query_count,
                found: layer.queries.len(),
            });
        }
    }

    for (query_index, query_row) in request.query_rows.iter().enumerate() {
        for (layer_index, (layer_plan, opening_layer)) in
            schedule.fri_layers.iter().zip(layers.iter()).enumerate()
        {
            let output_size = domain_size(layer_plan.output_bits)?;
            let expected_row = query_row % output_size;
            let query = &opening_layer.queries[query_index];
            if query.row_index != expected_row {
                return Err(PcsFriOpeningFoldError::LayerQueryRowMismatch {
                    layer_index: opening_layer.layer_index,
                    query_index,
                    expected: expected_row,
                    found: query.row_index,
                });
            }

            let values = query
                .values
                .iter()
                .map(|value| convert_ext(*value))
                .collect::<Result<Vec<_>, PcsFriOpeningFoldError>>()?;
            let challenge_index = schedule
                .challenge_count
                .checked_add(layer_index)
                .and_then(|index| index.checked_add(1))
                .ok_or(PcsFriOpeningFoldError::LengthOverflow)?;
            let challenge = *request.challenges.get(challenge_index).ok_or(
                PcsFriOpeningFoldError::MissingChallenge {
                    index: challenge_index,
                    len: request.challenges.len(),
                },
            )?;
            let folded = verify_fri_fold(
                schedule.extended_domain_bits,
                layer_plan.output_bits,
                layer_plan.input_bits,
                challenge,
                expected_row,
                &values,
            )?;

            let target = if let Some(next_plan) = schedule.fri_layers.get(layer_index + 1) {
                let next_output_size = domain_size(next_plan.output_bits)?;
                let value_index = usize::try_from(expected_row / next_output_size)
                    .map_err(|_| PcsFriOpeningFoldError::LengthOverflow)?;
                let next_layer = layers[layer_index + 1];
                let next_query = &next_layer.queries[query_index];
                let value = next_query.values.get(value_index).ok_or(
                    PcsFriOpeningFoldError::LayerValueIndexOutOfRange {
                        layer_index: next_layer.layer_index,
                        query_index,
                        value_index,
                        len: next_query.values.len(),
                    },
                )?;
                convert_ext(*value)?
            } else {
                let final_index = usize::try_from(expected_row)
                    .map_err(|_| PcsFriOpeningFoldError::LengthOverflow)?;
                let value = request.fri.final_polynomial.get(final_index).ok_or(
                    PcsFriOpeningFoldError::FinalIndexOutOfRange {
                        query_index,
                        index: final_index,
                        len: request.fri.final_polynomial.len(),
                    },
                )?;
                convert_ext(*value)?
            };

            if folded != target {
                return Ok(false);
            }
        }
    }

    Ok(true)
}

fn interpolate_fold_values(values: &[Ext3], bits: usize) -> Result<Vec<Ext3>, PcsFriFoldError> {
    let mut c0 = Vec::with_capacity(values.len());
    let mut c1 = Vec::with_capacity(values.len());
    let mut c2 = Vec::with_capacity(values.len());
    for value in values {
        c0.push(value.c0);
        c1.push(value.c1);
        c2.push(value.c2);
    }
    intt_in_place(&mut c0, bits)?;
    intt_in_place(&mut c1, bits)?;
    intt_in_place(&mut c2, bits)?;
    Ok(c0
        .into_iter()
        .zip(c1)
        .zip(c2)
        .map(|((c0, c1), c2)| Ext3::new(c0, c1, c2))
        .collect())
}

fn fold_shift(n_bits_ext: u32, prev_bits: u32) -> Felt {
    let mut shift = SHIFT;
    for _ in 0..(n_bits_ext - prev_bits) {
        shift = shift * shift;
    }
    shift
}

fn evaluate_extension_polynomial(coefficients: &[Ext3], point: Ext3) -> Ext3 {
    coefficients
        .iter()
        .rev()
        .fold(Ext3::ZERO, |acc, coefficient| acc * point + *coefficient)
}

fn scale_extension(value: Ext3, scalar: Felt) -> Ext3 {
    Ext3::new(value.c0 * scalar, value.c1 * scalar, value.c2 * scalar)
}

fn build_fri_transcript_prefix(
    request: PcsFriTranscriptCommitmentRequest<'_>,
) -> Result<(PoseidonTranscript, Vec<Ext3>), PcsTranscriptError> {
    if request.witness_roots.len() != request.root_challenge_draws.len() {
        return Err(PcsTranscriptError::RootChallengeDrawMismatch {
            root_count: request.witness_roots.len(),
            draw_count: request.root_challenge_draws.len(),
        });
    }

    let mut transcript = PoseidonTranscript::new(request.arity)?;
    let mut challenges = Vec::new();
    transcript.put(&request.constant_root);

    if !request.public_values.is_empty() {
        absorb_commit_values(
            &mut transcript,
            request.arity,
            request.hash_values,
            request.public_values,
        )?;
    }

    for (stage_index, (root, draw_count)) in request
        .witness_roots
        .iter()
        .zip(request.root_challenge_draws.iter())
        .enumerate()
    {
        let stage =
            u32::try_from(stage_index + 1).map_err(|_| PcsTranscriptError::LengthOverflow)?;
        transcript.put(root);
        absorb_transcript_stage_unit_values(
            &mut transcript,
            stage,
            request.unit_value_map,
            request.unit_values,
        )?;
        draw_transcript_fields(&mut transcript, *draw_count, &mut challenges);
    }

    if !request.evaluation_values.is_empty() {
        let values = flatten_extension_values_for_transcript(request.evaluation_values);
        absorb_commit_values(&mut transcript, request.arity, request.hash_values, &values)?;
    }
    draw_transcript_fields(
        &mut transcript,
        request.evaluation_challenge_draws,
        &mut challenges,
    );

    Ok((transcript, challenges))
}

fn absorb_transcript_stage_unit_values(
    transcript: &mut PoseidonTranscript,
    stage: u32,
    value_map: &[StageValue],
    values: &[Felt],
) -> Result<(), PcsTranscriptError> {
    let mut offset = 0_usize;
    for (value_index, value) in value_map.iter().enumerate() {
        let width = if value.stage == 1 { 1 } else { 3 };
        let end = offset
            .checked_add(width)
            .ok_or(PcsTranscriptError::LengthOverflow)?;
        if end > values.len() {
            return Err(PcsTranscriptError::UnitValueOutOfRange {
                value_index,
                offset,
                width,
                len: values.len(),
            });
        }
        if value.stage == stage && value.stage > 1 {
            transcript.put(&values[offset..end]);
        }
        offset = end;
    }
    Ok(())
}

fn draw_transcript_fields(transcript: &mut PoseidonTranscript, count: usize, out: &mut Vec<Ext3>) {
    for _ in 0..count {
        out.push(transcript.get_field());
    }
}

fn flatten_extension_values_for_transcript(values: &[Ext3]) -> Vec<Felt> {
    values
        .iter()
        .flat_map(|value| [value.c0, value.c1, value.c2])
        .collect()
}

fn flatten_extension_values(values: &[Ext3]) -> Result<Vec<Felt>, PcsFriMerkleError> {
    let len = values
        .len()
        .checked_mul(3)
        .ok_or(PcsFriMerkleError::LengthOverflow)?;
    let mut out = Vec::with_capacity(len);
    for value in values {
        out.push(value.c0);
        out.push(value.c1);
        out.push(value.c2);
    }
    Ok(out)
}

fn ordered_opening_layers(
    fri: &PcsFriOpeningUnitSegment,
    expected_count: usize,
) -> Result<Vec<&PcsFriOpeningLayerSegment>, PcsFriOpeningFoldError> {
    let mut layers = Vec::with_capacity(expected_count);
    for layer_index in 0..expected_count {
        let layer_index_u32 =
            u32::try_from(layer_index).map_err(|_| PcsFriOpeningFoldError::LengthOverflow)?;
        let layer = fri
            .layers
            .iter()
            .find(|layer| layer.layer_index == layer_index_u32)
            .ok_or(PcsFriOpeningFoldError::MissingLayer {
                layer_index: layer_index_u32,
            })?;
        layers.push(layer);
    }
    Ok(layers)
}

fn domain_size(bits: u32) -> Result<u64, PcsFriOpeningFoldError> {
    1_u64
        .checked_shl(bits)
        .ok_or(PcsFriOpeningFoldError::UnsupportedDomainBits { bits })
}

fn convert_ext(values: [u64; 3]) -> Result<Ext3, PcsFriOpeningFoldError> {
    Ok(Ext3::new(
        convert_felt(values[0])?,
        convert_felt(values[1])?,
        convert_felt(values[2])?,
    ))
}

fn convert_felt(value: u64) -> Result<Felt, PcsFriOpeningFoldError> {
    Felt::from_canonical(value).map_err(|error| match error {
        FieldError::NonCanonical { value } => PcsFriOpeningFoldError::NonCanonicalField { value },
    })
}

struct FriLayerTree {
    root: [Felt; HASH_WORDS],
    levels: Vec<Vec<[Felt; HASH_WORDS]>>,
    unpadded_counts: Vec<usize>,
    last_level: Vec<[Felt; HASH_WORDS]>,
    last_level_verification: u32,
    arity: usize,
}

impl FriLayerTree {
    fn query_siblings(
        &self,
        row_index: usize,
    ) -> Result<Vec<PcsFriOpeningLevelSegment>, PcsFriOpeningBuildError> {
        let mut siblings = Vec::new();
        let mut query_index = row_index;
        let mut level_index = 0;
        while !self.should_stop_at_level(level_index)? {
            let level = self
                .levels
                .get(level_index)
                .ok_or(PcsFriOpeningBuildError::LengthOverflow)?;
            let child_slot = query_index % self.arity;
            let group_start = (query_index / self.arity)
                .checked_mul(self.arity)
                .ok_or(PcsFriOpeningBuildError::LengthOverflow)?;
            let mut level_siblings = Vec::with_capacity(self.arity - 1);
            for slot in 0..self.arity {
                if slot == child_slot {
                    continue;
                }
                let sibling_index = group_start
                    .checked_add(slot)
                    .ok_or(PcsFriOpeningBuildError::LengthOverflow)?;
                let digest = level
                    .get(sibling_index)
                    .copied()
                    .ok_or(PcsFriOpeningBuildError::LengthOverflow)?;
                level_siblings.push(digest_to_u64s(digest));
            }
            siblings.push(PcsFriOpeningLevelSegment {
                siblings: level_siblings,
            });
            query_index /= self.arity;
            level_index += 1;
        }
        Ok(siblings)
    }

    fn should_stop_at_level(&self, level_index: usize) -> Result<bool, PcsFriOpeningBuildError> {
        let count = *self
            .unpadded_counts
            .get(level_index)
            .ok_or(PcsFriOpeningBuildError::LengthOverflow)?;
        if self.last_level_verification == 0 {
            Ok(count == 1)
        } else {
            Ok(count <= checked_pow(self.arity, self.last_level_verification)?)
        }
    }
}

fn group_fri_layer_values(
    layer_index: usize,
    polynomial: &[Ext3],
    output_size: usize,
    folding_factor: usize,
) -> Result<Vec<Vec<Ext3>>, PcsFriOpeningBuildError> {
    let expected = output_size
        .checked_mul(folding_factor)
        .ok_or(PcsFriOpeningBuildError::LengthOverflow)?;
    if polynomial.len() != expected {
        return Err(PcsFriOpeningBuildError::PolynomialLengthMismatch {
            layer_index,
            expected,
            found: polynomial.len(),
        });
    }

    let mut grouped = Vec::with_capacity(output_size);
    for row in 0..output_size {
        let mut values = Vec::with_capacity(folding_factor);
        for slot in 0..folding_factor {
            let index = slot
                .checked_mul(output_size)
                .and_then(|offset| offset.checked_add(row))
                .ok_or(PcsFriOpeningBuildError::LengthOverflow)?;
            values.push(polynomial[index]);
        }
        grouped.push(values);
    }
    Ok(grouped)
}

fn build_fri_layer_tree(
    rows: &[Vec<Ext3>],
    arity: usize,
    last_level_verification: u32,
) -> Result<FriLayerTree, PcsFriOpeningBuildError> {
    if rows.is_empty() {
        return Err(PcsFriOpeningBuildError::PolynomialLengthMismatch {
            layer_index: 0,
            expected: 1,
            found: 0,
        });
    }

    let mut current = rows
        .iter()
        .map(|row| {
            let flattened = flatten_extension_values(row)?;
            linear_hash(&flattened, arity).map_err(PcsFriMerkleError::from)
        })
        .collect::<Result<Vec<_>, PcsFriMerkleError>>()?;
    let mut levels = Vec::new();
    let mut unpadded_counts = Vec::new();
    loop {
        unpadded_counts.push(current.len());
        let mut padded = current.clone();
        if padded.len() > 1 {
            let extra_zeros = (arity - (padded.len() % arity)) % arity;
            padded.resize(
                padded
                    .len()
                    .checked_add(extra_zeros)
                    .ok_or(PcsFriOpeningBuildError::LengthOverflow)?,
                [Felt::ZERO; HASH_WORDS],
            );
        }
        levels.push(padded.clone());
        if current.len() == 1 {
            break;
        }

        current = padded
            .chunks_exact(arity)
            .map(|children| parent_hash(children, arity).map_err(PcsFriMerkleError::from))
            .collect::<Result<Vec<_>, PcsFriMerkleError>>()?;
    }

    let root = *levels
        .last()
        .and_then(|level| level.first())
        .ok_or(PcsFriOpeningBuildError::LengthOverflow)?;
    let last_level = if last_level_verification == 0 {
        Vec::new()
    } else {
        let target_count = checked_pow(arity, last_level_verification)?;
        let level_index = unpadded_counts
            .iter()
            .position(|count| *count <= target_count)
            .ok_or(PcsFriOpeningBuildError::LengthOverflow)?;
        let count = unpadded_counts[level_index];
        levels[level_index][..count].to_vec()
    };

    Ok(FriLayerTree {
        root,
        levels,
        unpadded_counts,
        last_level,
        last_level_verification,
        arity,
    })
}

fn build_domain_size(bits: u32) -> Result<usize, PcsFriOpeningBuildError> {
    1_usize
        .checked_shl(bits)
        .ok_or(PcsFriOpeningBuildError::UnsupportedDomainBits { bits })
}

fn checked_pow(base: usize, power: u32) -> Result<usize, PcsFriOpeningBuildError> {
    let mut out = 1_usize;
    for _ in 0..power {
        out = out
            .checked_mul(base)
            .ok_or(PcsFriOpeningBuildError::LengthOverflow)?;
    }
    Ok(out)
}

fn digest_to_u64s(digest: [Felt; HASH_WORDS]) -> [u64; HASH_WORDS] {
    digest.map(Felt::to_u64)
}
