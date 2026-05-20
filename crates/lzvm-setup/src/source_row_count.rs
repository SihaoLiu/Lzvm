use std::collections::BTreeMap;

use lzvm_pil::{
    evaluate_fixed_file_template_value_expression_with_values, AirInstanceDeclaration,
    AirTemplateDeclaration, CallArgument, ColumnInitializerKind, ColumnKind,
    FixedFileTemplateValue, SourceProgram,
};

use crate::source_key_directory::SourceKeyDirectoryMetadataError;

pub(crate) fn infer_source_row_count(
    program: &SourceProgram,
) -> Result<u64, SourceKeyDirectoryMetadataError> {
    let mut row_count = infer_source_row_count_from_air_units(program)?;
    for module in &program.modules {
        for declaration in &module.columns {
            if declaration.kind != ColumnKind::Fixed {
                continue;
            }
            let Some(initializer) = declaration.initializer.as_ref() else {
                continue;
            };
            if initializer.kind != ColumnInitializerKind::Sequence {
                continue;
            }
            let text = module
                .source
                .contents
                .get(initializer.span.start..initializer.span.end)
                .ok_or_else(|| unsupported_source_message("source fixed-column span is invalid"))?;
            match count_source_sequence_items(text) {
                Ok(count) => merge_source_row_count(&mut row_count, count)?,
                Err(_) if row_count.is_some() => {}
                Err(error) => return Err(error),
            }
        }
    }
    Ok(row_count.unwrap_or(2))
}

fn infer_source_row_count_from_air_units(
    program: &SourceProgram,
) -> Result<Option<u64>, SourceKeyDirectoryMetadataError> {
    let templates = program
        .modules
        .iter()
        .flat_map(|module| module.air_templates.iter())
        .map(|template| (template.name.as_str(), template))
        .collect::<BTreeMap<_, _>>();
    let constants = source_static_constant_values(program);
    let mut row_count = None;
    for instance in program
        .modules
        .iter()
        .flat_map(|module| module.air_instances.iter())
        .filter(|instance| !instance.virtual_instance)
    {
        let Some(template) = templates.get(instance.template.as_str()).copied() else {
            continue;
        };
        let values = source_air_instance_parameter_values(template, instance, &constants);
        let Some(FixedFileTemplateValue::Integer(value)) = values.get("N") else {
            continue;
        };
        let value = u64::try_from(*value)
            .map_err(|_| unsupported_source_message("source row count is out of range"))?;
        merge_source_row_count(&mut row_count, value)?;
    }
    Ok(row_count)
}

fn source_air_instance_parameter_values(
    template: &AirTemplateDeclaration,
    instance: &AirInstanceDeclaration,
    constants: &BTreeMap<String, FixedFileTemplateValue>,
) -> BTreeMap<String, FixedFileTemplateValue> {
    let mut values = constants.clone();
    for parameter in &template.parameters {
        if let Some(value) = parameter
            .default_expression
            .as_ref()
            .and_then(|expression| {
                evaluate_fixed_file_template_value_expression_with_values(expression, &values)
            })
        {
            values.insert(parameter.name.clone(), value);
        }
    }
    if let Some(arguments) = instance.args_expressions.as_ref() {
        apply_source_air_instance_arguments(template, arguments, &mut values);
    }
    values
}

fn apply_source_air_instance_arguments(
    template: &AirTemplateDeclaration,
    arguments: &[CallArgument],
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
) {
    let mut positional_index = 0;
    for argument in arguments {
        let Some(value) =
            evaluate_fixed_file_template_value_expression_with_values(&argument.value, values)
        else {
            continue;
        };
        if let Some(name) = argument.name.as_ref() {
            values.insert(name.clone(), value);
            continue;
        }
        if let Some(parameter) = template.parameters.get(positional_index) {
            values.insert(parameter.name.clone(), value);
        }
        positional_index += 1;
    }
}

fn source_static_constant_values(
    program: &SourceProgram,
) -> BTreeMap<String, FixedFileTemplateValue> {
    let mut values = BTreeMap::new();
    let declarations = program
        .modules
        .iter()
        .flat_map(|module| module.constants.iter())
        .collect::<Vec<_>>();
    let mut resolved = vec![false; declarations.len()];

    loop {
        let mut progressed = false;
        for (index, declaration) in declarations.iter().enumerate() {
            if resolved[index] {
                continue;
            }
            if !declaration.array_dims.is_empty() || values.contains_key(&declaration.name) {
                resolved[index] = true;
                progressed = true;
                continue;
            }
            let Some(expression) = declaration.initializer_expression.as_ref() else {
                continue;
            };
            let Some(value) =
                evaluate_fixed_file_template_value_expression_with_values(expression, &values)
            else {
                continue;
            };
            values.insert(declaration.name.clone(), value);
            resolved[index] = true;
            progressed = true;
        }
        if !progressed {
            break;
        }
    }

    values
}

fn merge_source_row_count(
    row_count: &mut Option<u64>,
    value: u64,
) -> Result<(), SourceKeyDirectoryMetadataError> {
    if value == 0 || !value.is_power_of_two() {
        return unsupported("source row count must be a power of two");
    }
    match *row_count {
        Some(expected) if expected != value => unsupported("source row counts must match"),
        Some(_) => Ok(()),
        None => {
            *row_count = Some(value);
            Ok(())
        }
    }
}

fn count_source_sequence_items(text: &str) -> Result<u64, SourceKeyDirectoryMetadataError> {
    let text = text.trim();
    if text.ends_with("...") {
        return unsupported("source fixed-column fill sequences need explicit metadata");
    }
    let inner = text
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .ok_or_else(|| {
            unsupported_source_message("source fixed-column sequence must use brackets")
        })?;
    let mut count = 0_u64;
    for item in split_top_level_commas(inner) {
        let item = item.trim();
        if item.is_empty() {
            continue;
        }
        count = count
            .checked_add(sequence_item_len(item)?)
            .ok_or_else(|| unsupported_source_message("source sequence length overflow"))?;
    }
    Ok(count)
}

fn split_top_level_commas(value: &str) -> Vec<&str> {
    let mut items = Vec::new();
    let mut depth = 0_i32;
    let mut start = 0;
    let mut quote = None::<char>;
    let mut escaped = false;

    for (index, character) in value.char_indices() {
        if let Some(active_quote) = quote {
            if escaped {
                escaped = false;
                continue;
            }
            if character == '\\' {
                escaped = true;
                continue;
            }
            if character == active_quote {
                quote = None;
            }
            continue;
        }

        match character {
            '"' | '\'' | '`' => quote = Some(character),
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            ',' if depth == 0 => {
                items.push(&value[start..index]);
                start = index + character.len_utf8();
            }
            _ => {}
        }
    }
    items.push(&value[start..]);
    items
}

fn sequence_item_len(item: &str) -> Result<u64, SourceKeyDirectoryMetadataError> {
    if let Some(index) = item.find("..") {
        if item[index..].starts_with("...") || item[index + 2..].contains("..") {
            return unsupported("source fixed-column sequence ranges must be finite");
        }
        let start = parse_i128_literal(item[..index].trim())?;
        let end = parse_i128_literal(item[index + 2..].trim())?;
        if end < start {
            return unsupported("source fixed-column descending ranges need explicit metadata");
        }
        let length = end
            .checked_sub(start)
            .and_then(|value| value.checked_add(1))
            .ok_or_else(|| unsupported_source_message("source range length overflow"))?;
        u64::try_from(length)
            .map_err(|_| unsupported_source_message("source range length overflow"))
    } else {
        Ok(1)
    }
}

fn parse_i128_literal(value: &str) -> Result<i128, SourceKeyDirectoryMetadataError> {
    if let Some(hex) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        return i128::from_str_radix(hex, 16)
            .map_err(|_| unsupported_source_message("source literal must be an integer"));
    }
    value
        .parse::<i128>()
        .map_err(|_| unsupported_source_message("source literal must be an integer"))
}

fn unsupported_source_message(message: impl Into<String>) -> SourceKeyDirectoryMetadataError {
    SourceKeyDirectoryMetadataError::UnsupportedSourceProgram {
        message: message.into(),
    }
}

fn unsupported<T>(message: impl Into<String>) -> Result<T, SourceKeyDirectoryMetadataError> {
    Err(unsupported_source_message(message))
}
