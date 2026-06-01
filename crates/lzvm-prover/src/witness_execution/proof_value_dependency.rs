use lzvm_artifacts::constraint_program::ConstraintProgram;
use lzvm_artifacts::hint_program::{HintOperand, HintProgram};

pub(super) fn regular_program_uses_proof_values(
    stage_count: u16,
    constraints: &ConstraintProgram,
    hints: &HintProgram,
) -> bool {
    let Some(proof_value_buffer) = stage_count.checked_add(9) else {
        return true;
    };
    regular_constraints_use_proof_values(constraints, proof_value_buffer)
        || regular_hints_use_proof_values(hints)
}

fn regular_constraints_use_proof_values(
    constraints: &ConstraintProgram,
    proof_value_buffer: u16,
) -> bool {
    for entry in &constraints.entries {
        let Some(args_offset) = usize::try_from(entry.args_offset).ok() else {
            return true;
        };
        let Some(args_count) = usize::try_from(entry.args_count).ok() else {
            return true;
        };
        let Some(args_end) = args_offset.checked_add(args_count) else {
            return true;
        };
        let Some(args) = constraints.args.get(args_offset..args_end) else {
            return true;
        };
        let mut chunks = args.chunks_exact(8);
        if chunks.any(|chunk| chunk[2] == proof_value_buffer || chunk[5] == proof_value_buffer) {
            return true;
        }
        if !chunks.remainder().is_empty() {
            return true;
        }
    }
    false
}

fn regular_hints_use_proof_values(hints: &HintProgram) -> bool {
    hints.hints.iter().any(|hint| {
        hint.fields.iter().any(|field| {
            field
                .values
                .iter()
                .any(|value| matches!(value.operand, HintOperand::ProofValue { .. }))
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use lzvm_artifacts::constraint_program::{ConstraintEntry, ConstraintProgram};
    use lzvm_artifacts::hint_program::{Hint, HintField, HintOperand, HintProgram, HintValue};

    #[test]
    fn detects_regular_constraint_proof_value_source() {
        let constraints = constraint_program_with_source_buffer(10);
        let hints = HintProgram { hints: Vec::new() };

        assert!(regular_program_uses_proof_values(1, &constraints, &hints));
    }

    #[test]
    fn detects_regular_hint_proof_value_operand() {
        let constraints = constraint_program_with_source_buffer(0);
        let hints = HintProgram {
            hints: vec![Hint {
                name: "source.lookup.proves".to_owned(),
                fields: vec![HintField {
                    name: "weight".to_owned(),
                    values: vec![HintValue {
                        operand: HintOperand::ProofValue { id: 0 },
                        positions: vec![0],
                    }],
                }],
            }],
        };

        assert!(regular_program_uses_proof_values(1, &constraints, &hints));
    }

    #[test]
    fn ignores_programs_without_proof_value_inputs() {
        let constraints = constraint_program_with_source_buffer(0);
        let hints = HintProgram { hints: Vec::new() };

        assert!(!regular_program_uses_proof_values(1, &constraints, &hints));
    }

    fn constraint_program_with_source_buffer(source_buffer: u16) -> ConstraintProgram {
        ConstraintProgram {
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
                source_line: "result".to_owned(),
            }],
            ops: vec![0],
            args: vec![0, 0, source_buffer, 0, 0, 3, 0, 0],
            numbers: Vec::new(),
        }
    }
}
