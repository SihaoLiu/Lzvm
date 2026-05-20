use std::collections::BTreeMap;

use lzvm_pil::{
    parse_function_body_statements, FunctionStatement, ParseError, SourceFile, SourceSpan, Token,
};

#[derive(Debug, Default)]
pub(crate) struct SourceControlBodyCaches {
    modules: BTreeMap<String, SourceControlBodyCache>,
}

impl SourceControlBodyCaches {
    pub(crate) fn module_cache(&mut self, source_name: &str) -> &mut SourceControlBodyCache {
        self.modules.entry(source_name.to_owned()).or_default()
    }
}

#[derive(Debug, Default)]
pub(crate) struct SourceControlBodyCache {
    statements: BTreeMap<(usize, usize), Vec<FunctionStatement>>,
    token_bounds: BTreeMap<(usize, usize), Option<(usize, usize)>>,
}

impl SourceControlBodyCache {
    pub(crate) fn span_token_bounds(
        &mut self,
        tokens: &[Token],
        span: SourceSpan,
    ) -> Option<(usize, usize)> {
        let key = (span.start, span.end);
        if let Some(bounds) = self.token_bounds.get(&key) {
            return *bounds;
        }

        let bounds = tokens
            .iter()
            .position(|token| token.start == span.start)
            .zip(
                tokens
                    .iter()
                    .position(|token| token.end == span.end)
                    .map(|index| index + 1),
            );
        self.token_bounds.insert(key, bounds);
        bounds
    }

    pub(crate) fn body_statements(
        &mut self,
        tokens: &[Token],
        body: SourceSpan,
        source: &SourceFile,
    ) -> Result<Vec<FunctionStatement>, ParseError> {
        let key = (body.start, body.end);
        if let Some(statements) = self.statements.get(&key) {
            return Ok(statements.clone());
        }

        let statements = parse_function_body_statements(tokens, body, source)?;
        self.statements.insert(key, statements.clone());
        Ok(statements)
    }
}
