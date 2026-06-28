use lzvm_artifacts::global_info::{GlobalInfo, NamedStageValue};
use lzvm_artifacts::hint_program::{HintOperand, HintProgram};
use lzvm_artifacts::setup_info::{CommitmentColumn, ConstantColumn, StageValue, UnitSetupInfo};
use lzvm_field::{Ext3, Felt};

use crate::global_constraints::GlobalConstraintInputs;
use crate::group_values::load_group_values_from_segments;
use crate::pcs_query_plan::uses_transcript_pcs_query_plan_inputs;
use crate::pcs_transcript_segments::derive_pcs_transcript_challenges_from_proof_segments;
use crate::proof_values::{flatten_pcs_proof_values, load_pcs_proof_values_from_segments};
use crate::regular_constraints::{RegularColumnMatrix, RegularConstraintInputs};

use super::{
    global_hint_input_requirements, HintEvalError, ResolveGlobalHintProofSegmentsError,
    ResolveGlobalHintProofSegmentsRequest, ResolvedHint, ResolvedHintField, ResolvedHintPayload,
    ResolvedHintValue,
};

pub fn resolve_global_hint_field(
    global_info: &GlobalInfo,
    program: &HintProgram,
    hint_index: usize,
    field_name: &str,
    inputs: GlobalConstraintInputs<'_>,
) -> Result<Vec<ResolvedHintValue>, HintEvalError> {
    let hint = program
        .hints
        .get(hint_index)
        .ok_or(HintEvalError::MissingHint {
            index: hint_index,
            count: program.hints.len(),
        })?;
    let field = hint
        .fields
        .iter()
        .find(|field| field.name == field_name)
        .ok_or_else(|| HintEvalError::MissingField {
            hint_index,
            name: field_name.to_owned(),
        })?;

    field
        .values
        .iter()
        .map(|value| {
            Ok(ResolvedHintValue {
                payload: resolve_global_operand(global_info, &value.operand, inputs)?,
                positions: value.positions.clone(),
            })
        })
        .collect()
}

pub fn resolve_global_hint_program(
    global_info: &GlobalInfo,
    program: &HintProgram,
    inputs: GlobalConstraintInputs<'_>,
) -> Result<Vec<ResolvedHint>, HintEvalError> {
    program
        .hints
        .iter()
        .map(|hint| {
            let fields = hint
                .fields
                .iter()
                .map(|field| {
                    let values = field
                        .values
                        .iter()
                        .map(|value| {
                            Ok(ResolvedHintValue {
                                payload: resolve_global_operand(
                                    global_info,
                                    &value.operand,
                                    inputs,
                                )?,
                                positions: value.positions.clone(),
                            })
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    Ok(ResolvedHintField {
                        name: field.name.clone(),
                        values,
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(ResolvedHint {
                name: hint.name.clone(),
                fields,
            })
        })
        .collect()
}

pub fn resolve_global_hint_program_from_proof_segments(
    request: ResolveGlobalHintProofSegmentsRequest<'_>,
) -> Result<Vec<ResolvedHint>, ResolveGlobalHintProofSegmentsError> {
    if request.program.hints.is_empty() {
        return Ok(Vec::new());
    }

    let requirements = global_hint_input_requirements(request.program);
    let public_values = if requirements.publics {
        request.public_values
    } else {
        &[]
    };
    let packed_proof_values = if requirements.proof_values {
        let proof_values =
            load_pcs_proof_values_from_segments(request.global_info, request.segments)
                .map_err(ResolveGlobalHintProofSegmentsError::ProofValues)?;
        flatten_pcs_proof_values(request.global_info, &proof_values)
            .map_err(ResolveGlobalHintProofSegmentsError::PackedProofValues)?
    } else {
        Vec::new()
    };
    let challenges = if requirements.challenges {
        if uses_transcript_pcs_query_plan_inputs(request.segments) {
            derive_pcs_transcript_challenges_from_proof_segments(
                request.schedule,
                request.public_values,
                request.segments,
            )
            .map_err(ResolveGlobalHintProofSegmentsError::Transcript)?
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };
    let group_values = if requirements.group_values {
        load_group_values_from_segments(request.global_info, request.segments)
            .map_err(ResolveGlobalHintProofSegmentsError::GroupValues)?
    } else {
        Vec::new()
    };

    resolve_global_hint_program(
        request.global_info,
        request.program,
        GlobalConstraintInputs {
            publics: public_values,
            proof_values: &packed_proof_values,
            challenges: &challenges,
            group_values: &group_values,
        },
    )
    .map_err(ResolveGlobalHintProofSegmentsError::Eval)
}

pub fn resolve_regular_hint_field(
    setup: &UnitSetupInfo,
    program: &HintProgram,
    hint_index: usize,
    field_name: &str,
    row: usize,
    inputs: RegularConstraintInputs<'_>,
) -> Result<Vec<ResolvedHintValue>, HintEvalError> {
    validate_regular_row(row, inputs)?;
    let hint = program
        .hints
        .get(hint_index)
        .ok_or(HintEvalError::MissingHint {
            index: hint_index,
            count: program.hints.len(),
        })?;
    let field = hint
        .fields
        .iter()
        .find(|field| field.name == field_name)
        .ok_or_else(|| HintEvalError::MissingField {
            hint_index,
            name: field_name.to_owned(),
        })?;

    field
        .values
        .iter()
        .map(|value| {
            Ok(ResolvedHintValue {
                payload: resolve_regular_operand(setup, &value.operand, row, inputs)?,
                positions: value.positions.clone(),
            })
        })
        .collect()
}

pub fn resolve_regular_hint_program_for_row(
    setup: &UnitSetupInfo,
    program: &HintProgram,
    row: usize,
    inputs: RegularConstraintInputs<'_>,
) -> Result<Vec<ResolvedHint>, HintEvalError> {
    validate_regular_row(row, inputs)?;
    program
        .hints
        .iter()
        .map(|hint| {
            let fields = hint
                .fields
                .iter()
                .map(|field| {
                    let values = field
                        .values
                        .iter()
                        .map(|value| {
                            Ok(ResolvedHintValue {
                                payload: resolve_regular_operand(
                                    setup,
                                    &value.operand,
                                    row,
                                    inputs,
                                )?,
                                positions: value.positions.clone(),
                            })
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    Ok(ResolvedHintField {
                        name: field.name.clone(),
                        values,
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(ResolvedHint {
                name: hint.name.clone(),
                fields,
            })
        })
        .collect()
}

fn resolve_global_operand(
    global_info: &GlobalInfo,
    operand: &HintOperand,
    inputs: GlobalConstraintInputs<'_>,
) -> Result<ResolvedHintPayload, HintEvalError> {
    match operand {
        HintOperand::Number(value) => Ok(ResolvedHintPayload::Scalar(
            Felt::from_canonical(*value)
                .map_err(|_| HintEvalError::NonCanonicalNumber { value: *value })?,
        )),
        HintOperand::String(value) => Ok(ResolvedHintPayload::Text(value.clone())),
        HintOperand::Public { id } => Ok(ResolvedHintPayload::Scalar(read_scalar(
            "public",
            inputs.publics,
            *id as usize,
        )?)),
        HintOperand::ProofValue { id } => {
            let (offset, width) = proof_value_offset(global_info, *id as usize)?;
            if width == 1 {
                Ok(ResolvedHintPayload::Scalar(read_scalar(
                    "proof value",
                    inputs.proof_values,
                    offset,
                )?))
            } else {
                Ok(ResolvedHintPayload::Extension(read_extension_from_scalars(
                    "proof value",
                    inputs.proof_values,
                    offset,
                )?))
            }
        }
        HintOperand::GroupValue { group_id, id } => {
            Ok(ResolvedHintPayload::Extension(read_group_value(
                global_info,
                inputs.group_values,
                *group_id as usize,
                *id as usize,
            )?))
        }
        HintOperand::Challenge { id } => Ok(ResolvedHintPayload::Extension(read_extension(
            "challenge",
            inputs.challenges,
            *id as usize,
        )?)),
        HintOperand::Temporary { .. } => Err(HintEvalError::UnsupportedOperand {
            operand: "temporary",
        }),
        HintOperand::Commitment { .. } => Err(HintEvalError::UnsupportedOperand {
            operand: "commitment",
        }),
        HintOperand::CommitmentElement { .. } => Err(HintEvalError::UnsupportedOperand {
            operand: "commitment element",
        }),
        HintOperand::Constant { .. } => Err(HintEvalError::UnsupportedOperand {
            operand: "constant",
        }),
        HintOperand::CustomCommitment { .. } => Err(HintEvalError::UnsupportedOperand {
            operand: "custom commitment",
        }),
        HintOperand::AirGroupValue { .. } => Err(HintEvalError::UnsupportedOperand {
            operand: "unit group value",
        }),
        HintOperand::AirValue { .. } => Err(HintEvalError::UnsupportedOperand {
            operand: "unit value",
        }),
    }
}

fn resolve_regular_operand(
    setup: &UnitSetupInfo,
    operand: &HintOperand,
    row: usize,
    inputs: RegularConstraintInputs<'_>,
) -> Result<ResolvedHintPayload, HintEvalError> {
    match operand {
        HintOperand::Number(value) => Ok(ResolvedHintPayload::Scalar(
            Felt::from_canonical(*value)
                .map_err(|_| HintEvalError::NonCanonicalNumber { value: *value })?,
        )),
        HintOperand::String(value) => Ok(ResolvedHintPayload::Text(value.clone())),
        HintOperand::Constant {
            id,
            row_offset_index,
        } => {
            let column = find_constant_column(setup, *id)?;
            let source_row = regular_source_row(*row_offset_index as usize, row, inputs)?;
            read_regular_matrix_payload(
                "constant",
                inputs.fixed_columns,
                inputs.domain_size,
                column.stage_id as usize,
                column.dimension,
                source_row,
            )
        }
        HintOperand::Commitment {
            id,
            row_offset_index,
        } => {
            let column = find_commitment_column(setup, *id)?;
            let source_row = regular_source_row(*row_offset_index as usize, row, inputs)?;
            let matrix = find_regular_stage_columns(inputs, column.stage as u16)?;
            read_regular_matrix_payload(
                "commitment",
                matrix,
                inputs.domain_size,
                column.stage_position as usize,
                column.dimension,
                source_row,
            )
        }
        HintOperand::CommitmentElement {
            id,
            element,
            row_offset_index,
        } => {
            let column = find_commitment_column(setup, *id)?;
            if *element >= column.dimension {
                return Err(HintEvalError::SourceIndexOutOfRange {
                    source: "commitment element",
                    index: *element as usize,
                    width: 1,
                    len: column.dimension as usize,
                });
            }
            let source_row = regular_source_row(*row_offset_index as usize, row, inputs)?;
            let matrix = find_regular_stage_columns(inputs, column.stage as u16)?;
            let source_column = column
                .stage_position
                .checked_add(*element)
                .ok_or(HintEvalError::LengthOverflow)? as usize;
            Ok(ResolvedHintPayload::Scalar(read_regular_matrix_scalar(
                "commitment element",
                matrix,
                inputs.domain_size,
                source_column,
                source_row,
            )?))
        }
        HintOperand::AirValue { id } => {
            let (offset, width) =
                stage_value_offset("unit value", &setup.unit_value_map, *id as usize, true)?;
            if width == 1 {
                Ok(ResolvedHintPayload::Scalar(read_scalar(
                    "unit value",
                    inputs.unit_values,
                    offset,
                )?))
            } else {
                Ok(ResolvedHintPayload::Extension(read_extension_from_scalars(
                    "unit value",
                    inputs.unit_values,
                    offset,
                )?))
            }
        }
        HintOperand::AirGroupValue { id } => {
            let (offset, width) = stage_value_offset(
                "unit group value",
                &setup.group_value_map,
                *id as usize,
                false,
            )?;
            let value = read_extension("unit group value", inputs.group_values, offset)?;
            if width == 1 {
                Ok(ResolvedHintPayload::Scalar(value.c0))
            } else {
                Ok(ResolvedHintPayload::Extension(value))
            }
        }
        HintOperand::Challenge { id } => Ok(ResolvedHintPayload::Extension(read_extension(
            "challenge",
            inputs.challenges,
            *id as usize,
        )?)),
        HintOperand::Public { id } => Ok(ResolvedHintPayload::Scalar(read_scalar(
            "public",
            inputs.publics,
            *id as usize,
        )?)),
        HintOperand::ProofValue { id } => Ok(ResolvedHintPayload::Scalar(read_scalar(
            "proof value",
            inputs.proof_values,
            *id as usize,
        )?)),
        HintOperand::Temporary { .. } => Err(HintEvalError::UnsupportedOperand {
            operand: "temporary",
        }),
        HintOperand::CustomCommitment { .. } => Err(HintEvalError::UnsupportedOperand {
            operand: "custom commitment",
        }),
        HintOperand::GroupValue { .. } => Err(HintEvalError::UnsupportedOperand {
            operand: "group value",
        }),
    }
}

fn proof_value_offset(
    global_info: &GlobalInfo,
    id: usize,
) -> Result<(usize, usize), HintEvalError> {
    if id >= global_info.proof_values_map.len() {
        return Err(HintEvalError::SourceIndexOutOfRange {
            source: "proof value",
            index: id,
            width: 1,
            len: global_info.proof_values_map.len(),
        });
    }

    let mut offset = 0usize;
    for (index, entry) in global_info.proof_values_map.iter().enumerate() {
        let width = if entry.stage == 1 { 1 } else { 3 };
        if index == id {
            return Ok((offset, width));
        }
        let dimension = proof_value_dimension(entry)?;
        let field_width = dimension
            .checked_mul(width)
            .ok_or(HintEvalError::LengthOverflow)?;
        offset = offset
            .checked_add(field_width)
            .ok_or(HintEvalError::LengthOverflow)?;
    }

    Err(HintEvalError::SourceIndexOutOfRange {
        source: "proof value",
        index: id,
        width: 1,
        len: global_info.proof_values_map.len(),
    })
}

fn proof_value_dimension(entry: &NamedStageValue) -> Result<usize, HintEvalError> {
    entry.lengths.iter().try_fold(1_usize, |dimension, length| {
        let length = usize::try_from(*length).map_err(|_| HintEvalError::LengthOverflow)?;
        dimension
            .checked_mul(length)
            .ok_or(HintEvalError::LengthOverflow)
    })
}

fn validate_regular_row(
    row: usize,
    inputs: RegularConstraintInputs<'_>,
) -> Result<(), HintEvalError> {
    if inputs.domain_size == 0 {
        return Err(HintEvalError::EmptyDomain);
    }
    if row >= inputs.domain_size {
        return Err(HintEvalError::RowIndexOutOfRange {
            row,
            domain_size: inputs.domain_size,
        });
    }
    Ok(())
}

fn find_constant_column(setup: &UnitSetupInfo, id: u32) -> Result<&ConstantColumn, HintEvalError> {
    setup
        .constant_columns
        .iter()
        .find(|column| column.pols_map_id == id)
        .ok_or(HintEvalError::MissingColumn {
            source: "constant",
            id,
        })
}

fn find_commitment_column(
    setup: &UnitSetupInfo,
    id: u32,
) -> Result<&CommitmentColumn, HintEvalError> {
    setup
        .commitment_columns
        .iter()
        .find(|column| column.pols_map_id == id)
        .ok_or(HintEvalError::MissingColumn {
            source: "commitment",
            id,
        })
}

fn stage_value_offset(
    source: &'static str,
    map: &[StageValue],
    id: usize,
    packed_fields: bool,
) -> Result<(usize, usize), HintEvalError> {
    let mut logical_id = 0_usize;
    let mut offset = 0usize;
    for entry in map {
        let width = if entry.stage == 1 { 1 } else { 3 };
        let dimension = stage_value_dimension(entry)?;
        if id >= logical_id {
            let element = id
                .checked_sub(logical_id)
                .ok_or(HintEvalError::LengthOverflow)?;
            if element < dimension {
                let element_offset = if packed_fields {
                    element
                        .checked_mul(width)
                        .ok_or(HintEvalError::LengthOverflow)?
                } else {
                    element
                };
                return Ok((
                    offset
                        .checked_add(element_offset)
                        .ok_or(HintEvalError::LengthOverflow)?,
                    width,
                ));
            }
        }
        logical_id = logical_id
            .checked_add(dimension)
            .ok_or(HintEvalError::LengthOverflow)?;
        let field_width = if packed_fields {
            dimension
                .checked_mul(width)
                .ok_or(HintEvalError::LengthOverflow)?
        } else {
            dimension
        };
        offset = offset
            .checked_add(field_width)
            .ok_or(HintEvalError::LengthOverflow)?;
    }

    Err(HintEvalError::SourceIndexOutOfRange {
        source,
        index: id,
        width: 1,
        len: logical_id,
    })
}

fn stage_value_dimension(entry: &StageValue) -> Result<usize, HintEvalError> {
    entry.lengths.iter().try_fold(1_usize, |dimension, length| {
        let length = usize::try_from(*length).map_err(|_| HintEvalError::LengthOverflow)?;
        dimension
            .checked_mul(length)
            .ok_or(HintEvalError::LengthOverflow)
    })
}

fn find_regular_stage_columns(
    inputs: RegularConstraintInputs<'_>,
    stage_index: u16,
) -> Result<RegularColumnMatrix<'_>, HintEvalError> {
    if let Some(stage_slot) = usize::from(stage_index).checked_sub(1) {
        if let Some(stage) = inputs
            .stage_columns
            .get(stage_slot)
            .filter(|stage| stage.stage_index == stage_index)
        {
            if inputs.stage_columns[..stage_slot]
                .iter()
                .all(|stage| stage.stage_index != stage_index)
            {
                return Ok(RegularColumnMatrix {
                    column_count: stage.column_count,
                    values: stage.values,
                });
            }
        }
    }

    inputs
        .stage_columns
        .iter()
        .find(|stage| stage.stage_index == stage_index)
        .map(|stage| RegularColumnMatrix {
            column_count: stage.column_count,
            values: stage.values,
        })
        .ok_or(HintEvalError::MissingStageColumns { stage_index })
}

fn regular_source_row(
    row_offset_index: usize,
    row: usize,
    inputs: RegularConstraintInputs<'_>,
) -> Result<usize, HintEvalError> {
    let offset = inputs.opening_point_offsets.get(row_offset_index).ok_or(
        HintEvalError::SourceIndexOutOfRange {
            source: "opening point",
            index: row_offset_index,
            width: 1,
            len: inputs.opening_point_offsets.len(),
        },
    )?;
    let domain_size =
        i128::try_from(inputs.domain_size).map_err(|_| HintEvalError::LengthOverflow)?;
    let shifted =
        i128::try_from(row).map_err(|_| HintEvalError::LengthOverflow)? + i128::from(*offset);
    Ok(shifted.rem_euclid(domain_size) as usize)
}

fn read_regular_matrix_payload(
    source: &'static str,
    matrix: RegularColumnMatrix<'_>,
    domain_size: usize,
    column: usize,
    dimension: u32,
    row: usize,
) -> Result<ResolvedHintPayload, HintEvalError> {
    match dimension {
        1 => Ok(ResolvedHintPayload::Scalar(read_regular_matrix_scalar(
            source,
            matrix,
            domain_size,
            column,
            row,
        )?)),
        3 => Ok(ResolvedHintPayload::Extension(
            read_regular_matrix_extension(source, matrix, domain_size, column, row)?,
        )),
        dimension => Err(HintEvalError::UnsupportedDimension { source, dimension }),
    }
}

fn read_regular_matrix_scalar(
    source: &'static str,
    matrix: RegularColumnMatrix<'_>,
    domain_size: usize,
    column: usize,
    row: usize,
) -> Result<Felt, HintEvalError> {
    let index = regular_matrix_index(source, matrix, domain_size, column, 1, row)?;
    read_scalar(source, matrix.values, index)
}

fn read_regular_matrix_extension(
    source: &'static str,
    matrix: RegularColumnMatrix<'_>,
    domain_size: usize,
    column: usize,
    row: usize,
) -> Result<Ext3, HintEvalError> {
    let index = regular_matrix_index(source, matrix, domain_size, column, 3, row)?;
    Ok(Ext3::new(
        read_scalar(source, matrix.values, index)?,
        read_scalar(source, matrix.values, index + 1)?,
        read_scalar(source, matrix.values, index + 2)?,
    ))
}

fn regular_matrix_index(
    source: &'static str,
    matrix: RegularColumnMatrix<'_>,
    domain_size: usize,
    column: usize,
    width: usize,
    row: usize,
) -> Result<usize, HintEvalError> {
    let expected = domain_size
        .checked_mul(matrix.column_count)
        .ok_or(HintEvalError::LengthOverflow)?;
    if matrix.values.len() != expected {
        return Err(HintEvalError::MatrixLengthMismatch {
            source,
            expected,
            found: matrix.values.len(),
        });
    }
    if column
        .checked_add(width)
        .is_none_or(|end| end > matrix.column_count)
    {
        return Err(HintEvalError::SourceIndexOutOfRange {
            source,
            index: column,
            width,
            len: matrix.column_count,
        });
    }
    row.checked_mul(matrix.column_count)
        .and_then(|base| base.checked_add(column))
        .ok_or(HintEvalError::LengthOverflow)
}

fn read_group_value(
    global_info: &GlobalInfo,
    values: &[Ext3],
    group_id: usize,
    id: usize,
) -> Result<Ext3, HintEvalError> {
    let group =
        global_info
            .aggregation_types
            .get(group_id)
            .ok_or(HintEvalError::GroupIndexOutOfRange {
                group_id,
                group_count: global_info.aggregation_types.len(),
            })?;
    if id >= group.len() {
        return Err(HintEvalError::SourceIndexOutOfRange {
            source: "group value",
            index: id,
            width: 1,
            len: group.len(),
        });
    }
    let base = global_info
        .aggregation_types
        .iter()
        .take(group_id)
        .try_fold(0usize, |offset, group| {
            offset
                .checked_add(group.len())
                .ok_or(HintEvalError::LengthOverflow)
        })?;
    read_extension("group value", values, base + id)
}

fn read_scalar(source: &'static str, values: &[Felt], index: usize) -> Result<Felt, HintEvalError> {
    values
        .get(index)
        .copied()
        .ok_or(HintEvalError::SourceIndexOutOfRange {
            source,
            index,
            width: 1,
            len: values.len(),
        })
}

fn read_extension(
    source: &'static str,
    values: &[Ext3],
    index: usize,
) -> Result<Ext3, HintEvalError> {
    values
        .get(index)
        .copied()
        .ok_or(HintEvalError::SourceIndexOutOfRange {
            source,
            index,
            width: 1,
            len: values.len(),
        })
}

fn read_extension_from_scalars(
    source: &'static str,
    values: &[Felt],
    index: usize,
) -> Result<Ext3, HintEvalError> {
    if index.checked_add(3).is_some_and(|end| end <= values.len()) {
        Ok(Ext3::new(
            values[index],
            values[index + 1],
            values[index + 2],
        ))
    } else {
        Err(HintEvalError::SourceIndexOutOfRange {
            source,
            index,
            width: 3,
            len: values.len(),
        })
    }
}
