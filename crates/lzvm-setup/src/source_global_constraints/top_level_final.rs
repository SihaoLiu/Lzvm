use crate::{
    source_final_proof_calls::source_final_proof_call_at,
    source_key_directory::SourceKeyDirectoryMetadataError,
};

use super::{
    lower_top_level_expression_statement, top_level_call, SourceGlobalConstraintBuilder,
    SourceTopLevelGlobalConstraintContext,
};

pub(super) fn lower_top_level_final_statement(
    context: &SourceTopLevelGlobalConstraintContext<'_, '_, '_>,
    index: usize,
    constraints: &mut SourceGlobalConstraintBuilder,
) -> Result<usize, SourceKeyDirectoryMetadataError> {
    let Some(call) = source_final_proof_call_at(context.tokens, index, &context.module.source)?
    else {
        return lower_top_level_expression_statement(context, index, constraints);
    };
    if top_level_call::lower_top_level_function_call(context, &call.expression, constraints)? {
        Ok(call.next_index)
    } else {
        lower_top_level_expression_statement(context, index, constraints)
    }
}
