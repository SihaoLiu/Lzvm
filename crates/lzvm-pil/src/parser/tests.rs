use super::{
    parse_air_group_declarations, parse_air_group_value_declarations,
    parse_air_template_declarations, parse_column_declarations, parse_commit_declarations,
    parse_container_declarations, parse_include_directives, parse_pragma_directives,
    parse_public_declarations, parse_public_table_declarations, parse_use_directives,
    parse_value_declarations, ColumnInitializerKind, ColumnKind, IncludeKind, IncludeVisibility,
    ParseError, ValueDeclarationKind,
};
use crate::SourceFile;
use std::path::PathBuf;

fn source(contents: &str) -> SourceFile {
    SourceFile {
        contents: contents.to_owned(),
        file_dir: PathBuf::from("/case"),
        full_path: PathBuf::from("/case/main.pil"),
        source_name: "main.pil".to_owned(),
    }
}

#[test]
fn parses_static_include_directives() {
    let source =
        source("include \"a.pil\";\nprivate require \"b.pil\";\npublic include \"c.pil\";");

    let directives = parse_include_directives(&source).expect("directives should parse");

    assert_eq!(directives.len(), 3);
    assert_eq!(directives[0].kind, IncludeKind::Include);
    assert_eq!(directives[0].visibility, IncludeVisibility::Public);
    assert_eq!(directives[0].file, "a.pil");
    assert_eq!(directives[1].kind, IncludeKind::Require);
    assert_eq!(directives[1].visibility, IncludeVisibility::Private);
    assert_eq!(directives[1].file, "b.pil");
    assert_eq!(directives[2].kind, IncludeKind::Include);
    assert_eq!(directives[2].visibility, IncludeVisibility::Public);
    assert_eq!(directives[2].file, "c.pil");
}

#[test]
fn ignores_visibility_modifiers_that_do_not_start_include_directives() {
    let source = source("public function f() { return; }\nprivate int x = 1;");

    let directives = parse_include_directives(&source).expect("source should parse");

    assert!(directives.is_empty());
}

#[test]
fn rejects_template_include_paths() {
    let source = source("include `dynamic/${name}.pil`;");

    let error = parse_include_directives(&source).expect_err("template path should fail");

    assert!(matches!(
        error,
        ParseError::TemplatePath { source_name, .. } if source_name == "main.pil"
    ));
}

#[test]
fn rejects_include_without_path_literal() {
    let source = source("include ;");

    let error = parse_include_directives(&source).expect_err("path should be required");

    assert!(matches!(
        error,
        ParseError::ExpectedPath { source_name, .. } if source_name == "main.pil"
    ));
}

#[test]
fn rejects_missing_statement_terminator() {
    let source = source("include \"a.pil\" const N = 1;");

    let error = parse_include_directives(&source).expect_err("semicolon should be required");

    assert!(matches!(
        error,
        ParseError::ExpectedTerminator { source_name, .. } if source_name == "main.pil"
    ));
}

#[test]
fn parses_pragma_directives_with_raw_values() {
    let source = source("#pragma arg -I pil,lib\n#pragma feature fast\nconst int N = 2**16;");

    let directives = parse_pragma_directives(&source).expect("pragmas should parse");

    assert_eq!(directives.len(), 2);
    assert_eq!(directives[0].value, "arg -I pil,lib");
    assert_eq!(
        &source.contents[directives[0].start..directives[0].end],
        "#pragma arg -I pil,lib"
    );
    assert_eq!(directives[1].value, "feature fast");
}

#[test]
fn parses_use_directives_with_names_and_aliases() {
    let source = source("use air.main;\nuse proof.root.branch alias local_root;\nuse pkg.item;");

    let directives = parse_use_directives(&source).expect("use directives should parse");

    assert_eq!(directives.len(), 3);
    assert_eq!(directives[0].name, "air.main");
    assert_eq!(directives[0].alias, None);
    assert_eq!(directives[1].name, "proof.root.branch");
    assert_eq!(directives[1].alias.as_deref(), Some("local_root"));
    assert_eq!(directives[2].name, "pkg.item");
}

#[test]
fn rejects_use_without_name_reference() {
    let source = source("use ;");

    let error = parse_use_directives(&source).expect_err("name should be required");

    assert!(matches!(
        error,
        ParseError::ExpectedName { source_name, .. } if source_name == "main.pil"
    ));
}

#[test]
fn rejects_use_alias_without_identifier() {
    let source = source("use pkg.item alias ;");

    let error = parse_use_directives(&source).expect_err("alias identifier should be required");

    assert!(matches!(
        error,
        ParseError::ExpectedAlias { source_name, .. } if source_name == "main.pil"
    ));
}

#[test]
fn rejects_use_without_statement_terminator() {
    let source = source("use pkg.item include \"x.pil\";");

    let error = parse_use_directives(&source).expect_err("semicolon should be required");

    assert!(matches!(
        error,
        ParseError::ExpectedTerminator { source_name, .. } if source_name == "main.pil"
    ));
}

#[test]
fn parses_container_declarations_with_names_and_aliases() {
    let source = source(
        "container air.main;\ncontainer proof.root.branch alias local_root;\ncontainer pkg.item;",
    );

    let declarations =
        parse_container_declarations(&source).expect("container declarations should parse");

    assert_eq!(declarations.len(), 3);
    assert_eq!(declarations[0].name, "air.main");
    assert_eq!(declarations[0].alias, None);
    assert_eq!(declarations[0].body, None);
    assert_eq!(declarations[1].name, "proof.root.branch");
    assert_eq!(declarations[1].alias.as_deref(), Some("local_root"));
    assert_eq!(declarations[1].body, None);
    assert_eq!(declarations[2].name, "pkg.item");
    assert_eq!(declarations[2].body, None);
}

#[test]
fn parses_closed_container_body_span() {
    let source = source("container air.main { col witness x; }");

    let declarations = parse_container_declarations(&source).expect("container body should parse");

    assert_eq!(declarations.len(), 1);
    assert_eq!(declarations[0].name, "air.main");
    let body = declarations[0].body.expect("body span should be recorded");
    assert_eq!(&source.contents[body.start..body.end], "{ col witness x; }");
    assert_eq!(declarations[0].end, body.end);
}

#[test]
fn parses_closed_container_alias_body_span() {
    let source = source("container proof.root alias local_root { }");

    let declarations = parse_container_declarations(&source).expect("container body should parse");

    assert_eq!(declarations.len(), 1);
    assert_eq!(declarations[0].name, "proof.root");
    assert_eq!(declarations[0].alias.as_deref(), Some("local_root"));
    let body = declarations[0].body.expect("body span should be recorded");
    assert_eq!(&source.contents[body.start..body.end], "{ }");
}

#[test]
fn keeps_nested_blocks_inside_closed_container_body_span() {
    let source = source("container pkg.item { function run() { return; } }");

    let declarations = parse_container_declarations(&source).expect("container body should parse");

    assert_eq!(declarations.len(), 1);
    let body = declarations[0].body.expect("body span should be recorded");
    assert_eq!(
        &source.contents[body.start..body.end],
        "{ function run() { return; } }"
    );
    assert_eq!(declarations[0].end, source.contents.len());
}

#[test]
fn rejects_container_without_name_reference() {
    let source = source("container ;");

    let error = parse_container_declarations(&source).expect_err("name should be required");

    assert!(matches!(
        error,
        ParseError::ExpectedName { source_name, .. } if source_name == "main.pil"
    ));
}

#[test]
fn rejects_container_alias_without_identifier() {
    let source = source("container pkg.item alias ;");

    let error =
        parse_container_declarations(&source).expect_err("alias identifier should be required");

    assert!(matches!(
        error,
        ParseError::ExpectedAlias { source_name, .. } if source_name == "main.pil"
    ));
}

#[test]
fn rejects_unclosed_container_body() {
    let source = source("container pkg.item { col witness x;");

    let error = parse_container_declarations(&source).expect_err("body should close");

    assert!(matches!(
        error,
        ParseError::ExpectedCloseBrace { source_name, .. } if source_name == "main.pil"
    ));
}

#[test]
fn parses_air_template_declarations_with_params_and_nested_body() {
    let source = source(
        "airtemplate Main(const int N = 2**22, string label = \"main\") \
         { col witness trace; if (N) { return; } }",
    );

    let declarations =
        parse_air_template_declarations(&source).expect("air templates should parse");

    assert_eq!(declarations.len(), 1);
    assert_eq!(declarations[0].name, "Main");
    assert_eq!(
        &source.contents[declarations[0].params.start..declarations[0].params.end],
        "(const int N = 2**22, string label = \"main\")"
    );
    assert_eq!(
        &source.contents[declarations[0].body.start..declarations[0].body.end],
        "{ col witness trace; if (N) { return; } }"
    );
}

#[test]
fn parses_air_group_declarations_with_nested_body_spans() {
    let source = source(
        "airgroup Main { virtual Range(id: 7) alias Range7; \
         for (int i = 0; i < 2; i++) { commit(i); } }\n\
         airgroup Aux { Main(); }",
    );

    let declarations = parse_air_group_declarations(&source).expect("air groups should parse");

    assert_eq!(declarations.len(), 2);
    assert_eq!(declarations[0].name, "Main");
    assert_eq!(
        &source.contents[declarations[0].body.start..declarations[0].body.end],
        "{ virtual Range(id: 7) alias Range7; for (int i = 0; i < 2; i++) { commit(i); } }"
    );
    assert_eq!(declarations[1].name, "Aux");
    assert_eq!(
        &source.contents[declarations[1].body.start..declarations[1].body.end],
        "{ Main(); }"
    );
}

#[test]
fn skips_air_group_scope_references_when_parsing_air_group_declarations() {
    let source = source("on final airgroup finalize();\nairgroup Main { Main(); }");

    let declarations = parse_air_group_declarations(&source).expect("air groups should parse");

    assert_eq!(declarations.len(), 1);
    assert_eq!(declarations[0].name, "Main");
}

#[test]
fn rejects_unclosed_air_group_body() {
    let source = source("airgroup Main { Main();");

    let error = parse_air_group_declarations(&source).expect_err("body should close");

    assert!(matches!(
        error,
        ParseError::ExpectedCloseBrace { source_name, .. } if source_name == "main.pil"
    ));
}

#[test]
fn parses_witness_column_declarations_with_array_items() {
    let source = source("col witness air.main[2], local[1][];");

    let declarations = parse_column_declarations(&source).expect("columns should parse");

    assert_eq!(declarations.len(), 1);
    assert_eq!(declarations[0].kind, ColumnKind::Witness);
    assert_eq!(declarations[0].commit, None);
    assert!(declarations[0].features.is_empty());
    assert_eq!(declarations[0].items.len(), 2);
    assert_eq!(declarations[0].items[0].name, "air.main");
    assert!(!declarations[0].items[0].template);
    assert_eq!(
        &source.contents[declarations[0].items[0].array_dims[0].start
            ..declarations[0].items[0].array_dims[0].end],
        "[2]"
    );
    assert_eq!(declarations[0].items[1].name, "local");
    assert_eq!(declarations[0].items[1].array_dims.len(), 2);
    assert_eq!(
        &source.contents[declarations[0].items[1].array_dims[0].start
            ..declarations[0].items[1].array_dims[0].end],
        "[1]"
    );
    assert_eq!(
        &source.contents[declarations[0].items[1].array_dims[1].start
            ..declarations[0].items[1].array_dims[1].end],
        "[]"
    );
}

#[test]
fn parses_custom_column_declarations_with_feature_spans() {
    let source = source("col local_commit stage(1 + (2)) virtual(foo(bar)) air.main, local;");

    let declarations = parse_column_declarations(&source).expect("columns should parse");

    assert_eq!(declarations.len(), 1);
    assert_eq!(declarations[0].kind, ColumnKind::Custom);
    assert_eq!(declarations[0].commit.as_deref(), Some("local_commit"));
    assert_eq!(declarations[0].features.len(), 2);
    assert_eq!(declarations[0].features[0].name, "stage");
    assert_eq!(
        &source.contents
            [declarations[0].features[0].args.start..declarations[0].features[0].args.end],
        "(1 + (2))"
    );
    assert_eq!(declarations[0].features[1].name, "virtual");
    assert_eq!(
        &source.contents
            [declarations[0].features[1].args.start..declarations[0].features[1].args.end],
        "(foo(bar))"
    );
    assert_eq!(declarations[0].items.len(), 2);
    assert_eq!(declarations[0].items[0].name, "air.main");
    assert_eq!(declarations[0].items[1].name, "local");
}

#[test]
fn parses_fixed_column_initializer_spans() {
    let source = source("col fixed stage(3) x = foo(bar[1] + baz);");

    let declarations = parse_column_declarations(&source).expect("columns should parse");

    assert_eq!(declarations.len(), 1);
    assert_eq!(declarations[0].kind, ColumnKind::Fixed);
    assert_eq!(declarations[0].features.len(), 1);
    assert_eq!(declarations[0].features[0].name, "stage");
    assert_eq!(declarations[0].items.len(), 1);
    assert_eq!(declarations[0].items[0].name, "x");
    let initializer = declarations[0]
        .initializer
        .expect("initializer should be recorded");
    assert_eq!(initializer.kind, ColumnInitializerKind::Expression);
    assert_eq!(
        &source.contents[initializer.span.start..initializer.span.end],
        "foo(bar[1] + baz)"
    );
}

#[test]
fn parses_sequence_initializer_spans() {
    let source = source("col fixed x = [foo(bar), baz[1]];");

    let declarations = parse_column_declarations(&source).expect("columns should parse");

    assert_eq!(declarations.len(), 1);
    let initializer = declarations[0]
        .initializer
        .expect("initializer should be recorded");
    assert_eq!(initializer.kind, ColumnInitializerKind::Sequence);
    assert_eq!(
        &source.contents[initializer.span.start..initializer.span.end],
        "[foo(bar), baz[1]]"
    );
}

#[test]
fn skips_col_cast_expressions() {
    let source = source("value = col(x);");

    let declarations = parse_column_declarations(&source).expect("source should parse");

    assert!(declarations.is_empty());
}

#[test]
fn parses_stage_value_declarations_with_defaults() {
    let source = source(
        "challenge stage(7) air.main[2], local;\nproofval proof.x;\nairval stage(4) unit.a;",
    );

    let declarations = parse_value_declarations(&source).expect("value declarations should parse");

    assert_eq!(declarations.len(), 3);
    assert_eq!(declarations[0].kind, ValueDeclarationKind::Challenge);
    assert_eq!(declarations[0].stage, 7);
    assert_eq!(declarations[0].items[0].name, "air.main");
    assert_eq!(declarations[0].items[0].array_dims.len(), 1);
    assert_eq!(declarations[0].items[1].name, "local");
    assert_eq!(declarations[1].kind, ValueDeclarationKind::ProofValue);
    assert_eq!(declarations[1].stage, 1);
    assert_eq!(declarations[1].items[0].name, "proof.x");
    assert_eq!(declarations[2].kind, ValueDeclarationKind::AirValue);
    assert_eq!(declarations[2].stage, 4);
    assert_eq!(declarations[2].items[0].name, "unit.a");
}

#[test]
fn skips_cast_expressions_when_parsing_value_declarations() {
    let source = source("result = challenge(stage(3));");

    let declarations = parse_value_declarations(&source).expect("source should parse");

    assert!(declarations.is_empty());
}

#[test]
fn parses_group_value_declarations_with_properties() {
    let source =
        source("airgroupval stage(5) default(foo(bar + 1)) aggregate(sum) group.a[2], local;");

    let declarations =
        parse_air_group_value_declarations(&source).expect("group values should parse");

    assert_eq!(declarations.len(), 1);
    assert_eq!(declarations[0].stage, 5);
    assert_eq!(
        &source.contents[declarations[0].default_value.expect("default").start
            ..declarations[0].default_value.expect("default").end],
        "(foo(bar + 1))"
    );
    assert_eq!(declarations[0].aggregate_type.as_deref(), Some("sum"));
    assert_eq!(declarations[0].items.len(), 2);
    assert_eq!(declarations[0].items[0].name, "group.a");
    assert_eq!(
        &source.contents[declarations[0].items[0].array_dims[0].start
            ..declarations[0].items[0].array_dims[0].end],
        "[2]"
    );
    assert_eq!(declarations[0].items[1].name, "local");
}

#[test]
fn parses_group_value_declarations_with_default_stage() {
    let source = source("airgroupval aggregate(prod) local;");

    let declarations =
        parse_air_group_value_declarations(&source).expect("group values should parse");

    assert_eq!(declarations.len(), 1);
    assert_eq!(declarations[0].stage, 2);
    assert_eq!(declarations[0].default_value, None);
    assert_eq!(declarations[0].aggregate_type.as_deref(), Some("prod"));
    assert_eq!(declarations[0].items[0].name, "local");
}

#[test]
fn rejects_duplicate_group_value_properties() {
    let source = source("airgroupval stage(1) stage(2) local;");

    let error = parse_air_group_value_declarations(&source).expect_err("duplicate should fail");

    assert!(matches!(
        error,
        ParseError::DuplicateProperty { source_name, name, .. }
            if source_name == "main.pil" && name == "stage"
    ));
}

#[test]
fn parses_commit_declarations_with_public_references() {
    let source =
        source("commit stage(3) public(air.main, proof.root) entry;\ncommit stage(2) local;");

    let declarations =
        parse_commit_declarations(&source).expect("commit declarations should parse");

    assert_eq!(declarations.len(), 2);
    assert_eq!(declarations[0].stage, 3);
    assert_eq!(declarations[0].publics, vec!["air.main", "proof.root"]);
    assert_eq!(declarations[0].name, "entry");
    assert_eq!(declarations[1].stage, 2);
    assert!(declarations[1].publics.is_empty());
    assert_eq!(declarations[1].name, "local");
}

#[test]
fn parses_public_declarations_with_lists_and_initializers() {
    let source = source("public air.main[2], local;\npublic scalar = foo(bar[1] + baz);");

    let declarations =
        parse_public_declarations(&source).expect("public declarations should parse");

    assert_eq!(declarations.len(), 2);
    assert_eq!(declarations[0].items.len(), 2);
    assert_eq!(declarations[0].items[0].name, "air.main");
    assert_eq!(
        &source.contents[declarations[0].items[0].array_dims[0].start
            ..declarations[0].items[0].array_dims[0].end],
        "[2]"
    );
    assert_eq!(declarations[0].items[1].name, "local");
    assert_eq!(declarations[0].initializer, None);
    assert_eq!(declarations[1].items.len(), 1);
    assert_eq!(declarations[1].items[0].name, "scalar");
    let initializer = declarations[1].initializer.expect("initializer");
    assert_eq!(
        &source.contents[initializer.start..initializer.end],
        "foo(bar[1] + baz)"
    );
}

#[test]
fn skips_public_modifiers_and_references_when_parsing_public_declarations() {
    let source = source("public include \"a.pil\";\ncommit stage(1) public(local) out;");

    let declarations = parse_public_declarations(&source).expect("source should parse");

    assert!(declarations.is_empty());
}

#[test]
fn parses_public_table_declarations_with_and_without_args() {
    let source = source(
        "publictable aggregate(sum, fold, foo(bar + 1), baz[2]) table[cols + 1][rows];\n\
         publictable aggregate(sum, fold) other[rows][cols + 1];",
    );

    let declarations =
        parse_public_table_declarations(&source).expect("public table declarations should parse");

    assert_eq!(declarations.len(), 2);
    assert_eq!(declarations[0].aggregate_type, "sum");
    assert_eq!(declarations[0].aggregate_function, "fold");
    assert_eq!(declarations[0].name, "table");
    let args = declarations[0].args.expect("args");
    assert_eq!(
        &source.contents[args.start..args.end],
        "foo(bar + 1), baz[2]"
    );
    assert_eq!(
        &source.contents[declarations[0].cols.start..declarations[0].cols.end],
        "[cols + 1]"
    );
    assert_eq!(
        &source.contents[declarations[0].rows.start..declarations[0].rows.end],
        "[rows]"
    );
    assert_eq!(declarations[1].aggregate_type, "sum");
    assert_eq!(declarations[1].aggregate_function, "fold");
    assert_eq!(declarations[1].name, "other");
    assert_eq!(declarations[1].args, None);
    assert_eq!(
        &source.contents[declarations[1].cols.start..declarations[1].cols.end],
        "[rows]"
    );
    assert_eq!(
        &source.contents[declarations[1].rows.start..declarations[1].rows.end],
        "[cols + 1]"
    );
}
