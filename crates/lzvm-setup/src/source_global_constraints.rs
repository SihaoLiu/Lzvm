use std::collections::{BTreeMap, BTreeSet};

use lzvm_artifacts::constraint_program::{GlobalConstraintEntry, GlobalConstraintProgram};
use lzvm_artifacts::global_info::GlobalInfo;
use lzvm_artifacts::global_program::GlobalProgram;
use lzvm_artifacts::hint_program::HintProgram;
use lzvm_pil::{
    lex_source, parse_expression, BinaryOperator, ConstantDeclaration, Expression, ExpressionKind,
    FixedFileTemplateValue, SourceProgram, SourceProgramModule, Token, TokenKind,
};

use crate::{
    source_key_directory::SourceKeyDirectoryMetadataError,
    source_scope::global_constraint_source_names,
    source_static_values::{evaluate_source_static_expression, source_scalar_constant_values},
};

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
    let mut constraints = SourceGlobalConstraintBuilder::default();
    for module in &program.modules {
        if !global_source_names.contains(&module.source_name) {
            continue;
        }
        lower_module_top_level_global_constraints(
            program,
            module,
            &proof_value_slots,
            &public_value_slots,
            &static_values,
            &mut constraints,
        )?;
    }
    Ok(GlobalProgram {
        constraints: constraints.finish(),
        hints: HintProgram { hints: Vec::new() },
    })
}

fn lower_module_top_level_global_constraints(
    program: &SourceProgram,
    module: &SourceProgramModule,
    proof_value_slots: &BTreeMap<String, SourceProofValueSlot>,
    public_value_slots: &BTreeMap<String, SourcePublicValueSlot>,
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
                        public_value_slots,
                        &alias_scope,
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
                    public_value_slots,
                    &alias_scope,
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
    public_value_slots: &BTreeMap<String, SourcePublicValueSlot>,
    alias_scope: &SourceGlobalAliasScope<'_>,
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
        public_value_slots,
        alias_scope,
        constraints,
    )?;
    Ok(next_index)
}

fn lower_top_level_global_constraint(
    expression: &Expression,
    source_line: &str,
    proof_value_slots: &BTreeMap<String, SourceProofValueSlot>,
    public_value_slots: &BTreeMap<String, SourcePublicValueSlot>,
    alias_scope: &SourceGlobalAliasScope<'_>,
    constraints: &mut SourceGlobalConstraintBuilder,
) -> Result<(), SourceKeyDirectoryMetadataError> {
    let mut resolving_aliases = BTreeSet::new();
    let mut resolving_array_aliases = BTreeSet::new();
    let Some(target) = proof_value_boolean_constraint_target(
        expression,
        alias_scope,
        &mut resolving_aliases,
        &mut resolving_array_aliases,
    ) else {
        return unsupported(format!(
            "top-level statements need global constraint lowering support: {}",
            source_line.trim()
        ));
    };
    if let Some(slot) = proof_value_slots.get(&target.name).copied() {
        if target.index.is_some() {
            return unsupported("top-level proof value constraints require scalar values");
        }
        return constraints.append_proof_value_boolean_constraint(
            slot.offset,
            proof_value_operand_dimension(slot.stage),
            source_line.trim().to_owned(),
        );
    }
    if let Some(slot) = public_value_slots.get(&target.name).copied() {
        if slot.stage != 1 {
            return unsupported("top-level public value constraints require scalar values");
        }
        let offset = public_value_target_offset(slot, target.index)?;
        return constraints
            .append_public_value_boolean_constraint(offset, source_line.trim().to_owned());
    }
    unsupported("top-level boolean constraint references an unknown value")
}

#[derive(Debug, Clone, Copy)]
struct SourceProofValueSlot {
    offset: u32,
    stage: u64,
}

#[derive(Debug, Clone, Copy)]
struct SourcePublicValueSlot {
    offset: u32,
    stage: u64,
    dimension: u32,
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

type SourceGlobalExpressionAliases = BTreeMap<String, Expression>;
type SourceGlobalExpressionArrayAliases = BTreeMap<String, SourceGlobalExpressionArrayAlias>;

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
        && expression_is_one(left)
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

fn static_u32_expression(
    expression: &Expression,
    alias_scope: &SourceGlobalAliasScope<'_>,
) -> Option<u32> {
    match evaluate_source_static_expression(
        alias_scope.program,
        expression,
        &alias_scope.static_values,
    )? {
        FixedFileTemplateValue::Integer(value) => u32::try_from(value).ok(),
        FixedFileTemplateValue::Boolean(value) => Some(u32::from(value)),
        FixedFileTemplateValue::String(_) => None,
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
