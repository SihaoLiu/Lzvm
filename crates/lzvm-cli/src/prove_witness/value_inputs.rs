use std::fs;
use std::path::Path;

use lzvm_artifacts::challenge_values_segment::{
    parse_challenge_values_segment, CHALLENGE_VALUES_SEGMENT_ID,
};
use lzvm_artifacts::global_info::GlobalInfo;
use lzvm_artifacts::group_values_segment::GROUP_VALUES_SEGMENT_ID;
use lzvm_artifacts::pcs_evaluation_segment::{
    parse_pcs_evaluation_segment, PCS_EVALUATION_SEGMENT_ID,
};
use lzvm_artifacts::pcs_proof_values_segment::PCS_PROOF_VALUES_SEGMENT_ID;
use lzvm_artifacts::proof::ProofSegment;
use lzvm_artifacts::unit_values_segment::{parse_unit_values_segment, UNIT_VALUES_SEGMENT_ID};
use lzvm_field::{Ext3, Felt};
use lzvm_prover::group_values::load_group_values_from_segments;
use lzvm_prover::proof_values::{flatten_pcs_proof_values, load_pcs_proof_values_from_segments};
use lzvm_prover::unit_values::{load_unit_values_for_identity_from_segments, ProveUnitValues};
use lzvm_prover::{ProveSchedule, ProveWitnessTraceCommitments};

pub(super) fn read_evaluation_values_segment_input(path: &Path) -> Result<ProofSegment, String> {
    let bytes = fs::read(path).map_err(|error| {
        format!(
            "read evaluation values segment failed: {}: {error}",
            path.display()
        )
    })?;
    parse_pcs_evaluation_segment(&bytes)
        .map_err(|error| format!("parse evaluation values segment failed: {error}"))?;
    Ok(ProofSegment {
        id: PCS_EVALUATION_SEGMENT_ID,
        data: bytes,
    })
}

pub(super) fn read_challenge_values_segment_input(path: &Path) -> Result<Vec<Ext3>, String> {
    let bytes = fs::read(path).map_err(|error| {
        format!(
            "read challenge values segment failed: {}: {error}",
            path.display()
        )
    })?;
    let segment = parse_challenge_values_segment(&bytes)
        .map_err(|error| format!("parse challenge values segment failed: {error}"))?;
    segment
        .values
        .into_iter()
        .enumerate()
        .map(|(index, words)| {
            Ok(Ext3::new(
                Felt::from_canonical(words[0]).map_err(|error| {
                    format!(
                        "parse challenge values segment failed: {}: value {index} word 0 is invalid: {error}",
                        path.display()
                    )
                })?,
                Felt::from_canonical(words[1]).map_err(|error| {
                    format!(
                        "parse challenge values segment failed: {}: value {index} word 1 is invalid: {error}",
                        path.display()
                    )
                })?,
                Felt::from_canonical(words[2]).map_err(|error| {
                    format!(
                        "parse challenge values segment failed: {}: value {index} word 2 is invalid: {error}",
                        path.display()
                    )
                })?,
            ))
        })
        .collect()
}

pub(super) fn read_challenge_values_proof_segment_input(
    path: &Path,
) -> Result<ProofSegment, String> {
    let bytes = fs::read(path).map_err(|error| {
        format!(
            "read challenge values segment failed: {}: {error}",
            path.display()
        )
    })?;
    parse_challenge_values_segment(&bytes)
        .map_err(|error| format!("parse challenge values segment failed: {error}"))?;
    Ok(ProofSegment {
        id: CHALLENGE_VALUES_SEGMENT_ID,
        data: bytes,
    })
}

pub(super) fn read_packed_proof_values_segment(
    global_info: &GlobalInfo,
    path: &Path,
) -> Result<Vec<Felt>, String> {
    let bytes = fs::read(path).map_err(|error| {
        format!(
            "read proof values segment failed: {}: {error}",
            path.display()
        )
    })?;
    let segment = ProofSegment {
        id: PCS_PROOF_VALUES_SEGMENT_ID,
        data: bytes,
    };
    let values = load_pcs_proof_values_from_segments(global_info, std::slice::from_ref(&segment))
        .map_err(|error| format!("load proof values segment failed: {error}"))?;
    flatten_pcs_proof_values(global_info, &values)
        .map_err(|error| format!("flatten proof values segment failed: {error}"))
}

pub(super) fn read_group_values_segment_input(
    global_info: &GlobalInfo,
    path: &Path,
) -> Result<Vec<Ext3>, String> {
    let bytes = fs::read(path).map_err(|error| {
        format!(
            "read group values segment failed: {}: {error}",
            path.display()
        )
    })?;
    let segment = ProofSegment {
        id: GROUP_VALUES_SEGMENT_ID,
        data: bytes,
    };
    load_group_values_from_segments(global_info, std::slice::from_ref(&segment))
        .map_err(|error| format!("load group values segment failed: {error}"))
}

pub(super) fn load_batch_unit_values_inputs(
    schedule: &ProveSchedule,
    outputs: &[ProveWitnessTraceCommitments],
    unit_values_segment_input: Option<&Path>,
    shared_unit_values: &[Felt],
) -> Result<Vec<ProveUnitValues>, String> {
    if let Some(path) = unit_values_segment_input {
        let bytes = fs::read(path).map_err(|error| {
            format!(
                "read unit values segment failed: {}: {error}",
                path.display()
            )
        })?;
        let parsed = parse_unit_values_segment(&bytes)
            .map_err(|error| format!("parse unit values segment failed: {error}"))?;
        let mut values = Vec::with_capacity(parsed.units.len());
        for unit in parsed.units {
            let unit_index = usize::try_from(unit.unit_index).map_err(|_| {
                format!(
                    "unit values segment unit index does not fit usize: {}",
                    unit.unit_index
                )
            })?;
            let schedule_unit = schedule.units.get(unit_index).ok_or_else(|| {
                format!("unit values segment unit index out of range: {unit_index}")
            })?;
            let packed_values = unit
                .values
                .into_iter()
                .enumerate()
                .map(|(index, value)| {
                    Felt::from_canonical(value).map_err(|error| {
                        format!(
                            "unit values segment unit {unit_index} field word {index} is invalid: {error}"
                        )
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            values.push(ProveUnitValues {
                unit_index,
                trace_instance_index: unit.trace_instance_index,
                unit_value_map: schedule_unit.unit_value_map.clone(),
                packed_values,
            });
        }
        return Ok(values);
    }

    if shared_unit_values.is_empty() {
        return Ok(Vec::new());
    }

    outputs
        .iter()
        .map(|output| {
            let unit_index = output.commitments().unit_index();
            let unit = schedule.units.get(unit_index).ok_or_else(|| {
                format!("unit values segment unit index out of range: {unit_index}")
            })?;
            Ok(ProveUnitValues {
                unit_index,
                trace_instance_index: output.commitments().trace_instance_index(),
                unit_value_map: unit.unit_value_map.clone(),
                packed_values: shared_unit_values.to_vec(),
            })
        })
        .collect()
}

pub(super) fn read_packed_unit_values_segment_for_unit(
    schedule: &ProveSchedule,
    unit_index: usize,
    trace_instance_index: u32,
    path: &Path,
) -> Result<Vec<Felt>, String> {
    let bytes = fs::read(path).map_err(|error| {
        format!(
            "read unit values segment failed: {}: {error}",
            path.display()
        )
    })?;
    let unit = schedule
        .units
        .get(unit_index)
        .ok_or_else(|| format!("unit values segment unit index out of range: {unit_index}"))?;
    let segment = ProofSegment {
        id: UNIT_VALUES_SEGMENT_ID,
        data: bytes,
    };
    load_unit_values_for_identity_from_segments(
        unit_index,
        trace_instance_index,
        &unit.unit_value_map,
        std::slice::from_ref(&segment),
    )
    .map_err(|error| format!("load unit values segment failed: {error}"))
}

pub(super) fn read_packed_values(path: &Path, label: &str) -> Result<Vec<Felt>, String> {
    let bytes = fs::read(path)
        .map_err(|error| format!("read {label} input failed: {}: {error}", path.display()))?;
    if bytes.len() % 8 != 0 {
        return Err(format!(
            "read {label} input failed: {}: byte length is not aligned to field words",
            path.display()
        ));
    }
    bytes
        .chunks_exact(8)
        .enumerate()
        .map(|(index, chunk)| {
            let value = u64::from_le_bytes(chunk.try_into().expect("chunk length checked"));
            Felt::from_canonical(value).map_err(|error| {
                format!(
                    "read {label} input failed: {}: field word {index} is invalid: {error}",
                    path.display()
                )
            })
        })
        .collect()
}

pub(super) fn read_packed_extension_values(path: &Path, label: &str) -> Result<Vec<Ext3>, String> {
    let values = read_packed_values(path, label)?;
    if values.len() % 3 != 0 {
        return Err(format!(
            "read {label} input failed: {}: field word count is not a multiple of 3",
            path.display()
        ));
    }
    Ok(values
        .chunks_exact(3)
        .map(|chunk| Ext3::new(chunk[0], chunk[1], chunk[2]))
        .collect())
}
