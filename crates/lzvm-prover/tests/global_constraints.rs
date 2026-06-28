use lzvm_artifacts::constraint_program::{GlobalConstraintEntry, GlobalConstraintProgram};
use lzvm_artifacts::global_info::{CurveKind, GlobalInfo};
use lzvm_field::{Ext3, Felt};
use lzvm_prover::global_constraints::{
    evaluate_global_constraints, validate_global_constraints,
    validate_global_constraints_from_proof_segments, GlobalConstraintEvalError,
    GlobalConstraintInputs, GlobalConstraintValidationError,
    ValidateGlobalConstraintProofSegmentsError, ValidateGlobalConstraintProofSegmentsRequest,
};
use lzvm_prover::ProveSchedule;

#[test]
fn evaluates_base_global_constraint_residuals() {
    let program = GlobalConstraintProgram {
        entries: vec![GlobalConstraintEntry {
            destination_dimension: 1,
            destination_id: 0,
            temp1_count: 1,
            temp3_count: 0,
            ops_count: 1,
            ops_offset: 0,
            args_count: 6,
            args_offset: 0,
            source_line: "base residual".to_owned(),
        }],
        ops: vec![0],
        args: vec![1, 0, 2, 0, 1, 0],
        numbers: vec![17],
    };

    let satisfied = evaluate_global_constraints(
        &program,
        GlobalConstraintInputs {
            publics: &[felt(17)],
            proof_values: &[],
            challenges: &[],
            group_values: &[],
        },
    )
    .expect("satisfied program should evaluate");
    assert_eq!(satisfied, vec![Ext3::ZERO]);

    let unsatisfied = evaluate_global_constraints(
        &program,
        GlobalConstraintInputs {
            publics: &[felt(12)],
            proof_values: &[],
            challenges: &[],
            group_values: &[],
        },
    )
    .expect("unsatisfied program should still evaluate");
    assert_eq!(unsatisfied, vec![ext([5, 0, 0])]);
}

#[test]
fn validates_satisfied_global_constraints() {
    let program = base_residual_program();

    validate_global_constraints(
        &program,
        GlobalConstraintInputs {
            publics: &[felt(17)],
            proof_values: &[],
            challenges: &[],
            group_values: &[],
        },
    )
    .expect("zero residual should validate");
}

#[test]
fn validates_global_constraints_from_proof_segments() {
    let program = base_residual_program();
    let global_info = global_info_without_values();
    let schedule = empty_schedule();

    validate_global_constraints_from_proof_segments(ValidateGlobalConstraintProofSegmentsRequest {
        program: &program,
        global_info: &global_info,
        schedule: &schedule,
        public_values: &[felt(17)],
        segments: &[],
    })
    .expect("zero residual should validate from proof segments");
}

#[test]
fn rejects_unsatisfied_global_constraints() {
    let program = base_residual_program();

    let error = validate_global_constraints(
        &program,
        GlobalConstraintInputs {
            publics: &[felt(12)],
            proof_values: &[],
            challenges: &[],
            group_values: &[],
        },
    )
    .expect_err("nonzero residual should reject");

    assert_eq!(
        error,
        GlobalConstraintValidationError::ConstraintViolation {
            constraint_index: 0,
            value: [5, 0, 0],
        }
    );
}

#[test]
fn rejects_global_constraint_violations_from_proof_segments() {
    let program = base_residual_program();
    let global_info = global_info_without_values();
    let schedule = empty_schedule();

    let error = validate_global_constraints_from_proof_segments(
        ValidateGlobalConstraintProofSegmentsRequest {
            program: &program,
            global_info: &global_info,
            schedule: &schedule,
            public_values: &[felt(12)],
            segments: &[],
        },
    )
    .expect_err("nonzero residual should reject");

    assert_eq!(
        error,
        ValidateGlobalConstraintProofSegmentsError::Validation(
            GlobalConstraintValidationError::ConstraintViolation {
                constraint_index: 0,
                value: [5, 0, 0],
            }
        )
    );
}

#[test]
fn evaluates_extension_shapes_with_flat_proof_value_offsets() {
    let program = GlobalConstraintProgram {
        entries: vec![GlobalConstraintEntry {
            destination_dimension: 3,
            destination_id: 1,
            temp1_count: 0,
            temp3_count: 2,
            ops_count: 2,
            ops_offset: 0,
            args_count: 12,
            args_offset: 0,
            source_line: "extension residual".to_owned(),
        }],
        ops: vec![1, 2],
        args: vec![
            2, 0, 3, 1, 1, 0, //
            1, 3, 4, 0, 6, 0,
        ],
        numbers: vec![],
    };

    let residuals = evaluate_global_constraints(
        &program,
        GlobalConstraintInputs {
            publics: &[felt(5)],
            proof_values: &[felt(99), felt(2), felt(3), felt(4)],
            challenges: &[ext([10, 15, 20])],
            group_values: &[],
        },
    )
    .expect("extension program should evaluate");

    assert_eq!(residuals, vec![Ext3::ZERO]);
}

#[test]
fn reads_group_values_as_flat_extension_fields() {
    let program = GlobalConstraintProgram {
        entries: vec![GlobalConstraintEntry {
            destination_dimension: 3,
            destination_id: 1,
            temp1_count: 0,
            temp3_count: 2,
            ops_count: 2,
            ops_offset: 0,
            args_count: 12,
            args_offset: 0,
            source_line: "group residual".to_owned(),
        }],
        ops: vec![2, 2],
        args: vec![
            0, 0, 3, 0, 5, 1, //
            1, 3, 4, 0, 6, 0,
        ],
        numbers: vec![],
    };

    let residuals = evaluate_global_constraints(
        &program,
        GlobalConstraintInputs {
            publics: &[],
            proof_values: &[felt(7), felt(11), felt(13)],
            challenges: &[ext([30, 40, 44])],
            group_values: &[ext([99, 23, 29]), ext([31, 37, 41])],
        },
    )
    .expect("group-value program should evaluate");

    assert_eq!(residuals, vec![Ext3::ZERO]);
}

#[test]
fn rejects_unknown_operation_shape() {
    let program = GlobalConstraintProgram {
        entries: vec![GlobalConstraintEntry {
            destination_dimension: 1,
            destination_id: 0,
            temp1_count: 1,
            temp3_count: 0,
            ops_count: 1,
            ops_offset: 0,
            args_count: 6,
            args_offset: 0,
            source_line: "bad shape".to_owned(),
        }],
        ops: vec![9],
        args: vec![0, 0, 2, 0, 2, 0],
        numbers: vec![1],
    };

    let error = evaluate_global_constraints(
        &program,
        GlobalConstraintInputs {
            publics: &[],
            proof_values: &[],
            challenges: &[],
            group_values: &[],
        },
    )
    .expect_err("shape should be rejected");

    assert_eq!(
        error,
        GlobalConstraintEvalError::UnsupportedOperationShape { shape: 9 }
    );
}

#[test]
fn rejects_argument_count_mismatch() {
    let program = GlobalConstraintProgram {
        entries: vec![GlobalConstraintEntry {
            destination_dimension: 1,
            destination_id: 0,
            temp1_count: 1,
            temp3_count: 0,
            ops_count: 1,
            ops_offset: 0,
            args_count: 5,
            args_offset: 0,
            source_line: "short args".to_owned(),
        }],
        ops: vec![0],
        args: vec![0, 0, 2, 0, 2],
        numbers: vec![1],
    };

    let error = evaluate_global_constraints(
        &program,
        GlobalConstraintInputs {
            publics: &[],
            proof_values: &[],
            challenges: &[],
            group_values: &[],
        },
    )
    .expect_err("short argument stream should be rejected");

    assert_eq!(
        error,
        GlobalConstraintEvalError::ArgumentCountMismatch {
            constraint_index: 0,
            consumed: 0,
            declared: 5
        }
    );
}

#[test]
fn rejects_extra_arguments_before_source_reads() {
    let program = GlobalConstraintProgram {
        entries: vec![GlobalConstraintEntry {
            destination_dimension: 1,
            destination_id: 0,
            temp1_count: 1,
            temp3_count: 0,
            ops_count: 1,
            ops_offset: 0,
            args_count: 7,
            args_offset: 0,
            source_line: "extra args".to_owned(),
        }],
        ops: vec![0],
        args: vec![0, 0, 1, 2, 2, 0, 9],
        numbers: vec![1],
    };

    let error = evaluate_global_constraints(
        &program,
        GlobalConstraintInputs {
            publics: &[felt(1)],
            proof_values: &[],
            challenges: &[],
            group_values: &[],
        },
    )
    .expect_err("extra argument stream should be rejected before source reads");

    assert_eq!(
        error,
        GlobalConstraintEvalError::ArgumentCountMismatch {
            constraint_index: 0,
            consumed: 6,
            declared: 7
        }
    );
}

#[test]
fn rejects_out_of_range_sources() {
    let program = GlobalConstraintProgram {
        entries: vec![GlobalConstraintEntry {
            destination_dimension: 1,
            destination_id: 0,
            temp1_count: 1,
            temp3_count: 0,
            ops_count: 1,
            ops_offset: 0,
            args_count: 6,
            args_offset: 0,
            source_line: "bad source".to_owned(),
        }],
        ops: vec![0],
        args: vec![0, 0, 1, 2, 2, 0],
        numbers: vec![1],
    };

    let error = evaluate_global_constraints(
        &program,
        GlobalConstraintInputs {
            publics: &[felt(1)],
            proof_values: &[],
            challenges: &[],
            group_values: &[],
        },
    )
    .expect_err("public source should be out of range");

    assert_eq!(
        error,
        GlobalConstraintEvalError::SourceIndexOutOfRange {
            buffer: "public",
            offset: 2,
            width: 1,
            len: 1
        }
    );
}

fn felt(value: u64) -> Felt {
    Felt::from_u64(value)
}

fn ext(values: [u64; 3]) -> Ext3 {
    Ext3::from_u64s(values)
}

fn base_residual_program() -> GlobalConstraintProgram {
    GlobalConstraintProgram {
        entries: vec![GlobalConstraintEntry {
            destination_dimension: 1,
            destination_id: 0,
            temp1_count: 1,
            temp3_count: 0,
            ops_count: 1,
            ops_offset: 0,
            args_count: 6,
            args_offset: 0,
            source_line: "base residual".to_owned(),
        }],
        ops: vec![0],
        args: vec![1, 0, 2, 0, 1, 0],
        numbers: vec![17],
    }
}

fn global_info_without_values() -> GlobalInfo {
    GlobalInfo {
        name: "global".to_owned(),
        air_groups: Vec::new(),
        airs: Vec::new(),
        curve: CurveKind::None,
        lattice_size: None,
        aggregation_types: Vec::new(),
        n_publics: 1,
        num_challenges: Vec::new(),
        num_proof_values: Vec::new(),
        proof_values_map: Vec::new(),
        publics_map: Vec::new(),
        transcript_arity: 2,
    }
}

fn empty_schedule() -> ProveSchedule {
    ProveSchedule {
        setup_hash: [0; 32],
        unit_count: 0,
        total_fixed_bytes: 0,
        total_pcs_material_bytes: 0,
        pcs_material_unit_count: 0,
        total_query_count: 0,
        max_extended_domain_bits: 0,
        units: Vec::new(),
    }
}
