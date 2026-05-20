use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Path, PathBuf};

use lzvm_artifacts::constraint_program::{GlobalConstraintEntry, GlobalConstraintProgram};
use lzvm_artifacts::expression_info::{
    ConstraintCode, ExpressionInfo, ExpressionInfoError, HintInfo,
};
use lzvm_artifacts::global_info::{
    encode_global_info, AggregationType, CurveKind, GlobalAir, GlobalInfo, GlobalInfoError,
    NamedStageValue, PublicValue,
};
use lzvm_artifacts::global_program::{encode_global_program, GlobalProgram, GlobalProgramError};
use lzvm_artifacts::hint_program::HintProgram;
use lzvm_artifacts::key_directory::{read_key_directory_layout, KeyDirectoryError, KeyUnitPaths};
use lzvm_artifacts::setup_info::{
    encode_unit_setup_info, Boundary, CommitmentColumn, ConstantColumn, EvaluationMapEntry,
    FriStep, SetupInfoError, StageValue, StarkStruct, UnitSetupInfo,
};
use lzvm_artifacts::verifier_info::{encode_verifier_info, VerifierInfoError};
use lzvm_pil::{
    evaluate_fixed_file_template_value_expression_with_values, lex_source, parse_expression,
    BinaryOperator, ColumnDeclaration, ColumnItem, ColumnKind, Expression, ExpressionKind,
    FixedFileTemplateValue, FunctionStatement, FunctionStatementKind, LexError, ParseError,
    SourceLoaderConfig, SourceProgram, SourceProgramError, SourceProgramLoader,
    SourceProgramModule, Token, TokenKind, UnaryOperator, ValueDeclarationKind,
};

use crate::{
    publish_staging_bytes,
    source_constraint_lowering::lower_source_template_boolean_constraint,
    source_expression_filters::{
        source_expression_assigns_fixed_index, source_expression_is_assignment,
        source_expression_is_equality_constraint,
    },
    source_row_count::{infer_source_row_counts, SourceUnitRowCounts},
    source_scalar_slots::SourceScalarSlots,
    source_scope::{
        concrete_template_names, declaration_in_function_body, declaration_in_inactive_template,
        global_constraint_source_names,
    },
    source_statement_hints::{
        lower_source_lookup_statement, lower_unsupported_source_assignment_statement,
        lower_unsupported_source_call_statement, lower_unsupported_source_constraint_statement,
        lower_unsupported_source_template_statement, source_statement_contains_assignment_operator,
        source_statement_first_token_kind,
    },
    source_static_values::{
        source_declaration_constant_values_from_cache, source_declaration_in_static_false_branch,
        source_scalar_constant_values, source_static_assignment_expression,
        source_static_if_statement_is_false, source_template_constant_value_cache,
        SourceTemplateConstantValueCache,
    },
    source_template_context::SourceTemplateLoweringContext,
    source_verifier_info::source_verifier_info,
    write_staging_bytes, SetupError,
};

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
            Self::UnsupportedSourceProgram { .. } => None,
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
        for unit in &layout.units {
            let Some(setup_path) = unit.setup_info_binary() else {
                continue;
            };
            let row_count = source_layout_unit_row_count(unit, &row_counts)?;
            let mut setup_info = source_unit_setup_info(&program, row_count)?;
            let expression_info = source_expression_info(&program, &setup_info)?;
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
    for module in &program.modules {
        if !module.public_tables.is_empty() {
            return unsupported("public tables need metadata lowering support");
        }
    }
    Ok(())
}

fn source_expression_info(
    program: &SourceProgram,
    setup: &UnitSetupInfo,
) -> Result<ExpressionInfo, SourceKeyDirectoryMetadataError> {
    let scalar_slots = SourceScalarSlots::from_setup(setup)
        .map_err(|error| unsupported_source_message(error.to_string()))?;
    let fixed_assignment_columns = source_fixed_assignment_column_names(program);
    let active_templates = concrete_template_names(program);
    let constant_values = source_scalar_constant_values(program, 1_u64 << setup.stark.n_bits);
    let mut hints = Vec::new();
    let mut constraints = Vec::new();
    for module in &program.modules {
        for template in &module.air_templates {
            if !active_templates.contains(&template.name) {
                continue;
            }
            let context = SourceTemplateLoweringContext {
                program,
                module,
                scalar_slots: &scalar_slots,
                fixed_columns: &fixed_assignment_columns,
                constant_values: &constant_values,
            };
            for statement in &template.statements {
                lower_source_template_statement(&context, statement, &mut hints, &mut constraints)?;
            }
        }
    }
    Ok(ExpressionInfo {
        hints,
        expressions: Vec::new(),
        constraints,
    })
}

fn source_fixed_assignment_column_names(program: &SourceProgram) -> BTreeSet<String> {
    let active_templates = concrete_template_names(program);
    program
        .modules
        .iter()
        .flat_map(|module| {
            module.columns.iter().filter(|declaration| {
                declaration.kind == ColumnKind::Fixed
                    && declaration.initializer.is_none()
                    && !declaration_in_function_body(module, declaration.start, declaration.end)
                    && !declaration_in_inactive_template(
                        module,
                        declaration.start,
                        declaration.end,
                        &active_templates,
                    )
            })
        })
        .flat_map(|declaration| declaration.items.iter().map(|item| item.name.clone()))
        .collect()
}

fn lower_source_template_statement(
    context: &SourceTemplateLoweringContext<'_>,
    statement: &FunctionStatement,
    hints: &mut Vec<HintInfo>,
    constraints: &mut Vec<ConstraintCode>,
) -> Result<(), SourceKeyDirectoryMetadataError> {
    if statement.kind == FunctionStatementKind::Declaration {
        return Ok(());
    }
    if source_static_if_statement_is_false(
        context.program,
        context.module,
        statement,
        context.constant_values,
    ) {
        return Ok(());
    }
    if statement.kind != FunctionStatementKind::Expression {
        hints.push(lower_unsupported_source_template_statement(
            context.module,
            statement,
        ));
        return Ok(());
    }

    if let Some(kind) =
        source_statement_first_token_kind(context.module, statement).map_err(|source| {
            SourceKeyDirectoryMetadataError::Lex {
                source_name: context.module.source_name.clone(),
                source,
            }
        })?
    {
        if matches!(kind, TokenKind::AirValue | TokenKind::Commit) {
            return Ok(());
        }
        if matches!(
            kind,
            TokenKind::Include | TokenKind::Require | TokenKind::Use
        ) {
            hints.push(lower_unsupported_source_template_statement(
                context.module,
                statement,
            ));
            return Ok(());
        }
    }
    if source_expression_assigns_fixed_index(
        statement.value_expression.as_ref(),
        context.fixed_columns,
    ) || source_static_assignment_expression(context.module, statement.value_expression.as_ref())
    {
        return Ok(());
    }
    if let Some(hint) =
        lower_source_lookup_statement(context.module, statement).map_err(|source| {
            SourceKeyDirectoryMetadataError::Lex {
                source_name: context.module.source_name.clone(),
                source,
            }
        })?
    {
        hints.push(hint);
        return Ok(());
    }
    let contains_assignment_operator =
        source_statement_contains_assignment_operator(context.module, statement).map_err(
            |source| SourceKeyDirectoryMetadataError::Lex {
                source_name: context.module.source_name.clone(),
                source,
            },
        )?;
    if source_expression_is_assignment(statement.value_expression.as_ref())
        || contains_assignment_operator
    {
        hints.push(lower_unsupported_source_assignment_statement(
            context.module,
            statement,
        ));
        return Ok(());
    }
    match lower_source_template_boolean_constraint(context.module, statement, context.scalar_slots)
    {
        Ok(Some(constraint)) => {
            constraints.push(constraint);
            return Ok(());
        }
        Ok(None)
            if source_expression_is_equality_constraint(statement.value_expression.as_ref()) =>
        {
            hints.push(lower_unsupported_source_constraint_statement(
                context.module,
                statement,
            ));
            return Ok(());
        }
        Ok(None) => {}
        Err(SourceKeyDirectoryMetadataError::UnsupportedSourceProgram { .. }) => {
            hints.push(lower_unsupported_source_constraint_statement(
                context.module,
                statement,
            ));
            return Ok(());
        }
        Err(error) => return Err(error),
    }
    if let Some(hint) =
        lower_unsupported_source_call_statement(context.module, statement).map_err(|source| {
            SourceKeyDirectoryMetadataError::Lex {
                source_name: context.module.source_name.clone(),
                source,
            }
        })?
    {
        hints.push(hint);
        return Ok(());
    }
    unsupported(format!(
        "air template statements need constraint lowering support: {}",
        source_statement_line(context.module, statement)
    ))
}

fn source_statement_line<'a>(
    module: &'a SourceProgramModule,
    statement: &FunctionStatement,
) -> &'a str {
    module.source.contents[statement.start..statement.end]
        .trim()
        .trim_end_matches(';')
        .trim()
}

fn source_global_program(
    program: &SourceProgram,
    global_info: &GlobalInfo,
) -> Result<GlobalProgram, SourceKeyDirectoryMetadataError> {
    let proof_value_slots = source_proof_value_slots(global_info)?;
    let global_source_names = global_constraint_source_names(program);
    let mut constraints = SourceGlobalConstraintBuilder::default();
    for module in &program.modules {
        if !global_source_names.contains(&module.source_name) {
            continue;
        }
        lower_module_top_level_global_constraints(module, &proof_value_slots, &mut constraints)?;
    }
    Ok(GlobalProgram {
        constraints: constraints.finish(),
        hints: HintProgram { hints: Vec::new() },
    })
}

fn lower_module_top_level_global_constraints(
    module: &SourceProgramModule,
    proof_value_slots: &BTreeMap<String, SourceProofValueSlot>,
    constraints: &mut SourceGlobalConstraintBuilder,
) -> Result<(), SourceKeyDirectoryMetadataError> {
    let tokens = lex_source(&module.source.contents).map_err(|source| {
        SourceKeyDirectoryMetadataError::Lex {
            source_name: module.source_name.clone(),
            source,
        }
    })?;
    let mut index = 0;
    while index < tokens.len() {
        let token = &tokens[index];
        match token.kind {
            TokenKind::Pragma => {
                index += 1;
            }
            kind if top_level_declaration_start(kind) => {
                index = skip_top_level_item(&tokens, index)?;
            }
            TokenKind::Identifier => {
                if let Some(next_index) = skip_known_top_level_metadata_directive(&tokens, index) {
                    index = next_index;
                } else {
                    index = lower_top_level_expression_statement(
                        module,
                        &tokens,
                        index,
                        proof_value_slots,
                        constraints,
                    )?;
                }
            }
            TokenKind::Public | TokenKind::Private
                if tokens.get(index + 1).is_some_and(|next| {
                    matches!(
                        next.kind,
                        TokenKind::Include | TokenKind::Require | TokenKind::Function
                    )
                }) =>
            {
                index = skip_top_level_item(&tokens, index)?;
            }
            _ => {
                index = lower_top_level_expression_statement(
                    module,
                    &tokens,
                    index,
                    proof_value_slots,
                    constraints,
                )?;
            }
        }
    }
    Ok(())
}

fn lower_top_level_expression_statement(
    module: &SourceProgramModule,
    tokens: &[Token],
    index: usize,
    proof_value_slots: &BTreeMap<String, SourceProofValueSlot>,
    constraints: &mut SourceGlobalConstraintBuilder,
) -> Result<usize, SourceKeyDirectoryMetadataError> {
    let next_index = skip_top_level_statement(tokens, index)?;
    let expression_end = next_index
        .checked_sub(1)
        .ok_or_else(|| unsupported_source_message("top-level statement has no expression"))?;
    let (expression, consumed) = parse_expression(&module.source, index, expression_end)?;
    if consumed != expression_end {
        return unsupported("top-level statement has unsupported trailing tokens");
    }
    lower_top_level_global_constraint(
        &expression,
        &module.source.contents[expression.start..expression.end],
        proof_value_slots,
        constraints,
    )?;
    Ok(next_index)
}

fn lower_top_level_global_constraint(
    expression: &Expression,
    source_line: &str,
    proof_value_slots: &BTreeMap<String, SourceProofValueSlot>,
    constraints: &mut SourceGlobalConstraintBuilder,
) -> Result<(), SourceKeyDirectoryMetadataError> {
    let Some(name) = proof_value_boolean_constraint_name(expression) else {
        return unsupported(format!(
            "top-level statements need global constraint lowering support: {}",
            source_line.trim()
        ));
    };
    let slot = proof_value_slots.get(name).copied().ok_or_else(|| {
        unsupported_source_message("top-level proof value constraint references an unknown value")
    })?;
    if slot.stage != 1 {
        return unsupported("top-level proof value constraints require stage-one values");
    }
    constraints.append_proof_value_boolean_constraint(slot.offset, source_line.trim().to_owned())
}

#[derive(Debug, Clone, Copy)]
struct SourceProofValueSlot {
    offset: u32,
    stage: u64,
}

fn source_proof_value_slots(
    global_info: &GlobalInfo,
) -> Result<BTreeMap<String, SourceProofValueSlot>, SourceKeyDirectoryMetadataError> {
    let mut slots = BTreeMap::new();
    let mut next_offset = 0_u32;
    for entry in &global_info.proof_values_map {
        slots.insert(
            entry.name.clone(),
            SourceProofValueSlot {
                offset: next_offset,
                stage: entry.stage,
            },
        );
        let width = if entry.stage == 1 { 1 } else { 3 };
        next_offset = next_offset
            .checked_add(width)
            .ok_or_else(|| unsupported_source_message("source proof value offset overflow"))?;
    }
    Ok(slots)
}

fn proof_value_boolean_constraint_name(expression: &Expression) -> Option<&str> {
    let ExpressionKind::Binary { op, left, right } = &strip_group_expression(expression).kind
    else {
        return None;
    };
    if *op != BinaryOperator::Multiply {
        return None;
    }

    if let Some(name) = expression_name(left) {
        if one_minus_name(right, name) || name_minus_one(right, name) {
            return Some(name);
        }
    }
    if let Some(name) = expression_name(right) {
        if one_minus_name(left, name) || name_minus_one(left, name) {
            return Some(name);
        }
    }
    None
}

fn strip_group_expression(expression: &Expression) -> &Expression {
    match &expression.kind {
        ExpressionKind::Group(inner) => strip_group_expression(inner),
        _ => expression,
    }
}

fn expression_name(expression: &Expression) -> Option<&str> {
    match &strip_group_expression(expression).kind {
        ExpressionKind::Name(name) => Some(name),
        _ => None,
    }
}

fn one_minus_name(expression: &Expression, name: &str) -> bool {
    let ExpressionKind::Binary { op, left, right } = &strip_group_expression(expression).kind
    else {
        return false;
    };
    *op == BinaryOperator::Subtract
        && expression_is_one(left)
        && expression_name(right) == Some(name)
}

fn name_minus_one(expression: &Expression, name: &str) -> bool {
    let ExpressionKind::Binary { op, left, right } = &strip_group_expression(expression).kind
    else {
        return false;
    };
    *op == BinaryOperator::Subtract
        && expression_name(left) == Some(name)
        && expression_is_one(right)
}

fn expression_is_one(expression: &Expression) -> bool {
    match &strip_group_expression(expression).kind {
        ExpressionKind::Integer(value) | ExpressionKind::HexInteger(value) => {
            parse_i128_literal(value).is_ok_and(|value| value == 1)
        }
        _ => false,
    }
}

fn skip_top_level_statement(
    tokens: &[Token],
    index: usize,
) -> Result<usize, SourceKeyDirectoryMetadataError> {
    let mut stack = Vec::<TokenKind>::new();
    let mut cursor = index;
    while let Some(token) = tokens.get(cursor) {
        if stack.is_empty() && token.kind == TokenKind::Semicolon {
            return Ok(cursor + 1);
        }

        match token.kind {
            TokenKind::LParen => stack.push(TokenKind::RParen),
            TokenKind::LBracket => stack.push(TokenKind::RBracket),
            TokenKind::LBrace => stack.push(TokenKind::RBrace),
            TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                let Some(expected) = stack.pop() else {
                    return unsupported("top-level statement has an unmatched closing delimiter");
                };
                if token.kind != expected {
                    return unsupported("top-level statement delimiters are not balanced");
                }
            }
            _ => {}
        }
        cursor += 1;
    }
    unsupported("top-level statement has no terminator")
}

#[derive(Default)]
struct SourceGlobalConstraintBuilder {
    entries: Vec<GlobalConstraintEntry>,
    ops: Vec<u8>,
    args: Vec<u16>,
    numbers: Vec<u64>,
}

impl SourceGlobalConstraintBuilder {
    fn append_proof_value_boolean_constraint(
        &mut self,
        proof_value_offset: u32,
        source_line: String,
    ) -> Result<(), SourceKeyDirectoryMetadataError> {
        let ops_offset = source_usize_to_u32(self.ops.len(), "source global op offset overflow")?;
        let args_offset =
            source_usize_to_u32(self.args.len(), "source global argument offset overflow")?;
        let one_offset = self.intern_number(1)?;
        let proof_value_offset =
            source_u32_to_u16(proof_value_offset, "source proof value offset overflow")?;
        let one_offset = source_u32_to_u16(one_offset, "source number offset overflow")?;

        self.ops.extend([0, 0]);
        self.args.extend([
            1,
            0,
            2,
            one_offset,
            3,
            proof_value_offset,
            2,
            1,
            3,
            proof_value_offset,
            0,
            0,
        ]);
        self.entries.push(GlobalConstraintEntry {
            destination_dimension: 1,
            destination_id: 1,
            temp1_count: 2,
            temp3_count: 0,
            ops_count: 2,
            ops_offset,
            args_count: 12,
            args_offset,
            source_line,
        });
        Ok(())
    }

    fn intern_number(&mut self, value: u64) -> Result<u32, SourceKeyDirectoryMetadataError> {
        if let Some(index) = self.numbers.iter().position(|existing| *existing == value) {
            return source_usize_to_u32(index, "source number offset overflow");
        }
        let index = source_usize_to_u32(self.numbers.len(), "source number offset overflow")?;
        self.numbers.push(value);
        Ok(index)
    }

    fn finish(self) -> GlobalConstraintProgram {
        GlobalConstraintProgram {
            entries: self.entries,
            ops: self.ops,
            args: self.args,
            numbers: self.numbers,
        }
    }
}

fn source_usize_to_u32(
    value: usize,
    message: &'static str,
) -> Result<u32, SourceKeyDirectoryMetadataError> {
    u32::try_from(value).map_err(|_| unsupported_source_message(message))
}

fn source_u32_to_u16(
    value: u32,
    message: &'static str,
) -> Result<u16, SourceKeyDirectoryMetadataError> {
    u16::try_from(value).map_err(|_| unsupported_source_message(message))
}

fn skip_known_top_level_metadata_directive(tokens: &[Token], index: usize) -> Option<usize> {
    let name = tokens.get(index)?;
    let open = tokens.get(index + 1)?;
    let close = tokens.get(index + 2)?;
    let semicolon = tokens.get(index + 3)?;
    if name.lexeme == "enable_range_stats"
        && open.kind == TokenKind::LParen
        && close.kind == TokenKind::RParen
        && semicolon.kind == TokenKind::Semicolon
    {
        Some(index + 4)
    } else {
        None
    }
}

fn top_level_declaration_start(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::AirGroup
            | TokenKind::AirGroupValue
            | TokenKind::AirTemplate
            | TokenKind::AirValue
            | TokenKind::Challenge
            | TokenKind::Col
            | TokenKind::Commit
            | TokenKind::Const
            | TokenKind::Constant
            | TokenKind::Container
            | TokenKind::Declare
            | TokenKind::Expr
            | TokenKind::Fe
            | TokenKind::For
            | TokenKind::Function
            | TokenKind::Include
            | TokenKind::Int
            | TokenKind::Package
            | TokenKind::ProofValue
            | TokenKind::Public
            | TokenKind::PublicTable
            | TokenKind::Require
            | TokenKind::String
            | TokenKind::Switch
            | TokenKind::Use
    )
}

fn skip_top_level_item(
    tokens: &[Token],
    index: usize,
) -> Result<usize, SourceKeyDirectoryMetadataError> {
    let mut stack = Vec::<TokenKind>::new();
    let mut cursor = index;
    while let Some(token) = tokens.get(cursor) {
        if stack.is_empty() && token.kind == TokenKind::Semicolon {
            return Ok(cursor + 1);
        }
        if stack.is_empty() && token.kind == TokenKind::LBrace {
            return skip_balanced_delimiter(tokens, cursor, TokenKind::RBrace);
        }

        match token.kind {
            TokenKind::LParen => stack.push(TokenKind::RParen),
            TokenKind::LBracket => stack.push(TokenKind::RBracket),
            TokenKind::LBrace => stack.push(TokenKind::RBrace),
            TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                let Some(expected) = stack.pop() else {
                    return unsupported("source declaration has an unmatched closing delimiter");
                };
                if token.kind != expected {
                    return unsupported("source declaration delimiters are not balanced");
                }
            }
            _ => {}
        }
        cursor += 1;
    }
    unsupported("source declaration has no terminator")
}

fn skip_balanced_delimiter(
    tokens: &[Token],
    index: usize,
    close_kind: TokenKind,
) -> Result<usize, SourceKeyDirectoryMetadataError> {
    let open_kind = tokens
        .get(index)
        .map(|token| token.kind)
        .ok_or_else(|| unsupported_source_message("source declaration has no body"))?;
    let mut depth = 0_usize;
    let mut cursor = index;
    while let Some(token) = tokens.get(cursor) {
        if token.kind == open_kind {
            depth = depth
                .checked_add(1)
                .ok_or_else(|| unsupported_source_message("source declaration nesting overflow"))?;
        } else if token.kind == close_kind {
            depth = depth
                .checked_sub(1)
                .ok_or_else(|| unsupported_source_message("source declaration body underflow"))?;
            if depth == 0 {
                return Ok(cursor + 1);
            }
        }
        cursor += 1;
    }
    unsupported("source declaration body is not closed")
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
    let num_challenges = source_challenge_counts(program, &constant_values)?;
    let (num_proof_values, proof_values_map) = source_proof_values(program, &constant_values)?;
    let publics_map = source_public_values(program, &constant_values)?;
    let (_, group_aggregation_types) = source_air_group_values(program, &constant_values)?;
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
        lattice_size: None,
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
) -> Result<Vec<PublicValue>, SourceKeyDirectoryMetadataError> {
    let mut seen = BTreeSet::new();
    let mut values = Vec::new();
    for module in &program.modules {
        for declaration in &module.publics {
            if declaration.initializer.is_some() {
                return unsupported("source public initializers need metadata lowering support");
            }
            for item in &declaration.items {
                if item.template {
                    return unsupported(
                        "template public-value names need instance lowering support",
                    );
                }
                if !seen.insert(item.name.clone()) {
                    return unsupported("duplicate source public value name");
                }
                values.push(PublicValue {
                    name: item.name.clone(),
                    stage: 1,
                    lengths: source_item_lengths(item, "source public value", constant_values)?
                        .into_iter()
                        .map(u64::from)
                        .collect(),
                });
            }
        }
    }
    Ok(values)
}

fn source_proof_values(
    program: &SourceProgram,
    constant_values: &BTreeMap<String, FixedFileTemplateValue>,
) -> Result<(Vec<u64>, Vec<NamedStageValue>), SourceKeyDirectoryMetadataError> {
    let mut seen = BTreeSet::new();
    let mut counts_by_stage = Vec::<u64>::new();
    let mut values = Vec::new();
    for module in &program.modules {
        for declaration in &module.values {
            if declaration.kind != ValueDeclarationKind::ProofValue {
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
                    lengths: source_item_lengths(item, "source proof value", constant_values)?
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
) -> Result<Vec<u64>, SourceKeyDirectoryMetadataError> {
    let mut seen = BTreeMap::<String, SourceChallengeShape>::new();
    let mut counts_by_stage = Vec::<u64>::new();
    for module in &program.modules {
        for declaration in &module.values {
            if declaration.kind != ValueDeclarationKind::Challenge {
                continue;
            }
            let stage = usize::try_from(declaration.stage)
                .map_err(|_| unsupported_source_message("source challenge stage overflow"))?;
            if stage == 0 {
                return unsupported("source challenge stage must be positive");
            }
            if counts_by_stage.len() < stage {
                counts_by_stage.resize(stage, 0);
            }
            for item in &declaration.items {
                if item.template {
                    return unsupported("template challenge names need instance lowering support");
                }
                let lengths = source_item_lengths(item, "source challenge", constant_values)?;
                let dimension = source_column_dimension(&lengths, "source challenge")?;
                let shape = SourceChallengeShape { stage, dimension };
                if let Some(existing) = seen.get(&item.name) {
                    if *existing != shape {
                        return unsupported("duplicate source challenge name");
                    }
                    continue;
                }
                seen.insert(item.name.clone(), shape);
                counts_by_stage[stage - 1] = counts_by_stage[stage - 1]
                    .checked_add(u64::from(dimension))
                    .ok_or_else(|| unsupported_source_message("source challenge count overflow"))?;
            }
        }
    }
    Ok(counts_by_stage)
}

fn source_unit_values(
    program: &SourceProgram,
    constant_values: &BTreeMap<String, FixedFileTemplateValue>,
    template_values: &SourceTemplateConstantValueCache,
) -> Result<Vec<StageValue>, SourceKeyDirectoryMetadataError> {
    let mut seen = BTreeMap::<String, (u32, Vec<u32>)>::new();
    let mut values = Vec::new();
    for module in &program.modules {
        for declaration in &module.values {
            if declaration.kind != ValueDeclarationKind::AirValue
                || declaration_in_function_body(module, declaration.start, declaration.end)
            {
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
            if declaration.stage == 0 {
                return unsupported("source air value stage must be positive");
            }
            for item in &declaration.items {
                if item.template {
                    return unsupported("template air-value names need instance lowering support");
                }
                let lengths = source_item_lengths(item, "source air value", declaration_values)?;
                let shape = (declaration.stage, lengths.clone());
                if let Some(existing) = seen.get(&item.name) {
                    if *existing != shape {
                        return unsupported("duplicate source air value name");
                    }
                    continue;
                }
                seen.insert(item.name.clone(), shape);
                values.push(StageValue {
                    name: item.name.clone(),
                    stage: declaration.stage,
                    lengths,
                });
            }
        }
    }
    Ok(values)
}

fn source_air_group_values(
    program: &SourceProgram,
    constant_values: &BTreeMap<String, FixedFileTemplateValue>,
) -> Result<(Vec<StageValue>, Vec<AggregationType>), SourceKeyDirectoryMetadataError> {
    let mut seen = BTreeSet::new();
    let mut values = Vec::new();
    let mut aggregation_types = Vec::new();
    for module in &program.modules {
        for declaration in &module.air_group_values {
            if declaration_in_function_body(module, declaration.start, declaration.end) {
                continue;
            }
            if declaration.stage == 0 {
                return unsupported("source air group value stage must be positive");
            }
            let aggregation_type =
                source_group_value_aggregation_type(&declaration.aggregate_type)?;
            if let Some(default_expression) = declaration.default_expression.as_ref() {
                let Some(FixedFileTemplateValue::Integer(default_value)) =
                    evaluate_fixed_file_template_value_expression_with_values(
                        default_expression,
                        constant_values,
                    )
                else {
                    return unsupported(
                        "source air group value defaults need proof lowering support",
                    );
                };
                let identity = match aggregation_type {
                    0 => 0,
                    1 => 1,
                    _ => return unsupported("unsupported source air group value aggregation type"),
                };
                if default_value != identity {
                    return unsupported(
                        "source air group value defaults need proof lowering support",
                    );
                }
            } else if declaration.default_value.is_some() {
                return unsupported("source air group value defaults need proof lowering support");
            }
            for item in &declaration.items {
                if item.template {
                    return unsupported(
                        "template air-group-value names need instance lowering support",
                    );
                }
                if !seen.insert(item.name.clone()) {
                    return unsupported("duplicate source air group value name");
                }
                values.push(StageValue {
                    name: item.name.clone(),
                    stage: declaration.stage,
                    lengths: source_item_lengths(item, "source air group value", constant_values)?,
                });
                aggregation_types.push(AggregationType { aggregation_type });
            }
        }
    }
    Ok((values, aggregation_types))
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
    let unit_value_map = source_unit_values(program, &constant_values, &template_values)?;
    let (group_value_map, _) = source_air_group_values(program, &constant_values)?;
    let challenge_count = source_challenge_counts(program, &constant_values)?
        .into_iter()
        .try_fold(0_usize, |acc, count| {
            usize::try_from(count)
                .ok()
                .and_then(|count| acc.checked_add(count))
        })
        .ok_or_else(|| unsupported_source_message("source challenge count overflow"))?;
    let public_count = source_public_values(program, &constant_values)?.len();
    let const_width = constant_columns
        .iter()
        .try_fold(0_u32, |acc, column| acc.checked_add(column.dimension))
        .ok_or_else(|| unsupported_source_message("source constant width overflow"))?;
    let mut section_widths = BTreeMap::from([("const".to_owned(), const_width)]);
    section_widths.extend(commitment_widths);

    Ok(UnitSetupInfo {
        n_stages,
        n_constants: u32::try_from(constant_columns.len())
            .map_err(|_| unsupported_source_message("too many source fixed columns"))?,
        constant_columns,
        n_publics: Some(
            u32::try_from(public_count)
                .map_err(|_| unsupported_source_message("too many source public values"))?,
        ),
        n_constraints: Some(0),
        q_degree: 3,
        opening_points: vec![0],
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
                let lengths = source_item_lengths(item, "source fixed-column", declaration_values)?;
                let dimension = source_column_dimension(&lengths, "source fixed-column")?;
                let id = u32::try_from(columns.len())
                    .map_err(|_| unsupported_source_message("too many source fixed columns"))?;
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
            let stage = source_column_stage(declaration, declaration_values)?;
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
                let lengths =
                    source_item_lengths(item, "source commitment-column", declaration_values)?;
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
    item: &ColumnItem,
    item_role: &str,
    constant_values: &BTreeMap<String, FixedFileTemplateValue>,
) -> Result<Vec<u32>, SourceKeyDirectoryMetadataError> {
    let mut lengths = Vec::with_capacity(item.array_dim_expressions.len());
    for expression in &item.array_dim_expressions {
        let Some(expression) = expression else {
            return unsupported(format!("{item_role} array dimensions must be static"));
        };
        let value = eval_u32_expression_with_values(expression, constant_values)?;
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
    expression: &Expression,
    values: &BTreeMap<String, FixedFileTemplateValue>,
) -> Result<u32, SourceKeyDirectoryMetadataError> {
    if let Some(FixedFileTemplateValue::Integer(value)) =
        evaluate_fixed_file_template_value_expression_with_values(expression, values)
    {
        return u32::try_from(value)
            .map_err(|_| unsupported_source_message("source expression is out of range"));
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
