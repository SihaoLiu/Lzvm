use lzvm_pil::{CallArgument, Expression, ExpressionKind};

use crate::{
    source_key_directory::SourceKeyDirectoryMetadataError,
    source_static_values::evaluate_source_static_expression,
};

use super::SourceTopLevelGlobalConstraintContext;

pub(super) fn lower_top_level_static_assertion(
    context: &SourceTopLevelGlobalConstraintContext<'_, '_, '_>,
    expression: &Expression,
    source_line: &str,
) -> Result<bool, SourceKeyDirectoryMetadataError> {
    let Some((name, arguments)) = source_call_expression(expression) else {
        return Ok(false);
    };
    if name != "assert_eq" || arguments.len() != 2 || arguments.iter().any(named_argument) {
        return Ok(false);
    }
    let left = evaluate_source_static_expression(
        context.program,
        &arguments[0].value,
        &context.alias_scope.static_values,
    );
    let right = evaluate_source_static_expression(
        context.program,
        &arguments[1].value,
        &context.alias_scope.static_values,
    );
    match (left, right) {
        (Some(left), Some(right)) if left == right => Ok(true),
        (Some(_), Some(_)) => Err(SourceKeyDirectoryMetadataError::StaticAssertionFailed {
            line: source_line.trim().to_owned(),
        }),
        _ => Ok(false),
    }
}

fn source_call_expression(expression: &Expression) -> Option<(&str, &[CallArgument])> {
    let ExpressionKind::Call { callee, args } = &expression.kind else {
        return None;
    };
    let ExpressionKind::Name(name) = &callee.kind else {
        return None;
    };
    Some((name.as_str(), args.as_slice()))
}

fn named_argument(argument: &CallArgument) -> bool {
    argument.name.is_some()
}
