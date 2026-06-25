use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

use lzvm_artifacts::challenge_values_segment::{
    parse_challenge_values_segment, ChallengeValuesSegmentError, CHALLENGE_VALUES_SEGMENT_ID,
};
use lzvm_artifacts::constant_opening_segment::CONSTANT_OPENING_SEGMENT_ID;
use lzvm_artifacts::contribution_segment::CONTRIBUTION_SEGMENT_ID;
use lzvm_artifacts::eth_block_input_segment::ETH_BLOCK_INPUT_SEGMENT_ID;
use lzvm_artifacts::global_info::{GlobalInfo, PublicValue};
use lzvm_artifacts::group_values_segment::GROUP_VALUES_SEGMENT_ID;
use lzvm_artifacts::key_directory::{
    key_directory_catalog_digest, read_key_directory_catalog, KeyDirectoryCatalog,
    KeyDirectoryError,
};
use lzvm_artifacts::pcs_evaluation_segment::PCS_EVALUATION_SEGMENT_ID;
use lzvm_artifacts::pcs_fri_segment::PCS_FRI_OPENING_SEGMENT_ID;
use lzvm_artifacts::pcs_material_segment::PCS_MATERIAL_MANIFEST_SEGMENT_ID;
use lzvm_artifacts::pcs_nonce_segment::PCS_QUERY_NONCE_SEGMENT_ID;
use lzvm_artifacts::pcs_proof_values_segment::PCS_PROOF_VALUES_SEGMENT_ID;
use lzvm_artifacts::pcs_query_segment::PCS_QUERY_PLAN_SEGMENT_ID;
use lzvm_artifacts::program_image::ProgramImageCommitmentCache;
use lzvm_artifacts::program_image_segment::PROGRAM_IMAGE_CACHE_SEGMENT_ID;
use lzvm_artifacts::proof::{
    read_proof_artifact_file, ProofArtifact, ProofArtifactError, ProofSegment,
};
use lzvm_artifacts::public_values::{read_public_values_file, PublicValues, PublicValuesError};
use lzvm_artifacts::setup_manifest::{
    build_setup_directory_manifest, validate_setup_directory_manifest_file,
    SetupDirectoryManifestError, SETUP_DIRECTORY_MANIFEST_FILE,
};
use lzvm_artifacts::trace_constraint_segment::{
    parse_trace_constraint_segment, TraceConstraintSegmentError, TRACE_CONSTRAINT_SEGMENT_ID,
};
use lzvm_artifacts::unit_values_segment::UNIT_VALUES_SEGMENT_ID;
use lzvm_artifacts::witness_opening_segment::WITNESS_OPENING_SEGMENT_ID;
use lzvm_artifacts::witness_segment::{
    parse_witness_commitment_segment, witness_commitment_segment_identity,
    WITNESS_COMMITMENT_SEGMENT_BASE_ID,
};
use lzvm_field::{Ext3, Felt, FieldError};

use crate::constant_opening::{
    validate_constant_opening_segments, ValidateConstantOpeningSegmentsError,
};
use crate::contribution::{
    aggregate_contribution_values, derive_global_challenge_from_proof_segments,
    load_contribution_segment_from_segments, ContributionChallengeError,
};
use crate::global_constraints::{
    validate_global_constraints, GlobalConstraintInputs, ValidateGlobalConstraintProofSegmentsError,
};
use crate::group_values::{load_group_values_from_segments, LoadGroupValuesSegmentError};
use crate::hint_eval::{
    global_hint_input_requirements, resolve_global_hint_program,
    ResolveGlobalHintProofSegmentsError, ResolvedHint,
};
use crate::pcs_fri::{
    validate_optional_pcs_fri_opening_proof_segments,
    validate_optional_pcs_fri_opening_proof_segments_with_transcript_challenges,
    ValidateOptionalPcsFriOpeningProofSegmentsError,
    ValidateOptionalPcsFriOpeningProofSegmentsRequest,
};
use crate::pcs_material_manifest::{
    validate_pcs_material_manifest_segments, ValidatePcsMaterialManifestSegmentsError,
};
use crate::pcs_query_plan::{
    load_pcs_query_plan_from_segments, uses_transcript_pcs_query_plan_inputs,
    validate_pcs_query_plan_segments, LoadPcsQueryPlanSegmentError,
    ValidatePcsQueryPlanSegmentsError,
};
use crate::pcs_transcript_segments::{
    derive_pcs_transcript_unit_challenges_from_proof_segments, PcsTranscriptProofSegmentsError,
    PcsTranscriptUnitChallenges,
};
use crate::proof_preflight::{
    public_values_as_fields, validate_proof_public_values_for_setup_preflight, ProofPreflightError,
    ProofPreflightReport, PublicValueFieldError, TraceConstraintPreflightUnit,
};
use crate::proof_values::{
    flatten_pcs_proof_values, load_pcs_proof_values_from_segments, LoadPcsProofValuesSegmentError,
    ProvePcsProofValuesSegmentError,
};
use crate::source_lookup_hints::{SourceLookupBalance, SourceLookupHintError};
use crate::unit_values::{
    load_unit_values_for_identity_from_parsed_segment, load_unit_values_segment_from_segments,
    validate_unit_values_units_match_query_units_from_segment, LoadUnitValuesSegmentError,
};
use crate::witness_commitment::{
    load_witness_commitment_segment_refs, LoadWitnessCommitmentSegmentsError,
};
use crate::witness_opening::{
    validate_witness_opening_segments, ValidateWitnessOpeningSegmentsError,
};
use crate::{derive_prove_schedule, ProveScheduleError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupPreflightReport {
    pub unit_count: usize,
    pub segment_count: usize,
    pub public_value_count: usize,
    pub public_values_hash: [u8; 32],
    pub public_value_field_count: usize,
    pub source_fixed_file_manifest_present: bool,
    pub source_fixed_file_manifest_entry_count: usize,
    pub source_program_archive_present: bool,
    pub source_program_archive_source_count: usize,
    pub source_program_archive_edge_count: usize,
    pub program_image_cache_count: usize,
    pub program_image_caches: Vec<ProgramImageCommitmentCache>,
    pub program_image_cache_hashes: Vec<[u8; 32]>,
    pub challenge_values_segment_count: usize,
    pub challenge_values_segment_byte_counts: Vec<usize>,
    pub challenge_values_value_counts: Vec<usize>,
    pub trace_constraint_segment_count: usize,
    pub trace_constraint_segment_byte_counts: Vec<usize>,
    pub trace_constraint_units: Vec<TraceConstraintPreflightUnit>,
    pub eth_block_input_count: usize,
    pub eth_block_input_hashes: Vec<[u8; 32]>,
    pub eth_block_input_byte_counts: Vec<usize>,
    pub eth_block_input_block_rlp_byte_counts: Vec<usize>,
    pub eth_block_input_extra_header_field_counts: Vec<usize>,
    pub eth_block_input_extra_body_field_counts: Vec<usize>,
    pub eth_block_input_block_hashes: Vec<[u8; 32]>,
    pub eth_block_input_parent_hashes: Vec<[u8; 32]>,
    pub eth_block_input_ommers_hashes: Vec<[u8; 32]>,
    pub eth_block_input_beneficiaries: Vec<[u8; 20]>,
    pub eth_block_input_state_roots: Vec<[u8; 32]>,
    pub eth_block_input_receipt_roots: Vec<[u8; 32]>,
    pub eth_block_input_logs_blooms: Vec<[u8; 256]>,
    pub eth_block_input_difficulties: Vec<[u8; 32]>,
    pub eth_block_input_block_numbers: Vec<u64>,
    pub eth_block_input_timestamps: Vec<u64>,
    pub eth_block_input_extra_data: Vec<Vec<u8>>,
    pub eth_block_input_gas_limits: Vec<u64>,
    pub eth_block_input_gas_used_values: Vec<u64>,
    pub eth_block_input_base_fees_per_gas: Vec<Option<[u8; 32]>>,
    pub eth_block_input_mix_hashes: Vec<[u8; 32]>,
    pub eth_block_input_nonces: Vec<[u8; 8]>,
    pub eth_block_input_transaction_roots: Vec<[u8; 32]>,
    pub eth_block_input_transaction_preimage_counts: Vec<usize>,
    pub eth_block_input_legacy_transaction_counts: Vec<usize>,
    pub eth_block_input_typed_transaction_counts: Vec<usize>,
    pub eth_block_input_receipts_rlp_byte_counts: Vec<Option<usize>>,
    pub eth_block_input_receipt_preimage_counts: Vec<Option<usize>>,
    pub eth_block_input_legacy_receipt_counts: Vec<Option<usize>>,
    pub eth_block_input_typed_receipt_counts: Vec<Option<usize>>,
    pub eth_block_input_withdrawal_roots: Vec<Option<[u8; 32]>>,
    pub eth_block_input_withdrawal_counts: Vec<Option<usize>>,
    pub eth_block_input_withdrawal_preimage_counts: Vec<Option<usize>>,
}

struct SetupPreflightGlobalValues {
    proof_values: Vec<Ext3>,
    group_values: Vec<Ext3>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetupPreflightError {
    Catalog(KeyDirectoryError),
    Proof(ProofPreflightError),
    CatalogHashMismatch,
    ProgramImageCacheSetupHashMismatch,
    SetupDirectoryManifest(SetupDirectoryManifestError),
    Schedule(ProveScheduleError),
    PublicValues(PublicValueFieldError),
    PcsMaterial(ValidatePcsMaterialManifestSegmentsError),
    WitnessCommitment(LoadWitnessCommitmentSegmentsError),
    PcsQueryPlan(ValidatePcsQueryPlanSegmentsError),
    ConstantOpening(ValidateConstantOpeningSegmentsError),
    WitnessOpening(ValidateWitnessOpeningSegmentsError),
    GlobalConstraints(ValidateGlobalConstraintProofSegmentsError),
    GlobalHints(ResolveGlobalHintProofSegmentsError),
    SourceLookup {
        message: String,
    },
    PcsFri(ValidateOptionalPcsFriOpeningProofSegmentsError),
    Contribution(ContributionChallengeError),
    ChallengeValues(ChallengeValuesSegmentError),
    ChallengeValueNonCanonical {
        value_index: usize,
        word_index: usize,
        source: FieldError,
    },
    DuplicateChallengeValuesSegment,
    ProofValues(LoadPcsProofValuesSegmentError),
    ProofValuePacking(ProvePcsProofValuesSegmentError),
    GroupValues(LoadGroupValuesSegmentError),
    UnitValues(LoadUnitValuesSegmentError),
    UnitValueQueryPlan(LoadPcsQueryPlanSegmentError),
    TraceConstraint(TraceConstraintSegmentError),
    TraceConstraintBinding {
        message: String,
    },
    MissingContributionChallengeValues,
    ContributionChallengeValuesMismatch,
    UnexpectedProofSegment {
        id: u32,
    },
    PublicValueEntryCountMismatch {
        expected: usize,
        found: usize,
    },
    PublicValueNameMismatch {
        index: usize,
        expected: String,
        found: String,
    },
    PublicValueElementCountMismatch {
        name: String,
        expected: usize,
        found: usize,
    },
    PublicValueFieldCountMismatch {
        expected: usize,
        found: usize,
    },
    PublicValueCountOverflow,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetupPreflightFileError {
    Catalog(KeyDirectoryError),
    Proof(ProofArtifactError),
    PublicValues(PublicValuesError),
    SetupPreflight(SetupPreflightError),
}

impl fmt::Display for SetupPreflightError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Catalog(error) => write!(f, "{error}"),
            Self::Proof(error) => write!(f, "{error}"),
            Self::CatalogHashMismatch => write!(f, "setup catalog fingerprint mismatch"),
            Self::ProgramImageCacheSetupHashMismatch => {
                write!(f, "program image cache setup hash mismatch")
            }
            Self::SetupDirectoryManifest(error) => write!(f, "{error}"),
            Self::Schedule(error) => write!(f, "{error}"),
            Self::PublicValues(error) => write!(f, "{error}"),
            Self::PcsMaterial(error) => write!(f, "{error}"),
            Self::WitnessCommitment(error) => write!(f, "{error}"),
            Self::PcsQueryPlan(error) => write!(f, "{error}"),
            Self::ConstantOpening(error) => write!(f, "{error}"),
            Self::WitnessOpening(error) => write!(f, "{error}"),
            Self::GlobalConstraints(error) => write!(f, "{error}"),
            Self::GlobalHints(error) => write!(f, "{error}"),
            Self::SourceLookup { message } => write!(f, "{message}"),
            Self::PcsFri(error) => write!(f, "{error}"),
            Self::Contribution(error) => write!(f, "{error}"),
            Self::ChallengeValues(error) => write!(f, "{error}"),
            Self::ChallengeValueNonCanonical {
                value_index,
                word_index,
                source,
            } => write!(
                f,
                "invalid challenge values segment value {value_index} word {word_index}: {source}"
            ),
            Self::DuplicateChallengeValuesSegment => {
                write!(f, "duplicate challenge values segment")
            }
            Self::ProofValues(error) => write!(f, "{error}"),
            Self::ProofValuePacking(error) => write!(f, "{error}"),
            Self::GroupValues(error) => write!(f, "{error}"),
            Self::UnitValues(error) => write!(f, "{error}"),
            Self::UnitValueQueryPlan(error) => write!(f, "{error}"),
            Self::TraceConstraint(error) => write!(f, "{error}"),
            Self::TraceConstraintBinding { message } => write!(f, "{message}"),
            Self::MissingContributionChallengeValues => {
                write!(f, "missing contribution challenge values")
            }
            Self::ContributionChallengeValuesMismatch => {
                write!(f, "contribution challenge values mismatch")
            }
            Self::UnexpectedProofSegment { id } => {
                write!(f, "unexpected setup proof segment id {id}")
            }
            Self::PublicValueEntryCountMismatch { expected, found } => write!(
                f,
                "public-values entry count mismatch: expected {expected}, found {found}"
            ),
            Self::PublicValueNameMismatch {
                index,
                expected,
                found,
            } => write!(
                f,
                "public-values entry {index} name mismatch: expected {expected}, found {found}"
            ),
            Self::PublicValueElementCountMismatch {
                name,
                expected,
                found,
            } => write!(
                f,
                "public-values entry {name} element count mismatch: expected {expected}, found {found}"
            ),
            Self::PublicValueFieldCountMismatch { expected, found } => write!(
                f,
                "public-values field count mismatch: expected {expected}, found {found}"
            ),
            Self::PublicValueCountOverflow => write!(f, "public-values count overflow"),
        }
    }
}

impl fmt::Display for SetupPreflightFileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Catalog(error) => write!(f, "{error}"),
            Self::Proof(error) => write!(f, "{error}"),
            Self::PublicValues(error) => write!(f, "{error}"),
            Self::SetupPreflight(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for SetupPreflightError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Catalog(error) => Some(error),
            Self::Proof(error) => Some(error),
            Self::SetupDirectoryManifest(error) => Some(error),
            Self::Schedule(error) => Some(error),
            Self::PublicValues(error) => Some(error),
            Self::PcsMaterial(error) => Some(error),
            Self::WitnessCommitment(error) => Some(error),
            Self::PcsQueryPlan(error) => Some(error),
            Self::ConstantOpening(error) => Some(error),
            Self::WitnessOpening(error) => Some(error),
            Self::GlobalConstraints(error) => Some(error),
            Self::GlobalHints(error) => Some(error),
            Self::PcsFri(error) => Some(error),
            Self::Contribution(error) => Some(error),
            Self::ChallengeValues(error) => Some(error),
            Self::ChallengeValueNonCanonical { source, .. } => Some(source),
            Self::ProofValues(error) => Some(error),
            Self::ProofValuePacking(error) => Some(error),
            Self::GroupValues(error) => Some(error),
            Self::UnitValues(error) => Some(error),
            Self::UnitValueQueryPlan(error) => Some(error),
            Self::TraceConstraint(error) => Some(error),
            Self::CatalogHashMismatch
            | Self::ProgramImageCacheSetupHashMismatch
            | Self::SourceLookup { .. }
            | Self::TraceConstraintBinding { .. }
            | Self::MissingContributionChallengeValues
            | Self::ContributionChallengeValuesMismatch
            | Self::DuplicateChallengeValuesSegment
            | Self::UnexpectedProofSegment { .. }
            | Self::PublicValueEntryCountMismatch { .. }
            | Self::PublicValueNameMismatch { .. }
            | Self::PublicValueElementCountMismatch { .. }
            | Self::PublicValueFieldCountMismatch { .. }
            | Self::PublicValueCountOverflow => None,
        }
    }
}

impl std::error::Error for SetupPreflightFileError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Catalog(error) => Some(error),
            Self::Proof(error) => Some(error),
            Self::PublicValues(error) => Some(error),
            Self::SetupPreflight(error) => Some(error),
        }
    }
}

impl From<KeyDirectoryError> for SetupPreflightFileError {
    fn from(error: KeyDirectoryError) -> Self {
        Self::Catalog(error)
    }
}

impl From<ProofArtifactError> for SetupPreflightFileError {
    fn from(error: ProofArtifactError) -> Self {
        Self::Proof(error)
    }
}

impl From<PublicValuesError> for SetupPreflightFileError {
    fn from(error: PublicValuesError) -> Self {
        Self::PublicValues(error)
    }
}

impl From<SetupPreflightError> for SetupPreflightFileError {
    fn from(error: SetupPreflightError) -> Self {
        Self::SetupPreflight(error)
    }
}

pub fn validate_setup_directory_manifest_if_present(
    root: &Path,
    catalog: &KeyDirectoryCatalog,
) -> Result<(), SetupDirectoryManifestError> {
    let expected = build_setup_directory_manifest(catalog)?;
    let path = root.join(SETUP_DIRECTORY_MANIFEST_FILE);
    validate_setup_directory_manifest_file(&path, &expected)
}

pub fn validate_setup_preflight_hashes(
    catalog: &KeyDirectoryCatalog,
    proof: &ProofArtifact,
    public_values: &PublicValues,
) -> Result<SetupPreflightReport, SetupPreflightError> {
    if proof.setup_hash != public_values.setup_hash {
        return Err(SetupPreflightError::Proof(
            ProofPreflightError::SetupHashMismatch,
        ));
    }

    let catalog_hash =
        key_directory_catalog_digest(catalog).map_err(SetupPreflightError::Catalog)?;
    if proof.setup_hash != catalog_hash {
        return Err(SetupPreflightError::CatalogHashMismatch);
    }

    let ProofPreflightReport {
        segment_count,
        public_value_count,
        public_values_hash,
        public_value_field_count,
        program_image_cache_count,
        program_image_caches,
        program_image_cache_hashes,
        challenge_values_segment_count,
        challenge_values_segment_byte_counts,
        challenge_values_value_counts,
        trace_constraint_segment_count,
        trace_constraint_segment_byte_counts,
        trace_constraint_units,
        eth_block_input_count,
        eth_block_input_hashes,
        eth_block_input_byte_counts,
        eth_block_input_block_rlp_byte_counts,
        eth_block_input_extra_header_field_counts,
        eth_block_input_extra_body_field_counts,
        eth_block_input_block_hashes,
        eth_block_input_parent_hashes,
        eth_block_input_ommers_hashes,
        eth_block_input_beneficiaries,
        eth_block_input_state_roots,
        eth_block_input_receipt_roots,
        eth_block_input_logs_blooms,
        eth_block_input_difficulties,
        eth_block_input_block_numbers,
        eth_block_input_timestamps,
        eth_block_input_extra_data,
        eth_block_input_gas_limits,
        eth_block_input_gas_used_values,
        eth_block_input_base_fees_per_gas,
        eth_block_input_mix_hashes,
        eth_block_input_nonces,
        eth_block_input_transaction_roots,
        eth_block_input_transaction_preimage_counts,
        eth_block_input_legacy_transaction_counts,
        eth_block_input_typed_transaction_counts,
        eth_block_input_receipts_rlp_byte_counts,
        eth_block_input_receipt_preimage_counts,
        eth_block_input_legacy_receipt_counts,
        eth_block_input_typed_receipt_counts,
        eth_block_input_withdrawal_roots,
        eth_block_input_withdrawal_counts,
        eth_block_input_withdrawal_preimage_counts,
    } = validate_proof_public_values_for_setup_preflight(proof, public_values)
        .map_err(SetupPreflightError::Proof)?;

    validate_public_values_metadata_with_field_count(
        &catalog.layout.global_info,
        public_values,
        public_value_field_count,
    )?;

    if program_image_caches
        .iter()
        .any(|cache| cache.constraint_system_digest != catalog_hash)
    {
        return Err(SetupPreflightError::ProgramImageCacheSetupHashMismatch);
    }

    Ok(SetupPreflightReport {
        unit_count: catalog.units.len(),
        segment_count,
        public_value_count,
        public_values_hash,
        public_value_field_count,
        source_fixed_file_manifest_present: catalog.source_fixed_file_manifest.is_some(),
        source_fixed_file_manifest_entry_count: catalog
            .source_fixed_file_manifest
            .as_ref()
            .map(|manifest| manifest.entries.len())
            .unwrap_or(0),
        source_program_archive_present: catalog.source_program_archive.is_some(),
        source_program_archive_source_count: catalog
            .source_program_archive
            .as_ref()
            .map(|archive| archive.sources.len())
            .unwrap_or(0),
        source_program_archive_edge_count: catalog
            .source_program_archive
            .as_ref()
            .map(|archive| archive.edges.len())
            .unwrap_or(0),
        program_image_cache_count,
        program_image_caches,
        program_image_cache_hashes,
        challenge_values_segment_count,
        challenge_values_segment_byte_counts,
        challenge_values_value_counts,
        trace_constraint_segment_count,
        trace_constraint_segment_byte_counts,
        trace_constraint_units,
        eth_block_input_count,
        eth_block_input_hashes,
        eth_block_input_byte_counts,
        eth_block_input_block_rlp_byte_counts,
        eth_block_input_extra_header_field_counts,
        eth_block_input_extra_body_field_counts,
        eth_block_input_block_hashes,
        eth_block_input_parent_hashes,
        eth_block_input_ommers_hashes,
        eth_block_input_beneficiaries,
        eth_block_input_state_roots,
        eth_block_input_receipt_roots,
        eth_block_input_logs_blooms,
        eth_block_input_difficulties,
        eth_block_input_block_numbers,
        eth_block_input_timestamps,
        eth_block_input_extra_data,
        eth_block_input_gas_limits,
        eth_block_input_gas_used_values,
        eth_block_input_base_fees_per_gas,
        eth_block_input_mix_hashes,
        eth_block_input_nonces,
        eth_block_input_transaction_roots,
        eth_block_input_transaction_preimage_counts,
        eth_block_input_legacy_transaction_counts,
        eth_block_input_typed_transaction_counts,
        eth_block_input_receipts_rlp_byte_counts,
        eth_block_input_receipt_preimage_counts,
        eth_block_input_legacy_receipt_counts,
        eth_block_input_typed_receipt_counts,
        eth_block_input_withdrawal_roots,
        eth_block_input_withdrawal_counts,
        eth_block_input_withdrawal_preimage_counts,
    })
}

pub fn validate_public_values_metadata(
    global_info: &GlobalInfo,
    public_values: &PublicValues,
) -> Result<(), SetupPreflightError> {
    let public_value_field_count =
        public_values
            .values
            .iter()
            .try_fold(0_usize, |count, entry| {
                count
                    .checked_add(entry.elements.len())
                    .ok_or(SetupPreflightError::PublicValueCountOverflow)
            })?;
    validate_public_values_metadata_with_field_count(
        global_info,
        public_values,
        public_value_field_count,
    )
}

fn validate_public_values_metadata_with_field_count(
    global_info: &GlobalInfo,
    public_values: &PublicValues,
    public_value_field_count: usize,
) -> Result<(), SetupPreflightError> {
    if public_values.values.len() != global_info.publics_map.len() {
        return Err(SetupPreflightError::PublicValueEntryCountMismatch {
            expected: global_info.publics_map.len(),
            found: public_values.values.len(),
        });
    }

    let mut expected_field_count = 0_usize;
    for (index, (expected, found)) in global_info
        .publics_map
        .iter()
        .zip(&public_values.values)
        .enumerate()
    {
        if expected.name != found.name {
            return Err(SetupPreflightError::PublicValueNameMismatch {
                index,
                expected: expected.name.clone(),
                found: found.name.clone(),
            });
        }
        let expected_elements = public_value_dimension(expected)?;
        if found.elements.len() != expected_elements {
            return Err(SetupPreflightError::PublicValueElementCountMismatch {
                name: expected.name.clone(),
                expected: expected_elements,
                found: found.elements.len(),
            });
        }
        expected_field_count = expected_field_count
            .checked_add(expected_elements)
            .ok_or(SetupPreflightError::PublicValueCountOverflow)?;
    }

    let declared_field_count = usize::try_from(global_info.n_publics)
        .map_err(|_| SetupPreflightError::PublicValueCountOverflow)?;
    if expected_field_count != declared_field_count {
        return Err(SetupPreflightError::PublicValueFieldCountMismatch {
            expected: declared_field_count,
            found: expected_field_count,
        });
    }
    if public_value_field_count != declared_field_count {
        return Err(SetupPreflightError::PublicValueFieldCountMismatch {
            expected: declared_field_count,
            found: public_value_field_count,
        });
    }

    Ok(())
}

fn public_value_dimension(value: &PublicValue) -> Result<usize, SetupPreflightError> {
    value.lengths.iter().try_fold(1_usize, |dimension, length| {
        let length =
            usize::try_from(*length).map_err(|_| SetupPreflightError::PublicValueCountOverflow)?;
        dimension
            .checked_mul(length)
            .ok_or(SetupPreflightError::PublicValueCountOverflow)
    })
}

pub fn validate_setup_preflight(
    catalog: &KeyDirectoryCatalog,
    proof: &ProofArtifact,
    public_values: &PublicValues,
) -> Result<SetupPreflightReport, SetupPreflightError> {
    let report = validate_setup_preflight_hashes(catalog, proof, public_values)?;
    let schedule = derive_prove_schedule(catalog).map_err(SetupPreflightError::Schedule)?;
    validate_setup_proof_segment_ids(&proof.segments)?;
    validate_optional_contribution_segment(catalog, proof)?;
    validate_optional_challenge_values_segment(proof)?;
    let contribution_proof_values =
        validate_optional_contribution_challenge_values(catalog, proof, public_values)?;
    let global_values = validate_global_value_segments(catalog, proof, contribution_proof_values)?;
    let uses_transcript_inputs = uses_transcript_pcs_query_plan_inputs(&proof.segments);
    let needs_public_fields = uses_transcript_inputs
        || !catalog.global_constraints.entries.is_empty()
        || !catalog.global_hints.hints.is_empty();
    let public_fields = if needs_public_fields {
        Some(public_values_as_fields(public_values).map_err(SetupPreflightError::PublicValues)?)
    } else {
        None
    };
    let transcript_public_fields = if uses_transcript_inputs {
        public_fields.as_deref().unwrap_or(&[])
    } else {
        &[]
    };

    validate_pcs_material_manifest_segments(&schedule, &proof.segments)
        .map_err(SetupPreflightError::PcsMaterial)?;
    let witness_segments = load_witness_commitment_segment_refs(&schedule.units, &proof.segments)
        .map_err(SetupPreflightError::WitnessCommitment)?;
    validate_optional_trace_constraint_segment(catalog, &schedule, proof, &witness_segments)?;
    validate_pcs_query_plan_segments(
        &schedule,
        proof.public_values_hash,
        transcript_public_fields,
        &proof.segments,
    )
    .map_err(SetupPreflightError::PcsQueryPlan)?;
    validate_optional_unit_value_segments(&schedule, proof)?;
    validate_constant_opening_segments(&schedule.units, &proof.segments)
        .map_err(SetupPreflightError::ConstantOpening)?;
    validate_witness_opening_segments(&schedule.units, &proof.segments)
        .map_err(SetupPreflightError::WitnessOpening)?;

    let mut transcript_unit_challenges = None;
    let mut transcript_challenges = None;
    if !catalog.global_constraints.entries.is_empty() {
        let public_values = public_fields.as_deref().unwrap_or(&[]);
        let packed_proof_values =
            flatten_pcs_proof_values(&catalog.layout.global_info, &global_values.proof_values)
                .map_err(ValidateGlobalConstraintProofSegmentsError::PackedProofValues)
                .map_err(SetupPreflightError::GlobalConstraints)?;
        let challenges = if uses_transcript_inputs {
            cached_setup_preflight_transcript_challenges(
                &mut transcript_unit_challenges,
                &mut transcript_challenges,
                &schedule,
                public_values,
                &proof.segments,
            )
            .map_err(ValidateGlobalConstraintProofSegmentsError::Transcript)
            .map_err(SetupPreflightError::GlobalConstraints)?
        } else {
            &[]
        };
        validate_global_constraints(
            &catalog.global_constraints,
            GlobalConstraintInputs {
                publics: public_values,
                proof_values: &packed_proof_values,
                challenges,
                group_values: &global_values.group_values,
            },
        )
        .map_err(ValidateGlobalConstraintProofSegmentsError::Validation)
        .map_err(SetupPreflightError::GlobalConstraints)?;
    }

    if !catalog.global_hints.hints.is_empty() {
        let full_public_values = public_fields.as_deref().unwrap_or(&[]);
        let requirements = global_hint_input_requirements(&catalog.global_hints);
        let public_values = if requirements.publics {
            full_public_values
        } else {
            &[]
        };
        let packed_proof_values = if requirements.proof_values {
            flatten_pcs_proof_values(&catalog.layout.global_info, &global_values.proof_values)
                .map_err(ResolveGlobalHintProofSegmentsError::PackedProofValues)
                .map_err(SetupPreflightError::GlobalHints)?
        } else {
            Vec::new()
        };
        let challenges = if uses_transcript_inputs && requirements.challenges {
            cached_setup_preflight_transcript_challenges(
                &mut transcript_unit_challenges,
                &mut transcript_challenges,
                &schedule,
                full_public_values,
                &proof.segments,
            )
            .map_err(ResolveGlobalHintProofSegmentsError::Transcript)
            .map_err(SetupPreflightError::GlobalHints)?
        } else {
            &[]
        };
        let group_values = if requirements.group_values {
            global_values.group_values.as_slice()
        } else {
            &[]
        };
        let resolved = resolve_global_hint_program(
            &catalog.layout.global_info,
            &catalog.global_hints,
            GlobalConstraintInputs {
                publics: public_values,
                proof_values: &packed_proof_values,
                challenges,
                group_values,
            },
        )
        .map_err(ResolveGlobalHintProofSegmentsError::Eval)
        .map_err(SetupPreflightError::GlobalHints)?;
        validate_global_source_lookup_hints(&resolved)?;
    }

    let verifier_codes = catalog
        .units
        .iter()
        .map(|unit| &unit.metadata.verifier.query)
        .collect::<Vec<_>>();
    let fri_opening_required_units = catalog
        .units
        .iter()
        .map(|unit| unit.metadata.verifier.quotient.expression_id.is_some())
        .collect::<Vec<_>>();
    let fri_request = ValidateOptionalPcsFriOpeningProofSegmentsRequest {
        schedule: &schedule,
        verifier_codes: &verifier_codes,
        fri_opening_required_units: &fri_opening_required_units,
        global_info: &catalog.layout.global_info,
        public_values: transcript_public_fields,
        segments: &proof.segments,
    };
    if transcript_unit_challenges.is_some() {
        let unit_challenges = cached_setup_preflight_transcript_unit_challenges(
            &mut transcript_unit_challenges,
            &schedule,
            transcript_public_fields,
            &proof.segments,
        )
        .map_err(ValidateOptionalPcsFriOpeningProofSegmentsError::Transcript)
        .map_err(SetupPreflightError::PcsFri)?;
        validate_optional_pcs_fri_opening_proof_segments_with_transcript_challenges(
            fri_request,
            unit_challenges,
        )
        .map_err(SetupPreflightError::PcsFri)?;
    } else {
        validate_optional_pcs_fri_opening_proof_segments(fri_request)
            .map_err(SetupPreflightError::PcsFri)?;
    }

    Ok(report)
}

fn cached_setup_preflight_transcript_unit_challenges<'a>(
    cache: &'a mut Option<Vec<PcsTranscriptUnitChallenges>>,
    schedule: &crate::ProveSchedule,
    public_values: &[Felt],
    segments: &[ProofSegment],
) -> Result<&'a [PcsTranscriptUnitChallenges], PcsTranscriptProofSegmentsError> {
    if cache.is_none() {
        *cache = Some(derive_pcs_transcript_unit_challenges_from_proof_segments(
            schedule,
            public_values,
            segments,
        )?);
    }
    Ok(cache.as_deref().unwrap_or(&[]))
}

fn cached_setup_preflight_transcript_challenges<'a>(
    unit_cache: &mut Option<Vec<PcsTranscriptUnitChallenges>>,
    challenge_cache: &'a mut Option<Vec<Ext3>>,
    schedule: &crate::ProveSchedule,
    public_values: &[Felt],
    segments: &[ProofSegment],
) -> Result<&'a [Ext3], PcsTranscriptProofSegmentsError> {
    if challenge_cache.is_none() {
        let unit_challenges = cached_setup_preflight_transcript_unit_challenges(
            unit_cache,
            schedule,
            public_values,
            segments,
        )?;
        *challenge_cache = Some(
            unit_challenges
                .iter()
                .flat_map(|unit| unit.challenges.iter().copied())
                .collect(),
        );
    }
    Ok(challenge_cache.as_deref().unwrap_or(&[]))
}

fn validate_global_source_lookup_hints(hints: &[ResolvedHint]) -> Result<(), SetupPreflightError> {
    let mut balance = SourceLookupBalance::default();
    balance
        .absorb(0, 0, hints)
        .map_err(source_lookup_setup_preflight_error)?;
    balance
        .validate_all_units()
        .map_err(source_lookup_setup_preflight_error)
}

fn source_lookup_setup_preflight_error(error: SourceLookupHintError) -> SetupPreflightError {
    SetupPreflightError::SourceLookup {
        message: error.to_string(),
    }
}

fn validate_optional_unit_value_segments(
    schedule: &crate::ProveSchedule,
    proof: &ProofArtifact,
) -> Result<(), SetupPreflightError> {
    if !proof
        .segments
        .iter()
        .any(|segment| segment.id == UNIT_VALUES_SEGMENT_ID)
    {
        return Ok(());
    }

    let query_plan = load_pcs_query_plan_from_segments(&proof.segments)
        .map_err(SetupPreflightError::UnitValueQueryPlan)?;
    let Some(unit_values) = load_unit_values_segment_from_segments(&proof.segments)
        .map_err(SetupPreflightError::UnitValues)?
    else {
        return Ok(());
    };

    validate_unit_values_units_match_query_units_from_segment(
        &query_plan.units,
        Some(&unit_values),
    )
    .map_err(SetupPreflightError::UnitValues)?;
    for unit_value in &unit_values.units {
        let unit_index = usize::try_from(unit_value.unit_index).map_err(|_| {
            SetupPreflightError::UnitValues(LoadUnitValuesSegmentError::UnitIndexOverflow {
                unit_index: usize::MAX,
            })
        })?;
        let unit = schedule
            .units
            .get(unit_index)
            .ok_or(SetupPreflightError::UnitValues(
                LoadUnitValuesSegmentError::MissingUnit { unit_index },
            ))?;
        load_unit_values_for_identity_from_parsed_segment(
            unit_index,
            unit_value.trace_instance_index,
            &unit.unit_value_map,
            Some(&unit_values),
        )
        .map(|_| ())
        .map_err(SetupPreflightError::UnitValues)?;
    }
    Ok(())
}

fn validate_global_value_segments(
    catalog: &KeyDirectoryCatalog,
    proof: &ProofArtifact,
    preloaded_proof_values: Option<Vec<Ext3>>,
) -> Result<SetupPreflightGlobalValues, SetupPreflightError> {
    let proof_values = match preloaded_proof_values {
        Some(proof_values) => proof_values,
        None => load_pcs_proof_values_from_segments(&catalog.layout.global_info, &proof.segments)
            .map_err(SetupPreflightError::ProofValues)?,
    };
    let group_values =
        load_group_values_from_segments(&catalog.layout.global_info, &proof.segments)
            .map_err(SetupPreflightError::GroupValues)?;

    Ok(SetupPreflightGlobalValues {
        proof_values,
        group_values,
    })
}

fn validate_optional_contribution_segment(
    catalog: &KeyDirectoryCatalog,
    proof: &ProofArtifact,
) -> Result<(), SetupPreflightError> {
    if !proof
        .segments
        .iter()
        .any(|segment| segment.id == CONTRIBUTION_SEGMENT_ID)
    {
        return Ok(());
    }

    let entries = load_contribution_segment_from_segments(&proof.segments)
        .map_err(ContributionChallengeError::from)
        .map_err(SetupPreflightError::Contribution)?;
    aggregate_contribution_values(&catalog.layout.global_info, &entries)
        .map(|_| ())
        .map_err(SetupPreflightError::Contribution)
}

fn validate_optional_challenge_values_segment(
    proof: &ProofArtifact,
) -> Result<(), SetupPreflightError> {
    let mut segments = proof
        .segments
        .iter()
        .filter(|segment| segment.id == CHALLENGE_VALUES_SEGMENT_ID);
    let Some(segment) = segments.next() else {
        return Ok(());
    };
    if segments.next().is_some() {
        return Err(SetupPreflightError::DuplicateChallengeValuesSegment);
    }
    let segment = parse_challenge_values_segment(&segment.data)
        .map_err(SetupPreflightError::ChallengeValues)?;
    validate_challenge_values_canonical(&segment.values)?;
    Ok(())
}

fn validate_challenge_values_canonical(values: &[[u64; 3]]) -> Result<(), SetupPreflightError> {
    for (value_index, words) in values.iter().enumerate() {
        for (word_index, word) in words.iter().copied().enumerate() {
            Felt::from_canonical(word).map_err(|source| {
                SetupPreflightError::ChallengeValueNonCanonical {
                    value_index,
                    word_index,
                    source,
                }
            })?;
        }
    }
    Ok(())
}

fn validate_optional_contribution_challenge_values(
    catalog: &KeyDirectoryCatalog,
    proof: &ProofArtifact,
    public_values: &PublicValues,
) -> Result<Option<Vec<Ext3>>, SetupPreflightError> {
    let has_contribution = proof
        .segments
        .iter()
        .any(|segment| segment.id == CONTRIBUTION_SEGMENT_ID);
    let Some(challenge_segment) = proof
        .segments
        .iter()
        .find(|segment| segment.id == CHALLENGE_VALUES_SEGMENT_ID)
    else {
        if has_contribution {
            return Err(SetupPreflightError::MissingContributionChallengeValues);
        }
        return Ok(None);
    };
    if !has_contribution {
        return Ok(None);
    }

    let challenge_values = parse_challenge_values_segment(&challenge_segment.data)
        .map_err(SetupPreflightError::ChallengeValues)?
        .values;
    let public_fields =
        public_values_as_fields(public_values).map_err(SetupPreflightError::PublicValues)?;
    let proof_values =
        load_pcs_proof_values_from_segments(&catalog.layout.global_info, &proof.segments)
            .map_err(SetupPreflightError::ProofValues)?;
    let packed_proof_values = flatten_pcs_proof_values(&catalog.layout.global_info, &proof_values)
        .map_err(SetupPreflightError::ProofValuePacking)?;
    let expected = derive_global_challenge_from_proof_segments(
        &catalog.layout.global_info,
        &public_fields,
        &packed_proof_values,
        &proof.segments,
    )
    .map_err(SetupPreflightError::Contribution)?;
    if challenge_values.as_slice() != [expected.to_u64s()] {
        return Err(SetupPreflightError::ContributionChallengeValuesMismatch);
    }
    Ok(Some(proof_values))
}

fn validate_setup_proof_segment_ids(segments: &[ProofSegment]) -> Result<(), SetupPreflightError> {
    for segment in segments {
        if is_setup_proof_segment_id(segment.id) {
            continue;
        }
        return Err(SetupPreflightError::UnexpectedProofSegment { id: segment.id });
    }
    Ok(())
}

fn validate_optional_trace_constraint_segment(
    catalog: &KeyDirectoryCatalog,
    schedule: &crate::ProveSchedule,
    proof: &ProofArtifact,
    witness_segments: &[&ProofSegment],
) -> Result<(), SetupPreflightError> {
    let Some(segment) = proof
        .segments
        .iter()
        .find(|segment| segment.id == TRACE_CONSTRAINT_SEGMENT_ID)
    else {
        if witness_segments.is_empty() {
            return Ok(());
        }
        return Err(SetupPreflightError::TraceConstraintBinding {
            message: "missing trace constraint evidence segment".to_owned(),
        });
    };
    let evidence = parse_trace_constraint_segment(&segment.data)
        .map_err(SetupPreflightError::TraceConstraint)?;

    let unit_count = u32::try_from(schedule.units.len()).map_err(|_| {
        SetupPreflightError::TraceConstraintBinding {
            message: "trace constraint evidence unit count overflow".to_owned(),
        }
    })?;
    let mut witness_shapes = BTreeMap::new();
    for witness_segment in witness_segments {
        let identity = witness_commitment_segment_identity(unit_count, witness_segment.id)
            .map_err(|error| SetupPreflightError::TraceConstraintBinding {
                message: format!("trace constraint evidence witness identity failed: {error}"),
            })?
            .ok_or_else(|| SetupPreflightError::TraceConstraintBinding {
                message: format!(
                    "trace constraint evidence missing witness identity for segment {}",
                    witness_segment.id
                ),
            })?;
        let parsed = parse_witness_commitment_segment(&witness_segment.data).map_err(|error| {
            SetupPreflightError::TraceConstraintBinding {
                message: format!("trace constraint evidence witness segment parse failed: {error}"),
            }
        })?;
        witness_shapes.insert(
            (identity.unit_index, identity.trace_instance_index),
            (parsed.trace_rows, parsed.trace_columns),
        );
    }

    let mut evidence_units = BTreeMap::new();
    for unit in evidence.units {
        evidence_units.insert(
            (unit.unit_index, unit.trace_instance_index),
            (
                (unit.trace_row_count, u64::from(unit.trace_column_count)),
                unit.regular_constraint_count,
            ),
        );
    }
    for (&identity, witness_shape) in &witness_shapes {
        let Some((evidence_shape, evidence_constraint_count)) = evidence_units.get(&identity)
        else {
            return Err(SetupPreflightError::TraceConstraintBinding {
                message: format!(
                    "trace constraint evidence missing witness identity: unit {}, trace instance {}",
                    identity.0, identity.1
                ),
            });
        };
        if *evidence_shape != *witness_shape {
            return Err(SetupPreflightError::TraceConstraintBinding {
                message: format!(
                    "trace constraint evidence shape mismatch for unit {}, trace instance {}",
                    identity.0, identity.1
                ),
            });
        }
        let unit_index = usize::try_from(identity.0).map_err(|_| {
            SetupPreflightError::TraceConstraintBinding {
                message: format!(
                    "trace constraint evidence unit index overflow: unit {}, trace instance {}",
                    identity.0, identity.1
                ),
            }
        })?;
        let Some(unit) = catalog.units.get(unit_index) else {
            return Err(SetupPreflightError::TraceConstraintBinding {
                message: format!(
                    "trace constraint evidence unknown unit: unit {}, trace instance {}",
                    identity.0, identity.1
                ),
            });
        };
        let expected_constraint_count =
            u32::try_from(unit.regular_constraints.entries.len()).map_err(|_| {
                SetupPreflightError::TraceConstraintBinding {
                    message: format!(
                        "trace constraint evidence constraint count overflow for unit {}, trace instance {}",
                        identity.0, identity.1
                    ),
                }
            })?;
        if *evidence_constraint_count != expected_constraint_count {
            return Err(SetupPreflightError::TraceConstraintBinding {
                message: format!(
                    "trace constraint evidence constraint count mismatch for unit {}, trace instance {}",
                    identity.0, identity.1
                ),
            });
        }
    }
    for identity in evidence_units.keys() {
        if !witness_shapes.contains_key(identity) {
            return Err(SetupPreflightError::TraceConstraintBinding {
                message: format!(
                    "trace constraint evidence unexpected witness identity: unit {}, trace instance {}",
                    identity.0, identity.1
                ),
            });
        }
    }
    Ok(())
}

pub(crate) fn is_setup_proof_segment_id(id: u32) -> bool {
    if (WITNESS_COMMITMENT_SEGMENT_BASE_ID..PCS_MATERIAL_MANIFEST_SEGMENT_ID).contains(&id) {
        return true;
    }

    matches!(
        id,
        PCS_MATERIAL_MANIFEST_SEGMENT_ID
            | PCS_QUERY_PLAN_SEGMENT_ID
            | WITNESS_OPENING_SEGMENT_ID
            | CONSTANT_OPENING_SEGMENT_ID
            | PCS_FRI_OPENING_SEGMENT_ID
            | PCS_QUERY_NONCE_SEGMENT_ID
            | PCS_EVALUATION_SEGMENT_ID
            | PCS_PROOF_VALUES_SEGMENT_ID
            | GROUP_VALUES_SEGMENT_ID
            | CHALLENGE_VALUES_SEGMENT_ID
            | UNIT_VALUES_SEGMENT_ID
            | TRACE_CONSTRAINT_SEGMENT_ID
            | PROGRAM_IMAGE_CACHE_SEGMENT_ID
            | CONTRIBUTION_SEGMENT_ID
            | ETH_BLOCK_INPUT_SEGMENT_ID
    )
}

pub fn validate_setup_preflight_from_files(
    setup_dir: impl AsRef<Path>,
    proof_path: impl AsRef<Path>,
    public_values_path: impl AsRef<Path>,
) -> Result<SetupPreflightReport, SetupPreflightFileError> {
    let setup_dir = setup_dir.as_ref();
    let catalog = read_key_directory_catalog(setup_dir)?;
    validate_setup_directory_manifest_if_present(setup_dir, &catalog)
        .map_err(SetupPreflightError::SetupDirectoryManifest)?;
    let proof = read_proof_artifact_file(proof_path)?;
    let public_values = read_public_values_file(public_values_path)?;
    validate_setup_preflight(&catalog, &proof, &public_values).map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use lzvm_artifacts::challenge_values_segment::{
        encode_challenge_values_segment, ChallengeValuesSegment, ChallengeValuesSegmentError,
        CHALLENGE_VALUES_SEGMENT_ID,
    };
    use lzvm_artifacts::pcs_material_segment::PCS_MATERIAL_MANIFEST_SEGMENT_ID;
    use lzvm_artifacts::proof::{ProofArtifact, ProofSegment};
    use lzvm_artifacts::witness_segment::WITNESS_COMMITMENT_SEGMENT_BASE_ID;
    use lzvm_field::{FieldError, MODULUS};

    use super::{
        validate_optional_challenge_values_segment, validate_setup_proof_segment_ids,
        SetupPreflightError,
    };

    const FIRST_CHALLENGE_VALUE_OFFSET: usize = 12;

    #[test]
    fn setup_proof_segment_id_check_accepts_challenge_values_segment() {
        let segments = vec![
            ProofSegment {
                id: WITNESS_COMMITMENT_SEGMENT_BASE_ID,
                data: vec![1],
            },
            ProofSegment {
                id: PCS_MATERIAL_MANIFEST_SEGMENT_ID,
                data: vec![1],
            },
            ProofSegment {
                id: CHALLENGE_VALUES_SEGMENT_ID,
                data: vec![1],
            },
        ];

        validate_setup_proof_segment_ids(&segments).expect("setup proof segments should validate");
    }

    #[test]
    fn setup_proof_segment_id_check_rejects_unknown_fixed_segments() {
        let unknown_fixed_segment_id = 10_099;
        let segments = vec![ProofSegment {
            id: unknown_fixed_segment_id,
            data: vec![1],
        }];

        let error = validate_setup_proof_segment_ids(&segments)
            .expect_err("unknown setup proof segment should reject");

        assert_eq!(
            error,
            SetupPreflightError::UnexpectedProofSegment {
                id: unknown_fixed_segment_id
            }
        );
    }

    #[test]
    fn challenge_values_preflight_accepts_encoded_segment() {
        let proof = ProofArtifact {
            setup_hash: [0; 32],
            public_values_hash: [0; 32],
            segments: vec![ProofSegment {
                id: CHALLENGE_VALUES_SEGMENT_ID,
                data: encode_challenge_values_segment(&ChallengeValuesSegment {
                    values: vec![[1, 2, 3]],
                })
                .expect("challenge values segment should encode"),
            }],
        };

        validate_optional_challenge_values_segment(&proof)
            .expect("challenge values segment should validate");
    }

    #[test]
    fn challenge_values_preflight_rejects_non_canonical_values() {
        let mut data = encode_challenge_values_segment(&ChallengeValuesSegment {
            values: vec![[1, 2, 3]],
        })
        .expect("challenge values segment should encode");
        data[FIRST_CHALLENGE_VALUE_OFFSET + 8..FIRST_CHALLENGE_VALUE_OFFSET + 16]
            .copy_from_slice(&MODULUS.to_le_bytes());
        let proof = ProofArtifact {
            setup_hash: [0; 32],
            public_values_hash: [0; 32],
            segments: vec![ProofSegment {
                id: CHALLENGE_VALUES_SEGMENT_ID,
                data,
            }],
        };

        let error = validate_optional_challenge_values_segment(&proof)
            .expect_err("non-canonical challenge values should reject");

        assert_eq!(
            error,
            SetupPreflightError::ChallengeValues(ChallengeValuesSegmentError::ValueNonCanonical {
                value_index: 0,
                word_index: 1,
                source: FieldError::NonCanonical { value: MODULUS },
            })
        );
    }

    #[test]
    fn challenge_values_preflight_rejects_duplicate_segments() {
        let data = encode_challenge_values_segment(&ChallengeValuesSegment {
            values: vec![[1, 2, 3]],
        })
        .expect("challenge values segment should encode");
        let proof = ProofArtifact {
            setup_hash: [0; 32],
            public_values_hash: [0; 32],
            segments: vec![
                ProofSegment {
                    id: CHALLENGE_VALUES_SEGMENT_ID,
                    data: data.clone(),
                },
                ProofSegment {
                    id: CHALLENGE_VALUES_SEGMENT_ID,
                    data,
                },
            ],
        };

        let error = validate_optional_challenge_values_segment(&proof)
            .expect_err("duplicate challenge values segments should reject");

        assert_eq!(error, SetupPreflightError::DuplicateChallengeValuesSegment);
    }

    #[test]
    fn challenge_values_preflight_rejects_invalid_segment() {
        let proof = ProofArtifact {
            setup_hash: [0; 32],
            public_values_hash: [0; 32],
            segments: vec![ProofSegment {
                id: CHALLENGE_VALUES_SEGMENT_ID,
                data: vec![0, 1, 2, 3],
            }],
        };

        let error = validate_optional_challenge_values_segment(&proof)
            .expect_err("invalid challenge values segment should reject");

        assert_eq!(
            error,
            SetupPreflightError::ChallengeValues(ChallengeValuesSegmentError::InvalidMagic)
        );
    }
}
