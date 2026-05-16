use lzvm_artifacts::hint_program::{HintOperand, HintProgram};

use super::{GlobalHintInputRequirements, RegularHintInputRequirements};

pub fn global_hint_input_requirements(program: &HintProgram) -> GlobalHintInputRequirements {
    let mut requirements = GlobalHintInputRequirements::default();
    for_hint_operand(program, |operand| match operand {
        HintOperand::Public { .. } => requirements.publics = true,
        HintOperand::ProofValue { .. } => requirements.proof_values = true,
        HintOperand::Challenge { .. } => requirements.challenges = true,
        HintOperand::GroupValue { .. } => requirements.group_values = true,
        _ => {}
    });
    requirements
}

pub fn regular_hint_input_requirements(program: &HintProgram) -> RegularHintInputRequirements {
    let mut requirements = RegularHintInputRequirements::default();
    for_hint_operand(program, |operand| match operand {
        HintOperand::Constant { .. } => requirements.fixed_columns = true,
        HintOperand::Commitment { .. } => requirements.stage_columns = true,
        HintOperand::Public { .. } => requirements.publics = true,
        HintOperand::AirValue { .. } => requirements.unit_values = true,
        HintOperand::ProofValue { .. } => requirements.proof_values = true,
        HintOperand::AirGroupValue { .. } => requirements.group_values = true,
        HintOperand::Challenge { .. } => requirements.challenges = true,
        _ => {}
    });
    requirements
}

fn for_hint_operand(program: &HintProgram, mut f: impl FnMut(&HintOperand)) {
    for hint in &program.hints {
        for field in &hint.fields {
            for value in &field.values {
                f(&value.operand);
            }
        }
    }
}
