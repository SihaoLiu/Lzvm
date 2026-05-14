use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitSetupInfo {
    pub n_stages: u32,
    pub n_constants: u32,
    pub constant_columns: Vec<ConstantColumn>,
    pub n_publics: Option<u32>,
    pub n_constraints: Option<u32>,
    pub q_degree: u32,
    pub opening_points: Vec<i64>,
    pub section_widths: BTreeMap<String, u32>,
    pub challenge_count: usize,
    pub eval_count: usize,
    pub boundaries: Vec<Boundary>,
    pub stark: StarkStruct,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StarkStruct {
    pub n_bits: u32,
    pub n_bits_ext: u32,
    pub n_queries: u32,
    pub steps: Vec<FriStep>,
    pub hash_commits: bool,
    pub last_level_verification: u32,
    pub pow_bits: u32,
    pub merkle_tree_arity: u32,
    pub verification_hash_type: Option<String>,
    pub transcript_arity: Option<u32>,
    pub merkle_tree_custom: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FriStep {
    pub n_bits: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Boundary {
    pub name: Option<String>,
    pub offset_min: Option<i64>,
    pub offset_max: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstantColumn {
    pub name: String,
    pub stage: u32,
    pub dimension: u32,
    pub pols_map_id: u32,
    pub stage_id: u32,
    pub lengths: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetupInfoError {
    Json { message: String },
    MissingField { field: &'static str },
    InvalidField { field: &'static str },
    MissingSectionWidth { name: String },
    InvalidDomainBits { n_bits: u32, n_bits_ext: u32 },
    InvalidFriSteps,
    ConstantColumnCountMismatch { expected: u32, found: usize },
    InvalidConstantColumn { index: usize },
    Io { message: String },
}

impl fmt::Display for SetupInfoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json { message } => write!(f, "setup-info json error: {message}"),
            Self::MissingField { field } => write!(f, "missing setup-info field: {field}"),
            Self::InvalidField { field } => write!(f, "invalid setup-info field: {field}"),
            Self::MissingSectionWidth { name } => {
                write!(f, "missing setup-info section width: {name}")
            }
            Self::InvalidDomainBits { n_bits, n_bits_ext } => write!(
                f,
                "invalid setup-info domain bits: n_bits {n_bits}, n_bits_ext {n_bits_ext}"
            ),
            Self::InvalidFriSteps => write!(f, "invalid setup-info FRI steps"),
            Self::ConstantColumnCountMismatch { expected, found } => write!(
                f,
                "setup-info constant-column count mismatch: expected {expected}, found {found}"
            ),
            Self::InvalidConstantColumn { index } => {
                write!(
                    f,
                    "invalid setup-info constant-column entry at index {index}"
                )
            }
            Self::Io { message } => write!(f, "setup-info io error: {message}"),
        }
    }
}

impl std::error::Error for SetupInfoError {}

impl UnitSetupInfo {
    pub fn stage_commit_widths(&self) -> Result<Vec<u32>, SetupInfoError> {
        let mut widths = Vec::with_capacity((self.n_stages + 1) as usize);
        for stage in 1..=self.n_stages + 1 {
            let name = format!("cm{stage}");
            let width = *self
                .section_widths
                .get(&name)
                .ok_or(SetupInfoError::MissingSectionWidth { name })?;
            widths.push(width);
        }
        Ok(widths)
    }
}

pub fn read_unit_setup_info_file(path: impl AsRef<Path>) -> Result<UnitSetupInfo, SetupInfoError> {
    let input = std::fs::read_to_string(path).map_err(|error| SetupInfoError::Io {
        message: error.to_string(),
    })?;
    parse_unit_setup_info_json(&input)
}

pub fn parse_unit_setup_info_json(input: &str) -> Result<UnitSetupInfo, SetupInfoError> {
    let value: serde_json::Value =
        serde_json::from_str(input).map_err(|error| SetupInfoError::Json {
            message: error.to_string(),
        })?;
    let object = as_object(&value, "$")?;

    let n_stages = required_u32(object, "nStages")?;
    let n_constants = required_u32(object, "nConstants")?;
    let constant_columns = parse_constant_columns(optional_array(object, "constPolsMap")?)?;
    validate_constant_columns(n_constants, &constant_columns)?;
    let q_degree = required_u32(object, "qDeg")?;
    let opening_points = required_i64_array(object, "openingPoints")?;
    let challenge_count = required_array(object, "challengesMap")?.len();
    let eval_count = required_array(object, "evMap")?.len();
    let boundaries = parse_boundaries(required_array(object, "boundaries")?)?;
    let stark = parse_stark_struct(required(object, "starkStruct")?)?;
    validate_domains(&stark)?;

    let section_widths = parse_u32_map(required(object, "mapSectionsN")?, "mapSectionsN")?;

    let info = UnitSetupInfo {
        n_stages,
        n_constants,
        constant_columns,
        n_publics: optional_u32(object, "nPublics")?,
        n_constraints: optional_u32(object, "nConstraints")?,
        q_degree,
        opening_points,
        section_widths,
        challenge_count,
        eval_count,
        boundaries,
        stark,
    };

    info.stage_commit_widths()?;
    Ok(info)
}

fn parse_constant_columns(
    values: Option<&Vec<serde_json::Value>>,
) -> Result<Vec<ConstantColumn>, SetupInfoError> {
    let Some(values) = values else {
        return Ok(Vec::new());
    };

    let mut out = Vec::with_capacity(values.len());
    for value in values {
        let object = as_object(value, "constPolsMap")?;
        out.push(ConstantColumn {
            name: required_string(object, "name")?,
            stage: required_u32(object, "stage")?,
            dimension: required_u32(object, "dim")?,
            pols_map_id: required_u32(object, "polsMapId")?,
            stage_id: required_u32(object, "stageId")?,
            lengths: optional_u32_array(object, "lengths")?.unwrap_or_default(),
        });
    }
    Ok(out)
}

fn validate_constant_columns(
    n_constants: u32,
    columns: &[ConstantColumn],
) -> Result<(), SetupInfoError> {
    if columns.is_empty() {
        return Ok(());
    }
    if columns.len() != n_constants as usize {
        return Err(SetupInfoError::ConstantColumnCountMismatch {
            expected: n_constants,
            found: columns.len(),
        });
    }

    let mut seen = vec![false; n_constants as usize];
    for (index, column) in columns.iter().enumerate() {
        let id = column.pols_map_id as usize;
        if column.stage != 0
            || column.dimension == 0
            || id >= seen.len()
            || seen[id]
            || column.stage_id != column.pols_map_id
        {
            return Err(SetupInfoError::InvalidConstantColumn { index });
        }
        seen[id] = true;
    }

    Ok(())
}

fn parse_stark_struct(value: &serde_json::Value) -> Result<StarkStruct, SetupInfoError> {
    let object = as_object(value, "starkStruct")?;
    let steps = parse_fri_steps(required_array(object, "steps")?)?;

    Ok(StarkStruct {
        n_bits: required_u32(object, "nBits")?,
        n_bits_ext: required_u32(object, "nBitsExt")?,
        n_queries: required_u32(object, "nQueries")?,
        steps,
        hash_commits: required_bool(object, "hashCommits")?,
        last_level_verification: required_u32(object, "lastLevelVerification")?,
        pow_bits: required_u32(object, "powBits")?,
        merkle_tree_arity: required_u32(object, "merkleTreeArity")?,
        verification_hash_type: optional_string(object, "verificationHashType")?,
        transcript_arity: optional_u32(object, "transcriptArity")?,
        merkle_tree_custom: optional_bool(object, "merkleTreeCustom")?,
    })
}

fn validate_domains(stark: &StarkStruct) -> Result<(), SetupInfoError> {
    if stark.n_bits_ext < stark.n_bits {
        return Err(SetupInfoError::InvalidDomainBits {
            n_bits: stark.n_bits,
            n_bits_ext: stark.n_bits_ext,
        });
    }

    let Some(first_step) = stark.steps.first() else {
        return Err(SetupInfoError::InvalidFriSteps);
    };
    if first_step.n_bits != stark.n_bits_ext {
        return Err(SetupInfoError::InvalidFriSteps);
    }

    for pair in stark.steps.windows(2) {
        if pair[1].n_bits >= pair[0].n_bits {
            return Err(SetupInfoError::InvalidFriSteps);
        }
    }

    Ok(())
}

fn parse_fri_steps(values: &[serde_json::Value]) -> Result<Vec<FriStep>, SetupInfoError> {
    let mut steps = Vec::with_capacity(values.len());
    for value in values {
        let object = as_object(value, "steps")?;
        steps.push(FriStep {
            n_bits: required_u32(object, "nBits")?,
        });
    }
    Ok(steps)
}

fn parse_boundaries(values: &[serde_json::Value]) -> Result<Vec<Boundary>, SetupInfoError> {
    let mut boundaries = Vec::with_capacity(values.len());
    for value in values {
        let object = as_object(value, "boundaries")?;
        boundaries.push(Boundary {
            name: optional_string(object, "name")?,
            offset_min: optional_i64(object, "offsetMin")?,
            offset_max: optional_i64(object, "offsetMax")?,
        });
    }
    Ok(boundaries)
}

fn parse_u32_map(
    value: &serde_json::Value,
    field: &'static str,
) -> Result<BTreeMap<String, u32>, SetupInfoError> {
    let object = value
        .as_object()
        .ok_or(SetupInfoError::InvalidField { field })?;
    let mut out = BTreeMap::new();
    for (key, value) in object {
        out.insert(key.clone(), value_to_u32(value, field)?);
    }
    Ok(out)
}

fn as_object<'a>(
    value: &'a serde_json::Value,
    field: &'static str,
) -> Result<&'a serde_json::Map<String, serde_json::Value>, SetupInfoError> {
    value
        .as_object()
        .ok_or(SetupInfoError::InvalidField { field })
}

fn required<'a>(
    object: &'a serde_json::Map<String, serde_json::Value>,
    field: &'static str,
) -> Result<&'a serde_json::Value, SetupInfoError> {
    object
        .get(field)
        .ok_or(SetupInfoError::MissingField { field })
}

fn required_array<'a>(
    object: &'a serde_json::Map<String, serde_json::Value>,
    field: &'static str,
) -> Result<&'a Vec<serde_json::Value>, SetupInfoError> {
    required(object, field)?
        .as_array()
        .ok_or(SetupInfoError::InvalidField { field })
}

fn optional_array<'a>(
    object: &'a serde_json::Map<String, serde_json::Value>,
    field: &'static str,
) -> Result<Option<&'a Vec<serde_json::Value>>, SetupInfoError> {
    object
        .get(field)
        .map(|value| {
            value
                .as_array()
                .ok_or(SetupInfoError::InvalidField { field })
        })
        .transpose()
}

fn required_string(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &'static str,
) -> Result<String, SetupInfoError> {
    required(object, field)?
        .as_str()
        .map(str::to_owned)
        .ok_or(SetupInfoError::InvalidField { field })
}

fn required_u32(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &'static str,
) -> Result<u32, SetupInfoError> {
    value_to_u32(required(object, field)?, field)
}

fn optional_u32(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &'static str,
) -> Result<Option<u32>, SetupInfoError> {
    object
        .get(field)
        .map(|value| value_to_u32(value, field))
        .transpose()
}

fn optional_u32_array(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &'static str,
) -> Result<Option<Vec<u32>>, SetupInfoError> {
    let Some(values) = optional_array(object, field)? else {
        return Ok(None);
    };
    let mut out = Vec::with_capacity(values.len());
    for value in values {
        out.push(value_to_u32(value, field)?);
    }
    Ok(Some(out))
}

fn value_to_u32(value: &serde_json::Value, field: &'static str) -> Result<u32, SetupInfoError> {
    let Some(number) = value.as_u64() else {
        return Err(SetupInfoError::InvalidField { field });
    };
    u32::try_from(number).map_err(|_| SetupInfoError::InvalidField { field })
}

fn required_bool(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &'static str,
) -> Result<bool, SetupInfoError> {
    required(object, field)?
        .as_bool()
        .ok_or(SetupInfoError::InvalidField { field })
}

fn optional_bool(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &'static str,
) -> Result<Option<bool>, SetupInfoError> {
    object
        .get(field)
        .map(|value| {
            value
                .as_bool()
                .ok_or(SetupInfoError::InvalidField { field })
        })
        .transpose()
}

fn optional_i64(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &'static str,
) -> Result<Option<i64>, SetupInfoError> {
    object
        .get(field)
        .map(|value| value.as_i64().ok_or(SetupInfoError::InvalidField { field }))
        .transpose()
}

fn required_i64_array(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &'static str,
) -> Result<Vec<i64>, SetupInfoError> {
    let values = required_array(object, field)?;
    let mut out = Vec::with_capacity(values.len());
    for value in values {
        let Some(number) = value.as_i64() else {
            return Err(SetupInfoError::InvalidField { field });
        };
        out.push(number);
    }
    Ok(out)
}

fn optional_string(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &'static str,
) -> Result<Option<String>, SetupInfoError> {
    object
        .get(field)
        .map(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or(SetupInfoError::InvalidField { field })
        })
        .transpose()
}
