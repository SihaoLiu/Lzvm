use std::collections::{BTreeMap, BTreeSet};

use lzvm_pil::{
    AirInstanceDeclaration, AirTemplateDeclaration, CallArgument, FixedFileTemplateValue,
    SourceProgram, SourceProgramModule,
};

use crate::source_static_values::{
    evaluate_source_static_expression, execute_static_template_range,
    source_declaration_constant_values, source_declaration_constant_values_from_cache,
    SourceTemplateConstantValueCache,
};

pub(crate) fn source_expression_template_values(
    program: &SourceProgram,
    module: &SourceProgramModule,
    template: &AirTemplateDeclaration,
    instance: Option<&AirInstanceDeclaration>,
    base_values: &BTreeMap<String, FixedFileTemplateValue>,
    template_values: &SourceTemplateConstantValueCache,
) -> BTreeMap<String, FixedFileTemplateValue> {
    let cached = source_declaration_constant_values_from_cache(
        module,
        template.body.start,
        template.body.end,
        base_values,
        template_values,
    );
    let Some(instance) = instance else {
        return cached.clone();
    };
    let mut values = base_values.clone();
    apply_source_expression_airgroup_static_values(program, module, instance, &mut values);
    let mut provided = BTreeSet::new();
    if let Some(arguments) = instance.args_expressions.as_ref() {
        apply_source_expression_instance_arguments(
            program,
            template,
            arguments,
            &mut values,
            &mut provided,
        );
    }
    bind_source_expression_template_defaults(program, template, &mut values, &provided);

    source_declaration_constant_values(
        program,
        module,
        template.body.end,
        template.body.end,
        &values,
    )
}

fn apply_source_expression_airgroup_static_values(
    program: &SourceProgram,
    module: &SourceProgramModule,
    instance: &AirInstanceDeclaration,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
) {
    let Some(group) = module
        .air_groups
        .iter()
        .find(|group| group.name == instance.air_group)
    else {
        return;
    };
    let _ =
        execute_static_template_range(program, module, group.body.start, instance.start, values);
}

fn bind_source_expression_template_defaults(
    program: &SourceProgram,
    template: &AirTemplateDeclaration,
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
    provided: &BTreeSet<String>,
) {
    for parameter in &template.parameters {
        if provided.contains(&parameter.name) {
            continue;
        }
        let Some(value) = parameter
            .default_expression
            .as_ref()
            .and_then(|expression| evaluate_source_static_expression(program, expression, values))
        else {
            values.remove(&parameter.name);
            continue;
        };
        values.insert(parameter.name.clone(), value);
    }
}

fn apply_source_expression_instance_arguments(
    program: &SourceProgram,
    template: &AirTemplateDeclaration,
    arguments: &[CallArgument],
    values: &mut BTreeMap<String, FixedFileTemplateValue>,
    provided: &mut BTreeSet<String>,
) {
    let mut positional_index = 0;
    for argument in arguments {
        let Some(value) = evaluate_source_static_expression(program, &argument.value, values)
        else {
            continue;
        };
        let name = if let Some(name) = argument.name.as_ref() {
            name
        } else {
            while template
                .parameters
                .get(positional_index)
                .is_some_and(|parameter| provided.contains(&parameter.name))
            {
                let Some(next) = positional_index.checked_add(1) else {
                    return;
                };
                positional_index = next;
            }
            let Some(parameter) = template.parameters.get(positional_index) else {
                continue;
            };
            &parameter.name
        };
        if provided.insert(name.clone()) {
            values.insert(name.clone(), value);
        }
        if argument.name.is_none() {
            let Some(next) = positional_index.checked_add(1) else {
                return;
            };
            positional_index = next;
        }
    }
}
