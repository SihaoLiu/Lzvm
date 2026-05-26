use std::collections::BTreeMap;

use lzvm_pil::{
    BinaryOperator, Expression, ExpressionKind, FixedFileTemplateValue, FunctionStatement,
    FunctionStatementDeclaration, SourceProgram, UnaryOperator,
};

use crate::{
    source_expression_info::SourceExpressionAliasScope,
    source_expression_strings::source_expression_string_call_value,
    source_static_tokens::{source_token_index_after_end, source_token_index_at_start},
    source_static_values::{
        evaluate_source_static_expression, insert_source_static_array,
        insert_source_static_array_length,
    },
    source_template_context::SourceTemplateLoweringContext,
};

pub(crate) fn apply_source_static_declaration(
    program: &SourceProgram,
    statement: &FunctionStatement,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
) -> bool {
    match statement.declaration.as_ref() {
        Some(FunctionStatementDeclaration::Constant(declaration)) => {
            if !declaration.array_dims.is_empty() {
                if let Some(expression) = declaration.initializer_expression.as_ref() {
                    let Some(elements) =
                        source_static_array_expression(program, expression, values)
                    else {
                        return false;
                    };
                    return insert_source_static_array(values, &declaration.name, elements)
                        .is_some();
                }
                if declaration.type_name.as_deref() != Some("int") {
                    return false;
                }
                let Some(elements) = source_static_variable_array_elements(
                    program,
                    None,
                    &declaration.array_dim_expressions,
                    values,
                ) else {
                    return false;
                };
                return insert_source_static_array(values, &declaration.name, elements).is_some();
            }
            let Some(expression) = declaration.initializer_expression.as_ref() else {
                return false;
            };
            let Some(value) = evaluate_source_static_expression(program, expression, values) else {
                return false;
            };
            values.insert(declaration.name.clone(), value);
            true
        }
        Some(FunctionStatementDeclaration::Variable(declaration)) => {
            if !declaration.array_dims.is_empty() {
                if declaration.type_name == "expr" {
                    return false;
                }
                let Some(elements) = source_static_variable_array_elements(
                    program,
                    declaration.initializer_expression.as_ref(),
                    &declaration.array_dim_expressions,
                    values,
                ) else {
                    return false;
                };
                return insert_source_static_array(values, &declaration.name, elements).is_some();
            }
            let Some(expression) = declaration.initializer_expression.as_ref() else {
                return false;
            };
            let Some(value) = evaluate_source_static_expression(program, expression, values) else {
                return false;
            };
            values.insert(declaration.name.clone(), value);
            true
        }
        Some(FunctionStatementDeclaration::Column(declaration)) => {
            let mut inserted = false;
            for item in &declaration.items {
                if item.array_dims.is_empty() {
                    continue;
                }
                let Some(lengths) =
                    source_static_array_dimensions(program, &item.array_dim_expressions, values)
                else {
                    continue;
                };
                if insert_source_static_column_array_lengths(values, &item.name, &lengths) {
                    inserted = true;
                }
                if let Some(binding_name) = source_static_binding_name(&item.name) {
                    if binding_name != item.name
                        && insert_source_static_column_array_lengths(values, binding_name, &lengths)
                    {
                        inserted = true;
                    }
                }
            }
            inserted
        }
        _ => false,
    }
}

pub(crate) fn apply_source_static_expression_statement(
    program: &SourceProgram,
    expression: Option<&Expression>,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
) -> bool {
    let Some(expression) = expression else {
        return false;
    };
    match &expression.kind {
        ExpressionKind::Unary { op, expr } => {
            let delta = match op {
                UnaryOperator::Increment => 1,
                UnaryOperator::Decrement => -1,
                _ => return false,
            };
            let Some(name) = source_expression_name(expr) else {
                return false;
            };
            apply_source_static_delta(name, delta, values)
        }
        ExpressionKind::Binary { op, left, right } => {
            let Some(name) = source_expression_name(left) else {
                return false;
            };
            if !values.contains_key(name) {
                return false;
            }
            let Some(right) = evaluate_source_static_expression(program, right, values) else {
                return false;
            };
            let value = match op {
                BinaryOperator::Assign => right,
                BinaryOperator::PlusAssign => {
                    let Some(current) = source_static_integer_value(values.get(name)) else {
                        return false;
                    };
                    let Some(right) = source_static_integer_value(Some(&right)) else {
                        return false;
                    };
                    let Some(value) = current.checked_add(right) else {
                        return false;
                    };
                    FixedFileTemplateValue::Integer(value)
                }
                BinaryOperator::MinusAssign => {
                    let Some(current) = source_static_integer_value(values.get(name)) else {
                        return false;
                    };
                    let Some(right) = source_static_integer_value(Some(&right)) else {
                        return false;
                    };
                    let Some(value) = current.checked_sub(right) else {
                        return false;
                    };
                    FixedFileTemplateValue::Integer(value)
                }
                BinaryOperator::StarAssign => {
                    let Some(current) = source_static_integer_value(values.get(name)) else {
                        return false;
                    };
                    let Some(right) = source_static_integer_value(Some(&right)) else {
                        return false;
                    };
                    let Some(value) = current.checked_mul(right) else {
                        return false;
                    };
                    FixedFileTemplateValue::Integer(value)
                }
                _ => return false,
            };
            values.insert(name.to_owned(), value);
            true
        }
        _ => false,
    }
}

pub(crate) fn apply_source_static_array_assignment_statement(
    context: &SourceTemplateLoweringContext<'_>,
    statement: &FunctionStatement,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
) -> bool {
    let Some(value) = statement.value.as_ref() else {
        return false;
    };
    let Some(index) = source_token_index_at_start(context.tokens, value.start) else {
        return false;
    };
    let Some(end) = source_token_index_after_end(context.tokens, value.end) else {
        return false;
    };
    crate::source_static_array_assignment::execute_source_static_array_assignment_statement(
        context.program,
        context.module,
        context.tokens,
        index,
        end,
        values,
    )
    .is_some()
}

pub(crate) fn apply_source_expression_string_declaration(
    context: &SourceTemplateLoweringContext<'_>,
    statement: &FunctionStatement,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &SourceExpressionAliasScope,
) -> bool {
    let (name, expression) = match statement.declaration.as_ref() {
        Some(FunctionStatementDeclaration::Constant(declaration))
            if declaration.type_name.as_deref() == Some("string") =>
        {
            (
                &declaration.name,
                declaration.initializer_expression.as_ref(),
            )
        }
        Some(FunctionStatementDeclaration::Variable(declaration))
            if declaration.type_name == "string" =>
        {
            (
                &declaration.name,
                declaration.initializer_expression.as_ref(),
            )
        }
        _ => return false,
    };
    let Some(expression) = expression else {
        return false;
    };
    let Some(value) = source_expression_string_call_value(
        context.program,
        expression,
        values,
        &alias_scope.expressions,
        &alias_scope.expression_arrays,
    ) else {
        return false;
    };
    values.insert(name.clone(), FixedFileTemplateValue::String(value));
    true
}

pub(crate) fn apply_source_expression_string_assignment(
    context: &SourceTemplateLoweringContext<'_>,
    statement: &FunctionStatement,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
    alias_scope: &SourceExpressionAliasScope,
) -> bool {
    let Some(Expression {
        kind: ExpressionKind::Binary { op, left, right },
        ..
    }) = statement
        .value_expression
        .as_ref()
        .map(strip_source_group_expression)
    else {
        return false;
    };
    if *op != BinaryOperator::Assign {
        return false;
    }
    let Some(name) = source_expression_name(left) else {
        return false;
    };
    if !values.contains_key(name) {
        return false;
    }
    let Some(value) = source_expression_string_call_value(
        context.program,
        right,
        values,
        &alias_scope.expressions,
        &alias_scope.expression_arrays,
    ) else {
        return false;
    };
    values.insert(name.to_owned(), FixedFileTemplateValue::String(value));
    true
}

fn source_static_variable_array_elements(
    program: &SourceProgram,
    initializer_expression: Option<&Expression>,
    dim_expressions: &[Option<Expression>],
    values: &BTreeMap<String, FixedFileTemplateValue>,
) -> Option<Vec<FixedFileTemplateValue>> {
    if let Some(expression) = initializer_expression {
        return source_static_array_expression(program, expression, values);
    }
    let dimensions = source_static_array_dimensions(program, dim_expressions, values)?;
    let mut length = 1_usize;
    for dimension in dimensions {
        length = length.checked_mul(dimension)?;
    }
    Some(vec![FixedFileTemplateValue::Integer(0); length])
}

fn source_static_array_dimensions(
    program: &SourceProgram,
    dim_expressions: &[Option<Expression>],
    values: &BTreeMap<String, FixedFileTemplateValue>,
) -> Option<Vec<usize>> {
    dim_expressions
        .iter()
        .map(|expression| {
            let value = evaluate_source_static_expression(program, expression.as_ref()?, values)?;
            let dimension = usize::try_from(source_static_integer_value(Some(&value))?).ok()?;
            (dimension != 0).then_some(dimension)
        })
        .collect()
}

fn source_static_binding_name(name: &str) -> Option<&str> {
    name.rsplit_once('.')
        .map(|(_, binding_name)| binding_name)
        .filter(|binding_name| !binding_name.is_empty())
}

fn insert_source_static_column_array_lengths(
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
    name: &str,
    lengths: &[usize],
) -> bool {
    let Some((&length, rest)) = lengths.split_first() else {
        return false;
    };
    let Some(length_value) = i128::try_from(length).ok() else {
        return false;
    };
    let mut inserted = insert_source_static_array_length(values, name, length_value).is_some();
    if rest.is_empty() {
        return inserted;
    }
    for index in 0..length {
        let slice_name = format!("{name}[{index}]");
        if insert_source_static_column_array_lengths(values, &slice_name, rest) {
            inserted = true;
        }
    }
    inserted
}

fn source_static_array_expression(
    program: &SourceProgram,
    expression: &Expression,
    values: &BTreeMap<String, FixedFileTemplateValue>,
) -> Option<Vec<FixedFileTemplateValue>> {
    let expression = strip_source_group_expression(expression);
    let ExpressionKind::Array(elements) = &expression.kind else {
        return None;
    };
    elements
        .iter()
        .map(|element| evaluate_source_static_expression(program, element, values))
        .collect()
}

fn apply_source_static_delta(
    name: &str,
    delta: i128,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
) -> bool {
    let Some(current) = source_static_integer_value(values.get(name)) else {
        return false;
    };
    let Some(value) = current.checked_add(delta) else {
        return false;
    };
    values.insert(name.to_owned(), FixedFileTemplateValue::Integer(value));
    true
}

fn source_expression_name(expression: &Expression) -> Option<&str> {
    match &expression.kind {
        ExpressionKind::Name(name) => Some(name),
        ExpressionKind::Group(inner) => source_expression_name(inner),
        _ => None,
    }
}

fn strip_source_group_expression(expression: &Expression) -> &Expression {
    match &expression.kind {
        ExpressionKind::Group(inner) => strip_source_group_expression(inner),
        _ => expression,
    }
}

fn source_static_integer_value(value: Option<&FixedFileTemplateValue>) -> Option<i128> {
    match value {
        Some(FixedFileTemplateValue::Integer(value)) => Some(*value),
        Some(FixedFileTemplateValue::Boolean(value)) => Some(i128::from(*value)),
        _ => None,
    }
}
