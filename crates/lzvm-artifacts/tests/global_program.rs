use lzvm_artifacts::constraint_program::parse_global_constraint_program;
use lzvm_artifacts::constraint_program::{GlobalConstraintEntry, GlobalConstraintProgram};
use lzvm_artifacts::global_program::{
    encode_global_program, parse_global_program, read_global_program_file, GlobalProgram,
};
use lzvm_artifacts::hint_program::{
    parse_global_hint_program, Hint, HintField, HintOperand, HintProgram, HintValue,
};
use std::fs;
use std::path::PathBuf;

fn sample_global_program() -> GlobalProgram {
    GlobalProgram {
        constraints: GlobalConstraintProgram {
            entries: vec![GlobalConstraintEntry {
                destination_dimension: 3,
                destination_id: 5,
                temp1_count: 7,
                temp3_count: 11,
                ops_count: 2,
                ops_offset: 0,
                args_count: 2,
                args_offset: 0,
                source_line: "global-a".to_owned(),
            }],
            ops: vec![1, 2],
            args: vec![10, 11],
            numbers: vec![20],
        },
        hints: HintProgram {
            hints: vec![Hint {
                name: "hint-a".to_owned(),
                fields: vec![HintField {
                    name: "field-a".to_owned(),
                    values: vec![HintValue {
                        operand: HintOperand::GroupValue { group_id: 2, id: 3 },
                        positions: vec![5, 8],
                    }],
                }],
            }],
        },
    }
}

fn temp_file_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("lzvm-global-program-{}-{name}", std::process::id()))
}

#[test]
fn encodes_and_parses_combined_global_program_sections() {
    let program = sample_global_program();
    let encoded = encode_global_program(&program).expect("program should encode");

    assert_eq!(
        parse_global_constraint_program(&encoded).expect("constraints should parse"),
        program.constraints
    );
    assert_eq!(
        parse_global_hint_program(&encoded).expect("hints should parse"),
        program.hints
    );
    assert_eq!(
        parse_global_program(&encoded).expect("program should parse"),
        program
    );
}

#[test]
fn reads_global_programs_from_a_file_path() {
    let path = temp_file_path("program.bin");
    let program = sample_global_program();
    fs::write(
        &path,
        encode_global_program(&program).expect("program should encode"),
    )
    .expect("program file should be written");

    let parsed = read_global_program_file(&path).expect("program should parse");
    fs::remove_file(&path).expect("program file should be removed");

    assert_eq!(parsed, program);
}
