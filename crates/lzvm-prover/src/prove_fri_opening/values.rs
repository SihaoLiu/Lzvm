use lzvm_artifacts::proof::ProofSegment;
use lzvm_field::{Ext3, Felt};

use crate::pcs_fri::PcsFriTranscriptCommitments;
#[cfg(feature = "cuda")]
use crate::witness_commitment::WitnessStageRetainedSourceDevice;
use crate::witness_execution::ProveWitnessAuxiliaryInputSlices;
use crate::witness_trace::WitnessTraceBuffer;
use crate::{ProveExecutionUnitArtifacts, ProveWitnessAuxiliaryInputs};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvePcsFriOpeningValues {
    pub unit_index: usize,
    pub trace_instance_index: u32,
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
    pub trace_instance_index: u32,
    pub execution_unit: &'a ProveExecutionUnitArtifacts,
    pub trace: &'a WitnessTraceBuffer,
    pub publics: &'a [Felt],
    pub auxiliary_inputs: &'a ProveWitnessAuxiliaryInputs,
    pub material_segment: &'a ProofSegment,
    pub witness_segment: &'a ProofSegment,
    pub evaluation_segment: &'a ProofSegment,
    pub binding_segments: &'a [ProofSegment],
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ProvePcsFriTranscriptTraceValueRef<'a> {
    pub unit_index: usize,
    pub execution_unit: &'a ProveExecutionUnitArtifacts,
    pub trace: &'a WitnessTraceBuffer,
    #[cfg(feature = "cuda")]
    pub(crate) stage_source_devices: Option<&'a [WitnessStageRetainedSourceDevice]>,
    pub publics: &'a [Felt],
    pub auxiliary_inputs: ProveWitnessAuxiliaryInputSlices<'a>,
    pub constant_root: [Felt; 4],
    pub witness_roots: &'a [[Felt; 4]],
    pub evaluation_values: &'a [Ext3],
    pub xi_challenge: Ext3,
    pub binding_segments: &'a [ProofSegment],
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ProvePcsFriTranscriptTraceSegmentValueRef<'a> {
    pub unit_index: usize,
    pub trace_instance_index: u32,
    pub execution_unit: &'a ProveExecutionUnitArtifacts,
    pub trace: &'a WitnessTraceBuffer,
    #[cfg(feature = "cuda")]
    pub(crate) stage_source_devices: Option<&'a [WitnessStageRetainedSourceDevice]>,
    pub publics: &'a [Felt],
    pub auxiliary_inputs: ProveWitnessAuxiliaryInputSlices<'a>,
    pub material_segment: &'a ProofSegment,
    pub witness_segment: &'a ProofSegment,
    pub evaluation_segment: &'a ProofSegment,
    pub binding_segments: &'a [ProofSegment],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvePcsFriTranscriptValues {
    pub unit_index: usize,
    pub trace_instance_index: u32,
    pub polynomial: Vec<Ext3>,
    pub commitments: PcsFriTranscriptCommitments,
}
