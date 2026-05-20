use std::collections::BTreeMap;

use lzvm_pil::{
    evaluate_fixed_file_template_value_expression_with_values, FixedFileTemplateValue,
    SourceProgram, SourceProgramModule,
};

pub(crate) fn source_scalar_constant_values(
    program: &SourceProgram,
    row_count: u64,
) -> BTreeMap<String, FixedFileTemplateValue> {
    let mut values = BTreeMap::from([
        (
            "BITS".to_owned(),
            FixedFileTemplateValue::Integer(i128::from(row_count.trailing_zeros())),
        ),
        (
            "N".to_owned(),
            FixedFileTemplateValue::Integer(i128::from(row_count)),
        ),
    ]);
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

pub(crate) fn source_declaration_constant_values(
    module: &SourceProgramModule,
    start: usize,
    end: usize,
    base_values: &BTreeMap<String, FixedFileTemplateValue>,
) -> BTreeMap<String, FixedFileTemplateValue> {
    let mut values = base_values.clone();
    let Some(template) = module
        .air_templates
        .iter()
        .find(|template| template.body.start <= start && end <= template.body.end)
    else {
        return values;
    };

    for parameter in &template.parameters {
        if values.contains_key(&parameter.name) {
            continue;
        }
        let Some(expression) = parameter.default_expression.as_ref() else {
            continue;
        };
        let Some(value) =
            evaluate_fixed_file_template_value_expression_with_values(expression, &values)
        else {
            continue;
        };
        values.insert(parameter.name.clone(), value);
    }

    for declaration in &module.constants {
        if declaration.start < template.body.start || declaration.end > template.body.end {
            continue;
        }
        if declaration.end > start
            || !declaration.array_dims.is_empty()
            || values.contains_key(&declaration.name)
        {
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
    }

    values
}
