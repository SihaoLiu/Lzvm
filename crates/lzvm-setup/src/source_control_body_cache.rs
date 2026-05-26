use std::collections::BTreeMap;
use std::sync::Arc;

use lzvm_artifacts::expression_info::{CodeOperand, CodeOperation};
use lzvm_pil::{
    parse_function_body_statements, Expression, FunctionStatement, ParseError, SourceFile,
    SourceSpan, Token,
};

use crate::source_statement_hints::SourceExpressionArrayAlias;

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
    returned_expression_array_aliases:
        BTreeMap<SourceReturnedArrayCallKey, Option<SourceExpressionArrayAlias>>,
    returned_expression_array_elements: BTreeMap<SourceReturnedArrayElementKey, Option<Expression>>,
    returned_constraint_array_elements:
        BTreeMap<SourceReturnedConstraintElementKey, Option<SourceConstraintFragment>>,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) struct SourceReturnedArrayCallKey {
    source_name: String,
    start: usize,
    end: usize,
    static_values: Vec<(String, String)>,
}

impl SourceReturnedArrayCallKey {
    pub(crate) fn new(
        source_name: String,
        start: usize,
        end: usize,
        static_values: Vec<(String, String)>,
    ) -> Self {
        Self {
            source_name,
            start,
            end,
            static_values,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) struct SourceReturnedArrayElementKey {
    call: SourceReturnedArrayCallKey,
    indices: Vec<usize>,
}

impl SourceReturnedArrayElementKey {
    pub(crate) fn new(call: SourceReturnedArrayCallKey, indices: Vec<usize>) -> Self {
        Self { call, indices }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) struct SourceReturnedConstraintElementKey {
    call: SourceReturnedArrayCallKey,
    indices: Vec<u32>,
    row_offset: i64,
}

impl SourceReturnedConstraintElementKey {
    pub(crate) fn new(
        call: SourceReturnedArrayCallKey,
        indices: Vec<u32>,
        row_offset: i64,
    ) -> Self {
        Self {
            call,
            indices,
            row_offset,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct SourceConstraintFragment {
    pub(crate) operations: Vec<CodeOperation>,
    pub(crate) result: CodeOperand,
    pub(crate) temporary_count: u32,
    pub(crate) offset_min: i64,
    pub(crate) offset_max: i64,
}

impl SourceControlBodyCache {
    pub(crate) fn returned_expression_array_alias(
        &self,
        key: &SourceReturnedArrayCallKey,
    ) -> Option<Option<SourceExpressionArrayAlias>> {
        self.returned_expression_array_aliases.get(key).cloned()
    }

    pub(crate) fn insert_returned_expression_array_alias(
        &mut self,
        key: SourceReturnedArrayCallKey,
        alias: Option<SourceExpressionArrayAlias>,
    ) {
        self.returned_expression_array_aliases.insert(key, alias);
    }

    pub(crate) fn returned_expression_array_element(
        &self,
        key: &SourceReturnedArrayElementKey,
    ) -> Option<Option<Expression>> {
        self.returned_expression_array_elements.get(key).cloned()
    }

    pub(crate) fn insert_returned_expression_array_element(
        &mut self,
        key: SourceReturnedArrayElementKey,
        expression: Option<Expression>,
    ) {
        self.returned_expression_array_elements
            .insert(key, expression);
    }

    pub(crate) fn returned_constraint_array_element(
        &self,
        key: &SourceReturnedConstraintElementKey,
    ) -> Option<Option<SourceConstraintFragment>> {
        self.returned_constraint_array_elements.get(key).cloned()
    }

    pub(crate) fn insert_returned_constraint_array_element(
        &mut self,
        key: SourceReturnedConstraintElementKey,
        fragment: Option<SourceConstraintFragment>,
    ) {
        self.returned_constraint_array_elements
            .insert(key, fragment);
    }

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
