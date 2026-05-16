use lzvm_artifacts::global_info::GlobalInfo;
use lzvm_artifacts::pcs_fri_segment::PcsFriOpeningUnitSegment;
use lzvm_artifacts::proof::ProofSegment;
use lzvm_artifacts::setup_info::StageValue;
use lzvm_artifacts::verifier_info::VerifierCode;
use lzvm_field::{Ext3, Felt};

use crate::ProveSchedule;

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
    pub binding_segments: &'a [ProofSegment],
}

#[derive(Debug, Clone, Copy)]
pub struct ValidateOptionalPcsFriOpeningProofSegmentsRequest<'a> {
    pub schedule: &'a ProveSchedule,
    pub verifier_codes: &'a [&'a VerifierCode],
    pub global_info: &'a GlobalInfo,
    pub public_values: &'a [Felt],
    pub segments: &'a [ProofSegment],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PcsFriTranscriptCommitments {
    pub challenges: Vec<Ext3>,
    pub layer_roots: Vec<[Felt; 4]>,
    pub final_polynomial: Vec<Ext3>,
    pub final_query_challenge: Ext3,
}
