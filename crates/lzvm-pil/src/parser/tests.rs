use super::{
    parse_air_group_declarations, parse_air_group_value_declarations,
    parse_air_instance_declarations, parse_air_template_declarations, parse_column_declarations,
    parse_commit_declarations, parse_constant_declarations, parse_container_declarations,
    parse_function_declarations, parse_include_directives, parse_pragma_directives,
    parse_public_declarations, parse_public_table_declarations, parse_use_directives,
    parse_value_declarations, parse_variable_declarations, BinaryOperator, ColumnInitializerKind,
    ColumnKind, ConstantDeclarationKind, Expression, ExpressionKind, FunctionStatementDeclaration,
    FunctionStatementKind, FunctionVisibility, IncludeKind, IncludeVisibility, ParseError,
    UnaryOperator, ValueDeclarationKind,
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

fn assert_row_offset_add_one(expression: &Expression, name: &str, offset_value: &str) {
    let ExpressionKind::Binary {
        op: BinaryOperator::Add,
        left,
        right,
    } = &expression.kind
    else {
        panic!("expression should be an addition");
    };
    assert!(matches!(
        &left.kind,
        ExpressionKind::RowOffset {
            target,
            offset,
            prior,
        } if !prior
            && matches!(&target.kind, ExpressionKind::Name(target_name) if target_name == name)
            && matches!(&offset.kind, ExpressionKind::Integer(value) if value == offset_value)
    ));
    assert!(matches!(
        &right.kind,
        ExpressionKind::Integer(value) if value == "1"
    ));
}

fn assert_binary_add_name_plus_one(expression: &Expression, name: &str) {
    let ExpressionKind::Binary {
        op: BinaryOperator::Add,
        left,
        right,
    } = &expression.kind
    else {
        panic!("expression should be an addition");
    };
    assert!(matches!(
        &left.kind,
        ExpressionKind::Name(value) if value == name
    ));
    assert!(matches!(
        &right.kind,
        ExpressionKind::Integer(value) if value == "1"
    ));
}

fn assert_integer_expression(expression: &Expression, expected: &str) {
    assert!(matches!(
        &expression.kind,
        ExpressionKind::Integer(value) if value == expected
    ));
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
fn parses_constant_declarations_and_skips_const_parameters() {
    let source = source(
        "constant LEGACY = 4;\n\
         const int OP_FLAG = 0x00;\n\
         function f(const expr op = 0) { const expr local[2]; return op; }",
    );

    let declarations =
        parse_constant_declarations(&source).expect("constant declarations should parse");

    assert_eq!(declarations.len(), 3);
    assert_eq!(declarations[0].kind, ConstantDeclarationKind::Constant);
    assert_eq!(declarations[0].type_name, None);
    assert_eq!(declarations[0].name, "LEGACY");
    assert_integer_expression(
        declarations[0]
            .initializer_expression
            .as_ref()
            .expect("legacy initializer expression"),
        "4",
    );
    assert_eq!(declarations[1].kind, ConstantDeclarationKind::Const);
    assert_eq!(declarations[1].type_name.as_deref(), Some("int"));
    assert_eq!(declarations[1].name, "OP_FLAG");
    assert!(matches!(
        declarations[1]
            .initializer_expression
            .as_ref()
            .expect("typed initializer expression")
            .kind,
        ExpressionKind::HexInteger(ref value) if value == "0x00"
    ));
    assert_eq!(declarations[2].kind, ConstantDeclarationKind::Const);
    assert_eq!(declarations[2].type_name.as_deref(), Some("expr"));
    assert_eq!(declarations[2].name, "local");
    assert_eq!(declarations[2].array_dims.len(), 1);
    assert_integer_expression(
        declarations[2].array_dim_expressions[0]
            .as_ref()
            .expect("array dimension expression"),
        "2",
    );
    assert!(declarations[2].initializer_expression.is_none());
}

#[test]
fn parses_variable_declarations_and_skips_signature_types() {
    let source = source(
        "function f(expr input): expr {\n\
           int aux = 200;\n\
           expr total = input + 1;\n\
           string label = \"main\";\n\
           fe scratch[2][3];\n\
           return total;\n\
         }",
    );

    let declarations =
        parse_variable_declarations(&source).expect("variable declarations should parse");

    assert_eq!(declarations.len(), 4);
    assert_eq!(declarations[0].type_name, "int");
    assert_eq!(declarations[0].name, "aux");
    assert_integer_expression(
        declarations[0]
            .initializer_expression
            .as_ref()
            .expect("integer initializer expression"),
        "200",
    );
    assert_eq!(declarations[1].type_name, "expr");
    assert_eq!(declarations[1].name, "total");
    assert_binary_add_name_plus_one(
        declarations[1]
            .initializer_expression
            .as_ref()
            .expect("expression initializer"),
        "input",
    );
    assert_eq!(declarations[2].type_name, "string");
    assert_eq!(declarations[2].name, "label");
    assert!(matches!(
        declarations[2]
            .initializer_expression
            .as_ref()
            .expect("string initializer")
            .kind,
        ExpressionKind::StringLiteral(ref value) if value == "main"
    ));
    assert_eq!(declarations[3].type_name, "fe");
    assert_eq!(declarations[3].name, "scratch");
    assert_eq!(declarations[3].array_dims.len(), 2);
    assert_integer_expression(
        declarations[3].array_dim_expressions[0]
            .as_ref()
            .expect("first dimension expression"),
        "2",
    );
    assert_integer_expression(
        declarations[3].array_dim_expressions[1]
            .as_ref()
            .expect("second dimension expression"),
        "3",
    );
    assert!(declarations[3].initializer_expression.is_none());
}

#[test]
fn parses_function_statement_declaration_payloads() {
    let source = source(
        "function build(): int {\n\
           const int LIMIT = 4;\n\
           int local = LIMIT;\n\
           col witness local.trace;\n\
           return local;\n\
         }",
    );

    let declarations = parse_function_declarations(&source).expect("functions should parse");
    let statements = &declarations[0].statements;

    assert_eq!(statements.len(), 4);

    match statements[0]
        .declaration
        .as_ref()
        .expect("constant declaration payload")
    {
        FunctionStatementDeclaration::Constant(declaration) => {
            assert_eq!(declaration.kind, ConstantDeclarationKind::Const);
            assert_eq!(declaration.type_name.as_deref(), Some("int"));
            assert_eq!(declaration.name, "LIMIT");
        }
        other => panic!("unexpected declaration payload: {other:?}"),
    }

    match statements[1]
        .declaration
        .as_ref()
        .expect("variable declaration payload")
    {
        FunctionStatementDeclaration::Variable(declaration) => {
            assert_eq!(declaration.type_name, "int");
            assert_eq!(declaration.name, "local");
            assert!(matches!(
                declaration
                    .initializer_expression
                    .as_ref()
                    .expect("initializer expression")
                    .kind,
                ExpressionKind::Name(ref name) if name == "LIMIT"
            ));
        }
        other => panic!("unexpected declaration payload: {other:?}"),
    }

    match statements[2]
        .declaration
        .as_ref()
        .expect("column declaration payload")
    {
        FunctionStatementDeclaration::Column(declaration) => {
            assert_eq!(declaration.kind, ColumnKind::Witness);
            assert_eq!(declaration.items.len(), 1);
            assert_eq!(declaration.items[0].name, "local.trace");
        }
        other => panic!("unexpected declaration payload: {other:?}"),
    }

    assert!(statements[3].declaration.is_none());
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
    let params = declarations[0].params.expect("params should be recorded");
    assert_eq!(
        &source.contents[params.start..params.end],
        "(const int N = 2**22, string label = \"main\")"
    );
    assert_eq!(declarations[0].parameters.len(), 2);
    assert!(declarations[0].parameters[0].is_const);
    assert_eq!(declarations[0].parameters[0].type_name, "int");
    assert_eq!(declarations[0].parameters[0].name, "N");
    assert!(matches!(
        declarations[0].parameters[0]
            .default_expression
            .as_ref()
            .expect("default expression")
            .kind,
        ExpressionKind::Binary {
            op: BinaryOperator::Power,
            ..
        }
    ));
    assert_eq!(declarations[0].parameters[1].type_name, "string");
    assert_eq!(declarations[0].parameters[1].name, "label");
    assert!(matches!(
        declarations[0].parameters[1]
            .default_expression
            .as_ref()
            .expect("default expression")
            .kind,
        ExpressionKind::StringLiteral(ref value) if value == "main"
    ));
    assert_eq!(
        &source.contents[declarations[0].body.start..declarations[0].body.end],
        "{ col witness trace; if (N) { return; } }"
    );
}

#[test]
fn parses_parameterless_air_template_declarations() {
    let source = source("airtemplate Empty { col witness trace; }");

    let declarations =
        parse_air_template_declarations(&source).expect("air templates should parse");

    assert_eq!(declarations.len(), 1);
    assert_eq!(declarations[0].name, "Empty");
    assert!(declarations[0].params.is_none());
    assert!(declarations[0].parameters.is_empty());
    assert_eq!(
        &source.contents[declarations[0].body.start..declarations[0].body.end],
        "{ col witness trace; }"
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
fn parses_air_instances_from_group_bodies() {
    let source = source(
        "airtemplate localAir(int N) { }\n\
         airgroup Main {\n\
           set_max_rows(20);\n\
           virtual Range(id: 7) alias Range7;\n\
           Dma();\n\
           localAir(N: 2**16) alias Local;\n\
           for (int i = 0; i < 2; i++) { Helper(); }\n\
         }",
    );

    let instances = parse_air_instance_declarations(&source).expect("air instances should parse");

    assert_eq!(instances.len(), 3);
    assert_eq!(instances[0].air_group, "Main");
    assert!(instances[0].virtual_instance);
    assert_eq!(instances[0].template, "Range");
    assert_eq!(instances[0].alias.as_deref(), Some("Range7"));
    assert_eq!(
        &source.contents[instances[0].args.start..instances[0].args.end],
        "(id: 7)"
    );
    let args_expressions = instances[0]
        .args_expressions
        .as_ref()
        .expect("air instance args should be recorded");
    assert_eq!(args_expressions.len(), 1);
    assert_eq!(args_expressions[0].name.as_deref(), Some("id"));
    assert_integer_expression(&args_expressions[0].value, "7");
    assert_eq!(instances[1].template, "Dma");
    assert!(!instances[1].virtual_instance);
    assert_eq!(
        &source.contents[instances[1].args.start..instances[1].args.end],
        "()"
    );
    assert!(instances[1]
        .args_expressions
        .as_ref()
        .expect("air instance args should be recorded")
        .is_empty());
    assert_eq!(instances[2].template, "localAir");
    assert_eq!(instances[2].alias.as_deref(), Some("Local"));
    let args_expressions = instances[2]
        .args_expressions
        .as_ref()
        .expect("air instance args should be recorded");
    assert_eq!(args_expressions.len(), 1);
    assert_eq!(args_expressions[0].name.as_deref(), Some("N"));
    assert!(matches!(
        &args_expressions[0].value.kind,
        ExpressionKind::Binary {
            op: BinaryOperator::Power,
            ..
        }
    ));
}

#[test]
fn parses_function_declarations_with_spans_and_visibility() {
    let source = source(
        "airtemplate Main(int N) {\n\
           function sum(int a, int b): int {\n\
             if (a < b) { return b; }\n\
             return a + b;\n\
           }\n\
           private function map_values(expr values[]): expr[] {\n\
             return values;\n\
           }\n\
         }\n\
         public function exported(const string name): string {\n\
           return name;\n\
         }\n\
         function procedure(int value) { col witness local; local === value; }",
    );

    let declarations = parse_function_declarations(&source).expect("functions should parse");

    assert_eq!(declarations.len(), 4);
    assert_eq!(declarations[0].name, "sum");
    assert_eq!(declarations[0].visibility, None);
    assert_eq!(declarations[0].parameters.len(), 2);
    assert!(!declarations[0].parameters[0].is_const);
    assert!(!declarations[0].parameters[0].by_reference);
    assert_eq!(declarations[0].parameters[0].type_name, "int");
    assert_eq!(declarations[0].parameters[0].name, "a");
    assert!(declarations[0].parameters[0].array_dims.is_empty());
    assert_eq!(
        &source.contents[declarations[0].params.start..declarations[0].params.end],
        "(int a, int b)"
    );
    assert_eq!(
        &source.contents[declarations[0]
            .return_type
            .expect("return type should be recorded")
            .start..declarations[0].return_type.unwrap().end],
        "int"
    );
    assert_eq!(
        &source.contents[declarations[0].body.start..declarations[0].body.end],
        "{\nif (a < b) { return b; }\nreturn a + b;\n}"
    );
    assert_eq!(
        declarations[0]
            .statements
            .iter()
            .map(|statement| statement.kind)
            .collect::<Vec<_>>(),
        vec![FunctionStatementKind::If, FunctionStatementKind::Return]
    );

    assert_eq!(declarations[1].name, "map_values");
    assert_eq!(
        declarations[1].visibility,
        Some(FunctionVisibility::Private)
    );
    assert_eq!(declarations[1].parameters.len(), 1);
    assert_eq!(declarations[1].parameters[0].type_name, "expr");
    assert_eq!(declarations[1].parameters[0].name, "values");
    assert_eq!(declarations[1].parameters[0].array_dims.len(), 1);
    assert_eq!(
        &source.contents[declarations[1].params.start..declarations[1].params.end],
        "(expr values[])"
    );
    assert_eq!(
        &source.contents[declarations[1]
            .return_type
            .expect("array return type should be recorded")
            .start..declarations[1].return_type.unwrap().end],
        "expr[]"
    );
    assert_eq!(declarations[1].statements.len(), 1);
    assert_eq!(
        declarations[1].statements[0].kind,
        FunctionStatementKind::Return
    );

    assert_eq!(declarations[2].name, "exported");
    assert_eq!(declarations[2].visibility, Some(FunctionVisibility::Public));
    assert!(declarations[2].parameters[0].is_const);
    assert_eq!(declarations[2].parameters[0].type_name, "string");
    assert_eq!(declarations[2].parameters[0].name, "name");
    assert_eq!(
        &source.contents[declarations[2]
            .return_type
            .expect("string return type should be recorded")
            .start..declarations[2].return_type.unwrap().end],
        "string"
    );

    assert_eq!(declarations[3].name, "procedure");
    assert_eq!(declarations[3].return_type, None);
    assert_eq!(
        declarations[3]
            .statements
            .iter()
            .map(|statement| statement.kind)
            .collect::<Vec<_>>(),
        vec![
            FunctionStatementKind::Declaration,
            FunctionStatementKind::Expression
        ]
    );
}

#[test]
fn parses_function_parameters_with_defaults_and_references() {
    let source = source(
        "function inc (int &k) { return; }\n\
         function sum_1 (const int a = 0, const int b = 0):int { return a + b; }\n\
         function array_prime(expr expressions[], int row_offset = -1): expr[] { return expressions; }",
    );

    let declarations = parse_function_declarations(&source).expect("functions should parse");

    assert_eq!(declarations.len(), 3);
    assert_eq!(declarations[0].parameters.len(), 1);
    assert!(declarations[0].parameters[0].by_reference);
    assert_eq!(declarations[0].parameters[0].type_name, "int");
    assert_eq!(declarations[0].parameters[0].name, "k");

    assert_eq!(declarations[1].parameters.len(), 2);
    assert!(declarations[1].parameters[0].is_const);
    assert_eq!(
        &source.contents[declarations[1].parameters[0]
            .default_value
            .expect("default should be recorded")
            .start
            ..declarations[1].parameters[0].default_value.unwrap().end],
        "0"
    );
    assert!(declarations[1].parameters[1].is_const);
    assert_eq!(
        &source.contents[declarations[1].parameters[1]
            .default_value
            .expect("default should be recorded")
            .start
            ..declarations[1].parameters[1].default_value.unwrap().end],
        "0"
    );
    assert!(matches!(
        &declarations[1].parameters[0]
            .default_expression
            .as_ref()
            .expect("default expression should be recorded")
            .kind,
        ExpressionKind::Integer(value) if value == "0"
    ));
    assert!(matches!(
        &declarations[1].parameters[1]
            .default_expression
            .as_ref()
            .expect("default expression should be recorded")
            .kind,
        ExpressionKind::Integer(value) if value == "0"
    ));

    assert_eq!(declarations[2].parameters.len(), 2);
    assert_eq!(declarations[2].parameters[0].type_name, "expr");
    assert_eq!(declarations[2].parameters[0].array_dims.len(), 1);
    assert_eq!(declarations[2].parameters[1].type_name, "int");
    assert_eq!(declarations[2].parameters[1].name, "row_offset");
    assert_eq!(
        &source.contents[declarations[2].parameters[1]
            .default_value
            .expect("default should be recorded")
            .start
            ..declarations[2].parameters[1].default_value.unwrap().end],
        "-1"
    );
    assert!(matches!(
        &declarations[2].parameters[1]
            .default_expression
            .as_ref()
            .expect("default expression should be recorded")
            .kind,
        ExpressionKind::Unary {
            op: UnaryOperator::Minus,
            ..
        }
    ));
}

#[test]
fn parses_function_body_statement_spans() {
    let source = source(
        "function choose(int value): int {\n\
           if (value == 0) { return 1; } else if (value == 1) { return 2; } else { return 3; }\n\
         }\n\
         function loop_sum(expr values[]): int {\n\
           int total = 0;\n\
           for (int i = 0; i < length(values); ++i) { total += values[i]; }\n\
           return total;\n\
         }",
    );

    let declarations = parse_function_declarations(&source).expect("functions should parse");

    assert_eq!(declarations.len(), 2);
    assert_eq!(declarations[0].statements.len(), 1);
    assert_eq!(
        declarations[0].statements[0].kind,
        FunctionStatementKind::If
    );
    assert_eq!(
        &source.contents[declarations[0].statements[0]
            .header
            .expect("if header should be recorded")
            .start..declarations[0].statements[0].header.unwrap().end],
        "(value == 0)"
    );
    assert_eq!(
        &source.contents[declarations[0].statements[0]
            .body
            .expect("if body should be recorded")
            .start..declarations[0].statements[0].body.unwrap().end],
        "{ return 1; }"
    );
    assert_eq!(
        &source.contents[declarations[0].statements[0].start..declarations[0].statements[0].end],
        "if (value == 0) { return 1; } else if (value == 1) { return 2; } else { return 3; }"
    );

    assert_eq!(
        declarations[1]
            .statements
            .iter()
            .map(|statement| statement.kind)
            .collect::<Vec<_>>(),
        vec![
            FunctionStatementKind::Declaration,
            FunctionStatementKind::For,
            FunctionStatementKind::Return
        ]
    );
    assert_eq!(
        &source.contents[declarations[1].statements[0]
            .value
            .expect("declaration value should be recorded")
            .start..declarations[1].statements[0].value.unwrap().end],
        "int total = 0"
    );
    assert_eq!(
        &source.contents[declarations[1].statements[1]
            .header
            .expect("loop header should be recorded")
            .start..declarations[1].statements[1].header.unwrap().end],
        "(int i = 0; i < length(values); ++i)"
    );
    match declarations[1].statements[1]
        .header_declaration
        .as_ref()
        .expect("loop header declaration should be recorded")
    {
        FunctionStatementDeclaration::Variable(declaration) => {
            assert_eq!(declaration.type_name, "int");
            assert_eq!(declaration.name, "i");
            assert_integer_expression(
                declaration
                    .initializer_expression
                    .as_ref()
                    .expect("loop initializer should be parsed"),
                "0",
            );
        }
        other => panic!("unexpected declaration payload: {other:?}"),
    }
    assert_eq!(
        &source.contents[declarations[1].statements[1]
            .body
            .expect("loop body should be recorded")
            .start..declarations[1].statements[1].body.unwrap().end],
        "{ total += values[i]; }"
    );
    assert_eq!(
        &source.contents[declarations[1].statements[2]
            .value
            .expect("return value should be recorded")
            .start..declarations[1].statements[2].value.unwrap().end],
        "total"
    );
}

#[test]
fn parses_function_statement_expression_trees() {
    let source = source(
        "function calc(int value): int {\n\
           if (value == 0) { return value + 1; }\n\
           value = value * (value + 1);\n\
           return value + 2;\n\
         }",
    );

    let declarations = parse_function_declarations(&source).expect("functions should parse");

    assert_eq!(declarations.len(), 1);
    assert_eq!(declarations[0].statements.len(), 3);

    let if_stmt = &declarations[0].statements[0];
    let header = if_stmt
        .header_expression
        .as_ref()
        .expect("if header should be parsed");
    assert!(matches!(
        &header.kind,
        ExpressionKind::Group(inner) if matches!(
            &inner.kind,
            ExpressionKind::Binary {
                op: BinaryOperator::EqualEqual,
                ..
            }
        )
    ));

    let assign_stmt = &declarations[0].statements[1];
    assert!(matches!(
        &assign_stmt
            .value_expression
            .as_ref()
            .expect("assignment should be parsed")
            .kind,
        ExpressionKind::Binary {
            op: BinaryOperator::Assign,
            ..
        }
    ));

    let final_return = &declarations[0].statements[2];
    assert!(matches!(
        &final_return
            .value_expression
            .as_ref()
            .expect("return should be parsed")
            .kind,
        ExpressionKind::Binary {
            op: BinaryOperator::Add,
            ..
        }
    ));
}

#[test]
fn rejects_unclosed_function_body() {
    let source = source("function sum(int a, int b): int { return a + b;");

    let error = parse_function_declarations(&source).expect_err("body should close");

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
    assert_integer_expression(
        declarations[0].items[0].array_dim_expressions[0]
            .as_ref()
            .expect("array dimension expression should be recorded"),
        "2",
    );
    assert_eq!(declarations[0].items[1].name, "local");
    assert_eq!(declarations[0].items[1].array_dims.len(), 2);
    assert_eq!(
        &source.contents[declarations[0].items[1].array_dims[0].start
            ..declarations[0].items[1].array_dims[0].end],
        "[1]"
    );
    assert_integer_expression(
        declarations[0].items[1].array_dim_expressions[0]
            .as_ref()
            .expect("array dimension expression should be recorded"),
        "1",
    );
    assert_eq!(
        &source.contents[declarations[0].items[1].array_dims[1].start
            ..declarations[0].items[1].array_dims[1].end],
        "[]"
    );
    assert!(declarations[0].items[1].array_dim_expressions[1].is_none());
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
    let args_expressions = declarations[0].features[0]
        .args_expressions
        .as_ref()
        .expect("feature args should be recorded");
    assert_eq!(args_expressions.len(), 1);
    let ExpressionKind::Binary {
        op: BinaryOperator::Add,
        left,
        right,
    } = &args_expressions[0].kind
    else {
        panic!("stage argument should be an addition");
    };
    assert_integer_expression(left.as_ref(), "1");
    assert!(
        matches!(&right.kind, ExpressionKind::Group(inner) if matches!(&inner.kind, ExpressionKind::Integer(value) if value == "2"))
    );
    assert_eq!(declarations[0].features[1].name, "virtual");
    assert_eq!(
        &source.contents
            [declarations[0].features[1].args.start..declarations[0].features[1].args.end],
        "(foo(bar))"
    );
    let args_expressions = declarations[0].features[1]
        .args_expressions
        .as_ref()
        .expect("feature args should be recorded");
    assert_eq!(args_expressions.len(), 1);
    assert!(matches!(
        &args_expressions[0].kind,
        ExpressionKind::Call { callee, args }
            if matches!(&callee.kind, ExpressionKind::Name(name) if name == "foo")
                && args.len() == 1
                && matches!(&args[0].value.kind, ExpressionKind::Name(name) if name == "bar")
    ));
    assert_eq!(declarations[0].items.len(), 2);
    assert_eq!(declarations[0].items[0].name, "air.main");
    assert_eq!(declarations[0].items[1].name, "local");
}

#[test]
fn parses_fixed_column_initializer_spans() {
    let source = source("col fixed stage(3) x = bar' + 1;");

    let declarations = parse_column_declarations(&source).expect("columns should parse");

    assert_eq!(declarations.len(), 1);
    assert_eq!(declarations[0].kind, ColumnKind::Fixed);
    assert_eq!(declarations[0].features.len(), 1);
    assert_eq!(declarations[0].features[0].name, "stage");
    assert_eq!(declarations[0].items.len(), 1);
    assert_eq!(declarations[0].items[0].name, "x");
    let initializer = declarations[0]
        .initializer
        .as_ref()
        .expect("initializer should be recorded");
    assert_eq!(initializer.kind, ColumnInitializerKind::Expression);
    assert_eq!(
        &source.contents[initializer.span.start..initializer.span.end],
        "bar' + 1"
    );
    assert_row_offset_add_one(
        initializer
            .expression
            .as_ref()
            .expect("initializer expression should be recorded"),
        "bar",
        "1",
    );
}

#[test]
fn parses_sequence_initializer_spans() {
    let source = source("col fixed x = [foo(bar), baz[1]];");

    let declarations = parse_column_declarations(&source).expect("columns should parse");

    assert_eq!(declarations.len(), 1);
    let initializer = declarations[0]
        .initializer
        .as_ref()
        .expect("initializer should be recorded");
    assert_eq!(initializer.kind, ColumnInitializerKind::Sequence);
    assert_eq!(
        &source.contents[initializer.span.start..initializer.span.end],
        "[foo(bar), baz[1]]"
    );
    assert!(initializer.expression.is_none());
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
    let source = source("airgroupval stage(5) default(bar' + 1) aggregate(sum) group.a[2], local;");

    let declarations =
        parse_air_group_value_declarations(&source).expect("group values should parse");

    assert_eq!(declarations.len(), 1);
    assert_eq!(declarations[0].stage, 5);
    assert_eq!(
        &source.contents[declarations[0].default_value.expect("default").start
            ..declarations[0].default_value.expect("default").end],
        "(bar' + 1)"
    );
    assert_row_offset_add_one(
        declarations[0]
            .default_expression
            .as_ref()
            .expect("default expression should be recorded"),
        "bar",
        "1",
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
    assert!(declarations[0].default_expression.is_none());
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
    let source = source("public air.main[2], local;\npublic scalar = lane'512 + 1;");

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
    assert!(declarations[0].initializer_expression.is_none());
    assert_eq!(declarations[1].items.len(), 1);
    assert_eq!(declarations[1].items[0].name, "scalar");
    let initializer = declarations[1].initializer.expect("initializer");
    assert_eq!(
        &source.contents[initializer.start..initializer.end],
        "lane'512 + 1"
    );
    assert_row_offset_add_one(
        declarations[1]
            .initializer_expression
            .as_ref()
            .expect("initializer expression should be recorded"),
        "lane",
        "512",
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
        "publictable aggregate(sum, fold, bar' + 1, baz[2]) table[cols + 1][rows];\n\
         publictable aggregate(sum, fold) other[rows' + 1][cols + 1];",
    );

    let declarations =
        parse_public_table_declarations(&source).expect("public table declarations should parse");

    assert_eq!(declarations.len(), 2);
    assert_eq!(declarations[0].aggregate_type, "sum");
    assert_eq!(declarations[0].aggregate_function, "fold");
    assert_eq!(declarations[0].name, "table");
    let args = declarations[0].args.expect("args");
    assert_eq!(&source.contents[args.start..args.end], "bar' + 1, baz[2]");
    let args_expressions = declarations[0]
        .args_expressions
        .as_ref()
        .expect("args expressions");
    assert_eq!(args_expressions.len(), 2);
    assert_row_offset_add_one(&args_expressions[0], "bar", "1");
    assert!(matches!(
        &args_expressions[1].kind,
        ExpressionKind::Index { target, index } if matches!(&target.kind, ExpressionKind::Name(name) if name == "baz")
            && matches!(&index.kind, ExpressionKind::Integer(value) if value == "2")
    ));
    assert_eq!(
        &source.contents[declarations[0].cols.start..declarations[0].cols.end],
        "[cols + 1]"
    );
    assert_binary_add_name_plus_one(
        declarations[0]
            .cols_expression
            .as_ref()
            .expect("cols expression"),
        "cols",
    );
    assert_eq!(
        &source.contents[declarations[0].rows.start..declarations[0].rows.end],
        "[rows]"
    );
    assert!(matches!(
        declarations[0]
            .rows_expression
            .as_ref()
            .expect("rows expression")
            .kind,
        ExpressionKind::Name(ref name) if name == "rows"
    ));
    assert_eq!(declarations[1].aggregate_type, "sum");
    assert_eq!(declarations[1].aggregate_function, "fold");
    assert_eq!(declarations[1].name, "other");
    assert_eq!(declarations[1].args, None);
    assert!(declarations[1].args_expressions.is_none());
    assert_eq!(
        &source.contents[declarations[1].cols.start..declarations[1].cols.end],
        "[rows' + 1]"
    );
    assert_row_offset_add_one(
        declarations[1]
            .cols_expression
            .as_ref()
            .expect("cols expression"),
        "rows",
        "1",
    );
    assert_eq!(
        &source.contents[declarations[1].rows.start..declarations[1].rows.end],
        "[cols + 1]"
    );
    assert_binary_add_name_plus_one(
        declarations[1]
            .rows_expression
            .as_ref()
            .expect("rows expression"),
        "cols",
    );
}
