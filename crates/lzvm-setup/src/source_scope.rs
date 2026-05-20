use std::collections::BTreeSet;

use lzvm_pil::{IncludeKind, SourceProgram, SourceProgramModule};

pub(crate) fn global_constraint_source_names(program: &SourceProgram) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    if let Some(source) = program.graph.sources.first() {
        names.insert(source.source_name.clone());
    }
    for edge in &program.graph.edges {
        if edge.kind == IncludeKind::Require {
            names.insert(edge.to.clone());
        }
    }
    names
}

pub(crate) fn concrete_template_names(program: &SourceProgram) -> BTreeSet<String> {
    program
        .air_units()
        .into_iter()
        .filter(|unit| !unit.virtual_instance)
        .map(|unit| unit.template_name)
        .collect()
}

pub(crate) fn declaration_in_inactive_template(
    module: &SourceProgramModule,
    start: usize,
    end: usize,
    active_templates: &BTreeSet<String>,
) -> bool {
    module
        .air_templates
        .iter()
        .find(|template| template.body.start <= start && end <= template.body.end)
        .is_some_and(|template| !active_templates.contains(&template.name))
}

pub(crate) fn declaration_in_function_body(
    module: &SourceProgramModule,
    start: usize,
    end: usize,
) -> bool {
    module
        .functions
        .iter()
        .any(|function| function.body.start <= start && end <= function.body.end)
}
