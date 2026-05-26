use std::cell::RefCell;
use std::collections::BTreeMap;
use std::collections::BTreeSet;

use lzvm_artifacts::setup_info::{StarkStruct, UnitSetupInfo};
use lzvm_pil::{
    BinaryOperator, Expression, ExpressionKind, FixedFileTemplateValue, SourceFile, SourceGraph,
    SourceProgram, SourceProgramModule,
};

use crate::{
    source_range_check_hints::SourceRangeCheckIds, source_scalar_slots::SourceScalarSlots,
    source_static_values::SourceTemplateConstantValueCache,
};

use super::alias_bindings::source_resolve_compound_assignment_alias_binding;
use super::*;

fn source_program() -> (SourceProgram, SourceProgramModule) {
    let source = SourceFile {
        contents: String::new(),
        file_dir: ".".into(),
        full_path: "test.pil".into(),
        source_name: "test.pil".to_owned(),
    };
    let module = SourceProgramModule {
        source_name: source.source_name.clone(),
        source,
        pragmas: Vec::new(),
        fixed_file_pragmas: Vec::new(),
        air_template_fixed_file_pragmas: Vec::new(),
        includes: Vec::new(),
        uses: Vec::new(),
        containers: Vec::new(),
        constants: Vec::new(),
        variables: Vec::new(),
        air_templates: Vec::new(),
        air_groups: Vec::new(),
        air_instances: Vec::new(),
        functions: Vec::new(),
        columns: Vec::new(),
        values: Vec::new(),
        air_group_values: Vec::new(),
        commits: Vec::new(),
        publics: Vec::new(),
        public_tables: Vec::new(),
    };
    let program = SourceProgram {
        graph: SourceGraph {
            sources: vec![module.source.clone()],
            edges: Vec::new(),
        },
        modules: vec![module.clone()],
    };
    (program, module)
}

fn name(value: &str) -> Expression {
    Expression {
        kind: ExpressionKind::Name(value.to_owned()),
        source_name: "test.pil".to_owned(),
        start: 0,
        end: value.len(),
    }
}

fn integer(value: &str) -> Expression {
    Expression {
        kind: ExpressionKind::Integer(value.to_owned()),
        source_name: "test.pil".to_owned(),
        start: 0,
        end: value.len(),
    }
}

fn add(left: Expression, right: Expression) -> Expression {
    Expression {
        kind: ExpressionKind::Binary {
            op: BinaryOperator::Add,
            left: Box::new(left),
            right: Box::new(right),
        },
        source_name: "test.pil".to_owned(),
        start: 0,
        end: 0,
    }
}

fn empty_setup_info() -> UnitSetupInfo {
    UnitSetupInfo {
        n_stages: 0,
        n_constants: 0,
        constant_columns: Vec::new(),
        n_publics: Some(0),
        n_constraints: Some(0),
        q_degree: 0,
        opening_points: Vec::new(),
        section_widths: BTreeMap::new(),
        challenge_count: 0,
        eval_count: 0,
        evaluation_map: Vec::new(),
        boundaries: Vec::new(),
        commitment_columns: Vec::new(),
        unit_value_map: Vec::new(),
        group_value_map: Vec::new(),
        stark: StarkStruct {
            n_bits: 0,
            n_bits_ext: 0,
            n_queries: 0,
            steps: Vec::new(),
            hash_commits: false,
            last_level_verification: 0,
            pow_bits: 0,
            merkle_tree_arity: 0,
            verification_hash_type: None,
            transcript_arity: None,
            merkle_tree_custom: None,
        },
    }
}

#[test]
fn source_expression_precheck_skips_unresolvable_column_expressions() {
    let (program, module) = source_program();
    let values = BTreeMap::new();
    let alias_scope = SourceExpressionAliasScope::default();
    let expression = add(name("a"), add(name("b"), name("c")));

    assert!(!source_expression_may_resolve(
        &program,
        &module,
        &expression,
        &values,
        &alias_scope,
        true,
        true,
    ));
}

#[test]
fn source_expression_precheck_detects_aliases_and_static_values() {
    let (program, module) = source_program();
    let mut values = BTreeMap::new();
    values.insert("n".to_owned(), FixedFileTemplateValue::Integer(7));
    let mut alias_scope = SourceExpressionAliasScope::default();
    alias_scope
        .expressions_mut()
        .insert("x".to_owned(), name("y"));

    assert!(source_expression_may_resolve(
        &program,
        &module,
        &add(name("a"), name("x")),
        &values,
        &alias_scope,
        true,
        true,
    ));
    assert!(source_expression_may_resolve(
        &program,
        &module,
        &add(name("a"), name("n")),
        &values,
        &SourceExpressionAliasScope::default(),
        true,
        true,
    ));
}

#[test]
fn source_expression_precheck_detects_array_alias_indices() {
    let (program, module) = source_program();
    let values = BTreeMap::new();
    let mut alias_scope = SourceExpressionAliasScope::default();
    alias_scope.expression_arrays_mut().insert(
        "xs".to_owned(),
        SourceExpressionArrayAlias::Values(vec![name("a")]),
    );
    let expression = Expression {
        kind: ExpressionKind::Index {
            target: Box::new(name("xs")),
            index: Box::new(integer("0")),
        },
        source_name: "test.pil".to_owned(),
        start: 0,
        end: 0,
    };

    assert!(source_expression_may_resolve(
        &program,
        &module,
        &expression,
        &values,
        &alias_scope,
        true,
        true,
    ));
}

#[test]
fn source_static_value_precheck_rejects_column_arithmetic() {
    let (program, _) = source_program();
    let values = BTreeMap::new();

    assert!(!source_expression_static_value_can_apply(
        &program,
        &add(name("column"), integer("1")),
        &values,
    ));
}

#[test]
fn source_static_value_precheck_detects_static_arithmetic() {
    let (program, _) = source_program();
    let mut values = BTreeMap::new();
    values.insert("n".to_owned(), FixedFileTemplateValue::Integer(7));

    assert!(source_expression_static_value_can_apply(
        &program,
        &add(name("n"), integer("1")),
        &values,
    ));
}

#[test]
fn source_compound_alias_resolution_keeps_left_snapshot() {
    let (program, module) = source_program();
    let tokens = Vec::new();
    let setup = empty_setup_info();
    let scalar_slots = SourceScalarSlots::from_setup(&setup, &[], &[], &[]).unwrap();
    let opening_points = Vec::new();
    let fixed_columns = BTreeSet::new();
    let range_checks = RefCell::new(SourceRangeCheckIds::default());
    let active_templates = BTreeSet::new();
    let constant_values = BTreeMap::new();
    let template_values = SourceTemplateConstantValueCache::new();
    let context = SourceTemplateLoweringContext {
        program: &program,
        module: &module,
        tokens: &tokens,
        scalar_slots: &scalar_slots,
        opening_points: &opening_points,
        fixed_columns: &fixed_columns,
        range_checks: &range_checks,
        active_templates: &active_templates,
        constant_values: &constant_values,
        template_values: &template_values,
        final_air_calls_enabled: false,
    };
    let mut values = BTreeMap::new();
    values.insert("n".to_owned(), FixedFileTemplateValue::Integer(7));
    let mut alias_scope = SourceExpressionAliasScope::default();
    alias_scope
        .expressions_mut()
        .insert("old".to_owned(), name("new"));
    alias_scope
        .expressions_mut()
        .insert("acc".to_owned(), add(name("old"), name("n")));
    let mut body_cache = SourceControlBodyCache::default();
    let mut call_stack = BTreeSet::new();

    assert!(source_resolve_compound_assignment_alias_binding(
        &context,
        "acc",
        &values,
        &mut alias_scope,
        &mut body_cache,
        &mut call_stack,
        BinaryOperator::Add,
    ));

    let expression = alias_scope.expressions.get("acc").unwrap();
    let ExpressionKind::Binary { left, right, .. } = &expression.kind else {
        panic!("compound alias should remain a binary expression");
    };
    assert!(matches!(&left.kind, ExpressionKind::Name(name) if name == "old"));
    assert!(matches!(&right.kind, ExpressionKind::Integer(value) if value == "7"));
}
