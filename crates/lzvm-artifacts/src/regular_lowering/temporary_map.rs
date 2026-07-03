use std::collections::{BTreeMap, BTreeSet};

use crate::expression_info::{CodeDestination, CodeOperand, CodeOperation};

use super::{usize_to_u32, RegularProgramLoweringError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TemporaryMap {
    one: BTreeMap<u32, u32>,
    three: BTreeMap<u32, u32>,
    pub(super) count1: u32,
    pub(super) count3: u32,
}

impl TemporaryMap {
    pub(super) fn build(operations: &[CodeOperation]) -> Result<Self, RegularProgramLoweringError> {
        let mut one = Vec::new();
        let mut three = Vec::new();
        let mut defined = BTreeSet::new();
        for (index, operation) in operations.iter().enumerate() {
            for operand in &operation.sources {
                observe_operand(operand, index, &mut one, &mut three)?;
                if let CodeOperand::Temporary { id, dimension } = operand {
                    if !defined.contains(&(*id, *dimension)) {
                        return Err(RegularProgramLoweringError::TemporaryReadBeforeWrite {
                            id: *id,
                            dimension: *dimension,
                            operation_index: index,
                        });
                    }
                }
            }
            observe_destination(&operation.destination, index, &mut one, &mut three)?;
            if let CodeDestination::Temporary { id, dimension } = &operation.destination {
                defined.insert((*id, *dimension));
            }
        }
        let one = compact_segments(one)?;
        let three = compact_segments(three)?;
        Ok(Self {
            count1: usize_to_u32(one.values().copied().collect::<BTreeSet<_>>().len())?,
            count3: usize_to_u32(three.values().copied().collect::<BTreeSet<_>>().len())?,
            one,
            three,
        })
    }

    pub(super) fn compact_id(
        &self,
        id: u32,
        dimension: u32,
    ) -> Result<u32, RegularProgramLoweringError> {
        let map = match dimension {
            1 => &self.one,
            3 => &self.three,
            dimension => {
                return Err(RegularProgramLoweringError::UnsupportedDimension { dimension });
            }
        };
        map.get(&id)
            .copied()
            .ok_or(RegularProgramLoweringError::MissingTemporary { id, dimension })
    }
}

fn observe_destination(
    destination: &CodeDestination,
    index: usize,
    one: &mut Vec<Segment>,
    three: &mut Vec<Segment>,
) -> Result<(), RegularProgramLoweringError> {
    if let CodeDestination::Temporary { id, dimension } = destination {
        observe_temporary(*id, *dimension, index, one, three)?;
    }
    Ok(())
}

fn observe_operand(
    operand: &CodeOperand,
    index: usize,
    one: &mut Vec<Segment>,
    three: &mut Vec<Segment>,
) -> Result<(), RegularProgramLoweringError> {
    if let CodeOperand::Temporary { id, dimension } = operand {
        observe_temporary(*id, *dimension, index, one, three)?;
    }
    Ok(())
}

fn observe_temporary(
    id: u32,
    dimension: u32,
    index: usize,
    one: &mut Vec<Segment>,
    three: &mut Vec<Segment>,
) -> Result<(), RegularProgramLoweringError> {
    let segments = match dimension {
        1 => one,
        3 => three,
        dimension => {
            return Err(RegularProgramLoweringError::UnsupportedDimension { dimension });
        }
    };
    if let Some(segment) = segments.iter_mut().find(|segment| segment.id == id) {
        segment.end = segment.end.max(index);
    } else {
        segments.push(Segment {
            start: index,
            end: index,
            id,
        });
    }
    Ok(())
}

fn compact_segments(
    mut segments: Vec<Segment>,
) -> Result<BTreeMap<u32, u32>, RegularProgramLoweringError> {
    segments.sort_by_key(|segment| (segment.end, segment.start, segment.id));
    let mut subsets: Vec<Vec<Segment>> = Vec::new();
    for segment in segments {
        let mut closest_subset = None;
        let mut min_distance = usize::MAX;
        for (index, subset) in subsets.iter().enumerate() {
            let last = subset
                .last()
                .ok_or(RegularProgramLoweringError::LengthOverflow)?;
            if segments_intersect(segment, *last) {
                continue;
            }
            let distance = last.end.abs_diff(segment.start);
            if distance < min_distance {
                min_distance = distance;
                closest_subset = Some(index);
            }
        }
        if let Some(index) = closest_subset {
            subsets[index].push(segment);
        } else {
            subsets.push(vec![segment]);
        }
    }

    let mut out = BTreeMap::new();
    for (index, subset) in subsets.iter().enumerate() {
        let compact_id = usize_to_u32(index)?;
        for segment in subset {
            out.insert(segment.id, compact_id);
        }
    }
    Ok(out)
}

fn segments_intersect(left: Segment, right: Segment) -> bool {
    right.start < left.end && left.start < right.end
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Segment {
    start: usize,
    end: usize,
    id: u32,
}
