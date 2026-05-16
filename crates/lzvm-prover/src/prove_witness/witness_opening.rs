use std::collections::{BTreeMap, BTreeSet};

use lzvm_artifacts::pcs_query_segment::{
    parse_pcs_query_plan_segment, PcsQueryPlanSegment, PcsQueryPlanUnit,
};
use lzvm_artifacts::proof::ProofSegment;
use lzvm_artifacts::witness_opening_segment::{
    encode_witness_opening_segment, WitnessOpeningLevelSegment, WitnessOpeningQuerySegment,
    WitnessOpeningSegment, WitnessOpeningStageSegment, WitnessOpeningUnitSegment,
    WITNESS_OPENING_SEGMENT_ID,
};

use crate::witness_commitment::open_witness_stage_commitment;
use crate::witness_execution::ProveWitnessCommitments;
use crate::ProveSchedule;

use super::ProveWitnessOpeningSegmentError;

pub fn build_witness_opening_segment(
    schedule: &ProveSchedule,
    query_segment: &ProofSegment,
    output: &ProveWitnessCommitments,
) -> Result<ProofSegment, ProveWitnessOpeningSegmentError> {
    let query_plan = parse_query_plan_segment(query_segment)?;
    build_witness_opening_segment_from_query_plan(schedule, &query_plan, &[output])
}

pub fn build_witness_opening_segment_batch(
    schedule: &ProveSchedule,
    query_segment: &ProofSegment,
    outputs: &[&ProveWitnessCommitments],
) -> Result<ProofSegment, ProveWitnessOpeningSegmentError> {
    let query_plan = parse_query_plan_segment(query_segment)?;
    build_witness_opening_segment_from_query_plan(schedule, &query_plan, outputs)
}

fn parse_query_plan_segment(
    query_segment: &ProofSegment,
) -> Result<PcsQueryPlanSegment, ProveWitnessOpeningSegmentError> {
    Ok(parse_pcs_query_plan_segment(&query_segment.data)?)
}

fn build_witness_opening_segment_from_query_plan(
    schedule: &ProveSchedule,
    query_plan: &PcsQueryPlanSegment,
    outputs: &[&ProveWitnessCommitments],
) -> Result<ProofSegment, ProveWitnessOpeningSegmentError> {
    let mut outputs_by_unit = BTreeMap::new();
    for output in outputs {
        let unit_index_u32 = u32::try_from(output.unit_index()).map_err(|_| {
            ProveWitnessOpeningSegmentError::UnitIndexOverflow {
                unit_index: output.unit_index(),
            }
        })?;
        if outputs_by_unit.insert(unit_index_u32, *output).is_some() {
            return Err(ProveWitnessOpeningSegmentError::DuplicateOutputUnit {
                unit_index: output.unit_index(),
            });
        }
    }

    let query_units = query_plan
        .units
        .iter()
        .map(|unit| unit.unit_index)
        .collect::<BTreeSet<_>>();
    for unit_index_u32 in outputs_by_unit.keys() {
        if !query_units.contains(unit_index_u32) {
            return Err(ProveWitnessOpeningSegmentError::MissingQueryUnit {
                unit_index: *unit_index_u32 as usize,
            });
        }
    }

    let mut units = Vec::with_capacity(query_plan.units.len());
    for query_unit in &query_plan.units {
        let unit_index = query_unit.unit_index as usize;
        let output = outputs_by_unit
            .get(&query_unit.unit_index)
            .ok_or(ProveWitnessOpeningSegmentError::MissingOutputUnit { unit_index })?;
        units.push(build_witness_opening_unit_segment(
            schedule, query_unit, output,
        )?);
    }

    let segment = WitnessOpeningSegment { units };
    Ok(ProofSegment {
        id: WITNESS_OPENING_SEGMENT_ID,
        data: encode_witness_opening_segment(&segment)?,
    })
}

fn build_witness_opening_unit_segment(
    schedule: &ProveSchedule,
    query_unit: &PcsQueryPlanUnit,
    output: &ProveWitnessCommitments,
) -> Result<WitnessOpeningUnitSegment, ProveWitnessOpeningSegmentError> {
    let unit_index_u32 = u32::try_from(output.unit_index()).map_err(|_| {
        ProveWitnessOpeningSegmentError::UnitIndexOverflow {
            unit_index: output.unit_index(),
        }
    })?;
    if query_unit.unit_index != unit_index_u32 {
        return Err(ProveWitnessOpeningSegmentError::MissingQueryUnit {
            unit_index: output.unit_index(),
        });
    }
    let unit = schedule.units.get(output.unit_index()).ok_or(
        ProveWitnessOpeningSegmentError::UnitIndexOutOfRange {
            unit_index: output.unit_index(),
            unit_count: schedule.units.len(),
        },
    )?;
    let mut queries = Vec::with_capacity(query_unit.queries.len());
    for row_index in &query_unit.queries {
        let mut stages = Vec::with_capacity(output.stage_commitments().stage_count());
        for commitment in output.stage_commitments().commitments() {
            let stage_index = commitment.stage_index();
            let width = unit
                .stage_commit_widths
                .get(stage_index.checked_sub(1).ok_or(
                    ProveWitnessOpeningSegmentError::StageIndexOutOfRange {
                        stage_index,
                        stage_count: unit.stage_commit_widths.len(),
                    },
                )?)
                .ok_or(ProveWitnessOpeningSegmentError::StageIndexOutOfRange {
                    stage_index,
                    stage_count: unit.stage_commit_widths.len(),
                })?;
            let opening = open_witness_stage_commitment(
                commitment,
                *row_index,
                unit.extended_domain_size,
                usize::try_from(*width).map_err(|_| {
                    ProveWitnessOpeningSegmentError::StageIndexOutOfRange {
                        stage_index,
                        stage_count: unit.stage_commit_widths.len(),
                    }
                })?,
            )?;
            stages.push(WitnessOpeningStageSegment {
                stage_index: u32::try_from(stage_index).map_err(|_| {
                    ProveWitnessOpeningSegmentError::StageIndexOutOfRange {
                        stage_index,
                        stage_count: unit.stage_commit_widths.len(),
                    }
                })?,
                values: opening
                    .values()
                    .iter()
                    .map(|value| value.to_u64())
                    .collect(),
                siblings: opening
                    .siblings()
                    .iter()
                    .map(|level| WitnessOpeningLevelSegment {
                        siblings: level
                            .iter()
                            .map(|digest| digest.map(|value| value.to_u64()))
                            .collect(),
                    })
                    .collect(),
            });
        }
        queries.push(WitnessOpeningQuerySegment {
            row_index: *row_index,
            stages,
        });
    }

    Ok(WitnessOpeningUnitSegment {
        unit_index: unit_index_u32,
        queries,
    })
}
