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

pub(crate) fn source_expression_unit_instance<'a>(
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
