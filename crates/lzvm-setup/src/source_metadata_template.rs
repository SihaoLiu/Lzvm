use std::collections::{BTreeMap, BTreeSet};

use lzvm_pil::{
    AirInstanceDeclaration, AirTemplateDeclaration, CallArgument, FixedFileTemplateValue,
    FunctionStatement, FunctionStatementKind, SourceProgram, SourceProgramModule, SourceSpan,
    Token,
};

use crate::{
    source_control_body_cache::SourceControlBodyCache,
    source_key_directory::SourceKeyDirectoryMetadataError,
    source_static_values::{
        evaluate_source_static_expression, source_declaration_constant_values_from_cache,
        source_declaration_in_static_false_branch, SourceTemplateConstantValueCache,
    },
    source_template_if::{
        source_static_if_body_span_with_tokens, source_static_if_body_statements_with_tokens,
    },
};

pub(crate) fn source_metadata_unit_instance<'a>(
    program: &'a SourceProgram,
    unit_name: Option<(&str, &str)>,
) -> Option<&'a AirInstanceDeclaration> {
    let (group_name, unit_name) = unit_name?;
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
    units
        .into_iter()
        .zip(instances)
        .find_map(|(unit, instance)| {
            (unit.group_name == group_name && unit.unit_name == unit_name).then_some(instance)
        })
}

pub(crate) fn source_metadata_template_instances<'a>(
    program: &'a SourceProgram,
    template_name: &str,
) -> Vec<&'a AirInstanceDeclaration> {
    program
        .modules
        .iter()
        .flat_map(|module| module.air_instances.iter())
        .filter(|instance| !instance.virtual_instance && instance.template == template_name)
        .collect()
}

pub(crate) fn source_metadata_declaration_template(
    module: &SourceProgramModule,
    start: usize,
    end: usize,
) -> Option<&AirTemplateDeclaration> {
    module
        .air_templates
        .iter()
        .find(|template| template.body.start <= start && end <= template.body.end)
}

pub(crate) fn source_metadata_template_values(
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
    let mut provided = BTreeSet::new();
    if let Some(arguments) = instance.args_expressions.as_ref() {
        apply_source_metadata_instance_arguments(
            program,
            template,
            arguments,
            &mut values,
            &mut provided,
        );
    }
    bind_source_metadata_template_defaults(program, template, &mut values, &provided);

    let parameter_names = template
        .parameters
        .iter()
        .map(|parameter| parameter.name.as_str())
        .collect::<BTreeSet<_>>();
    for (name, value) in cached {
        if !parameter_names.contains(name.as_str()) {
            values.insert(name.clone(), value.clone());
        }
    }

    values
}

fn bind_source_metadata_template_defaults(
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

fn apply_source_metadata_instance_arguments(
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

pub(crate) fn source_declaration_in_unselected_static_branch(
    program: &SourceProgram,
    module: &SourceProgramModule,
    tokens: &[Token],
    body_cache: &mut SourceControlBodyCache,
    start: usize,
    end: usize,
    values: &BTreeMap<String, FixedFileTemplateValue>,
) -> Result<bool, SourceKeyDirectoryMetadataError> {
    let Some(template) = source_metadata_declaration_template(module, start, end) else {
        return Ok(source_declaration_in_static_false_branch(
            program, module, start, end, values,
        ));
    };
    let mut context = SourceStaticBranchContext {
        program,
        module,
        tokens,
        body_cache,
        values,
    };
    source_declaration_unselected_in_statements(&mut context, &template.statements, start, end)
}

struct SourceStaticBranchContext<'a, 'b> {
    program: &'a SourceProgram,
    module: &'a SourceProgramModule,
    tokens: &'a [Token],
    body_cache: &'b mut SourceControlBodyCache,
    values: &'a BTreeMap<String, FixedFileTemplateValue>,
}

fn source_declaration_unselected_in_statements(
    context: &mut SourceStaticBranchContext<'_, '_>,
    statements: &[FunctionStatement],
    start: usize,
    end: usize,
) -> Result<bool, SourceKeyDirectoryMetadataError> {
    for statement in statements {
        if !(statement.start <= start && end <= statement.end) {
            continue;
        }
        if statement.kind != FunctionStatementKind::If {
            continue;
        }
        let Some(selection) = source_static_if_body_span_with_tokens(
            context.program,
            context.module,
            context.tokens,
            statement,
            context.values,
            context.body_cache,
        )?
        else {
            return Ok(false);
        };
        let Some(selected) = selection else {
            return Ok(true);
        };
        if !source_span_contains(selected, start, end) {
            return Ok(true);
        }
        let Some(body_statements) = source_static_if_body_statements_with_tokens(
            context.program,
            context.module,
            context.tokens,
            statement,
            context.values,
            context.body_cache,
        )?
        else {
            return Ok(false);
        };
        return source_declaration_unselected_in_statements(context, &body_statements, start, end);
    }
    Ok(false)
}

fn source_span_contains(span: SourceSpan, start: usize, end: usize) -> bool {
    span.start <= start && end <= span.end
}
