use std::collections::BTreeSet;
use std::fmt;
use std::path::{Path, PathBuf};

use lzvm_artifacts::challenge_values_segment::{
    parse_challenge_values_segment, ChallengeValuesSegmentError, CHALLENGE_VALUES_SEGMENT_ID,
};
use lzvm_artifacts::contribution_segment::{
    encode_contribution_segment, parse_contribution_segment, ContributionEntry,
    ContributionSegment, ContributionSegmentError, CONTRIBUTION_SEGMENT_ID,
};
use lzvm_artifacts::eth_block_input_segment::ETH_BLOCK_INPUT_SEGMENT_ID;
use lzvm_artifacts::global_info::{CurveKind, GlobalInfo, NamedStageValue};
use lzvm_artifacts::key_directory::{read_key_directory_catalog, KeyDirectoryError};
use lzvm_artifacts::program_image_segment::PROGRAM_IMAGE_CACHE_SEGMENT_ID;
use lzvm_artifacts::proof::{read_proof_artifact_file, ProofArtifactError, ProofSegment};
use lzvm_artifacts::public_values::{read_public_values_file, PublicValuesError};
use lzvm_artifacts::setup_info::StageValue;
use lzvm_artifacts::setup_manifest::SetupDirectoryManifestError;
use lzvm_artifacts::verification_key::VerificationKeyRoot;
use lzvm_field::{poseidon2_hash_16, Ext3, Felt, FieldError, PoseidonTranscript, TranscriptError};

use crate::proof_preflight::{public_values_as_fields, PublicValueFieldError};
use crate::proof_values::{
    flatten_pcs_proof_values, load_pcs_proof_values_from_segments, LoadPcsProofValuesSegmentError,
    ProvePcsProofValuesSegmentError,
};
use crate::setup_preflight::{
    is_setup_proof_segment_id, validate_setup_directory_manifest_if_present,
    validate_setup_preflight_hashes, SetupPreflightError,
};
use crate::{ProveUnitSchedule, ProveWitnessTraceCommitments};
use sha2::{Digest, Sha256};

const CONTRIBUTION_ROOT_SLOT_START: usize = 4;
const CONTRIBUTION_ROOT_SLOT_END: usize = 8;
const CONTRIBUTION_HASH_STATE_WIDTH: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProveContributionEntry {
    pub worker_index: u32,
    pub group_id: u32,
    pub aggregated: bool,
    pub values: Vec<Felt>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InternalContributionInput {
    pub root: [Felt; 4],
    pub values: Vec<Felt>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContributionChallengeReport {
    pub proof_count: usize,
    pub segment_count: usize,
    pub public_value_count: usize,
    pub public_values_hash: [u8; 32],
    pub public_value_field_count: usize,
    pub program_image_cache_count: usize,
    pub program_image_cache_hashes: Vec<[u8; 32]>,
    pub eth_block_input_count: usize,
    pub eth_block_input_hashes: Vec<[u8; 32]>,
    pub eth_block_input_byte_counts: Vec<usize>,
    pub eth_block_input_block_rlp_byte_counts: Vec<usize>,
    pub eth_block_input_extra_header_field_counts: Vec<usize>,
    pub eth_block_input_extra_body_field_counts: Vec<usize>,
    pub proof_value_count: usize,
    pub contribution_count: usize,
    pub challenge: Ext3,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProveContributionSegmentError {
    Segment(ContributionSegmentError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadContributionSegmentError {
    MissingSegment,
    DuplicateSegment,
    NonCanonicalValue {
        entry_index: usize,
        index: usize,
        source: FieldError,
    },
    Segment(ContributionSegmentError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContributionChallengeError {
    UnsupportedCurve {
        curve: CurveKind,
    },
    MissingLatticeSize,
    LatticeSizeOverflow {
        value: u64,
    },
    LatticeSizeNotMultipleOfHashState {
        value: u64,
    },
    EmptyEntries,
    EmptyValues {
        entry_index: usize,
    },
    ContributionInputTooShort {
        input_index: usize,
        found: usize,
    },
    DuplicateEntry {
        worker_index: u32,
        group_id: u32,
    },
    ValueCountMismatch {
        entry_index: usize,
        expected: usize,
        found: usize,
    },
    ProofValueCountMismatch {
        expected: usize,
        found: usize,
    },
    UnitValueCountMismatch {
        expected: usize,
        found: usize,
    },
    MissingStageOneContributionRoot {
        unit_index: usize,
    },
    VerificationKeyValueCountMismatch {
        expected: usize,
        found: usize,
    },
    VerificationKeyNonCanonicalValue {
        index: usize,
        source: FieldError,
    },
    Load(LoadContributionSegmentError),
    LengthOverflow,
    Transcript(TranscriptError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContributionChallengeFileError {
    MissingProofs,
    Catalog(KeyDirectoryError),
    SetupDirectoryManifest(SetupDirectoryManifestError),
    Proof(ProofArtifactError),
    PublicValues(PublicValuesError),
    SetupPreflight(SetupPreflightError),
    UnexpectedProofSegment { id: u32 },
    PublicValueField(PublicValueFieldError),
    ProofValues(LoadPcsProofValuesSegmentError),
    ProofValuePacking(ProvePcsProofValuesSegmentError),
    ProofValueMismatch { proof_index: usize },
    BindingSegmentMismatch { proof_index: usize, id: u32 },
    ChallengeValues(ChallengeValuesSegmentError),
    DuplicateChallengeValuesSegment,
    ContributionChallengeValuesMismatch,
    Contribution(ContributionChallengeError),
}

impl fmt::Display for ProveContributionSegmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Segment(error) => write!(f, "contribution segment encode failed: {error}"),
        }
    }
}

impl std::error::Error for ProveContributionSegmentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Segment(error) => Some(error),
        }
    }
}

impl From<ContributionSegmentError> for ProveContributionSegmentError {
    fn from(error: ContributionSegmentError) -> Self {
        Self::Segment(error)
    }
}

impl fmt::Display for LoadContributionSegmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSegment => write!(f, "missing contribution segment"),
            Self::DuplicateSegment => write!(f, "duplicate contribution segment"),
            Self::NonCanonicalValue {
                entry_index,
                index,
                source,
            } => write!(
                f,
                "invalid contribution segment entry {entry_index} value {index}: {source}"
            ),
            Self::Segment(error) => write!(f, "invalid contribution segment: {error}"),
        }
    }
}

impl std::error::Error for LoadContributionSegmentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::NonCanonicalValue { source, .. } => Some(source),
            Self::Segment(error) => Some(error),
            Self::MissingSegment | Self::DuplicateSegment => None,
        }
    }
}

impl fmt::Display for ContributionChallengeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedCurve { curve } => {
                write!(f, "unsupported contribution curve mode: {curve:?}")
            }
            Self::MissingLatticeSize => write!(f, "missing contribution lattice size"),
            Self::LatticeSizeOverflow { value } => {
                write!(f, "contribution lattice size does not fit usize: {value}")
            }
            Self::LatticeSizeNotMultipleOfHashState { value } => write!(
                f,
                "contribution lattice size must be a positive multiple of 16: {value}"
            ),
            Self::EmptyEntries => write!(f, "contribution list has no entries"),
            Self::EmptyValues { entry_index } => {
                write!(f, "contribution entry {entry_index} has no values")
            }
            Self::ContributionInputTooShort { input_index, found } => write!(
                f,
                "contribution input {input_index} has {found} values, expected at least 8"
            ),
            Self::DuplicateEntry {
                worker_index,
                group_id,
            } => write!(
                f,
                "duplicate contribution entry for worker {worker_index} group {group_id}"
            ),
            Self::ValueCountMismatch {
                entry_index,
                expected,
                found,
            } => write!(
                f,
                "contribution entry {entry_index} value count mismatch: expected {expected}, found {found}"
            ),
            Self::ProofValueCountMismatch { expected, found } => write!(
                f,
                "contribution proof value count mismatch: expected {expected}, found {found}"
            ),
            Self::UnitValueCountMismatch { expected, found } => write!(
                f,
                "contribution unit value count mismatch: expected {expected}, found {found}"
            ),
            Self::MissingStageOneContributionRoot { unit_index } => write!(
                f,
                "contribution witness output unit {unit_index} is missing stage-one root"
            ),
            Self::VerificationKeyValueCountMismatch { expected, found } => write!(
                f,
                "contribution verification-key value count mismatch: expected {expected}, found {found}"
            ),
            Self::VerificationKeyNonCanonicalValue { index, source } => write!(
                f,
                "contribution verification-key value {index} is non-canonical: {source}"
            ),
            Self::Load(error) => write!(f, "{error}"),
            Self::LengthOverflow => write!(f, "contribution challenge length overflow"),
            Self::Transcript(error) => write!(f, "contribution challenge transcript failed: {error}"),
        }
    }
}

impl std::error::Error for ContributionChallengeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Transcript(error) => Some(error),
            Self::Load(error) => Some(error),
            Self::VerificationKeyNonCanonicalValue { source, .. } => Some(source),
            Self::UnsupportedCurve { .. }
            | Self::MissingLatticeSize
            | Self::LatticeSizeOverflow { .. }
            | Self::LatticeSizeNotMultipleOfHashState { .. }
            | Self::EmptyEntries
            | Self::EmptyValues { .. }
            | Self::ContributionInputTooShort { .. }
            | Self::DuplicateEntry { .. }
            | Self::ValueCountMismatch { .. }
            | Self::ProofValueCountMismatch { .. }
            | Self::UnitValueCountMismatch { .. }
            | Self::MissingStageOneContributionRoot { .. }
            | Self::VerificationKeyValueCountMismatch { .. }
            | Self::LengthOverflow => None,
        }
    }
}

impl From<TranscriptError> for ContributionChallengeError {
    fn from(error: TranscriptError) -> Self {
        Self::Transcript(error)
    }
}

impl From<LoadContributionSegmentError> for ContributionChallengeError {
    fn from(error: LoadContributionSegmentError) -> Self {
        Self::Load(error)
    }
}

impl fmt::Display for ContributionChallengeFileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingProofs => write!(f, "contribution proof set has no proofs"),
            Self::Catalog(error) => write!(f, "{error}"),
            Self::SetupDirectoryManifest(error) => write!(f, "{error}"),
            Self::Proof(error) => write!(f, "{error}"),
            Self::PublicValues(error) => write!(f, "{error}"),
            Self::SetupPreflight(error) => write!(f, "{error}"),
            Self::UnexpectedProofSegment { id } => {
                write!(f, "unexpected contribution proof segment id {id}")
            }
            Self::PublicValueField(error) => write!(f, "{error}"),
            Self::ProofValues(error) => write!(f, "{error}"),
            Self::ProofValuePacking(error) => write!(f, "{error}"),
            Self::ProofValueMismatch { proof_index } => {
                write!(f, "contribution proof {proof_index} proof values mismatch")
            }
            Self::BindingSegmentMismatch { proof_index, id } => {
                write!(
                    f,
                    "contribution proof {proof_index} binding segment {id} mismatch"
                )
            }
            Self::ChallengeValues(error) => {
                write!(f, "invalid contribution challenge values: {error}")
            }
            Self::DuplicateChallengeValuesSegment => {
                write!(f, "duplicate challenge values segment")
            }
            Self::ContributionChallengeValuesMismatch => {
                write!(f, "contribution challenge values mismatch")
            }
            Self::Contribution(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for ContributionChallengeFileError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Catalog(error) => Some(error),
            Self::SetupDirectoryManifest(error) => Some(error),
            Self::Proof(error) => Some(error),
            Self::PublicValues(error) => Some(error),
            Self::SetupPreflight(error) => Some(error),
            Self::PublicValueField(error) => Some(error),
            Self::ProofValues(error) => Some(error),
            Self::ProofValuePacking(error) => Some(error),
            Self::ChallengeValues(error) => Some(error),
            Self::Contribution(error) => Some(error),
            Self::MissingProofs
            | Self::UnexpectedProofSegment { .. }
            | Self::ProofValueMismatch { .. }
            | Self::BindingSegmentMismatch { .. }
            | Self::DuplicateChallengeValuesSegment
            | Self::ContributionChallengeValuesMismatch => None,
        }
    }
}

impl From<KeyDirectoryError> for ContributionChallengeFileError {
    fn from(error: KeyDirectoryError) -> Self {
        Self::Catalog(error)
    }
}

impl From<SetupDirectoryManifestError> for ContributionChallengeFileError {
    fn from(error: SetupDirectoryManifestError) -> Self {
        Self::SetupDirectoryManifest(error)
    }
}

impl From<ProofArtifactError> for ContributionChallengeFileError {
    fn from(error: ProofArtifactError) -> Self {
        Self::Proof(error)
    }
}

impl From<PublicValuesError> for ContributionChallengeFileError {
    fn from(error: PublicValuesError) -> Self {
        Self::PublicValues(error)
    }
}

impl From<SetupPreflightError> for ContributionChallengeFileError {
    fn from(error: SetupPreflightError) -> Self {
        Self::SetupPreflight(error)
    }
}

impl From<PublicValueFieldError> for ContributionChallengeFileError {
    fn from(error: PublicValueFieldError) -> Self {
        Self::PublicValueField(error)
    }
}

impl From<LoadPcsProofValuesSegmentError> for ContributionChallengeFileError {
    fn from(error: LoadPcsProofValuesSegmentError) -> Self {
        Self::ProofValues(error)
    }
}

impl From<ProvePcsProofValuesSegmentError> for ContributionChallengeFileError {
    fn from(error: ProvePcsProofValuesSegmentError) -> Self {
        Self::ProofValuePacking(error)
    }
}

impl From<ChallengeValuesSegmentError> for ContributionChallengeFileError {
    fn from(error: ChallengeValuesSegmentError) -> Self {
        Self::ChallengeValues(error)
    }
}

impl From<ContributionChallengeError> for ContributionChallengeFileError {
    fn from(error: ContributionChallengeError) -> Self {
        Self::Contribution(error)
    }
}

pub fn build_contribution_segment(
    entries: &[ProveContributionEntry],
) -> Result<Option<ProofSegment>, ProveContributionSegmentError> {
    if entries.is_empty() {
        return Ok(None);
    }

    let segment = ContributionSegment {
        entries: entries
            .iter()
            .map(|entry| ContributionEntry {
                worker_index: entry.worker_index,
                group_id: entry.group_id,
                aggregated: entry.aggregated,
                values: entry.values.iter().map(|value| value.to_u64()).collect(),
            })
            .collect(),
    };
    Ok(Some(ProofSegment {
        id: CONTRIBUTION_SEGMENT_ID,
        data: encode_contribution_segment(&segment)?,
    }))
}

pub fn load_contribution_segment_from_segments(
    segments: &[ProofSegment],
) -> Result<Vec<ProveContributionEntry>, LoadContributionSegmentError> {
    let mut matching_segments = segments
        .iter()
        .filter(|segment| segment.id == CONTRIBUTION_SEGMENT_ID);
    let segment = matching_segments
        .next()
        .ok_or(LoadContributionSegmentError::MissingSegment)?;
    if matching_segments.next().is_some() {
        return Err(LoadContributionSegmentError::DuplicateSegment);
    }
    let parsed =
        parse_contribution_segment(&segment.data).map_err(LoadContributionSegmentError::Segment)?;

    parsed
        .entries
        .into_iter()
        .enumerate()
        .map(raw_contribution_entry)
        .collect()
}

pub fn aggregate_contribution_values(
    global_info: &GlobalInfo,
    entries: &[ProveContributionEntry],
) -> Result<Vec<Felt>, ContributionChallengeError> {
    validate_contribution_entries(entries)?;
    match &global_info.curve {
        CurveKind::None => aggregate_lattice_contributions(global_info, entries),
        CurveKind::EcGfp5 | CurveKind::EcMasFp5 => {
            Err(ContributionChallengeError::UnsupportedCurve {
                curve: global_info.curve.clone(),
            })
        }
    }
}

pub fn build_internal_contribution_input(
    root: [Felt; 4],
    verification_key: &VerificationKeyRoot,
    unit_value_map: &[StageValue],
    packed_unit_values: &[Felt],
) -> Result<InternalContributionInput, ContributionChallengeError> {
    let values =
        build_internal_contribution_values(verification_key, unit_value_map, packed_unit_values)?;
    Ok(InternalContributionInput { root, values })
}

pub fn build_witness_contribution_input(
    verification_key: &VerificationKeyRoot,
    unit: &ProveUnitSchedule,
    output: &ProveWitnessTraceCommitments,
    packed_unit_values: &[Felt],
) -> Result<InternalContributionInput, ContributionChallengeError> {
    let unit_index = output.commitments().unit_index();
    let root = output
        .commitments()
        .stage_commitments()
        .commitments()
        .iter()
        .find(|commitment| commitment.stage_index() == 1)
        .ok_or(ContributionChallengeError::MissingStageOneContributionRoot { unit_index })?
        .root();
    build_internal_contribution_input(
        root,
        verification_key,
        &unit.unit_value_map,
        packed_unit_values,
    )
}

pub fn derive_worker_contribution_entry(
    global_info: &GlobalInfo,
    worker_index: u32,
    group_id: u32,
    inputs: &[InternalContributionInput],
) -> Result<ProveContributionEntry, ContributionChallengeError> {
    if inputs.is_empty() {
        return Err(ContributionChallengeError::EmptyEntries);
    }
    let lattice_size = contribution_lattice_size(global_info)?;
    validate_lattice_hash_width(global_info, lattice_size)?;

    match &global_info.curve {
        CurveKind::None => {
            let mut values = vec![Felt::ZERO; lattice_size];
            for (input_index, input) in inputs.iter().enumerate() {
                let contribution =
                    derive_internal_contribution_values(input_index, input, lattice_size)?;
                for (out, value) in values.iter_mut().zip(contribution) {
                    *out = *out + value;
                }
            }
            Ok(ProveContributionEntry {
                worker_index,
                group_id,
                aggregated: false,
                values,
            })
        }
        CurveKind::EcGfp5 | CurveKind::EcMasFp5 => {
            Err(ContributionChallengeError::UnsupportedCurve {
                curve: global_info.curve.clone(),
            })
        }
    }
}

pub fn derive_global_challenge_from_contributions(
    global_info: &GlobalInfo,
    public_values: &[Felt],
    packed_proof_values: &[Felt],
    entries: &[ProveContributionEntry],
) -> Result<Ext3, ContributionChallengeError> {
    derive_global_challenge_with_bound_segments(
        global_info,
        public_values,
        packed_proof_values,
        &[],
        entries,
    )
}

fn derive_global_challenge_with_bound_segments(
    global_info: &GlobalInfo,
    public_values: &[Felt],
    packed_proof_values: &[Felt],
    bound_segments: &[ProofSegment],
    entries: &[ProveContributionEntry],
) -> Result<Ext3, ContributionChallengeError> {
    let aggregated = aggregate_contribution_values(global_info, entries)?;
    let proof_values = stage_one_proof_values(global_info, packed_proof_values)?;
    let transcript_arity = usize::try_from(global_info.transcript_arity)
        .map_err(|_| ContributionChallengeError::LengthOverflow)?;
    let mut transcript = PoseidonTranscript::new(transcript_arity)?;
    transcript.put(public_values);
    absorb_bound_segments(&mut transcript, bound_segments)?;
    if !proof_values.is_empty() {
        transcript.put(&proof_values);
    }
    transcript.put(&aggregated);
    Ok(transcript.get_field())
}

pub fn derive_global_challenge_from_proof_segments(
    global_info: &GlobalInfo,
    public_values: &[Felt],
    packed_proof_values: &[Felt],
    segments: &[ProofSegment],
) -> Result<Ext3, ContributionChallengeError> {
    let entries = load_contribution_segment_from_segments(segments)?;
    let bound_segments = contribution_bound_segments(segments);
    derive_global_challenge_with_bound_segments(
        global_info,
        public_values,
        packed_proof_values,
        &bound_segments,
        &entries,
    )
}

fn absorb_bound_segments(
    transcript: &mut PoseidonTranscript,
    segments: &[ProofSegment],
) -> Result<(), ContributionChallengeError> {
    if segments.is_empty() {
        return Ok(());
    }
    let segment_count =
        u64::try_from(segments.len()).map_err(|_| ContributionChallengeError::LengthOverflow)?;
    transcript.put(&[Felt::from_u64(segment_count)]);
    for segment in segments {
        transcript.put(&digest_words(&hash_bound_segment(segment)?));
    }
    Ok(())
}

fn contribution_bound_segments(segments: &[ProofSegment]) -> Vec<ProofSegment> {
    let mut segments = segments
        .iter()
        .filter(|segment| {
            matches!(
                segment.id,
                PROGRAM_IMAGE_CACHE_SEGMENT_ID | ETH_BLOCK_INPUT_SEGMENT_ID
            )
        })
        .cloned()
        .collect::<Vec<_>>();
    segments.sort_by_key(|segment| segment.id);
    segments
}

fn hash_bound_segment(segment: &ProofSegment) -> Result<[u8; 32], ContributionChallengeError> {
    let mut hasher = Sha256::new();
    hasher.update(segment.id.to_le_bytes());
    let byte_count = u64::try_from(segment.data.len())
        .map_err(|_| ContributionChallengeError::LengthOverflow)?;
    hasher.update(byte_count.to_le_bytes());
    hasher.update(Sha256::digest(&segment.data));
    Ok(hasher.finalize().into())
}

fn digest_words(digest: &[u8; 32]) -> [Felt; 4] {
    let mut words = [Felt::ZERO; 4];
    for (index, word) in words.iter_mut().enumerate() {
        let start = index * 8;
        let mut bytes = [0_u8; 8];
        bytes.copy_from_slice(&digest[start..start + 8]);
        *word = Felt::from_le_bytes(bytes);
    }
    words
}

fn first_binding_mismatch_id(expected: &[ProofSegment], found: &[ProofSegment]) -> u32 {
    for (left, right) in expected.iter().zip(found) {
        if left != right {
            return left.id.min(right.id);
        }
    }
    expected
        .get(found.len())
        .or_else(|| found.get(expected.len()))
        .map(|segment| segment.id)
        .unwrap_or(0)
}

pub fn derive_global_challenge_from_files(
    setup_dir: impl AsRef<Path>,
    proof_path: impl AsRef<Path>,
    public_values_path: impl AsRef<Path>,
) -> Result<ContributionChallengeReport, ContributionChallengeFileError> {
    let setup_dir = setup_dir.as_ref();
    let catalog = read_key_directory_catalog(setup_dir)?;
    validate_setup_directory_manifest_if_present(setup_dir, &catalog)?;
    let proof = read_proof_artifact_file(proof_path)?;
    let public_values = read_public_values_file(public_values_path)?;
    let public_report = validate_setup_preflight_hashes(&catalog, &proof, &public_values)?;
    validate_contribution_proof_segment_ids(&proof.segments)?;
    let public_fields = public_values_as_fields(&public_values)?;
    let proof_values =
        load_pcs_proof_values_from_segments(&catalog.layout.global_info, &proof.segments)?;
    let packed_proof_values = flatten_pcs_proof_values(&catalog.layout.global_info, &proof_values)?;
    let entries = load_contribution_segment_from_segments(&proof.segments)
        .map_err(ContributionChallengeError::from)?;
    let bound_segments = contribution_bound_segments(&proof.segments);
    let challenge = derive_global_challenge_with_bound_segments(
        &catalog.layout.global_info,
        &public_fields,
        &packed_proof_values,
        &bound_segments,
        &entries,
    )?;
    validate_optional_contribution_challenge_values(&proof.segments, challenge)?;

    Ok(ContributionChallengeReport {
        proof_count: 1,
        segment_count: public_report.segment_count,
        public_value_count: public_report.public_value_count,
        public_values_hash: public_report.public_values_hash,
        public_value_field_count: public_report.public_value_field_count,
        program_image_cache_count: public_report.program_image_cache_count,
        program_image_cache_hashes: public_report.program_image_cache_hashes,
        eth_block_input_count: public_report.eth_block_input_count,
        eth_block_input_hashes: public_report.eth_block_input_hashes,
        eth_block_input_byte_counts: public_report.eth_block_input_byte_counts,
        eth_block_input_block_rlp_byte_counts: public_report.eth_block_input_block_rlp_byte_counts,
        eth_block_input_extra_header_field_counts: public_report
            .eth_block_input_extra_header_field_counts,
        eth_block_input_extra_body_field_counts: public_report
            .eth_block_input_extra_body_field_counts,
        proof_value_count: packed_proof_values.len(),
        contribution_count: entries.len(),
        challenge,
    })
}

pub fn derive_global_challenge_from_contribution_proofs(
    setup_dir: impl AsRef<Path>,
    public_values_path: impl AsRef<Path>,
    proof_paths: &[PathBuf],
) -> Result<ContributionChallengeReport, ContributionChallengeFileError> {
    if proof_paths.is_empty() {
        return Err(ContributionChallengeFileError::MissingProofs);
    }

    let setup_dir = setup_dir.as_ref();
    let catalog = read_key_directory_catalog(setup_dir)?;
    validate_setup_directory_manifest_if_present(setup_dir, &catalog)?;
    let public_values = read_public_values_file(public_values_path)?;
    let public_fields = public_values_as_fields(&public_values)?;

    let mut segment_count = 0_usize;
    let mut public_value_count = None;
    let mut public_values_hash = None;
    let mut proof_value_count = 0_usize;
    let mut program_image_cache_count = None;
    let mut program_image_cache_hashes = None::<Vec<[u8; 32]>>;
    let mut eth_block_input_count = None;
    let mut eth_block_input_hashes = None::<Vec<[u8; 32]>>;
    let mut eth_block_input_byte_counts = None::<Vec<usize>>;
    let mut eth_block_input_block_rlp_byte_counts = None::<Vec<usize>>;
    let mut eth_block_input_extra_header_field_counts = None::<Vec<usize>>;
    let mut eth_block_input_extra_body_field_counts = None::<Vec<usize>>;
    let mut packed_proof_values = None::<Vec<Felt>>;
    let mut bound_segments = None::<Vec<ProofSegment>>;
    let mut entries = Vec::new();
    let mut embedded_challenges = Vec::new();

    for (proof_index, proof_path) in proof_paths.iter().enumerate() {
        let proof = read_proof_artifact_file(proof_path)?;
        let public_report = validate_setup_preflight_hashes(&catalog, &proof, &public_values)?;
        validate_contribution_proof_segment_ids(&proof.segments)?;
        segment_count = segment_count
            .checked_add(public_report.segment_count)
            .ok_or(ContributionChallengeError::LengthOverflow)?;
        public_value_count = public_value_count.or(Some(public_report.public_value_count));
        public_values_hash = public_values_hash.or(Some(public_report.public_values_hash));
        program_image_cache_count =
            program_image_cache_count.or(Some(public_report.program_image_cache_count));
        program_image_cache_hashes =
            program_image_cache_hashes.or(Some(public_report.program_image_cache_hashes.clone()));
        eth_block_input_count = eth_block_input_count.or(Some(public_report.eth_block_input_count));
        eth_block_input_hashes =
            eth_block_input_hashes.or(Some(public_report.eth_block_input_hashes.clone()));
        eth_block_input_byte_counts =
            eth_block_input_byte_counts.or(Some(public_report.eth_block_input_byte_counts.clone()));
        eth_block_input_block_rlp_byte_counts = eth_block_input_block_rlp_byte_counts.or(Some(
            public_report.eth_block_input_block_rlp_byte_counts.clone(),
        ));
        eth_block_input_extra_header_field_counts =
            eth_block_input_extra_header_field_counts.or(Some(
                public_report
                    .eth_block_input_extra_header_field_counts
                    .clone(),
            ));
        eth_block_input_extra_body_field_counts = eth_block_input_extra_body_field_counts.or(Some(
            public_report
                .eth_block_input_extra_body_field_counts
                .clone(),
        ));

        let proof_values =
            load_pcs_proof_values_from_segments(&catalog.layout.global_info, &proof.segments)?;
        let packed = flatten_pcs_proof_values(&catalog.layout.global_info, &proof_values)?;
        if let Some(expected) = &packed_proof_values {
            if expected != &packed {
                return Err(ContributionChallengeFileError::ProofValueMismatch { proof_index });
            }
        } else {
            proof_value_count = packed.len();
            packed_proof_values = Some(packed);
        }
        let proof_bound_segments = contribution_bound_segments(&proof.segments);
        if let Some(expected) = &bound_segments {
            if expected != &proof_bound_segments {
                return Err(ContributionChallengeFileError::BindingSegmentMismatch {
                    proof_index,
                    id: first_binding_mismatch_id(expected, &proof_bound_segments),
                });
            }
        } else {
            bound_segments = Some(proof_bound_segments);
        }

        let mut proof_entries = load_contribution_segment_from_segments(&proof.segments)
            .map_err(ContributionChallengeError::from)?;
        entries.append(&mut proof_entries);
        if let Some(challenge) = load_optional_contribution_challenge_values(&proof.segments)? {
            embedded_challenges.push(challenge);
        }
    }

    let packed_proof_values = packed_proof_values.unwrap_or_default();
    let bound_segments = bound_segments.unwrap_or_default();
    let challenge = derive_global_challenge_with_bound_segments(
        &catalog.layout.global_info,
        &public_fields,
        &packed_proof_values,
        &bound_segments,
        &entries,
    )?;
    for embedded in embedded_challenges {
        if embedded != challenge.to_u64s() {
            return Err(ContributionChallengeFileError::ContributionChallengeValuesMismatch);
        }
    }

    Ok(ContributionChallengeReport {
        proof_count: proof_paths.len(),
        segment_count,
        public_value_count: public_value_count.unwrap_or(0),
        public_values_hash: public_values_hash.unwrap_or([0; 32]),
        public_value_field_count: public_fields.len(),
        program_image_cache_count: program_image_cache_count.unwrap_or(0),
        program_image_cache_hashes: program_image_cache_hashes.unwrap_or_default(),
        eth_block_input_count: eth_block_input_count.unwrap_or(0),
        eth_block_input_hashes: eth_block_input_hashes.unwrap_or_default(),
        eth_block_input_byte_counts: eth_block_input_byte_counts.unwrap_or_default(),
        eth_block_input_block_rlp_byte_counts: eth_block_input_block_rlp_byte_counts
            .unwrap_or_default(),
        eth_block_input_extra_header_field_counts: eth_block_input_extra_header_field_counts
            .unwrap_or_default(),
        eth_block_input_extra_body_field_counts: eth_block_input_extra_body_field_counts
            .unwrap_or_default(),
        proof_value_count,
        contribution_count: entries.len(),
        challenge,
    })
}

fn validate_contribution_proof_segment_ids(
    segments: &[ProofSegment],
) -> Result<(), ContributionChallengeFileError> {
    for segment in segments {
        if is_setup_proof_segment_id(segment.id) {
            continue;
        }
        return Err(ContributionChallengeFileError::UnexpectedProofSegment { id: segment.id });
    }
    Ok(())
}

fn load_optional_contribution_challenge_values(
    segments: &[ProofSegment],
) -> Result<Option<[u64; 3]>, ContributionChallengeFileError> {
    let mut matching_segments = segments
        .iter()
        .filter(|segment| segment.id == CHALLENGE_VALUES_SEGMENT_ID);
    let Some(segment) = matching_segments.next() else {
        return Ok(None);
    };
    if matching_segments.next().is_some() {
        return Err(ContributionChallengeFileError::DuplicateChallengeValuesSegment);
    }

    let values = parse_challenge_values_segment(&segment.data)?.values;
    let [value] = values.as_slice() else {
        return Err(ContributionChallengeFileError::ContributionChallengeValuesMismatch);
    };
    Ok(Some(*value))
}

fn validate_optional_contribution_challenge_values(
    segments: &[ProofSegment],
    expected: Ext3,
) -> Result<(), ContributionChallengeFileError> {
    let Some(values) = load_optional_contribution_challenge_values(segments)? else {
        return Ok(());
    };
    if values != expected.to_u64s() {
        return Err(ContributionChallengeFileError::ContributionChallengeValuesMismatch);
    }
    Ok(())
}

fn aggregate_lattice_contributions(
    global_info: &GlobalInfo,
    entries: &[ProveContributionEntry],
) -> Result<Vec<Felt>, ContributionChallengeError> {
    let expected = contribution_lattice_size(global_info)?;
    let mut out = vec![Felt::ZERO; expected];
    for (entry_index, entry) in entries.iter().enumerate() {
        if entry.values.len() != expected {
            return Err(ContributionChallengeError::ValueCountMismatch {
                entry_index,
                expected,
                found: entry.values.len(),
            });
        }
        for (index, value) in entry.values.iter().copied().enumerate() {
            out[index] = out[index] + value;
        }
    }
    Ok(out)
}

fn build_internal_contribution_values(
    verification_key: &VerificationKeyRoot,
    unit_value_map: &[StageValue],
    packed_unit_values: &[Felt],
) -> Result<Vec<Felt>, ContributionChallengeError> {
    let VerificationKeyRoot::FieldElements(key_values) = verification_key;
    if key_values.len() != CONTRIBUTION_ROOT_SLOT_START {
        return Err(
            ContributionChallengeError::VerificationKeyValueCountMismatch {
                expected: CONTRIBUTION_ROOT_SLOT_START,
                found: key_values.len(),
            },
        );
    }

    let expected_unit_values = expected_packed_stage_value_count(unit_value_map)?;
    if packed_unit_values.len() != expected_unit_values {
        return Err(ContributionChallengeError::UnitValueCountMismatch {
            expected: expected_unit_values,
            found: packed_unit_values.len(),
        });
    }

    let stage_one_count = unit_value_map.iter().try_fold(0_usize, |count, entry| {
        if entry.stage == 1 {
            count
                .checked_add(stage_value_dimension(entry)?)
                .ok_or(ContributionChallengeError::LengthOverflow)
        } else {
            Ok(count)
        }
    })?;
    let capacity = CONTRIBUTION_ROOT_SLOT_END
        .checked_add(stage_one_count)
        .ok_or(ContributionChallengeError::LengthOverflow)?;
    let mut values = Vec::with_capacity(capacity);
    for (index, value) in key_values.iter().copied().enumerate() {
        values.push(Felt::from_canonical(value).map_err(|source| {
            ContributionChallengeError::VerificationKeyNonCanonicalValue { index, source }
        })?);
    }
    values.extend([Felt::ZERO; CONTRIBUTION_ROOT_SLOT_START]);

    let mut offset = 0_usize;
    for entry in unit_value_map {
        let dimension = stage_value_dimension(entry)?;
        if entry.stage == 1 {
            let end = offset
                .checked_add(dimension)
                .ok_or(ContributionChallengeError::LengthOverflow)?;
            values.extend_from_slice(&packed_unit_values[offset..end]);
            offset = end;
        } else {
            let width = dimension
                .checked_mul(3)
                .ok_or(ContributionChallengeError::LengthOverflow)?;
            offset = offset
                .checked_add(width)
                .ok_or(ContributionChallengeError::LengthOverflow)?;
        }
    }
    Ok(values)
}

fn derive_internal_contribution_values(
    input_index: usize,
    input: &InternalContributionInput,
    lattice_size: usize,
) -> Result<Vec<Felt>, ContributionChallengeError> {
    if input.values.len() < CONTRIBUTION_ROOT_SLOT_END {
        return Err(ContributionChallengeError::ContributionInputTooShort {
            input_index,
            found: input.values.len(),
        });
    }

    let mut values_to_hash = input.values.clone();
    values_to_hash[CONTRIBUTION_ROOT_SLOT_START..CONTRIBUTION_ROOT_SLOT_END]
        .copy_from_slice(&input.root);

    let mut transcript = PoseidonTranscript::new(4)?;
    transcript.put(&values_to_hash);
    let state = transcript.get_state_words();

    let mut values = vec![Felt::ZERO; lattice_size];
    values[..CONTRIBUTION_HASH_STATE_WIDTH]
        .copy_from_slice(&state[..CONTRIBUTION_HASH_STATE_WIDTH]);

    let mut offset = CONTRIBUTION_HASH_STATE_WIDTH;
    while offset < values.len() {
        let mut input = [Felt::ZERO; CONTRIBUTION_HASH_STATE_WIDTH];
        input.copy_from_slice(&values[offset - CONTRIBUTION_HASH_STATE_WIDTH..offset]);
        let output = poseidon2_hash_16(input);
        values[offset..offset + CONTRIBUTION_HASH_STATE_WIDTH].copy_from_slice(&output);
        offset += CONTRIBUTION_HASH_STATE_WIDTH;
    }
    Ok(values)
}

fn expected_packed_stage_value_count(
    unit_value_map: &[StageValue],
) -> Result<usize, ContributionChallengeError> {
    unit_value_map.iter().try_fold(0_usize, |count, value| {
        let dimension = stage_value_dimension(value)?;
        let width = if value.stage == 1 { 1 } else { 3 };
        let value_count = dimension
            .checked_mul(width)
            .ok_or(ContributionChallengeError::LengthOverflow)?;
        count
            .checked_add(value_count)
            .ok_or(ContributionChallengeError::LengthOverflow)
    })
}

fn stage_value_dimension(entry: &StageValue) -> Result<usize, ContributionChallengeError> {
    entry.lengths.iter().try_fold(1_usize, |dimension, length| {
        let length =
            usize::try_from(*length).map_err(|_| ContributionChallengeError::LengthOverflow)?;
        dimension
            .checked_mul(length)
            .ok_or(ContributionChallengeError::LengthOverflow)
    })
}

fn contribution_lattice_size(
    global_info: &GlobalInfo,
) -> Result<usize, ContributionChallengeError> {
    global_info
        .lattice_size
        .ok_or(ContributionChallengeError::MissingLatticeSize)
        .and_then(|value| {
            usize::try_from(value)
                .map_err(|_| ContributionChallengeError::LatticeSizeOverflow { value })
        })
}

fn validate_lattice_hash_width(
    global_info: &GlobalInfo,
    lattice_size: usize,
) -> Result<(), ContributionChallengeError> {
    let value = global_info
        .lattice_size
        .ok_or(ContributionChallengeError::MissingLatticeSize)?;
    if lattice_size == 0 || !lattice_size.is_multiple_of(CONTRIBUTION_HASH_STATE_WIDTH) {
        return Err(ContributionChallengeError::LatticeSizeNotMultipleOfHashState { value });
    }
    Ok(())
}

fn stage_one_proof_values(
    global_info: &GlobalInfo,
    packed_proof_values: &[Felt],
) -> Result<Vec<Felt>, ContributionChallengeError> {
    let expected = expected_packed_proof_value_count(global_info)?;
    if packed_proof_values.len() != expected {
        return Err(ContributionChallengeError::ProofValueCountMismatch {
            expected,
            found: packed_proof_values.len(),
        });
    }

    let mut out = Vec::with_capacity(global_info.stage_one_proof_value_count());
    let mut offset = 0_usize;
    for entry in &global_info.proof_values_map {
        let dimension = proof_value_dimension(entry)?;
        if entry.stage == 1 {
            let end = offset
                .checked_add(dimension)
                .ok_or(ContributionChallengeError::LengthOverflow)?;
            out.extend_from_slice(&packed_proof_values[offset..end]);
            offset = end;
        } else {
            let width = dimension
                .checked_mul(3)
                .ok_or(ContributionChallengeError::LengthOverflow)?;
            offset = offset
                .checked_add(width)
                .ok_or(ContributionChallengeError::LengthOverflow)?;
        }
    }
    Ok(out)
}

fn expected_packed_proof_value_count(
    global_info: &GlobalInfo,
) -> Result<usize, ContributionChallengeError> {
    global_info
        .proof_values_map
        .iter()
        .try_fold(0_usize, |count, entry| {
            let dimension = proof_value_dimension(entry)?;
            let width = if entry.stage == 1 { 1 } else { 3 };
            let entry_count = dimension
                .checked_mul(width)
                .ok_or(ContributionChallengeError::LengthOverflow)?;
            count
                .checked_add(entry_count)
                .ok_or(ContributionChallengeError::LengthOverflow)
        })
}

fn proof_value_dimension(entry: &NamedStageValue) -> Result<usize, ContributionChallengeError> {
    entry.lengths.iter().try_fold(1_usize, |dimension, length| {
        let length =
            usize::try_from(*length).map_err(|_| ContributionChallengeError::LengthOverflow)?;
        dimension
            .checked_mul(length)
            .ok_or(ContributionChallengeError::LengthOverflow)
    })
}

fn validate_contribution_entries(
    entries: &[ProveContributionEntry],
) -> Result<(), ContributionChallengeError> {
    if entries.is_empty() {
        return Err(ContributionChallengeError::EmptyEntries);
    }
    let mut seen = BTreeSet::new();
    for (entry_index, entry) in entries.iter().enumerate() {
        if entry.values.is_empty() {
            return Err(ContributionChallengeError::EmptyValues { entry_index });
        }
        if !seen.insert((entry.worker_index, entry.group_id)) {
            return Err(ContributionChallengeError::DuplicateEntry {
                worker_index: entry.worker_index,
                group_id: entry.group_id,
            });
        }
    }
    Ok(())
}

fn raw_contribution_entry(
    (entry_index, entry): (usize, ContributionEntry),
) -> Result<ProveContributionEntry, LoadContributionSegmentError> {
    let values = entry
        .values
        .into_iter()
        .enumerate()
        .map(|(index, value)| {
            Felt::from_canonical(value).map_err(|source| {
                LoadContributionSegmentError::NonCanonicalValue {
                    entry_index,
                    index,
                    source,
                }
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(ProveContributionEntry {
        worker_index: entry.worker_index,
        group_id: entry.group_id,
        aggregated: entry.aggregated,
        values,
    })
}

#[cfg(test)]
mod tests {
    use lzvm_artifacts::challenge_values_segment::CHALLENGE_VALUES_SEGMENT_ID;
    use lzvm_artifacts::constant_opening_segment::CONSTANT_OPENING_SEGMENT_ID;
    use lzvm_artifacts::eth_block_input_segment::ETH_BLOCK_INPUT_SEGMENT_ID;
    use lzvm_artifacts::group_values_segment::GROUP_VALUES_SEGMENT_ID;
    use lzvm_artifacts::pcs_evaluation_segment::PCS_EVALUATION_SEGMENT_ID;
    use lzvm_artifacts::pcs_fri_segment::PCS_FRI_OPENING_SEGMENT_ID;
    use lzvm_artifacts::pcs_material_segment::PCS_MATERIAL_MANIFEST_SEGMENT_ID;
    use lzvm_artifacts::pcs_nonce_segment::PCS_QUERY_NONCE_SEGMENT_ID;
    use lzvm_artifacts::pcs_proof_values_segment::PCS_PROOF_VALUES_SEGMENT_ID;
    use lzvm_artifacts::pcs_query_segment::PCS_QUERY_PLAN_SEGMENT_ID;
    use lzvm_artifacts::program_image_segment::PROGRAM_IMAGE_CACHE_SEGMENT_ID;
    use lzvm_artifacts::proof::ProofSegment;
    use lzvm_artifacts::unit_values_segment::UNIT_VALUES_SEGMENT_ID;
    use lzvm_artifacts::witness_opening_segment::WITNESS_OPENING_SEGMENT_ID;
    use lzvm_artifacts::witness_segment::WITNESS_COMMITMENT_SEGMENT_BASE_ID;

    use super::validate_contribution_proof_segment_ids;
    use crate::contribution::CONTRIBUTION_SEGMENT_ID;

    #[test]
    fn accepts_binding_segments_in_contribution_proof_inputs() {
        let segments = vec![
            ProofSegment {
                id: PCS_PROOF_VALUES_SEGMENT_ID,
                data: vec![1],
            },
            ProofSegment {
                id: CONTRIBUTION_SEGMENT_ID,
                data: vec![2],
            },
            ProofSegment {
                id: PROGRAM_IMAGE_CACHE_SEGMENT_ID,
                data: vec![3],
            },
            ProofSegment {
                id: WITNESS_COMMITMENT_SEGMENT_BASE_ID,
                data: vec![4],
            },
            ProofSegment {
                id: WITNESS_COMMITMENT_SEGMENT_BASE_ID + 1,
                data: vec![5],
            },
            ProofSegment {
                id: PCS_MATERIAL_MANIFEST_SEGMENT_ID,
                data: vec![6],
            },
            ProofSegment {
                id: PCS_QUERY_PLAN_SEGMENT_ID,
                data: vec![7],
            },
            ProofSegment {
                id: WITNESS_OPENING_SEGMENT_ID,
                data: vec![8],
            },
            ProofSegment {
                id: CONSTANT_OPENING_SEGMENT_ID,
                data: vec![9],
            },
            ProofSegment {
                id: PCS_FRI_OPENING_SEGMENT_ID,
                data: vec![10],
            },
            ProofSegment {
                id: PCS_QUERY_NONCE_SEGMENT_ID,
                data: vec![11],
            },
            ProofSegment {
                id: PCS_EVALUATION_SEGMENT_ID,
                data: vec![12],
            },
            ProofSegment {
                id: GROUP_VALUES_SEGMENT_ID,
                data: vec![13],
            },
            ProofSegment {
                id: UNIT_VALUES_SEGMENT_ID,
                data: vec![14],
            },
            ProofSegment {
                id: CHALLENGE_VALUES_SEGMENT_ID,
                data: vec![15],
            },
            ProofSegment {
                id: ETH_BLOCK_INPUT_SEGMENT_ID,
                data: vec![16],
            },
        ];

        validate_contribution_proof_segment_ids(&segments)
            .expect("binding segments should be allowed in contribution proofs");
    }
}
