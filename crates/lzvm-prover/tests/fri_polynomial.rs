use lzvm_artifacts::expression_program::{ExpressionEntry, ExpressionProgram};
use lzvm_field::{Ext3, Felt};
use lzvm_prover::fri_polynomial::{
    build_fri_polynomial, FriPolynomialColumnMatrix, FriPolynomialInputs, FriPolynomialStageColumns,
};

#[test]
fn builds_fri_polynomial_from_expression_program_over_extended_rows() {
    let program = ExpressionProgram {
        max_tmp1: 0,
        max_tmp3: 1,
        max_args: 8,
        max_ops: 1,
        entries: vec![ExpressionEntry {
            expression_id: 7,
            destination_dimension: 3,
            destination_id: 0,
            stage: 3,
            temp1_count: 0,
            temp3_count: 1,
            ops_count: 1,
            ops_offset: 0,
            args_count: 8,
            args_offset: 0,
            source_line: "native polynomial row".to_owned(),
        }],
        ops: vec![2],
        args: vec![1, 0, 1, 0, 0, 12, 0, 0],
        numbers: Vec::new(),
    };
    let stage_values = [
        felt(11),
        felt(12),
        felt(13),
        felt(21),
        felt(22),
        felt(23),
        felt(31),
        felt(32),
        felt(33),
    ];
    let stage_columns = [FriPolynomialStageColumns {
        stage_index: 1,
        column_count: 3,
        values: &stage_values,
    }];

    let polynomial = build_fri_polynomial(
        &program,
        7,
        FriPolynomialInputs {
            domain_size: 3,
            stage_count: 1,
            fixed_columns: FriPolynomialColumnMatrix::default(),
            stage_columns: &stage_columns,
            custom_fixed_columns: &[],
            opening_point_offsets: &[0],
            challenges: &[Ext3::from_u64s([1, 2, 3])],
            ..FriPolynomialInputs::default()
        },
    )
    .expect("polynomial should build");

    assert_eq!(
        polynomial,
        vec![
            Ext3::from_u64s([10, 10, 10]),
            Ext3::from_u64s([20, 20, 20]),
            Ext3::from_u64s([30, 30, 30]),
        ]
    );
}

fn felt(value: u64) -> Felt {
    Felt::from_u64(value)
}
