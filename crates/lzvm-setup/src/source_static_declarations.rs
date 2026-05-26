use std::collections::BTreeMap;

use lzvm_pil::{
    ConstantDeclaration, Expression, FixedFileTemplateValue, SourceProgram, SourceProgramModule,
    SourceSpan, Token, VariableDeclaration,
};

use crate::source_static_values::{
    evaluate_source_static_expression_or_token_span, insert_source_static_array,
    source_static_array_expression, static_value_integer,
};

pub(crate) fn execute_static_template_declaration(
    program: &SourceProgram,
    module: &SourceProgramModule,
    tokens: &[Token],
    index: usize,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
) -> Option<()> {
    let start = tokens.get(index)?.start;
    if let Some(declaration) = module
        .constants
        .iter()
        .find(|declaration| declaration.start == start)
    {
        return execute_static_constant_declaration(program, module, tokens, declaration, values);
    }

    let declaration = module
        .variables
        .iter()
        .find(|declaration| declaration.start == start)?;
    execute_static_variable_declaration(program, module, tokens, declaration, values)
}

fn execute_static_constant_declaration(
    program: &SourceProgram,
    module: &SourceProgramModule,
    tokens: &[Token],
    declaration: &ConstantDeclaration,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
) -> Option<()> {
    if values.contains_key(&declaration.name) {
        return Some(());
    }
    if !declaration.array_dims.is_empty() {
        if declaration.initializer_expression.is_none()
            && declaration.type_name.as_deref() != Some("int")
        {
            return None;
        }
        let elements = source_static_array_elements(
            program,
            module,
            tokens,
            declaration.initializer_expression.as_ref(),
            &declaration.array_dim_expressions,
            &declaration.array_dims,
            values,
        )?;
        insert_source_static_array(values, &declaration.name, elements)?;
        return Some(());
    }
    let value = evaluate_source_static_expression_or_token_span(
        program,
        &module.source,
        tokens,
        declaration.initializer_expression.as_ref(),
        declaration.initializer,
        values,
    )?;
    values.insert(declaration.name.clone(), value);
    Some(())
}

fn execute_static_variable_declaration(
    program: &SourceProgram,
    module: &SourceProgramModule,
    tokens: &[Token],
    declaration: &VariableDeclaration,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
) -> Option<()> {
    if !declaration.array_dims.is_empty() {
        let elements = source_static_array_elements(
            program,
            module,
            tokens,
            declaration.initializer_expression.as_ref(),
            &declaration.array_dim_expressions,
            &declaration.array_dims,
            values,
        )?;
        insert_source_static_array(values, &declaration.name, elements)?;
        return Some(());
    }
    let value = evaluate_source_static_expression_or_token_span(
        program,
        &module.source,
        tokens,
        declaration.initializer_expression.as_ref(),
        declaration.initializer,
        values,
    )?;
    values.insert(declaration.name.clone(), value);
    Some(())
}

fn source_static_array_elements(
    program: &SourceProgram,
    module: &SourceProgramModule,
    tokens: &[Token],
    initializer_expression: Option<&Expression>,
    dim_expressions: &[Option<Expression>],
    dims: &[SourceSpan],
    values: &BTreeMap<String, FixedFileTemplateValue>,
) -> Option<Vec<FixedFileTemplateValue>> {
    if let Some(expression) = initializer_expression {
        return source_static_array_expression(program, expression, values);
    }
    if dim_expressions.len() != dims.len() {
        return None;
    }
    let mut length = 1_usize;
    for (expression, span) in dim_expressions.iter().zip(dims) {
        let value = evaluate_source_static_expression_or_token_span(
            program,
            &module.source,
            tokens,
            expression.as_ref(),
            Some(*span),
            values,
        )?;
        let dimension = usize::try_from(static_value_integer(&value)?).ok()?;
        if dimension == 0 {
            return None;
        }
        length = length.checked_mul(dimension)?;
    }
    Some(vec![FixedFileTemplateValue::Integer(0); length])
}
