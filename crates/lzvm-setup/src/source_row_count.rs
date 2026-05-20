use std::collections::BTreeMap;

use lzvm_pil::{
    evaluate_fixed_file_template_value_expression_with_values, AirInstanceDeclaration,
    AirTemplateDeclaration, CallArgument, ColumnInitializerKind, ColumnKind,
    FixedFileTemplateValue, SourceProgram,
};

use crate::source_key_directory::SourceKeyDirectoryMetadataError;

pub(crate) type SourceUnitRowCounts = BTreeMap<(usize, usize), u64>;

pub(crate) fn infer_source_row_counts(
    program: &SourceProgram,
) -> Result<SourceUnitRowCounts, SourceKeyDirectoryMetadataError> {
    let mut row_counts = infer_source_row_counts_from_air_units(program)?;
    if row_counts.is_empty() {
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
                    .ok_or_else(|| {
                        unsupported_source_message("source fixed-column span is invalid")
                    })?;
                match count_source_sequence_items(text) {
                    Ok(count) => merge_source_sequence_count(program, &mut row_counts, count)?,
                    Err(_) if !row_counts.is_empty() => {}
                    Err(error) => return Err(error),
                }
            }
        }
    }
    if row_counts.is_empty() {
        for unit in program
            .air_units()
            .into_iter()
            .filter(|unit| !unit.virtual_instance)
        {
            let group_id = usize::try_from(unit.group_id)
                .map_err(|_| unsupported_source_message("negative source group id"))?;
            let unit_id = usize::try_from(unit.unit_id)
                .map_err(|_| unsupported_source_message("negative source unit id"))?;
            row_counts.insert((group_id, unit_id), 2);
        }
    }
    Ok(row_counts)
}

fn infer_source_row_counts_from_air_units(
    program: &SourceProgram,
) -> Result<SourceUnitRowCounts, SourceKeyDirectoryMetadataError> {
    let templates = program
        .modules
        .iter()
        .flat_map(|module| module.air_templates.iter())
        .map(|template| (template.name.as_str(), template))
        .collect::<BTreeMap<_, _>>();
    let constants = source_static_constant_values(program);
    let mut row_counts = SourceUnitRowCounts::new();
    let units = program
        .air_units()
        .into_iter()
        .filter(|unit| !unit.virtual_instance)
        .collect::<Vec<_>>();
    let instances = program
        .modules
        .iter()
        .flat_map(|module| module.air_instances.iter())
        .filter(|instance| !instance.virtual_instance)
        .collect::<Vec<_>>();
    for (unit, instance) in units.into_iter().zip(instances) {
        let group_id = usize::try_from(unit.group_id)
            .map_err(|_| unsupported_source_message("negative source group id"))?;
        let unit_id = usize::try_from(unit.unit_id)
            .map_err(|_| unsupported_source_message("negative source unit id"))?;
        if unit.template_name != instance.template {
            return unsupported("source air unit order mismatch");
        }
        let Some(template) = templates.get(instance.template.as_str()).copied() else {
            continue;
        };
        let values = source_air_instance_parameter_values(template, instance, &constants);
        let Some(FixedFileTemplateValue::Integer(value)) = values.get("N") else {
            continue;
        };
        let value = u64::try_from(*value)
            .map_err(|_| unsupported_source_message("source row count is out of range"))?;
        validate_source_row_count(value)?;
        row_counts.insert((group_id, unit_id), value);
    }
    Ok(row_counts)
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

fn merge_source_sequence_count(
    program: &SourceProgram,
    row_counts: &mut SourceUnitRowCounts,
    value: u64,
) -> Result<(), SourceKeyDirectoryMetadataError> {
    validate_source_row_count(value)?;
    if row_counts.is_empty() {
        for unit in program
            .air_units()
            .into_iter()
            .filter(|unit| !unit.virtual_instance)
        {
            let group_id = usize::try_from(unit.group_id)
                .map_err(|_| unsupported_source_message("negative source group id"))?;
            let unit_id = usize::try_from(unit.unit_id)
                .map_err(|_| unsupported_source_message("negative source unit id"))?;
            row_counts.insert((group_id, unit_id), value);
        }
        return Ok(());
    }
    for row_count in row_counts.values().copied() {
        if row_count != value {
            return unsupported("source row counts must match");
        }
    }
    Ok(())
}

fn validate_source_row_count(value: u64) -> Result<(), SourceKeyDirectoryMetadataError> {
    if value == 0 || !value.is_power_of_two() {
        return unsupported("source row count must be a power of two");
    }
    Ok(())
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
        let (start, start_repeat) = parse_range_endpoint_count(item[..index].trim())?;
        let (end, end_repeat) = parse_range_endpoint_count(item[index + 2..].trim())?;
        if start_repeat != end_repeat {
            return unsupported("source fixed-column range repeat counts must match");
        }
        let range_length = if start <= end {
            end.checked_sub(start)
        } else {
            start.checked_sub(end)
        }
        .and_then(|value| value.checked_add(1))
        .ok_or_else(|| unsupported_source_message("source range length overflow"))?;
        let range_length = u64::try_from(range_length)
            .map_err(|_| unsupported_source_message("source range length overflow"))?;
        range_length
            .checked_mul(start_repeat)
            .ok_or_else(|| unsupported_source_message("source range length overflow"))
    } else {
        parse_sequence_repeat_count(item)
    }
}

fn parse_sequence_repeat_count(value: &str) -> Result<u64, SourceKeyDirectoryMetadataError> {
    let Some((_, repeat)) = value.split_once(':') else {
        return Ok(1);
    };
    parse_u64_literal(repeat.trim())
}

fn parse_range_endpoint_count(value: &str) -> Result<(i128, u64), SourceKeyDirectoryMetadataError> {
    let Some((value, repeat)) = value.split_once(':') else {
        return Ok((parse_i128_literal(value)?, 1));
    };
    let value = parse_i128_literal(value.trim())?;
    let repeat = parse_u64_literal(repeat.trim())?;
    Ok((value, repeat))
}

fn parse_u64_literal(value: &str) -> Result<u64, SourceKeyDirectoryMetadataError> {
    let value = parse_i128_literal(value)?;
    u64::try_from(value).map_err(|_| unsupported_source_message("source literal must be unsigned"))
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
