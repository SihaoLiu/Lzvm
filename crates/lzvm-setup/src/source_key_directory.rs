#![allow(clippy::map_entry, clippy::too_many_arguments)]

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use lzvm_artifacts::expression_info::{CodeOperand, ExpressionInfo, ExpressionInfoError};
use lzvm_artifacts::global_info::{
    encode_global_info, AggregationType, CurveKind, GlobalAir, GlobalInfo, GlobalInfoError,
    NamedStageValue, PublicValue,
};
use lzvm_artifacts::global_program::{encode_global_program, GlobalProgramError};
use lzvm_artifacts::key_directory::{
    read_key_directory_layout, KeyDirectoryError, KeyUnitKind, KeyUnitPaths,
};
use lzvm_artifacts::setup_info::{
    encode_unit_setup_info, Boundary, CommitmentColumn, ConstantColumn, EvaluationMapEntry,
    FriStep, SetupInfoError, StageValue, StarkStruct, UnitSetupInfo,
};
use lzvm_artifacts::verifier_info::{encode_verifier_info, VerifierInfoError};
use lzvm_pil::{
    lex_source, AirGroupValueDeclaration, AirInstanceDeclaration, AirTemplateDeclaration,
    ColumnDeclaration, ColumnItem, ColumnKind, FixedFileTemplateValue, LexError, ParseError,
    SourceLoaderConfig, SourceProgram, SourceProgramError, SourceProgramLoader,
    SourceProgramModule, Token, ValueDeclarationKind,
};

use crate::{
    publish_staging_bytes,
    source_control_body_cache::{SourceControlBodyCache, SourceControlBodyCaches},
    source_expression_info::source_expression_info,
    source_global_constraints::source_global_program,
    source_global_values::{
        source_challenge_counts, source_challenge_slots, source_proof_values, source_public_values,
    },
    source_metadata_template::{
        source_declaration_in_unselected_static_branch, source_metadata_declaration_template,
        source_metadata_template_instances, source_metadata_template_values,
        source_metadata_unit_instance,
    },
    source_opening_points::source_opening_points,
    source_range_check_hints::SourceRangeCheckIds,
    source_row_count::{infer_source_row_counts, SourceUnitRowCounts},
    source_scalar_slots::SourceChallengeSlotMetadata,
    source_scope::{
        concrete_template_names, declaration_in_function_body, declaration_in_inactive_template,
    },
    source_static_values::{
        evaluate_source_static_expression, source_declaration_constant_values_from_cache,
        source_declaration_in_static_false_branch, source_scalar_constant_values,
        source_template_constant_value_cache, static_value_integer,
        SourceTemplateConstantValueCache,
    },
    source_verifier_info::source_verifier_info,
    write_staging_bytes, SetupError,
};

mod template_items;

use template_items::source_column_stage;
pub(crate) use template_items::{source_column_dimension, source_item_lengths, source_item_name};

const SOURCE_GLOBAL_LATTICE_SIZE: u64 = 368;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceKeyDirectoryMetadataRequest {
    pub working_dir: PathBuf,
    pub include_paths: Vec<PathBuf>,
    pub include_path_first: bool,
    pub main_file: PathBuf,
    pub setup_dir: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceKeyDirectoryMetadataReport {
    pub setup_dir: PathBuf,
    pub unit_count: usize,
    pub bytes_written: u64,
}

struct SourceUnitTemplateContext {
    constant_values: BTreeMap<String, FixedFileTemplateValue>,
    template_values: SourceTemplateConstantValueCache,
    challenge_counts: Vec<u64>,
    proof_values_map: Vec<NamedStageValue>,
    publics_map: Vec<PublicValue>,
    challenge_slots: Vec<SourceChallengeSlotMetadata>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceKeyDirectoryMetadataError {
    SourceProgram(SourceProgramError),
    GlobalInfo(GlobalInfoError),
    GlobalProgram(GlobalProgramError),
    SetupInfo(SetupInfoError),
    ExpressionInfo(ExpressionInfoError),
    VerifierInfo(VerifierInfoError),
    KeyDirectory(KeyDirectoryError),
    Setup(SetupError),
    Parse(ParseError),
    Lex {
        source_name: String,
        source: LexError,
    },
    UnsupportedSourceProgram {
        message: String,
    },
    StaticAssertionFailed {
        line: String,
    },
}

struct SourceUnitMetadataPayload {
    setup_path: PathBuf,
    setup_bytes: Vec<u8>,
    expression_path: Option<PathBuf>,
    expression_bytes: Vec<u8>,
    verifier_path: Option<PathBuf>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SourceUnitMetadataPayloadKey {
    setup_path: PathBuf,
    expression_path: Option<PathBuf>,
    verifier_path: Option<PathBuf>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct SourceMetadataTemplateValueKey {
    source_name: String,
    template_start: usize,
    template_end: usize,
    instance_source_name: Option<String>,
    instance_start: Option<usize>,
    instance_end: Option<usize>,
}

#[derive(Default)]
struct SourceMetadataTemplateValueCache {
    values: BTreeMap<SourceMetadataTemplateValueKey, Arc<BTreeMap<String, FixedFileTemplateValue>>>,
}

impl fmt::Display for SourceKeyDirectoryMetadataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SourceProgram(error) => write!(f, "{error}"),
            Self::GlobalInfo(error) => write!(f, "{error}"),
            Self::GlobalProgram(error) => write!(f, "{error}"),
            Self::SetupInfo(error) => write!(f, "{error}"),
            Self::ExpressionInfo(error) => write!(f, "{error}"),
            Self::VerifierInfo(error) => write!(f, "{error}"),
            Self::KeyDirectory(error) => write!(f, "{error}"),
            Self::Setup(error) => write!(f, "{error}"),
            Self::Parse(error) => write!(f, "{error}"),
            Self::Lex {
                source_name,
                source,
            } => {
                write!(
                    f,
                    "source setup metadata lexing failed in {source_name}: {source}"
                )
            }
            Self::UnsupportedSourceProgram { message } => {
                write!(f, "unsupported source setup metadata: {message}")
            }
            Self::StaticAssertionFailed { line } => {
                write!(f, "source static assertion failed: {line}")
            }
        }
    }
}

impl std::error::Error for SourceKeyDirectoryMetadataError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::SourceProgram(error) => Some(error),
            Self::GlobalInfo(error) => Some(error),
            Self::GlobalProgram(error) => Some(error),
            Self::SetupInfo(error) => Some(error),
            Self::ExpressionInfo(error) => Some(error),
            Self::VerifierInfo(error) => Some(error),
            Self::KeyDirectory(error) => Some(error),
            Self::Setup(error) => Some(error),
            Self::Parse(error) => Some(error),
            Self::Lex { source, .. } => Some(source),
            Self::UnsupportedSourceProgram { .. } | Self::StaticAssertionFailed { .. } => None,
        }
    }
}

impl From<GlobalInfoError> for SourceKeyDirectoryMetadataError {
    fn from(error: GlobalInfoError) -> Self {
        Self::GlobalInfo(error)
    }
}

impl From<GlobalProgramError> for SourceKeyDirectoryMetadataError {
    fn from(error: GlobalProgramError) -> Self {
        Self::GlobalProgram(error)
    }
}

impl From<SetupInfoError> for SourceKeyDirectoryMetadataError {
    fn from(error: SetupInfoError) -> Self {
        Self::SetupInfo(error)
    }
}

impl From<ExpressionInfoError> for SourceKeyDirectoryMetadataError {
    fn from(error: ExpressionInfoError) -> Self {
        Self::ExpressionInfo(error)
    }
}

impl From<VerifierInfoError> for SourceKeyDirectoryMetadataError {
    fn from(error: VerifierInfoError) -> Self {
        Self::VerifierInfo(error)
    }
}

impl From<KeyDirectoryError> for SourceKeyDirectoryMetadataError {
    fn from(error: KeyDirectoryError) -> Self {
        Self::KeyDirectory(error)
    }
}

impl From<SetupError> for SourceKeyDirectoryMetadataError {
    fn from(error: SetupError) -> Self {
        Self::Setup(error)
    }
}

impl From<ParseError> for SourceKeyDirectoryMetadataError {
    fn from(error: ParseError) -> Self {
        Self::Parse(error)
    }
}

pub fn write_source_key_directory_metadata(
    request: &SourceKeyDirectoryMetadataRequest,
) -> Result<SourceKeyDirectoryMetadataReport, SourceKeyDirectoryMetadataError> {
    let mut loader = SourceProgramLoader::new(SourceLoaderConfig {
        working_dir: request.working_dir.clone(),
        include_paths: request.include_paths.clone(),
        include_path_first: request.include_path_first,
    });
    let program = loader
        .load_main(&request.main_file)
        .map_err(SourceKeyDirectoryMetadataError::SourceProgram)?;
    validate_supported_source_program(&program)?;

    let row_counts = infer_source_row_counts(&program)?;
    let global_info = source_global_info(&program, &row_counts)?;
    let global_program = source_global_program(&program, &global_info)?;
    let global_info_path = request.setup_dir.join("pilout.globalInfo.bin");
    let global_info_bytes = encode_global_info(&global_info)?;
    let global_program_bytes = encode_global_program(&global_program)?;
    let verifier_bytes = encode_verifier_info(&source_verifier_info())?;

    let mut bytes_written = 0_u64;
    bytes_written = bytes_written.saturating_add(write_validated_bytes(
        &global_info_path,
        &global_info_bytes,
        "write source global metadata staging file",
        "publish source global metadata",
    )?);

    let payload_result = (|| {
        let layout = read_key_directory_layout(&request.setup_dir)?;
        let mut unit_payloads = Vec::new();
        let mut body_caches = SourceControlBodyCaches::default();
        let range_checks = RefCell::new(SourceRangeCheckIds::default());
        let active_templates = concrete_template_names(&program);
        let mut template_contexts = BTreeMap::<u64, SourceUnitTemplateContext>::new();
        for (unit_index, unit) in layout.units.iter().enumerate() {
            let Some(payload_key) = source_unit_metadata_payload_key(unit) else {
                continue;
            };
            if !source_unit_metadata_payload_is_last_writer(&layout.units, unit_index) {
                continue;
            }
            let row_count = source_layout_unit_row_count(unit, &row_counts)?;
            if !template_contexts.contains_key(&row_count) {
                let constant_values = source_scalar_constant_values(&program, row_count);
                let template_values =
                    source_template_constant_value_cache(&program, &constant_values);
                let challenge_counts = source_challenge_counts(
                    &program,
                    &constant_values,
                    &active_templates,
                    &template_values,
                    &mut body_caches,
                )?;
                let (_, proof_values_map) = source_proof_values(
                    &program,
                    &constant_values,
                    &active_templates,
                    &template_values,
                    &mut body_caches,
                )?;
                let publics_map = source_public_values(
                    &program,
                    &constant_values,
                    &active_templates,
                    &template_values,
                    &mut body_caches,
                )?;
                let challenge_slots = source_challenge_slots(
                    &program,
                    &constant_values,
                    &active_templates,
                    &template_values,
                    &mut body_caches,
                )?;
                template_contexts.insert(
                    row_count,
                    SourceUnitTemplateContext {
                        constant_values,
                        template_values,
                        challenge_counts,
                        proof_values_map,
                        publics_map,
                        challenge_slots,
                    },
                );
            }
            let context = template_contexts
                .get(&row_count)
                .ok_or_else(|| unsupported_source_message("source template context is missing"))?;
            let mut setup_info = source_unit_setup_info(
                &program,
                row_count,
                unit.group_name.as_deref().zip(unit.unit_name.as_deref()),
                &context.constant_values,
                &active_templates,
                &context.template_values,
                &context.challenge_counts,
                &context.proof_values_map,
                &context.publics_map,
                &mut body_caches,
            )?;
            let expression_info = if unit.kind == KeyUnitKind::Basic {
                source_expression_info(
                    &program,
                    &setup_info,
                    unit.group_name.as_deref(),
                    unit.unit_name.as_deref(),
                    &global_info.publics_map,
                    &context.challenge_slots,
                    &global_info.proof_values_map,
                    &context.constant_values,
                    &active_templates,
                    &context.template_values,
                    &mut body_caches,
                    &range_checks,
                )?
            } else {
                ExpressionInfo {
                    hints: Vec::new(),
                    expressions: Vec::new(),
                    constraints: Vec::new(),
                }
            };
            include_expression_opening_points(&mut setup_info, &expression_info);
            setup_info.n_constraints = Some(
                u32::try_from(expression_info.constraints.len())
                    .map_err(|_| unsupported_source_message("too many source constraints"))?,
            );
            if !expression_info.constraints.is_empty() && setup_info.n_stages == 0 {
                setup_info.n_stages = 1;
                setup_info
                    .section_widths
                    .entry("cm2".to_owned())
                    .or_insert(1);
            }
            let setup_bytes = encode_unit_setup_info(&setup_info)?;
            let expression_bytes =
                lzvm_artifacts::expression_info::encode_expression_info(&expression_info)?;
            unit_payloads.push(SourceUnitMetadataPayload {
                setup_path: payload_key.setup_path,
                setup_bytes,
                expression_path: payload_key.expression_path,
                expression_bytes,
                verifier_path: payload_key.verifier_path,
            });
        }
        Ok::<_, SourceKeyDirectoryMetadataError>((layout.units.len(), unit_payloads))
    })();
    let (unit_count, unit_payloads) = match payload_result {
        Ok(payload) => payload,
        Err(error) => {
            let _ = std::fs::remove_file(&global_info_path);
            return Err(error);
        }
    };

    bytes_written = bytes_written.saturating_add(write_validated_bytes(
        &request.setup_dir.join("pilout.globalConstraints.bin"),
        &global_program_bytes,
        "write source global program staging file",
        "publish source global program",
    )?);

    for payload in unit_payloads {
        bytes_written = bytes_written.saturating_add(write_validated_bytes(
            &payload.setup_path,
            &payload.setup_bytes,
            "write source setup metadata staging file",
            "publish source setup metadata",
        )?);
        if let Some(path) = payload.expression_path {
            bytes_written = bytes_written.saturating_add(write_validated_bytes(
                &path,
                &payload.expression_bytes,
                "write source expression metadata staging file",
                "publish source expression metadata",
            )?);
        }
        if let Some(path) = payload.verifier_path {
            bytes_written = bytes_written.saturating_add(write_validated_bytes(
                &path,
                &verifier_bytes,
                "write source verifier metadata staging file",
                "publish source verifier metadata",
            )?);
        }
    }

    Ok(SourceKeyDirectoryMetadataReport {
        setup_dir: request.setup_dir.clone(),
        unit_count,
        bytes_written,
    })
}

fn write_validated_bytes(
    path: &Path,
    bytes: &[u8],
    write_role: &'static str,
    publish_role: &'static str,
) -> Result<u64, SourceKeyDirectoryMetadataError> {
    let staging_path = write_staging_bytes(path, bytes, write_role)?;
    publish_staging_bytes(&staging_path, path, publish_role).map_err(Into::into)
}

fn source_layout_unit_row_count(
    unit: &KeyUnitPaths,
    row_counts: &SourceUnitRowCounts,
) -> Result<u64, SourceKeyDirectoryMetadataError> {
    if let (Some(group_id), Some(unit_id)) = (unit.group_id, unit.unit_id) {
        return row_counts
            .get(&(group_id, unit_id))
            .copied()
            .ok_or_else(|| {
                unsupported_source_message(format!(
                    "missing source row count for group {group_id} unit {unit_id}"
                ))
            });
    }
    if let Some(group_id) = unit.group_id {
        if let Some((_, row_count)) = row_counts
            .iter()
            .find(|((candidate_group_id, _), _)| *candidate_group_id == group_id)
        {
            return Ok(*row_count);
        }
    }
    row_counts
        .values()
        .next()
        .copied()
        .ok_or_else(|| unsupported_source_message("source row counts are empty"))
}

fn source_unit_metadata_payload_is_last_writer(units: &[KeyUnitPaths], index: usize) -> bool {
    let Some(unit) = units.get(index) else {
        return false;
    };
    let Some(key) = source_unit_metadata_payload_key(unit) else {
        return false;
    };
    !units[index + 1..]
        .iter()
        .any(|unit| source_unit_metadata_payload_key(unit).is_some_and(|later| later == key))
}

fn source_unit_metadata_payload_key(unit: &KeyUnitPaths) -> Option<SourceUnitMetadataPayloadKey> {
    Some(SourceUnitMetadataPayloadKey {
        setup_path: unit.setup_info_binary()?,
        expression_path: unit.expression_info_binary(),
        verifier_path: unit.verifier_info_binary(),
    })
}

fn source_metadata_template_values_cached(
    program: &SourceProgram,
    module: &SourceProgramModule,
    template: &AirTemplateDeclaration,
    instance: Option<&AirInstanceDeclaration>,
    base_values: &BTreeMap<String, FixedFileTemplateValue>,
    template_values: &SourceTemplateConstantValueCache,
    cache: &mut SourceMetadataTemplateValueCache,
) -> Arc<BTreeMap<String, FixedFileTemplateValue>> {
    let key = SourceMetadataTemplateValueKey {
        source_name: module.source_name.clone(),
        template_start: template.body.start,
        template_end: template.body.end,
        instance_source_name: instance.map(|instance| instance.source_name.clone()),
        instance_start: instance.map(|instance| instance.start),
        instance_end: instance.map(|instance| instance.end),
    };
    if let Some(values) = cache.values.get(&key) {
        return Arc::clone(values);
    }
    let values = Arc::new(source_metadata_template_values(
        program,
        module,
        template,
        instance,
        base_values,
        template_values,
    ));
    cache.values.insert(key, Arc::clone(&values));
    values
}

fn validate_supported_source_program(
    program: &SourceProgram,
) -> Result<(), SourceKeyDirectoryMetadataError> {
    let constant_values = source_scalar_constant_values(program, SOURCE_GLOBAL_LATTICE_SIZE);
    let template_values = source_template_constant_value_cache(program, &constant_values);
    let active_templates = concrete_template_names(program);
    for module in &program.modules {
        for declaration in &module.public_tables {
            if declaration_in_function_body(module, declaration.start, declaration.end)
                || declaration_in_inactive_template(
                    module,
                    declaration.start,
                    declaration.end,
                    &active_templates,
                )
            {
                continue;
            }
            let declaration_values = source_declaration_constant_values_from_cache(
                module,
                declaration.start,
                declaration.end,
                &constant_values,
                &template_values,
            );
            if source_declaration_in_static_false_branch(
                program,
                module,
                declaration.start,
                declaration.end,
                declaration_values,
            ) {
                continue;
            }
            return unsupported("public tables need metadata lowering support");
        }
    }
    Ok(())
}

pub(crate) fn unsupported<T>(
    message: impl Into<String>,
) -> Result<T, SourceKeyDirectoryMetadataError> {
    Err(SourceKeyDirectoryMetadataError::UnsupportedSourceProgram {
        message: message.into(),
    })
}

fn source_global_info(
    program: &SourceProgram,
    row_counts: &SourceUnitRowCounts,
) -> Result<GlobalInfo, SourceKeyDirectoryMetadataError> {
    let row_count = row_counts.values().next().copied().unwrap_or(2);
    let constant_values = source_scalar_constant_values(program, row_count);
    let template_values = source_template_constant_value_cache(program, &constant_values);
    let active_templates = concrete_template_names(program);
    let mut body_caches = SourceControlBodyCaches::default();
    let num_challenges = source_challenge_counts(
        program,
        &constant_values,
        &active_templates,
        &template_values,
        &mut body_caches,
    )?;
    let (num_proof_values, proof_values_map) = source_proof_values(
        program,
        &constant_values,
        &active_templates,
        &template_values,
        &mut body_caches,
    )?;
    let publics_map = source_public_values(
        program,
        &constant_values,
        &active_templates,
        &template_values,
        &mut body_caches,
    )?;
    let (_, group_aggregation_types) = source_air_group_values(
        program,
        None,
        &constant_values,
        &active_templates,
        &template_values,
        &mut body_caches,
    )?;
    let mut groups = BTreeMap::<usize, (String, BTreeMap<usize, String>)>::new();
    for unit in program
        .air_units()
        .into_iter()
        .filter(|unit| !unit.virtual_instance)
    {
        let group_id = usize::try_from(unit.group_id)
            .map_err(|_| unsupported_source_message("negative source group id"))?;
        let unit_id = usize::try_from(unit.unit_id)
            .map_err(|_| unsupported_source_message("negative source unit id"))?;
        let entry = groups
            .entry(group_id)
            .or_insert_with(|| (unit.group_name.clone(), BTreeMap::new()));
        if entry.0 != unit.group_name {
            return unsupported("source group id maps to multiple names");
        }
        entry.1.insert(unit_id, unit.unit_name);
    }

    if groups.is_empty() {
        return unsupported("source program has no concrete air units");
    }

    let mut air_groups = Vec::with_capacity(groups.len());
    let mut airs = Vec::with_capacity(groups.len());
    for (expected_group_id, (group_id, (group_name, units))) in groups.into_iter().enumerate() {
        if expected_group_id != group_id {
            return unsupported("source group ids must be contiguous");
        }
        if units.is_empty() {
            return unsupported("source group has no concrete units");
        }
        let mut group_units = Vec::with_capacity(units.len());
        for (expected_unit_id, (unit_id, unit_name)) in units.into_iter().enumerate() {
            if expected_unit_id != unit_id {
                return unsupported("source unit ids must be contiguous within each group");
            }
            let row_count = row_counts
                .get(&(group_id, unit_id))
                .copied()
                .ok_or_else(|| {
                    unsupported_source_message(format!(
                        "missing source row count for group {group_id} unit {unit_id}"
                    ))
                })?;
            group_units.push(GlobalAir {
                name: unit_name,
                num_rows: row_count,
                has_compressor: false,
            });
        }
        air_groups.push(group_name);
        airs.push(group_units);
    }
    let aggregation_types = vec![group_aggregation_types; air_groups.len()];

    Ok(GlobalInfo {
        name: "source-program".to_owned(),
        air_groups,
        airs,
        curve: CurveKind::None,
        lattice_size: Some(SOURCE_GLOBAL_LATTICE_SIZE),
        aggregation_types,
        n_publics: source_public_count(&publics_map)?,
        num_challenges,
        num_proof_values,
        proof_values_map,
        publics_map,
        transcript_arity: 4,
    })
}

fn source_public_count(publics: &[PublicValue]) -> Result<u64, SourceKeyDirectoryMetadataError> {
    publics.iter().try_fold(0_u64, |count, entry| {
        let dimension = entry.lengths.iter().try_fold(1_u64, |dimension, length| {
            dimension
                .checked_mul(*length)
                .ok_or_else(|| unsupported_source_message("source public value count overflow"))
        })?;
        count
            .checked_add(dimension)
            .ok_or_else(|| unsupported_source_message("source public value count overflow"))
    })
}

fn source_unit_values(
    program: &SourceProgram,
    unit_name: Option<(&str, &str)>,
    constant_values: &BTreeMap<String, FixedFileTemplateValue>,
    active_templates: &BTreeSet<String>,
    template_values: &SourceTemplateConstantValueCache,
    body_caches: &mut SourceControlBodyCaches,
    metadata_values: &mut SourceMetadataTemplateValueCache,
) -> Result<Vec<StageValue>, SourceKeyDirectoryMetadataError> {
    let mut seen = BTreeMap::<String, (u32, Vec<u32>)>::new();
    let mut values = Vec::new();
    let unit_instance = source_metadata_unit_instance(program, unit_name);
    for module in &program.modules {
        let tokens = lex_source(&module.source.contents).map_err(|source| {
            SourceKeyDirectoryMetadataError::Lex {
                source_name: module.source_name.clone(),
                source,
            }
        })?;
        let body_cache = body_caches.module_cache(&module.source_name);
        for declaration in &module.values {
            if declaration.kind != ValueDeclarationKind::AirValue
                || declaration_in_function_body(module, declaration.start, declaration.end)
                || declaration_in_inactive_template(
                    module,
                    declaration.start,
                    declaration.end,
                    active_templates,
                )
            {
                continue;
            }
            let Some(declaration_template) =
                source_metadata_declaration_template(module, declaration.start, declaration.end)
            else {
                let declaration_values = source_declaration_constant_values_from_cache(
                    module,
                    declaration.start,
                    declaration.end,
                    constant_values,
                    template_values,
                );
                if source_declaration_in_static_false_branch(
                    program,
                    module,
                    declaration.start,
                    declaration.end,
                    declaration_values,
                ) {
                    continue;
                }
                source_push_unit_values(
                    program,
                    declaration.stage,
                    &declaration.items,
                    declaration_values,
                    &mut seen,
                    &mut values,
                )?;
                continue;
            };
            if unit_instance.is_some_and(|instance| declaration_template.name != instance.template)
            {
                continue;
            }
            let declaration_values = source_metadata_template_values_cached(
                program,
                module,
                declaration_template,
                unit_instance,
                constant_values,
                template_values,
                metadata_values,
            );
            if source_declaration_in_unselected_static_branch(
                program,
                module,
                &tokens,
                body_cache,
                declaration.start,
                declaration.end,
                &declaration_values,
            )? {
                continue;
            }
            source_push_unit_values(
                program,
                declaration.stage,
                &declaration.items,
                &declaration_values,
                &mut seen,
                &mut values,
            )?;
        }
    }
    Ok(values)
}

fn source_push_unit_values(
    program: &SourceProgram,
    stage: u32,
    items: &[ColumnItem],
    declaration_values: &BTreeMap<String, FixedFileTemplateValue>,
    seen: &mut BTreeMap<String, (u32, Vec<u32>)>,
    values: &mut Vec<StageValue>,
) -> Result<(), SourceKeyDirectoryMetadataError> {
    if stage == 0 {
        return unsupported("source air value stage must be positive");
    }
    for item in items {
        let name = source_item_name(program, item, "source air value", declaration_values)?;
        let lengths = source_item_lengths(program, item, "source air value", declaration_values)?;
        let shape = (stage, lengths.clone());
        if let Some(existing) = seen.get(&name) {
            if *existing != shape {
                return unsupported("duplicate source air value name");
            }
            continue;
        }
        seen.insert(name.clone(), shape);
        values.push(StageValue {
            name,
            stage,
            lengths,
        });
    }
    Ok(())
}

pub(crate) fn source_air_group_values(
    program: &SourceProgram,
    unit_name: Option<(&str, &str)>,
    constant_values: &BTreeMap<String, FixedFileTemplateValue>,
    active_templates: &BTreeSet<String>,
    template_values: &SourceTemplateConstantValueCache,
    body_caches: &mut SourceControlBodyCaches,
) -> Result<(Vec<StageValue>, Vec<AggregationType>), SourceKeyDirectoryMetadataError> {
    let mut metadata_values = SourceMetadataTemplateValueCache::default();
    source_air_group_values_with_cache(
        program,
        unit_name,
        constant_values,
        active_templates,
        template_values,
        body_caches,
        &mut metadata_values,
    )
}

fn source_air_group_values_with_cache(
    program: &SourceProgram,
    unit_name: Option<(&str, &str)>,
    constant_values: &BTreeMap<String, FixedFileTemplateValue>,
    active_templates: &BTreeSet<String>,
    template_values: &SourceTemplateConstantValueCache,
    body_caches: &mut SourceControlBodyCaches,
    metadata_values: &mut SourceMetadataTemplateValueCache,
) -> Result<(Vec<StageValue>, Vec<AggregationType>), SourceKeyDirectoryMetadataError> {
    let mut seen = BTreeSet::new();
    let mut values = Vec::new();
    let mut aggregation_types = Vec::new();
    let unit_instance = source_metadata_unit_instance(program, unit_name);
    for module in &program.modules {
        let tokens = lex_source(&module.source.contents).map_err(|source| {
            SourceKeyDirectoryMetadataError::Lex {
                source_name: module.source_name.clone(),
                source,
            }
        })?;
        let body_cache = body_caches.module_cache(&module.source_name);
        let mut template_context = SourceGroupValueTemplateContext {
            program,
            module,
            tokens: &tokens,
            body_cache,
            constant_values,
            template_values,
            metadata_values,
        };
        for declaration in &module.air_group_values {
            if declaration_in_function_body(module, declaration.start, declaration.end)
                || declaration_in_inactive_template(
                    module,
                    declaration.start,
                    declaration.end,
                    active_templates,
                )
            {
                continue;
            }
            let Some(declaration_template) =
                source_metadata_declaration_template(module, declaration.start, declaration.end)
            else {
                let declaration_values = source_declaration_constant_values_from_cache(
                    module,
                    declaration.start,
                    declaration.end,
                    constant_values,
                    template_values,
                );
                if source_declaration_in_static_false_branch(
                    program,
                    module,
                    declaration.start,
                    declaration.end,
                    declaration_values,
                ) {
                    continue;
                }
                source_push_air_group_values(
                    program,
                    declaration,
                    declaration_values,
                    &mut seen,
                    &mut values,
                    &mut aggregation_types,
                )?;
                continue;
            };

            let declaration_values = if let Some(instance) = unit_instance {
                if declaration_template.name != instance.template {
                    continue;
                }
                let values = source_metadata_template_values_cached(
                    program,
                    module,
                    declaration_template,
                    Some(instance),
                    constant_values,
                    template_values,
                    template_context.metadata_values,
                );
                if source_declaration_in_unselected_static_branch(
                    program,
                    module,
                    &tokens,
                    template_context.body_cache,
                    declaration.start,
                    declaration.end,
                    &values,
                )? {
                    continue;
                }
                Some(values)
            } else {
                source_air_group_values_for_any_instance(
                    &mut template_context,
                    declaration,
                    declaration_template,
                )?
            };
            let Some(declaration_values) = declaration_values else {
                continue;
            };
            source_push_air_group_values(
                program,
                declaration,
                &declaration_values,
                &mut seen,
                &mut values,
                &mut aggregation_types,
            )?;
        }
    }
    Ok((values, aggregation_types))
}

struct SourceGroupValueTemplateContext<'a, 'b> {
    program: &'a SourceProgram,
    module: &'a SourceProgramModule,
    tokens: &'a [Token],
    body_cache: &'b mut SourceControlBodyCache,
    constant_values: &'a BTreeMap<String, FixedFileTemplateValue>,
    template_values: &'a SourceTemplateConstantValueCache,
    metadata_values: &'b mut SourceMetadataTemplateValueCache,
}

fn source_air_group_values_for_any_instance(
    context: &mut SourceGroupValueTemplateContext<'_, '_>,
    declaration: &AirGroupValueDeclaration,
    declaration_template: &AirTemplateDeclaration,
) -> Result<Option<Arc<BTreeMap<String, FixedFileTemplateValue>>>, SourceKeyDirectoryMetadataError>
{
    for instance in source_metadata_template_instances(context.program, &declaration_template.name)
    {
        let values = source_metadata_template_values_cached(
            context.program,
            context.module,
            declaration_template,
            Some(instance),
            context.constant_values,
            context.template_values,
            context.metadata_values,
        );
        if source_declaration_in_unselected_static_branch(
            context.program,
            context.module,
            context.tokens,
            context.body_cache,
            declaration.start,
            declaration.end,
            &values,
        )? {
            continue;
        }
        return Ok(Some(values));
    }
    Ok(None)
}

fn source_push_air_group_values(
    program: &SourceProgram,
    declaration: &AirGroupValueDeclaration,
    declaration_values: &BTreeMap<String, FixedFileTemplateValue>,
    seen: &mut BTreeSet<String>,
    values: &mut Vec<StageValue>,
    aggregation_types: &mut Vec<AggregationType>,
) -> Result<(), SourceKeyDirectoryMetadataError> {
    if declaration.stage == 0 {
        return unsupported("source air group value stage must be positive");
    }
    let aggregation_type = source_group_value_aggregation_type(&declaration.aggregate_type)?;
    if let Some(default_expression) = declaration.default_expression.as_ref() {
        let Some(default_value) =
            evaluate_source_static_expression(program, default_expression, declaration_values)
                .as_ref()
                .and_then(static_value_integer)
        else {
            return unsupported("source air group value defaults need proof lowering support");
        };
        let identity = match aggregation_type {
            0 => 0,
            1 => 1,
            _ => return unsupported("unsupported source air group value aggregation type"),
        };
        if default_value != identity {
            return unsupported("source air group value defaults need proof lowering support");
        }
    } else if declaration.default_value.is_some() {
        return unsupported("source air group value defaults need proof lowering support");
    }
    for item in &declaration.items {
        let name = source_item_name(program, item, "source air group value", declaration_values)?;
        if !seen.insert(name.clone()) {
            return unsupported("duplicate source air group value name");
        }
        let lengths =
            source_item_lengths(program, item, "source air group value", declaration_values)?;
        let dimension = source_column_dimension(&lengths, "source air group value")?;
        values.push(StageValue {
            name,
            stage: declaration.stage,
            lengths,
        });
        aggregation_types.extend((0..dimension).map(|_| AggregationType { aggregation_type }));
    }
    Ok(())
}

fn source_group_value_aggregation_type(
    name: &Option<String>,
) -> Result<u64, SourceKeyDirectoryMetadataError> {
    match name.as_deref() {
        Some("sum") => Ok(0),
        Some("prod") => Ok(1),
        Some(_) => unsupported("unsupported source air group value aggregation type"),
        None => unsupported("source air group value aggregation type is required"),
    }
}

pub(crate) fn unsupported_source_message(
    message: impl Into<String>,
) -> SourceKeyDirectoryMetadataError {
    SourceKeyDirectoryMetadataError::UnsupportedSourceProgram {
        message: message.into(),
    }
}

fn source_unit_setup_info(
    program: &SourceProgram,
    row_count: u64,
    unit_name: Option<(&str, &str)>,
    constant_values: &BTreeMap<String, FixedFileTemplateValue>,
    active_templates: &BTreeSet<String>,
    template_values: &SourceTemplateConstantValueCache,
    challenge_counts: &[u64],
    proof_values_map: &[NamedStageValue],
    publics_map: &[PublicValue],
    body_caches: &mut SourceControlBodyCaches,
) -> Result<UnitSetupInfo, SourceKeyDirectoryMetadataError> {
    let n_bits = row_count.trailing_zeros();
    let n_bits_ext = n_bits
        .checked_add(1)
        .ok_or_else(|| unsupported_source_message("source domain is too large"))?;
    let mut metadata_values = SourceMetadataTemplateValueCache::default();
    let constant_columns = source_constant_columns(
        program,
        unit_name,
        constant_values,
        active_templates,
        template_values,
        body_caches,
        &mut metadata_values,
    )?;
    let commitment_columns = source_commitment_columns(
        program,
        unit_name,
        constant_values,
        active_templates,
        template_values,
        body_caches,
        &mut metadata_values,
    )?;
    let unit_value_map = source_unit_values(
        program,
        unit_name,
        constant_values,
        active_templates,
        template_values,
        body_caches,
        &mut metadata_values,
    )?;
    let (group_value_map, _) = source_air_group_values_with_cache(
        program,
        unit_name,
        constant_values,
        active_templates,
        template_values,
        body_caches,
        &mut metadata_values,
    )?;
    let required_max_stage = source_required_setup_max_stage(
        &commitment_columns,
        &unit_value_map,
        &group_value_map,
        challenge_counts,
        proof_values_map,
        publics_map,
    )?;
    let (n_stages, commitment_widths) =
        source_commitment_section_widths(&commitment_columns, required_max_stage)?;
    let opening_points = source_opening_points(
        program,
        unit_name,
        constant_values,
        active_templates,
        template_values,
        body_caches,
    )?;
    let challenge_count = challenge_counts
        .iter()
        .copied()
        .try_fold(0_usize, |acc, count| {
            usize::try_from(count)
                .ok()
                .and_then(|count| acc.checked_add(count))
        })
        .ok_or_else(|| unsupported_source_message("source challenge count overflow"))?;
    let public_count = source_public_count(publics_map)?;
    let const_width = constant_columns
        .iter()
        .try_fold(0_u32, |acc, column| acc.checked_add(column.dimension))
        .ok_or_else(|| unsupported_source_message("source constant width overflow"))?;
    let mut section_widths = BTreeMap::from([("const".to_owned(), const_width)]);
    section_widths.extend(commitment_widths);

    Ok(UnitSetupInfo {
        n_stages,
        n_constants: const_width,
        constant_columns,
        n_publics: Some(
            u32::try_from(public_count)
                .map_err(|_| unsupported_source_message("too many source public values"))?,
        ),
        n_constraints: Some(0),
        q_degree: 3,
        opening_points,
        section_widths,
        challenge_count,
        eval_count: 0,
        evaluation_map: Vec::<EvaluationMapEntry>::new(),
        boundaries: Vec::<Boundary>::new(),
        commitment_columns,
        unit_value_map,
        group_value_map,
        stark: StarkStruct {
            n_bits,
            n_bits_ext,
            n_queries: 1,
            steps: source_fri_steps(n_bits_ext),
            hash_commits: true,
            last_level_verification: 2,
            pow_bits: 0,
            merkle_tree_arity: 4,
            verification_hash_type: Some("GL".to_owned()),
            transcript_arity: Some(4),
            merkle_tree_custom: Some(true),
        },
    })
}

fn source_required_setup_max_stage(
    commitment_columns: &[CommitmentColumn],
    unit_value_map: &[StageValue],
    group_value_map: &[StageValue],
    challenge_counts: &[u64],
    proof_values_map: &[NamedStageValue],
    publics_map: &[PublicValue],
) -> Result<u32, SourceKeyDirectoryMetadataError> {
    let mut max_stage = commitment_columns
        .iter()
        .map(|column| column.stage)
        .max()
        .unwrap_or(0);
    max_stage = max_stage.max(source_stage_values_max_stage(unit_value_map));
    max_stage = max_stage.max(source_stage_values_max_stage(group_value_map));
    max_stage = max_stage.max(
        u32::try_from(challenge_counts.len())
            .map_err(|_| unsupported_source_message("source challenge stage overflow"))?,
    );
    for value in proof_values_map {
        max_stage =
            max_stage
                .max(u32::try_from(value.stage).map_err(|_| {
                    unsupported_source_message("source proof value stage overflow")
                })?);
    }
    for value in publics_map {
        max_stage = max_stage.max(
            u32::try_from(value.stage)
                .map_err(|_| unsupported_source_message("source public value stage overflow"))?,
        );
    }
    Ok(max_stage)
}

fn source_stage_values_max_stage(values: &[StageValue]) -> u32 {
    values.iter().map(|value| value.stage).max().unwrap_or(0)
}

fn include_expression_opening_points(setup: &mut UnitSetupInfo, expressions: &ExpressionInfo) {
    for expression in &expressions.expressions {
        for operation in &expression.operations {
            include_operation_opening_points(&mut setup.opening_points, operation.sources.iter());
        }
    }
    for constraint in &expressions.constraints {
        for operation in &constraint.operations {
            include_operation_opening_points(&mut setup.opening_points, operation.sources.iter());
        }
    }
}

fn include_operation_opening_points<'a>(
    opening_points: &mut Vec<i64>,
    operands: impl Iterator<Item = &'a CodeOperand>,
) {
    for operand in operands {
        let prime = match operand {
            CodeOperand::ConstantAt { prime, .. }
            | CodeOperand::Commitment { prime, .. }
            | CodeOperand::CommitmentElement { prime, .. }
            | CodeOperand::CustomCommitment { prime, .. } => *prime,
            _ => None,
        };
        if let Some(prime) = prime {
            include_opening_point(opening_points, prime);
        }
    }
}

fn include_opening_point(opening_points: &mut Vec<i64>, point: i64) {
    if !opening_points.contains(&point) {
        opening_points.push(point);
    }
}

fn source_fri_steps(n_bits_ext: u32) -> Vec<FriStep> {
    let final_bits = if n_bits_ext == 1 { 0 } else { 1 };
    vec![
        FriStep { n_bits: n_bits_ext },
        FriStep { n_bits: final_bits },
    ]
}

fn source_constant_columns(
    program: &SourceProgram,
    unit_name: Option<(&str, &str)>,
    constant_values: &BTreeMap<String, FixedFileTemplateValue>,
    active_templates: &BTreeSet<String>,
    template_values: &SourceTemplateConstantValueCache,
    body_caches: &mut SourceControlBodyCaches,
    metadata_values: &mut SourceMetadataTemplateValueCache,
) -> Result<Vec<ConstantColumn>, SourceKeyDirectoryMetadataError> {
    let mut seen = BTreeSet::new();
    let mut columns = Vec::new();
    let mut next_position = 0_u32;
    let unit_instance = source_metadata_unit_instance(program, unit_name);
    for module in &program.modules {
        let tokens = lex_source(&module.source.contents).map_err(|source| {
            SourceKeyDirectoryMetadataError::Lex {
                source_name: module.source_name.clone(),
                source,
            }
        })?;
        let body_cache = body_caches.module_cache(&module.source_name);
        for declaration in &module.columns {
            if declaration.kind != ColumnKind::Fixed {
                continue;
            }
            let in_function_body =
                declaration_in_function_body(module, declaration.start, declaration.end);
            if in_function_body
                && !source_l1_fixed_column_needed(program, unit_instance, declaration)
            {
                continue;
            }
            if !in_function_body
                && declaration_in_inactive_template(
                    module,
                    declaration.start,
                    declaration.end,
                    active_templates,
                )
            {
                continue;
            }
            let Some(declaration_template) =
                source_metadata_declaration_template(module, declaration.start, declaration.end)
            else {
                let declaration_values = source_declaration_constant_values_from_cache(
                    module,
                    declaration.start,
                    declaration.end,
                    constant_values,
                    template_values,
                );
                if !in_function_body
                    && source_declaration_in_static_false_branch(
                        program,
                        module,
                        declaration.start,
                        declaration.end,
                        declaration_values,
                    )
                {
                    continue;
                }
                source_push_constant_columns(
                    program,
                    declaration,
                    declaration_values,
                    &mut seen,
                    &mut columns,
                    &mut next_position,
                )?;
                continue;
            };
            let declaration_values = if let Some(instance) = unit_instance {
                if declaration_template.name != instance.template {
                    continue;
                }
                source_metadata_template_values_cached(
                    program,
                    module,
                    declaration_template,
                    Some(instance),
                    constant_values,
                    template_values,
                    metadata_values,
                )
            } else {
                continue;
            };
            if !in_function_body
                && source_declaration_in_unselected_static_branch(
                    program,
                    module,
                    &tokens,
                    body_cache,
                    declaration.start,
                    declaration.end,
                    &declaration_values,
                )?
            {
                continue;
            }
            source_push_constant_columns(
                program,
                declaration,
                &declaration_values,
                &mut seen,
                &mut columns,
                &mut next_position,
            )?;
        }
    }
    Ok(columns)
}

fn source_l1_fixed_column_needed(
    program: &SourceProgram,
    unit_instance: Option<&AirInstanceDeclaration>,
    declaration: &ColumnDeclaration,
) -> bool {
    declaration
        .items
        .iter()
        .any(|item| item.name == "air.__L1__")
        && source_unit_template_calls_get_l1(program, unit_instance)
}

fn source_unit_template_calls_get_l1(
    program: &SourceProgram,
    unit_instance: Option<&AirInstanceDeclaration>,
) -> bool {
    let Some(unit_instance) = unit_instance else {
        return false;
    };
    program.modules.iter().any(|module| {
        module.air_templates.iter().any(|template| {
            template.name == unit_instance.template
                && module
                    .source
                    .contents
                    .get(template.body.start..template.body.end)
                    .is_some_and(|body| body.contains("get_L1"))
        })
    })
}

fn source_push_constant_columns(
    program: &SourceProgram,
    declaration: &ColumnDeclaration,
    declaration_values: &BTreeMap<String, FixedFileTemplateValue>,
    seen: &mut BTreeSet<String>,
    columns: &mut Vec<ConstantColumn>,
    next_position: &mut u32,
) -> Result<(), SourceKeyDirectoryMetadataError> {
    for item in &declaration.items {
        let name = source_item_name(program, item, "source fixed-column", declaration_values)?;
        if !seen.insert(name.clone()) {
            continue;
        }
        let lengths =
            source_item_lengths(program, item, "source fixed-column", declaration_values)?;
        let dimension = source_column_dimension(&lengths, "source fixed-column")?;
        let id = *next_position;
        *next_position = next_position
            .checked_add(dimension)
            .ok_or_else(|| unsupported_source_message("source constant width overflow"))?;
        columns.push(ConstantColumn {
            name,
            stage: 0,
            dimension,
            pols_map_id: id,
            stage_id: id,
            lengths,
        });
    }
    Ok(())
}

fn source_commitment_columns(
    program: &SourceProgram,
    unit_name: Option<(&str, &str)>,
    constant_values: &BTreeMap<String, FixedFileTemplateValue>,
    active_templates: &BTreeSet<String>,
    template_values: &SourceTemplateConstantValueCache,
    body_caches: &mut SourceControlBodyCaches,
    metadata_values: &mut SourceMetadataTemplateValueCache,
) -> Result<Vec<CommitmentColumn>, SourceKeyDirectoryMetadataError> {
    let mut seen = BTreeSet::new();
    let mut columns = Vec::new();
    let mut stages = BTreeMap::<u32, SourceCommitmentStageCursor>::new();
    let unit_instance = source_metadata_unit_instance(program, unit_name);
    for module in &program.modules {
        let tokens = lex_source(&module.source.contents).map_err(|source| {
            SourceKeyDirectoryMetadataError::Lex {
                source_name: module.source_name.clone(),
                source,
            }
        })?;
        let body_cache = body_caches.module_cache(&module.source_name);
        for declaration in &module.columns {
            if matches!(declaration.kind, ColumnKind::Fixed) {
                continue;
            }
            if declaration_in_function_body(module, declaration.start, declaration.end) {
                continue;
            }
            if declaration_in_inactive_template(
                module,
                declaration.start,
                declaration.end,
                active_templates,
            ) {
                continue;
            }
            let Some(declaration_template) =
                source_metadata_declaration_template(module, declaration.start, declaration.end)
            else {
                let declaration_values = source_declaration_constant_values_from_cache(
                    module,
                    declaration.start,
                    declaration.end,
                    constant_values,
                    template_values,
                );
                if source_declaration_in_static_false_branch(
                    program,
                    module,
                    declaration.start,
                    declaration.end,
                    declaration_values,
                ) {
                    continue;
                }
                source_push_commitment_columns(
                    program,
                    declaration,
                    declaration_values,
                    &mut seen,
                    &mut stages,
                    &mut columns,
                )?;
                continue;
            };
            let declaration_values = if let Some(instance) = unit_instance {
                if declaration_template.name != instance.template {
                    continue;
                }
                source_metadata_template_values_cached(
                    program,
                    module,
                    declaration_template,
                    Some(instance),
                    constant_values,
                    template_values,
                    metadata_values,
                )
            } else {
                source_metadata_template_values_cached(
                    program,
                    module,
                    declaration_template,
                    None,
                    constant_values,
                    template_values,
                    metadata_values,
                )
            };
            if source_declaration_in_unselected_static_branch(
                program,
                module,
                &tokens,
                body_cache,
                declaration.start,
                declaration.end,
                &declaration_values,
            )? {
                continue;
            }
            source_push_commitment_columns(
                program,
                declaration,
                &declaration_values,
                &mut seen,
                &mut stages,
                &mut columns,
            )?;
        }
    }
    Ok(columns)
}

#[derive(Debug, Clone, Copy, Default)]
struct SourceCommitmentStageCursor {
    next_id: u32,
    next_position: u32,
}

fn source_push_commitment_columns(
    program: &SourceProgram,
    declaration: &ColumnDeclaration,
    declaration_values: &BTreeMap<String, FixedFileTemplateValue>,
    seen: &mut BTreeSet<String>,
    stages: &mut BTreeMap<u32, SourceCommitmentStageCursor>,
    columns: &mut Vec<CommitmentColumn>,
) -> Result<(), SourceKeyDirectoryMetadataError> {
    let stage = source_column_stage(program, declaration, declaration_values)?;
    for item in &declaration.items {
        let name = source_item_name(
            program,
            item,
            "source commitment-column",
            declaration_values,
        )?;
        if !seen.insert(name.clone()) {
            continue;
        }
        let lengths = source_item_lengths(
            program,
            item,
            "source commitment-column",
            declaration_values,
        )?;
        let dimension = source_column_dimension(&lengths, "source commitment-column")?;
        let cursor = stages.entry(stage).or_default();
        let stage_id = cursor.next_id;
        let stage_position = cursor.next_position;
        cursor.next_id = cursor
            .next_id
            .checked_add(1)
            .ok_or_else(|| unsupported_source_message("source commitment stage id overflow"))?;
        cursor.next_position = cursor
            .next_position
            .checked_add(dimension)
            .ok_or_else(|| unsupported_source_message("source commitment stage width overflow"))?;
        let pols_map_id = u32::try_from(columns.len())
            .map_err(|_| unsupported_source_message("too many source commitment columns"))?;
        columns.push(CommitmentColumn {
            name,
            stage,
            dimension,
            pols_map_id,
            stage_id,
            stage_position,
            intermediate: declaration.kind == ColumnKind::Custom,
            lengths,
        });
    }
    Ok(())
}

fn source_commitment_section_widths(
    columns: &[CommitmentColumn],
    required_max_stage: u32,
) -> Result<(u32, BTreeMap<String, u32>), SourceKeyDirectoryMetadataError> {
    if columns.is_empty() {
        let max_stage = required_max_stage.max(2);
        let mut widths = BTreeMap::new();
        for stage in 1..=max_stage {
            widths.insert(format!("cm{stage}"), 1);
        }
        return Ok((
            max_stage
                .checked_sub(1)
                .ok_or_else(|| unsupported_source_message("source commitment stage underflow"))?,
            widths,
        ));
    }

    let mut widths = BTreeMap::<u32, u32>::new();
    for column in columns {
        let end = column
            .stage_position
            .checked_add(column.dimension)
            .ok_or_else(|| unsupported_source_message("source commitment stage width overflow"))?;
        widths
            .entry(column.stage)
            .and_modify(|width| *width = (*width).max(end))
            .or_insert(end);
    }
    let max_stage = widths
        .keys()
        .copied()
        .next_back()
        .ok_or_else(|| unsupported_source_message("source commitment stage set is empty"))?
        .max(required_max_stage);
    let mut section_widths = BTreeMap::new();
    for stage in 1..=max_stage {
        let width = widths.get(&stage).copied().unwrap_or(1);
        section_widths.insert(format!("cm{stage}"), width);
    }
    let n_stages = max_stage
        .checked_sub(1)
        .ok_or_else(|| unsupported_source_message("source commitment stage underflow"))?;
    Ok((n_stages, section_widths))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use lzvm_artifacts::key_directory::{KeyUnitKind, KeyUnitPaths};

    use super::source_unit_metadata_payload_is_last_writer;

    #[test]
    fn metadata_payload_last_writer_skips_overwritten_shared_paths() {
        let shared_metadata = PathBuf::from("root/program/group/recursive2/recursive2");
        let units = vec![
            key_unit(
                KeyUnitKind::Basic,
                Some(0),
                Some(0),
                "root/program/group/airs/a/air/a",
                "root/program/group/airs/a/air/a",
            ),
            key_unit(
                KeyUnitKind::RecursiveFirst,
                Some(0),
                Some(0),
                "root/program/group/airs/a/recursive1/recursive1",
                shared_metadata.clone(),
            ),
            key_unit(
                KeyUnitKind::Basic,
                Some(0),
                Some(1),
                "root/program/group/airs/b/air/b",
                "root/program/group/airs/b/air/b",
            ),
            key_unit(
                KeyUnitKind::RecursiveFirst,
                Some(0),
                Some(1),
                "root/program/group/airs/b/recursive1/recursive1",
                shared_metadata.clone(),
            ),
            key_unit(
                KeyUnitKind::RecursiveSecond,
                Some(0),
                None,
                shared_metadata.clone(),
                shared_metadata,
            ),
        ];

        assert!(source_unit_metadata_payload_is_last_writer(&units, 0));
        assert!(!source_unit_metadata_payload_is_last_writer(&units, 1));
        assert!(source_unit_metadata_payload_is_last_writer(&units, 2));
        assert!(!source_unit_metadata_payload_is_last_writer(&units, 3));
        assert!(source_unit_metadata_payload_is_last_writer(&units, 4));
    }

    fn key_unit(
        kind: KeyUnitKind,
        group_id: Option<usize>,
        unit_id: Option<usize>,
        prefix: impl Into<PathBuf>,
        metadata_prefix: impl Into<PathBuf>,
    ) -> KeyUnitPaths {
        let prefix = prefix.into();
        KeyUnitPaths {
            kind,
            group_id,
            unit_id,
            group_name: group_id.map(|_| "group".to_owned()),
            unit_name: unit_id.map(|id| format!("unit{id}")),
            prefix: prefix.clone(),
            metadata_prefix: Some(metadata_prefix.into()),
            program_prefix: Some(prefix.clone()),
            verification_key_prefix: prefix.clone(),
            fixed_columns: prefix.with_extension("const"),
            constant_tree: prefix.with_extension("consttree"),
        }
    }
}
