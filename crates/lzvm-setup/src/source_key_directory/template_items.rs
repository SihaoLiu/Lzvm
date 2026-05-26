use std::collections::BTreeMap;
use std::path::PathBuf;

use lzvm_pil::{
    lex_source, parse_expression_tokens, BinaryOperator, ColumnDeclaration, ColumnItem, Expression,
    ExpressionKind, FixedFileTemplateValue, SourceFile, SourceProgram, UnaryOperator,
};

use crate::source_static_values::{evaluate_source_static_expression, static_value_integer};

use super::{unsupported, unsupported_source_message, SourceKeyDirectoryMetadataError};

pub(super) fn source_column_stage(
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

pub(crate) fn source_column_dimension(
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

pub(crate) fn source_item_lengths(
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

pub(crate) fn source_item_name(
    program: &SourceProgram,
    item: &ColumnItem,
    item_role: &str,
    constant_values: &BTreeMap<String, FixedFileTemplateValue>,
) -> Result<String, SourceKeyDirectoryMetadataError> {
    if !item.template {
        return Ok(item.name.clone());
    }
    source_template_text(program, &item.name, item_role, constant_values)
}

fn source_template_text(
    program: &SourceProgram,
    template: &str,
    item_role: &str,
    constant_values: &BTreeMap<String, FixedFileTemplateValue>,
) -> Result<String, SourceKeyDirectoryMetadataError> {
    let mut resolved = String::new();
    let mut cursor = 0;

    while let Some(relative_start) = template[cursor..].find("${") {
        let segment_start = cursor + relative_start;
        resolved.push_str(&template[cursor..segment_start]);
        let expression_start = segment_start + 2;
        let expression_end = source_template_expression_end(template, expression_start)
            .ok_or_else(|| {
                unsupported_source_message(format!("{item_role} template name is not closed"))
            })?;
        let expression = &template[expression_start..expression_end];
        let value =
            source_template_expression_value(program, expression, item_role, constant_values)?;
        resolved.push_str(&source_template_value_string(&value));
        cursor = expression_end + 1;
    }

    resolved.push_str(&template[cursor..]);
    Ok(resolved)
}

fn source_template_expression_value(
    program: &SourceProgram,
    expression: &str,
    item_role: &str,
    constant_values: &BTreeMap<String, FixedFileTemplateValue>,
) -> Result<FixedFileTemplateValue, SourceKeyDirectoryMetadataError> {
    let expression_source = SourceFile {
        contents: expression.to_owned(),
        file_dir: PathBuf::new(),
        full_path: PathBuf::new(),
        source_name: format!("{item_role} template name"),
    };
    let tokens = lex_source(&expression_source.contents).map_err(|source| {
        SourceKeyDirectoryMetadataError::Lex {
            source_name: expression_source.source_name.clone(),
            source,
        }
    })?;
    let (parsed, next_index) =
        parse_expression_tokens(&tokens, 0, tokens.len(), &expression_source)?;
    if next_index != tokens.len() {
        return unsupported(format!("{item_role} template name has trailing tokens"));
    }
    evaluate_source_static_expression(program, &parsed, constant_values).ok_or_else(|| {
        unsupported_source_message(format!("{item_role} template name must be static"))
    })
}

fn source_template_expression_end(template: &str, start: usize) -> Option<usize> {
    let bytes = template.as_bytes();
    let mut index = start;
    let mut quote = None;
    let mut escaped = false;
    let mut brace_depth = 0_usize;

    while index < bytes.len() {
        let byte = bytes[index];
        if let Some(delimiter) = quote {
            if escaped {
                escaped = false;
                index += 1;
                continue;
            }
            if byte == b'\\' {
                escaped = true;
                index += 1;
                continue;
            }
            if byte == delimiter {
                quote = None;
            }
            index += 1;
            continue;
        }

        match byte {
            b'"' | b'\'' | b'`' => quote = Some(byte),
            b'{' => brace_depth += 1,
            b'}' => {
                if brace_depth == 0 {
                    return Some(index);
                }
                brace_depth -= 1;
            }
            _ => {}
        }
        index += 1;
    }

    None
}

fn source_template_value_string(value: &FixedFileTemplateValue) -> String {
    match value {
        FixedFileTemplateValue::Integer(value) => value.to_string(),
        FixedFileTemplateValue::Boolean(value) => value.to_string(),
        FixedFileTemplateValue::String(value) => value.clone(),
    }
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
