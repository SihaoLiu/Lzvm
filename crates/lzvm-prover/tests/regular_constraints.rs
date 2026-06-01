use lzvm_artifacts::constraint_program::{ConstraintEntry, ConstraintProgram};
use lzvm_field::{Ext3, Felt};
use lzvm_prover::regular_constraints::{
    evaluate_regular_constraints, RegularColumnMatrix, RegularConstraintEvalError,
    RegularConstraintInputs, RegularStageColumns,
};

#[test]
fn reports_regular_constraint_violations_inside_declared_row_bounds() {
    let program = ConstraintProgram {
        entries: vec![ConstraintEntry {
            stage: 1,
            destination_dimension: 1,
            destination_id: 0,
            first_row: 1,
            last_row: 3,
            temp1_count: 1,
            temp3_count: 0,
            ops_count: 1,
            ops_offset: 0,
            args_count: 8,
            args_offset: 0,
            intermediate: false,
            source_line: "row bounded residual".to_owned(),
        }],
        ops: vec![0],
        args: vec![1, 0, 0, 0, 0, 8, 0, 0],
        numbers: vec![10],
    };
    let fixed = [felt(10), felt(11), felt(10), felt(12)];

    let results = evaluate_regular_constraints(
        &program,
        RegularConstraintInputs {
            domain_size: 4,
            stage_count: 1,
            fixed_columns: RegularColumnMatrix {
                column_count: 1,
                values: &fixed,
            },
            stage_columns: &[],
            custom_fixed_columns: &[],
            opening_point_offsets: &[0],
            ..RegularConstraintInputs::default()
        },
    )
    .expect("regular constraint should evaluate");

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].constraint_index, 0);
    assert_eq!(results[0].invalid_rows.len(), 1);
    assert_eq!(results[0].invalid_rows[0].row, 1);
    assert_eq!(results[0].invalid_rows[0].value, Ext3::from_u64s([1, 0, 0]));
}

#[test]
fn treats_declared_last_row_as_exclusive() {
    let program = ConstraintProgram {
        entries: vec![ConstraintEntry {
            stage: 1,
            destination_dimension: 1,
            destination_id: 0,
            first_row: 0,
            last_row: 1,
            temp1_count: 1,
            temp3_count: 0,
            ops_count: 1,
            ops_offset: 0,
            args_count: 8,
            args_offset: 0,
            intermediate: false,
            source_line: "exclusive row residual".to_owned(),
        }],
        ops: vec![0],
        args: vec![1, 0, 0, 0, 0, 8, 0, 0],
        numbers: vec![10],
    };
    let fixed = [felt(10), felt(11)];

    let results = evaluate_regular_constraints(
        &program,
        RegularConstraintInputs {
            domain_size: 2,
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
}

#[test]
fn reports_missing_domain_helper_values_by_name() {
    let program = ConstraintProgram {
        entries: vec![ConstraintEntry {
            stage: 1,
            destination_dimension: 1,
            destination_id: 0,
            first_row: 0,
            last_row: 1,
            temp1_count: 1,
            temp3_count: 0,
            ops_count: 1,
            ops_offset: 0,
            args_count: 8,
            args_offset: 0,
            intermediate: false,
            source_line: "domain helper residual".to_owned(),
        }],
        ops: vec![0],
        args: vec![0, 0, 3, 0, 0, 8, 0, 0],
        numbers: vec![0],
    };

    let error = evaluate_regular_constraints(
        &program,
        RegularConstraintInputs {
            domain_size: 1,
            stage_count: 1,
            opening_point_offsets: &[0],
            ..RegularConstraintInputs::default()
        },
    )
    .expect_err("missing domain helper value should be reported by source name");

    assert_eq!(
        error,
        RegularConstraintEvalError::SourceIndexOutOfRange {
            buffer: "domain point",
            offset: 0,
            width: 1,
            len: 0,
        }
    );
}

#[test]
fn reads_domain_and_zerofier_helper_values() {
    let program = ConstraintProgram {
        entries: vec![ConstraintEntry {
            stage: 1,
            destination_dimension: 1,
            destination_id: 0,
            first_row: 0,
            last_row: 2,
            temp1_count: 1,
            temp3_count: 0,
            ops_count: 1,
            ops_offset: 0,
            args_count: 8,
            args_offset: 0,
            intermediate: false,
            source_line: "domain helper residual".to_owned(),
        }],
        ops: vec![0],
        args: vec![0, 0, 3, 0, 0, 3, 1, 0],
        numbers: Vec::new(),
    };
    let zerofier_values = [felt(3), felt(4)];

    let results = evaluate_regular_constraints(
        &program,
        RegularConstraintInputs {
            domain_size: 2,
            stage_count: 1,
            domain_points: &[felt(10), felt(20)],
            zerofier_values: RegularColumnMatrix {
                column_count: 1,
                values: &zerofier_values,
            },
            opening_point_offsets: &[0],
            ..RegularConstraintInputs::default()
        },
    )
    .expect("regular constraint should evaluate helper values");

    assert_eq!(results[0].invalid_rows.len(), 2);
    assert_eq!(results[0].invalid_rows[0].row, 0);
    assert_eq!(
        results[0].invalid_rows[0].value,
        Ext3::from_u64s([13, 0, 0])
    );
    assert_eq!(results[0].invalid_rows[1].row, 1);
    assert_eq!(
        results[0].invalid_rows[1].value,
        Ext3::from_u64s([24, 0, 0])
    );
}

#[test]
fn evaluates_extension_regular_constraints_from_stage_and_challenges() {
    let program = ConstraintProgram {
        entries: vec![ConstraintEntry {
            stage: 1,
            destination_dimension: 3,
            destination_id: 0,
            first_row: 0,
            last_row: 3,
            temp1_count: 0,
            temp3_count: 1,
            ops_count: 1,
            ops_offset: 0,
            args_count: 8,
            args_offset: 0,
            intermediate: false,
            source_line: "extension row residual".to_owned(),
        }],
        ops: vec![2],
        args: vec![1, 0, 1, 0, 0, 12, 0, 0],
        numbers: vec![],
    };
    let stage_values = [
        felt(10),
        felt(11),
        felt(12),
        felt(10),
        felt(11),
        felt(12),
        felt(13),
        felt(11),
        felt(12),
    ];
    let stage_columns = [RegularStageColumns {
        stage_index: 1,
        column_count: 3,
        values: &stage_values,
    }];

    let results = evaluate_regular_constraints(
        &program,
        RegularConstraintInputs {
            domain_size: 3,
            stage_count: 1,
            stage_columns: &stage_columns,
            opening_point_offsets: &[0],
            challenges: &[Ext3::from_u64s([10, 11, 12])],
            ..RegularConstraintInputs::default()
        },
    )
    .expect("extension regular constraint should evaluate");

    assert_eq!(results[0].invalid_rows.len(), 1);
    assert_eq!(results[0].invalid_rows[0].row, 2);
    assert_eq!(results[0].invalid_rows[0].value, Ext3::from_u64s([3, 0, 0]));
}

#[test]
fn applies_opening_point_row_offsets_cyclically() {
    let program = ConstraintProgram {
        entries: vec![ConstraintEntry {
            stage: 1,
            destination_dimension: 1,
            destination_id: 0,
            first_row: 1,
            last_row: 3,
            temp1_count: 1,
            temp3_count: 0,
            ops_count: 1,
            ops_offset: 0,
            args_count: 8,
            args_offset: 0,
            intermediate: false,
            source_line: "shifted row residual".to_owned(),
        }],
        ops: vec![0],
        args: vec![3, 0, 0, 0, 1, 8, 0, 0],
        numbers: vec![30],
    };
    let fixed = [felt(10), felt(20), felt(30)];

    let results = evaluate_regular_constraints(
        &program,
        RegularConstraintInputs {
            domain_size: 3,
            stage_count: 1,
            fixed_columns: RegularColumnMatrix {
                column_count: 1,
                values: &fixed,
            },
            opening_point_offsets: &[0, 1],
            ..RegularConstraintInputs::default()
        },
    )
    .expect("shifted regular constraint should evaluate");

    assert_eq!(results[0].invalid_rows.len(), 1);
    assert_eq!(results[0].invalid_rows[0].row, 2);
    assert_eq!(
        results[0].invalid_rows[0].value,
        Ext3::from_u64s([20, 0, 0])
    );
}

#[test]
fn applies_negative_and_large_opening_point_row_offsets_cyclically() {
    let program = ConstraintProgram {
        entries: vec![
            ConstraintEntry {
                stage: 1,
                destination_dimension: 1,
                destination_id: 0,
                first_row: 0,
                last_row: 1,
                temp1_count: 1,
                temp3_count: 0,
                ops_count: 1,
                ops_offset: 0,
                args_count: 8,
                args_offset: 0,
                intermediate: false,
                source_line: "negative shifted row residual".to_owned(),
            },
            ConstraintEntry {
                stage: 1,
                destination_dimension: 1,
                destination_id: 0,
                first_row: 1,
                last_row: 2,
                temp1_count: 1,
                temp3_count: 0,
                ops_count: 1,
                ops_offset: 1,
                args_count: 8,
                args_offset: 8,
                intermediate: false,
                source_line: "large shifted row residual".to_owned(),
            },
        ],
        ops: vec![0, 0],
        args: vec![
            1, 0, 0, 0, 1, 8, 0, 0, //
            1, 0, 0, 0, 2, 8, 0, 0,
        ],
        numbers: vec![30],
    };
    let fixed = [felt(10), felt(20), felt(30)];

    let results = evaluate_regular_constraints(
        &program,
        RegularConstraintInputs {
            domain_size: 3,
            stage_count: 1,
            fixed_columns: RegularColumnMatrix {
                column_count: 1,
                values: &fixed,
            },
            opening_point_offsets: &[0, -1, 4],
            ..RegularConstraintInputs::default()
        },
    )
    .expect("shifted regular constraints should evaluate");

    assert_eq!(results[0].invalid_rows, Vec::new());
    assert_eq!(results[1].invalid_rows, Vec::new());
}

#[test]
fn reports_first_executed_source_error_before_later_source_decode_error() {
    let program = ConstraintProgram {
        entries: vec![ConstraintEntry {
            stage: 1,
            destination_dimension: 1,
            destination_id: 0,
            first_row: 0,
            last_row: 1,
            temp1_count: 1,
            temp3_count: 0,
            ops_count: 2,
            ops_offset: 0,
            args_count: 16,
            args_offset: 0,
            intermediate: false,
            source_line: "source error precedence residual".to_owned(),
        }],
        ops: vec![0, 0],
        args: vec![
            0, 0, 0, 1, 0, 8, 0, 0, //
            0, 0, 1, 0, 0, 8, 0, 0,
        ],
        numbers: vec![0],
    };
    let fixed = [felt(7)];

    let error = evaluate_regular_constraints(
        &program,
        RegularConstraintInputs {
            domain_size: 1,
            stage_count: 1,
            fixed_columns: RegularColumnMatrix {
                column_count: 1,
                values: &fixed,
            },
            opening_point_offsets: &[0],
            ..RegularConstraintInputs::default()
        },
    )
    .expect_err("first executed source error should be reported first");

    assert_eq!(
        error,
        RegularConstraintEvalError::SourceIndexOutOfRange {
            buffer: "column matrix",
            offset: 1,
            width: 1,
            len: 1,
        }
    );
}

fn felt(value: u64) -> Felt {
    Felt::from_u64(value)
}
