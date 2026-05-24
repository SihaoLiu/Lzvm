use std::collections::BTreeMap;

use lzvm_pil::{FixedFileTemplateValue, SourceProgram, SourceProgramModule};

use crate::{
    source_scope::declaration_in_function_body,
    source_static_values::evaluate_source_static_expression,
};

pub(crate) fn insert_source_scalar_variable_values(
    program: &SourceProgram,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
) {
    let variables = program
        .modules
        .iter()
        .flat_map(|module| {
            module.variables.iter().filter(move |declaration| {
                !declaration_in_function_body(module, declaration.start, declaration.end)
                    && !declaration_in_template(module, declaration.start, declaration.end)
            })
        })
        .collect::<Vec<_>>();
    let mut resolved_variables = vec![false; variables.len()];
    loop {
        let mut progressed = false;
        for (index, declaration) in variables.iter().enumerate() {
            if resolved_variables[index] {
                continue;
            }
            if !declaration.array_dims.is_empty() {
                continue;
            }
            if values.contains_key(&declaration.name) {
                resolved_variables[index] = true;
                progressed = true;
                continue;
            }
            let Some(expression) = declaration.initializer_expression.as_ref() else {
                continue;
            };
            let Some(value) = evaluate_source_static_expression(program, expression, values) else {
                continue;
            };
            values.insert(declaration.name.clone(), value);
            resolved_variables[index] = true;
            progressed = true;
        }
        if !progressed {
            break;
        }
    }
}

fn declaration_in_template(module: &SourceProgramModule, start: usize, end: usize) -> bool {
    module
        .air_templates
        .iter()
        .any(|template| template.body.start <= start && end <= template.body.end)
}
