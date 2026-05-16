use std::collections::BTreeSet;
use std::fmt;
use std::path::{Path, PathBuf};

use lzvm_artifacts::contribution_segment::{
    encode_contribution_segment, parse_contribution_segment, ContributionEntry,
    ContributionSegment, ContributionSegmentError, CONTRIBUTION_SEGMENT_ID,
};
use lzvm_artifacts::global_info::{CurveKind, GlobalInfo};
use lzvm_artifacts::key_directory::{read_key_directory_catalog, KeyDirectoryError};
use lzvm_artifacts::proof::{read_proof_artifact_file, ProofArtifactError, ProofSegment};
use lzvm_artifacts::public_values::{read_public_values_file, PublicValuesError};
use lzvm_artifacts::setup_manifest::SetupDirectoryManifestError;
use lzvm_field::{poseidon2_hash_16, Ext3, Felt, FieldError, PoseidonTranscript, TranscriptError};

use crate::proof_preflight::{public_values_as_fields, PublicValueFieldError};
use crate::proof_values::{
    flatten_pcs_proof_values, load_pcs_proof_values_from_segments, LoadPcsProofValuesSegmentError,
    ProvePcsProofValuesSegmentError,
};
use crate::setup_preflight::{
    validate_setup_directory_manifest_if_present, validate_setup_preflight_hashes,
    SetupPreflightError,
};

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
    PublicValueField(PublicValueFieldError),
    ProofValues(LoadPcsProofValuesSegmentError),
    ProofValuePacking(ProvePcsProofValuesSegmentError),
    ProofValueMismatch { proof_index: usize },
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
            Self::MissingSegment => None,
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
            Self::PublicValueField(error) => write!(f, "{error}"),
            Self::ProofValues(error) => write!(f, "{error}"),
            Self::ProofValuePacking(error) => write!(f, "{error}"),
            Self::ProofValueMismatch { proof_index } => {
                write!(f, "contribution proof {proof_index} proof values mismatch")
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
            Self::Contribution(error) => Some(error),
            Self::MissingProofs | Self::ProofValueMismatch { .. } => None,
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
    let segment = segments
        .iter()
        .find(|segment| segment.id == CONTRIBUTION_SEGMENT_ID)
        .ok_or(LoadContributionSegmentError::MissingSegment)?;
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
    let aggregated = aggregate_contribution_values(global_info, entries)?;
    let proof_values = stage_one_proof_values(global_info, packed_proof_values)?;
    let transcript_arity = usize::try_from(global_info.transcript_arity)
        .map_err(|_| ContributionChallengeError::LengthOverflow)?;
    let mut transcript = PoseidonTranscript::new(transcript_arity)?;
    transcript.put(public_values);
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
    derive_global_challenge_from_contributions(
        global_info,
        public_values,
        packed_proof_values,
        &entries,
    )
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
    let public_fields = public_values_as_fields(&public_values)?;
    let proof_values =
        load_pcs_proof_values_from_segments(&catalog.layout.global_info, &proof.segments)?;
    let packed_proof_values = flatten_pcs_proof_values(&catalog.layout.global_info, &proof_values)?;
    let entries = load_contribution_segment_from_segments(&proof.segments)
        .map_err(ContributionChallengeError::from)?;
    let challenge = derive_global_challenge_from_contributions(
        &catalog.layout.global_info,
        &public_fields,
        &packed_proof_values,
        &entries,
    )?;

    Ok(ContributionChallengeReport {
        proof_count: 1,
        segment_count: public_report.segment_count,
        public_value_count: public_report.public_value_count,
        proof_value_count: proof_values.len(),
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
    let mut proof_value_count = 0_usize;
    let mut packed_proof_values = None::<Vec<Felt>>;
    let mut entries = Vec::new();

    for (proof_index, proof_path) in proof_paths.iter().enumerate() {
        let proof = read_proof_artifact_file(proof_path)?;
        let public_report = validate_setup_preflight_hashes(&catalog, &proof, &public_values)?;
        segment_count = segment_count
            .checked_add(public_report.segment_count)
            .ok_or(ContributionChallengeError::LengthOverflow)?;
        public_value_count = public_value_count.or(Some(public_report.public_value_count));

        let proof_values =
            load_pcs_proof_values_from_segments(&catalog.layout.global_info, &proof.segments)?;
        let packed = flatten_pcs_proof_values(&catalog.layout.global_info, &proof_values)?;
        if let Some(expected) = &packed_proof_values {
            if expected != &packed {
                return Err(ContributionChallengeFileError::ProofValueMismatch { proof_index });
            }
        } else {
            proof_value_count = proof_values.len();
            packed_proof_values = Some(packed);
        }

        let mut proof_entries = load_contribution_segment_from_segments(&proof.segments)
            .map_err(ContributionChallengeError::from)?;
        entries.append(&mut proof_entries);
    }

    let packed_proof_values = packed_proof_values.unwrap_or_default();
    let challenge = derive_global_challenge_from_contributions(
        &catalog.layout.global_info,
        &public_fields,
        &packed_proof_values,
        &entries,
    )?;

    Ok(ContributionChallengeReport {
        proof_count: proof_paths.len(),
        segment_count,
        public_value_count: public_value_count.unwrap_or(0),
        proof_value_count,
        contribution_count: entries.len(),
        challenge,
    })
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
    if !lattice_size.is_multiple_of(CONTRIBUTION_HASH_STATE_WIDTH) {
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
        if entry.stage == 1 {
            out.push(packed_proof_values[offset]);
            offset = offset
                .checked_add(1)
                .ok_or(ContributionChallengeError::LengthOverflow)?;
        } else {
            offset = offset
                .checked_add(3)
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
            count
                .checked_add(if entry.stage == 1 { 1 } else { 3 })
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
