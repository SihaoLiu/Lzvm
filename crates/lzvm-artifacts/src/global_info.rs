use std::fmt;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobalInfo {
    pub name: String,
    pub air_groups: Vec<String>,
    pub airs: Vec<Vec<GlobalAir>>,
    pub curve: CurveKind,
    pub lattice_size: Option<u64>,
    pub aggregation_types: Vec<Vec<AggregationType>>,
    pub n_publics: u64,
    pub num_challenges: Vec<u64>,
    pub num_proof_values: Vec<u64>,
    pub proof_values_map: Vec<NamedStageValue>,
    pub publics_map: Vec<PublicValue>,
    pub transcript_arity: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobalAir {
    pub name: String,
    pub num_rows: u64,
    pub has_compressor: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AggregationType {
    pub aggregation_type: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedStageValue {
    pub name: String,
    pub stage: u64,
    pub id: Option<u64>,
    pub lengths: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicValue {
    pub name: String,
    pub stage: u64,
    pub lengths: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CurveKind {
    None,
    EcGfp5,
    EcMasFp5,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GlobalInfoError {
    Json {
        message: String,
    },
    MissingField {
        field: &'static str,
    },
    InvalidField {
        field: &'static str,
    },
    UnknownCurve {
        curve: String,
    },
    AirGroupCountMismatch {
        air_groups: usize,
        airs: usize,
        aggregation_types: usize,
    },
    EmptyAirGroup {
        airgroup_id: usize,
    },
    InvalidRowCount {
        airgroup_id: usize,
        air_id: usize,
    },
    InvalidStage {
        field: &'static str,
        index: usize,
    },
    PublicCountMismatch {
        expected: u64,
        found: usize,
    },
    InvalidTranscriptArity,
    Io {
        message: String,
    },
}

impl fmt::Display for GlobalInfoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json { message } => write!(f, "global-info json error: {message}"),
            Self::MissingField { field } => write!(f, "missing global-info field: {field}"),
            Self::InvalidField { field } => write!(f, "invalid global-info field: {field}"),
            Self::UnknownCurve { curve } => write!(f, "unknown global-info curve: {curve}"),
            Self::AirGroupCountMismatch {
                air_groups,
                airs,
                aggregation_types,
            } => write!(
                f,
                "global-info air group count mismatch: air_groups {air_groups}, airs {airs}, aggregation_types {aggregation_types}"
            ),
            Self::EmptyAirGroup { airgroup_id } => {
                write!(f, "global-info air group {airgroup_id} has no units")
            }
            Self::InvalidRowCount {
                airgroup_id,
                air_id,
            } => write!(
                f,
                "global-info unit row count is invalid at group {airgroup_id}, unit {air_id}"
            ),
            Self::InvalidStage { field, index } => {
                write!(f, "global-info {field} stage is invalid at index {index}")
            }
            Self::PublicCountMismatch { expected, found } => write!(
                f,
                "global-info public count mismatch: expected {expected}, found {found}"
            ),
            Self::InvalidTranscriptArity => write!(f, "global-info transcript arity is invalid"),
            Self::Io { message } => write!(f, "global-info io error: {message}"),
        }
    }
}

impl std::error::Error for GlobalInfoError {}

impl GlobalInfo {
    pub fn total_air_count(&self) -> usize {
        self.airs.iter().map(Vec::len).sum()
    }

    pub fn stage_one_proof_value_count(&self) -> usize {
        self.proof_values_map
            .iter()
            .filter(|entry| entry.stage == 1)
            .count()
    }
}

pub fn read_global_info_file(path: impl AsRef<Path>) -> Result<GlobalInfo, GlobalInfoError> {
    let input = std::fs::read_to_string(path).map_err(|error| GlobalInfoError::Io {
        message: error.to_string(),
    })?;
    parse_global_info_json(&input)
}

pub fn parse_global_info_json(input: &str) -> Result<GlobalInfo, GlobalInfoError> {
    let value: serde_json::Value =
        serde_json::from_str(input).map_err(|error| GlobalInfoError::Json {
            message: error.to_string(),
        })?;
    let object = as_object(&value, "$")?;

    let name = required_string(object, "name")?;
    let air_groups = required_string_array(object, "air_groups")?;
    let airs = parse_airs(required_array(object, "airs")?)?;
    let aggregation_types = parse_aggregation_types(required_array(object, "aggTypes")?)?;
    validate_air_group_shape(&air_groups, &airs, &aggregation_types)?;

    let proof_values_map =
        parse_named_stage_values(optional_array(object, "proofValuesMap")?, "proofValuesMap")?;
    let publics_map = parse_public_values(optional_array(object, "publicsMap")?)?;
    let n_publics = required_u64(object, "nPublics")?;
    if n_publics != publics_map.len() as u64 {
        return Err(GlobalInfoError::PublicCountMismatch {
            expected: n_publics,
            found: publics_map.len(),
        });
    }

    let transcript_arity = required_u64(object, "transcriptArity")?;
    if transcript_arity == 0 {
        return Err(GlobalInfoError::InvalidTranscriptArity);
    }

    Ok(GlobalInfo {
        name,
        air_groups,
        airs,
        curve: parse_curve(&required_string(object, "curve")?)?,
        lattice_size: optional_u64(object, "latticeSize")?,
        aggregation_types,
        n_publics,
        num_challenges: required_u64_array(object, "numChallenges")?,
        num_proof_values: optional_u64_array(object, "numProofValues")?.unwrap_or_default(),
        proof_values_map,
        publics_map,
        transcript_arity,
    })
}

fn parse_airs(values: &[serde_json::Value]) -> Result<Vec<Vec<GlobalAir>>, GlobalInfoError> {
    let mut out = Vec::with_capacity(values.len());
    for (airgroup_id, group) in values.iter().enumerate() {
        let units = group
            .as_array()
            .ok_or(GlobalInfoError::InvalidField { field: "airs" })?;
        let mut parsed_units = Vec::with_capacity(units.len());
        for (air_id, unit) in units.iter().enumerate() {
            let object = as_object(unit, "airs")?;
            let num_rows = required_u64(object, "num_rows")?;
            if num_rows == 0 {
                return Err(GlobalInfoError::InvalidRowCount {
                    airgroup_id,
                    air_id,
                });
            }
            parsed_units.push(GlobalAir {
                name: required_string(object, "name")?,
                num_rows,
                has_compressor: optional_bool(object, "hasCompressor")?.unwrap_or(false),
            });
        }
        out.push(parsed_units);
    }
    Ok(out)
}

fn parse_aggregation_types(
    values: &[serde_json::Value],
) -> Result<Vec<Vec<AggregationType>>, GlobalInfoError> {
    let mut out = Vec::with_capacity(values.len());
    for group in values {
        let entries = group
            .as_array()
            .ok_or(GlobalInfoError::InvalidField { field: "aggTypes" })?;
        let mut parsed_entries = Vec::with_capacity(entries.len());
        for entry in entries {
            let object = as_object(entry, "aggTypes")?;
            parsed_entries.push(AggregationType {
                aggregation_type: required_u64(object, "aggType")?,
            });
        }
        out.push(parsed_entries);
    }
    Ok(out)
}

fn validate_air_group_shape(
    air_groups: &[String],
    airs: &[Vec<GlobalAir>],
    aggregation_types: &[Vec<AggregationType>],
) -> Result<(), GlobalInfoError> {
    if air_groups.len() != airs.len() || air_groups.len() != aggregation_types.len() {
        return Err(GlobalInfoError::AirGroupCountMismatch {
            air_groups: air_groups.len(),
            airs: airs.len(),
            aggregation_types: aggregation_types.len(),
        });
    }
    for (airgroup_id, units) in airs.iter().enumerate() {
        if units.is_empty() {
            return Err(GlobalInfoError::EmptyAirGroup { airgroup_id });
        }
    }
    Ok(())
}

fn parse_named_stage_values(
    values: Option<&Vec<serde_json::Value>>,
    field: &'static str,
) -> Result<Vec<NamedStageValue>, GlobalInfoError> {
    let Some(values) = values else {
        return Ok(Vec::new());
    };
    let mut out = Vec::with_capacity(values.len());
    for (index, value) in values.iter().enumerate() {
        let object = as_object(value, field)?;
        let stage = optional_u64(object, "stage")?.unwrap_or(0);
        if stage == 0 {
            return Err(GlobalInfoError::InvalidStage { field, index });
        }
        out.push(NamedStageValue {
            name: required_string(object, "name")?,
            stage,
            id: optional_u64(object, "id")?,
            lengths: optional_u64_array(object, "lengths")?.unwrap_or_default(),
        });
    }
    Ok(out)
}

fn parse_public_values(
    values: Option<&Vec<serde_json::Value>>,
) -> Result<Vec<PublicValue>, GlobalInfoError> {
    let Some(values) = values else {
        return Ok(Vec::new());
    };
    let mut out = Vec::with_capacity(values.len());
    for (index, value) in values.iter().enumerate() {
        let object = as_object(value, "publicsMap")?;
        let stage = optional_u64(object, "stage")?.unwrap_or(0);
        if stage == 0 {
            return Err(GlobalInfoError::InvalidStage {
                field: "publicsMap",
                index,
            });
        }
        out.push(PublicValue {
            name: required_string(object, "name")?,
            stage,
            lengths: optional_u64_array(object, "lengths")?.unwrap_or_default(),
        });
    }
    Ok(out)
}

fn parse_curve(curve: &str) -> Result<CurveKind, GlobalInfoError> {
    match curve {
        "None" => Ok(CurveKind::None),
        "EcGFp5" => Ok(CurveKind::EcGfp5),
        "EcMasFp5" => Ok(CurveKind::EcMasFp5),
        _ => Err(GlobalInfoError::UnknownCurve {
            curve: curve.to_owned(),
        }),
    }
}

fn as_object<'a>(
    value: &'a serde_json::Value,
    field: &'static str,
) -> Result<&'a serde_json::Map<String, serde_json::Value>, GlobalInfoError> {
    value
        .as_object()
        .ok_or(GlobalInfoError::InvalidField { field })
}

fn required<'a>(
    object: &'a serde_json::Map<String, serde_json::Value>,
    field: &'static str,
) -> Result<&'a serde_json::Value, GlobalInfoError> {
    object
        .get(field)
        .ok_or(GlobalInfoError::MissingField { field })
}

fn required_array<'a>(
    object: &'a serde_json::Map<String, serde_json::Value>,
    field: &'static str,
) -> Result<&'a Vec<serde_json::Value>, GlobalInfoError> {
    required(object, field)?
        .as_array()
        .ok_or(GlobalInfoError::InvalidField { field })
}

fn optional_array<'a>(
    object: &'a serde_json::Map<String, serde_json::Value>,
    field: &'static str,
) -> Result<Option<&'a Vec<serde_json::Value>>, GlobalInfoError> {
    object
        .get(field)
        .map(|value| {
            value
                .as_array()
                .ok_or(GlobalInfoError::InvalidField { field })
        })
        .transpose()
}

fn required_string(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &'static str,
) -> Result<String, GlobalInfoError> {
    required(object, field)?
        .as_str()
        .map(str::to_owned)
        .ok_or(GlobalInfoError::InvalidField { field })
}

fn required_string_array(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &'static str,
) -> Result<Vec<String>, GlobalInfoError> {
    let values = required_array(object, field)?;
    let mut out = Vec::with_capacity(values.len());
    for value in values {
        out.push(
            value
                .as_str()
                .map(str::to_owned)
                .ok_or(GlobalInfoError::InvalidField { field })?,
        );
    }
    Ok(out)
}

fn required_u64(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &'static str,
) -> Result<u64, GlobalInfoError> {
    value_to_u64(required(object, field)?, field)
}

fn optional_u64(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &'static str,
) -> Result<Option<u64>, GlobalInfoError> {
    object
        .get(field)
        .map(|value| value_to_u64(value, field))
        .transpose()
}

fn value_to_u64(value: &serde_json::Value, field: &'static str) -> Result<u64, GlobalInfoError> {
    value
        .as_u64()
        .ok_or(GlobalInfoError::InvalidField { field })
}

fn optional_bool(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &'static str,
) -> Result<Option<bool>, GlobalInfoError> {
    object
        .get(field)
        .map(|value| {
            value
                .as_bool()
                .ok_or(GlobalInfoError::InvalidField { field })
        })
        .transpose()
}

fn required_u64_array(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &'static str,
) -> Result<Vec<u64>, GlobalInfoError> {
    let values = required_array(object, field)?;
    let mut out = Vec::with_capacity(values.len());
    for value in values {
        out.push(value_to_u64(value, field)?);
    }
    Ok(out)
}

fn optional_u64_array(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &'static str,
) -> Result<Option<Vec<u64>>, GlobalInfoError> {
    let Some(values) = optional_array(object, field)? else {
        return Ok(None);
    };
    let mut out = Vec::with_capacity(values.len());
    for value in values {
        out.push(value_to_u64(value, field)?);
    }
    Ok(Some(out))
}
