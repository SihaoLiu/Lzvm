use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

use lzvm_artifacts::challenge_values_segment::{
    parse_challenge_values_segment, ChallengeValuesSegmentError, CHALLENGE_VALUES_SEGMENT_ID,
};
use lzvm_artifacts::contribution_segment::CONTRIBUTION_SEGMENT_ID;
use lzvm_artifacts::global_info::{GlobalInfo, PublicValue};
use lzvm_artifacts::key_directory::{
    key_directory_catalog_digest, read_key_directory_catalog, KeyDirectoryCatalog,
    KeyDirectoryError,
};
use lzvm_artifacts::program_image::ProgramImageCommitmentCache;
use lzvm_artifacts::proof::{
    read_proof_artifact_file, ProofArtifact, ProofArtifactError, ProofSegment,
};
use lzvm_artifacts::public_values::{read_public_values_file, PublicValues, PublicValuesError};
use lzvm_artifacts::setup_manifest::{
    build_setup_directory_manifest, validate_setup_directory_manifest_file,
    SetupDirectoryManifestError, SETUP_DIRECTORY_MANIFEST_FILE,
};
use lzvm_artifacts::trace_constraint_segment::TraceConstraintSegmentError;
use lzvm_artifacts::unit_values_segment::UNIT_VALUES_SEGMENT_ID;
use lzvm_field::{Ext3, Felt};

use crate::constant_opening::{
    validate_constant_opening_segments, ValidateConstantOpeningSegmentsError,
};
use crate::contribution::{
    aggregate_contribution_values, derive_global_challenge_from_loaded_contributions,
    load_contribution_segment_from_segments, ContributionChallengeError, ProveContributionEntry,
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
    validate_optional_pcs_fri_opening_proof_segments_with_preflight_values,
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
    derive_pcs_transcript_unit_challenges_from_loaded_witness_segments,
    PcsTranscriptProofSegmentsError, PcsTranscriptUnitChallenges,
};
use crate::proof_preflight::{
    public_values_as_fields, validate_proof_public_values_for_setup_preflight_with_fields,
    ProofPreflightError, ProofPreflightReport, PublicValueFieldError, TraceConstraintPreflightUnit,
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
    load_witness_commitment_segment_refs_with_shapes, LoadWitnessCommitmentSegmentsError,
    LoadedWitnessCommitmentSegmentRef,
};
use crate::witness_opening::{
    validate_witness_opening_segments, ValidateWitnessOpeningSegmentsError,
};
use crate::{derive_prove_schedule, ProveSchedule, ProveScheduleError};

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
    pub framed_guest_input_count: usize,
    pub framed_guest_input_hashes: Vec<[u8; 32]>,
    pub framed_guest_input_byte_counts: Vec<usize>,
    pub framed_guest_input_chunk_counts: Vec<usize>,
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

struct SetupPreflightValidation {
    report: SetupPreflightReport,
    public_value_fields: Vec<Felt>,
}

struct SetupPreflightGlobalValues {
    proof_values: Vec<Ext3>,
    packed_proof_values: Option<Vec<Felt>>,
    group_values: Vec<Ext3>,
}

struct SetupPreflightContributionProofValues {
    proof_values: Vec<Ext3>,
    packed_proof_values: Vec<Felt>,
}

struct SetupPreflightTranscriptChallengeCache<'a> {
    schedule: &'a ProveSchedule,
    public_values: &'a [Felt],
    segments: &'a [ProofSegment],
    witness_segments: &'a [LoadedWitnessCommitmentSegmentRef<'a>],
    unit_challenges: Option<Vec<PcsTranscriptUnitChallenges>>,
    flat_challenges: Option<Vec<Ext3>>,
}

impl<'a> SetupPreflightTranscriptChallengeCache<'a> {
    fn new(
        schedule: &'a ProveSchedule,
        public_values: &'a [Felt],
        segments: &'a [ProofSegment],
        witness_segments: &'a [LoadedWitnessCommitmentSegmentRef<'a>],
    ) -> Self {
        Self {
            schedule,
            public_values,
            segments,
            witness_segments,
            unit_challenges: None,
            flat_challenges: None,
        }
    }

    fn has_unit_challenges(&self) -> bool {
        self.unit_challenges.is_some()
    }

    fn unit_challenges(
        &mut self,
    ) -> Result<&[PcsTranscriptUnitChallenges], PcsTranscriptProofSegmentsError> {
        if self.unit_challenges.is_none() {
            self.unit_challenges = Some(
                derive_pcs_transcript_unit_challenges_from_loaded_witness_segments(
                    self.schedule,
                    self.public_values,
                    self.segments,
                    self.witness_segments,
                )?,
            );
        }
        Ok(self.unit_challenges.as_deref().unwrap_or(&[]))
    }

    fn flat_challenges(&mut self) -> Result<&[Ext3], PcsTranscriptProofSegmentsError> {
        if self.flat_challenges.is_none() {
            self.flat_challenges = Some(
                self.unit_challenges()?
                    .iter()
                    .flat_map(|unit| unit.challenges.iter().copied())
                    .collect(),
            );
        }
        Ok(self.flat_challenges.as_deref().unwrap_or(&[]))
    }
}

struct SetupPreflightPackedProofValueCache<'a> {
    global_info: &'a GlobalInfo,
    proof_values: &'a [Ext3],
    packed_proof_values: Option<Vec<Felt>>,
}

impl<'a> SetupPreflightPackedProofValueCache<'a> {
    fn new(
        global_info: &'a GlobalInfo,
        proof_values: &'a [Ext3],
        preloaded_packed_proof_values: Option<Vec<Felt>>,
    ) -> Self {
        Self {
            global_info,
            proof_values,
            packed_proof_values: preloaded_packed_proof_values,
        }
    }

    fn packed_values(&mut self) -> Result<&[Felt], ProvePcsProofValuesSegmentError> {
        if self.packed_proof_values.is_none() {
            self.packed_proof_values = Some(flatten_pcs_proof_values(
                self.global_info,
                self.proof_values,
            )?);
        }
        Ok(self.packed_proof_values.as_deref().unwrap_or(&[]))
    }
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
    Ok(validate_setup_preflight_hashes_with_fields(catalog, proof, public_values)?.report)
}

fn validate_setup_preflight_hashes_with_fields(
    catalog: &KeyDirectoryCatalog,
    proof: &ProofArtifact,
    public_values: &PublicValues,
) -> Result<SetupPreflightValidation, SetupPreflightError> {
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

    let proof_validation =
        validate_proof_public_values_for_setup_preflight_with_fields(proof, public_values)
            .map_err(SetupPreflightError::Proof)?;
    let public_value_fields = proof_validation.public_value_fields;
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
        framed_guest_input_count,
        framed_guest_input_hashes,
        framed_guest_input_byte_counts,
        framed_guest_input_chunk_counts,
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
    } = proof_validation.report;

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

    Ok(SetupPreflightValidation {
        report: SetupPreflightReport {
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
            framed_guest_input_count,
            framed_guest_input_hashes,
            framed_guest_input_byte_counts,
            framed_guest_input_chunk_counts,
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
        },
        public_value_fields,
    })
}

pub fn validate_public_values_metadata(
    global_info: &GlobalInfo,
    public_values: &PublicValues,
) -> Result<(), SetupPreflightError> {
    let public_value_field_count = public_values_as_fields(public_values)
        .map_err(SetupPreflightError::PublicValues)?
        .len();
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
        if length == 0 {
            return Err(SetupPreflightError::PublicValueCountOverflow);
        }
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
    let validation = validate_setup_preflight_hashes_with_fields(catalog, proof, public_values)?;
    let public_fields = validation.public_value_fields;
    let report = validation.report;
    let schedule = derive_prove_schedule(catalog).map_err(SetupPreflightError::Schedule)?;
    let contribution_entries = validate_optional_contribution_segment(catalog, proof)?;
    let challenge_values = validate_optional_challenge_values_segment(proof)?;
    let contribution_proof_values = validate_optional_contribution_challenge_values(
        catalog,
        proof,
        &public_fields,
        contribution_entries.as_deref(),
        challenge_values.as_deref(),
    )?;
    let global_values = validate_global_value_segments(catalog, proof, contribution_proof_values)?;
    let uses_transcript_inputs = uses_transcript_pcs_query_plan_inputs(&proof.segments);
    let transcript_public_fields = if uses_transcript_inputs {
        public_fields.as_slice()
    } else {
        &[]
    };

    validate_pcs_material_manifest_segments(&schedule, &proof.segments)
        .map_err(SetupPreflightError::PcsMaterial)?;
    let witness_segments =
        load_witness_commitment_segment_refs_with_shapes(&schedule.units, &proof.segments)
            .map_err(SetupPreflightError::WitnessCommitment)?;
    validate_optional_trace_constraint_segment(
        catalog,
        &report.trace_constraint_units,
        &witness_segments,
    )?;
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

    let mut transcript_challenges = SetupPreflightTranscriptChallengeCache::new(
        &schedule,
        transcript_public_fields,
        &proof.segments,
        &witness_segments,
    );
    let mut packed_proof_values_cache = SetupPreflightPackedProofValueCache::new(
        &catalog.layout.global_info,
        &global_values.proof_values,
        global_values.packed_proof_values,
    );
    if !catalog.global_constraints.entries.is_empty() {
        let public_values = public_fields.as_slice();
        let packed_proof_values = packed_proof_values_cache
            .packed_values()
            .map_err(ValidateGlobalConstraintProofSegmentsError::PackedProofValues)
            .map_err(SetupPreflightError::GlobalConstraints)?;
        let challenges = if uses_transcript_inputs {
            transcript_challenges
                .flat_challenges()
                .map_err(ValidateGlobalConstraintProofSegmentsError::Transcript)
                .map_err(SetupPreflightError::GlobalConstraints)?
        } else {
            &[]
        };
        validate_global_constraints(
            &catalog.global_constraints,
            GlobalConstraintInputs {
                publics: public_values,
                proof_values: packed_proof_values,
                challenges,
                group_values: &global_values.group_values,
            },
        )
        .map_err(ValidateGlobalConstraintProofSegmentsError::Validation)
        .map_err(SetupPreflightError::GlobalConstraints)?;
    }

    if !catalog.global_hints.hints.is_empty() {
        let full_public_values = public_fields.as_slice();
        let requirements = global_hint_input_requirements(&catalog.global_hints);
        let public_values = if requirements.publics {
            full_public_values
        } else {
            &[]
        };
        let packed_proof_values = if requirements.proof_values {
            packed_proof_values_cache
                .packed_values()
                .map_err(ResolveGlobalHintProofSegmentsError::PackedProofValues)
                .map_err(SetupPreflightError::GlobalHints)?
        } else {
            &[]
        };
        let challenges = if uses_transcript_inputs && requirements.challenges {
            transcript_challenges
                .flat_challenges()
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
                proof_values: packed_proof_values,
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
    let unit_challenges = if transcript_challenges.has_unit_challenges() {
        Some(
            transcript_challenges
                .unit_challenges()
                .map_err(ValidateOptionalPcsFriOpeningProofSegmentsError::Transcript)
                .map_err(SetupPreflightError::PcsFri)?,
        )
    } else {
        None
    };
    validate_optional_pcs_fri_opening_proof_segments_with_preflight_values(
        fri_request,
        unit_challenges,
        &global_values.proof_values,
    )
    .map_err(SetupPreflightError::PcsFri)?;

    Ok(report)
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
    for query_unit in &query_plan.units {
        let unit_index = usize::try_from(query_unit.unit_index).map_err(|_| {
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
            query_unit.trace_instance_index,
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
    preloaded_proof_values: Option<SetupPreflightContributionProofValues>,
) -> Result<SetupPreflightGlobalValues, SetupPreflightError> {
    let (proof_values, packed_proof_values) = match preloaded_proof_values {
        Some(values) => (values.proof_values, Some(values.packed_proof_values)),
        None => (
            load_pcs_proof_values_from_segments(&catalog.layout.global_info, &proof.segments)
                .map_err(SetupPreflightError::ProofValues)?,
            None,
        ),
    };
    let group_values =
        load_group_values_from_segments(&catalog.layout.global_info, &proof.segments)
            .map_err(SetupPreflightError::GroupValues)?;

    Ok(SetupPreflightGlobalValues {
        proof_values,
        packed_proof_values,
        group_values,
    })
}

fn validate_optional_contribution_segment(
    catalog: &KeyDirectoryCatalog,
    proof: &ProofArtifact,
) -> Result<Option<Vec<ProveContributionEntry>>, SetupPreflightError> {
    if !proof
        .segments
        .iter()
        .any(|segment| segment.id == CONTRIBUTION_SEGMENT_ID)
    {
        return Ok(None);
    }

    let entries = load_contribution_segment_from_segments(&proof.segments)
        .map_err(ContributionChallengeError::from)
        .map_err(SetupPreflightError::Contribution)?;
    aggregate_contribution_values(&catalog.layout.global_info, &entries)
        .map_err(SetupPreflightError::Contribution)?;
    Ok(Some(entries))
}

fn validate_optional_challenge_values_segment(
    proof: &ProofArtifact,
) -> Result<Option<Vec<[u64; 3]>>, SetupPreflightError> {
    let mut segments = proof
        .segments
        .iter()
        .filter(|segment| segment.id == CHALLENGE_VALUES_SEGMENT_ID);
    let Some(segment) = segments.next() else {
        return Ok(None);
    };
    if segments.next().is_some() {
        return Err(SetupPreflightError::DuplicateChallengeValuesSegment);
    }
    let segment = parse_challenge_values_segment(&segment.data)
        .map_err(SetupPreflightError::ChallengeValues)?;
    Ok(Some(segment.values))
}

fn validate_optional_contribution_challenge_values(
    catalog: &KeyDirectoryCatalog,
    proof: &ProofArtifact,
    public_fields: &[Felt],
    contribution_entries: Option<&[ProveContributionEntry]>,
    challenge_values: Option<&[[u64; 3]]>,
) -> Result<Option<SetupPreflightContributionProofValues>, SetupPreflightError> {
    let Some(challenge_values) = challenge_values else {
        if contribution_entries.is_some() {
            return Err(SetupPreflightError::MissingContributionChallengeValues);
        }
        return Ok(None);
    };
    let Some(contribution_entries) = contribution_entries else {
        return Ok(None);
    };

    let proof_values =
        load_pcs_proof_values_from_segments(&catalog.layout.global_info, &proof.segments)
            .map_err(SetupPreflightError::ProofValues)?;
    let packed_proof_values = flatten_pcs_proof_values(&catalog.layout.global_info, &proof_values)
        .map_err(SetupPreflightError::ProofValuePacking)?;
    let expected = derive_global_challenge_from_loaded_contributions(
        &catalog.layout.global_info,
        public_fields,
        &packed_proof_values,
        &proof.segments,
        contribution_entries,
    )
    .map_err(SetupPreflightError::Contribution)?;
    if challenge_values.len() != 1 || challenge_values[0] != expected.to_u64s() {
        return Err(SetupPreflightError::ContributionChallengeValuesMismatch);
    }
    Ok(Some(SetupPreflightContributionProofValues {
        proof_values,
        packed_proof_values,
    }))
}

fn validate_optional_trace_constraint_segment(
    catalog: &KeyDirectoryCatalog,
    trace_constraint_units: &[TraceConstraintPreflightUnit],
    witness_segments: &[LoadedWitnessCommitmentSegmentRef<'_>],
) -> Result<(), SetupPreflightError> {
    if trace_constraint_units.is_empty() {
        if witness_segments.is_empty() {
            return Ok(());
        }
        return Err(SetupPreflightError::TraceConstraintBinding {
            message: "missing trace constraint evidence segment".to_owned(),
        });
    }

    let mut witness_shapes = BTreeMap::new();
    for witness_segment in witness_segments {
        witness_shapes.insert(
            (
                witness_segment.identity.unit_index,
                witness_segment.identity.trace_instance_index,
            ),
            (
                witness_segment.witness.trace_rows,
                witness_segment.witness.trace_columns,
            ),
        );
    }

    let mut evidence_units = BTreeMap::new();
    for unit in trace_constraint_units {
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
    use lzvm_artifacts::global_info::{CurveKind, GlobalInfo, PublicValue};
    use lzvm_artifacts::proof::{ProofArtifact, ProofSegment};
    use lzvm_artifacts::public_values::{PublicValueEntry, PublicValues};
    use lzvm_field::{FieldError, MODULUS};

    use super::{
        validate_optional_challenge_values_segment,
        validate_public_values_metadata_with_field_count, SetupPreflightError,
    };

    const FIRST_CHALLENGE_VALUE_OFFSET: usize = 12;

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
    fn public_values_metadata_rejects_zero_dimensions_before_field_count_accepts() {
        let global_info = GlobalInfo {
            name: "global".to_owned(),
            air_groups: Vec::new(),
            airs: Vec::new(),
            curve: CurveKind::None,
            lattice_size: None,
            aggregation_types: Vec::new(),
            n_publics: 1,
            num_challenges: Vec::new(),
            num_proof_values: Vec::new(),
            proof_values_map: Vec::new(),
            publics_map: vec![
                PublicValue {
                    name: "scalar".to_owned(),
                    stage: 1,
                    lengths: Vec::new(),
                },
                PublicValue {
                    name: "zero".to_owned(),
                    stage: 1,
                    lengths: vec![0],
                },
            ],
            transcript_arity: 4,
        };
        let public_values = PublicValues {
            schema_version: 1,
            setup_hash: [0; 32],
            values: vec![
                PublicValueEntry {
                    name: "scalar".to_owned(),
                    elements: vec![7],
                },
                PublicValueEntry {
                    name: "zero".to_owned(),
                    elements: Vec::new(),
                },
            ],
        };

        assert_eq!(
            validate_public_values_metadata_with_field_count(&global_info, &public_values, 1),
            Err(SetupPreflightError::PublicValueCountOverflow)
        );
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
