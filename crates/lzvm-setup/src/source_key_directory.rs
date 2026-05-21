use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Path, PathBuf};

use lzvm_artifacts::expression_info::ExpressionInfoError;
use lzvm_artifacts::global_info::{
    encode_global_info, AggregationType, CurveKind, GlobalAir, GlobalInfo, GlobalInfoError,
    NamedStageValue, PublicValue,
};
use lzvm_artifacts::global_program::{encode_global_program, GlobalProgramError};
use lzvm_artifacts::key_directory::{read_key_directory_layout, KeyDirectoryError, KeyUnitPaths};
use lzvm_artifacts::setup_info::{
    encode_unit_setup_info, Boundary, CommitmentColumn, ConstantColumn, EvaluationMapEntry,
    FriStep, SetupInfoError, StageValue, StarkStruct, UnitSetupInfo,
};
use lzvm_artifacts::verifier_info::{encode_verifier_info, VerifierInfoError};
use lzvm_pil::{
    lex_source, AirGroupValueDeclaration, AirTemplateDeclaration, BinaryOperator,
    ColumnDeclaration, ColumnItem, ColumnKind, Expression, ExpressionKind, FixedFileTemplateValue,
    LexError, ParseError, PublicDeclaration, SourceLoaderConfig, SourceProgram, SourceProgramError,
    SourceProgramLoader, SourceProgramModule, Token, UnaryOperator, ValueDeclarationKind,
};

use crate::{
    publish_staging_bytes,
    source_control_body_cache::{SourceControlBodyCache, SourceControlBodyCaches},
    source_expression_info::source_expression_info,
    source_global_constraints::source_global_program,
    source_metadata_template::{
        source_declaration_in_unselected_static_branch, source_metadata_declaration_template,
        source_metadata_template_instances, source_metadata_template_values,
        source_metadata_unit_instance,
    },
    source_opening_points::source_opening_points,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SourceChallengeShape {
    stage: usize,
    dimension: u32,
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
        let active_templates = concrete_template_names(&program);
        for unit in &layout.units {
            let Some(setup_path) = unit.setup_info_binary() else {
                continue;
            };
            let row_count = source_layout_unit_row_count(unit, &row_counts)?;
            let mut setup_info = source_unit_setup_info(
                &program,
                row_count,
                unit.group_name.as_deref().zip(unit.unit_name.as_deref()),
                &mut body_caches,
            )?;
            let unit_constant_values = source_scalar_constant_values(&program, row_count);
            let unit_template_values =
                source_template_constant_value_cache(&program, &unit_constant_values);
            let challenge_slots = source_challenge_slots(
                &program,
                &unit_constant_values,
                &active_templates,
                &unit_template_values,
            )?;
            let expression_info = source_expression_info(
                &program,
                &setup_info,
                unit.group_name.as_deref().zip(unit.unit_name.as_deref()),
                &global_info.publics_map,
                &challenge_slots,
                &global_info.proof_values_map,
                &mut body_caches,
            )?;
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
                setup_path,
                setup_bytes,
                expression_path: unit.expression_info_binary(),
                expression_bytes,
                verifier_path: unit.verifier_info_binary(),
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

fn unsupported<T>(message: impl Into<String>) -> Result<T, SourceKeyDirectoryMetadataError> {
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
    )?;
    let (num_proof_values, proof_values_map) = source_proof_values(
        program,
        &constant_values,
        &active_templates,
        &template_values,
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
        n_publics: u64::try_from(publics_map.len())
            .map_err(|_| unsupported_source_message("too many source public values"))?,
        num_challenges,
        num_proof_values,
        proof_values_map,
        publics_map,
        transcript_arity: 4,
    })
}

fn source_public_values(
    program: &SourceProgram,
    constant_values: &BTreeMap<String, FixedFileTemplateValue>,
    active_templates: &BTreeSet<String>,
    template_values: &SourceTemplateConstantValueCache,
    body_caches: &mut SourceControlBodyCaches,
) -> Result<Vec<PublicValue>, SourceKeyDirectoryMetadataError> {
    let mut seen = BTreeSet::new();
    let mut values = Vec::new();
    for module in &program.modules {
        let tokens = lex_source(&module.source.contents).map_err(|source| {
            SourceKeyDirectoryMetadataError::Lex {
                source_name: module.source_name.clone(),
                source,
            }
        })?;
        let body_cache = body_caches.module_cache(&module.source_name);
        let mut template_context = SourcePublicTemplateContext {
            program,
            module,
            tokens: &tokens,
            body_cache,
            constant_values,
            template_values,
        };
        for declaration in &module.publics {
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
                source_push_public_values(
                    program,
                    declaration,
                    declaration_values,
                    &mut seen,
                    &mut values,
                )?;
                continue;
            };
            let Some(declaration_values) = source_public_values_for_any_instance(
                &mut template_context,
                declaration,
                declaration_template,
            )?
            else {
                continue;
            };
            source_push_public_values(
                program,
                declaration,
                &declaration_values,
                &mut seen,
                &mut values,
            )?;
        }
    }
    Ok(values)
}

struct SourcePublicTemplateContext<'a, 'b> {
    program: &'a SourceProgram,
    module: &'a SourceProgramModule,
    tokens: &'a [Token],
    body_cache: &'b mut SourceControlBodyCache,
    constant_values: &'a BTreeMap<String, FixedFileTemplateValue>,
    template_values: &'a SourceTemplateConstantValueCache,
}

fn source_public_values_for_any_instance(
    context: &mut SourcePublicTemplateContext<'_, '_>,
    declaration: &PublicDeclaration,
    declaration_template: &AirTemplateDeclaration,
) -> Result<Option<BTreeMap<String, FixedFileTemplateValue>>, SourceKeyDirectoryMetadataError> {
    for instance in source_metadata_template_instances(context.program, &declaration_template.name)
    {
        let values = source_metadata_template_values(
            context.program,
            context.module,
            declaration_template,
            Some(instance),
            context.constant_values,
            context.template_values,
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

fn source_push_public_values(
    program: &SourceProgram,
    declaration: &PublicDeclaration,
    declaration_values: &BTreeMap<String, FixedFileTemplateValue>,
    seen: &mut BTreeSet<String>,
    values: &mut Vec<PublicValue>,
) -> Result<(), SourceKeyDirectoryMetadataError> {
    if declaration.initializer.is_some() {
        return unsupported("source public initializers need metadata lowering support");
    }
    for item in &declaration.items {
        if item.template {
            return unsupported("template public-value names need instance lowering support");
        }
        if !seen.insert(item.name.clone()) {
            return unsupported("duplicate source public value name");
        }
        values.push(PublicValue {
            name: item.name.clone(),
            stage: 1,
            lengths: source_item_lengths(program, item, "source public value", declaration_values)?
                .into_iter()
                .map(u64::from)
                .collect(),
        });
    }
    Ok(())
}

fn source_proof_values(
    program: &SourceProgram,
    constant_values: &BTreeMap<String, FixedFileTemplateValue>,
    active_templates: &BTreeSet<String>,
    template_values: &SourceTemplateConstantValueCache,
) -> Result<(Vec<u64>, Vec<NamedStageValue>), SourceKeyDirectoryMetadataError> {
    let mut seen = BTreeSet::new();
    let mut counts_by_stage = Vec::<u64>::new();
    let mut values = Vec::new();
    for module in &program.modules {
        for declaration in &module.values {
            if declaration.kind != ValueDeclarationKind::ProofValue {
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
            let stage = usize::try_from(declaration.stage)
                .map_err(|_| unsupported_source_message("source proof value stage overflow"))?;
            if stage == 0 {
                return unsupported("source proof value stage must be positive");
            }
            if counts_by_stage.len() < stage {
                counts_by_stage.resize(stage, 0);
            }
            for item in &declaration.items {
                if item.template {
                    return unsupported(
                        "template proof-value names need instance lowering support",
                    );
                }
                if !seen.insert(item.name.clone()) {
                    return unsupported("duplicate source proof value name");
                }
                counts_by_stage[stage - 1] =
                    counts_by_stage[stage - 1].checked_add(1).ok_or_else(|| {
                        unsupported_source_message("source proof value count overflow")
                    })?;
                values.push(NamedStageValue {
                    name: item.name.clone(),
                    stage: u64::from(declaration.stage),
                    id: None,
                    lengths: source_item_lengths(
                        program,
                        item,
                        "source proof value",
                        declaration_values,
                    )?
                    .into_iter()
                    .map(u64::from)
                    .collect(),
                });
            }
        }
    }
    Ok((counts_by_stage, values))
}

fn source_challenge_counts(
    program: &SourceProgram,
    constant_values: &BTreeMap<String, FixedFileTemplateValue>,
    active_templates: &BTreeSet<String>,
    template_values: &SourceTemplateConstantValueCache,
) -> Result<Vec<u64>, SourceKeyDirectoryMetadataError> {
    let slots =
        source_challenge_slots(program, constant_values, active_templates, template_values)?;
    let mut counts_by_stage = Vec::<u64>::new();
    for slot in slots {
        let stage_index = usize::try_from(slot.stage)
            .map_err(|_| unsupported_source_message("source challenge stage overflow"))?
            .checked_sub(1)
            .ok_or_else(|| unsupported_source_message("source challenge stage underflow"))?;
        if counts_by_stage.len() <= stage_index {
            counts_by_stage.resize(stage_index + 1, 0);
        }
        counts_by_stage[stage_index] = counts_by_stage[stage_index]
            .checked_add(u64::from(slot.dimension))
            .ok_or_else(|| unsupported_source_message("source challenge count overflow"))?;
    }
    Ok(counts_by_stage)
}

fn source_challenge_slots(
    program: &SourceProgram,
    constant_values: &BTreeMap<String, FixedFileTemplateValue>,
    active_templates: &BTreeSet<String>,
    template_values: &SourceTemplateConstantValueCache,
) -> Result<Vec<SourceChallengeSlotMetadata>, SourceKeyDirectoryMetadataError> {
    let mut seen = BTreeMap::<String, SourceChallengeShape>::new();
    let mut slots = Vec::<SourceChallengeSlotMetadata>::new();
    let mut next_stage_ids = BTreeMap::<usize, u32>::new();
    for module in &program.modules {
        for declaration in &module.values {
            if declaration.kind != ValueDeclarationKind::Challenge {
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
            let stage = usize::try_from(declaration.stage)
                .map_err(|_| unsupported_source_message("source challenge stage overflow"))?;
            if stage == 0 {
                return unsupported("source challenge stage must be positive");
            }
            for item in &declaration.items {
                if item.template {
                    return unsupported("template challenge names need instance lowering support");
                }
                let lengths =
                    source_item_lengths(program, item, "source challenge", declaration_values)?;
                let dimension = source_column_dimension(&lengths, "source challenge")?;
                let shape = SourceChallengeShape { stage, dimension };
                if let Some(existing) = seen.get(&item.name) {
                    if *existing != shape {
                        return unsupported("duplicate source challenge name");
                    }
                    continue;
                }
                seen.insert(item.name.clone(), shape);
                let stage_id = *next_stage_ids.get(&stage).unwrap_or(&0);
                next_stage_ids.insert(
                    stage,
                    stage_id.checked_add(dimension).ok_or_else(|| {
                        unsupported_source_message("source challenge id overflow")
                    })?,
                );
                slots.push(SourceChallengeSlotMetadata {
                    name: item.name.clone(),
                    id: 0,
                    stage: u32::try_from(stage).map_err(|_| {
                        unsupported_source_message("source challenge stage overflow")
                    })?,
                    stage_id,
                    dimension,
                });
            }
        }
    }

    let max_stage = next_stage_ids.keys().copied().max().unwrap_or(0);
    let mut stage_bases = vec![0_u32; max_stage];
    let mut cursor = 0_u32;
    for stage in 1..=max_stage {
        stage_bases[stage - 1] = cursor;
        cursor = cursor
            .checked_add(*next_stage_ids.get(&stage).unwrap_or(&0))
            .ok_or_else(|| unsupported_source_message("source challenge id overflow"))?;
    }

    for slot in &mut slots {
        let stage_index = usize::try_from(slot.stage)
            .map_err(|_| unsupported_source_message("source challenge stage overflow"))?
            .checked_sub(1)
            .ok_or_else(|| unsupported_source_message("source challenge stage underflow"))?;
        slot.id = stage_bases
            .get(stage_index)
            .copied()
            .ok_or_else(|| unsupported_source_message("source challenge stage overflow"))?
            .checked_add(slot.stage_id)
            .ok_or_else(|| unsupported_source_message("source challenge id overflow"))?;
    }

    Ok(slots)
}

fn source_unit_values(
    program: &SourceProgram,
    unit_name: Option<(&str, &str)>,
    constant_values: &BTreeMap<String, FixedFileTemplateValue>,
    active_templates: &BTreeSet<String>,
    template_values: &SourceTemplateConstantValueCache,
    body_caches: &mut SourceControlBodyCaches,
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
            let declaration_values = source_metadata_template_values(
                program,
                module,
                declaration_template,
                unit_instance,
                constant_values,
                template_values,
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
        if item.template {
            return unsupported("template air-value names need instance lowering support");
        }
        let lengths = source_item_lengths(program, item, "source air value", declaration_values)?;
        let shape = (stage, lengths.clone());
        if let Some(existing) = seen.get(&item.name) {
            if *existing != shape {
                return unsupported("duplicate source air value name");
            }
            continue;
        }
        seen.insert(item.name.clone(), shape);
        values.push(StageValue {
            name: item.name.clone(),
            stage,
            lengths,
        });
    }
    Ok(())
}

fn source_air_group_values(
    program: &SourceProgram,
    unit_name: Option<(&str, &str)>,
    constant_values: &BTreeMap<String, FixedFileTemplateValue>,
    active_templates: &BTreeSet<String>,
    template_values: &SourceTemplateConstantValueCache,
    body_caches: &mut SourceControlBodyCaches,
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
                let values = source_metadata_template_values(
                    program,
                    module,
                    declaration_template,
                    Some(instance),
                    constant_values,
                    template_values,
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
}

fn source_air_group_values_for_any_instance(
    context: &mut SourceGroupValueTemplateContext<'_, '_>,
    declaration: &AirGroupValueDeclaration,
    declaration_template: &AirTemplateDeclaration,
) -> Result<Option<BTreeMap<String, FixedFileTemplateValue>>, SourceKeyDirectoryMetadataError> {
    for instance in source_metadata_template_instances(context.program, &declaration_template.name)
    {
        let values = source_metadata_template_values(
            context.program,
            context.module,
            declaration_template,
            Some(instance),
            context.constant_values,
            context.template_values,
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
        if item.template {
            return unsupported("template air-group-value names need instance lowering support");
        }
        if !seen.insert(item.name.clone()) {
            return unsupported("duplicate source air group value name");
        }
        values.push(StageValue {
            name: item.name.clone(),
            stage: declaration.stage,
            lengths: source_item_lengths(
                program,
                item,
                "source air group value",
                declaration_values,
            )?,
        });
        aggregation_types.push(AggregationType { aggregation_type });
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

fn unsupported_source_message(message: impl Into<String>) -> SourceKeyDirectoryMetadataError {
    SourceKeyDirectoryMetadataError::UnsupportedSourceProgram {
        message: message.into(),
    }
}

fn source_unit_setup_info(
    program: &SourceProgram,
    row_count: u64,
    unit_name: Option<(&str, &str)>,
    body_caches: &mut SourceControlBodyCaches,
) -> Result<UnitSetupInfo, SourceKeyDirectoryMetadataError> {
    let n_bits = row_count.trailing_zeros();
    let n_bits_ext = n_bits
        .checked_add(1)
        .ok_or_else(|| unsupported_source_message("source domain is too large"))?;
    let constant_values = source_scalar_constant_values(program, row_count);
    let template_values = source_template_constant_value_cache(program, &constant_values);
    let active_templates = concrete_template_names(program);
    let constant_columns = source_constant_columns(
        program,
        &constant_values,
        &active_templates,
        &template_values,
    )?;
    let commitment_columns = source_commitment_columns(
        program,
        &constant_values,
        &active_templates,
        &template_values,
    )?;
    let (n_stages, commitment_widths) = source_commitment_section_widths(&commitment_columns)?;
    let unit_value_map = source_unit_values(
        program,
        unit_name,
        &constant_values,
        &active_templates,
        &template_values,
        body_caches,
    )?;
    let (group_value_map, _) = source_air_group_values(
        program,
        unit_name,
        &constant_values,
        &active_templates,
        &template_values,
        body_caches,
    )?;
    let opening_points = source_opening_points(
        program,
        unit_name,
        &constant_values,
        &active_templates,
        &template_values,
        body_caches,
    )?;
    let challenge_count = source_challenge_counts(
        program,
        &constant_values,
        &active_templates,
        &template_values,
    )?
    .into_iter()
    .try_fold(0_usize, |acc, count| {
        usize::try_from(count)
            .ok()
            .and_then(|count| acc.checked_add(count))
    })
    .ok_or_else(|| unsupported_source_message("source challenge count overflow"))?;
    let public_count = source_public_values(
        program,
        &constant_values,
        &active_templates,
        &template_values,
        body_caches,
    )?
    .len();
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
            steps: vec![FriStep { n_bits: n_bits_ext }, FriStep { n_bits: 1 }],
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

fn source_constant_columns(
    program: &SourceProgram,
    constant_values: &BTreeMap<String, FixedFileTemplateValue>,
    active_templates: &BTreeSet<String>,
    template_values: &SourceTemplateConstantValueCache,
) -> Result<Vec<ConstantColumn>, SourceKeyDirectoryMetadataError> {
    let mut seen = BTreeSet::new();
    let mut columns = Vec::new();
    let mut next_position = 0_u32;
    for module in &program.modules {
        for declaration in &module.columns {
            if declaration.kind != ColumnKind::Fixed {
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
            for item in &declaration.items {
                if item.template {
                    return unsupported(format!(
                        "template fixed-column names need instance lowering support in {} at {}",
                        declaration.source_name, declaration.start
                    ));
                }
                if !seen.insert(item.name.clone()) {
                    continue;
                }
                let lengths =
                    source_item_lengths(program, item, "source fixed-column", declaration_values)?;
                let dimension = source_column_dimension(&lengths, "source fixed-column")?;
                let id = next_position;
                next_position = next_position
                    .checked_add(dimension)
                    .ok_or_else(|| unsupported_source_message("source constant width overflow"))?;
                columns.push(ConstantColumn {
                    name: item.name.clone(),
                    stage: 0,
                    dimension,
                    pols_map_id: id,
                    stage_id: id,
                    lengths,
                });
            }
        }
    }
    Ok(columns)
}

fn source_commitment_columns(
    program: &SourceProgram,
    constant_values: &BTreeMap<String, FixedFileTemplateValue>,
    active_templates: &BTreeSet<String>,
    template_values: &SourceTemplateConstantValueCache,
) -> Result<Vec<CommitmentColumn>, SourceKeyDirectoryMetadataError> {
    let mut seen = BTreeSet::new();
    let mut columns = Vec::new();
    let mut stages = BTreeMap::<u32, SourceCommitmentStageCursor>::new();
    for module in &program.modules {
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
            let stage = source_column_stage(program, declaration, declaration_values)?;
            for item in &declaration.items {
                if item.template {
                    return unsupported(format!(
                        "template commitment-column names need instance lowering support in {} at {}",
                        declaration.source_name, declaration.start
                    ));
                }
                if !seen.insert(item.name.clone()) {
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
                cursor.next_id = cursor.next_id.checked_add(1).ok_or_else(|| {
                    unsupported_source_message("source commitment stage id overflow")
                })?;
                cursor.next_position =
                    cursor.next_position.checked_add(dimension).ok_or_else(|| {
                        unsupported_source_message("source commitment stage width overflow")
                    })?;
                let pols_map_id = u32::try_from(columns.len()).map_err(|_| {
                    unsupported_source_message("too many source commitment columns")
                })?;
                columns.push(CommitmentColumn {
                    name: item.name.clone(),
                    stage,
                    dimension,
                    pols_map_id,
                    stage_id,
                    stage_position,
                    intermediate: declaration.kind == ColumnKind::Custom,
                    lengths,
                });
            }
        }
    }
    Ok(columns)
}

#[derive(Debug, Clone, Copy, Default)]
struct SourceCommitmentStageCursor {
    next_id: u32,
    next_position: u32,
}

fn source_commitment_section_widths(
    columns: &[CommitmentColumn],
) -> Result<(u32, BTreeMap<String, u32>), SourceKeyDirectoryMetadataError> {
    if columns.is_empty() {
        return Ok((
            1,
            BTreeMap::from([("cm1".to_owned(), 1), ("cm2".to_owned(), 1)]),
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
    let max_stage = *widths
        .keys()
        .next_back()
        .ok_or_else(|| unsupported_source_message("source commitment stage set is empty"))?;
    let mut section_widths = BTreeMap::new();
    for stage in 1..=max_stage {
        let Some(width) = widths.get(&stage).copied() else {
            return unsupported("source commitment stages must be contiguous");
        };
        section_widths.insert(format!("cm{stage}"), width);
    }
    let n_stages = max_stage
        .checked_sub(1)
        .ok_or_else(|| unsupported_source_message("source commitment stage underflow"))?;
    Ok((n_stages, section_widths))
}

fn source_column_stage(
    program: &SourceProgram,
    declaration: &ColumnDeclaration,
    constant_values: &BTreeMap<String, FixedFileTemplateValue>,
) -> Result<u32, SourceKeyDirectoryMetadataError> {
    let mut stage = None;
    for feature in &declaration.features {
        if feature.name != "stage" {
            continue;
        }
        if stage.is_some() {
            return unsupported("duplicate source column stage feature");
        }
        let Some(args) = feature.args_expressions.as_ref() else {
            return unsupported("source column stage must be static");
        };
        let [expression] = args.as_slice() else {
            return unsupported("source column stage must have one argument");
        };
        stage = Some(eval_u32_expression_with_values(
            program,
            expression,
            constant_values,
        )?);
    }
    let stage = stage.unwrap_or(1);
    if stage == 0 {
        return unsupported("source commitment column stage must be positive");
    }
    Ok(stage)
}

fn source_column_dimension(
    lengths: &[u32],
    item_role: &str,
) -> Result<u32, SourceKeyDirectoryMetadataError> {
    if lengths.is_empty() {
        return Ok(1);
    }
    lengths
        .iter()
        .try_fold(1_u32, |acc, length| acc.checked_mul(*length))
        .ok_or_else(|| unsupported_source_message(format!("{item_role} dimension overflow")))
}

fn source_item_lengths(
    program: &SourceProgram,
    item: &ColumnItem,
    item_role: &str,
    constant_values: &BTreeMap<String, FixedFileTemplateValue>,
) -> Result<Vec<u32>, SourceKeyDirectoryMetadataError> {
    let mut lengths = Vec::with_capacity(item.array_dim_expressions.len());
    for expression in &item.array_dim_expressions {
        let Some(expression) = expression else {
            return unsupported(format!("{item_role} array dimensions must be static"));
        };
        let value = eval_u32_expression_with_values(program, expression, constant_values)?;
        if value == 0 {
            return unsupported(format!("{item_role} array dimensions must be positive"));
        }
        lengths.push(value);
    }
    Ok(lengths)
}

fn eval_u32_expression(expression: &Expression) -> Result<u32, SourceKeyDirectoryMetadataError> {
    let value = eval_i128_expression(expression)?;
    u32::try_from(value)
        .map_err(|_| unsupported_source_message("source expression is out of range"))
}

fn eval_u32_expression_with_values(
    program: &SourceProgram,
    expression: &Expression,
    values: &BTreeMap<String, FixedFileTemplateValue>,
) -> Result<u32, SourceKeyDirectoryMetadataError> {
    if let Some(value) = evaluate_source_static_expression(program, expression, values) {
        if let Some(value) = static_value_integer(&value) {
            return u32::try_from(value)
                .map_err(|_| unsupported_source_message("source expression is out of range"));
        }
    }
    eval_u32_expression(expression)
}

fn eval_i128_expression(expression: &Expression) -> Result<i128, SourceKeyDirectoryMetadataError> {
    match &expression.kind {
        ExpressionKind::Integer(value) | ExpressionKind::HexInteger(value) => {
            parse_i128_literal(value)
        }
        ExpressionKind::Group(value) => eval_i128_expression(value),
        ExpressionKind::Unary { op, expr } => {
            let value = eval_i128_expression(expr)?;
            match op {
                UnaryOperator::Plus => Ok(value),
                UnaryOperator::Minus => value
                    .checked_neg()
                    .ok_or_else(|| unsupported_source_message("source expression overflow")),
                _ => unsupported("unsupported source unary expression"),
            }
        }
        ExpressionKind::Binary { op, left, right } => {
            let left = eval_i128_expression(left)?;
            let right = eval_i128_expression(right)?;
            match op {
                BinaryOperator::Add => left
                    .checked_add(right)
                    .ok_or_else(|| unsupported_source_message("source expression overflow")),
                BinaryOperator::Subtract => left
                    .checked_sub(right)
                    .ok_or_else(|| unsupported_source_message("source expression overflow")),
                BinaryOperator::Multiply => left
                    .checked_mul(right)
                    .ok_or_else(|| unsupported_source_message("source expression overflow")),
                BinaryOperator::Divide if right != 0 => Ok(left / right),
                BinaryOperator::Modulo if right != 0 => Ok(left % right),
                _ => unsupported("unsupported source binary expression"),
            }
        }
        _ => unsupported(format!(
            "unsupported source expression in {} at {}",
            expression.source_name, expression.start
        )),
    }
}

fn parse_i128_literal(value: &str) -> Result<i128, SourceKeyDirectoryMetadataError> {
    let value = value.trim().replace('_', "");
    if let Some(hex) = value
        .strip_prefix("-0x")
        .or_else(|| value.strip_prefix("-0X"))
    {
        let parsed = i128::from_str_radix(hex, 16)
            .map_err(|_| unsupported_source_message("invalid source integer literal"))?;
        parsed
            .checked_neg()
            .ok_or_else(|| unsupported_source_message("source integer literal overflow"))
    } else if let Some(hex) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        i128::from_str_radix(hex, 16)
            .map_err(|_| unsupported_source_message("invalid source integer literal"))
    } else {
        value
            .parse::<i128>()
            .map_err(|_| unsupported_source_message("invalid source integer literal"))
    }
}
