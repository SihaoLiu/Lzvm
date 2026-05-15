use std::fmt;

use lzvm_artifacts::pcs_material_segment::{
    parse_pcs_material_manifest_segment, PcsMaterialManifestSegmentError, PcsMaterialManifestUnit,
    PCS_MATERIAL_MANIFEST_SEGMENT_ID,
};
use lzvm_artifacts::proof::ProofSegment;

use crate::ProveSchedule;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidatePcsMaterialManifestSegmentsError {
    MissingSegment,
    Segment(PcsMaterialManifestSegmentError),
    UnitCountMismatch,
    UnitIndexOverflow,
    MissingUnitMaterial { unit_index: usize },
    UnitMismatch { unit_index: usize },
}

impl fmt::Display for ValidatePcsMaterialManifestSegmentsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSegment => write!(f, "missing PCS material manifest segment"),
            Self::Segment(error) => write!(f, "invalid PCS material manifest segment: {error}"),
            Self::UnitCountMismatch => write!(f, "PCS material manifest unit count mismatch"),
            Self::UnitIndexOverflow => write!(f, "PCS material manifest unit index overflow"),
            Self::MissingUnitMaterial { unit_index } => {
                write!(
                    f,
                    "setup catalog PCS material missing for unit {unit_index}"
                )
            }
            Self::UnitMismatch { unit_index } => {
                write!(f, "PCS material manifest mismatch for unit {unit_index}")
            }
        }
    }
}

impl std::error::Error for ValidatePcsMaterialManifestSegmentsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Segment(error) => Some(error),
            Self::MissingSegment
            | Self::UnitCountMismatch
            | Self::UnitIndexOverflow
            | Self::MissingUnitMaterial { .. }
            | Self::UnitMismatch { .. } => None,
        }
    }
}

pub fn validate_pcs_material_manifest_segments(
    schedule: &ProveSchedule,
    segments: &[ProofSegment],
) -> Result<(), ValidatePcsMaterialManifestSegmentsError> {
    let segment = segments
        .iter()
        .find(|segment| segment.id == PCS_MATERIAL_MANIFEST_SEGMENT_ID)
        .ok_or(ValidatePcsMaterialManifestSegmentsError::MissingSegment)?;
    let manifest = parse_pcs_material_manifest_segment(&segment.data)
        .map_err(ValidatePcsMaterialManifestSegmentsError::Segment)?;
    if manifest.units.len() != schedule.units.len() {
        return Err(ValidatePcsMaterialManifestSegmentsError::UnitCountMismatch);
    }

    for (index, (manifest_unit, schedule_unit)) in
        manifest.units.iter().zip(schedule.units.iter()).enumerate()
    {
        let expected_unit_index = u32::try_from(index)
            .map_err(|_| ValidatePcsMaterialManifestSegmentsError::UnitIndexOverflow)?;
        if manifest_unit.unit_index != expected_unit_index {
            return Err(ValidatePcsMaterialManifestSegmentsError::UnitMismatch {
                unit_index: index,
            });
        }
        validate_manifest_unit(index, manifest_unit, schedule_unit)?;
    }
    Ok(())
}

fn validate_manifest_unit(
    unit_index: usize,
    manifest: &PcsMaterialManifestUnit,
    unit: &crate::ProveUnitSchedule,
) -> Result<(), ValidatePcsMaterialManifestSegmentsError> {
    let Some(plan_digest) = unit.pcs_material_plan_digest else {
        return Err(ValidatePcsMaterialManifestSegmentsError::MissingUnitMaterial { unit_index });
    };
    let Some(fixed_column_digest) = unit.pcs_material_fixed_column_digest else {
        return Err(ValidatePcsMaterialManifestSegmentsError::MissingUnitMaterial { unit_index });
    };
    let Some(constant_tree_digest) = unit.pcs_material_constant_tree_digest else {
        return Err(ValidatePcsMaterialManifestSegmentsError::MissingUnitMaterial { unit_index });
    };
    let Some(constant_tree_root) = unit.pcs_material_constant_tree_root else {
        return Err(ValidatePcsMaterialManifestSegmentsError::MissingUnitMaterial { unit_index });
    };
    let Some(fixed_byte_count) = unit.pcs_material_fixed_byte_count else {
        return Err(ValidatePcsMaterialManifestSegmentsError::MissingUnitMaterial { unit_index });
    };
    let Some(constant_tree_byte_count) = unit.pcs_material_constant_tree_byte_count else {
        return Err(ValidatePcsMaterialManifestSegmentsError::MissingUnitMaterial { unit_index });
    };
    let Some(leaf_byte_count) = unit.pcs_material_leaf_byte_count else {
        return Err(ValidatePcsMaterialManifestSegmentsError::MissingUnitMaterial { unit_index });
    };
    let Some(node_byte_count) = unit.pcs_material_node_byte_count else {
        return Err(ValidatePcsMaterialManifestSegmentsError::MissingUnitMaterial { unit_index });
    };

    if manifest.plan_digest != plan_digest
        || manifest.fixed_column_digest != fixed_column_digest
        || manifest.constant_tree_digest != constant_tree_digest
        || manifest.constant_tree_root != constant_tree_root
        || manifest.fixed_byte_count != fixed_byte_count
        || manifest.constant_tree_byte_count != constant_tree_byte_count
        || manifest.leaf_byte_count != leaf_byte_count
        || manifest.node_byte_count != node_byte_count
    {
        return Err(ValidatePcsMaterialManifestSegmentsError::UnitMismatch { unit_index });
    }
    Ok(())
}
