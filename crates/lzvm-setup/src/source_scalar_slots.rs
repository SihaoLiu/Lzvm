use std::collections::BTreeMap;
use std::fmt;

use lzvm_artifacts::expression_info::CodeOperand;
use lzvm_artifacts::global_info::PublicValue;
use lzvm_artifacts::setup_info::UnitSetupInfo;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SourceCommitmentSlot {
    id: u32,
    stage: u32,
    dimension: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SourceUnitValueSlot {
    id: u32,
    stage: u32,
    dimension: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SourceConstantSlot {
    id: u32,
    stage: u32,
    dimension: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SourcePublicSlot {
    offset: u32,
    stage: u64,
    dimension: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SourceScalarSlots {
    commitments: BTreeMap<String, SourceCommitmentSlot>,
    unit_values: BTreeMap<String, SourceUnitValueSlot>,
    constants: BTreeMap<String, SourceConstantSlot>,
    publics: BTreeMap<String, SourcePublicSlot>,
}

impl SourceScalarSlots {
    pub(crate) fn from_setup(
        setup: &UnitSetupInfo,
        public_values: &[PublicValue],
    ) -> Result<Self, SourceScalarSlotError> {
        let mut commitments = BTreeMap::new();
        for (index, column) in setup.commitment_columns.iter().enumerate() {
            commitments.insert(
                column.name.clone(),
                SourceCommitmentSlot {
                    id: usize_to_u32(index, "source commitment id overflow")?,
                    stage: column.stage,
                    dimension: column.dimension,
                },
            );
        }

        let mut unit_values = BTreeMap::new();
        for (index, value) in setup.unit_value_map.iter().enumerate() {
            unit_values.insert(
                value.name.clone(),
                SourceUnitValueSlot {
                    id: usize_to_u32(index, "source unit value id overflow")?,
                    stage: value.stage,
                    dimension: stage_value_dimension(&value.lengths)?,
                },
            );
        }

        let mut constants = BTreeMap::new();
        for column in &setup.constant_columns {
            constants.insert(
                column.name.clone(),
                SourceConstantSlot {
                    id: column.pols_map_id,
                    stage: column.stage,
                    dimension: column.dimension,
                },
            );
        }

        let mut publics = BTreeMap::new();
        let mut public_offset = 0_u32;
        for value in public_values {
            let dimension = public_value_dimension(&value.lengths)?;
            publics.insert(
                value.name.clone(),
                SourcePublicSlot {
                    offset: public_offset,
                    stage: value.stage,
                    dimension,
                },
            );
            public_offset = public_offset.checked_add(dimension).ok_or(
                SourceScalarSlotError::LengthOverflow("source public value offset overflow"),
            )?;
        }

        Ok(Self {
            commitments,
            unit_values,
            constants,
            publics,
        })
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
            if slot.stage != 1 || slot.dimension != 1 {
                return Err(SourceScalarSlotError::UnsupportedValueShape {
                    name: name.to_owned(),
                });
            }
            return Ok(CodeOperand::air_value(slot.id, Some(slot.stage), None, 1));
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
                    "source indexed constraints require commitment values: {name}"
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

fn stage_value_dimension(lengths: &[u32]) -> Result<u32, SourceScalarSlotError> {
    lengths.iter().try_fold(1_u32, |acc, length| {
        acc.checked_mul(*length)
            .ok_or(SourceScalarSlotError::LengthOverflow(
                "source unit value dimension overflow",
            ))
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
