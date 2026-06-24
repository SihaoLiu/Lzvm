use lzvm_artifacts::pcs_material_segment::PCS_MATERIAL_MANIFEST_SEGMENT_ID;
use lzvm_artifacts::proof::ProofSegment;
use lzvm_artifacts::witness_segment::{
    parse_witness_commitment_segment, witness_commitment_segment_identity,
    WITNESS_COMMITMENT_SEGMENT_BASE_ID,
};

use crate::ProveUnitSchedule;

use super::LoadWitnessCommitmentSegmentsError;

pub fn load_witness_commitment_segments(
    units: &[ProveUnitSchedule],
    segments: &[ProofSegment],
) -> Result<Vec<ProofSegment>, LoadWitnessCommitmentSegmentsError> {
    Ok(load_witness_commitment_segment_refs(units, segments)?
        .into_iter()
        .cloned()
        .collect())
}

pub fn load_witness_commitment_segment_refs<'a>(
    units: &[ProveUnitSchedule],
    segments: &'a [ProofSegment],
) -> Result<Vec<&'a ProofSegment>, LoadWitnessCommitmentSegmentsError> {
    let unit_count = u32::try_from(units.len())
        .map_err(|_| LoadWitnessCommitmentSegmentsError::UnitCountOverflow)?;
    let Some(end_id) = WITNESS_COMMITMENT_SEGMENT_BASE_ID.checked_add(unit_count) else {
        return Err(LoadWitnessCommitmentSegmentsError::SegmentIdOverflow);
    };
    if end_id > PCS_MATERIAL_MANIFEST_SEGMENT_ID {
        return Err(LoadWitnessCommitmentSegmentsError::SegmentIdOverflow);
    }
    let mut out = Vec::new();
    let mut seen_unit_identities = std::collections::BTreeSet::new();

    for segment in segments {
        if segment.id < WITNESS_COMMITMENT_SEGMENT_BASE_ID {
            continue;
        }
        if segment.id >= PCS_MATERIAL_MANIFEST_SEGMENT_ID {
            continue;
        }
        let Some(identity) = witness_commitment_segment_identity(unit_count, segment.id)
            .map_err(|_| LoadWitnessCommitmentSegmentsError::SegmentIdOverflow)?
        else {
            continue;
        };
        let unit_index = usize::try_from(identity.unit_index)
            .map_err(|_| LoadWitnessCommitmentSegmentsError::UnitIndexOverflow)?;
        if !seen_unit_identities.insert((identity.unit_index, identity.trace_instance_index)) {
            return Err(LoadWitnessCommitmentSegmentsError::DuplicateSegment { unit_index });
        }
        validate_witness_commitment_segment(units, segment)?;
        out.push(segment);
    }

    if out.is_empty() {
        return Err(LoadWitnessCommitmentSegmentsError::MissingSegment);
    }
    out.sort_by_key(|segment| segment.id);
    Ok(out)
}

fn validate_witness_commitment_segment(
    units: &[ProveUnitSchedule],
    segment: &ProofSegment,
) -> Result<(), LoadWitnessCommitmentSegmentsError> {
    let unit_count = u32::try_from(units.len())
        .map_err(|_| LoadWitnessCommitmentSegmentsError::UnitCountOverflow)?;
    let identity = witness_commitment_segment_identity(unit_count, segment.id)
        .map_err(|_| LoadWitnessCommitmentSegmentsError::SegmentIdOverflow)?
        .ok_or(LoadWitnessCommitmentSegmentsError::UnitIndexOverflow)?;
    let unit_index = usize::try_from(identity.unit_index)
        .map_err(|_| LoadWitnessCommitmentSegmentsError::UnitIndexOverflow)?;
    let parsed = parse_witness_commitment_segment(&segment.data)
        .map_err(|source| LoadWitnessCommitmentSegmentsError::Segment { unit_index, source })?;
    if parsed.unit_index != identity.unit_index {
        return Err(LoadWitnessCommitmentSegmentsError::UnitMismatch { unit_index });
    }
    let unit = units
        .get(unit_index)
        .ok_or(LoadWitnessCommitmentSegmentsError::UnitIndexOverflow)?;
    if parsed.trace_rows != unit.base_domain_size {
        return Err(LoadWitnessCommitmentSegmentsError::RowCountMismatch { unit_index });
    }
    let trace_columns = unit
        .stage_commit_widths
        .iter()
        .try_fold(0_u64, |acc, width| acc.checked_add(u64::from(*width)))
        .ok_or(LoadWitnessCommitmentSegmentsError::ColumnCountOverflow)?;
    if parsed.trace_columns != trace_columns {
        return Err(LoadWitnessCommitmentSegmentsError::ColumnCountMismatch { unit_index });
    }
    if parsed.stages.len() != unit.stage_commit_widths.len() {
        return Err(LoadWitnessCommitmentSegmentsError::StageCountMismatch { unit_index });
    }
    for (stage_index, stage) in parsed.stages.iter().enumerate() {
        let expected_stage_index = u32::try_from(stage_index + 1)
            .map_err(|_| LoadWitnessCommitmentSegmentsError::StageIndexOverflow)?;
        if stage.stage_index != expected_stage_index {
            return Err(LoadWitnessCommitmentSegmentsError::StageIndexMismatch { unit_index });
        }
        if stage.arity != unit.merkle_tree_arity {
            return Err(LoadWitnessCommitmentSegmentsError::ArityMismatch { unit_index });
        }
        if stage.tree_byte_count == 0 {
            return Err(LoadWitnessCommitmentSegmentsError::EmptyTree { unit_index });
        }
    }
    Ok(())
}
