use lzvm_artifacts::constraint_program::{
    parse_regular_constraint_program, ConstraintEntry, ConstraintProgram,
};
use lzvm_artifacts::expression_program::{
    parse_expression_program, ExpressionEntry, ExpressionProgram,
};
use lzvm_artifacts::hint_program::{
    parse_regular_hint_program, Hint, HintField, HintOperand, HintProgram, HintValue,
};
use lzvm_artifacts::regular_program::{
    encode_regular_program, parse_regular_program, read_regular_program_file,
    regular_program_from_expression_info, verifier_program_from_verifier_info, RegularProgram,
    RegularProgramLoweringError,
};
use lzvm_artifacts::{
    expression_info::{
        BoundaryKind, CodeDestination, CodeOperand, CodeOperation, ConstraintCode, ExpressionCode,
        ExpressionInfo, ExpressionInfoError, OperationKind,
    },
    global_info::{
        AggregationType, CurveKind, GlobalAir, GlobalInfo, NamedStageValue, PublicValue,
    },
    setup_info::{CommitmentColumn, FriStep, StageValue, StarkStruct, UnitSetupInfo},
    verifier_info::{
        VerifierCode, VerifierDestination, VerifierInfo, VerifierInfoError, VerifierOperand,
        VerifierOperation, VerifierOperationKind,
    },
};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

fn sample_regular_program() -> RegularProgram {
    RegularProgram {
        expressions: ExpressionProgram {
            max_tmp1: 1,
            max_tmp3: 0,
            max_args: 3,
            max_ops: 1,
            entries: vec![ExpressionEntry {
                expression_id: 7,
                destination_dimension: 1,
                destination_id: 0,
                stage: 1,
                temp1_count: 1,
                temp3_count: 0,
                ops_count: 1,
                ops_offset: 0,
                args_count: 3,
                args_offset: 0,
                source_line: "expr-a".to_owned(),
            }],
            ops: vec![0],
            args: vec![0, 1, 2],
            numbers: vec![42],
        },
        constraints: ConstraintProgram {
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
                args_count: 2,
                args_offset: 0,
                intermediate: false,
                source_line: "constraint-a".to_owned(),
            }],
            ops: vec![1],
            args: vec![3, 4],
            numbers: vec![99],
        },
        hints: HintProgram {
            hints: vec![Hint {
                name: "hint-a".to_owned(),
                fields: vec![HintField {
                    name: "field-a".to_owned(),
                    values: vec![HintValue {
                        operand: HintOperand::Temporary {
                            id: 0,
                            dimension: Some(1),
                        },
                        positions: vec![0],
                    }],
                }],
            }],
        },
    }
}

fn temp_file_path(name: &str) -> PathBuf {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("workspace root should resolve")
        .join("temp")
        .join(format!(
            "lzvm-regular-program-{}-{name}",
            std::process::id()
        ));
    fs::create_dir_all(path.parent().expect("fixture path should have parent"))
        .expect("fixture directory should be created");
    path
}

#[test]
fn encodes_and_parses_combined_regular_program_sections() {
    let program = sample_regular_program();
    let encoded = encode_regular_program(&program).expect("program should encode");

    assert_eq!(
        parse_expression_program(&encoded).expect("expressions should parse"),
        program.expressions
    );
    assert_eq!(
        parse_regular_constraint_program(&encoded).expect("constraints should parse"),
        program.constraints
    );
    assert_eq!(
        parse_regular_hint_program(&encoded).expect("hints should parse"),
        program.hints
    );
    assert_eq!(
        parse_regular_program(&encoded).expect("program should parse"),
        program
    );
}

#[test]
fn reads_regular_programs_from_a_file_path() {
    let path = temp_file_path("program.bin");
    let program = sample_regular_program();
    fs::write(
        &path,
        encode_regular_program(&program).expect("program should encode"),
    )
    .expect("program file should be written");

    let parsed = read_regular_program_file(&path).expect("program should parse");
    fs::remove_file(&path).expect("program file should be removed");

    assert_eq!(parsed, program);
}

#[test]
fn builds_regular_program_from_expression_info() {
    let info = ExpressionInfo {
        hints: Vec::new(),
        expressions: vec![ExpressionCode {
            expression_id: 11,
            stage: 1,
            line: "public plus constant".to_owned(),
            temporary_count: 1,
            destination: None,
            operations: vec![CodeOperation {
                op: OperationKind::Add,
                destination: CodeDestination::temporary(0, 1),
                sources: vec![CodeOperand::public(0, 1), CodeOperand::number(5, 1)],
            }],
        }],
        constraints: vec![ConstraintCode {
            stage: 1,
            boundary: BoundaryKind::EveryRow,
            offset_min: None,
            offset_max: None,
            line: "public minus constant".to_owned(),
            intermediate: false,
            temporary_count: 1,
            operations: vec![CodeOperation {
                op: OperationKind::Sub,
                destination: CodeDestination::temporary(0, 1),
                sources: vec![CodeOperand::public(0, 1), CodeOperand::number(5, 1)],
            }],
        }],
    };

    let program =
        regular_program_from_expression_info(&info, &minimal_setup_info()).expect("program lowers");

    assert_eq!(program.expressions.max_tmp1, 1);
    assert_eq!(program.expressions.max_tmp3, 0);
    assert_eq!(program.expressions.max_args, 8);
    assert_eq!(program.expressions.max_ops, 1);
    assert_eq!(program.expressions.ops, vec![0]);
    assert_eq!(program.expressions.args, vec![0, 0, 7, 0, 0, 8, 0, 0]);
    assert_eq!(program.expressions.numbers, vec![5]);
    assert_eq!(program.expressions.entries[0].expression_id, 11);
    assert_eq!(program.expressions.entries[0].destination_dimension, 1);
    assert_eq!(program.expressions.entries[0].destination_id, 0);

    assert_eq!(program.constraints.ops, vec![0]);
    assert_eq!(program.constraints.args, vec![1, 0, 7, 0, 0, 8, 0, 0]);
    assert_eq!(program.constraints.numbers, vec![5]);
    assert_eq!(program.constraints.entries[0].first_row, 0);
    assert_eq!(program.constraints.entries[0].last_row, 4);
    assert_eq!(program.hints.hints, Vec::new());

    let encoded = encode_regular_program(&program).expect("program should encode");
    assert_eq!(
        parse_regular_program(&encoded).expect("program should parse"),
        program
    );
}

#[test]
fn rejects_duplicate_expression_ids_when_lowering() {
    let expression = ExpressionCode {
        expression_id: 11,
        stage: 1,
        line: "public plus constant".to_owned(),
        temporary_count: 1,
        destination: None,
        operations: vec![CodeOperation {
            op: OperationKind::Add,
            destination: CodeDestination::temporary(0, 1),
            sources: vec![CodeOperand::public(0, 1), CodeOperand::number(5, 1)],
        }],
    };
    let info = ExpressionInfo {
        hints: Vec::new(),
        expressions: vec![expression.clone(), expression],
        constraints: Vec::new(),
    };

    assert_eq!(
        regular_program_from_expression_info(&info, &minimal_setup_info()),
        Err(RegularProgramLoweringError::Expression(
            ExpressionInfoError::DuplicateExpressionId { expression_id: 11 }
        ))
    );
}

#[test]
fn lowers_extension_constraint_sources_in_canonical_order() {
    let info = ExpressionInfo {
        hints: Vec::new(),
        expressions: Vec::new(),
        constraints: vec![ConstraintCode {
            stage: 1,
            boundary: BoundaryKind::EveryFrame,
            offset_min: Some(0),
            offset_max: Some(1),
            line: "challenge minus stage value".to_owned(),
            intermediate: true,
            temporary_count: 1,
            operations: vec![CodeOperation {
                op: OperationKind::Sub,
                destination: CodeDestination::temporary(0, 3),
                sources: vec![
                    CodeOperand::challenge(0, Some(1), Some(0), 3),
                    CodeOperand::commitment_at(0, Some(0), 3),
                ],
            }],
        }],
    };

    let program = regular_program_from_expression_info(&info, &setup_info_with_commitment())
        .expect("program lowers");

    assert_eq!(program.constraints.ops, vec![2]);
    assert_eq!(program.constraints.args, vec![3, 0, 1, 0, 0, 12, 0, 0]);
    assert_eq!(program.constraints.entries[0].destination_dimension, 3);
    assert_eq!(program.constraints.entries[0].destination_id, 0);
    assert_eq!(program.constraints.entries[0].first_row, 0);
    assert_eq!(program.constraints.entries[0].last_row, 3);
    assert_eq!(program.constraints.entries[0].temp1_count, 0);
    assert_eq!(program.constraints.entries[0].temp3_count, 1);
    assert!(program.constraints.entries[0].intermediate);
}

#[test]
fn lowers_copy_operations_as_add_zero() {
    let info = ExpressionInfo {
        hints: Vec::new(),
        expressions: vec![ExpressionCode {
            expression_id: 12,
            stage: 1,
            line: "copy expression".to_owned(),
            temporary_count: 2,
            destination: None,
            operations: vec![
                CodeOperation {
                    op: OperationKind::Add,
                    destination: CodeDestination::temporary(0, 1),
                    sources: vec![CodeOperand::public(0, 1), CodeOperand::number(5, 1)],
                },
                CodeOperation {
                    op: OperationKind::Copy,
                    destination: CodeDestination::temporary(1, 1),
                    sources: vec![CodeOperand::temporary(0, 1)],
                },
            ],
        }],
        constraints: Vec::new(),
    };

    let program =
        regular_program_from_expression_info(&info, &minimal_setup_info()).expect("program lowers");

    assert_eq!(program.expressions.max_tmp1, 1);
    assert_eq!(program.expressions.ops, vec![0, 0]);
    assert_eq!(
        program.expressions.args,
        vec![0, 0, 7, 0, 0, 8, 0, 0, 0, 0, 5, 0, 0, 8, 1, 0]
    );
    assert_eq!(program.expressions.numbers, vec![5, 0]);
    assert_eq!(program.expressions.entries[0].destination_id, 0);
    assert_eq!(program.expressions.entries[0].temp1_count, 1);
}

#[test]
fn rejects_temporary_sources_before_definition() {
    let info = ExpressionInfo {
        hints: Vec::new(),
        expressions: vec![ExpressionCode {
            expression_id: 12,
            stage: 1,
            line: "undefined temporary".to_owned(),
            temporary_count: 2,
            destination: None,
            operations: vec![
                CodeOperation {
                    op: OperationKind::Copy,
                    destination: CodeDestination::temporary(0, 1),
                    sources: vec![CodeOperand::number(5, 1)],
                },
                CodeOperation {
                    op: OperationKind::Add,
                    destination: CodeDestination::temporary(0, 1),
                    sources: vec![CodeOperand::temporary(0, 1), CodeOperand::temporary(1, 1)],
                },
            ],
        }],
        constraints: Vec::new(),
    };

    assert_eq!(
        regular_program_from_expression_info(&info, &minimal_setup_info()),
        Err(RegularProgramLoweringError::Expression(
            ExpressionInfoError::TemporaryReadBeforeWrite {
                temporary_id: 1,
                dimension: 1,
                operation_index: 1
            }
        ))
    );
}

#[test]
fn clamps_negative_frame_lower_bounds_to_first_row() {
    let info = ExpressionInfo {
        hints: Vec::new(),
        expressions: Vec::new(),
        constraints: vec![ConstraintCode {
            stage: 1,
            boundary: BoundaryKind::EveryFrame,
            offset_min: Some(-1),
            offset_max: Some(2),
            line: "bounded frame".to_owned(),
            intermediate: false,
            temporary_count: 1,
            operations: vec![CodeOperation {
                op: OperationKind::Add,
                destination: CodeDestination::temporary(0, 1),
                sources: vec![CodeOperand::public(0, 1), CodeOperand::number(0, 1)],
            }],
        }],
    };

    let program =
        regular_program_from_expression_info(&info, &minimal_setup_info()).expect("program lowers");

    assert_eq!(program.constraints.entries[0].first_row, 0);
    assert_eq!(program.constraints.entries[0].last_row, 2);
}

#[test]
fn lowers_regular_expression_helper_operands() {
    let info = ExpressionInfo {
        hints: Vec::new(),
        expressions: vec![ExpressionCode {
            expression_id: 13,
            stage: 1,
            line: "helper expression".to_owned(),
            temporary_count: 2,
            destination: None,
            operations: vec![
                CodeOperation {
                    op: OperationKind::Add,
                    destination: CodeDestination::temporary(0, 3),
                    sources: vec![CodeOperand::evaluation(0, 3), CodeOperand::constant(0, 1)],
                },
                CodeOperation {
                    op: OperationKind::Mul,
                    destination: CodeDestination::temporary(1, 3),
                    sources: vec![
                        CodeOperand::opening_denominator(0, Some(0), 3),
                        CodeOperand::boundary_zerofier(0, 1),
                    ],
                },
            ],
        }],
        constraints: Vec::new(),
    };

    let program =
        regular_program_from_expression_info(&info, &minimal_setup_info()).expect("program lowers");

    assert_eq!(program.expressions.ops, vec![1, 1]);
    assert_eq!(
        program.expressions.args,
        vec![0, 0, 13, 0, 0, 0, 0, 0, 2, 0, 4, 0, 0, 3, 1, 0]
    );
    assert_eq!(program.expressions.max_tmp3, 1);
    assert_eq!(program.expressions.entries[0].destination_dimension, 3);
    assert_eq!(program.expressions.entries[0].destination_id, 0);
}

#[test]
fn lowers_extension_number_operands() {
    let info = ExpressionInfo {
        hints: Vec::new(),
        expressions: vec![ExpressionCode {
            expression_id: 7,
            stage: 2,
            line: "constant quotient expression".to_owned(),
            temporary_count: 1,
            destination: None,
            operations: vec![CodeOperation {
                op: OperationKind::Add,
                destination: CodeDestination::temporary(0, 3),
                sources: vec![CodeOperand::number(10, 3), CodeOperand::number(0, 3)],
            }],
        }],
        constraints: Vec::new(),
    };

    let program =
        regular_program_from_expression_info(&info, &minimal_setup_info()).expect("program lowers");

    assert_eq!(program.expressions.ops, vec![2]);
    assert_eq!(program.expressions.args, vec![0, 0, 8, 0, 0, 8, 3, 0]);
    assert_eq!(program.expressions.numbers, vec![10, 0, 0, 0, 0, 0]);
    assert_eq!(program.expressions.max_tmp3, 1);
    assert_eq!(program.expressions.max_args, 8);
    assert_eq!(program.expressions.entries[0].destination_dimension, 3);
    assert_eq!(program.expressions.entries[0].temp3_count, 1);
}

#[test]
fn lowers_unit_and_group_value_sources() {
    let info = ExpressionInfo {
        hints: Vec::new(),
        expressions: Vec::new(),
        constraints: vec![ConstraintCode {
            stage: 2,
            boundary: BoundaryKind::EveryRow,
            offset_min: None,
            offset_max: None,
            line: "value constraint".to_owned(),
            intermediate: false,
            temporary_count: 1,
            operations: vec![CodeOperation {
                op: OperationKind::Add,
                destination: CodeDestination::temporary(0, 3),
                sources: vec![
                    CodeOperand::air_group_value(0, Some(2), Some(0), 3),
                    CodeOperand::air_value(0, Some(1), Some(0), 1),
                ],
            }],
        }],
    };

    let program = regular_program_from_expression_info(&info, &setup_info_with_values())
        .expect("program lowers");

    assert_eq!(program.constraints.ops, vec![1]);
    assert_eq!(program.constraints.args, vec![0, 0, 11, 0, 0, 9, 0, 0]);
}

#[test]
fn rejects_zero_length_unit_value_dimensions_when_lowering_sources() {
    let info = ExpressionInfo {
        hints: Vec::new(),
        expressions: Vec::new(),
        constraints: vec![ConstraintCode {
            stage: 2,
            boundary: BoundaryKind::EveryRow,
            offset_min: None,
            offset_max: None,
            line: "value constraint".to_owned(),
            intermediate: false,
            temporary_count: 1,
            operations: vec![CodeOperation {
                op: OperationKind::Copy,
                destination: CodeDestination::temporary(0, 1),
                sources: vec![CodeOperand::air_value(0, Some(1), Some(0), 1)],
            }],
        }],
    };
    let mut setup = setup_info_with_values();
    setup.unit_value_map = vec![
        StageValue {
            name: "unit.zero".to_owned(),
            stage: 1,
            lengths: vec![0],
        },
        StageValue {
            name: "unit.actual".to_owned(),
            stage: 1,
            lengths: Vec::new(),
        },
    ];

    assert_eq!(
        regular_program_from_expression_info(&info, &setup),
        Err(RegularProgramLoweringError::LengthOverflow)
    );
}

#[test]
fn builds_verifier_program_from_verifier_info() {
    let info = VerifierInfo {
        quotient: VerifierCode {
            expression_id: None,
            stage: None,
            line: "quotient check".to_owned(),
            temporary_count: 1,
            operations: vec![VerifierOperation {
                op: VerifierOperationKind::Copy,
                destination: VerifierDestination::temporary(0, 3),
                sources: vec![VerifierOperand::number(1, 1)],
            }],
        },
        query: VerifierCode {
            expression_id: Some(9),
            stage: Some(3),
            line: "query check".to_owned(),
            temporary_count: 1,
            operations: vec![VerifierOperation {
                op: VerifierOperationKind::Add,
                destination: VerifierDestination::temporary(0, 3),
                sources: vec![
                    VerifierOperand::boundary_zerofier(1, 1),
                    VerifierOperand::proof_value(1, 3),
                ],
            }],
        },
    };

    let program = verifier_program_from_verifier_info(
        &info,
        &minimal_setup_info(),
        &global_info_with_values(),
    )
    .expect("verifier program should lower");

    assert_eq!(program.max_tmp1, 0);
    assert_eq!(program.max_tmp3, 1);
    assert_eq!(program.max_args, 8);
    assert_eq!(program.max_ops, 1);
    assert_eq!(program.ops, vec![1, 1]);
    assert_eq!(
        program.args,
        vec![0, 0, 8, 1, 0, 8, 0, 0, 0, 0, 10, 1, 0, 3, 2, 0]
    );
    assert_eq!(program.numbers, vec![1, 0, 0, 0]);
    assert_eq!(program.entries[0].expression_id, 0);
    assert_eq!(program.entries[0].stage, 2);
    assert_eq!(program.entries[0].source_line, "quotient check");
    assert_eq!(program.entries[1].expression_id, 9);
    assert_eq!(program.entries[1].stage, 3);
    assert_eq!(program.entries[1].source_line, "query check");
}

#[test]
fn rejects_mixed_verifier_temporary_dimensions_when_lowering() {
    let info = VerifierInfo {
        quotient: VerifierCode {
            expression_id: None,
            stage: None,
            line: "quotient check".to_owned(),
            temporary_count: 1,
            operations: vec![
                VerifierOperation {
                    op: VerifierOperationKind::Copy,
                    destination: VerifierDestination::temporary(0, 1),
                    sources: vec![VerifierOperand::number(1, 1)],
                },
                VerifierOperation {
                    op: VerifierOperationKind::Copy,
                    destination: VerifierDestination::temporary(0, 3),
                    sources: vec![VerifierOperand::number(1, 1)],
                },
            ],
        },
        query: VerifierCode {
            expression_id: None,
            stage: None,
            line: "query check".to_owned(),
            temporary_count: 1,
            operations: vec![VerifierOperation {
                op: VerifierOperationKind::Copy,
                destination: VerifierDestination::temporary(0, 3),
                sources: vec![VerifierOperand::number(1, 1)],
            }],
        },
    };

    assert_eq!(
        verifier_program_from_verifier_info(
            &info,
            &minimal_setup_info(),
            &global_info_with_values()
        ),
        Err(RegularProgramLoweringError::Verifier(
            VerifierInfoError::TemporaryDimensionMismatch {
                temporary_id: 0,
                expected_dimension: 1,
                found_dimension: 3,
                operation_index: 1
            }
        ))
    );
}

#[test]
fn lowers_verifier_proof_value_offsets_with_array_lengths() {
    let info = VerifierInfo {
        quotient: VerifierCode {
            expression_id: None,
            stage: None,
            line: "quotient check".to_owned(),
            temporary_count: 1,
            operations: vec![VerifierOperation {
                op: VerifierOperationKind::Copy,
                destination: VerifierDestination::temporary(0, 3),
                sources: vec![VerifierOperand::number(1, 1)],
            }],
        },
        query: VerifierCode {
            expression_id: Some(9),
            stage: Some(3),
            line: "query check".to_owned(),
            temporary_count: 1,
            operations: vec![VerifierOperation {
                op: VerifierOperationKind::Add,
                destination: VerifierDestination::temporary(0, 3),
                sources: vec![
                    VerifierOperand::boundary_zerofier(1, 1),
                    VerifierOperand::proof_value(1, 3),
                ],
            }],
        },
    };
    let mut global = global_info_with_values();
    global.proof_values_map[0].lengths = vec![2];

    let program = verifier_program_from_verifier_info(&info, &minimal_setup_info(), &global)
        .expect("verifier program should lower");

    assert_eq!(&program.args[10..14], &[10, 2, 0, 3]);
}

#[test]
fn rejects_zero_length_verifier_proof_value_dimensions() {
    let info = VerifierInfo {
        quotient: VerifierCode {
            expression_id: None,
            stage: None,
            line: "quotient check".to_owned(),
            temporary_count: 1,
            operations: vec![VerifierOperation {
                op: VerifierOperationKind::Copy,
                destination: VerifierDestination::temporary(0, 3),
                sources: vec![VerifierOperand::number(1, 1)],
            }],
        },
        query: VerifierCode {
            expression_id: Some(9),
            stage: Some(3),
            line: "query check".to_owned(),
            temporary_count: 1,
            operations: vec![VerifierOperation {
                op: VerifierOperationKind::Copy,
                destination: VerifierDestination::temporary(0, 3),
                sources: vec![VerifierOperand::proof_value(1, 3)],
            }],
        },
    };
    let mut global = global_info_with_values();
    global.proof_values_map[0].lengths = vec![0];

    assert_eq!(
        verifier_program_from_verifier_info(&info, &minimal_setup_info(), &global),
        Err(RegularProgramLoweringError::LengthOverflow)
    );
}

fn minimal_setup_info() -> UnitSetupInfo {
    UnitSetupInfo {
        n_stages: 1,
        n_constants: 0,
        constant_columns: Vec::new(),
        n_publics: Some(1),
        n_constraints: Some(1),
        q_degree: 1,
        opening_points: vec![0],
        section_widths: BTreeMap::new(),
        challenge_count: 0,
        eval_count: 0,
        evaluation_map: Vec::new(),
        boundaries: Vec::new(),
        commitment_columns: Vec::new(),
        unit_value_map: Vec::new(),
        group_value_map: Vec::new(),
        stark: StarkStruct {
            n_bits: 2,
            n_bits_ext: 3,
            n_queries: 1,
            steps: vec![FriStep { n_bits: 3 }],
            hash_commits: false,
            last_level_verification: 0,
            pow_bits: 0,
            merkle_tree_arity: 2,
            verification_hash_type: None,
            transcript_arity: None,
            merkle_tree_custom: None,
        },
    }
}

fn global_info_with_values() -> GlobalInfo {
    GlobalInfo {
        name: "sample-program".to_owned(),
        air_groups: vec!["group-a".to_owned()],
        airs: vec![vec![GlobalAir {
            name: "unit-a".to_owned(),
            num_rows: 4,
            has_compressor: false,
        }]],
        curve: CurveKind::None,
        lattice_size: Some(4),
        aggregation_types: vec![Vec::<AggregationType>::new()],
        n_publics: 1,
        num_challenges: vec![1],
        num_proof_values: vec![2],
        proof_values_map: vec![
            NamedStageValue {
                name: "proof-a".to_owned(),
                stage: 1,
                id: None,
                lengths: Vec::new(),
            },
            NamedStageValue {
                name: "proof-b".to_owned(),
                stage: 2,
                id: None,
                lengths: Vec::new(),
            },
        ],
        publics_map: vec![PublicValue {
            name: "public-a".to_owned(),
            stage: 1,
            lengths: Vec::new(),
        }],
        transcript_arity: 4,
    }
}

fn setup_info_with_commitment() -> UnitSetupInfo {
    let mut setup = minimal_setup_info();
    setup.commitment_columns = vec![CommitmentColumn {
        name: "stage.alpha".to_owned(),
        stage: 1,
        dimension: 3,
        pols_map_id: 0,
        stage_id: 0,
        stage_position: 0,
        intermediate: false,
        lengths: Vec::new(),
    }];
    setup
}

fn setup_info_with_values() -> UnitSetupInfo {
    let mut setup = minimal_setup_info();
    setup.unit_value_map = vec![StageValue {
        name: "unit.alpha".to_owned(),
        stage: 1,
        lengths: Vec::new(),
    }];
    setup.group_value_map = vec![StageValue {
        name: "group.alpha".to_owned(),
        stage: 2,
        lengths: Vec::new(),
    }];
    setup
}
