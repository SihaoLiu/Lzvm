use lzvm_prover::guest_instruction::{
    RiscvBranchKind, RiscvInstruction, RiscvLoadKind, RiscvOpImmKind, RiscvOpKind, RiscvStoreKind,
};
use lzvm_prover::guest_machine::GuestMachineReport;
use lzvm_prover::zisk_main::{
    lower_guest_report, ZiskMainLowerError, ZiskMainOp, ZiskMainSource, ZiskMainStore,
};

const PC: u64 = 0x8000_0000;

#[test]
fn lowers_addi_from_zero_as_copy_immediate() {
    let instruction = lower_guest_report(&report(
        4,
        RiscvInstruction::OpImm {
            kind: RiscvOpImmKind::Addi,
            rd: 3,
            rs1: 0,
            immediate: 17,
        },
    ))
    .expect("addi should lower");

    assert_eq!(instruction.pc, PC);
    assert_eq!(instruction.a, ZiskMainSource::Immediate(0));
    assert_eq!(instruction.b, ZiskMainSource::Immediate(17));
    assert_eq!(instruction.op, ZiskMainOp::CopyB);
    assert_eq!(instruction.store, ZiskMainStore::Register(3));
    assert_eq!(instruction.jmp_offset1, 4);
    assert_eq!(instruction.jmp_offset2, 4);
}

#[test]
fn lowers_register_add_as_binary_op() {
    let instruction = lower_guest_report(&report(
        4,
        RiscvInstruction::Op {
            kind: RiscvOpKind::Add,
            rd: 6,
            rs1: 4,
            rs2: 5,
        },
    ))
    .expect("add should lower");

    assert_eq!(instruction.a, ZiskMainSource::Register(4));
    assert_eq!(instruction.b, ZiskMainSource::Register(5));
    assert_eq!(instruction.op, ZiskMainOp::Add);
    assert_eq!(instruction.store, ZiskMainStore::Register(6));
}

#[test]
fn lowers_register_alu_ops_as_binary_ops() {
    let cases = [
        (RiscvOpKind::Sub, ZiskMainOp::Sub),
        (RiscvOpKind::Sll, ZiskMainOp::Sll),
        (RiscvOpKind::Slt, ZiskMainOp::Lt),
        (RiscvOpKind::Sltu, ZiskMainOp::Ltu),
        (RiscvOpKind::Xor, ZiskMainOp::Xor),
        (RiscvOpKind::Srl, ZiskMainOp::Srl),
        (RiscvOpKind::Sra, ZiskMainOp::Sra),
        (RiscvOpKind::Or, ZiskMainOp::Or),
        (RiscvOpKind::And, ZiskMainOp::And),
    ];

    for (kind, op) in cases {
        let instruction = lower_guest_report(&report(
            4,
            RiscvInstruction::Op {
                kind,
                rd: 6,
                rs1: 4,
                rs2: 5,
            },
        ))
        .expect("register ALU op should lower");

        assert_eq!(instruction.a, ZiskMainSource::Register(4));
        assert_eq!(instruction.b, ZiskMainSource::Register(5));
        assert_eq!(instruction.op, op);
        assert_eq!(instruction.store, ZiskMainStore::Register(6));
    }
}

#[test]
fn lowers_immediate_alu_ops_as_binary_ops() {
    let cases = [
        (RiscvOpImmKind::Slti, ZiskMainOp::Lt),
        (RiscvOpImmKind::Sltiu, ZiskMainOp::Ltu),
        (RiscvOpImmKind::Xori, ZiskMainOp::Xor),
        (RiscvOpImmKind::Ori, ZiskMainOp::Or),
        (RiscvOpImmKind::Andi, ZiskMainOp::And),
        (RiscvOpImmKind::Slli, ZiskMainOp::Sll),
        (RiscvOpImmKind::Srli, ZiskMainOp::Srl),
        (RiscvOpImmKind::Srai, ZiskMainOp::Sra),
    ];

    for (kind, op) in cases {
        let instruction = lower_guest_report(&report(
            4,
            RiscvInstruction::OpImm {
                kind,
                rd: 6,
                rs1: 4,
                immediate: -3,
            },
        ))
        .expect("immediate ALU op should lower");

        assert_eq!(instruction.a, ZiskMainSource::Register(4));
        assert_eq!(instruction.b, ZiskMainSource::Immediate((-3_i64) as u64));
        assert_eq!(instruction.op, op);
        assert_eq!(instruction.store, ZiskMainStore::Register(6));
    }
}

#[test]
fn uses_zisk_alu_op_codes() {
    assert_eq!(ZiskMainOp::Ltu.code(), 0x06);
    assert_eq!(ZiskMainOp::Lt.code(), 0x07);
    assert_eq!(ZiskMainOp::Sub.code(), 0x0b);
    assert_eq!(ZiskMainOp::And.code(), 0x0e);
    assert_eq!(ZiskMainOp::Or.code(), 0x0f);
    assert_eq!(ZiskMainOp::Xor.code(), 0x10);
    assert_eq!(ZiskMainOp::Sll.code(), 0x21);
    assert_eq!(ZiskMainOp::Srl.code(), 0x22);
    assert_eq!(ZiskMainOp::Sra.code(), 0x23);
}

#[test]
fn lowers_branch_ops_as_pc_relative_flag_offsets() {
    let cases = [
        (RiscvBranchKind::Beq, 0x09, 12, 4),
        (RiscvBranchKind::Bne, 0x09, 4, 12),
        (RiscvBranchKind::Blt, ZiskMainOp::Lt.code(), 12, 4),
        (RiscvBranchKind::Bge, ZiskMainOp::Lt.code(), 4, 12),
        (RiscvBranchKind::Bltu, ZiskMainOp::Ltu.code(), 12, 4),
        (RiscvBranchKind::Bgeu, ZiskMainOp::Ltu.code(), 4, 12),
    ];

    for (kind, op_code, jmp_offset1, jmp_offset2) in cases {
        let instruction = lower_guest_report(&report_with_next_pc(
            4,
            PC + 12,
            RiscvInstruction::Branch {
                kind,
                rs1: 4,
                rs2: 5,
                offset: 12,
            },
        ))
        .expect("branch should lower");

        assert_eq!(instruction.a, ZiskMainSource::Register(4));
        assert_eq!(instruction.b, ZiskMainSource::Register(5));
        assert_eq!(instruction.op.code(), op_code);
        assert_eq!(instruction.store, ZiskMainStore::None);
        assert!(!instruction.store_pc);
        assert!(!instruction.set_pc);
        assert_eq!(instruction.jmp_offset1, jmp_offset1);
        assert_eq!(instruction.jmp_offset2, jmp_offset2);
    }
}

#[test]
fn lowers_doubleword_load_as_indirect_copy_to_register() {
    let instruction = lower_guest_report(&report(
        4,
        RiscvInstruction::Load {
            kind: RiscvLoadKind::Ld,
            rd: 7,
            rs1: 4,
            offset: 16,
        },
    ))
    .expect("ld should lower");

    assert_eq!(instruction.a, ZiskMainSource::Register(4));
    assert_eq!(instruction.b, ZiskMainSource::Indirect(16));
    assert_eq!(instruction.op, ZiskMainOp::CopyB);
    assert_eq!(instruction.store, ZiskMainStore::Register(7));
    assert_eq!(instruction.ind_width, 8);
}

#[test]
fn lowers_doubleword_store_as_register_copy_to_indirect_store() {
    let instruction = lower_guest_report(&report(
        4,
        RiscvInstruction::Store {
            kind: RiscvStoreKind::Sd,
            rs1: 4,
            rs2: 7,
            offset: -8,
        },
    ))
    .expect("sd should lower");

    assert_eq!(instruction.a, ZiskMainSource::Register(4));
    assert_eq!(instruction.b, ZiskMainSource::Register(7));
    assert_eq!(instruction.op, ZiskMainOp::CopyB);
    assert_eq!(instruction.store, ZiskMainStore::Indirect(-8));
    assert_eq!(instruction.ind_width, 8);
}

#[test]
fn lowers_unsigned_loads_as_indirect_copies_to_register() {
    let cases = [
        (RiscvLoadKind::Lbu, 1),
        (RiscvLoadKind::Lhu, 2),
        (RiscvLoadKind::Lwu, 4),
    ];

    for (kind, width) in cases {
        let instruction = lower_guest_report(&report(
            4,
            RiscvInstruction::Load {
                kind,
                rd: 7,
                rs1: 4,
                offset: 12,
            },
        ))
        .expect("unsigned load should lower");

        assert_eq!(instruction.a, ZiskMainSource::Register(4));
        assert_eq!(instruction.b, ZiskMainSource::Indirect(12));
        assert_eq!(instruction.op, ZiskMainOp::CopyB);
        assert_eq!(instruction.store, ZiskMainStore::Register(7));
        assert_eq!(instruction.ind_width, width);
    }
}

#[test]
fn lowers_signed_loads_as_indirect_sign_extension_to_register() {
    let cases = [
        (RiscvLoadKind::Lb, ZiskMainOp::SignExtendB, 1),
        (RiscvLoadKind::Lh, ZiskMainOp::SignExtendH, 2),
        (RiscvLoadKind::Lw, ZiskMainOp::SignExtendW, 4),
    ];

    for (kind, op, width) in cases {
        let instruction = lower_guest_report(&report(
            4,
            RiscvInstruction::Load {
                kind,
                rd: 7,
                rs1: 4,
                offset: 12,
            },
        ))
        .expect("signed load should lower");

        assert_eq!(instruction.a, ZiskMainSource::Register(4));
        assert_eq!(instruction.b, ZiskMainSource::Indirect(12));
        assert_eq!(instruction.op, op);
        assert_eq!(instruction.store, ZiskMainStore::Register(7));
        assert_eq!(instruction.ind_width, width);
    }
}

#[test]
fn uses_zisk_sign_extension_op_codes() {
    assert_eq!(ZiskMainOp::SignExtendB.code(), 0x27);
    assert_eq!(ZiskMainOp::SignExtendH.code(), 0x28);
    assert_eq!(ZiskMainOp::SignExtendW.code(), 0x29);
}

#[test]
fn lowers_narrow_stores_as_register_copy_to_indirect_store() {
    let cases = [
        (RiscvStoreKind::Sb, 1),
        (RiscvStoreKind::Sh, 2),
        (RiscvStoreKind::Sw, 4),
    ];

    for (kind, width) in cases {
        let instruction = lower_guest_report(&report(
            4,
            RiscvInstruction::Store {
                kind,
                rs1: 4,
                rs2: 7,
                offset: -12,
            },
        ))
        .expect("narrow store should lower");

        assert_eq!(instruction.a, ZiskMainSource::Register(4));
        assert_eq!(instruction.b, ZiskMainSource::Register(7));
        assert_eq!(instruction.op, ZiskMainOp::CopyB);
        assert_eq!(instruction.store, ZiskMainStore::Indirect(-12));
        assert_eq!(instruction.ind_width, width);
    }
}

#[test]
fn uses_reported_instruction_size_for_jump_offsets() {
    let instruction = lower_guest_report(&report(
        2,
        RiscvInstruction::OpImm {
            kind: RiscvOpImmKind::Addi,
            rd: 1,
            rs1: 1,
            immediate: -1,
        },
    ))
    .expect("compressed addi should lower with its decoded size");

    assert_eq!(instruction.b, ZiskMainSource::Immediate(u64::MAX));
    assert_eq!(instruction.jmp_offset1, 2);
    assert_eq!(instruction.jmp_offset2, 2);
}

#[test]
fn rejects_unsupported_main_lowering_instruction() {
    let error = lower_guest_report(&report(4, RiscvInstruction::Ecall))
        .expect_err("unsupported lowering should fail");

    assert_eq!(
        error,
        ZiskMainLowerError::UnsupportedInstruction {
            instruction: RiscvInstruction::Ecall
        }
    );
}

#[test]
fn rejects_invalid_instruction_byte_length() {
    let error = lower_guest_report(&report(
        3,
        RiscvInstruction::OpImm {
            kind: RiscvOpImmKind::Addi,
            rd: 1,
            rs1: 0,
            immediate: 0,
        },
    ))
    .expect_err("invalid instruction size should fail");

    assert_eq!(
        error,
        ZiskMainLowerError::InvalidInstructionByteLen {
            pc: PC,
            byte_len: 3
        }
    );
}

#[test]
fn rejects_inconsistent_sequential_next_pc() {
    let error = lower_guest_report(&report_with_next_pc(
        4,
        PC + 8,
        RiscvInstruction::OpImm {
            kind: RiscvOpImmKind::Addi,
            rd: 1,
            rs1: 0,
            immediate: 0,
        },
    ))
    .expect_err("mismatched sequential next pc should fail");

    assert_eq!(
        error,
        ZiskMainLowerError::InconsistentSequentialNextPc {
            pc: PC,
            next_pc: PC + 8,
            instruction_byte_len: 4
        }
    );
}

fn report(instruction_byte_len: usize, instruction: RiscvInstruction) -> GuestMachineReport {
    report_with_next_pc(
        instruction_byte_len,
        PC + instruction_byte_len as u64,
        instruction,
    )
}

fn report_with_next_pc(
    instruction_byte_len: usize,
    next_pc: u64,
    instruction: RiscvInstruction,
) -> GuestMachineReport {
    GuestMachineReport {
        address: PC,
        instruction_byte_len,
        instruction,
        next_pc,
        register_writes: Vec::new(),
        memory_accesses: Vec::new(),
    }
}
