use lzvm_prover::guest_instruction::{
    RiscvAmoWidth, RiscvBranchKind, RiscvCsr, RiscvDmaKind, RiscvFenceKind, RiscvInstruction,
    RiscvLoadKind, RiscvOp32Kind, RiscvOpImm32Kind, RiscvOpImmKind, RiscvOpKind,
    RiscvPrecompileKind, RiscvStoreKind,
};
use lzvm_prover::guest_machine::{GuestMachineReport, ZISK_ARCHITECTURE_ID};
use lzvm_prover::zisk_fcalls::ZISK_INPUT_ADDRESS;
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
    assert!(instruction.is_external_op);
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
        assert!(instruction.is_external_op);
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
        assert!(instruction.is_external_op);
    }
}

#[test]
fn lowers_immediate_word_ops_as_m32_binary_ops() {
    let cases = [
        (RiscvOpImm32Kind::Addiw, -3, 0x1a),
        (RiscvOpImm32Kind::Slliw, 31, 0x24),
        (RiscvOpImm32Kind::Srliw, 31, 0x25),
        (RiscvOpImm32Kind::Sraiw, 31, 0x26),
    ];

    for (kind, immediate, op_code) in cases {
        let instruction = lower_guest_report(&report(
            4,
            RiscvInstruction::OpImm32 {
                kind,
                rd: 6,
                rs1: 4,
                immediate,
            },
        ))
        .expect("immediate word ALU op should lower");

        assert_eq!(instruction.a, ZiskMainSource::Register(4));
        assert_eq!(instruction.b, ZiskMainSource::Immediate(immediate as u64));
        assert_eq!(instruction.op.code(), op_code);
        assert_eq!(instruction.store, ZiskMainStore::Register(6));
        assert!(instruction.m32);
        assert!(instruction.is_external_op);
    }
}

#[test]
fn lowers_register_word_ops_as_m32_binary_ops() {
    let cases = [
        (RiscvOp32Kind::Addw, 0x1a),
        (RiscvOp32Kind::Subw, 0x1b),
        (RiscvOp32Kind::Sllw, 0x24),
        (RiscvOp32Kind::Srlw, 0x25),
        (RiscvOp32Kind::Sraw, 0x26),
    ];

    for (kind, op_code) in cases {
        let instruction = lower_guest_report(&report(
            4,
            RiscvInstruction::Op32 {
                kind,
                rd: 6,
                rs1: 4,
                rs2: 5,
            },
        ))
        .expect("register word ALU op should lower");

        assert_eq!(instruction.a, ZiskMainSource::Register(4));
        assert_eq!(instruction.b, ZiskMainSource::Register(5));
        assert_eq!(instruction.op.code(), op_code);
        assert_eq!(instruction.store, ZiskMainStore::Register(6));
        assert!(instruction.m32);
        assert!(instruction.is_external_op);
    }
}

#[test]
fn lowers_register_m_extension_ops_as_external_arith_ops() {
    let cases = [
        (RiscvOpKind::Mul, 0xb4),
        (RiscvOpKind::Mulh, 0xb5),
        (RiscvOpKind::Mulhsu, 0xb3),
        (RiscvOpKind::Mulhu, 0xb1),
        (RiscvOpKind::Div, 0xba),
        (RiscvOpKind::Divu, 0xb8),
        (RiscvOpKind::Rem, 0xbb),
        (RiscvOpKind::Remu, 0xb9),
    ];

    for (kind, op_code) in cases {
        let instruction = lower_guest_report(&report(
            4,
            RiscvInstruction::Op {
                kind,
                rd: 6,
                rs1: 4,
                rs2: 5,
            },
        ))
        .expect("register M extension op should lower");

        assert_eq!(instruction.a, ZiskMainSource::Register(4));
        assert_eq!(instruction.b, ZiskMainSource::Register(5));
        assert_eq!(instruction.op.code(), op_code);
        assert_eq!(instruction.store, ZiskMainStore::Register(6));
        assert!(instruction.is_external_op);
        assert!(!instruction.m32);
    }
}

#[test]
fn lowers_register_word_m_extension_ops_as_external_arith_ops() {
    let cases = [
        (RiscvOp32Kind::Mulw, 0xb6),
        (RiscvOp32Kind::Divw, 0xbe),
        (RiscvOp32Kind::Divuw, 0xbc),
        (RiscvOp32Kind::Remw, 0xbf),
        (RiscvOp32Kind::Remuw, 0xbd),
    ];

    for (kind, op_code) in cases {
        let instruction = lower_guest_report(&report(
            4,
            RiscvInstruction::Op32 {
                kind,
                rd: 6,
                rs1: 4,
                rs2: 5,
            },
        ))
        .expect("register word M extension op should lower");

        assert_eq!(instruction.a, ZiskMainSource::Register(4));
        assert_eq!(instruction.b, ZiskMainSource::Register(5));
        assert_eq!(instruction.op.code(), op_code);
        assert_eq!(instruction.store, ZiskMainStore::Register(6));
        assert!(instruction.is_external_op);
        assert!(instruction.m32);
    }
}

#[test]
fn lowers_discarded_m_extension_ops_as_noop_rows() {
    let register_cases = [
        RiscvOpKind::Mul,
        RiscvOpKind::Mulh,
        RiscvOpKind::Mulhsu,
        RiscvOpKind::Mulhu,
        RiscvOpKind::Div,
        RiscvOpKind::Divu,
        RiscvOpKind::Rem,
        RiscvOpKind::Remu,
    ];

    for kind in register_cases {
        let instruction = lower_guest_report(&report(
            4,
            RiscvInstruction::Op {
                kind,
                rd: 0,
                rs1: 4,
                rs2: 5,
            },
        ))
        .expect("discarded M extension op should lower");

        assert_eq!(instruction.a, ZiskMainSource::Immediate(0));
        assert_eq!(instruction.b, ZiskMainSource::Immediate(0));
        assert_eq!(instruction.op, ZiskMainOp::Flag);
        assert_eq!(instruction.store, ZiskMainStore::None);
        assert!(!instruction.store_pc);
        assert!(!instruction.set_pc);
        assert_eq!(instruction.jmp_offset1, 4);
        assert_eq!(instruction.jmp_offset2, 4);
        assert!(!instruction.is_external_op);
    }

    let word_cases = [
        RiscvOp32Kind::Mulw,
        RiscvOp32Kind::Divw,
        RiscvOp32Kind::Divuw,
        RiscvOp32Kind::Remw,
        RiscvOp32Kind::Remuw,
    ];

    for kind in word_cases {
        let instruction = lower_guest_report(&report(
            4,
            RiscvInstruction::Op32 {
                kind,
                rd: 0,
                rs1: 4,
                rs2: 5,
            },
        ))
        .expect("discarded word M extension op should lower");

        assert_eq!(instruction.a, ZiskMainSource::Immediate(0));
        assert_eq!(instruction.b, ZiskMainSource::Immediate(0));
        assert_eq!(instruction.op, ZiskMainOp::Flag);
        assert_eq!(instruction.store, ZiskMainStore::None);
        assert!(!instruction.m32);
        assert!(!instruction.is_external_op);
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
    assert_eq!(ZiskMainOp::Mulhu.code(), 0xb1);
    assert_eq!(ZiskMainOp::Mulhsu.code(), 0xb3);
    assert_eq!(ZiskMainOp::Mul.code(), 0xb4);
    assert_eq!(ZiskMainOp::Mulh.code(), 0xb5);
    assert_eq!(ZiskMainOp::MulW.code(), 0xb6);
    assert_eq!(ZiskMainOp::Divu.code(), 0xb8);
    assert_eq!(ZiskMainOp::Remu.code(), 0xb9);
    assert_eq!(ZiskMainOp::Div.code(), 0xba);
    assert_eq!(ZiskMainOp::Rem.code(), 0xbb);
    assert_eq!(ZiskMainOp::DivuW.code(), 0xbc);
    assert_eq!(ZiskMainOp::RemuW.code(), 0xbd);
    assert_eq!(ZiskMainOp::DivW.code(), 0xbe);
    assert_eq!(ZiskMainOp::RemW.code(), 0xbf);
}

#[test]
fn lowers_zisk_precompile_ops_as_precompiled_rows() {
    let cases = [
        (RiscvPrecompileKind::Add256, 0xf0, 6),
        (RiscvPrecompileKind::Keccak, 0xf1, 0),
        (RiscvPrecompileKind::Arith256, 0xf2, 0),
        (RiscvPrecompileKind::Arith256Mod, 0xf3, 0),
        (RiscvPrecompileKind::Secp256k1Add, 0xf4, 0),
        (RiscvPrecompileKind::Secp256k1Dbl, 0xf5, 0),
    ];

    for (kind, op_code, rd) in cases {
        let instruction = lower_guest_report(&report(
            4,
            RiscvInstruction::ZiskPrecompile { kind, rs1: 4, rd },
        ))
        .expect("precompile op should lower");

        assert_eq!(instruction.a, ZiskMainSource::Immediate(0));
        assert_eq!(instruction.b, ZiskMainSource::Register(4));
        assert_eq!(instruction.op.code(), op_code);
        assert_eq!(instruction.store, register_store(rd));
        assert!(instruction.is_precompiled);
        assert!(instruction.is_external_op);
        assert!(!instruction.m32);
    }
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
fn lowers_lui_and_auipc_as_immediate_rows() {
    let lui = lower_guest_report(&report(
        4,
        RiscvInstruction::Lui {
            rd: 3,
            immediate: -4096,
        },
    ))
    .expect("lui should lower");

    assert_eq!(lui.a, ZiskMainSource::Immediate(0));
    assert_eq!(lui.b, ZiskMainSource::Immediate((-4096_i64) as u64));
    assert_eq!(lui.op, ZiskMainOp::CopyB);
    assert_eq!(lui.store, ZiskMainStore::Register(3));
    assert!(!lui.store_pc);
    assert!(!lui.set_pc);
    assert_eq!(lui.jmp_offset1, 4);
    assert_eq!(lui.jmp_offset2, 4);

    let auipc = lower_guest_report(&report(
        4,
        RiscvInstruction::Auipc {
            rd: 4,
            immediate: 0x3000,
        },
    ))
    .expect("auipc should lower");

    assert_eq!(auipc.a, ZiskMainSource::Immediate(0));
    assert_eq!(auipc.b, ZiskMainSource::Immediate(0));
    assert_eq!(auipc.op, ZiskMainOp::Flag);
    assert_eq!(auipc.store, ZiskMainStore::Register(4));
    assert!(auipc.store_pc);
    assert!(!auipc.set_pc);
    assert_eq!(auipc.jmp_offset1, 4);
    assert_eq!(auipc.jmp_offset2, 0x3000);

    let auipc_x0 = lower_guest_report(&report(
        4,
        RiscvInstruction::Auipc {
            rd: 0,
            immediate: 0x2000,
        },
    ))
    .expect("auipc to x0 should lower");

    assert_eq!(auipc_x0.store, ZiskMainStore::None);
    assert!(!auipc_x0.store_pc);
    assert_eq!(auipc_x0.jmp_offset1, 4);
    assert_eq!(auipc_x0.jmp_offset2, 0x2000);
}

#[test]
fn lowers_jump_rows_as_pc_store_control_flow() {
    let jal = lower_guest_report(&report_with_next_pc(
        4,
        PC + 12,
        RiscvInstruction::Jal { rd: 5, offset: 12 },
    ))
    .expect("jal should lower");

    assert_eq!(jal.a, ZiskMainSource::Immediate(0));
    assert_eq!(jal.b, ZiskMainSource::Immediate(0));
    assert_eq!(jal.op, ZiskMainOp::Flag);
    assert_eq!(jal.store, ZiskMainStore::Register(5));
    assert!(jal.store_pc);
    assert!(!jal.set_pc);
    assert_eq!(jal.jmp_offset1, 12);
    assert_eq!(jal.jmp_offset2, 4);

    let jalr = lower_guest_report(&report_with_next_pc(
        4,
        PC + 0x100,
        RiscvInstruction::Jalr {
            rd: 6,
            rs1: 7,
            offset: -8,
        },
    ))
    .expect("even-offset jalr should lower");

    assert_eq!(jalr.a, ZiskMainSource::Immediate(!1));
    assert_eq!(jalr.b, ZiskMainSource::Register(7));
    assert_eq!(jalr.op, ZiskMainOp::And);
    assert_eq!(jalr.store, ZiskMainStore::Register(6));
    assert!(jalr.store_pc);
    assert!(jalr.set_pc);
    assert_eq!(jalr.jmp_offset1, -8);
    assert_eq!(jalr.jmp_offset2, 4);
}

#[test]
fn lowers_pc_store_to_x0_without_register_store() {
    let instruction = lower_guest_report(&report_with_next_pc(
        4,
        PC + 12,
        RiscvInstruction::Jal { rd: 0, offset: 12 },
    ))
    .expect("jump to x0 should lower");

    assert_eq!(instruction.store, ZiskMainStore::None);
    assert!(!instruction.store_pc);
}

#[test]
fn lowers_fence_ops_as_noop_flag_rows() {
    let cases = [
        (RiscvFenceKind::Fence, 0, 0xf, 0xf),
        (RiscvFenceKind::FenceTso, 8, 3, 3),
        (RiscvFenceKind::FenceI, 0, 0, 0),
    ];

    for (kind, mode, predecessor, successor) in cases {
        let instruction = lower_guest_report(&report(
            4,
            RiscvInstruction::Fence {
                kind,
                mode,
                predecessor,
                successor,
            },
        ))
        .expect("fence should lower");

        assert_eq!(instruction.a, ZiskMainSource::Immediate(0));
        assert_eq!(instruction.b, ZiskMainSource::Immediate(0));
        assert_eq!(instruction.op, ZiskMainOp::Flag);
        assert_eq!(instruction.store, ZiskMainStore::None);
        assert!(!instruction.store_pc);
        assert!(!instruction.set_pc);
        assert_eq!(instruction.jmp_offset1, 4);
        assert_eq!(instruction.jmp_offset2, 4);
        assert_eq!(instruction.ind_width, 0);
        assert!(!instruction.m32);
    }
}

#[test]
fn lowers_representable_fixed_csr_reads_as_immediate_copies() {
    let cases = [
        (RiscvCsr::Mvendorid, 0),
        (RiscvCsr::Marchid, ZISK_ARCHITECTURE_ID),
        (RiscvCsr::Mimpid, 0),
        (RiscvCsr::Mhartid, 0),
    ];

    for (index, (csr, value)) in cases.into_iter().enumerate() {
        let rd = u8::try_from(index + 1).expect("test register should fit");
        let instruction = lower_guest_report(&report(4, RiscvInstruction::CsrRead { csr, rd }))
            .expect("fixed CSR read should lower");

        assert_eq!(instruction.a, ZiskMainSource::Immediate(0));
        assert_eq!(instruction.b, ZiskMainSource::Immediate(value));
        assert_eq!(instruction.op, ZiskMainOp::CopyB);
        assert_eq!(instruction.store, ZiskMainStore::Register(rd));
        assert_eq!(instruction.jmp_offset1, 4);
        assert_eq!(instruction.jmp_offset2, 4);
    }
}

#[test]
fn rejects_fixed_csr_reads_that_do_not_fit_signed_immediates() {
    let instruction = RiscvInstruction::CsrRead {
        csr: RiscvCsr::Misa,
        rd: 6,
    };
    let error = lower_guest_report(&report(4, instruction))
        .expect_err("wide fixed CSR read should remain unsupported");

    assert_eq!(
        error,
        ZiskMainLowerError::UnsupportedInstruction { instruction }
    );
}

#[test]
fn rejects_counter_csr_reads() {
    let cases = [
        RiscvCsr::Mcycle,
        RiscvCsr::Minstret,
        RiscvCsr::Mcycleh,
        RiscvCsr::Minstreth,
        RiscvCsr::Cycle,
        RiscvCsr::Time,
        RiscvCsr::Instret,
        RiscvCsr::Cycleh,
        RiscvCsr::Timeh,
        RiscvCsr::Instreth,
    ];

    for csr in cases {
        let instruction = RiscvInstruction::CsrRead { csr, rd: 6 };
        let error = lower_guest_report(&report(4, instruction))
            .expect_err("counter CSR read should remain unsupported");

        assert_eq!(
            error,
            ZiskMainLowerError::UnsupportedInstruction { instruction }
        );
    }
}

#[test]
fn lowers_discarded_csr_reads_as_noop_rows() {
    let cases = [
        RiscvCsr::Misa,
        RiscvCsr::Mvendorid,
        RiscvCsr::Marchid,
        RiscvCsr::Mimpid,
        RiscvCsr::Mhartid,
        RiscvCsr::Mcycle,
        RiscvCsr::Minstret,
        RiscvCsr::Mcycleh,
        RiscvCsr::Minstreth,
        RiscvCsr::Cycle,
        RiscvCsr::Time,
        RiscvCsr::Instret,
        RiscvCsr::Cycleh,
        RiscvCsr::Timeh,
        RiscvCsr::Instreth,
    ];

    for csr in cases {
        let instruction = lower_guest_report(&report(4, RiscvInstruction::CsrRead { csr, rd: 0 }))
            .expect("discarded CSR read should lower");

        assert_eq!(instruction.a, ZiskMainSource::Immediate(0));
        assert_eq!(instruction.b, ZiskMainSource::Immediate(0));
        assert_eq!(instruction.op, ZiskMainOp::Flag);
        assert_eq!(instruction.store, ZiskMainStore::None);
        assert_eq!(instruction.jmp_offset1, 4);
        assert_eq!(instruction.jmp_offset2, 4);
    }
}

#[test]
fn rejects_odd_offset_jalr_for_single_row_lowering() {
    let error = lower_guest_report(&report_with_next_pc(
        4,
        PC + 0x100,
        RiscvInstruction::Jalr {
            rd: 6,
            rs1: 7,
            offset: 3,
        },
    ))
    .expect_err("odd-offset jalr needs multi-row lowering");

    assert_eq!(
        error,
        ZiskMainLowerError::UnsupportedInstruction {
            instruction: RiscvInstruction::Jalr {
                rd: 6,
                rs1: 7,
                offset: 3
            }
        }
    );
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
fn lowers_load_reserved_rows_as_indirect_loads() {
    let cases = [
        (RiscvAmoWidth::Word, ZiskMainOp::SignExtendW, 4),
        (RiscvAmoWidth::Doubleword, ZiskMainOp::CopyB, 8),
    ];

    for (width, op, ind_width) in cases {
        let instruction = lower_guest_report(&report(
            4,
            RiscvInstruction::LoadReserved {
                width,
                rd: 7,
                rs1: 4,
                acquire: true,
                release: true,
            },
        ))
        .expect("load-reserved should lower");

        assert_eq!(instruction.a, ZiskMainSource::Register(4));
        assert_eq!(instruction.b, ZiskMainSource::Indirect(0));
        assert_eq!(instruction.op, op);
        assert_eq!(instruction.store, ZiskMainStore::Register(7));
        assert_eq!(instruction.ind_width, ind_width);
    }
}

#[test]
fn uses_zisk_sign_extension_op_codes() {
    assert_eq!(ZiskMainOp::SignExtendB.code(), 0x27);
    assert_eq!(ZiskMainOp::SignExtendH.code(), 0x28);
    assert_eq!(ZiskMainOp::SignExtendW.code(), 0x29);
}

#[test]
fn uses_zisk_dma_op_codes() {
    assert_eq!(ZiskMainOp::DmaMemCpy.code(), 0xd0);
    assert_eq!(ZiskMainOp::DmaMemCmp.code(), 0xd1);
    assert_eq!(ZiskMainOp::DmaInputCpy.code(), 0xd2);
    assert_eq!(ZiskMainOp::DmaXMemCpy.code(), 0xd6);
    assert_eq!(ZiskMainOp::DmaXMemCmp.code(), 0xd7);
    assert_eq!(ZiskMainOp::DmaXMemSet.code(), 0xd9);
}

#[test]
fn lowers_zisk_dma_prepare_as_internal_copy() {
    let instruction = lower_guest_report(&report(
        4,
        RiscvInstruction::ZiskDmaPrepare {
            kind: RiscvDmaKind::Memcpy,
            rs1: 11,
        },
    ))
    .expect("dma prepare should lower");

    assert_eq!(instruction.a, ZiskMainSource::Immediate(0x0813));
    assert_eq!(instruction.b, ZiskMainSource::Register(11));
    assert_eq!(instruction.op, ZiskMainOp::CopyB);
    assert_eq!(instruction.store, ZiskMainStore::None);
    assert!(!instruction.is_external_op);
}

#[test]
fn lowers_zisk_fcall_param_as_copy_with_word_count() {
    let instruction = lower_guest_report(&report(
        4,
        RiscvInstruction::ZiskFcallParam { port: 2, rs1: 12 },
    ))
    .expect("fcall param should lower");

    assert_eq!(instruction.a, ZiskMainSource::Immediate(4));
    assert_eq!(instruction.b, ZiskMainSource::Register(12));
    assert_eq!(instruction.op, ZiskMainOp::CopyB);
    assert_eq!(instruction.store, ZiskMainStore::None);
    assert!(!instruction.is_external_op);
}

#[test]
fn lowers_zisk_fcall_invoke_as_copy_with_function_id() {
    let instruction = lower_guest_report(&report(
        4,
        RiscvInstruction::ZiskFcallInvoke { function_id: 22 },
    ))
    .expect("fcall invoke should lower");

    assert_eq!(instruction.a, ZiskMainSource::Immediate(22));
    assert_eq!(instruction.b, ZiskMainSource::Immediate(0));
    assert_eq!(instruction.op, ZiskMainOp::CopyB);
    assert_eq!(instruction.store, ZiskMainStore::None);
    assert!(!instruction.is_external_op);
}

#[test]
fn lowers_zisk_fcall_result_as_free_input_read() {
    let instruction = lower_guest_report(&report(4, RiscvInstruction::ZiskFcallResult { rd: 10 }))
        .expect("fcall result should lower");

    assert_eq!(instruction.a, ZiskMainSource::Immediate(0));
    assert_eq!(instruction.b, ZiskMainSource::Memory(ZISK_INPUT_ADDRESS));
    assert_eq!(instruction.op, ZiskMainOp::CopyB);
    assert_eq!(instruction.store, ZiskMainStore::Register(10));
    assert!(!instruction.is_external_op);
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
        precompile_memory_accesses: Vec::new(),
        precompile_result: None,
    }
}

fn register_store(index: u8) -> ZiskMainStore {
    if index == 0 {
        ZiskMainStore::None
    } else {
        ZiskMainStore::Register(index)
    }
}
