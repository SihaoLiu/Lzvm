use super::*;
use std::cell::Cell;

#[test]
fn regular_constraint_buffer_resolution_is_per_source_not_per_row() {
    let program = ConstraintProgram {
        entries: vec![ConstraintEntry {
            stage: 1,
            destination_dimension: 1,
            destination_id: 0,
            first_row: 0,
            last_row: 8,
            temp1_count: 1,
            temp3_count: 0,
            ops_count: 1,
            ops_offset: 0,
            args_count: 8,
            args_offset: 0,
            intermediate: false,
            source_line: "layout resolution residual".to_owned(),
        }],
        ops: vec![0],
        args: vec![1, 0, 0, 0, 0, 8, 0, 0],
        numbers: vec![3],
    };
    let fixed = vec![Felt::from_u64(3); 8];

    BUFFER_RESOLVE_COUNT.with(|count| count.set(0));
    CACHED_SOURCE_COUNT.with(|count| count.set(0));
    let results = evaluate_regular_constraints(
        &program,
        RegularConstraintInputs {
            domain_size: 8,
            stage_count: 1,
            fixed_columns: RegularColumnMatrix {
                column_count: 1,
                values: &fixed,
            },
            opening_point_offsets: &[0],
            ..RegularConstraintInputs::default()
        },
    )
    .expect("regular constraint should evaluate");

    assert_eq!(results[0].invalid_rows, Vec::new());
    assert!(
        BUFFER_RESOLVE_COUNT.with(Cell::get) <= 2,
        "buffer layout should be resolved once per operation source"
    );
    assert!(
        CACHED_SOURCE_COUNT.with(Cell::get) <= 2,
        "decoded sources should be read without checking the cache on every row"
    );
}

#[test]
fn source_row_offset_fallback_wraps_out_of_range_rows() {
    assert_eq!(source_row_with_offset(5, 0, 3).expect("row should wrap"), 2);
    assert_eq!(source_row_with_offset(5, 1, 3).expect("row should wrap"), 0);
}

#[test]
fn prepared_source_row_offset_matches_fallback_wrap() {
    assert_eq!(source_row_with_offset_prepared(5, 0, 3), 2);
    assert_eq!(source_row_with_offset_prepared(5, 1, 3), 0);
    assert_eq!(source_row_with_offset_prepared(1, 9, 8), 2);
}

#[test]
fn base_only_regular_constraints_use_prepared_rows_after_first_row() {
    let program = ConstraintProgram {
        entries: vec![ConstraintEntry {
            stage: 1,
            destination_dimension: 1,
            destination_id: 0,
            first_row: 0,
            last_row: 8,
            temp1_count: 2,
            temp3_count: 0,
            ops_count: 2,
            ops_offset: 0,
            args_count: 16,
            args_offset: 0,
            intermediate: false,
            source_line: "base-only prepared rows".to_owned(),
        }],
        ops: vec![0, 0],
        args: vec![2, 1, 0, 0, 0, 8, 0, 0, 1, 0, 5, 1, 0, 8, 1, 0],
        numbers: vec![3, 9],
    };
    let fixed = vec![Felt::from_u64(3); 8];

    BASE_ONLY_PREPARED_ROW_COUNT.with(|count| count.set(0));
    BASE_ONLY_TMP1_CLEAR_COUNT.with(|count| count.set(0));
    BASE_ONLY_TMP3_CLEAR_COUNT.with(|count| count.set(0));
    let results = evaluate_regular_constraints(
        &program,
        RegularConstraintInputs {
            domain_size: 8,
            stage_count: 1,
            fixed_columns: RegularColumnMatrix {
                column_count: 1,
                values: &fixed,
            },
            opening_point_offsets: &[0],
            ..RegularConstraintInputs::default()
        },
    )
    .expect("regular constraint should evaluate");

    assert_eq!(results[0].invalid_rows, Vec::new());
    assert_eq!(BASE_ONLY_PREPARED_ROW_COUNT.with(Cell::get), 7);
    assert_eq!(BASE_ONLY_TMP1_CLEAR_COUNT.with(Cell::get), 0);
    assert_eq!(BASE_ONLY_TMP3_CLEAR_COUNT.with(Cell::get), 0);
}

#[test]
fn base_only_regular_constraints_clear_tmp3_for_tmp3_reads() {
    let program = ConstraintProgram {
        entries: vec![ConstraintEntry {
            stage: 1,
            destination_dimension: 1,
            destination_id: 0,
            first_row: 0,
            last_row: 8,
            temp1_count: 1,
            temp3_count: 1,
            ops_count: 1,
            ops_offset: 0,
            args_count: 8,
            args_offset: 0,
            intermediate: false,
            source_line: "base-only tmp3 clear".to_owned(),
        }],
        ops: vec![0],
        args: vec![0, 0, 6, 0, 0, 8, 0, 0],
        numbers: vec![0],
    };

    BASE_ONLY_PREPARED_ROW_COUNT.with(|count| count.set(0));
    BASE_ONLY_TMP3_CLEAR_COUNT.with(|count| count.set(0));
    let results = evaluate_regular_constraints(
        &program,
        RegularConstraintInputs {
            domain_size: 8,
            stage_count: 1,
            opening_point_offsets: &[0],
            ..RegularConstraintInputs::default()
        },
    )
    .expect("regular constraint should evaluate");

    assert_eq!(results[0].invalid_rows, Vec::new());
    assert_eq!(BASE_ONLY_PREPARED_ROW_COUNT.with(Cell::get), 7);
    assert_eq!(BASE_ONLY_TMP3_CLEAR_COUNT.with(Cell::get), 7);
}

#[test]
fn prepared_base_tmp3_reads_discard_stale_scratch_values() {
    let entry = ConstraintEntry {
        stage: 1,
        destination_dimension: 1,
        destination_id: 0,
        first_row: 0,
        last_row: 1,
        temp1_count: 1,
        temp3_count: 1,
        ops_count: 0,
        ops_offset: 0,
        args_count: 0,
        args_offset: 0,
        intermediate: false,
        source_line: "base-only tmp3 stale scratch".to_owned(),
    };
    let program = PreparedBaseProgram {
        operations: vec![PreparedBaseOperation::Generic(
            PreparedGenericBaseOperation {
                kind: 0,
                destination_offset: 0,
                src0: PreparedBaseSource::Tmp3(0),
                src1: PreparedBaseSource::Constant(Felt::ZERO),
            },
        )],
        clear_tmp1: false,
        clear_tmp3: true,
    };
    let mut tmp1 = [Felt::ZERO];
    let mut tmp3 = [Felt::from_u64(9), Felt::from_u64(10), Felt::from_u64(11)];

    let value = evaluate_prepared_base_row(0, &entry, &program, 1, &mut tmp1, &mut tmp3);

    assert_eq!(value, Felt::ZERO);
}

#[test]
fn base_only_constant_pairs_precompute_for_prepared_rows() {
    for (kind, left, right, expected) in [(0, 2, 5, 7), (1, 9, 4, 5), (2, 3, 4, 12), (3, 2, 9, 7)] {
        let program = ConstraintProgram {
            entries: vec![ConstraintEntry {
                stage: 1,
                destination_dimension: 1,
                destination_id: 1,
                first_row: 0,
                last_row: 8,
                temp1_count: 2,
                temp3_count: 0,
                ops_count: 2,
                ops_offset: 0,
                args_count: 16,
                args_offset: 0,
                intermediate: false,
                source_line: "base-only constant pair".to_owned(),
            }],
            ops: vec![0, 0],
            args: vec![kind, 0, 8, 0, 0, 8, 1, 0, 1, 1, 5, 0, 0, 8, 2, 0],
            numbers: vec![left, right, expected],
        };

        BASE_ONLY_PREPARED_ROW_COUNT.with(|count| count.set(0));
        BASE_ONLY_KIND_DISPATCH_COUNT.with(|count| count.set(0));
        let results = evaluate_regular_constraints(
            &program,
            RegularConstraintInputs {
                domain_size: 8,
                stage_count: 1,
                ..RegularConstraintInputs::default()
            },
        )
        .expect("regular constraint should evaluate");

        assert_eq!(results[0].invalid_rows, Vec::new(), "kind {kind}");
        assert_eq!(
            BASE_ONLY_PREPARED_ROW_COUNT.with(Cell::get),
            7,
            "kind {kind}"
        );
        assert_eq!(
            BASE_ONLY_KIND_DISPATCH_COUNT.with(Cell::get),
            0,
            "kind {kind}"
        );
    }
}

#[test]
fn base_only_matrix_pairs_specialize_for_prepared_rows() {
    for (kind, left, right, expected) in [(0, 2, 5, 7), (1, 9, 4, 5), (2, 3, 4, 12), (3, 2, 9, 7)] {
        let program = ConstraintProgram {
            entries: vec![ConstraintEntry {
                stage: 1,
                destination_dimension: 1,
                destination_id: 1,
                first_row: 0,
                last_row: 8,
                temp1_count: 2,
                temp3_count: 0,
                ops_count: 2,
                ops_offset: 0,
                args_count: 16,
                args_offset: 0,
                intermediate: false,
                source_line: "base-only matrix pair".to_owned(),
            }],
            ops: vec![0, 0],
            args: vec![kind, 0, 1, 0, 0, 1, 1, 0, 1, 1, 5, 0, 0, 8, 0, 0],
            numbers: vec![expected],
        };
        let row_values: Vec<_> = (0..8)
            .flat_map(|_| [Felt::from_u64(left), Felt::from_u64(right)])
            .collect();
        let stage_columns = [RegularStageColumns {
            stage_index: 1,
            column_count: 2,
            values: &row_values,
        }];

        BASE_ONLY_PREPARED_ROW_COUNT.with(|count| count.set(0));
        BASE_ONLY_KIND_DISPATCH_COUNT.with(|count| count.set(0));
        let results = evaluate_regular_constraints(
            &program,
            RegularConstraintInputs {
                domain_size: 8,
                stage_count: 1,
                stage_columns: &stage_columns,
                opening_point_offsets: &[0],
                ..RegularConstraintInputs::default()
            },
        )
        .expect("regular constraint should evaluate");

        assert_eq!(results[0].invalid_rows, Vec::new(), "kind {kind}");
        assert_eq!(
            BASE_ONLY_PREPARED_ROW_COUNT.with(Cell::get),
            7,
            "kind {kind}"
        );
        assert_eq!(
            BASE_ONLY_KIND_DISPATCH_COUNT.with(Cell::get),
            0,
            "kind {kind}"
        );
    }
}

#[test]
fn base_only_matrix_pairs_preserve_independent_row_offsets() {
    let program = ConstraintProgram {
        entries: vec![ConstraintEntry {
            stage: 1,
            destination_dimension: 1,
            destination_id: 0,
            first_row: 0,
            last_row: 4,
            temp1_count: 1,
            temp3_count: 0,
            ops_count: 1,
            ops_offset: 0,
            args_count: 8,
            args_offset: 0,
            intermediate: false,
            source_line: "base-only matrix offset pair".to_owned(),
        }],
        ops: vec![0],
        args: vec![1, 0, 1, 0, 1, 1, 1, 0],
        numbers: Vec::new(),
    };
    let row_values = [10, 11, 11, 12, 12, 13, 13, 10].map(Felt::from_u64);
    let stage_columns = [RegularStageColumns {
        stage_index: 1,
        column_count: 2,
        values: &row_values,
    }];

    BASE_ONLY_PREPARED_ROW_COUNT.with(|count| count.set(0));
    BASE_ONLY_KIND_DISPATCH_COUNT.with(|count| count.set(0));
    let results = evaluate_regular_constraints(
        &program,
        RegularConstraintInputs {
            domain_size: 4,
            stage_count: 1,
            stage_columns: &stage_columns,
            opening_point_offsets: &[0, 1],
            ..RegularConstraintInputs::default()
        },
    )
    .expect("regular constraint should evaluate");

    assert_eq!(results[0].invalid_rows, Vec::new());
    assert_eq!(BASE_ONLY_PREPARED_ROW_COUNT.with(Cell::get), 3);
    assert_eq!(BASE_ONLY_KIND_DISPATCH_COUNT.with(Cell::get), 0);
}

#[test]
fn base_only_matrix_zero_offsets_skip_prepared_row_offset() {
    let program = ConstraintProgram {
        entries: vec![ConstraintEntry {
            stage: 1,
            destination_dimension: 1,
            destination_id: 0,
            first_row: 0,
            last_row: 8,
            temp1_count: 1,
            temp3_count: 0,
            ops_count: 1,
            ops_offset: 0,
            args_count: 8,
            args_offset: 0,
            intermediate: false,
            source_line: "base-only matrix zero offset".to_owned(),
        }],
        ops: vec![0],
        args: vec![1, 0, 1, 0, 0, 8, 0, 0],
        numbers: vec![3],
    };
    let row_values = vec![Felt::from_u64(3); 8];
    let stage_columns = [RegularStageColumns {
        stage_index: 1,
        column_count: 1,
        values: &row_values,
    }];

    BASE_ONLY_PREPARED_ROW_COUNT.with(|count| count.set(0));
    PREPARED_SOURCE_ROW_OFFSET_COUNT.with(|count| count.set(0));
    let results = evaluate_regular_constraints(
        &program,
        RegularConstraintInputs {
            domain_size: 8,
            stage_count: 1,
            stage_columns: &stage_columns,
            opening_point_offsets: &[0],
            ..RegularConstraintInputs::default()
        },
    )
    .expect("regular constraint should evaluate");

    assert_eq!(results[0].invalid_rows, Vec::new());
    assert_eq!(BASE_ONLY_PREPARED_ROW_COUNT.with(Cell::get), 7);
    assert_eq!(PREPARED_SOURCE_ROW_OFFSET_COUNT.with(Cell::get), 0);
}

#[test]
fn base_only_direct_common_source_pairs_preserve_row_semantics() {
    let entry = ConstraintEntry {
        stage: 1,
        destination_dimension: 1,
        destination_id: 2,
        first_row: 0,
        last_row: 4,
        temp1_count: 3,
        temp3_count: 0,
        ops_count: 4,
        ops_offset: 0,
        args_count: 32,
        args_offset: 0,
        intermediate: false,
        source_line: "base-only direct sources".to_owned(),
    };
    let program = ConstraintProgram {
        entries: vec![entry.clone()],
        ops: vec![0, 0, 0, 0],
        args: vec![
            1, 0, 0, 0, 1, 8, 0, 0, 3, 1, 0, 0, 1, 5, 0, 0, 3, 2, 5, 1, 0, 8, 0, 0, 1, 2, 5, 2, 0,
            5, 2, 0,
        ],
        numbers: vec![5],
    };
    let fixed = [11, 12, 13, 14].map(Felt::from_u64);
    let inputs = RegularConstraintInputs {
        domain_size: 4,
        stage_count: 1,
        fixed_columns: RegularColumnMatrix {
            column_count: 1,
            values: &fixed,
        },
        opening_point_offsets: &[0, 1],
        ..RegularConstraintInputs::default()
    };
    let ops = entry_ops(0, &entry, &program).expect("operation span should read");
    let args = entry_args(0, &entry, &program).expect("argument span should read");
    let mut context = RowEvaluationContext {
        constraint_index: 0,
        entry: &entry,
        ops,
        args,
        operations: vec![None; ops.len()],
        sources: vec![[None, None]; ops.len()],
        program: &program,
        inputs,
        layout: BufferLayout::new(inputs),
    };
    let prepared = prepared_operations(&mut context).expect("operations should prepare");
    let base_program = prepared_base_operations(&entry, &prepared, &program, inputs)
        .expect("base program should prepare")
        .expect("all operations are base operations");

    assert!(matches!(
        base_program.operations.as_slice(),
        [
            PreparedBaseOperation::MatrixConstantSub(_),
            PreparedBaseOperation::MatrixTmp1RSub(_),
            PreparedBaseOperation::Tmp1ConstantRSub(_),
            PreparedBaseOperation::Tmp1Tmp1Sub(_),
        ]
    ));

    BASE_ONLY_KIND_DISPATCH_COUNT.with(|count| count.set(0));
    let results =
        evaluate_regular_constraints(&program, inputs).expect("regular constraint should evaluate");
    assert_eq!(results[0].invalid_rows, Vec::new());
    assert_eq!(BASE_ONLY_KIND_DISPATCH_COUNT.with(Cell::get), 0);
}

#[test]
fn base_only_direct_common_source_pairs_specialize_supported_kinds() {
    let matrix = PreparedBaseMatrix {
        values: &[],
        column_count: 1,
        column: 0,
        row_offset: None,
    };
    let cases = [
        (
            prepared_base_operation(
                0,
                0,
                PreparedBaseSource::Matrix(matrix),
                PreparedBaseSource::Tmp1(0),
            ),
            "matrix_tmp1_add",
        ),
        (
            prepared_base_operation(
                1,
                0,
                PreparedBaseSource::Matrix(matrix),
                PreparedBaseSource::Tmp1(0),
            ),
            "matrix_tmp1_sub",
        ),
        (
            prepared_base_operation(
                2,
                0,
                PreparedBaseSource::Matrix(matrix),
                PreparedBaseSource::Tmp1(0),
            ),
            "matrix_tmp1_mul",
        ),
        (
            prepared_base_operation(
                3,
                0,
                PreparedBaseSource::Matrix(matrix),
                PreparedBaseSource::Tmp1(0),
            ),
            "matrix_tmp1_rsub",
        ),
        (
            prepared_base_operation(
                0,
                0,
                PreparedBaseSource::Matrix(matrix),
                PreparedBaseSource::Matrix(matrix),
            ),
            "matrix_matrix_add",
        ),
        (
            prepared_base_operation(
                1,
                0,
                PreparedBaseSource::Matrix(matrix),
                PreparedBaseSource::Matrix(matrix),
            ),
            "matrix_matrix_sub",
        ),
        (
            prepared_base_operation(
                2,
                0,
                PreparedBaseSource::Matrix(matrix),
                PreparedBaseSource::Matrix(matrix),
            ),
            "matrix_matrix_mul",
        ),
        (
            prepared_base_operation(
                3,
                0,
                PreparedBaseSource::Matrix(matrix),
                PreparedBaseSource::Matrix(matrix),
            ),
            "matrix_matrix_rsub",
        ),
        (
            prepared_base_operation(
                0,
                0,
                PreparedBaseSource::Tmp1(0),
                PreparedBaseSource::Tmp1(1),
            ),
            "tmp1_tmp1_add",
        ),
        (
            prepared_base_operation(
                1,
                0,
                PreparedBaseSource::Tmp1(0),
                PreparedBaseSource::Tmp1(1),
            ),
            "tmp1_tmp1_sub",
        ),
        (
            prepared_base_operation(
                2,
                0,
                PreparedBaseSource::Tmp1(0),
                PreparedBaseSource::Tmp1(1),
            ),
            "tmp1_tmp1_mul",
        ),
        (
            prepared_base_operation(
                3,
                0,
                PreparedBaseSource::Tmp1(0),
                PreparedBaseSource::Tmp1(1),
            ),
            "tmp1_tmp1_rsub",
        ),
        (
            prepared_base_operation(
                0,
                0,
                PreparedBaseSource::Matrix(matrix),
                PreparedBaseSource::Constant(Felt::ONE),
            ),
            "matrix_constant_add",
        ),
        (
            prepared_base_operation(
                1,
                0,
                PreparedBaseSource::Matrix(matrix),
                PreparedBaseSource::Constant(Felt::ONE),
            ),
            "matrix_constant_sub",
        ),
        (
            prepared_base_operation(
                2,
                0,
                PreparedBaseSource::Matrix(matrix),
                PreparedBaseSource::Constant(Felt::ONE),
            ),
            "matrix_constant_mul",
        ),
        (
            prepared_base_operation(
                3,
                0,
                PreparedBaseSource::Matrix(matrix),
                PreparedBaseSource::Constant(Felt::ONE),
            ),
            "matrix_constant_rsub",
        ),
        (
            prepared_base_operation(
                0,
                0,
                PreparedBaseSource::Tmp1(0),
                PreparedBaseSource::Constant(Felt::ONE),
            ),
            "tmp1_constant_add",
        ),
        (
            prepared_base_operation(
                1,
                0,
                PreparedBaseSource::Tmp1(0),
                PreparedBaseSource::Constant(Felt::ONE),
            ),
            "tmp1_constant_sub",
        ),
        (
            prepared_base_operation(
                2,
                0,
                PreparedBaseSource::Tmp1(0),
                PreparedBaseSource::Constant(Felt::ONE),
            ),
            "tmp1_constant_mul",
        ),
        (
            prepared_base_operation(
                3,
                0,
                PreparedBaseSource::Tmp1(0),
                PreparedBaseSource::Constant(Felt::ONE),
            ),
            "tmp1_constant_rsub",
        ),
        (
            prepared_base_operation(
                2,
                0,
                PreparedBaseSource::Constant(Felt::from_u64(3)),
                PreparedBaseSource::Constant(Felt::from_u64(4)),
            ),
            "constant_assign",
        ),
    ];

    for (operation, expected) in cases {
        let actual = match operation {
            PreparedBaseOperation::MatrixTmp1Add(_) => "matrix_tmp1_add",
            PreparedBaseOperation::MatrixTmp1Sub(_) => "matrix_tmp1_sub",
            PreparedBaseOperation::MatrixTmp1Mul(_) => "matrix_tmp1_mul",
            PreparedBaseOperation::MatrixTmp1RSub(_) => "matrix_tmp1_rsub",
            PreparedBaseOperation::MatrixMatrixAdd(_) => "matrix_matrix_add",
            PreparedBaseOperation::MatrixMatrixSub(_) => "matrix_matrix_sub",
            PreparedBaseOperation::MatrixMatrixMul(_) => "matrix_matrix_mul",
            PreparedBaseOperation::MatrixMatrixRSub(_) => "matrix_matrix_rsub",
            PreparedBaseOperation::Tmp1Tmp1Add(_) => "tmp1_tmp1_add",
            PreparedBaseOperation::Tmp1Tmp1Sub(_) => "tmp1_tmp1_sub",
            PreparedBaseOperation::Tmp1Tmp1Mul(_) => "tmp1_tmp1_mul",
            PreparedBaseOperation::Tmp1Tmp1RSub(_) => "tmp1_tmp1_rsub",
            PreparedBaseOperation::MatrixConstantAdd(_) => "matrix_constant_add",
            PreparedBaseOperation::MatrixConstantSub(_) => "matrix_constant_sub",
            PreparedBaseOperation::MatrixConstantMul(_) => "matrix_constant_mul",
            PreparedBaseOperation::MatrixConstantRSub(_) => "matrix_constant_rsub",
            PreparedBaseOperation::Tmp1ConstantAdd(_) => "tmp1_constant_add",
            PreparedBaseOperation::Tmp1ConstantSub(_) => "tmp1_constant_sub",
            PreparedBaseOperation::Tmp1ConstantMul(_) => "tmp1_constant_mul",
            PreparedBaseOperation::Tmp1ConstantRSub(_) => "tmp1_constant_rsub",
            PreparedBaseOperation::ConstantAssign(_) => "constant_assign",
            PreparedBaseOperation::MatrixTmp1RepeatedAdd(_) => "matrix_tmp1_repeated_add",
            PreparedBaseOperation::Generic(_) => "generic",
        };
        assert_eq!(actual, expected);
    }
}

#[test]
fn base_only_repeated_matrix_tmp1_adds_are_coalesced() {
    let entry = ConstraintEntry {
        stage: 1,
        destination_dimension: 1,
        destination_id: 0,
        first_row: 0,
        last_row: 8,
        temp1_count: 1,
        temp3_count: 0,
        ops_count: 7,
        ops_offset: 0,
        args_count: 56,
        args_offset: 0,
        intermediate: false,
        source_line: "base-only repeated matrix add".to_owned(),
    };
    let program = ConstraintProgram {
        entries: vec![entry.clone()],
        ops: vec![0; 7],
        args: vec![
            0, 0, 0, 0, 0, 8, 0, 0, //
            0, 0, 0, 0, 0, 5, 0, 0, //
            0, 0, 0, 0, 0, 5, 0, 0, //
            0, 0, 0, 0, 0, 5, 0, 0, //
            0, 0, 0, 0, 0, 5, 0, 0, //
            0, 0, 0, 0, 0, 5, 0, 0, //
            1, 0, 5, 0, 0, 8, 1, 0,
        ],
        numbers: vec![Felt::ZERO.to_u64(), Felt::from_u64(42).to_u64()],
    };
    let fixed = vec![Felt::from_u64(7); 8];
    let inputs = RegularConstraintInputs {
        domain_size: 8,
        stage_count: 1,
        fixed_columns: RegularColumnMatrix {
            column_count: 1,
            values: &fixed,
        },
        opening_point_offsets: &[0],
        ..RegularConstraintInputs::default()
    };
    let ops = entry_ops(0, &entry, &program).expect("operation span should read");
    let args = entry_args(0, &entry, &program).expect("argument span should read");
    let mut context = RowEvaluationContext {
        constraint_index: 0,
        entry: &entry,
        ops,
        args,
        operations: vec![None; ops.len()],
        sources: vec![[None, None]; ops.len()],
        program: &program,
        inputs,
        layout: BufferLayout::new(inputs),
    };
    let prepared = prepared_operations(&mut context).expect("operations should prepare");
    let base_program = prepared_base_operations(&entry, &prepared, &program, inputs)
        .expect("base program should prepare")
        .expect("all operations are base operations");

    assert_eq!(base_program.operations.len(), 3);
    match base_program.operations.as_slice() {
        [PreparedBaseOperation::MatrixConstantAdd(_), PreparedBaseOperation::MatrixTmp1RepeatedAdd(operation), PreparedBaseOperation::Tmp1ConstantSub(_)] =>
        {
            assert_eq!(operation.count, Felt::from_u64(5))
        }
        operations => panic!("unexpected coalesced operations: {operations:?}"),
    }
    let results =
        evaluate_regular_constraints(&program, inputs).expect("regular constraint should evaluate");
    assert_eq!(results[0].invalid_rows, Vec::new());
}

#[test]
fn repeated_matrix_tmp1_add_coalescing_respects_boundaries() {
    let values = [Felt::ONE; 8];
    let matrix = PreparedBaseMatrix {
        values: &values,
        column_count: 2,
        column: 0,
        row_offset: None,
    };
    let other_column = PreparedBaseMatrix {
        column: 1,
        ..matrix
    };
    let other_offset = PreparedBaseMatrix {
        row_offset: std::num::NonZeroUsize::new(1),
        ..matrix
    };
    let add = |destination_offset, tmp1_offset, matrix| {
        PreparedBaseOperation::MatrixTmp1Add(PreparedMatrixTmp1Operation {
            destination_offset,
            matrix,
            tmp1_offset,
        })
    };

    let cases = [
        vec![add(1, 0, matrix), add(1, 0, matrix)],
        vec![add(0, 0, matrix), add(1, 1, matrix)],
        vec![add(0, 0, matrix), add(0, 0, other_column)],
        vec![add(0, 0, matrix), add(0, 0, other_offset)],
        vec![
            add(0, 0, matrix),
            PreparedBaseOperation::ConstantAssign(PreparedConstantAssignOperation {
                destination_offset: 0,
                value: Felt::ZERO,
            }),
            add(0, 0, matrix),
        ],
    ];

    for operations in cases {
        let coalesced = coalesce_repeated_matrix_tmp1_adds(operations);
        assert!(
            coalesced.iter().all(|operation| !matches!(
                operation,
                PreparedBaseOperation::MatrixTmp1RepeatedAdd(_)
            )),
            "boundary case should not coalesce: {coalesced:?}"
        );
    }
}

#[test]
fn regular_constraint_worker_buckets_balance_entry_costs() {
    let entries = vec![
        test_entry_with_cost(0, 1),
        test_entry_with_cost(0, 1),
        test_entry_with_cost(0, 1),
        test_entry_with_cost(0, 1),
        test_entry_with_cost(0, 10_000),
        test_entry_with_cost(0, 10_000),
    ];

    let buckets = plan_constraint_entry_worker_buckets(&entries, 2);
    let heavy_owner_0 = buckets
        .iter()
        .position(|bucket| bucket.contains(&4))
        .expect("first heavy entry should be assigned");
    let heavy_owner_1 = buckets
        .iter()
        .position(|bucket| bucket.contains(&5))
        .expect("second heavy entry should be assigned");

    assert_ne!(heavy_owner_0, heavy_owner_1);
}

#[test]
fn worker_regular_constraints_preserve_results_for_non_contiguous_buckets() {
    let mut program = worker_bucket_test_program([
        test_entry_with_cost(0, 10),
        test_entry_with_cost(10, 9),
        test_entry_with_cost(19, 1),
        test_entry_with_cost(20, 1),
    ]);
    for (index, entry) in program.entries.iter_mut().enumerate() {
        entry.ops_count = 1;
        entry.args_count = 8;
        entry.ops_offset = 0;
        entry.args_offset = 0;
        entry.source_line = format!("worker bucket result {index}");
    }
    let fixed = vec![Felt::ZERO; 21];
    let inputs = RegularConstraintInputs {
        domain_size: 21,
        stage_count: 1,
        fixed_columns: RegularColumnMatrix {
            column_count: 1,
            values: &fixed,
        },
        opening_point_offsets: &[0],
        ..RegularConstraintInputs::default()
    };

    let sequential = evaluate_regular_constraints(&program, inputs)
        .expect("sequential regular constraints should evaluate");
    let worker = evaluate_regular_constraints_with_workers(&program, inputs, 2)
        .expect("worker regular constraints should evaluate");

    assert_eq!(worker, sequential);
}

#[test]
fn worker_regular_constraints_report_lowest_index_error_across_buckets() {
    let mut program = worker_bucket_test_program([
        test_entry_with_rows(0, 100),
        test_entry_with_rows(0, 99),
        test_entry_with_rows(100, 10),
        test_entry_with_rows(0, 1),
    ]);
    program.entries[1].ops_offset = 99;
    program.entries[3].ops_offset = 99;

    let error = evaluate_regular_constraints_with_workers(
        &program,
        RegularConstraintInputs {
            domain_size: 110,
            stage_count: 1,
            fixed_columns: RegularColumnMatrix {
                column_count: 1,
                values: &vec![Felt::ZERO; 110],
            },
            opening_point_offsets: &[0],
            ..RegularConstraintInputs::default()
        },
        2,
    )
    .expect_err("lowest indexed worker error should be reported");

    assert_eq!(
        error,
        RegularConstraintEvalError::OperationSpanOutOfBounds {
            constraint_index: 1,
        }
    );
}

#[test]
fn worker_bucket_stops_after_first_entry_error() {
    let mut program = worker_bucket_test_program([
        test_entry_with_rows(0, 10),
        test_entry_with_rows(0, 10),
        test_entry_with_rows(0, 100),
    ]);
    program.entries[0].ops_offset = 99;
    program.entries[1].source_line = "__lzvm_test_panic_if_evaluated__".to_owned();

    let error = evaluate_regular_constraints_with_workers(
        &program,
        RegularConstraintInputs {
            domain_size: 100,
            stage_count: 1,
            fixed_columns: RegularColumnMatrix {
                column_count: 1,
                values: &vec![Felt::ZERO; 100],
            },
            opening_point_offsets: &[0],
            ..RegularConstraintInputs::default()
        },
        2,
    )
    .expect_err("worker bucket should stop at the first recoverable error");

    assert_eq!(
        error,
        RegularConstraintEvalError::OperationSpanOutOfBounds {
            constraint_index: 0,
        }
    );
}

fn worker_bucket_test_program<const N: usize>(entries: [ConstraintEntry; N]) -> ConstraintProgram {
    ConstraintProgram {
        entries: entries.into_iter().collect(),
        ops: vec![0],
        args: vec![0, 0, 0, 0, 0, 8, 0, 0],
        numbers: vec![0],
    }
}

fn test_entry_with_rows(first_row: u32, row_count: u32) -> ConstraintEntry {
    ConstraintEntry {
        last_row: first_row + row_count,
        ..test_entry_with_cost(first_row, 1)
    }
}

fn test_entry_with_cost(first_row: u32, ops_count: u32) -> ConstraintEntry {
    ConstraintEntry {
        stage: 1,
        destination_dimension: 1,
        destination_id: 0,
        first_row,
        last_row: first_row + 1,
        temp1_count: 1,
        temp3_count: 0,
        ops_count,
        ops_offset: 0,
        args_count: ops_count * 8,
        args_offset: 0,
        intermediate: false,
        source_line: "worker bucket cost".to_owned(),
    }
}

#[test]
fn base_only_regular_constraints_clear_tmp1_for_unwritten_reads() {
    let program = ConstraintProgram {
        entries: vec![ConstraintEntry {
            stage: 1,
            destination_dimension: 1,
            destination_id: 0,
            first_row: 0,
            last_row: 8,
            temp1_count: 1,
            temp3_count: 0,
            ops_count: 1,
            ops_offset: 0,
            args_count: 8,
            args_offset: 0,
            intermediate: false,
            source_line: "base-only clear tmp1".to_owned(),
        }],
        ops: vec![0],
        args: vec![0, 0, 5, 0, 0, 8, 0, 0],
        numbers: vec![0],
    };

    BASE_ONLY_PREPARED_ROW_COUNT.with(|count| count.set(0));
    BASE_ONLY_TMP1_CLEAR_COUNT.with(|count| count.set(0));
    let results = evaluate_regular_constraints(
        &program,
        RegularConstraintInputs {
            domain_size: 8,
            stage_count: 1,
            opening_point_offsets: &[0],
            ..RegularConstraintInputs::default()
        },
    )
    .expect("regular constraint should evaluate");

    assert_eq!(results[0].invalid_rows, Vec::new());
    assert_eq!(BASE_ONLY_PREPARED_ROW_COUNT.with(Cell::get), 7);
    assert_eq!(BASE_ONLY_TMP1_CLEAR_COUNT.with(Cell::get), 7);
}

#[test]
fn base_only_regular_constraints_validate_kind_before_prepared_rows() {
    let program = ConstraintProgram {
        entries: vec![ConstraintEntry {
            stage: 1,
            destination_dimension: 1,
            destination_id: 0,
            first_row: 0,
            last_row: 8,
            temp1_count: 1,
            temp3_count: 0,
            ops_count: 1,
            ops_offset: 0,
            args_count: 8,
            args_offset: 0,
            intermediate: false,
            source_line: "base-only invalid operation".to_owned(),
        }],
        ops: vec![0],
        args: vec![9, 0, 0, 0, 0, 8, 0, 0],
        numbers: vec![3],
    };
    let fixed = vec![Felt::from_u64(3); 8];

    BASE_ONLY_PREPARED_ROW_COUNT.with(|count| count.set(0));
    let error = evaluate_regular_constraints(
        &program,
        RegularConstraintInputs {
            domain_size: 8,
            stage_count: 1,
            fixed_columns: RegularColumnMatrix {
                column_count: 1,
                values: &fixed,
            },
            opening_point_offsets: &[0],
            ..RegularConstraintInputs::default()
        },
    )
    .expect_err("unsupported operation kind should fail before prepared rows");

    assert_eq!(
        error,
        RegularConstraintEvalError::UnsupportedOperationKind { kind: 9 }
    );
    assert_eq!(BASE_ONLY_PREPARED_ROW_COUNT.with(Cell::get), 0);
}
