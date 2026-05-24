use std::collections::{BTreeMap, BTreeSet};

use lzvm_pil::{AirInstanceDeclaration, ColumnKind, FixedFileTemplateValue, SourceProgram};

use crate::{
    source_scope::{declaration_in_function_body, declaration_in_inactive_template},
    source_static_values::{
        source_declaration_constant_values_from_cache, source_declaration_in_static_false_branch,
        SourceTemplateConstantValueCache,
    },
};

pub(crate) fn source_fixed_assignment_column_names(
    program: &SourceProgram,
    active_templates: &BTreeSet<String>,
    constant_values: &BTreeMap<String, FixedFileTemplateValue>,
    template_values: &SourceTemplateConstantValueCache,
) -> BTreeSet<String> {
    program
        .modules
        .iter()
        .flat_map(|module| {
            module.columns.iter().filter(|declaration| {
                if declaration.kind != ColumnKind::Fixed
                    || declaration.initializer.is_some()
                    || declaration_in_function_body(module, declaration.start, declaration.end)
                    || declaration_in_inactive_template(
                        module,
                        declaration.start,
                        declaration.end,
                        active_templates,
                    )
                {
                    return false;
                }
                let declaration_values = source_declaration_constant_values_from_cache(
                    module,
                    declaration.start,
                    declaration.end,
                    constant_values,
                    template_values,
                );
                !source_declaration_in_static_false_branch(
                    program,
                    module,
                    declaration.start,
                    declaration.end,
                    declaration_values,
                )
            })
        })
        .flat_map(|declaration| declaration.items.iter().map(|item| item.name.clone()))
        .collect()
}

pub(crate) fn source_expression_unit_instances<'a>(
    program: &'a SourceProgram,
    group_name: Option<&str>,
    unit_name: Option<&str>,
) -> Option<Vec<&'a AirInstanceDeclaration>> {
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
    Some(
        units
            .into_iter()
            .zip(instances)
            .filter_map(|(unit, instance)| {
                if group_name.is_some_and(|group_name| unit.group_name != group_name) {
                    return None;
                }
                if unit_name.is_some_and(|unit_name| unit.unit_name != unit_name) {
                    return None;
                }
                Some(instance)
            })
            .collect(),
    )
}

pub(crate) fn source_expression_template_instances<'a>(
    instances: Option<&[&'a AirInstanceDeclaration]>,
    template_name: &str,
) -> Vec<Option<&'a AirInstanceDeclaration>> {
    let Some(instances) = instances else {
        return vec![None];
    };
    instances
        .iter()
        .copied()
        .filter(|instance| instance.template == template_name)
        .map(Some)
        .collect()
}
