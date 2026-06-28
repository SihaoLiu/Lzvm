use lzvm_artifacts::{
    expression_program::{ExpressionEntry, ExpressionProgram},
    setup_info::Boundary,
};
use lzvm_field::{Ext3, Felt};
use lzvm_prover::fri_polynomial::{
    build_fri_domain_points, build_fri_polynomial, derive_opening_xis, FriPolynomialColumnMatrix,
    FriPolynomialError, FriPolynomialInputs, FriPolynomialStageColumns, FriPolynomialZerofierTable,
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

#[test]
fn reads_domain_and_zerofier_helper_values() {
    let program = ExpressionProgram {
        max_tmp1: 1,
        max_tmp3: 0,
        max_args: 8,
        max_ops: 1,
        entries: vec![ExpressionEntry {
            expression_id: 8,
            destination_dimension: 1,
            destination_id: 0,
            stage: 3,
            temp1_count: 1,
            temp3_count: 0,
            ops_count: 1,
            ops_offset: 0,
            args_count: 8,
            args_offset: 0,
            source_line: "native helper row".to_owned(),
        }],
        ops: vec![0],
        args: vec![0, 0, 3, 0, 0, 3, 1, 0],
        numbers: Vec::new(),
    };
    let zerofier_values = [felt(3), felt(4)];

    let polynomial = build_fri_polynomial(
        &program,
        8,
        FriPolynomialInputs {
            domain_size: 2,
            stage_count: 1,
            fixed_columns: FriPolynomialColumnMatrix::default(),
            domain_points: &[felt(10), felt(20)],
            zerofier_values: FriPolynomialColumnMatrix {
                column_count: 1,
                values: &zerofier_values,
            },
            ..FriPolynomialInputs::default()
        },
    )
    .expect("polynomial should build");

    assert_eq!(
        polynomial,
        vec![Ext3::from_u64s([13, 0, 0]), Ext3::from_u64s([24, 0, 0])]
    );
}

#[test]
fn computes_opening_denominator_helpers_from_domain_points_and_opening_xis() {
    let program = ExpressionProgram {
        max_tmp1: 0,
        max_tmp3: 1,
        max_args: 8,
        max_ops: 1,
        entries: vec![ExpressionEntry {
            expression_id: 9,
            destination_dimension: 3,
            destination_id: 0,
            stage: 3,
            temp1_count: 0,
            temp3_count: 1,
            ops_count: 1,
            ops_offset: 0,
            args_count: 8,
            args_offset: 0,
            source_line: "native opening row".to_owned(),
        }],
        ops: vec![2],
        args: vec![0, 0, 4, 0, 0, 8, 0, 0],
        numbers: vec![0, 0, 0],
    };
    let expected = (Ext3::from_u64s([5, 0, 0]) - Ext3::from_u64s([2, 1, 0]))
        .inverse()
        .expect("denominator should be nonzero");

    let polynomial = build_fri_polynomial(
        &program,
        9,
        FriPolynomialInputs {
            domain_size: 1,
            stage_count: 1,
            fixed_columns: FriPolynomialColumnMatrix::default(),
            domain_points: &[felt(5)],
            opening_xis: &[Ext3::from_u64s([2, 1, 0])],
            ..FriPolynomialInputs::default()
        },
    )
    .expect("polynomial should build");

    assert_eq!(polynomial, vec![expected]);
}

#[test]
fn rejects_extra_operation_arguments_before_row_sources() {
    let program = ExpressionProgram {
        max_tmp1: 1,
        max_tmp3: 0,
        max_args: 9,
        max_ops: 1,
        entries: vec![ExpressionEntry {
            expression_id: 10,
            destination_dimension: 1,
            destination_id: 0,
            stage: 3,
            temp1_count: 1,
            temp3_count: 0,
            ops_count: 1,
            ops_offset: 0,
            args_count: 9,
            args_offset: 0,
            source_line: "native malformed row".to_owned(),
        }],
        ops: vec![0],
        args: vec![0, 0, 99, 0, 0, 99, 0, 0, 7],
        numbers: Vec::new(),
    };

    let error = build_fri_polynomial(
        &program,
        10,
        FriPolynomialInputs {
            domain_size: 2,
            stage_count: 1,
            fixed_columns: FriPolynomialColumnMatrix::default(),
            opening_point_offsets: &[0],
            ..FriPolynomialInputs::default()
        },
    )
    .expect_err("extra operation argument should be rejected before row sources");

    assert_eq!(
        error,
        FriPolynomialError::ArgumentCountMismatch {
            expression_id: 10,
            consumed: 8,
            declared: 9,
        }
    );
}

#[test]
fn builds_shifted_extended_domain_points() {
    let root = Felt::root_of_unity(2).expect("domain root should exist");

    let points = build_fri_domain_points(2).expect("domain points should build");

    assert_eq!(
        points,
        vec![
            felt(7),
            felt(7) * root,
            felt(7) * root.pow(2),
            felt(7) * root.pow(3),
        ]
    );
}

#[test]
fn derives_opening_xis_from_base_domain_offsets() {
    let root = Felt::root_of_unity(3).expect("domain root should exist");
    let xi = Ext3::from_u64s([3, 5, 7]);

    let values = derive_opening_xis(3, &[0, 2, -1], xi).expect("opening points should derive");

    assert_eq!(
        values,
        vec![
            xi,
            xi * Ext3::new(root.pow(2), Felt::ZERO, Felt::ZERO),
            xi * Ext3::new(
                root.inverse().expect("root should be nonzero"),
                Felt::ZERO,
                Felt::ZERO
            ),
        ]
    );
}

#[test]
fn builds_zerofier_table_for_every_and_first_row_boundaries() {
    let root = Felt::root_of_unity(2).expect("domain root should exist");
    let points = build_fri_domain_points(3).expect("domain points should build");

    let table = FriPolynomialZerofierTable::build(
        2,
        3,
        &[
            Boundary {
                name: Some("everyRow".to_owned()),
                offset_min: None,
                offset_max: None,
            },
            Boundary {
                name: Some("firstRow".to_owned()),
                offset_min: None,
                offset_max: None,
            },
        ],
    )
    .expect("zerofier table should build");

    assert_eq!(table.column_count, 2);
    assert_eq!(table.values.len(), 16);
    for (row, x) in points.iter().copied().enumerate() {
        let every = (x.pow(4) - Felt::ONE)
            .inverse()
            .expect("shifted coset should not hit the base domain");
        let first = ((x - Felt::ONE) * every)
            .inverse()
            .expect("shifted coset should not hit one");
        assert_eq!(table.values[row * 2], every);
        assert_eq!(table.values[row * 2 + 1], first);
    }
    assert_eq!(root.pow(4), Felt::ONE);
}

#[test]
fn builds_zerofier_table_for_frame_boundaries() {
    let root = Felt::root_of_unity(2).expect("domain root should exist");
    let points = build_fri_domain_points(3).expect("domain points should build");

    let table = FriPolynomialZerofierTable::build(
        2,
        3,
        &[Boundary {
            name: Some("everyFrame".to_owned()),
            offset_min: Some(1),
            offset_max: Some(1),
        }],
    )
    .expect("zerofier table should build");

    assert_eq!(table.column_count, 1);
    for (row, x) in points.iter().copied().enumerate() {
        let expected = (x - Felt::ONE) * (x - root.pow(3));
        assert_eq!(table.values[row], expected);
    }
}

fn felt(value: u64) -> Felt {
    Felt::from_u64(value)
}
