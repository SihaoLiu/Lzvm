use std::fmt;

use lzvm_artifacts::key_directory::KeyUnitKind;
use lzvm_artifacts::pcs_material_segment::{
    encode_pcs_material_manifest_segment, parse_pcs_material_manifest_segment,
    PcsMaterialManifestSegment, PcsMaterialManifestSegmentError, PcsMaterialManifestUnit,
    PCS_MATERIAL_MANIFEST_SEGMENT_ID,
};
use lzvm_artifacts::proof::ProofSegment;

use crate::ProveSchedule;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProvePcsMaterialSegmentError {
    MissingMaterial {
        unit_index: usize,
        kind: KeyUnitKind,
    },
    UnitIndexOverflow {
        unit_index: usize,
    },
    Segment(PcsMaterialManifestSegmentError),
}

impl fmt::Display for ProvePcsMaterialSegmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingMaterial { unit_index, kind } => write!(
                f,
                "prove PCS material segment is missing material for unit {unit_index} ({kind})"
            ),
            Self::UnitIndexOverflow { unit_index } => {
                write!(
                    f,
                    "prove PCS material segment unit index does not fit u32: {unit_index}"
                )
            }
            Self::Segment(error) => write!(f, "prove PCS material segment encode failed: {error}"),
        }
    }
}

impl std::error::Error for ProvePcsMaterialSegmentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Segment(error) => Some(error),
            Self::MissingMaterial { .. } | Self::UnitIndexOverflow { .. } => None,
        }
    }
}

impl From<PcsMaterialManifestSegmentError> for ProvePcsMaterialSegmentError {
    fn from(error: PcsMaterialManifestSegmentError) -> Self {
        Self::Segment(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidatePcsMaterialManifestSegmentsError {
    MissingSegment,
    DuplicateSegment,
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
            Self::DuplicateSegment => write!(f, "duplicate PCS material manifest segment"),
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
            | Self::DuplicateSegment
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
    let mut matching_segments = segments
        .iter()
        .filter(|segment| segment.id == PCS_MATERIAL_MANIFEST_SEGMENT_ID);
    let segment = matching_segments
        .next()
        .ok_or(ValidatePcsMaterialManifestSegmentsError::MissingSegment)?;
    if matching_segments.next().is_some() {
        return Err(ValidatePcsMaterialManifestSegmentsError::DuplicateSegment);
    }
    let manifest = parse_pcs_material_manifest_segment(&segment.data)
        .map_err(ValidatePcsMaterialManifestSegmentsError::Segment)?;
    validate_parsed_pcs_material_manifest_matches_schedule(schedule, &manifest)
}

pub(crate) fn validate_parsed_pcs_material_manifest_matches_schedule(
    schedule: &ProveSchedule,
    manifest: &PcsMaterialManifestSegment,
) -> Result<(), ValidatePcsMaterialManifestSegmentsError> {
    for manifest_unit in &manifest.units {
        let unit_index = usize::try_from(manifest_unit.unit_index)
            .map_err(|_| ValidatePcsMaterialManifestSegmentsError::UnitIndexOverflow)?;
        if unit_index >= schedule.units.len() {
            return Err(ValidatePcsMaterialManifestSegmentsError::UnitMismatch { unit_index });
        }
    }
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
