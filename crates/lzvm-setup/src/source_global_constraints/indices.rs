use std::collections::BTreeSet;

use lzvm_pil::{Expression, ExpressionKind};

use crate::source_key_directory::SourceKeyDirectoryMetadataError;

use super::{
    static_u32_expression, strip_group_expression, unsupported, unsupported_source_message,
    SourceGlobalAliasScope, SourceGlobalExpressionArrayAlias, SourceGlobalExpressionArrayAliases,
};

pub(super) fn source_global_index_chain(
    target: &Expression,
    index: &Expression,
    alias_scope: &SourceGlobalAliasScope<'_>,
) -> Option<(String, Vec<u32>)> {
    let mut indices = vec![static_u32_expression(index, alias_scope)?];
    let mut cursor = strip_group_expression(target);
    loop {
        match &cursor.kind {
            ExpressionKind::Name(name) => {
                indices.reverse();
                return Some((name.clone(), indices));
            }
            ExpressionKind::Index { target, index } => {
                indices.push(static_u32_expression(index, alias_scope)?);
                cursor = strip_group_expression(target);
            }
            _ => return None,
        }
    }
}

pub(super) fn source_global_named_array_alias_target(
    expression_array_aliases: &SourceGlobalExpressionArrayAliases,
    name: &str,
    resolving_aliases: &mut BTreeSet<String>,
) -> Option<String> {
    let alias = expression_array_aliases.get(name)?;
    match alias {
        SourceGlobalExpressionArrayAlias::Name(alias_name) => {
            if !expression_array_aliases.contains_key(alias_name) {
                return Some(alias_name.clone());
            }
            if !resolving_aliases.insert(name.to_owned()) {
                return None;
            }
            let target = source_global_named_array_alias_target(
                expression_array_aliases,
                alias_name,
                resolving_aliases,
            );
            resolving_aliases.remove(name);
            target
        }
        SourceGlobalExpressionArrayAlias::Values(_) => None,
    }
}

pub(super) fn source_global_public_value_lengths(
    lengths: &[u64],
) -> Result<Vec<u32>, SourceKeyDirectoryMetadataError> {
    lengths
        .iter()
        .map(|length| {
            u32::try_from(*length)
                .map_err(|_| unsupported_source_message("source public value dimension overflow"))
        })
        .collect()
}

pub(super) fn source_global_named_stage_value_lengths(
    lengths: &[u64],
) -> Result<Vec<u32>, SourceKeyDirectoryMetadataError> {
    lengths
        .iter()
        .map(|length| {
            u32::try_from(*length)
                .map_err(|_| unsupported_source_message("source stage value dimension overflow"))
        })
        .collect()
}

pub(super) fn source_global_linear_index(
    indices: &[u32],
    lengths: &[u32],
    dimension: u32,
    scalar_message: &'static str,
    range_message: &'static str,
    overflow_message: &'static str,
) -> Result<u32, SourceKeyDirectoryMetadataError> {
    match indices {
        [] if dimension == 1 => Ok(0),
        [] => unsupported(scalar_message),
        [index] if *index < dimension => Ok(*index),
        [_] => unsupported(range_message),
        _ if indices.len() != lengths.len() => {
            unsupported("top-level source index rank does not match source value shape")
        }
        _ => {
            let mut linear = 0_u32;
            for (index, length) in indices.iter().zip(lengths) {
                if index >= length {
                    return unsupported(range_message);
                }
                linear = linear
                    .checked_mul(*length)
                    .and_then(|base| base.checked_add(*index))
                    .ok_or_else(|| unsupported_source_message(overflow_message))?;
            }
            if linear >= dimension {
                return unsupported(range_message);
            }
            Ok(linear)
        }
    }
}
