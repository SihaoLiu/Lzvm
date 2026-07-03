use std::collections::{BTreeMap, BTreeSet};
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
    pub(crate) lengths: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SourceScalarSlotError {
    LengthOverflow(&'static str),
    UnknownValue {
        name: String,
    },
    DuplicateValueName {
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceChallengeSlot {
    id: u32,
    stage: u32,
    stage_id: u32,
    dimension: u32,
    lengths: Vec<u32>,
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
            insert_source_scalar_slot(
                &mut commitments,
                column.name.clone(),
                SourceCommitmentSlot {
                    id: usize_to_u32(index, "source commitment id overflow")?,
                    stage: column.stage,
                    dimension: column.dimension,
                    lengths: column.lengths.clone(),
                },
            )?;
        }

        let mut unit_values = BTreeMap::new();
        let mut unit_value_id = 0_u32;
        for value in &setup.unit_value_map {
            let source_dimension =
                stage_value_dimension(&value.lengths, "source unit value dimension overflow")?;
            insert_source_scalar_slot(
                &mut unit_values,
                value.name.clone(),
                SourceUnitValueSlot {
                    id: unit_value_id,
                    stage: value.stage,
                    source_dimension,
                    operand_dimension: if value.stage == 1 { 1 } else { 3 },
                    lengths: value.lengths.clone(),
                },
            )?;
            unit_value_id = unit_value_id.checked_add(source_dimension).ok_or(
                SourceScalarSlotError::LengthOverflow("source unit value id overflow"),
            )?;
        }

        let mut group_values = BTreeMap::new();
        let mut group_value_id = 0_u32;
        for value in &setup.group_value_map {
            let source_dimension =
                stage_value_dimension(&value.lengths, "source group value dimension overflow")?;
            insert_source_scalar_slot(
                &mut group_values,
                value.name.clone(),
                SourceGroupValueSlot {
                    id: group_value_id,
                    stage: value.stage,
                    source_dimension,
                    operand_dimension: if value.stage == 1 { 1 } else { 3 },
                    lengths: value.lengths.clone(),
                },
            )?;
            group_value_id = group_value_id.checked_add(source_dimension).ok_or(
                SourceScalarSlotError::LengthOverflow("source group value id overflow"),
            )?;
        }

        let mut constants = BTreeMap::new();
        for column in &setup.constant_columns {
            insert_source_scalar_slot(
                &mut constants,
                column.name.clone(),
                SourceConstantSlot {
                    id: column.pols_map_id,
                    stage: column.stage,
                    dimension: column.dimension,
                    lengths: column.lengths.clone(),
                },
            )?;
        }

        let mut publics = BTreeMap::new();
        let mut public_offset = 0_u32;
        for value in public_values {
            let dimension = public_value_dimension(&value.lengths)?;
            let lengths = public_value_lengths(&value.lengths)?;
            insert_source_scalar_slot(
                &mut publics,
                value.name.clone(),
                SourcePublicSlot {
                    offset: public_offset,
                    stage: value.stage,
                    dimension,
                    lengths,
                },
            )?;
            public_offset = public_offset.checked_add(dimension).ok_or(
                SourceScalarSlotError::LengthOverflow("source public value offset overflow"),
            )?;
        }

        let mut challenges = BTreeMap::new();
        for value in challenge_values {
            insert_source_scalar_slot(
                &mut challenges,
                value.name.clone(),
                SourceChallengeSlot {
                    id: value.id,
                    stage: value.stage,
                    stage_id: value.stage_id,
                    dimension: value.dimension,
                    lengths: value.lengths.clone(),
                },
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
            insert_source_scalar_slot(
                &mut proof_value_slots,
                value.name.clone(),
                SourceProofValueSlot {
                    offset: proof_value_offset,
                    stage,
                    source_dimension,
                    operand_dimension,
                    lengths,
                },
            )?;
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
            insert_source_scalar_slot(
                &mut publics,
                value.name.clone(),
                SourcePublicSlot {
                    offset: public_offset,
                    stage: value.stage,
                    dimension,
                    lengths,
                },
            )?;
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
            insert_source_scalar_slot(
                &mut proof_value_slots,
                value.name.clone(),
                SourceProofValueSlot {
                    offset: proof_value_offset,
                    stage,
                    source_dimension,
                    operand_dimension,
                    lengths,
                },
            )?;
            proof_value_offset = proof_value_offset.checked_add(field_width).ok_or(
                SourceScalarSlotError::LengthOverflow("source proof value offset overflow"),
            )?;
        }

        let mut names = BTreeSet::new();
        insert_source_scalar_names(&mut names, publics.keys())?;
        insert_source_scalar_names(&mut names, proof_value_slots.keys())?;

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
        if self.source_slot_match_count(name) > 1 {
            return None;
        }
        let allow_local = self.exact_source_slot_count(name) == 0;
        if let Some(slot) = source_slot_get(&self.commitments, name, allow_local) {
            return Some(slot.dimension);
        }
        if let Some(slot) = source_slot_get(&self.unit_values, name, allow_local) {
            return Some(slot.source_dimension);
        }
        if let Some(slot) = source_slot_get(&self.group_values, name, allow_local) {
            return Some(slot.source_dimension);
        }
        if let Some(slot) = source_slot_get(&self.constants, name, allow_local) {
            return Some(slot.dimension);
        }
        if let Some(slot) = source_slot_get(&self.publics, name, allow_local) {
            return Some(slot.dimension);
        }
        if let Some(slot) = source_slot_get(&self.challenges, name, allow_local) {
            return Some(slot.dimension);
        }
        source_slot_get(&self.proof_values, name, allow_local).map(|slot| slot.source_dimension)
    }

    pub(crate) fn operand(&self, name: &str) -> Result<CodeOperand, SourceScalarSlotError> {
        let allow_local = self.source_slot_lookup_uses_local(name)?;
        if let Some(slot) = source_slot_get(&self.commitments, name, allow_local) {
            if !slot.lengths.is_empty() {
                return Err(SourceScalarSlotError::UnsupportedValueShape {
                    name: name.to_owned(),
                });
            }
            return Ok(CodeOperand::commitment(slot.id, slot.dimension));
        }

        if let Some(slot) = source_slot_get(&self.unit_values, name, allow_local) {
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

        if let Some(slot) = source_slot_get(&self.group_values, name, allow_local) {
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

        if let Some(slot) = source_slot_get(&self.constants, name, allow_local) {
            if slot.stage != 0 || slot.dimension != 1 {
                return Err(SourceScalarSlotError::UnsupportedValueShape {
                    name: name.to_owned(),
                });
            }
            return Ok(CodeOperand::constant(slot.id, 1));
        }

        if let Some(slot) = source_slot_get(&self.publics, name, allow_local) {
            if slot.stage != 1 || slot.dimension != 1 {
                return Err(SourceScalarSlotError::UnsupportedValueShape {
                    name: name.to_owned(),
                });
            }
            return Ok(CodeOperand::public(slot.offset, 1));
        }

        if let Some(slot) = source_slot_get(&self.challenges, name, allow_local) {
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

        if let Some(slot) = source_slot_get(&self.proof_values, name, allow_local) {
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

        let allow_local = self.source_slot_lookup_uses_local(name)?;
        if let Some(slot) = source_slot_get(&self.commitments, name, allow_local) {
            if slot.stage != 1 || slot.dimension != 1 {
                return Err(SourceScalarSlotError::UnsupportedValueShape {
                    name: name.to_owned(),
                });
            }
            return Ok(CodeOperand::commitment_at(slot.id, Some(row_offset), 1));
        }

        if let Some(slot) = source_slot_get(&self.constants, name, allow_local) {
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
        let allow_local = self.source_slot_lookup_uses_local(name)?;
        if let Some(slot) = source_slot_get(&self.commitments, name, allow_local) {
            if slot.stage != 1 {
                return Err(SourceScalarSlotError::UnsupportedValueShape {
                    name: name.to_owned(),
                });
            }
            return (0..slot.dimension)
                .map(|index| self.operand_index_at(name, index, row_offset))
                .collect();
        }

        if let Some(slot) = source_slot_get(&self.publics, name, allow_local) {
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

        if let Some(slot) = source_slot_get(&self.challenges, name, allow_local) {
            if row_offset != 0 {
                return Err(SourceScalarSlotError::UnsupportedRowOffset {
                    name: name.to_owned(),
                });
            }
            return (0..slot.dimension)
                .map(|index| self.operand_index_at(name, index, row_offset))
                .collect();
        }

        if let Some(slot) = source_slot_get(&self.unit_values, name, allow_local) {
            if row_offset != 0 {
                return Err(SourceScalarSlotError::UnsupportedRowOffset {
                    name: name.to_owned(),
                });
            }
            return (0..slot.source_dimension)
                .map(|index| self.operand_index_at(name, index, row_offset))
                .collect();
        }

        if let Some(slot) = source_slot_get(&self.group_values, name, allow_local) {
            if row_offset != 0 {
                return Err(SourceScalarSlotError::UnsupportedRowOffset {
                    name: name.to_owned(),
                });
            }
            return (0..slot.source_dimension)
                .map(|index| self.operand_index_at(name, index, row_offset))
                .collect();
        }

        if let Some(slot) = source_slot_get(&self.constants, name, allow_local) {
            if slot.stage != 0 {
                return Err(SourceScalarSlotError::UnsupportedValueShape {
                    name: name.to_owned(),
                });
            }
            return (0..slot.dimension)
                .map(|index| self.operand_index_at(name, index, row_offset))
                .collect();
        }

        if let Some(slot) = source_slot_get(&self.proof_values, name, allow_local) {
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
        let allow_local = self.source_slot_lookup_uses_local(name)?;
        if let Some(slot) = source_slot_get(&self.commitments, name, allow_local) {
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
                return Ok(CodeOperand::commitment_at(
                    slot.id,
                    (row_offset != 0).then_some(row_offset),
                    1,
                ));
            }
            return Ok(CodeOperand::commitment_element_at(
                slot.id,
                index,
                (row_offset != 0).then_some(row_offset),
                1,
            ));
        }

        if let Some(slot) = source_slot_get(&self.publics, name, allow_local) {
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

        if let Some(slot) = source_slot_get(&self.challenges, name, allow_local) {
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

        if let Some(slot) = source_slot_get(&self.unit_values, name, allow_local) {
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

        if let Some(slot) = source_slot_get(&self.group_values, name, allow_local) {
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

        if let Some(slot) = source_slot_get(&self.constants, name, allow_local) {
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

        if let Some(slot) = source_slot_get(&self.proof_values, name, allow_local) {
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

        let allow_local = self.source_slot_lookup_uses_local(name)?;
        if let Some(slot) = source_slot_get(&self.commitments, name, allow_local) {
            let index = linear_source_index(name, indices, &slot.lengths)?;
            return self.operand_index_at(name, index, row_offset);
        }

        if let Some(slot) = source_slot_get(&self.constants, name, allow_local) {
            let index = linear_source_index(name, indices, &slot.lengths)?;
            return self.operand_index_at(name, index, row_offset);
        }

        if let Some(slot) = source_slot_get(&self.publics, name, allow_local) {
            let index = linear_source_index(name, indices, &slot.lengths)?;
            return self.operand_index_at(name, index, row_offset);
        }

        if let Some(slot) = source_slot_get(&self.unit_values, name, allow_local) {
            let index = linear_source_index(name, indices, &slot.lengths)?;
            return self.operand_index_at(name, index, row_offset);
        }

        if let Some(slot) = source_slot_get(&self.group_values, name, allow_local) {
            let index = linear_source_index(name, indices, &slot.lengths)?;
            return self.operand_index_at(name, index, row_offset);
        }

        if let Some(slot) = source_slot_get(&self.proof_values, name, allow_local) {
            let index = linear_source_index(name, indices, &slot.lengths)?;
            return self.operand_index_at(name, index, row_offset);
        }

        if let Some(slot) = source_slot_get(&self.challenges, name, allow_local) {
            let index = linear_source_index(name, indices, &slot.lengths)?;
            return self.operand_index_at(name, index, row_offset);
        }

        Err(SourceScalarSlotError::UnsupportedIndex {
            name: name.to_owned(),
        })
    }

    fn source_slot_lookup_uses_local(&self, name: &str) -> Result<bool, SourceScalarSlotError> {
        let match_count = self.source_slot_match_count(name);
        if match_count > 1 {
            return Err(SourceScalarSlotError::DuplicateValueName {
                name: name.to_owned(),
            });
        }
        Ok(self.exact_source_slot_count(name) == 0)
    }

    fn source_slot_match_count(&self, name: &str) -> usize {
        let exact_count = self.exact_source_slot_count(name);
        if exact_count > 0 {
            return exact_count;
        }
        [
            source_slot_get(&self.commitments, name, true).is_some(),
            source_slot_get(&self.unit_values, name, true).is_some(),
            source_slot_get(&self.group_values, name, true).is_some(),
            source_slot_get(&self.constants, name, true).is_some(),
            source_slot_get(&self.publics, name, true).is_some(),
            source_slot_get(&self.challenges, name, true).is_some(),
            source_slot_get(&self.proof_values, name, true).is_some(),
        ]
        .into_iter()
        .filter(|matched| *matched)
        .count()
    }

    fn exact_source_slot_count(&self, name: &str) -> usize {
        [
            self.commitments.contains_key(name),
            self.unit_values.contains_key(name),
            self.group_values.contains_key(name),
            self.constants.contains_key(name),
            self.publics.contains_key(name),
            self.challenges.contains_key(name),
            self.proof_values.contains_key(name),
        ]
        .into_iter()
        .filter(|matched| *matched)
        .count()
    }
}

impl fmt::Display for SourceScalarSlotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LengthOverflow(message) => write!(f, "{message}"),
            Self::UnknownValue { name } => {
                write!(f, "source constraint references unknown value {name}")
            }
            Self::DuplicateValueName { name } => {
                write!(f, "duplicate source constraint value name {name}")
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

fn insert_source_scalar_names<'a>(
    seen: &mut BTreeSet<&'a str>,
    names: impl IntoIterator<Item = &'a String>,
) -> Result<(), SourceScalarSlotError> {
    for name in names {
        if !seen.insert(name.as_str()) {
            return Err(SourceScalarSlotError::DuplicateValueName { name: name.clone() });
        }
    }
    Ok(())
}

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

fn insert_source_scalar_slot<T: Clone>(
    slots: &mut BTreeMap<String, T>,
    name: String,
    slot: T,
) -> Result<(), SourceScalarSlotError> {
    let local_name = name
        .rsplit_once('.')
        .map(|(_, local_name)| local_name)
        .filter(|local_name| !local_name.is_empty())
        .map(str::to_owned);
    if slots.contains_key(&name) {
        return Err(SourceScalarSlotError::DuplicateValueName { name });
    }
    if let Some(local_name) = local_name.as_ref() {
        if slots.contains_key(local_name) {
            return Err(SourceScalarSlotError::DuplicateValueName {
                name: local_name.clone(),
            });
        }
    }
    slots.insert(name, slot.clone());
    if let Some(local_name) = local_name {
        slots.insert(local_name, slot);
    }
    Ok(())
}

fn source_slot_get<'a, T>(
    slots: &'a BTreeMap<String, T>,
    name: &str,
    allow_local: bool,
) -> Option<&'a T> {
    slots.get(name).or_else(|| {
        if !allow_local {
            return None;
        }
        name.rsplit_once('.')
            .map(|(_, local_name)| local_name)
            .filter(|local_name| !local_name.is_empty())
            .and_then(|local_name| slots.get(local_name))
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

#[cfg(test)]
mod tests {
    use super::*;

    fn slots_with_unit_and_group_local_collision() -> SourceScalarSlots {
        let mut unit_values = BTreeMap::new();
        insert_source_scalar_slot(
            &mut unit_values,
            "air.late".to_owned(),
            SourceUnitValueSlot {
                id: 0,
                stage: 1,
                source_dimension: 1,
                operand_dimension: 1,
                lengths: Vec::new(),
            },
        )
        .expect("unit value should insert");
        let mut group_values = BTreeMap::new();
        insert_source_scalar_slot(
            &mut group_values,
            "group.late".to_owned(),
            SourceGroupValueSlot {
                id: 0,
                stage: 1,
                source_dimension: 1,
                operand_dimension: 1,
                lengths: Vec::new(),
            },
        )
        .expect("group value should insert");

        SourceScalarSlots {
            commitments: BTreeMap::new(),
            unit_values,
            group_values,
            constants: BTreeMap::new(),
            publics: BTreeMap::new(),
            challenges: BTreeMap::new(),
            proof_values: BTreeMap::new(),
        }
    }

    #[test]
    fn rejects_duplicate_local_slot_names() {
        let mut slots = BTreeMap::new();
        insert_source_scalar_slot(&mut slots, "left.value".to_owned(), 1_u32)
            .expect("first slot should insert");

        assert_eq!(
            insert_source_scalar_slot(&mut slots, "right.value".to_owned(), 2_u32),
            Err(SourceScalarSlotError::DuplicateValueName {
                name: "value".to_owned(),
            })
        );
    }

    #[test]
    fn rejects_ambiguous_local_names_at_lookup_time() {
        let slots = slots_with_unit_and_group_local_collision();

        assert_eq!(
            slots.operand("late"),
            Err(SourceScalarSlotError::DuplicateValueName {
                name: "late".to_owned(),
            })
        );
    }

    #[test]
    fn resolves_qualified_names_when_local_names_collide() {
        let slots = slots_with_unit_and_group_local_collision();

        assert!(slots.operand("air.late").is_ok());
        assert!(slots.operand("group.late").is_ok());
    }

    #[test]
    fn rejects_global_public_and_proof_local_name_collisions() {
        let publics = vec![PublicValue {
            name: "publics.value".to_owned(),
            stage: 1,
            lengths: Vec::new(),
        }];
        let proof_values = vec![NamedStageValue {
            name: "proof.value".to_owned(),
            stage: 1,
            id: None,
            lengths: Vec::new(),
        }];

        assert_eq!(
            SourceScalarSlots::from_global(&publics, &proof_values),
            Err(SourceScalarSlotError::DuplicateValueName {
                name: "value".to_owned(),
            })
        );
    }
}
