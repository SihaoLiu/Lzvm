use lzvm_artifacts::proof::ProofSegment;
use lzvm_field::{Ext3, Felt};

use crate::pcs_fri::PcsFriTranscriptCommitments;
use crate::witness_trace::WitnessTraceBuffer;
use crate::{ProveExecutionUnitArtifacts, ProveWitnessAuxiliaryInputs};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvePcsFriOpeningValues {
    pub unit_index: usize,
    pub challenges: Vec<Ext3>,
    pub polynomial: Vec<Ext3>,
}

#[derive(Debug, Clone, Copy)]
pub struct ProvePcsFriOpeningTraceValues<'a> {
    pub unit_index: usize,
    pub execution_unit: &'a ProveExecutionUnitArtifacts,
    pub trace: &'a WitnessTraceBuffer,
    pub publics: &'a [Felt],
    pub auxiliary_inputs: &'a ProveWitnessAuxiliaryInputs,
    pub challenges: &'a [Ext3],
    pub xi_challenge: Ext3,
}

#[derive(Debug, Clone, Copy)]
pub struct ProvePcsFriTranscriptTraceValues<'a> {
    pub unit_index: usize,
    pub execution_unit: &'a ProveExecutionUnitArtifacts,
    pub trace: &'a WitnessTraceBuffer,
    pub publics: &'a [Felt],
    pub auxiliary_inputs: &'a ProveWitnessAuxiliaryInputs,
    pub constant_root: [Felt; 4],
    pub witness_roots: &'a [[Felt; 4]],
    pub evaluation_values: &'a [Ext3],
    pub xi_challenge: Ext3,
    pub binding_segments: &'a [ProofSegment],
}

#[derive(Debug, Clone, Copy)]
pub struct ProvePcsFriTranscriptTraceSegmentValues<'a> {
    pub unit_index: usize,
    pub execution_unit: &'a ProveExecutionUnitArtifacts,
    pub trace: &'a WitnessTraceBuffer,
    pub publics: &'a [Felt],
    pub auxiliary_inputs: &'a ProveWitnessAuxiliaryInputs,
    pub material_segment: &'a ProofSegment,
    pub witness_segment: &'a ProofSegment,
    pub evaluation_segment: &'a ProofSegment,
    pub binding_segments: &'a [ProofSegment],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvePcsFriTranscriptValues {
    pub unit_index: usize,
    pub polynomial: Vec<Ext3>,
    pub commitments: PcsFriTranscriptCommitments,
}
