use std::collections::BTreeSet;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use crate::sectioned::{
    encode_sectioned_file, parse_sectioned_file, SectionedError, SectionedFile, SectionedSection,
};

pub const TRACE_BUNDLE_KIND: [u8; 4] = *b"trb0";
pub const TRACE_BUNDLE_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceBundle {
    pub units: Vec<TraceBundleUnit>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceBundleUnit {
    pub unit_index: u32,
    pub trace_bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TraceBundleError {
    EmptyUnits,
    EmptyTraceBytes { unit_index: u32 },
    DuplicateUnitIndex { unit_index: u32 },
    ReadFailed { path: PathBuf, message: String },
    Sectioned(SectionedError),
}

impl fmt::Display for TraceBundleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyUnits => write!(f, "trace bundle has no units"),
            Self::EmptyTraceBytes { unit_index } => {
                write!(f, "trace bundle unit {unit_index} has no bytes")
            }
            Self::DuplicateUnitIndex { unit_index } => {
                write!(f, "duplicate trace bundle unit index: {unit_index}")
            }
            Self::ReadFailed { path, message } => {
                write!(
                    f,
                    "failed to read trace bundle file {}: {message}",
                    path.display()
                )
            }
            Self::Sectioned(error) => write!(f, "trace bundle file error: {error}"),
        }
    }
}

impl std::error::Error for TraceBundleError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Sectioned(error) => Some(error),
            Self::EmptyUnits
            | Self::EmptyTraceBytes { .. }
            | Self::DuplicateUnitIndex { .. }
            | Self::ReadFailed { .. } => None,
        }
    }
}

impl From<SectionedError> for TraceBundleError {
    fn from(error: SectionedError) -> Self {
        Self::Sectioned(error)
    }
}

impl TraceBundle {
    pub fn unit_count(&self) -> usize {
        self.units.len()
    }

    pub fn trace_bytes_for_unit(&self, unit_index: u32) -> Option<&[u8]> {
        self.units
            .iter()
            .find(|unit| unit.unit_index == unit_index)
            .map(|unit| unit.trace_bytes.as_slice())
    }
}

pub fn encode_trace_bundle(value: &TraceBundle) -> Result<Vec<u8>, TraceBundleError> {
    if value.units.is_empty() {
        return Err(TraceBundleError::EmptyUnits);
    }

    let mut seen = BTreeSet::new();
    let mut sections = Vec::with_capacity(value.units.len());
    for unit in &value.units {
        if unit.trace_bytes.is_empty() {
            return Err(TraceBundleError::EmptyTraceBytes {
                unit_index: unit.unit_index,
            });
        }
        if !seen.insert(unit.unit_index) {
            return Err(TraceBundleError::DuplicateUnitIndex {
                unit_index: unit.unit_index,
            });
        }
        sections.push(SectionedSection {
            id: unit.unit_index,
            data: unit.trace_bytes.clone(),
        });
    }

    let encoded = encode_sectioned_file(&SectionedFile {
        kind: TRACE_BUNDLE_KIND,
        version: TRACE_BUNDLE_VERSION,
        sections,
    })?;
    Ok(encoded)
}

pub fn parse_trace_bundle(bytes: &[u8]) -> Result<TraceBundle, TraceBundleError> {
    let parsed = parse_sectioned_file(bytes, TRACE_BUNDLE_KIND, TRACE_BUNDLE_VERSION)?;
    if parsed.sections.is_empty() {
        return Err(TraceBundleError::EmptyUnits);
    }

    let mut seen = BTreeSet::new();
    let mut units = Vec::with_capacity(parsed.sections.len());
    for section in parsed.sections {
        if section.data.is_empty() {
            return Err(TraceBundleError::EmptyTraceBytes {
                unit_index: section.id,
            });
        }
        if !seen.insert(section.id) {
            return Err(TraceBundleError::DuplicateUnitIndex {
                unit_index: section.id,
            });
        }
        units.push(TraceBundleUnit {
            unit_index: section.id,
            trace_bytes: section.data,
        });
    }

    Ok(TraceBundle { units })
}

pub fn read_trace_bundle_file(path: &Path) -> Result<TraceBundle, TraceBundleError> {
    let bytes = fs::read(path).map_err(|error| TraceBundleError::ReadFailed {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;
    parse_trace_bundle(&bytes)
}
