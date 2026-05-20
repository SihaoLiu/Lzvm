use std::collections::BTreeMap;
use std::sync::Arc;

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
    statements: BTreeMap<(usize, usize), Arc<[FunctionStatement]>>,
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
    ) -> Result<Arc<[FunctionStatement]>, ParseError> {
        let key = (body.start, body.end);
        if let Some(statements) = self.statements.get(&key) {
            return Ok(Arc::clone(statements));
        }

        let statements = parse_function_body_statements(tokens, body, source)?.into();
        self.statements.insert(key, Arc::clone(&statements));
        Ok(statements)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::{Duration, Instant};

    use lzvm_pil::{lex_source, SourceFile, SourceSpan};

    use super::SourceControlBodyCache;

    #[test]
    fn body_statements_reuses_cached_body_handles() {
        let mut contents = "{\n".to_owned();
        for index in 0..512 {
            contents.push_str(&format!(
                "table.value[{index}] = ((({index} + 1) * 3) + (({index} + 2) * 5));\n"
            ));
        }
        contents.push('}');
        let source = SourceFile {
            contents: contents.clone(),
            file_dir: PathBuf::new(),
            full_path: PathBuf::from("main.pil"),
            source_name: "main.pil".to_owned(),
        };
        let tokens = lex_source(&contents).expect("source should lex");
        let body = SourceSpan {
            start: 0,
            end: contents.len(),
        };
        let mut cache = SourceControlBodyCache::default();
        let first = cache
            .body_statements(&tokens, body, &source)
            .expect("body should parse");
        assert_eq!(first.len(), 512);

        let started = Instant::now();
        for _ in 0..2000 {
            let cached = cache
                .body_statements(&tokens, body, &source)
                .expect("body should come from cache");
            assert_eq!(cached.len(), 512);
        }
        let elapsed = started.elapsed();
        assert!(
            elapsed < Duration::from_millis(200),
            "cached body lookups took {elapsed:?}"
        );
    }
}
