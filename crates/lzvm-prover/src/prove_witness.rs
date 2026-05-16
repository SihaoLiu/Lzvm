mod query_plan;
mod types;

pub use query_plan::*;
pub use types::*;

use std::collections::{BTreeMap, BTreeSet};

use lzvm_artifacts::constant_opening_segment::{
    encode_constant_opening_segment, ConstantOpeningLevelSegment, ConstantOpeningQuerySegment,
    ConstantOpeningSegment, ConstantOpeningUnitSegment, CONSTANT_OPENING_SEGMENT_ID,
};
use lzvm_artifacts::constant_tree::read_constant_tree_file;
use lzvm_artifacts::key_directory::KeyDirectoryCatalog;
use lzvm_artifacts::pcs_evaluation_segment::{
    encode_pcs_evaluation_segment, PcsEvaluationSegment, PcsEvaluationUnitSegment,
    PCS_EVALUATION_SEGMENT_ID,
};
use lzvm_artifacts::pcs_material_segment::{
    encode_pcs_material_manifest_segment, PcsMaterialManifestSegment, PcsMaterialManifestUnit,
    PCS_MATERIAL_MANIFEST_SEGMENT_ID,
};
use lzvm_artifacts::pcs_query_segment::{
    parse_pcs_query_plan_segment, PcsQueryPlanSegment, PcsQueryPlanUnit,
};
use lzvm_artifacts::proof::ProofSegment;
use lzvm_artifacts::witness_opening_segment::{
    encode_witness_opening_segment, WitnessOpeningLevelSegment, WitnessOpeningQuerySegment,
    WitnessOpeningSegment, WitnessOpeningStageSegment, WitnessOpeningUnitSegment,
    WITNESS_OPENING_SEGMENT_ID,
};
use lzvm_artifacts::witness_segment::{
    encode_witness_commitment_segment, WitnessCommitmentSegment, WitnessCommitmentStageSegment,
    WITNESS_COMMITMENT_SEGMENT_BASE_ID,
};
use lzvm_field::Ext3;
use sha2::{Digest, Sha256};

use crate::constant_tree_opening::open_constant_tree_row;
use crate::witness_commitment::open_witness_stage_commitment;
use crate::witness_execution::ProveWitnessCommitments;
use crate::ProveSchedule;

pub fn build_witness_commitment_segment(
    output: &ProveWitnessCommitments,
) -> Result<ProofSegment, ProveWitnessSegmentError> {
    let unit_index =
        u32::try_from(output.unit_index()).map_err(|_| ProveWitnessSegmentError::LengthOverflow)?;
    let id = WITNESS_COMMITMENT_SEGMENT_BASE_ID
        .checked_add(unit_index)
        .ok_or(ProveWitnessSegmentError::LengthOverflow)?;
    let mut stages = Vec::with_capacity(output.stage_commitments().stage_count());
    for commitment in output.stage_commitments().commitments() {
        let stage_index = u32::try_from(commitment.stage_index())
            .map_err(|_| ProveWitnessSegmentError::LengthOverflow)?;
        let arity = u32::try_from(commitment.arity())
            .map_err(|_| ProveWitnessSegmentError::LengthOverflow)?;
        let tree_byte_count = u64::try_from(commitment.tree_bytes().len())
            .map_err(|_| ProveWitnessSegmentError::LengthOverflow)?;
        stages.push(WitnessCommitmentStageSegment {
            stage_index,
            arity,
            root: commitment.root().map(|value| value.to_u64()),
            tree_byte_count,
            tree_digest: Sha256::digest(commitment.tree_bytes()).into(),
        });
    }

    let segment = WitnessCommitmentSegment {
        unit_index,
        input_byte_count: u64::try_from(output.input_byte_count())
            .map_err(|_| ProveWitnessSegmentError::LengthOverflow)?,
        trace_rows: u64::try_from(output.trace_row_count())
            .map_err(|_| ProveWitnessSegmentError::LengthOverflow)?,
        trace_columns: u64::try_from(output.trace_column_count())
            .map_err(|_| ProveWitnessSegmentError::LengthOverflow)?,
        stages,
    };
    Ok(ProofSegment {
        id,
        data: encode_witness_commitment_segment(&segment)?,
    })
}

pub fn build_pcs_material_manifest_segment(
    schedule: &ProveSchedule,
) -> Result<ProofSegment, ProvePcsMaterialSegmentError> {
    let mut units = Vec::with_capacity(schedule.units.len());
    for (unit_index, unit) in schedule.units.iter().enumerate() {
        let unit_index_u32 = u32::try_from(unit_index)
            .map_err(|_| ProvePcsMaterialSegmentError::UnitIndexOverflow { unit_index })?;
        units.push(PcsMaterialManifestUnit {
            unit_index: unit_index_u32,
            plan_digest: unit.pcs_material_plan_digest.ok_or(
                ProvePcsMaterialSegmentError::MissingMaterial {
                    unit_index,
                    kind: unit.kind,
                },
            )?,
            fixed_column_digest: unit.pcs_material_fixed_column_digest.ok_or(
                ProvePcsMaterialSegmentError::MissingMaterial {
                    unit_index,
                    kind: unit.kind,
                },
            )?,
            constant_tree_digest: unit.pcs_material_constant_tree_digest.ok_or(
                ProvePcsMaterialSegmentError::MissingMaterial {
                    unit_index,
                    kind: unit.kind,
                },
            )?,
            constant_tree_root: unit.pcs_material_constant_tree_root.ok_or(
                ProvePcsMaterialSegmentError::MissingMaterial {
                    unit_index,
                    kind: unit.kind,
                },
            )?,
            fixed_byte_count: unit.pcs_material_fixed_byte_count.ok_or(
                ProvePcsMaterialSegmentError::MissingMaterial {
                    unit_index,
                    kind: unit.kind,
                },
            )?,
            constant_tree_byte_count: unit.pcs_material_constant_tree_byte_count.ok_or(
                ProvePcsMaterialSegmentError::MissingMaterial {
                    unit_index,
                    kind: unit.kind,
                },
            )?,
            leaf_byte_count: unit.pcs_material_leaf_byte_count.ok_or(
                ProvePcsMaterialSegmentError::MissingMaterial {
                    unit_index,
                    kind: unit.kind,
                },
            )?,
            node_byte_count: unit.pcs_material_node_byte_count.ok_or(
                ProvePcsMaterialSegmentError::MissingMaterial {
                    unit_index,
                    kind: unit.kind,
                },
            )?,
        });
    }
    let manifest = PcsMaterialManifestSegment { units };
    Ok(ProofSegment {
        id: PCS_MATERIAL_MANIFEST_SEGMENT_ID,
        data: encode_pcs_material_manifest_segment(&manifest)?,
    })
}

pub fn build_pcs_evaluation_segment(
    schedule: &ProveSchedule,
    values: &[ProvePcsEvaluationValues],
) -> Result<ProofSegment, ProvePcsEvaluationSegmentError> {
    let mut units = Vec::with_capacity(values.len());
    for input in values {
        let unit = schedule.units.get(input.unit_index).ok_or(
            ProvePcsEvaluationSegmentError::UnitIndexOutOfRange {
                unit_index: input.unit_index,
                unit_count: schedule.units.len(),
            },
        )?;
        let expected_value_count = unit.expected_evaluation_value_count();
        if input.values.len() != expected_value_count {
            return Err(ProvePcsEvaluationSegmentError::ValueCountMismatch {
                unit_index: input.unit_index,
                expected: expected_value_count,
                found: input.values.len(),
            });
        }
        units.push(PcsEvaluationUnitSegment {
            unit_index: u32::try_from(input.unit_index).map_err(|_| {
                ProvePcsEvaluationSegmentError::UnitIndexOverflow {
                    unit_index: input.unit_index,
                }
            })?,
            values: input.values.iter().copied().map(Ext3::to_u64s).collect(),
        });
    }
    units.sort_by_key(|unit| unit.unit_index);

    let segment = PcsEvaluationSegment { units };
    Ok(ProofSegment {
        id: PCS_EVALUATION_SEGMENT_ID,
        data: encode_pcs_evaluation_segment(&segment)?,
    })
}

pub fn build_witness_opening_segment(
    schedule: &ProveSchedule,
    query_segment: &ProofSegment,
    output: &ProveWitnessCommitments,
) -> Result<ProofSegment, ProveWitnessOpeningSegmentError> {
    let query_plan = parse_pcs_query_plan_segment(&query_segment.data)?;
    build_witness_opening_segment_from_query_plan(schedule, &query_plan, &[output])
}

pub fn build_witness_opening_segment_batch(
    schedule: &ProveSchedule,
    query_segment: &ProofSegment,
    outputs: &[&ProveWitnessCommitments],
) -> Result<ProofSegment, ProveWitnessOpeningSegmentError> {
    let query_plan = parse_pcs_query_plan_segment(&query_segment.data)?;
    build_witness_opening_segment_from_query_plan(schedule, &query_plan, outputs)
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

pub fn build_constant_opening_segment(
    catalog: &KeyDirectoryCatalog,
    schedule: &ProveSchedule,
    query_segment: &ProofSegment,
) -> Result<ProofSegment, ProveConstantOpeningSegmentError> {
    let query_plan = parse_pcs_query_plan_segment(&query_segment.data)?;
    let mut units = Vec::with_capacity(query_plan.units.len());
    for query_unit in &query_plan.units {
        let unit_index = usize::try_from(query_unit.unit_index).map_err(|_| {
            ProveConstantOpeningSegmentError::UnitIndexOverflow {
                unit_index: query_unit.unit_index,
            }
        })?;
        let schedule_unit = schedule.units.get(unit_index).ok_or(
            ProveConstantOpeningSegmentError::UnitIndexOutOfRange {
                unit_index,
                unit_count: schedule.units.len(),
            },
        )?;
        let catalog_unit = catalog.units.get(unit_index).ok_or(
            ProveConstantOpeningSegmentError::UnitIndexOutOfRange {
                unit_index,
                unit_count: catalog.units.len(),
            },
        )?;
        let tree = read_constant_tree_file(
            &catalog_unit.paths.constant_tree,
            &catalog_unit.metadata.setup,
        )
        .map_err(|source| ProveConstantOpeningSegmentError::ConstantTree { unit_index, source })?;
        let arity = usize::try_from(schedule_unit.merkle_tree_arity).map_err(|_| {
            ProveConstantOpeningSegmentError::UnitIndexOutOfRange {
                unit_index,
                unit_count: schedule.units.len(),
            }
        })?;
        let mut queries = Vec::with_capacity(query_unit.queries.len());
        for row_index in &query_unit.queries {
            let opening = open_constant_tree_row(&tree, *row_index, arity)?;
            queries.push(ConstantOpeningQuerySegment {
                row_index: *row_index,
                values: opening
                    .values()
                    .iter()
                    .map(|value| value.to_u64())
                    .collect(),
                siblings: opening
                    .siblings()
                    .iter()
                    .map(|level| ConstantOpeningLevelSegment {
                        siblings: level
                            .iter()
                            .map(|digest| digest.map(|value| value.to_u64()))
                            .collect(),
                    })
                    .collect(),
            });
        }
        units.push(ConstantOpeningUnitSegment {
            unit_index: query_unit.unit_index,
            queries,
        });
    }

    let segment = ConstantOpeningSegment { units };
    Ok(ProofSegment {
        id: CONSTANT_OPENING_SEGMENT_ID,
        data: encode_constant_opening_segment(&segment)?,
    })
}
