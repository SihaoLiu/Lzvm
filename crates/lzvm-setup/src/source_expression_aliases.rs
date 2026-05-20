use lzvm_pil::{FunctionStatement, FunctionStatementDeclaration};

use crate::source_constraint_lowering::SourceExpressionAliases;

pub(crate) fn collect_source_template_expression_alias(
    statement: &FunctionStatement,
    expression_aliases: &mut SourceExpressionAliases,
) {
    let Some(FunctionStatementDeclaration::Constant(declaration)) = statement.declaration.as_ref()
    else {
        return;
    };
    if declaration.type_name.as_deref() != Some("expr") || !declaration.array_dims.is_empty() {
        return;
    }
    let Some(expression) = declaration.initializer_expression.as_ref() else {
        return;
    };
    expression_aliases.insert(declaration.name.clone(), expression.clone());
}
