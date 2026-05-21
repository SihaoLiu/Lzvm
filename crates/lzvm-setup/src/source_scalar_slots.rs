use std::collections::BTreeMap;
use std::fmt;

use lzvm_artifacts::expression_info::CodeOperand;
use lzvm_artifacts::global_info::{NamedStageValue, PublicValue};
use lzvm_artifacts::setup_info::UnitSetupInfo;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SourceChallengeSlotMetadata {
    pub(crate) name: String,
    pub(crate) id: u32,
    pub(crate) stage: u32,
    pub(crate) stage_id: u32,
    pub(crate) dimension: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SourceScalarSlotError {
    LengthOverflow(&'static str),
    UnknownValue {
        name: String,
    },
    UnsupportedValueShape {
        name: String,
    },
    UnsupportedRowOffset {
        name: String,
    },
    UnsupportedIndex {
        name: String,
    },
    IndexOutOfRange {
        name: String,
        index: u32,
        dimension: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceCommitmentSlot {
    id: u32,
    stage: u32,
    dimension: u32,
    lengths: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceUnitValueSlot {
    id: u32,
    stage: u32,
    source_dimension: u32,
    operand_dimension: u32,
    lengths: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceGroupValueSlot {
    id: u32,
    stage: u32,
    source_dimension: u32,
    operand_dimension: u32,
    lengths: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceConstantSlot {
    id: u32,
    stage: u32,
    dimension: u32,
    lengths: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SourcePublicSlot {
    offset: u32,
    stage: u64,
    dimension: u32,
    lengths: Vec<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SourceChallengeSlot {
    id: u32,
    stage: u32,
    stage_id: u32,
    dimension: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceProofValueSlot {
    offset: u32,
    stage: u32,
    source_dimension: u32,
    operand_dimension: u32,
    lengths: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SourceScalarSlots {
    commitments: BTreeMap<String, SourceCommitmentSlot>,
    unit_values: BTreeMap<String, SourceUnitValueSlot>,
    group_values: BTreeMap<String, SourceGroupValueSlot>,
    constants: BTreeMap<String, SourceConstantSlot>,
    publics: BTreeMap<String, SourcePublicSlot>,
    challenges: BTreeMap<String, SourceChallengeSlot>,
    proof_values: BTreeMap<String, SourceProofValueSlot>,
}

impl SourceScalarSlots {
    pub(crate) fn from_setup(
        setup: &UnitSetupInfo,
        public_values: &[PublicValue],
        challenge_values: &[SourceChallengeSlotMetadata],
        proof_values: &[NamedStageValue],
    ) -> Result<Self, SourceScalarSlotError> {
        let mut commitments = BTreeMap::new();
        for (index, column) in setup.commitment_columns.iter().enumerate() {
            commitments.insert(
                column.name.clone(),
                SourceCommitmentSlot {
                    id: usize_to_u32(index, "source commitment id overflow")?,
                    stage: column.stage,
                    dimension: column.dimension,
                    lengths: column.lengths.clone(),
                },
            );
        }

        let mut unit_values = BTreeMap::new();
        let mut unit_value_id = 0_u32;
        for value in &setup.unit_value_map {
            let source_dimension =
                stage_value_dimension(&value.lengths, "source unit value dimension overflow")?;
            unit_values.insert(
                value.name.clone(),
                SourceUnitValueSlot {
                    id: unit_value_id,
                    stage: value.stage,
                    source_dimension,
                    operand_dimension: if value.stage == 1 { 1 } else { 3 },
                    lengths: value.lengths.clone(),
                },
            );
            unit_value_id = unit_value_id.checked_add(source_dimension).ok_or(
                SourceScalarSlotError::LengthOverflow("source unit value id overflow"),
            )?;
        }

        let mut group_values = BTreeMap::new();
        let mut group_value_id = 0_u32;
        for value in &setup.group_value_map {
            let source_dimension =
                stage_value_dimension(&value.lengths, "source group value dimension overflow")?;
            group_values.insert(
                value.name.clone(),
                SourceGroupValueSlot {
                    id: group_value_id,
                    stage: value.stage,
                    source_dimension,
                    operand_dimension: if value.stage == 1 { 1 } else { 3 },
                    lengths: value.lengths.clone(),
                },
            );
            group_value_id = group_value_id.checked_add(source_dimension).ok_or(
                SourceScalarSlotError::LengthOverflow("source group value id overflow"),
            )?;
        }

        let mut constants = BTreeMap::new();
        for column in &setup.constant_columns {
            constants.insert(
                column.name.clone(),
                SourceConstantSlot {
                    id: column.pols_map_id,
                    stage: column.stage,
                    dimension: column.dimension,
                    lengths: column.lengths.clone(),
                },
            );
        }

        let mut publics = BTreeMap::new();
        let mut public_offset = 0_u32;
        for value in public_values {
            let dimension = public_value_dimension(&value.lengths)?;
            let lengths = public_value_lengths(&value.lengths)?;
            publics.insert(
                value.name.clone(),
                SourcePublicSlot {
                    offset: public_offset,
                    stage: value.stage,
                    dimension,
                    lengths,
                },
            );
            public_offset = public_offset.checked_add(dimension).ok_or(
                SourceScalarSlotError::LengthOverflow("source public value offset overflow"),
            )?;
        }

        let challenges = challenge_values
            .iter()
            .map(|value| {
                (
                    value.name.clone(),
                    SourceChallengeSlot {
                        id: value.id,
                        stage: value.stage,
                        stage_id: value.stage_id,
                        dimension: value.dimension,
                    },
                )
            })
            .collect();

        let mut proof_value_slots = BTreeMap::new();
        let mut proof_value_offset = 0_u32;
        for value in proof_values {
            let stage = u32::try_from(value.stage).map_err(|_| {
                SourceScalarSlotError::LengthOverflow("source proof value stage overflow")
            })?;
            let operand_dimension = if value.stage == 1 { 1 } else { 3 };
            let source_dimension = named_stage_value_dimension(&value.lengths)?;
            let lengths = named_stage_value_lengths(&value.lengths)?;
            let field_width = source_dimension.checked_mul(operand_dimension).ok_or(
                SourceScalarSlotError::LengthOverflow("source proof value offset overflow"),
            )?;
            proof_value_slots.insert(
                value.name.clone(),
                SourceProofValueSlot {
                    offset: proof_value_offset,
                    stage,
                    source_dimension,
                    operand_dimension,
                    lengths,
                },
            );
            proof_value_offset = proof_value_offset.checked_add(field_width).ok_or(
                SourceScalarSlotError::LengthOverflow("source proof value offset overflow"),
            )?;
        }

        Ok(Self {
            commitments,
            unit_values,
            group_values,
            constants,
            publics,
            challenges,
            proof_values: proof_value_slots,
        })
    }

    pub(crate) fn from_global(
        public_values: &[PublicValue],
        proof_values: &[NamedStageValue],
    ) -> Result<Self, SourceScalarSlotError> {
        let mut publics = BTreeMap::new();
        let mut public_offset = 0_u32;
        for value in public_values {
            let dimension = public_value_dimension(&value.lengths)?;
            let lengths = public_value_lengths(&value.lengths)?;
            publics.insert(
                value.name.clone(),
                SourcePublicSlot {
                    offset: public_offset,
                    stage: value.stage,
                    dimension,
                    lengths,
                },
            );
            public_offset = public_offset.checked_add(dimension).ok_or(
                SourceScalarSlotError::LengthOverflow("source public value offset overflow"),
            )?;
        }

        let mut proof_value_slots = BTreeMap::new();
        let mut proof_value_offset = 0_u32;
        for value in proof_values {
            let stage = u32::try_from(value.stage).map_err(|_| {
                SourceScalarSlotError::LengthOverflow("source proof value stage overflow")
            })?;
            let operand_dimension = if value.stage == 1 { 1 } else { 3 };
            let source_dimension = named_stage_value_dimension(&value.lengths)?;
            let lengths = named_stage_value_lengths(&value.lengths)?;
            let field_width = source_dimension.checked_mul(operand_dimension).ok_or(
                SourceScalarSlotError::LengthOverflow("source proof value offset overflow"),
            )?;
            proof_value_slots.insert(
                value.name.clone(),
                SourceProofValueSlot {
                    offset: proof_value_offset,
                    stage,
                    source_dimension,
                    operand_dimension,
                    lengths,
                },
            );
            proof_value_offset = proof_value_offset.checked_add(field_width).ok_or(
                SourceScalarSlotError::LengthOverflow("source proof value offset overflow"),
            )?;
        }

        Ok(Self {
            commitments: BTreeMap::new(),
            unit_values: BTreeMap::new(),
            group_values: BTreeMap::new(),
            constants: BTreeMap::new(),
            publics,
            challenges: BTreeMap::new(),
            proof_values: proof_value_slots,
        })
    }

    pub(crate) fn source_dimension(&self, name: &str) -> Option<u32> {
        if let Some(slot) = self.commitments.get(name) {
            return Some(slot.dimension);
        }
        if let Some(slot) = self.unit_values.get(name) {
            return Some(slot.source_dimension);
        }
        if let Some(slot) = self.group_values.get(name) {
            return Some(slot.source_dimension);
        }
        if let Some(slot) = self.constants.get(name) {
            return Some(slot.dimension);
        }
        if let Some(slot) = self.publics.get(name) {
            return Some(slot.dimension);
        }
        if let Some(slot) = self.challenges.get(name) {
            return Some(slot.dimension);
        }
        self.proof_values
            .get(name)
            .map(|slot| slot.source_dimension)
    }

    pub(crate) fn operand(&self, name: &str) -> Result<CodeOperand, SourceScalarSlotError> {
        if let Some(slot) = self.commitments.get(name) {
            if slot.stage != 1 || slot.dimension != 1 {
                return Err(SourceScalarSlotError::UnsupportedValueShape {
                    name: name.to_owned(),
                });
            }
            return Ok(CodeOperand::commitment(slot.id, 1));
        }

        if let Some(slot) = self.unit_values.get(name) {
            if slot.source_dimension != 1 {
                return Err(SourceScalarSlotError::UnsupportedValueShape {
                    name: name.to_owned(),
                });
            }
            return Ok(CodeOperand::air_value(
                slot.id,
                Some(slot.stage),
                None,
                slot.operand_dimension,
            ));
        }

        if let Some(slot) = self.group_values.get(name) {
            if slot.source_dimension != 1 {
                return Err(SourceScalarSlotError::UnsupportedValueShape {
                    name: name.to_owned(),
                });
            }
            return Ok(CodeOperand::air_group_value(
                slot.id,
                Some(slot.stage),
                None,
                slot.operand_dimension,
            ));
        }

        if let Some(slot) = self.constants.get(name) {
            if slot.stage != 0 || slot.dimension != 1 {
                return Err(SourceScalarSlotError::UnsupportedValueShape {
                    name: name.to_owned(),
                });
            }
            return Ok(CodeOperand::constant(slot.id, 1));
        }

        if let Some(slot) = self.publics.get(name) {
            if slot.stage != 1 || slot.dimension != 1 {
                return Err(SourceScalarSlotError::UnsupportedValueShape {
                    name: name.to_owned(),
                });
            }
            return Ok(CodeOperand::public(slot.offset, 1));
        }

        if let Some(slot) = self.challenges.get(name) {
            if slot.dimension != 1 {
                return Err(SourceScalarSlotError::UnsupportedValueShape {
                    name: name.to_owned(),
                });
            }
            return Ok(CodeOperand::challenge(
                slot.id,
                Some(slot.stage),
                Some(slot.stage_id),
                3,
            ));
        }

        if let Some(slot) = self.proof_values.get(name) {
            if slot.source_dimension != 1 {
                return Err(SourceScalarSlotError::UnsupportedValueShape {
                    name: name.to_owned(),
                });
            }
            return Ok(CodeOperand::proof_value_at(
                slot.offset,
                Some(slot.stage),
                slot.operand_dimension,
            ));
        }

        Err(SourceScalarSlotError::UnknownValue {
            name: name.to_owned(),
        })
    }

    pub(crate) fn operand_at(
        &self,
        name: &str,
        row_offset: i64,
    ) -> Result<CodeOperand, SourceScalarSlotError> {
        if row_offset == 0 {
            return self.operand(name);
        }

        if let Some(slot) = self.commitments.get(name) {
            if slot.stage != 1 || slot.dimension != 1 {
                return Err(SourceScalarSlotError::UnsupportedValueShape {
                    name: name.to_owned(),
                });
            }
            return Ok(CodeOperand::commitment_at(slot.id, Some(row_offset), 1));
        }

        if let Some(slot) = self.constants.get(name) {
            if slot.stage != 0 || slot.dimension != 1 {
                return Err(SourceScalarSlotError::UnsupportedValueShape {
                    name: name.to_owned(),
                });
            }
            return Ok(CodeOperand::constant_at(slot.id, Some(row_offset), 1));
        }

        Err(SourceScalarSlotError::UnsupportedRowOffset {
            name: name.to_owned(),
        })
    }

    pub(crate) fn operand_elements_at(
        &self,
        name: &str,
        row_offset: i64,
    ) -> Result<Vec<CodeOperand>, SourceScalarSlotError> {
        if let Some(slot) = self.commitments.get(name) {
            if slot.stage != 1 {
                return Err(SourceScalarSlotError::UnsupportedValueShape {
                    name: name.to_owned(),
                });
            }
            return (0..slot.dimension)
                .map(|index| self.operand_index_at(name, index, row_offset))
                .collect();
        }

        if let Some(slot) = self.publics.get(name) {
            if slot.stage != 1 {
                return Err(SourceScalarSlotError::UnsupportedValueShape {
                    name: name.to_owned(),
                });
            }
            if row_offset != 0 {
                return Err(SourceScalarSlotError::UnsupportedRowOffset {
                    name: name.to_owned(),
                });
            }
            return (0..slot.dimension)
                .map(|index| self.operand_index_at(name, index, row_offset))
                .collect();
        }

        if let Some(slot) = self.challenges.get(name) {
            if row_offset != 0 {
                return Err(SourceScalarSlotError::UnsupportedRowOffset {
                    name: name.to_owned(),
                });
            }
            return (0..slot.dimension)
                .map(|index| self.operand_index_at(name, index, row_offset))
                .collect();
        }

        if let Some(slot) = self.unit_values.get(name) {
            if row_offset != 0 {
                return Err(SourceScalarSlotError::UnsupportedRowOffset {
                    name: name.to_owned(),
                });
            }
            return (0..slot.source_dimension)
                .map(|index| self.operand_index_at(name, index, row_offset))
                .collect();
        }

        if let Some(slot) = self.group_values.get(name) {
            if row_offset != 0 {
                return Err(SourceScalarSlotError::UnsupportedRowOffset {
                    name: name.to_owned(),
                });
            }
            return (0..slot.source_dimension)
                .map(|index| self.operand_index_at(name, index, row_offset))
                .collect();
        }

        if let Some(slot) = self.constants.get(name) {
            if slot.stage != 0 {
                return Err(SourceScalarSlotError::UnsupportedValueShape {
                    name: name.to_owned(),
                });
            }
            return (0..slot.dimension)
                .map(|index| self.operand_index_at(name, index, row_offset))
                .collect();
        }

        if let Some(slot) = self.proof_values.get(name) {
            if row_offset != 0 {
                return Err(SourceScalarSlotError::UnsupportedRowOffset {
                    name: name.to_owned(),
                });
            }
            return (0..slot.source_dimension)
                .map(|index| self.operand_index_at(name, index, row_offset))
                .collect();
        }

        Ok(vec![self.operand_at(name, row_offset)?])
    }

    pub(crate) fn operand_index_at(
        &self,
        name: &str,
        index: u32,
        row_offset: i64,
    ) -> Result<CodeOperand, SourceScalarSlotError> {
        if let Some(slot) = self.commitments.get(name) {
            if slot.stage != 1 {
                return Err(SourceScalarSlotError::UnsupportedValueShape {
                    name: name.to_owned(),
                });
            }
            if index >= slot.dimension {
                return Err(SourceScalarSlotError::IndexOutOfRange {
                    name: name.to_owned(),
                    index,
                    dimension: slot.dimension,
                });
            }
            if slot.dimension == 1 {
                return self.operand_at(name, row_offset);
            }
            return Ok(CodeOperand::commitment_element_at(
                slot.id,
                index,
                (row_offset != 0).then_some(row_offset),
                1,
            ));
        }

        if let Some(slot) = self.publics.get(name) {
            if slot.stage != 1 {
                return Err(SourceScalarSlotError::UnsupportedValueShape {
                    name: name.to_owned(),
                });
            }
            if row_offset != 0 {
                return Err(SourceScalarSlotError::UnsupportedRowOffset {
                    name: name.to_owned(),
                });
            }
            if index >= slot.dimension {
                return Err(SourceScalarSlotError::IndexOutOfRange {
                    name: name.to_owned(),
                    index,
                    dimension: slot.dimension,
                });
            }
            let offset =
                slot.offset
                    .checked_add(index)
                    .ok_or(SourceScalarSlotError::LengthOverflow(
                        "source public value offset overflow",
                    ))?;
            return Ok(CodeOperand::public(offset, 1));
        }

        if let Some(slot) = self.challenges.get(name) {
            if row_offset != 0 {
                return Err(SourceScalarSlotError::UnsupportedRowOffset {
                    name: name.to_owned(),
                });
            }
            if index >= slot.dimension {
                return Err(SourceScalarSlotError::IndexOutOfRange {
                    name: name.to_owned(),
                    index,
                    dimension: slot.dimension,
                });
            }
            let id = slot
                .id
                .checked_add(index)
                .ok_or(SourceScalarSlotError::LengthOverflow(
                    "source challenge id overflow",
                ))?;
            let stage_id =
                slot.stage_id
                    .checked_add(index)
                    .ok_or(SourceScalarSlotError::LengthOverflow(
                        "source challenge stage id overflow",
                    ))?;
            return Ok(CodeOperand::challenge(
                id,
                Some(slot.stage),
                Some(stage_id),
                3,
            ));
        }

        if let Some(slot) = self.unit_values.get(name) {
            if row_offset != 0 {
                return Err(SourceScalarSlotError::UnsupportedRowOffset {
                    name: name.to_owned(),
                });
            }
            if index >= slot.source_dimension {
                return Err(SourceScalarSlotError::IndexOutOfRange {
                    name: name.to_owned(),
                    index,
                    dimension: slot.source_dimension,
                });
            }
            let id = slot
                .id
                .checked_add(index)
                .ok_or(SourceScalarSlotError::LengthOverflow(
                    "source unit value id overflow",
                ))?;
            return Ok(CodeOperand::air_value(
                id,
                Some(slot.stage),
                None,
                slot.operand_dimension,
            ));
        }

        if let Some(slot) = self.group_values.get(name) {
            if row_offset != 0 {
                return Err(SourceScalarSlotError::UnsupportedRowOffset {
                    name: name.to_owned(),
                });
            }
            if index >= slot.source_dimension {
                return Err(SourceScalarSlotError::IndexOutOfRange {
                    name: name.to_owned(),
                    index,
                    dimension: slot.source_dimension,
                });
            }
            let id = slot
                .id
                .checked_add(index)
                .ok_or(SourceScalarSlotError::LengthOverflow(
                    "source group value id overflow",
                ))?;
            return Ok(CodeOperand::air_group_value(
                id,
                Some(slot.stage),
                None,
                slot.operand_dimension,
            ));
        }

        if let Some(slot) = self.constants.get(name) {
            if slot.stage != 0 {
                return Err(SourceScalarSlotError::UnsupportedValueShape {
                    name: name.to_owned(),
                });
            }
            if index >= slot.dimension {
                return Err(SourceScalarSlotError::IndexOutOfRange {
                    name: name.to_owned(),
                    index,
                    dimension: slot.dimension,
                });
            }
            let id = slot
                .id
                .checked_add(index)
                .ok_or(SourceScalarSlotError::LengthOverflow(
                    "source constant id overflow",
                ))?;
            if row_offset == 0 {
                return Ok(CodeOperand::constant(id, 1));
            }
            return Ok(CodeOperand::constant_at(id, Some(row_offset), 1));
        }

        if let Some(slot) = self.proof_values.get(name) {
            if row_offset != 0 {
                return Err(SourceScalarSlotError::UnsupportedRowOffset {
                    name: name.to_owned(),
                });
            }
            if index >= slot.source_dimension {
                return Err(SourceScalarSlotError::IndexOutOfRange {
                    name: name.to_owned(),
                    index,
                    dimension: slot.source_dimension,
                });
            }
            let offset = index.checked_mul(slot.operand_dimension).ok_or(
                SourceScalarSlotError::LengthOverflow("source proof value offset overflow"),
            )?;
            let id =
                slot.offset
                    .checked_add(offset)
                    .ok_or(SourceScalarSlotError::LengthOverflow(
                        "source proof value offset overflow",
                    ))?;
            return Ok(CodeOperand::proof_value_at(
                id,
                Some(slot.stage),
                slot.operand_dimension,
            ));
        }

        Err(SourceScalarSlotError::UnsupportedIndex {
            name: name.to_owned(),
        })
    }

    pub(crate) fn operand_indices_at(
        &self,
        name: &str,
        indices: &[u32],
        row_offset: i64,
    ) -> Result<CodeOperand, SourceScalarSlotError> {
        if indices.len() == 1 {
            return self.operand_index_at(name, indices[0], row_offset);
        }

        if let Some(slot) = self.commitments.get(name) {
            let index = linear_source_index(name, indices, &slot.lengths)?;
            return self.operand_index_at(name, index, row_offset);
        }

        if let Some(slot) = self.constants.get(name) {
            let index = linear_source_index(name, indices, &slot.lengths)?;
            return self.operand_index_at(name, index, row_offset);
        }

        if let Some(slot) = self.publics.get(name) {
            let index = linear_source_index(name, indices, &slot.lengths)?;
            return self.operand_index_at(name, index, row_offset);
        }

        if let Some(slot) = self.unit_values.get(name) {
            let index = linear_source_index(name, indices, &slot.lengths)?;
            return self.operand_index_at(name, index, row_offset);
        }

        if let Some(slot) = self.group_values.get(name) {
            let index = linear_source_index(name, indices, &slot.lengths)?;
            return self.operand_index_at(name, index, row_offset);
        }

        if let Some(slot) = self.proof_values.get(name) {
            let index = linear_source_index(name, indices, &slot.lengths)?;
            return self.operand_index_at(name, index, row_offset);
        }

        Err(SourceScalarSlotError::UnsupportedIndex {
            name: name.to_owned(),
        })
    }
}

impl fmt::Display for SourceScalarSlotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LengthOverflow(message) => write!(f, "{message}"),
            Self::UnknownValue { name } => {
                write!(f, "source constraint references unknown value {name}")
            }
            Self::UnsupportedValueShape { name } => write!(
                f,
                "source boolean constraints require scalar source values: {name}"
            ),
            Self::UnsupportedRowOffset { name } => {
                write!(
                    f,
                    "source row offsets require commitment or fixed source values: {name}"
                )
            }
            Self::UnsupportedIndex { name } => {
                write!(
                    f,
                    "source indexed constraints require indexed source values: {name}"
                )
            }
            Self::IndexOutOfRange {
                name,
                index,
                dimension,
            } => write!(
                f,
                "source indexed constraint index {index} is outside {name} dimension {dimension}"
            ),
        }
    }
}

impl std::error::Error for SourceScalarSlotError {}

fn usize_to_u32(value: usize, message: &'static str) -> Result<u32, SourceScalarSlotError> {
    u32::try_from(value).map_err(|_| SourceScalarSlotError::LengthOverflow(message))
}

fn stage_value_dimension(
    lengths: &[u32],
    overflow_message: &'static str,
) -> Result<u32, SourceScalarSlotError> {
    lengths.iter().try_fold(1_u32, |acc, length| {
        acc.checked_mul(*length)
            .ok_or(SourceScalarSlotError::LengthOverflow(overflow_message))
    })
}

fn public_value_dimension(lengths: &[u64]) -> Result<u32, SourceScalarSlotError> {
    let dimension = lengths.iter().try_fold(1_u64, |acc, length| {
        acc.checked_mul(*length)
            .ok_or(SourceScalarSlotError::LengthOverflow(
                "source public value dimension overflow",
            ))
    })?;
    u32::try_from(dimension).map_err(|_| {
        SourceScalarSlotError::LengthOverflow("source public value dimension overflow")
    })
}

fn public_value_lengths(lengths: &[u64]) -> Result<Vec<u32>, SourceScalarSlotError> {
    lengths
        .iter()
        .map(|length| {
            u32::try_from(*length).map_err(|_| {
                SourceScalarSlotError::LengthOverflow("source public value dimension overflow")
            })
        })
        .collect()
}

fn named_stage_value_dimension(lengths: &[u64]) -> Result<u32, SourceScalarSlotError> {
    let dimension = lengths.iter().try_fold(1_u64, |acc, length| {
        acc.checked_mul(*length)
            .ok_or(SourceScalarSlotError::LengthOverflow(
                "source stage value dimension overflow",
            ))
    })?;
    u32::try_from(dimension)
        .map_err(|_| SourceScalarSlotError::LengthOverflow("source stage value dimension overflow"))
}

fn named_stage_value_lengths(lengths: &[u64]) -> Result<Vec<u32>, SourceScalarSlotError> {
    lengths
        .iter()
        .map(|length| {
            u32::try_from(*length).map_err(|_| {
                SourceScalarSlotError::LengthOverflow("source stage value dimension overflow")
            })
        })
        .collect()
}

fn linear_source_index(
    name: &str,
    indices: &[u32],
    lengths: &[u32],
) -> Result<u32, SourceScalarSlotError> {
    if indices.is_empty() || indices.len() != lengths.len() {
        return Err(SourceScalarSlotError::UnsupportedIndex {
            name: name.to_owned(),
        });
    }

    indices
        .iter()
        .zip(lengths)
        .try_fold(0_u32, |acc, (index, length)| {
            if index >= length {
                return Err(SourceScalarSlotError::IndexOutOfRange {
                    name: name.to_owned(),
                    index: *index,
                    dimension: *length,
                });
            }
            acc.checked_mul(*length)
                .and_then(|base| base.checked_add(*index))
                .ok_or(SourceScalarSlotError::LengthOverflow(
                    "source indexed value offset overflow",
                ))
        })
}
