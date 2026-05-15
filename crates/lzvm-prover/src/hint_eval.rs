use std::fmt;

use lzvm_artifacts::global_info::GlobalInfo;
use lzvm_artifacts::hint_program::{HintOperand, HintProgram};
use lzvm_field::{Ext3, Felt};

use crate::global_constraints::GlobalConstraintInputs;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedHintValue {
    pub payload: ResolvedHintPayload,
    pub positions: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedHintField {
    pub name: String,
    pub values: Vec<ResolvedHintValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedHint {
    pub name: String,
    pub fields: Vec<ResolvedHintField>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedHintPayload {
    Scalar(Felt),
    Extension(Ext3),
    Text(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HintEvalError {
    MissingHint {
        index: usize,
        count: usize,
    },
    MissingField {
        hint_index: usize,
        name: String,
    },
    UnsupportedOperand {
        operand: &'static str,
    },
    NonCanonicalNumber {
        value: u64,
    },
    SourceIndexOutOfRange {
        source: &'static str,
        index: usize,
        width: usize,
        len: usize,
    },
    GroupIndexOutOfRange {
        group_id: usize,
        group_count: usize,
    },
    LengthOverflow,
}

impl fmt::Display for HintEvalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingHint { index, count } => {
                write!(f, "hint index {index} is outside hint count {count}")
            }
            Self::MissingField { hint_index, name } => {
                write!(f, "hint {hint_index} has no field {name}")
            }
            Self::UnsupportedOperand { operand } => {
                write!(f, "unsupported hint operand: {operand}")
            }
            Self::NonCanonicalNumber { value } => {
                write!(f, "non-canonical hint number: {value}")
            }
            Self::SourceIndexOutOfRange {
                source,
                index,
                width,
                len,
            } => write!(
                f,
                "hint {source} index {index} with width {width} is outside length {len}"
            ),
            Self::GroupIndexOutOfRange {
                group_id,
                group_count,
            } => write!(
                f,
                "hint group index {group_id} is outside group count {group_count}"
            ),
            Self::LengthOverflow => write!(f, "hint evaluation length overflow"),
        }
    }
}

impl std::error::Error for HintEvalError {}

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
        offset = offset
            .checked_add(width)
            .ok_or(HintEvalError::LengthOverflow)?;
    }

    Err(HintEvalError::SourceIndexOutOfRange {
        source: "proof value",
        index: id,
        width: 1,
        len: global_info.proof_values_map.len(),
    })
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
