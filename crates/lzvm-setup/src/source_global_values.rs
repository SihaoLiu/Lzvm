use std::collections::{BTreeMap, BTreeSet};

use lzvm_artifacts::global_info::{NamedStageValue, PublicValue};
use lzvm_pil::{
    lex_source, AirTemplateDeclaration, FixedFileTemplateValue, PublicDeclaration, SourceProgram,
    SourceProgramModule, Token, ValueDeclaration, ValueDeclarationKind,
};

use crate::{
    source_control_body_cache::{SourceControlBodyCache, SourceControlBodyCaches},
    source_key_directory::{
        source_column_dimension, source_item_lengths, source_item_name, unsupported,
        unsupported_source_message, SourceKeyDirectoryMetadataError,
    },
    source_metadata_template::{
        source_declaration_in_unselected_static_branch, source_metadata_declaration_template,
        source_metadata_template_instances, source_metadata_template_values,
    },
    source_scalar_slots::SourceChallengeSlotMetadata,
    source_scope::{declaration_in_function_body, declaration_in_inactive_template},
    source_static_values::{
        source_declaration_constant_values_from_cache, source_declaration_in_static_false_branch,
        SourceTemplateConstantValueCache,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SourceChallengeShape {
    stage: usize,
    dimension: u32,
}

pub(crate) fn source_public_values(
    program: &SourceProgram,
    constant_values: &BTreeMap<String, FixedFileTemplateValue>,
    active_templates: &BTreeSet<String>,
    template_values: &SourceTemplateConstantValueCache,
    body_caches: &mut SourceControlBodyCaches,
) -> Result<Vec<PublicValue>, SourceKeyDirectoryMetadataError> {
    let mut seen = BTreeSet::new();
    let mut values = Vec::new();
    for module in &program.modules {
        let tokens = lex_source(&module.source.contents).map_err(|source| {
            SourceKeyDirectoryMetadataError::Lex {
                source_name: module.source_name.clone(),
                source,
            }
        })?;
        let body_cache = body_caches.module_cache(&module.source_name);
        let mut template_context = SourcePublicTemplateContext {
            program,
            module,
            tokens: &tokens,
            body_cache,
            constant_values,
            template_values,
        };
        for declaration in &module.publics {
            if declaration_in_function_body(module, declaration.start, declaration.end)
                || declaration_in_inactive_template(
                    module,
                    declaration.start,
                    declaration.end,
                    active_templates,
                )
            {
                continue;
            }
            let Some(declaration_template) =
                source_metadata_declaration_template(module, declaration.start, declaration.end)
            else {
                let declaration_values = source_declaration_constant_values_from_cache(
                    module,
                    declaration.start,
                    declaration.end,
                    constant_values,
                    template_values,
                );
                if source_declaration_in_static_false_branch(
                    program,
                    module,
                    declaration.start,
                    declaration.end,
                    declaration_values,
                ) {
                    continue;
                }
                source_push_public_values(
                    program,
                    declaration,
                    declaration_values,
                    &mut seen,
                    &mut values,
                )?;
                continue;
            };
            let Some(declaration_values) = source_public_values_for_any_instance(
                &mut template_context,
                declaration,
                declaration_template,
            )?
            else {
                continue;
            };
            source_push_public_values(
                program,
                declaration,
                &declaration_values,
                &mut seen,
                &mut values,
            )?;
        }
    }
    Ok(values)
}

struct SourcePublicTemplateContext<'a, 'b> {
    program: &'a SourceProgram,
    module: &'a SourceProgramModule,
    tokens: &'a [Token],
    body_cache: &'b mut SourceControlBodyCache,
    constant_values: &'a BTreeMap<String, FixedFileTemplateValue>,
    template_values: &'a SourceTemplateConstantValueCache,
}

fn source_public_values_for_any_instance(
    context: &mut SourcePublicTemplateContext<'_, '_>,
    declaration: &PublicDeclaration,
    declaration_template: &AirTemplateDeclaration,
) -> Result<Option<BTreeMap<String, FixedFileTemplateValue>>, SourceKeyDirectoryMetadataError> {
    for instance in source_metadata_template_instances(context.program, &declaration_template.name)
    {
        let values = source_metadata_template_values(
            context.program,
            context.module,
            declaration_template,
            Some(instance),
            context.constant_values,
            context.template_values,
        );
        if source_declaration_in_unselected_static_branch(
            context.program,
            context.module,
            context.tokens,
            context.body_cache,
            declaration.start,
            declaration.end,
            &values,
        )? {
            continue;
        }
        return Ok(Some(values));
    }
    Ok(None)
}

fn source_push_public_values(
    program: &SourceProgram,
    declaration: &PublicDeclaration,
    declaration_values: &BTreeMap<String, FixedFileTemplateValue>,
    seen: &mut BTreeSet<String>,
    values: &mut Vec<PublicValue>,
) -> Result<(), SourceKeyDirectoryMetadataError> {
    if declaration.initializer.is_some() {
        return unsupported("source public initializers need metadata lowering support");
    }
    for item in &declaration.items {
        let name = source_item_name(program, item, "source public value", declaration_values)?;
        if !seen.insert(name.clone()) {
            return unsupported("duplicate source public value name");
        }
        values.push(PublicValue {
            name,
            stage: 1,
            lengths: source_item_lengths(program, item, "source public value", declaration_values)?
                .into_iter()
                .map(u64::from)
                .collect(),
        });
    }
    Ok(())
}

pub(crate) fn source_proof_values(
    program: &SourceProgram,
    constant_values: &BTreeMap<String, FixedFileTemplateValue>,
    active_templates: &BTreeSet<String>,
    template_values: &SourceTemplateConstantValueCache,
    body_caches: &mut SourceControlBodyCaches,
) -> Result<(Vec<u64>, Vec<NamedStageValue>), SourceKeyDirectoryMetadataError> {
    let mut seen = BTreeSet::new();
    let mut counts_by_stage = Vec::<u64>::new();
    let mut values = Vec::new();
    for module in &program.modules {
        let tokens = lex_source(&module.source.contents).map_err(|source| {
            SourceKeyDirectoryMetadataError::Lex {
                source_name: module.source_name.clone(),
                source,
            }
        })?;
        let body_cache = body_caches.module_cache(&module.source_name);
        let mut template_context = SourceValueTemplateContext {
            program,
            module,
            tokens: &tokens,
            body_cache,
            constant_values,
            template_values,
        };
        for declaration in &module.values {
            if declaration.kind != ValueDeclarationKind::ProofValue {
                continue;
            }
            if declaration_in_inactive_template(
                module,
                declaration.start,
                declaration.end,
                active_templates,
            ) {
                continue;
            }
            let Some(declaration_template) =
                source_metadata_declaration_template(module, declaration.start, declaration.end)
            else {
                let declaration_values = source_declaration_constant_values_from_cache(
                    module,
                    declaration.start,
                    declaration.end,
                    constant_values,
                    template_values,
                );
                if source_declaration_in_static_false_branch(
                    program,
                    module,
                    declaration.start,
                    declaration.end,
                    declaration_values,
                ) {
                    continue;
                }
                source_push_proof_values(
                    program,
                    declaration,
                    declaration_values,
                    &mut seen,
                    &mut counts_by_stage,
                    &mut values,
                )?;
                continue;
            };
            let Some(declaration_values) = source_value_declaration_values_for_any_instance(
                &mut template_context,
                declaration,
                declaration_template,
            )?
            else {
                continue;
            };
            source_push_proof_values(
                program,
                declaration,
                &declaration_values,
                &mut seen,
                &mut counts_by_stage,
                &mut values,
            )?;
        }
    }
    Ok((counts_by_stage, values))
}

struct SourceValueTemplateContext<'a, 'b> {
    program: &'a SourceProgram,
    module: &'a SourceProgramModule,
    tokens: &'a [Token],
    body_cache: &'b mut SourceControlBodyCache,
    constant_values: &'a BTreeMap<String, FixedFileTemplateValue>,
    template_values: &'a SourceTemplateConstantValueCache,
}

fn source_value_declaration_values_for_any_instance(
    context: &mut SourceValueTemplateContext<'_, '_>,
    declaration: &ValueDeclaration,
    declaration_template: &AirTemplateDeclaration,
) -> Result<Option<BTreeMap<String, FixedFileTemplateValue>>, SourceKeyDirectoryMetadataError> {
    for instance in source_metadata_template_instances(context.program, &declaration_template.name)
    {
        let values = source_metadata_template_values(
            context.program,
            context.module,
            declaration_template,
            Some(instance),
            context.constant_values,
            context.template_values,
        );
        if source_declaration_in_unselected_static_branch(
            context.program,
            context.module,
            context.tokens,
            context.body_cache,
            declaration.start,
            declaration.end,
            &values,
        )? {
            continue;
        }
        return Ok(Some(values));
    }
    Ok(None)
}

fn source_push_proof_values(
    program: &SourceProgram,
    declaration: &ValueDeclaration,
    declaration_values: &BTreeMap<String, FixedFileTemplateValue>,
    seen: &mut BTreeSet<String>,
    counts_by_stage: &mut Vec<u64>,
    values: &mut Vec<NamedStageValue>,
) -> Result<(), SourceKeyDirectoryMetadataError> {
    let stage = usize::try_from(declaration.stage)
        .map_err(|_| unsupported_source_message("source proof value stage overflow"))?;
    if stage == 0 {
        return unsupported("source proof value stage must be positive");
    }
    if counts_by_stage.len() < stage {
        counts_by_stage.resize(stage, 0);
    }
    for item in &declaration.items {
        let name = source_item_name(program, item, "source proof value", declaration_values)?;
        if !seen.insert(name.clone()) {
            return unsupported("duplicate source proof value name");
        }
        counts_by_stage[stage - 1] = counts_by_stage[stage - 1]
            .checked_add(1)
            .ok_or_else(|| unsupported_source_message("source proof value count overflow"))?;
        values.push(NamedStageValue {
            name,
            stage: u64::from(declaration.stage),
            id: None,
            lengths: source_item_lengths(program, item, "source proof value", declaration_values)?
                .into_iter()
                .map(u64::from)
                .collect(),
        });
    }
    Ok(())
}

pub(crate) fn source_challenge_counts(
    program: &SourceProgram,
    constant_values: &BTreeMap<String, FixedFileTemplateValue>,
    active_templates: &BTreeSet<String>,
    template_values: &SourceTemplateConstantValueCache,
    body_caches: &mut SourceControlBodyCaches,
) -> Result<Vec<u64>, SourceKeyDirectoryMetadataError> {
    let slots = source_challenge_slots(
        program,
        constant_values,
        active_templates,
        template_values,
        body_caches,
    )?;
    let mut counts_by_stage = Vec::<u64>::new();
    for slot in slots {
        let stage_index = usize::try_from(slot.stage)
            .map_err(|_| unsupported_source_message("source challenge stage overflow"))?
            .checked_sub(1)
            .ok_or_else(|| unsupported_source_message("source challenge stage underflow"))?;
        if counts_by_stage.len() <= stage_index {
            counts_by_stage.resize(stage_index + 1, 0);
        }
        counts_by_stage[stage_index] = counts_by_stage[stage_index]
            .checked_add(u64::from(slot.dimension))
            .ok_or_else(|| unsupported_source_message("source challenge count overflow"))?;
    }
    Ok(counts_by_stage)
}

pub(crate) fn source_challenge_slots(
    program: &SourceProgram,
    constant_values: &BTreeMap<String, FixedFileTemplateValue>,
    active_templates: &BTreeSet<String>,
    template_values: &SourceTemplateConstantValueCache,
    body_caches: &mut SourceControlBodyCaches,
) -> Result<Vec<SourceChallengeSlotMetadata>, SourceKeyDirectoryMetadataError> {
    let mut seen = BTreeMap::<String, SourceChallengeShape>::new();
    let mut slots = Vec::<SourceChallengeSlotMetadata>::new();
    let mut next_stage_ids = BTreeMap::<usize, u32>::new();
    for module in &program.modules {
        let tokens = lex_source(&module.source.contents).map_err(|source| {
            SourceKeyDirectoryMetadataError::Lex {
                source_name: module.source_name.clone(),
                source,
            }
        })?;
        let body_cache = body_caches.module_cache(&module.source_name);
        let mut template_context = SourceValueTemplateContext {
            program,
            module,
            tokens: &tokens,
            body_cache,
            constant_values,
            template_values,
        };
        for declaration in &module.values {
            if declaration.kind != ValueDeclarationKind::Challenge {
                continue;
            }
            if declaration_in_inactive_template(
                module,
                declaration.start,
                declaration.end,
                active_templates,
            ) {
                continue;
            }
            let Some(declaration_template) =
                source_metadata_declaration_template(module, declaration.start, declaration.end)
            else {
                let declaration_values = source_declaration_constant_values_from_cache(
                    module,
                    declaration.start,
                    declaration.end,
                    constant_values,
                    template_values,
                );
                if source_declaration_in_static_false_branch(
                    program,
                    module,
                    declaration.start,
                    declaration.end,
                    declaration_values,
                ) {
                    continue;
                }
                source_push_challenge_slots(
                    program,
                    declaration,
                    declaration_values,
                    &mut seen,
                    &mut next_stage_ids,
                    &mut slots,
                )?;
                continue;
            };
            let Some(declaration_values) = source_value_declaration_values_for_any_instance(
                &mut template_context,
                declaration,
                declaration_template,
            )?
            else {
                continue;
            };
            source_push_challenge_slots(
                program,
                declaration,
                &declaration_values,
                &mut seen,
                &mut next_stage_ids,
                &mut slots,
            )?;
        }
    }

    let max_stage = next_stage_ids.keys().copied().max().unwrap_or(0);
    let mut stage_bases = vec![0_u32; max_stage];
    let mut cursor = 0_u32;
    for stage in 1..=max_stage {
        stage_bases[stage - 1] = cursor;
        cursor = cursor
            .checked_add(*next_stage_ids.get(&stage).unwrap_or(&0))
            .ok_or_else(|| unsupported_source_message("source challenge id overflow"))?;
    }

    for slot in &mut slots {
        let stage_index = usize::try_from(slot.stage)
            .map_err(|_| unsupported_source_message("source challenge stage overflow"))?
            .checked_sub(1)
            .ok_or_else(|| unsupported_source_message("source challenge stage underflow"))?;
        slot.id = stage_bases
            .get(stage_index)
            .copied()
            .ok_or_else(|| unsupported_source_message("source challenge stage overflow"))?
            .checked_add(slot.stage_id)
            .ok_or_else(|| unsupported_source_message("source challenge id overflow"))?;
    }

    Ok(slots)
}

fn source_push_challenge_slots(
    program: &SourceProgram,
    declaration: &ValueDeclaration,
    declaration_values: &BTreeMap<String, FixedFileTemplateValue>,
    seen: &mut BTreeMap<String, SourceChallengeShape>,
    next_stage_ids: &mut BTreeMap<usize, u32>,
    slots: &mut Vec<SourceChallengeSlotMetadata>,
) -> Result<(), SourceKeyDirectoryMetadataError> {
    let stage = usize::try_from(declaration.stage)
        .map_err(|_| unsupported_source_message("source challenge stage overflow"))?;
    if stage == 0 {
        return unsupported("source challenge stage must be positive");
    }
    for item in &declaration.items {
        let name = source_item_name(program, item, "source challenge", declaration_values)?;
        let lengths = source_item_lengths(program, item, "source challenge", declaration_values)?;
        let dimension = source_column_dimension(&lengths, "source challenge")?;
        let shape = SourceChallengeShape { stage, dimension };
        if let Some(existing) = seen.get(&name) {
            if *existing != shape {
                return unsupported("duplicate source challenge name");
            }
            continue;
        }
        seen.insert(name.clone(), shape);
        let stage_id = *next_stage_ids.get(&stage).unwrap_or(&0);
        next_stage_ids.insert(
            stage,
            stage_id
                .checked_add(dimension)
                .ok_or_else(|| unsupported_source_message("source challenge id overflow"))?,
        );
        slots.push(SourceChallengeSlotMetadata {
            name,
            id: 0,
            stage: u32::try_from(stage)
                .map_err(|_| unsupported_source_message("source challenge stage overflow"))?,
            stage_id,
            dimension,
        });
    }
    Ok(())
}
