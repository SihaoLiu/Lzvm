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
    encode_regular_program, parse_regular_program, read_regular_program_file, RegularProgram,
};
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
    std::env::temp_dir().join(format!(
        "lzvm-regular-program-{}-{name}",
        std::process::id()
    ))
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
