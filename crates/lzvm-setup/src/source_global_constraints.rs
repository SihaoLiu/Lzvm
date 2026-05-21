use std::collections::{BTreeMap, BTreeSet};

use lzvm_artifacts::constraint_program::{GlobalConstraintEntry, GlobalConstraintProgram};
use lzvm_artifacts::global_info::GlobalInfo;
use lzvm_artifacts::global_program::GlobalProgram;
use lzvm_artifacts::setup_info::StageValue;
use lzvm_field::{Felt, MODULUS};
use lzvm_pil::{
    lex_source, parse_expression, BinaryOperator, ConstantDeclaration, Expression, ExpressionKind,
    FixedFileTemplateValue, PublicDeclaration, SourceProgram, SourceProgramModule, Token,
    TokenKind, UnaryOperator,
};

use crate::{
    source_control_body_cache::SourceControlBodyCaches,
    source_global_values::source_challenge_slots,
    source_key_directory::{
        source_air_group_values, source_item_lengths, source_item_name,
        SourceKeyDirectoryMetadataError,
    },
    source_scalar_slots::SourceChallengeSlotMetadata,
    source_scope::{concrete_template_names, global_constraint_source_names},
    source_static_values::{
        evaluate_source_static_expression, source_declaration_in_static_false_branch,
        source_scalar_constant_values, source_static_array_expression,
        source_template_constant_value_cache, static_value_integer,
    },
};

mod hints;
mod residuals;
mod top_level_call;
mod top_level_for;
mod top_level_if;

pub(crate) fn source_global_program(
    program: &SourceProgram,
    global_info: &GlobalInfo,
) -> Result<GlobalProgram, SourceKeyDirectoryMetadataError> {
    let proof_value_slots = source_proof_value_slots(global_info)?;
    let public_value_slots = source_public_value_slots(global_info)?;
    let global_source_names = global_constraint_source_names(program);
    let static_values = global_info
        .lattice_size
        .map(|row_count| source_scalar_constant_values(program, row_count))
        .unwrap_or_default();
    let template_values = source_template_constant_value_cache(program, &static_values);
    let active_templates = concrete_template_names(program);
    let mut body_caches = SourceControlBodyCaches::default();
    let challenge_metadata = source_challenge_slots(
        program,
        &static_values,
        &active_templates,
        &template_values,
        &mut body_caches,
    )?;
    let challenge_slots = source_global_challenge_slots(&challenge_metadata);
    let (group_value_metadata, _) = source_air_group_values(
        program,
        None,
        &static_values,
        &active_templates,
        &template_values,
        &mut body_caches,
    )?;
    let group_value_slots = source_global_group_value_slots(&group_value_metadata)?;
    let slots = SourceGlobalSlots {
        proof_values: &proof_value_slots,
        public_values: &public_value_slots,
        challenges: &challenge_slots,
        group_values: &group_value_slots,
    };
    let hints = hints::source_global_hints(program, global_info, &static_values, &mut body_caches)?;
    let mut constraints = SourceGlobalConstraintBuilder::default();
    for module in &program.modules {
        if !global_source_names.contains(&module.source_name) {
            continue;
        }
        lower_module_public_initializer_constraints(
            program,
            module,
            &public_value_slots,
            &static_values,
            &mut constraints,
        )?;
        lower_module_top_level_global_constraints(
            program,
            module,
            &slots,
            &static_values,
            &mut constraints,
        )?;
    }
    Ok(GlobalProgram {
        constraints: constraints.finish(),
        hints,
    })
}

fn lower_module_public_initializer_constraints(
    program: &SourceProgram,
    module: &SourceProgramModule,
    public_value_slots: &BTreeMap<String, SourcePublicValueSlot>,
    static_values: &BTreeMap<String, FixedFileTemplateValue>,
    constraints: &mut SourceGlobalConstraintBuilder,
) -> Result<(), SourceKeyDirectoryMetadataError> {
    for declaration in &module.publics {
        if declaration.initializer.is_none() {
            continue;
        }
        if source_global_declaration_in_nested_body(module, declaration.start, declaration.end)
            || source_declaration_in_static_false_branch(
                program,
                module,
                declaration.start,
                declaration.end,
                static_values,
            )
        {
            continue;
        }
        lower_public_initializer_constraint(
            program,
            module,
            declaration,
            public_value_slots,
            static_values,
            constraints,
        )?;
    }
    Ok(())
}

fn lower_public_initializer_constraint(
    program: &SourceProgram,
    module: &SourceProgramModule,
    declaration: &PublicDeclaration,
    public_value_slots: &BTreeMap<String, SourcePublicValueSlot>,
    static_values: &BTreeMap<String, FixedFileTemplateValue>,
    constraints: &mut SourceGlobalConstraintBuilder,
) -> Result<(), SourceKeyDirectoryMetadataError> {
    let (name, dimension) = source_public_initializer_target(program, declaration, static_values)?;
    let Some(slot) = public_value_slots.get(&name).copied() else {
        return unsupported("source public initializer references an unknown public value");
    };
    let slot_dimension = usize::try_from(slot.dimension)
        .map_err(|_| unsupported_source_message("source public initializer dimension overflow"))?;
    if slot.stage != 1 || slot_dimension != dimension {
        return unsupported("source public initializer dimension does not match metadata");
    }
    let Some(expression) = declaration.initializer_expression.as_ref() else {
        return unsupported("source public initializers must be static field values");
    };
    let values =
        source_public_initializer_field_values(program, expression, static_values, dimension)?;
    let source_line = module.source.contents[declaration.start..declaration.end]
        .trim()
        .to_owned();
    for (index, value) in values.into_iter().enumerate() {
        let offset = source_usize_to_u32(index, "source public initializer offset overflow")
            .and_then(|index| {
                slot.offset.checked_add(index).ok_or_else(|| {
                    unsupported_source_message("source public initializer offset overflow")
                })
            })?;
        constraints.append_public_value_constant_constraint(offset, value, source_line.clone())?;
    }
    Ok(())
}

fn source_public_initializer_target(
    program: &SourceProgram,
    declaration: &PublicDeclaration,
    static_values: &BTreeMap<String, FixedFileTemplateValue>,
) -> Result<(String, usize), SourceKeyDirectoryMetadataError> {
    if declaration.items.len() != 1 {
        return unsupported("source public initializers require one public value");
    }
    let item = declaration
        .items
        .first()
        .ok_or_else(|| unsupported_source_message("source public initializer has no value"))?;
    let name = source_item_name(program, item, "source public value", static_values)?;
    let lengths = source_item_lengths(program, item, "source public value", static_values)?;
    let dimension = if lengths.is_empty() {
        1
    } else {
        lengths.iter().try_fold(1_usize, |dimension, length| {
            dimension
                .checked_mul(usize::try_from(*length).map_err(|_| {
                    unsupported_source_message("source public initializer dimension overflow")
                })?)
                .ok_or_else(|| {
                    unsupported_source_message("source public initializer dimension overflow")
                })
        })?
    };
    Ok((name, dimension))
}

fn source_public_initializer_field_values(
    program: &SourceProgram,
    expression: &Expression,
    static_values: &BTreeMap<String, FixedFileTemplateValue>,
    dimension: usize,
) -> Result<Vec<u64>, SourceKeyDirectoryMetadataError> {
    let values = if dimension == 1 {
        vec![
            evaluate_source_static_expression(program, expression, static_values).ok_or_else(
                || {
                    unsupported_source_message(
                        "source public initializers must be static field values",
                    )
                },
            )?,
        ]
    } else {
        source_static_array_expression(program, expression, static_values).ok_or_else(|| {
            unsupported_source_message("source public initializers must be static field values")
        })?
    };
    if values.len() != dimension {
        return unsupported(
            "source public initializer length does not match public value dimension",
        );
    }
    values
        .iter()
        .map(source_public_initializer_field_value)
        .collect()
}

fn source_public_initializer_field_value(
    value: &FixedFileTemplateValue,
) -> Result<u64, SourceKeyDirectoryMetadataError> {
    let Some(value) = static_value_integer(value) else {
        return unsupported("source public initializers must be static field values");
    };
    let modulus = i128::from(MODULUS);
    let canonical = value.rem_euclid(modulus);
    u64::try_from(canonical)
        .map_err(|_| unsupported_source_message("source public initializer value overflow"))
}

fn lower_module_top_level_global_constraints(
    program: &SourceProgram,
    module: &SourceProgramModule,
    slots: &SourceGlobalSlots<'_>,
    static_values: &BTreeMap<String, FixedFileTemplateValue>,
    constraints: &mut SourceGlobalConstraintBuilder,
) -> Result<(), SourceKeyDirectoryMetadataError> {
    let alias_scope = SourceGlobalAliasScope {
        program,
        expressions: top_level_global_expression_aliases(module),
        expression_arrays: top_level_global_expression_array_aliases(module),
        static_values: static_values.clone(),
    };
    let tokens = lex_source(&module.source.contents).map_err(|source| {
        SourceKeyDirectoryMetadataError::Lex {
            source_name: module.source_name.clone(),
            source,
        }
    })?;
    let context = SourceTopLevelGlobalConstraintContext {
        program,
        module,
        tokens: &tokens,
        slots,
        alias_scope: &alias_scope,
    };
    lower_top_level_global_constraints_range(&context, 0, tokens.len(), constraints)
}

pub(super) struct SourceTopLevelGlobalConstraintContext<'a, 'b, 'c> {
    program: &'a SourceProgram,
    module: &'a SourceProgramModule,
    tokens: &'a [Token],
    slots: &'a SourceGlobalSlots<'b>,
    alias_scope: &'a SourceGlobalAliasScope<'c>,
}

fn lower_top_level_global_constraints_range(
    context: &SourceTopLevelGlobalConstraintContext<'_, '_, '_>,
    start: usize,
    end: usize,
    constraints: &mut SourceGlobalConstraintBuilder,
) -> Result<(), SourceKeyDirectoryMetadataError> {
    let mut index = start;
    while index < end {
        let token = &context.tokens[index];
        match token.kind {
            TokenKind::Pragma => {
                index += 1;
            }
            TokenKind::For => {
                if let Some(next_index) = top_level_for::lower_top_level_static_for_statement(
                    context.program,
                    context.module,
                    context.tokens,
                    index,
                    context.slots,
                    context.alias_scope,
                    constraints,
                )? {
                    index = next_index;
                } else {
                    index = skip_top_level_item(context.tokens, index)?;
                }
            }
            TokenKind::If => {
                index =
                    top_level_if::lower_top_level_static_if_statement(context, index, constraints)?;
            }
            kind if top_level_declaration_start(kind) => {
                index = skip_top_level_item(context.tokens, index)?;
            }
            TokenKind::Identifier => {
                if let Some(next_index) =
                    skip_known_top_level_metadata_directive(context.tokens, index)
                {
                    index = next_index;
                } else {
                    index = lower_top_level_expression_statement(context, index, constraints)?;
                }
            }
            TokenKind::Public | TokenKind::Private
                if context.tokens.get(index + 1).is_some_and(|next| {
                    matches!(
                        next.kind,
                        TokenKind::Include | TokenKind::Require | TokenKind::Function
                    )
                }) =>
            {
                index = skip_top_level_item(context.tokens, index)?;
            }
            _ => {
                index = lower_top_level_expression_statement(context, index, constraints)?;
            }
        }
    }
    Ok(())
}

fn lower_top_level_expression_statement(
    context: &SourceTopLevelGlobalConstraintContext<'_, '_, '_>,
    index: usize,
    constraints: &mut SourceGlobalConstraintBuilder,
) -> Result<usize, SourceKeyDirectoryMetadataError> {
    let next_index = skip_top_level_statement(context.tokens, index)?;
    let expression_end = next_index
        .checked_sub(1)
        .ok_or_else(|| unsupported_source_message("top-level statement has no expression"))?;
    let (expression, consumed) = parse_expression(&context.module.source, index, expression_end)?;
    if consumed != expression_end {
        return unsupported("top-level statement has unsupported trailing tokens");
    }
    if top_level_call::lower_top_level_function_call(context, &expression, constraints)? {
        return Ok(next_index);
    }
    lower_top_level_global_constraint(
        &expression,
        &context.module.source.contents[expression.start..expression.end],
        context.slots,
        context.alias_scope,
        constraints,
    )?;
    Ok(next_index)
}

fn lower_top_level_global_constraint(
    expression: &Expression,
    source_line: &str,
    slots: &SourceGlobalSlots<'_>,
    alias_scope: &SourceGlobalAliasScope<'_>,
    constraints: &mut SourceGlobalConstraintBuilder,
) -> Result<(), SourceKeyDirectoryMetadataError> {
    let mut resolving_aliases = BTreeSet::new();
    let mut resolving_array_aliases = BTreeSet::new();
    if let Some(target) = proof_value_boolean_constraint_target(
        expression,
        alias_scope,
        &mut resolving_aliases,
        &mut resolving_array_aliases,
    ) {
        if let Some(slot) = slots.proof_values.get(&target.name).copied() {
            if target.index.is_some() {
                return unsupported("top-level proof value constraints require scalar values");
            }
            return constraints.append_proof_value_boolean_constraint(
                slot.offset,
                proof_value_operand_dimension(slot.stage),
                source_line.trim().to_owned(),
            );
        }
        if let Some(slot) = slots.public_values.get(&target.name).copied() {
            if slot.stage != 1 {
                return unsupported("top-level public value constraints require scalar values");
            }
            let offset = public_value_target_offset(slot, target.index)?;
            return constraints
                .append_public_value_boolean_constraint(offset, source_line.trim().to_owned());
        }
    }
    let source_line = source_line.trim().to_owned();
    if constraints.append_base_residual_constraint(
        expression,
        slots.proof_values,
        slots.public_values,
        alias_scope,
        source_line.clone(),
    )? {
        return Ok(());
    }
    if constraints.append_ext_residual_constraint(
        expression,
        slots,
        alias_scope,
        source_line.clone(),
    )? {
        return Ok(());
    }
    unsupported(format!(
        "top-level statements need global constraint lowering support: {}",
        source_line.trim()
    ))
}

#[derive(Debug, Clone, Copy)]
struct SourceProofValueSlot {
    offset: u32,
    stage: u64,
    dimension: u32,
}

#[derive(Debug, Clone, Copy)]
struct SourcePublicValueSlot {
    offset: u32,
    stage: u64,
    dimension: u32,
}

#[derive(Debug, Clone, Copy)]
struct SourceChallengeSlot {
    id: u32,
    dimension: u32,
}

#[derive(Debug, Clone, Copy)]
struct SourceGroupValueSlot {
    offset: u32,
    stage: u32,
    dimension: u32,
}

#[derive(Debug, Clone, Copy)]
struct SourceGlobalBaseOperand {
    buffer: u16,
    offset: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceBooleanTarget {
    name: String,
    index: Option<u32>,
}

struct SourceGlobalAliasScope<'a> {
    program: &'a SourceProgram,
    expressions: SourceGlobalExpressionAliases,
    expression_arrays: SourceGlobalExpressionArrayAliases,
    static_values: BTreeMap<String, FixedFileTemplateValue>,
}

struct SourceGlobalSlots<'a> {
    proof_values: &'a BTreeMap<String, SourceProofValueSlot>,
    public_values: &'a BTreeMap<String, SourcePublicValueSlot>,
    challenges: &'a BTreeMap<String, SourceChallengeSlot>,
    group_values: &'a BTreeMap<String, SourceGroupValueSlot>,
}

type SourceGlobalExpressionAliases = BTreeMap<String, Expression>;
type SourceGlobalExpressionArrayAliases = BTreeMap<String, SourceGlobalExpressionArrayAlias>;

#[derive(Clone)]
enum SourceGlobalExpressionArrayAlias {
    Name(String),
    Values(Vec<Expression>),
}

fn top_level_global_expression_aliases(
    module: &SourceProgramModule,
) -> SourceGlobalExpressionAliases {
    module
        .constants
        .iter()
        .filter(|declaration| source_top_level_expr_alias_declaration(module, declaration))
        .filter_map(|declaration| {
            Some((
                declaration.name.clone(),
                declaration.initializer_expression.as_ref()?.clone(),
            ))
        })
        .collect()
}

fn source_top_level_expr_alias_declaration(
    module: &SourceProgramModule,
    declaration: &ConstantDeclaration,
) -> bool {
    declaration.type_name.as_deref() == Some("expr")
        && declaration.array_dims.is_empty()
        && !source_global_declaration_in_nested_body(module, declaration.start, declaration.end)
}

fn top_level_global_expression_array_aliases(
    module: &SourceProgramModule,
) -> SourceGlobalExpressionArrayAliases {
    module
        .constants
        .iter()
        .filter(|declaration| source_top_level_expr_array_alias_declaration(module, declaration))
        .filter_map(|declaration| {
            Some((
                declaration.name.clone(),
                source_global_expression_array_alias(declaration.initializer_expression.as_ref()?)?,
            ))
        })
        .collect()
}

fn source_top_level_expr_array_alias_declaration(
    module: &SourceProgramModule,
    declaration: &ConstantDeclaration,
) -> bool {
    declaration.type_name.as_deref() == Some("expr")
        && !declaration.array_dims.is_empty()
        && !source_global_declaration_in_nested_body(module, declaration.start, declaration.end)
}

fn source_global_expression_array_alias(
    expression: &Expression,
) -> Option<SourceGlobalExpressionArrayAlias> {
    match &strip_group_expression(expression).kind {
        ExpressionKind::Name(name) => Some(SourceGlobalExpressionArrayAlias::Name(name.clone())),
        ExpressionKind::Array(expressions) => Some(SourceGlobalExpressionArrayAlias::Values(
            expressions.clone(),
        )),
        _ => None,
    }
}

fn source_global_declaration_in_nested_body(
    module: &SourceProgramModule,
    start: usize,
    end: usize,
) -> bool {
    source_global_declaration_in_template_body(module, start, end)
        || source_global_declaration_in_group_body(module, start, end)
        || source_global_declaration_in_container_body(module, start, end)
        || source_global_declaration_in_function_body(module, start, end)
}

fn source_global_declaration_in_template_body(
    module: &SourceProgramModule,
    start: usize,
    end: usize,
) -> bool {
    module
        .air_templates
        .iter()
        .any(|template| template.body.start <= start && end <= template.body.end)
}

fn source_global_declaration_in_group_body(
    module: &SourceProgramModule,
    start: usize,
    end: usize,
) -> bool {
    module
        .air_groups
        .iter()
        .any(|group| group.body.start <= start && end <= group.body.end)
}

fn source_global_declaration_in_container_body(
    module: &SourceProgramModule,
    start: usize,
    end: usize,
) -> bool {
    module.containers.iter().any(|container| {
        container
            .body
            .is_some_and(|body| body.start <= start && end <= body.end)
    })
}

fn source_global_declaration_in_function_body(
    module: &SourceProgramModule,
    start: usize,
    end: usize,
) -> bool {
    module
        .functions
        .iter()
        .any(|function| function.body.start <= start && end <= function.body.end)
}

fn source_proof_value_slots(
    global_info: &GlobalInfo,
) -> Result<BTreeMap<String, SourceProofValueSlot>, SourceKeyDirectoryMetadataError> {
    let mut slots = BTreeMap::new();
    let mut next_offset = 0_u32;
    for entry in &global_info.proof_values_map {
        let dimension = source_global_named_stage_value_dimension(&entry.lengths)?;
        slots.insert(
            entry.name.clone(),
            SourceProofValueSlot {
                offset: next_offset,
                stage: entry.stage,
                dimension,
            },
        );
        let width = if entry.stage == 1 { 1 } else { 3 };
        let field_width = dimension
            .checked_mul(width)
            .ok_or_else(|| unsupported_source_message("source proof value offset overflow"))?;
        next_offset = next_offset
            .checked_add(field_width)
            .ok_or_else(|| unsupported_source_message("source proof value offset overflow"))?;
    }
    Ok(slots)
}

fn source_public_value_slots(
    global_info: &GlobalInfo,
) -> Result<BTreeMap<String, SourcePublicValueSlot>, SourceKeyDirectoryMetadataError> {
    let mut slots = BTreeMap::new();
    let mut next_offset = 0_u32;
    for entry in &global_info.publics_map {
        let dimension = source_global_public_value_dimension(&entry.lengths)?;
        slots.insert(
            entry.name.clone(),
            SourcePublicValueSlot {
                offset: next_offset,
                stage: entry.stage,
                dimension,
            },
        );
        next_offset = next_offset
            .checked_add(dimension)
            .ok_or_else(|| unsupported_source_message("source public value offset overflow"))?;
    }
    Ok(slots)
}

fn source_global_challenge_slots(
    slots: &[SourceChallengeSlotMetadata],
) -> BTreeMap<String, SourceChallengeSlot> {
    slots
        .iter()
        .map(|slot| {
            (
                slot.name.clone(),
                SourceChallengeSlot {
                    id: slot.id,
                    dimension: slot.dimension,
                },
            )
        })
        .collect()
}

fn source_global_group_value_slots(
    values: &[StageValue],
) -> Result<BTreeMap<String, SourceGroupValueSlot>, SourceKeyDirectoryMetadataError> {
    let mut slots = BTreeMap::new();
    let mut next_offset = 0_u32;
    for value in values {
        let dimension = source_global_stage_value_dimension(&value.lengths)?;
        slots.insert(
            value.name.clone(),
            SourceGroupValueSlot {
                offset: next_offset,
                stage: value.stage,
                dimension,
            },
        );
        let width = if value.stage == 1 { 1 } else { 3 };
        let field_width = dimension
            .checked_mul(width)
            .ok_or_else(|| unsupported_source_message("source group value offset overflow"))?;
        next_offset = next_offset
            .checked_add(field_width)
            .ok_or_else(|| unsupported_source_message("source group value offset overflow"))?;
    }
    Ok(slots)
}

fn public_value_target_offset(
    slot: SourcePublicValueSlot,
    index: Option<u32>,
) -> Result<u32, SourceKeyDirectoryMetadataError> {
    let Some(index) = index else {
        if slot.dimension == 1 {
            return Ok(slot.offset);
        }
        return unsupported("top-level public value constraints require scalar values");
    };
    if index >= slot.dimension {
        return unsupported("top-level public value index is out of range");
    }
    slot.offset
        .checked_add(index)
        .ok_or_else(|| unsupported_source_message("source public value offset overflow"))
}

fn challenge_target_offset(
    slot: SourceChallengeSlot,
    index: Option<u32>,
) -> Result<u32, SourceKeyDirectoryMetadataError> {
    let index = match index {
        Some(index) => {
            if index >= slot.dimension {
                return unsupported("top-level challenge value index is out of range");
            }
            index
        }
        None => {
            if slot.dimension == 1 {
                0
            } else {
                return unsupported("top-level challenge constraints require scalar values");
            }
        }
    };
    slot.id
        .checked_add(index)
        .and_then(|id| id.checked_mul(3))
        .ok_or_else(|| unsupported_source_message("source challenge offset overflow"))
}

fn group_value_target_offset(
    slot: SourceGroupValueSlot,
    index: Option<u32>,
) -> Result<u32, SourceKeyDirectoryMetadataError> {
    let index = match index {
        Some(index) => {
            if index >= slot.dimension {
                return unsupported("top-level group value index is out of range");
            }
            index
        }
        None => {
            if slot.dimension == 1 {
                0
            } else {
                return unsupported("top-level group value constraints require scalar values");
            }
        }
    };
    let width = if slot.stage == 1 { 1 } else { 3 };
    slot.offset
        .checked_add(
            index
                .checked_mul(width)
                .ok_or_else(|| unsupported_source_message("source group value offset overflow"))?,
        )
        .ok_or_else(|| unsupported_source_message("source group value offset overflow"))
}

fn proof_value_target_offset(
    slot: SourceProofValueSlot,
    index: Option<u32>,
) -> Result<u32, SourceKeyDirectoryMetadataError> {
    let index = match index {
        Some(index) => {
            if index >= slot.dimension {
                return unsupported("top-level proof value index is out of range");
            }
            index
        }
        None => {
            if slot.dimension == 1 {
                0
            } else {
                return unsupported("top-level proof value constraints require scalar values");
            }
        }
    };
    let width = proof_value_operand_dimension(slot.stage);
    slot.offset
        .checked_add(
            index
                .checked_mul(width)
                .ok_or_else(|| unsupported_source_message("source proof value offset overflow"))?,
        )
        .ok_or_else(|| unsupported_source_message("source proof value offset overflow"))
}

fn proof_value_boolean_constraint_target(
    expression: &Expression,
    alias_scope: &SourceGlobalAliasScope<'_>,
    resolving_aliases: &mut BTreeSet<String>,
    resolving_array_aliases: &mut BTreeSet<String>,
) -> Option<SourceBooleanTarget> {
    let ExpressionKind::Binary { op, left, right } = &strip_group_expression(expression).kind
    else {
        return None;
    };
    if *op != BinaryOperator::Multiply {
        return None;
    }

    if let Some(target) = expression_target(
        left,
        alias_scope,
        resolving_aliases,
        resolving_array_aliases,
    ) {
        if one_minus_target(
            right,
            &target,
            alias_scope,
            resolving_aliases,
            resolving_array_aliases,
        ) || target_minus_one(
            right,
            &target,
            alias_scope,
            resolving_aliases,
            resolving_array_aliases,
        ) {
            return Some(target);
        }
    }
    if let Some(target) = expression_target(
        right,
        alias_scope,
        resolving_aliases,
        resolving_array_aliases,
    ) {
        if one_minus_target(
            left,
            &target,
            alias_scope,
            resolving_aliases,
            resolving_array_aliases,
        ) || target_minus_one(
            left,
            &target,
            alias_scope,
            resolving_aliases,
            resolving_array_aliases,
        ) {
            return Some(target);
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

fn expression_target(
    expression: &Expression,
    alias_scope: &SourceGlobalAliasScope<'_>,
    resolving_aliases: &mut BTreeSet<String>,
    resolving_array_aliases: &mut BTreeSet<String>,
) -> Option<SourceBooleanTarget> {
    match &strip_group_expression(expression).kind {
        ExpressionKind::Name(name) => {
            if let Some(alias) = alias_scope.expressions.get(name) {
                if !resolving_aliases.insert(name.clone()) {
                    return None;
                }
                let target = expression_target(
                    alias,
                    alias_scope,
                    resolving_aliases,
                    resolving_array_aliases,
                );
                resolving_aliases.remove(name);
                return target;
            }
            Some(SourceBooleanTarget {
                name: name.clone(),
                index: None,
            })
        }
        ExpressionKind::Index { target, index } => {
            let ExpressionKind::Name(name) = &strip_group_expression(target).kind else {
                return None;
            };
            let index = static_u32_expression(index, alias_scope)?;
            if let Some(alias) = alias_scope.expression_arrays.get(name) {
                let element = source_global_expression_array_alias_element(
                    alias,
                    usize::try_from(index).ok()?,
                    &alias_scope.expression_arrays,
                    resolving_array_aliases,
                )?;
                return match element {
                    SourceGlobalExpressionArrayAliasElement::Expression(expression) => {
                        expression_target(
                            expression,
                            alias_scope,
                            resolving_aliases,
                            resolving_array_aliases,
                        )
                    }
                    SourceGlobalExpressionArrayAliasElement::NamedArray(name) => {
                        Some(SourceBooleanTarget {
                            name: name.to_owned(),
                            index: Some(index),
                        })
                    }
                };
            }
            Some(SourceBooleanTarget {
                name: name.clone(),
                index: Some(index),
            })
        }
        _ => None,
    }
}

enum SourceGlobalExpressionArrayAliasElement<'a> {
    Expression(&'a Expression),
    NamedArray(&'a str),
}

fn source_global_expression_array_alias_element<'a>(
    alias: &'a SourceGlobalExpressionArrayAlias,
    index: usize,
    expression_array_aliases: &'a SourceGlobalExpressionArrayAliases,
    resolving_array_aliases: &mut BTreeSet<String>,
) -> Option<SourceGlobalExpressionArrayAliasElement<'a>> {
    match alias {
        SourceGlobalExpressionArrayAlias::Name(name) => {
            if let Some(next_alias) = expression_array_aliases.get(name) {
                if !resolving_array_aliases.insert(name.clone()) {
                    return None;
                }
                let element = source_global_expression_array_alias_element(
                    next_alias,
                    index,
                    expression_array_aliases,
                    resolving_array_aliases,
                );
                resolving_array_aliases.remove(name);
                return element;
            }
            Some(SourceGlobalExpressionArrayAliasElement::NamedArray(name))
        }
        SourceGlobalExpressionArrayAlias::Values(expressions) => expressions
            .get(index)
            .map(SourceGlobalExpressionArrayAliasElement::Expression),
    }
}

fn one_minus_target(
    expression: &Expression,
    target: &SourceBooleanTarget,
    alias_scope: &SourceGlobalAliasScope<'_>,
    resolving_aliases: &mut BTreeSet<String>,
    resolving_array_aliases: &mut BTreeSet<String>,
) -> bool {
    let ExpressionKind::Binary { op, left, right } = &strip_group_expression(expression).kind
    else {
        return false;
    };
    *op == BinaryOperator::Subtract
        && expression_is_one(left, alias_scope)
        && expression_target(
            right,
            alias_scope,
            resolving_aliases,
            resolving_array_aliases,
        )
        .as_ref()
            == Some(target)
}

fn target_minus_one(
    expression: &Expression,
    target: &SourceBooleanTarget,
    alias_scope: &SourceGlobalAliasScope<'_>,
    resolving_aliases: &mut BTreeSet<String>,
    resolving_array_aliases: &mut BTreeSet<String>,
) -> bool {
    let ExpressionKind::Binary { op, left, right } = &strip_group_expression(expression).kind
    else {
        return false;
    };
    *op == BinaryOperator::Subtract
        && expression_target(
            left,
            alias_scope,
            resolving_aliases,
            resolving_array_aliases,
        )
        .as_ref()
            == Some(target)
        && expression_is_one(right, alias_scope)
}

fn expression_is_one(expression: &Expression, alias_scope: &SourceGlobalAliasScope<'_>) -> bool {
    match evaluate_source_static_expression(
        alias_scope.program,
        expression,
        &alias_scope.static_values,
    ) {
        Some(value) => static_value_integer(&value).is_some_and(|value| value == 1),
        None => false,
    }
}

fn static_u32_expression(
    expression: &Expression,
    alias_scope: &SourceGlobalAliasScope<'_>,
) -> Option<u32> {
    let value = evaluate_source_static_expression(
        alias_scope.program,
        expression,
        &alias_scope.static_values,
    )?;
    u32::try_from(static_value_integer(&value)?).ok()
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

#[derive(Clone, Copy)]
struct SourceGlobalConstraintBuilderCheckpoint {
    entries: usize,
    ops: usize,
    args: usize,
    numbers: usize,
}

impl SourceGlobalConstraintBuilder {
    fn checkpoint(&self) -> SourceGlobalConstraintBuilderCheckpoint {
        SourceGlobalConstraintBuilderCheckpoint {
            entries: self.entries.len(),
            ops: self.ops.len(),
            args: self.args.len(),
            numbers: self.numbers.len(),
        }
    }

    fn rollback(&mut self, checkpoint: SourceGlobalConstraintBuilderCheckpoint) {
        self.entries.truncate(checkpoint.entries);
        self.ops.truncate(checkpoint.ops);
        self.args.truncate(checkpoint.args);
        self.numbers.truncate(checkpoint.numbers);
    }

    fn append_base_residual_constraint(
        &mut self,
        expression: &Expression,
        proof_value_slots: &BTreeMap<String, SourceProofValueSlot>,
        public_value_slots: &BTreeMap<String, SourcePublicValueSlot>,
        alias_scope: &SourceGlobalAliasScope<'_>,
        source_line: String,
    ) -> Result<bool, SourceKeyDirectoryMetadataError> {
        let ops_offset = source_usize_to_u32(self.ops.len(), "source global op offset overflow")?;
        let args_offset =
            source_usize_to_u32(self.args.len(), "source global argument offset overflow")?;
        let ops_start = self.ops.len();
        let args_start = self.args.len();
        let numbers_start = self.numbers.len();
        let (destination_id, temp1_count) = {
            let mut context = SourceGlobalBaseLoweringContext {
                builder: self,
                proof_value_slots,
                public_value_slots,
                alias_scope,
                resolving_aliases: BTreeSet::new(),
                resolving_array_aliases: BTreeSet::new(),
                next_temp: 0,
            };
            let Some(operand) = lower_global_base_residual_operand(expression, &mut context)?
            else {
                self.ops.truncate(ops_start);
                self.args.truncate(args_start);
                self.numbers.truncate(numbers_start);
                return Ok(false);
            };
            let destination = context.ensure_temp_operand(operand)?;
            (destination.offset, context.next_temp)
        };
        let ops_count = source_usize_to_u32(
            self.ops.len().saturating_sub(ops_start),
            "source global op count overflow",
        )?;
        let args_count = source_usize_to_u32(
            self.args.len().saturating_sub(args_start),
            "source global argument count overflow",
        )?;
        self.entries.push(GlobalConstraintEntry {
            destination_dimension: 1,
            destination_id,
            temp1_count,
            temp3_count: 0,
            ops_count,
            ops_offset,
            args_count,
            args_offset,
            source_line,
        });
        Ok(true)
    }

    fn append_ext_residual_constraint(
        &mut self,
        expression: &Expression,
        slots: &SourceGlobalSlots<'_>,
        alias_scope: &SourceGlobalAliasScope<'_>,
        source_line: String,
    ) -> Result<bool, SourceKeyDirectoryMetadataError> {
        residuals::append_ext_residual_constraint(self, expression, slots, alias_scope, source_line)
    }

    fn append_public_value_constant_constraint(
        &mut self,
        public_value_offset: u32,
        value: u64,
        source_line: String,
    ) -> Result<(), SourceKeyDirectoryMetadataError> {
        let ops_offset = source_usize_to_u32(self.ops.len(), "source global op offset overflow")?;
        let args_offset =
            source_usize_to_u32(self.args.len(), "source global argument offset overflow")?;
        let public_value_offset =
            source_u32_to_u16(public_value_offset, "source public value offset overflow")?;
        let value_offset =
            source_u32_to_u16(self.intern_number(value)?, "source number offset overflow")?;

        self.ops.push(0);
        self.args
            .extend([1, 0, 1, public_value_offset, 2, value_offset]);
        self.entries.push(GlobalConstraintEntry {
            destination_dimension: 1,
            destination_id: 0,
            temp1_count: 1,
            temp3_count: 0,
            ops_count: 1,
            ops_offset,
            args_count: 6,
            args_offset,
            source_line,
        });
        Ok(())
    }

    fn append_public_value_boolean_constraint(
        &mut self,
        public_value_offset: u32,
        source_line: String,
    ) -> Result<(), SourceKeyDirectoryMetadataError> {
        self.append_base_scalar_boolean_constraint(
            1,
            public_value_offset,
            "source public value offset overflow",
            source_line,
        )
    }

    fn append_proof_value_boolean_constraint(
        &mut self,
        proof_value_offset: u32,
        dimension: u32,
        source_line: String,
    ) -> Result<(), SourceKeyDirectoryMetadataError> {
        match dimension {
            1 => self.append_base_proof_value_boolean_constraint(proof_value_offset, source_line),
            3 => self.append_ext_proof_value_boolean_constraint(proof_value_offset, source_line),
            _ => unsupported("unsupported source proof value dimension"),
        }
    }

    fn append_base_proof_value_boolean_constraint(
        &mut self,
        proof_value_offset: u32,
        source_line: String,
    ) -> Result<(), SourceKeyDirectoryMetadataError> {
        self.append_base_scalar_boolean_constraint(
            3,
            proof_value_offset,
            "source proof value offset overflow",
            source_line,
        )
    }

    fn append_base_scalar_boolean_constraint(
        &mut self,
        source_buffer: u16,
        source_offset: u32,
        offset_overflow_message: &'static str,
        source_line: String,
    ) -> Result<(), SourceKeyDirectoryMetadataError> {
        let ops_offset = source_usize_to_u32(self.ops.len(), "source global op offset overflow")?;
        let args_offset =
            source_usize_to_u32(self.args.len(), "source global argument offset overflow")?;
        let one_offset = self.intern_number(1)?;
        let source_offset = source_u32_to_u16(source_offset, offset_overflow_message)?;
        let one_offset = source_u32_to_u16(one_offset, "source number offset overflow")?;

        self.ops.extend([0, 0]);
        self.args.extend([
            1,
            0,
            2,
            one_offset,
            source_buffer,
            source_offset,
            2,
            1,
            source_buffer,
            source_offset,
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

    fn append_ext_proof_value_boolean_constraint(
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

        self.ops.extend([1, 2]);
        self.args.extend([
            3,
            0,
            3,
            proof_value_offset,
            2,
            one_offset,
            2,
            3,
            3,
            proof_value_offset,
            4,
            0,
        ]);
        self.entries.push(GlobalConstraintEntry {
            destination_dimension: 3,
            destination_id: 1,
            temp1_count: 0,
            temp3_count: 2,
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

struct SourceGlobalBaseLoweringContext<'a, 'b> {
    builder: &'a mut SourceGlobalConstraintBuilder,
    proof_value_slots: &'b BTreeMap<String, SourceProofValueSlot>,
    public_value_slots: &'b BTreeMap<String, SourcePublicValueSlot>,
    alias_scope: &'b SourceGlobalAliasScope<'b>,
    resolving_aliases: BTreeSet<String>,
    resolving_array_aliases: BTreeSet<String>,
    next_temp: u32,
}

impl SourceGlobalBaseLoweringContext<'_, '_> {
    fn number_operand(
        &mut self,
        value: u64,
    ) -> Result<SourceGlobalBaseOperand, SourceKeyDirectoryMetadataError> {
        Ok(SourceGlobalBaseOperand {
            buffer: 2,
            offset: self.builder.intern_number(value)?,
        })
    }

    fn zero_operand(&mut self) -> Result<SourceGlobalBaseOperand, SourceKeyDirectoryMetadataError> {
        self.number_operand(0)
    }

    fn append_base_binary_op(
        &mut self,
        kind: u16,
        left: SourceGlobalBaseOperand,
        right: SourceGlobalBaseOperand,
    ) -> Result<SourceGlobalBaseOperand, SourceKeyDirectoryMetadataError> {
        let destination = self.next_temp;
        self.next_temp = self
            .next_temp
            .checked_add(1)
            .ok_or_else(|| unsupported_source_message("source global temporary overflow"))?;
        let destination =
            source_u32_to_u16(destination, "source global temporary offset overflow")?;
        let left_offset = source_u32_to_u16(left.offset, "source global operand offset overflow")?;
        let right_offset =
            source_u32_to_u16(right.offset, "source global operand offset overflow")?;

        self.builder.ops.push(0);
        self.builder.args.extend([
            kind,
            destination,
            left.buffer,
            left_offset,
            right.buffer,
            right_offset,
        ]);
        Ok(SourceGlobalBaseOperand {
            buffer: 0,
            offset: u32::from(destination),
        })
    }

    fn ensure_temp_operand(
        &mut self,
        operand: SourceGlobalBaseOperand,
    ) -> Result<SourceGlobalBaseOperand, SourceKeyDirectoryMetadataError> {
        if operand.buffer == 0 {
            return Ok(operand);
        }
        let zero = self.zero_operand()?;
        self.append_base_binary_op(1, operand, zero)
    }
}

fn lower_global_base_residual_operand(
    expression: &Expression,
    context: &mut SourceGlobalBaseLoweringContext<'_, '_>,
) -> Result<Option<SourceGlobalBaseOperand>, SourceKeyDirectoryMetadataError> {
    if let Some(value) = evaluate_source_static_expression(
        context.alias_scope.program,
        expression,
        &context.alias_scope.static_values,
    ) {
        return Ok(Some(
            context.number_operand(source_public_initializer_field_value(&value)?)?,
        ));
    }
    match &strip_group_expression(expression).kind {
        ExpressionKind::Group(inner) => lower_global_base_residual_operand(inner, context),
        ExpressionKind::Name(name) => lower_global_base_name_operand(name, None, context),
        ExpressionKind::Index { target, index } => {
            lower_global_base_index_operand(target, index, context)
        }
        ExpressionKind::Unary { op, expr } => match op {
            UnaryOperator::Plus => lower_global_base_residual_operand(expr, context),
            UnaryOperator::Minus => {
                let Some(value) = lower_global_base_residual_operand(expr, context)? else {
                    return Ok(None);
                };
                let zero = context.zero_operand()?;
                context.append_base_binary_op(1, zero, value).map(Some)
            }
            _ => Ok(None),
        },
        ExpressionKind::Binary { op, left, right } => {
            let kind = match op {
                BinaryOperator::Add => 0,
                BinaryOperator::Subtract => 1,
                BinaryOperator::Multiply => 2,
                BinaryOperator::Divide | BinaryOperator::Backslash => {
                    return lower_global_base_static_divisor_operand(left, right, context);
                }
                BinaryOperator::Power => {
                    return lower_global_base_static_exponent_operand(left, right, context);
                }
                _ => return Ok(None),
            };
            let Some(left) = lower_global_base_residual_operand(left, context)? else {
                return Ok(None);
            };
            let Some(right) = lower_global_base_residual_operand(right, context)? else {
                return Ok(None);
            };
            context.append_base_binary_op(kind, left, right).map(Some)
        }
        _ => Ok(None),
    }
}

fn lower_global_base_static_divisor_operand(
    left: &Expression,
    right: &Expression,
    context: &mut SourceGlobalBaseLoweringContext<'_, '_>,
) -> Result<Option<SourceGlobalBaseOperand>, SourceKeyDirectoryMetadataError> {
    let Some(value) = evaluate_source_static_expression(
        context.alias_scope.program,
        right,
        &context.alias_scope.static_values,
    ) else {
        return Ok(None);
    };
    let divisor = Felt::from_u64(source_public_initializer_field_value(&value)?);
    let inverse = divisor
        .inverse()
        .ok_or_else(|| unsupported_source_message("source global constraint division by zero"))?;
    let Some(left) = lower_global_base_residual_operand(left, context)? else {
        return Ok(None);
    };
    let inverse = context.number_operand(inverse.to_u64())?;
    context.append_base_binary_op(2, left, inverse).map(Some)
}

fn lower_global_base_static_exponent_operand(
    left: &Expression,
    right: &Expression,
    context: &mut SourceGlobalBaseLoweringContext<'_, '_>,
) -> Result<Option<SourceGlobalBaseOperand>, SourceKeyDirectoryMetadataError> {
    let Some(mut exponent) = static_u32_expression(right, context.alias_scope) else {
        return Ok(None);
    };
    if exponent == 0 {
        return context.number_operand(1).map(Some);
    }
    let Some(mut power) = lower_global_base_residual_operand(left, context)? else {
        return Ok(None);
    };
    let mut result = None;
    while exponent > 0 {
        if exponent & 1 == 1 {
            result = Some(match result {
                Some(value) => context.append_base_binary_op(2, value, power)?,
                None => power,
            });
        }
        exponent >>= 1;
        if exponent > 0 {
            power = context.append_base_binary_op(2, power, power)?;
        }
    }
    Ok(result)
}

fn lower_global_base_index_operand(
    target: &Expression,
    index: &Expression,
    context: &mut SourceGlobalBaseLoweringContext<'_, '_>,
) -> Result<Option<SourceGlobalBaseOperand>, SourceKeyDirectoryMetadataError> {
    let ExpressionKind::Name(name) = &strip_group_expression(target).kind else {
        return Ok(None);
    };
    let Some(index) = static_u32_expression(index, context.alias_scope) else {
        return Ok(None);
    };
    if let Some(alias) = context.alias_scope.expression_arrays.get(name) {
        if !context.resolving_array_aliases.insert(name.clone()) {
            return Ok(None);
        }
        let element = source_global_expression_array_alias_element(
            alias,
            usize::try_from(index)
                .map_err(|_| unsupported_source_message("source global index overflow"))?,
            &context.alias_scope.expression_arrays,
            &mut context.resolving_array_aliases,
        );
        context.resolving_array_aliases.remove(name);
        return match element {
            Some(SourceGlobalExpressionArrayAliasElement::Expression(expression)) => {
                lower_global_base_residual_operand(expression, context)
            }
            Some(SourceGlobalExpressionArrayAliasElement::NamedArray(name)) => {
                lower_global_base_name_operand(name, Some(index), context)
            }
            None => Ok(None),
        };
    }
    lower_global_base_name_operand(name, Some(index), context)
}

fn lower_global_base_name_operand(
    name: &str,
    index: Option<u32>,
    context: &mut SourceGlobalBaseLoweringContext<'_, '_>,
) -> Result<Option<SourceGlobalBaseOperand>, SourceKeyDirectoryMetadataError> {
    if index.is_none() {
        if let Some(alias) = context.alias_scope.expressions.get(name) {
            if !context.resolving_aliases.insert(name.to_owned()) {
                return Ok(None);
            }
            let operand = lower_global_base_residual_operand(alias, context);
            context.resolving_aliases.remove(name);
            return operand;
        }
    }
    if let Some(slot) = context.public_value_slots.get(name).copied() {
        if slot.stage != 1 {
            return unsupported("top-level base residuals require base-field public values");
        }
        let offset = public_value_target_offset(slot, index)?;
        return Ok(Some(SourceGlobalBaseOperand { buffer: 1, offset }));
    }
    if let Some(slot) = context.proof_value_slots.get(name).copied() {
        if proof_value_operand_dimension(slot.stage) != 1 {
            return Ok(None);
        }
        let offset = proof_value_target_offset(slot, index)?;
        return Ok(Some(SourceGlobalBaseOperand { buffer: 3, offset }));
    }
    Ok(None)
}

fn proof_value_operand_dimension(stage: u64) -> u32 {
    if stage == 1 {
        1
    } else {
        3
    }
}

fn source_global_public_value_dimension(
    lengths: &[u64],
) -> Result<u32, SourceKeyDirectoryMetadataError> {
    let dimension = lengths.iter().try_fold(1_u64, |acc, length| {
        acc.checked_mul(*length)
            .ok_or_else(|| unsupported_source_message("source public value dimension overflow"))
    })?;
    u32::try_from(dimension)
        .map_err(|_| unsupported_source_message("source public value dimension overflow"))
}

fn source_global_named_stage_value_dimension(
    lengths: &[u64],
) -> Result<u32, SourceKeyDirectoryMetadataError> {
    let dimension = lengths.iter().try_fold(1_u64, |acc, length| {
        acc.checked_mul(*length)
            .ok_or_else(|| unsupported_source_message("source proof value dimension overflow"))
    })?;
    u32::try_from(dimension)
        .map_err(|_| unsupported_source_message("source proof value dimension overflow"))
}

fn source_global_stage_value_dimension(
    lengths: &[u32],
) -> Result<u32, SourceKeyDirectoryMetadataError> {
    lengths.iter().try_fold(1_u32, |acc, length| {
        acc.checked_mul(*length)
            .ok_or_else(|| unsupported_source_message("source stage value dimension overflow"))
    })
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

fn unsupported_source_message(message: impl Into<String>) -> SourceKeyDirectoryMetadataError {
    SourceKeyDirectoryMetadataError::UnsupportedSourceProgram {
        message: message.into(),
    }
}
