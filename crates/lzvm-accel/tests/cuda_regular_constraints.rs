#![cfg(feature = "cuda")]

use lzvm_accel::{
    cuda_regular_constraints_base, CudaRegularConstraintEntry, CudaRegularConstraintInputs,
    CudaRegularStage,
};

const TMP1_BUFFER: u16 = 5;
const NUMBER_BUFFER: u16 = 8;

#[test]
fn cuda_regular_constraints_accepts_zero_residuals() {
    let fixed = [3, 3, 3, 3];
    let stage = [8, 8, 8, 8];
    let stages = [CudaRegularStage {
        stage_index: 1,
        column_count: 1,
        values: &stage,
    }];
    let entries = [entry()];
    let ops = [0, 0];
    let args = [
        0,
        0,
        0,
        0,
        0,
        NUMBER_BUFFER,
        0,
        0,
        1,
        1,
        TMP1_BUFFER,
        0,
        0,
        1,
        0,
        0,
    ];

    let results = cuda_regular_constraints_base(
        &entries,
        &ops,
        &args,
        CudaRegularConstraintInputs {
            domain_size: 4,
            stage_count: 1,
            fixed_column_count: 1,
            fixed_values: &fixed,
            stages: &stages,
            opening_point_offsets: &[0],
            numbers: &[5],
            unit_values: &[],
        },
    )
    .expect("cuda regular constraint evaluation should succeed");

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].invalid_row, None);
}

#[test]
fn cuda_regular_constraints_reports_first_invalid_row() {
    let fixed = [3, 3, 3, 3];
    let stage = [8, 9, 7, 8];
    let stages = [CudaRegularStage {
        stage_index: 1,
        column_count: 1,
        values: &stage,
    }];
    let entries = [entry()];
    let ops = [0, 0];
    let args = [
        0,
        0,
        0,
        0,
        0,
        NUMBER_BUFFER,
        0,
        0,
        1,
        1,
        TMP1_BUFFER,
        0,
        0,
        1,
        0,
        0,
    ];

    let results = cuda_regular_constraints_base(
        &entries,
        &ops,
        &args,
        CudaRegularConstraintInputs {
            domain_size: 4,
            stage_count: 1,
            fixed_column_count: 1,
            fixed_values: &fixed,
            stages: &stages,
            opening_point_offsets: &[0],
            numbers: &[5],
            unit_values: &[],
        },
    )
    .expect("cuda regular constraint evaluation should succeed");

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].invalid_row, Some(1));
    assert_eq!(results[0].value, 0xffff_ffff_0000_0000);
}

fn entry() -> CudaRegularConstraintEntry {
    CudaRegularConstraintEntry {
        destination_id: 1,
        first_row: 0,
        last_row: 4,
        temp1_count: 2,
        ops_count: 2,
        ops_offset: 0,
        args_count: 16,
        args_offset: 0,
    }
}
