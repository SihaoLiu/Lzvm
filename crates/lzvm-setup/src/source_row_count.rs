use std::collections::BTreeMap;
use std::path::PathBuf;

use lzvm_pil::{
    evaluate_fixed_file_template_value_expression_with_values, lex_source, parse_expression,
    AirInstanceDeclaration, AirTemplateDeclaration, CallArgument, ColumnInitializerKind,
    ColumnKind, FixedFileTemplateValue, SourceFile, SourceProgram,
};

use crate::source_key_directory::SourceKeyDirectoryMetadataError;

pub(crate) type SourceUnitRowCounts = BTreeMap<(usize, usize), u64>;

pub(crate) fn infer_source_row_counts(
    program: &SourceProgram,
) -> Result<SourceUnitRowCounts, SourceKeyDirectoryMetadataError> {
    let mut row_counts = infer_source_row_counts_from_air_units(program)?;
    if row_counts.is_empty() {
        let constants = source_static_constant_values(program);
        let mut first_sequence_error = None;
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
                match count_source_sequence_items(text, &constants) {
                    Ok(count) => merge_source_sequence_count(program, &mut row_counts, count)?,
                    Err(error) => {
                        first_sequence_error.get_or_insert(error);
                    }
                }
            }
        }
        if row_counts.is_empty() {
            if let Some(error) = first_sequence_error {
                return Err(error);
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

fn count_source_sequence_items(
    text: &str,
    constants: &BTreeMap<String, FixedFileTemplateValue>,
) -> Result<u64, SourceKeyDirectoryMetadataError> {
    count_source_sequence_items_with_last(text, constants).map(|count| count.len)
}

fn count_source_sequence_items_with_last(
    text: &str,
    constants: &BTreeMap<String, FixedFileTemplateValue>,
) -> Result<SequenceItemCount, SourceKeyDirectoryMetadataError> {
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
    let mut previous_value = None;
    let mut value_before_previous = None;
    let mut pending_comma_progression = None::<SequenceProgressionKind>;
    for item in split_top_level_commas(inner) {
        let item = item.trim();
        if item.is_empty() {
            continue;
        }
        if let Some(kind) = comma_progression_marker(item) {
            if pending_comma_progression.is_some() {
                return unsupported("source fixed-column progression needs explicit metadata");
            }
            if value_before_previous.is_none() || previous_value.is_none() {
                return unsupported("source fixed-column progression needs explicit metadata");
            }
            pending_comma_progression = Some(kind);
            continue;
        }
        let item_count = if let Some(kind) = pending_comma_progression.take() {
            sequence_comma_progression_count(
                item,
                kind,
                constants,
                value_before_previous,
                previous_value,
            )?
        } else {
            sequence_item_count(item, constants, previous_value)?
        };
        count = count
            .checked_add(item_count.len)
            .ok_or_else(|| unsupported_source_message("source sequence length overflow"))?;
        value_before_previous = if item_count.len == 1 {
            previous_value
        } else {
            None
        };
        previous_value = item_count.last_value;
    }
    if pending_comma_progression.is_some() {
        return unsupported("source fixed-column progression needs explicit metadata");
    }
    Ok(SequenceItemCount {
        len: count,
        last_value: previous_value,
    })
}

#[derive(Debug, Clone, Copy)]
struct SequenceItemCount {
    len: u64,
    last_value: Option<i128>,
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

fn sequence_item_count(
    item: &str,
    constants: &BTreeMap<String, FixedFileTemplateValue>,
    previous_value: Option<i128>,
) -> Result<SequenceItemCount, SourceKeyDirectoryMetadataError> {
    if let Some((index, kind)) = top_level_progression_index(item) {
        sequence_progression_count(item, index, kind, constants, previous_value)
    } else if let Some(index) = top_level_range_index(item) {
        if item[index..].starts_with("...") || item[index + 2..].contains("..") {
            return unsupported("source fixed-column sequence ranges must be finite");
        }
        let (start, start_repeat) = parse_range_endpoint_count(item[..index].trim(), constants)?;
        let (end, end_repeat) = parse_range_endpoint_count(item[index + 2..].trim(), constants)?;
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
        let len = range_length
            .checked_mul(start_repeat)
            .ok_or_else(|| unsupported_source_message("source range length overflow"))?;
        Ok(SequenceItemCount {
            len,
            last_value: Some(end),
        })
    } else if let Some(index) = top_level_delimiter_index(item, ':') {
        let value_count = sequence_repeat_value_count(item[..index].trim(), constants)?;
        let repeat = parse_u64_static_integer(item[index + 1..].trim(), constants)?;
        let len = value_count
            .len
            .checked_mul(repeat)
            .ok_or_else(|| unsupported_source_message("source sequence length overflow"))?;
        Ok(SequenceItemCount {
            len,
            last_value: if repeat == 0 {
                None
            } else {
                value_count.last_value
            },
        })
    } else {
        Ok(SequenceItemCount {
            len: 1,
            last_value: parse_i128_static_integer(item, constants).ok(),
        })
    }
}

fn sequence_repeat_value_count(
    value: &str,
    constants: &BTreeMap<String, FixedFileTemplateValue>,
) -> Result<SequenceItemCount, SourceKeyDirectoryMetadataError> {
    if value.starts_with('[') && value.ends_with(']') {
        count_source_sequence_items_with_last(value, constants)
    } else {
        Ok(SequenceItemCount {
            len: 1,
            last_value: parse_i128_static_integer(value, constants).ok(),
        })
    }
}

fn sequence_progression_count(
    item: &str,
    progression_index: usize,
    kind: SequenceProgressionKind,
    constants: &BTreeMap<String, FixedFileTemplateValue>,
    previous_value: Option<i128>,
) -> Result<SequenceItemCount, SourceKeyDirectoryMetadataError> {
    let previous = previous_value.ok_or_else(|| {
        unsupported_source_message("source fixed-column progression needs explicit metadata")
    })?;
    let current = parse_i128_static_integer(item[..progression_index].trim(), constants)?;
    let last_start = progression_index + kind.marker_len();
    let last = item.get(last_start..).unwrap_or_default().trim();
    if last.is_empty() {
        return unsupported("source fixed-column progression needs explicit metadata");
    }
    let last = parse_i128_static_integer(last, constants)?;
    let len = match kind {
        SequenceProgressionKind::Add => {
            let step = current
                .checked_sub(previous)
                .ok_or_else(|| unsupported_source_message("source progression length overflow"))?;
            sequence_add_progression_len(current, last, step)?
        }
        SequenceProgressionKind::Mul => {
            sequence_mul_or_div_progression_len(previous, current, last)?
        }
    };
    Ok(SequenceItemCount {
        len,
        last_value: Some(last),
    })
}

fn comma_progression_marker(item: &str) -> Option<SequenceProgressionKind> {
    match item {
        "..+.." => Some(SequenceProgressionKind::Add),
        "..*.." => Some(SequenceProgressionKind::Mul),
        _ => None,
    }
}

fn sequence_comma_progression_count(
    last: &str,
    kind: SequenceProgressionKind,
    constants: &BTreeMap<String, FixedFileTemplateValue>,
    previous_value: Option<i128>,
    current_value: Option<i128>,
) -> Result<SequenceItemCount, SourceKeyDirectoryMetadataError> {
    let previous = previous_value.ok_or_else(|| {
        unsupported_source_message("source fixed-column progression needs explicit metadata")
    })?;
    let current = current_value.ok_or_else(|| {
        unsupported_source_message("source fixed-column progression needs explicit metadata")
    })?;
    let last = parse_i128_static_integer(last, constants)?;
    let inclusive_len = match kind {
        SequenceProgressionKind::Add => {
            let step = current
                .checked_sub(previous)
                .ok_or_else(|| unsupported_source_message("source progression length overflow"))?;
            sequence_add_progression_len(current, last, step)?
        }
        SequenceProgressionKind::Mul => {
            sequence_mul_or_div_progression_len(previous, current, last)?
        }
    };
    Ok(SequenceItemCount {
        len: inclusive_len.saturating_sub(1),
        last_value: Some(last),
    })
}

fn sequence_add_progression_len(
    current: i128,
    last: i128,
    step: i128,
) -> Result<u64, SourceKeyDirectoryMetadataError> {
    if (step > 0 && current > last)
        || (step < 0 && current < last)
        || (step == 0 && current != last)
    {
        return unsupported("source fixed-column progression needs explicit metadata");
    }
    let mut value = current;
    let mut len = 0_u64;
    loop {
        len = len
            .checked_add(1)
            .ok_or_else(|| unsupported_source_message("source progression length overflow"))?;
        if value == last {
            break;
        }
        value = value
            .checked_add(step)
            .ok_or_else(|| unsupported_source_message("source progression length overflow"))?;
        if (step > 0 && value > last) || (step < 0 && value < last) {
            return unsupported("source fixed-column progression needs explicit metadata");
        }
    }
    Ok(len)
}

fn sequence_mul_or_div_progression_len(
    previous: i128,
    current: i128,
    last: i128,
) -> Result<u64, SourceKeyDirectoryMetadataError> {
    if previous > 0 && current >= previous && current % previous == 0 {
        return sequence_mul_progression_len(current, last, current / previous);
    }
    if current > 0 && previous > current && previous % current == 0 {
        return sequence_div_progression_len(current, last, previous / current);
    }
    unsupported("source fixed-column progression needs explicit metadata")
}

fn sequence_mul_progression_len(
    current: i128,
    last: i128,
    factor: i128,
) -> Result<u64, SourceKeyDirectoryMetadataError> {
    if factor <= 1 && current != last {
        return unsupported("source fixed-column progression needs explicit metadata");
    }
    let mut value = current;
    let mut len = 0_u64;
    loop {
        len = len
            .checked_add(1)
            .ok_or_else(|| unsupported_source_message("source progression length overflow"))?;
        if value == last {
            break;
        }
        value = value
            .checked_mul(factor)
            .ok_or_else(|| unsupported_source_message("source progression length overflow"))?;
        if value > last {
            return unsupported("source fixed-column progression needs explicit metadata");
        }
    }
    Ok(len)
}

fn sequence_div_progression_len(
    current: i128,
    last: i128,
    divisor: i128,
) -> Result<u64, SourceKeyDirectoryMetadataError> {
    if divisor <= 1 && current != last {
        return unsupported("source fixed-column progression needs explicit metadata");
    }
    let mut value = current;
    let mut len = 0_u64;
    loop {
        len = len
            .checked_add(1)
            .ok_or_else(|| unsupported_source_message("source progression length overflow"))?;
        if value == last {
            break;
        }
        if value % divisor != 0 {
            return unsupported("source fixed-column progression needs explicit metadata");
        }
        value /= divisor;
        if value < last {
            return unsupported("source fixed-column progression needs explicit metadata");
        }
    }
    Ok(len)
}

fn parse_range_endpoint_count(
    value: &str,
    constants: &BTreeMap<String, FixedFileTemplateValue>,
) -> Result<(i128, u64), SourceKeyDirectoryMetadataError> {
    let Some((value, repeat)) = value.split_once(':') else {
        return Ok((parse_i128_static_integer(value, constants)?, 1));
    };
    let value = parse_i128_static_integer(value.trim(), constants)?;
    let repeat = parse_u64_static_integer(repeat.trim(), constants)?;
    Ok((value, repeat))
}

#[derive(Debug, Clone, Copy)]
enum SequenceProgressionKind {
    Add,
    Mul,
}

impl SequenceProgressionKind {
    fn marker_len(self) -> usize {
        match self {
            Self::Add | Self::Mul => 5,
        }
    }
}

fn top_level_progression_index(value: &str) -> Option<(usize, SequenceProgressionKind)> {
    let mut cursor = 0;
    while let Some(index) = top_level_delimiter_index(&value[cursor..], '.') {
        let index = cursor + index;
        if value[index..].starts_with("..+..") {
            return Some((index, SequenceProgressionKind::Add));
        }
        if value[index..].starts_with("..*..") {
            return Some((index, SequenceProgressionKind::Mul));
        }
        cursor = index + 1;
    }
    None
}

fn top_level_range_index(value: &str) -> Option<usize> {
    let mut cursor = 0;
    while let Some(index) = top_level_delimiter_index(&value[cursor..], '.') {
        let index = cursor + index;
        if value[index..].starts_with("..") {
            return Some(index);
        }
        cursor = index + 1;
    }
    None
}

fn top_level_delimiter_index(value: &str, delimiter: char) -> Option<usize> {
    let mut depth = 0_i32;
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
            _ if character == delimiter && depth == 0 => return Some(index),
            _ => {}
        }
    }
    None
}

fn parse_u64_static_integer(
    value: &str,
    constants: &BTreeMap<String, FixedFileTemplateValue>,
) -> Result<u64, SourceKeyDirectoryMetadataError> {
    let value = parse_i128_static_integer(value, constants)?;
    u64::try_from(value).map_err(|_| unsupported_source_message("source literal must be unsigned"))
}

fn parse_i128_static_integer(
    value: &str,
    constants: &BTreeMap<String, FixedFileTemplateValue>,
) -> Result<i128, SourceKeyDirectoryMetadataError> {
    if let Ok(value) = parse_i128_literal(value) {
        return Ok(value);
    }
    let source = SourceFile {
        contents: value.to_owned(),
        file_dir: PathBuf::new(),
        full_path: PathBuf::from("<source-row-count>"),
        source_name: "<source-row-count>".to_owned(),
    };
    let token_count = lex_source(value)
        .map_err(|_| unsupported_source_message("source literal must be an integer"))?
        .len();
    let (expression, next_index) = parse_expression(&source, 0, token_count)
        .map_err(|_| unsupported_source_message("source literal must be an integer"))?;
    if next_index != token_count {
        return Err(unsupported_source_message(
            "source literal must be an integer",
        ));
    }
    match evaluate_fixed_file_template_value_expression_with_values(&expression, constants) {
        Some(FixedFileTemplateValue::Integer(value)) => Ok(value),
        _ => Err(unsupported_source_message(
            "source literal must be an integer",
        )),
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
