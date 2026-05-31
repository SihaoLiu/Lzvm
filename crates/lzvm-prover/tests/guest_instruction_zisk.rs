use lzvm_prover::guest_instruction::{
    decode_riscv_instruction, RiscvDmaKind, RiscvInstruction, RiscvPrecompileKind,
};

fn csrrs(rd: u8, csr: u16, rs1: u8) -> u32 {
    assert!(rd < 32);
    assert!(csr < 4096);
    assert!(rs1 < 32);
    (u32::from(csr) << 20) | (u32::from(rs1) << 15) | (2 << 12) | (u32::from(rd) << 7) | 0x73
}

fn csrs(csr: u16, rs1: u8) -> u32 {
    csrrs(0, csr, rs1)
}

fn csrwi(csr: u16, immediate: u8) -> u32 {
    assert!(csr < 4096);
    assert!(immediate < 32);
    (u32::from(csr) << 20) | (u32::from(immediate) << 15) | (5 << 12) | 0x73
}

#[test]
fn decodes_zisk_free_call_csr_instructions() {
    assert_eq!(
        decode_riscv_instruction(csrs(0x08f0, 5)),
        RiscvInstruction::ZiskFcallParam { port: 0, rs1: 5 }
    );
    assert_eq!(
        decode_riscv_instruction(csrs(0x08ff, 6)),
        RiscvInstruction::ZiskFcallParam { port: 15, rs1: 6 }
    );
    assert_eq!(
        decode_riscv_instruction(csrwi(0x08c0, 7)),
        RiscvInstruction::ZiskFcallInvoke { function_id: 7 }
    );
    assert_eq!(
        decode_riscv_instruction(csrwi(0x08df, 31)),
        RiscvInstruction::ZiskFcallInvoke { function_id: 1023 }
    );
    assert_eq!(
        decode_riscv_instruction(csrrs(10, 0x0ffe, 0)),
        RiscvInstruction::ZiskFcallResult { rd: 10 }
    );
}

#[test]
fn decodes_zisk_dma_and_precompile_csr_instructions() {
    assert_eq!(
        decode_riscv_instruction(csrrs(0, 0x0800, 10)),
        RiscvInstruction::ZiskPrecompile {
            kind: RiscvPrecompileKind::Keccak,
            rs1: 10,
            rd: 0,
        }
    );
    assert_eq!(
        decode_riscv_instruction(csrrs(0, 0x0801, 11)),
        RiscvInstruction::ZiskPrecompile {
            kind: RiscvPrecompileKind::Arith256,
            rs1: 11,
            rd: 0,
        }
    );
    assert_eq!(
        decode_riscv_instruction(csrrs(0, 0x0802, 12)),
        RiscvInstruction::ZiskPrecompile {
            kind: RiscvPrecompileKind::Arith256Mod,
            rs1: 12,
            rd: 0,
        }
    );
    assert_eq!(
        decode_riscv_instruction(csrrs(0, 0x0803, 10)),
        RiscvInstruction::ZiskPrecompile {
            kind: RiscvPrecompileKind::Secp256k1Add,
            rs1: 10,
            rd: 0,
        }
    );
    assert_eq!(
        decode_riscv_instruction(csrrs(0, 0x0804, 15)),
        RiscvInstruction::ZiskPrecompile {
            kind: RiscvPrecompileKind::Secp256k1Dbl,
            rs1: 15,
            rd: 0,
        }
    );
    assert_eq!(
        decode_riscv_instruction(csrrs(5, 0x0811, 13)),
        RiscvInstruction::ZiskPrecompile {
            kind: RiscvPrecompileKind::Add256,
            rs1: 13,
            rd: 5,
        }
    );
    assert_eq!(
        decode_riscv_instruction(csrrs(0, 0x0813, 14)),
        RiscvInstruction::ZiskDmaPrepare {
            kind: RiscvDmaKind::Memcpy,
            rs1: 14,
        }
    );
    assert_eq!(
        decode_riscv_instruction(csrrs(0, 0x0814, 15)),
        RiscvInstruction::ZiskDmaPrepare {
            kind: RiscvDmaKind::Memcmp,
            rs1: 15,
        }
    );
    assert_eq!(
        decode_riscv_instruction(csrrs(0, 0x0816, 16)),
        RiscvInstruction::ZiskDmaPrepare {
            kind: RiscvDmaKind::Memset,
            rs1: 16,
        }
    );
}
