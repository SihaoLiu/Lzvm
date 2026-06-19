use std::collections::VecDeque;
use std::fmt;

use smallvec::SmallVec;

use crate::guest_instruction::{
    decode_guest_instruction, GuestInstructionError, RiscvAmoKind, RiscvAmoWidth, RiscvBranchKind,
    RiscvCsr, RiscvDmaKind, RiscvEncodedInstruction, RiscvInstruction, RiscvLoadKind,
    RiscvOp32Kind, RiscvOpImm32Kind, RiscvOpImmKind, RiscvOpKind, RiscvStoreKind,
};
use crate::guest_memory::GuestMemoryError;

mod memory;
mod precompile;
pub use memory::GuestMachineMemory;
use precompile::execute_precompile;

const GUEST_REGISTER_COUNT: usize = 32;
const RV64IMAC_MISA: u64 = (2_u64 << 62) | 1_u64 | (1_u64 << 2) | (1_u64 << 8) | (1_u64 << 12);
pub const ZISK_ARCHITECTURE_ID: u64 = 0x0fff_eeee;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuestFcallParam {
    pub port: u8,
    pub value: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuestFcallRequest {
    pub function_id: u16,
    pub params: Vec<GuestFcallParam>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuestFcallResponse {
    pub results: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuestFcallError {
    Handler { message: String },
}

impl fmt::Display for GuestFcallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Handler { message } => write!(f, "guest free-call handler failed: {message}"),
        }
    }
}

impl std::error::Error for GuestFcallError {}

pub trait GuestFcallHandler {
    fn handle_fcall(
        &mut self,
        request: GuestFcallRequest,
        memory: &mut GuestMachineMemory,
    ) -> Result<GuestFcallResponse, GuestFcallError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuestMachineState {
    pc: u64,
    registers: [u64; GUEST_REGISTER_COUNT],
    reservation: Option<GuestMemoryReservation>,
    pending_dma: Option<GuestDmaPrepare>,
    fcall_params: Vec<GuestFcallParam>,
    fcall_results: VecDeque<u64>,
    retired_instructions: u64,
    dma_proof_value_flags: GuestDmaProofValueFlags,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GuestMemoryReservation {
    address: u64,
    width: RiscvAmoWidth,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GuestDmaPrepare {
    kind: RiscvDmaKind,
    first_arg: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GuestDmaExecute {
    pending: GuestDmaPrepare,
    dst: u64,
    count: u64,
    rd: u8,
    fill_byte: Option<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GuestDmaProofValueFlags {
    pub enable_dma_64_aligned_inputcpy: bool,
    pub enable_dma_64_aligned_mem: bool,
    pub enable_dma_64_aligned_memcpy: bool,
    pub enable_dma_64_aligned_memset: bool,
    pub enable_dma_unaligned: bool,
}

impl GuestMachineState {
    pub fn new(entry_address: u64) -> Self {
        Self {
            pc: entry_address,
            registers: [0; GUEST_REGISTER_COUNT],
            reservation: None,
            pending_dma: None,
            fcall_params: Vec::new(),
            fcall_results: VecDeque::new(),
            retired_instructions: 0,
            dma_proof_value_flags: GuestDmaProofValueFlags::default(),
        }
    }

    pub fn pc(&self) -> u64 {
        self.pc
    }

    pub fn register(&self, index: usize) -> Option<u64> {
        self.registers.get(index).copied()
    }

    pub fn registers(&self) -> &[u64; GUEST_REGISTER_COUNT] {
        &self.registers
    }

    pub fn dma_proof_value_flags(&self) -> GuestDmaProofValueFlags {
        self.dma_proof_value_flags
    }

    pub fn set_register(&mut self, index: usize, value: u64) -> Result<(), GuestMachineError> {
        if index >= GUEST_REGISTER_COUNT {
            return Err(GuestMachineError::InvalidRegisterIndex { index });
        }
        if index != 0 {
            self.registers[index] = value;
        }
        Ok(())
    }

    fn set_pc(&mut self, pc: u64) {
        self.pc = pc;
    }

    fn write_nonzero_decoded_register(&mut self, index: u8, value: u64) {
        debug_assert_ne!(index, 0);
        self.registers[usize::from(index)] = value;
    }

    fn read_decoded_register(&self, index: u8) -> u64 {
        self.registers[usize::from(index)]
    }

    fn set_reservation(&mut self, address: u64, width: RiscvAmoWidth) {
        self.reservation = Some(GuestMemoryReservation { address, width });
    }

    fn clear_reservation(&mut self) {
        self.reservation = None;
    }

    fn clear_reservation_if_overlaps(&mut self, address: u64, byte_len: usize) {
        if self.reservation_overlaps(address, byte_len) {
            self.clear_reservation();
        }
    }

    fn set_pending_dma(&mut self, kind: RiscvDmaKind, first_arg: u64) {
        self.pending_dma = Some(GuestDmaPrepare { kind, first_arg });
    }

    fn take_pending_dma(&mut self) -> Option<GuestDmaPrepare> {
        self.pending_dma.take()
    }

    fn retired_instructions(&self) -> u64 {
        self.retired_instructions
    }

    fn retire_instruction(&mut self) {
        self.retired_instructions = self.retired_instructions.wrapping_add(1);
    }

    fn record_dma_proof_value_flags(&mut self, kind: RiscvDmaKind, dst: u64, src: u64, count: u64) {
        if dma_loop_count(dst, count) == 0 {
            return;
        }
        match kind {
            RiscvDmaKind::Memcpy => {
                if dma_dst_is_aligned_with_src(dst, src) {
                    self.dma_proof_value_flags.enable_dma_64_aligned_memcpy = true;
                } else {
                    self.dma_proof_value_flags.enable_dma_unaligned = true;
                }
            }
            RiscvDmaKind::Memcmp => {
                self.record_dma_memcmp_proof_value_flags(dst, src, count, false);
            }
            RiscvDmaKind::Memset => {
                self.dma_proof_value_flags.enable_dma_64_aligned_memset = true;
            }
            RiscvDmaKind::Inputcpy => {
                self.dma_proof_value_flags.enable_dma_64_aligned_inputcpy = true;
            }
        }
    }

    fn record_dma_memcmp_proof_value_flags(
        &mut self,
        dst: u64,
        src: u64,
        effective_count: u64,
        mismatch: bool,
    ) {
        if dma_memcmp_loop_count(dst, effective_count, mismatch) == 0 {
            return;
        }
        if dma_dst_is_aligned_with_src(dst, src) {
            self.dma_proof_value_flags.enable_dma_64_aligned_mem = true;
        } else {
            self.dma_proof_value_flags.enable_dma_unaligned = true;
        }
    }

    fn push_fcall_param(&mut self, port: u8, value: u64) {
        self.fcall_params.push(GuestFcallParam { port, value });
    }

    fn take_fcall_request(&mut self, function_id: u16) -> GuestFcallRequest {
        let params = std::mem::take(&mut self.fcall_params);
        GuestFcallRequest {
            function_id,
            params,
        }
    }

    fn set_fcall_results(&mut self, results: Vec<u64>) {
        self.fcall_results = results.into();
    }

    fn pop_fcall_result(&mut self) -> Option<u64> {
        self.fcall_results.pop_front()
    }

    fn pop_fcall_result_bytes(
        &mut self,
        address: u64,
        byte_len: usize,
    ) -> Result<Vec<u8>, GuestMachineError> {
        if !byte_len.is_multiple_of(8) {
            return Err(GuestMachineError::InvalidFcallResultByteCount { address, byte_len });
        }
        let word_count = byte_len / 8;
        if self.fcall_results.len() < word_count {
            return Err(GuestMachineError::MissingFcallResult { address });
        }
        let mut bytes = Vec::with_capacity(byte_len);
        for _ in 0..word_count {
            let word = self
                .fcall_results
                .pop_front()
                .expect("fcall result count was checked");
            bytes.extend_from_slice(&word.to_le_bytes());
        }
        Ok(bytes)
    }

    fn reservation_matches(&self, address: u64, width: RiscvAmoWidth) -> bool {
        self.reservation
            .is_some_and(|reservation| reservation.address == address && reservation.width == width)
    }

    fn reservation_overlaps(&self, address: u64, byte_len: usize) -> bool {
        let Some(reservation) = self.reservation else {
            return false;
        };
        let byte_len = byte_len as u64;
        let reservation_len = amo_width_byte_len(reservation.width) as u64;
        let end = address.saturating_add(byte_len);
        let reservation_end = reservation.address.saturating_add(reservation_len);
        address < reservation_end && reservation.address < end
    }
}

struct GuestMachineStateCheckpoint {
    pc: u64,
    reservation: Option<GuestMemoryReservation>,
    pending_dma: Option<GuestDmaPrepare>,
    fcall_params: Option<Vec<GuestFcallParam>>,
    fcall_results: Option<VecDeque<u64>>,
    retired_instructions: u64,
    dma_proof_value_flags: GuestDmaProofValueFlags,
}

impl GuestMachineStateCheckpoint {
    fn new(state: &GuestMachineState, instruction: RiscvInstruction) -> Self {
        let preserves_fcall_params =
            matches!(instruction, RiscvInstruction::ZiskFcallInvoke { .. });
        let preserves_fcall_results =
            matches!(instruction, RiscvInstruction::ZiskFcallResult { .. })
                || matches!(
                    state.pending_dma,
                    Some(GuestDmaPrepare {
                        kind: RiscvDmaKind::Inputcpy,
                        ..
                    })
                );

        Self {
            pc: state.pc,
            reservation: state.reservation,
            pending_dma: state.pending_dma,
            fcall_params: preserves_fcall_params.then(|| state.fcall_params.clone()),
            fcall_results: preserves_fcall_results.then(|| state.fcall_results.clone()),
            retired_instructions: state.retired_instructions,
            dma_proof_value_flags: state.dma_proof_value_flags,
        }
    }

    fn restore(self, state: &mut GuestMachineState) {
        state.pc = self.pc;
        state.reservation = self.reservation;
        state.pending_dma = self.pending_dma;
        if let Some(fcall_params) = self.fcall_params {
            state.fcall_params = fcall_params;
        }
        if let Some(fcall_results) = self.fcall_results {
            state.fcall_results = fcall_results;
        }
        state.retired_instructions = self.retired_instructions;
        state.dma_proof_value_flags = self.dma_proof_value_flags;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuestRegisterWrite {
    pub index: u8,
    pub value: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuestMemoryAccessKind {
    Read,
    Write,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuestMemoryAccess {
    pub kind: GuestMemoryAccessKind,
    pub address: u64,
    pub byte_len: usize,
    pub value: u64,
}

pub type GuestRegisterWriteList = SmallVec<[GuestRegisterWrite; 1]>;
type GuestRegisterRollbackList = SmallVec<[(u8, u64); 1]>;
pub type GuestMemoryAccessList = SmallVec<[GuestMemoryAccess; 1]>;
pub type GuestPrecompileMemoryAccessList = Box<[GuestMemoryAccess]>;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct GuestInstructionEffects {
    register_writes: GuestRegisterWriteList,
    register_rollback: GuestRegisterRollbackList,
    memory_accesses: GuestMemoryAccessList,
    precompile_memory_accesses: Vec<GuestMemoryAccess>,
    precompile_result: Option<u64>,
}

impl GuestInstructionEffects {
    fn record_register_write(&mut self, index: u8, previous_value: u64, value: u64) {
        if index != 0 {
            self.register_rollback.push((index, previous_value));
            self.register_writes
                .push(GuestRegisterWrite { index, value });
        }
    }

    fn restore_registers(&self, state: &mut GuestMachineState) {
        for &(index, value) in self.register_rollback.iter().rev() {
            state.write_nonzero_decoded_register(index, value);
        }
    }

    fn record_precompile_result(&mut self, value: u64) {
        self.precompile_result = Some(value);
    }

    fn record_precompile_memory_read(&mut self, address: u64, byte_len: usize, value: u64) {
        self.precompile_memory_accesses.push(GuestMemoryAccess {
            kind: GuestMemoryAccessKind::Read,
            address,
            byte_len,
            value,
        });
    }

    fn record_precompile_memory_write(&mut self, address: u64, byte_len: usize, value: u64) {
        self.precompile_memory_accesses.push(GuestMemoryAccess {
            kind: GuestMemoryAccessKind::Write,
            address,
            byte_len,
            value,
        });
    }

    fn record_memory_read(&mut self, address: u64, byte_len: usize, value: u64) {
        self.memory_accesses.push(GuestMemoryAccess {
            kind: GuestMemoryAccessKind::Read,
            address,
            byte_len,
            value,
        });
    }

    fn record_memory_write(&mut self, address: u64, byte_len: usize, value: u64) {
        self.memory_accesses.push(GuestMemoryAccess {
            kind: GuestMemoryAccessKind::Write,
            address,
            byte_len,
            value,
        });
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuestMachineReport {
    pub address: u64,
    pub instruction_byte_len: usize,
    pub instruction: RiscvInstruction,
    pub next_pc: u64,
    pub register_writes: GuestRegisterWriteList,
    pub memory_accesses: GuestMemoryAccessList,
    pub precompile_memory_accesses: GuestPrecompileMemoryAccessList,
    pub precompile_result: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GuestMachinePreparedInstruction {
    address: u64,
    byte_len: usize,
    instruction: RiscvInstruction,
}

impl GuestMachinePreparedInstruction {
    pub(crate) fn instruction(self) -> RiscvInstruction {
        self.instruction
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuestMachineRunReport {
    pub executed_instructions: u64,
    pub halt: GuestMachineHalt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuestMachineExecutionTrace {
    pub run: GuestMachineRunReport,
    pub reports: Vec<GuestMachineReport>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum GuestMachineTraceSliceStatus {
    Halted(GuestMachineHalt),
    Paused {
        pc: u64,
        instruction: RiscvInstruction,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuestMachineHalt {
    Ecall { address: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuestMachineError {
    Fetch(GuestInstructionError),
    Memory(GuestMemoryError),
    InvalidRegisterIndex {
        index: usize,
    },
    ProgramCounterOverflow {
        address: u64,
        byte_len: usize,
    },
    MisalignedAtomicAccess {
        address: u64,
        width: RiscvAmoWidth,
    },
    UnsupportedInstructionLength {
        address: u64,
        halfword: u16,
    },
    UnsupportedInstruction {
        address: u64,
        instruction: RiscvInstruction,
    },
    MissingFcallHandler {
        address: u64,
        function_id: u16,
    },
    Fcall {
        address: u64,
        function_id: u16,
        source: GuestFcallError,
    },
    MissingFcallResult {
        address: u64,
    },
    InvalidFcallResultByteCount {
        address: u64,
        byte_len: usize,
    },
    NonInvertibleSecp256k1Scalar {
        address: u64,
    },
    ZeroArith256Modulus {
        address: u64,
    },
    PreparedInstructionPcMismatch {
        expected: u64,
        found: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuestMachineRunError {
    Instruction(GuestMachineError),
    InstructionLimitExceeded { instruction_limit: u64, pc: u64 },
}

impl fmt::Display for GuestMachineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fetch(error) => write!(f, "guest machine instruction fetch failed: {error}"),
            Self::Memory(error) => write!(f, "guest machine memory access failed: {error}"),
            Self::InvalidRegisterIndex { index } => {
                write!(f, "guest machine register index is invalid: {index}")
            }
            Self::ProgramCounterOverflow { address, byte_len } => write!(
                f,
                "guest machine program counter overflows: address {address}, byte length {byte_len}"
            ),
            Self::MisalignedAtomicAccess { address, width } => write!(
                f,
                "guest machine atomic memory access is misaligned: address {address}, width {width:?}"
            ),
            Self::UnsupportedInstructionLength { address, halfword } => write!(
                f,
                "guest machine instruction length is unsupported: address {address}, halfword {halfword:#06x}"
            ),
            Self::UnsupportedInstruction {
                address,
                instruction,
            } => write!(
                f,
                "guest machine instruction is unsupported: address {address}, instruction {instruction:?}"
            ),
            Self::MissingFcallHandler {
                address,
                function_id,
            } => write!(
                f,
                "guest machine free-call handler is missing: address {address}, function id {function_id}"
            ),
            Self::Fcall {
                address,
                function_id,
                source,
            } => write!(
                f,
                "guest machine free-call failed: address {address}, function id {function_id}: {source}"
            ),
            Self::MissingFcallResult { address } => write!(
                f,
                "guest machine free-call result is missing: address {address}"
            ),
            Self::InvalidFcallResultByteCount { address, byte_len } => write!(
                f,
                "guest machine free-call result byte count is invalid: address {address}, byte length {byte_len}"
            ),
            Self::NonInvertibleSecp256k1Scalar { address } => write!(
                f,
                "guest machine secp256k1 scalar is not invertible: address {address}"
            ),
            Self::ZeroArith256Modulus { address } => write!(
                f,
                "guest machine arith256_mod modulus is zero: address {address}"
            ),
            Self::PreparedInstructionPcMismatch { expected, found } => write!(
                f,
                "guest machine prepared instruction pc mismatch: expected {expected}, found {found}"
            ),
        }
    }
}

impl fmt::Display for GuestMachineRunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Instruction(error) => write!(f, "guest machine run instruction failed: {error}"),
            Self::InstructionLimitExceeded {
                instruction_limit,
                pc,
            } => write!(
                f,
                "guest machine run exceeded instruction limit {instruction_limit} at pc {pc}"
            ),
        }
    }
}

impl std::error::Error for GuestMachineError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Fetch(error) => Some(error),
            Self::Memory(error) => Some(error),
            Self::Fcall { source, .. } => Some(source),
            Self::InvalidRegisterIndex { .. }
            | Self::ProgramCounterOverflow { .. }
            | Self::MisalignedAtomicAccess { .. }
            | Self::UnsupportedInstructionLength { .. }
            | Self::UnsupportedInstruction { .. }
            | Self::MissingFcallHandler { .. }
            | Self::MissingFcallResult { .. }
            | Self::InvalidFcallResultByteCount { .. }
            | Self::NonInvertibleSecp256k1Scalar { .. }
            | Self::ZeroArith256Modulus { .. }
            | Self::PreparedInstructionPcMismatch { .. } => None,
        }
    }
}

impl std::error::Error for GuestMachineRunError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Instruction(error) => Some(error),
            Self::InstructionLimitExceeded { .. } => None,
        }
    }
}

impl From<GuestMemoryError> for GuestMachineError {
    fn from(error: GuestMemoryError) -> Self {
        Self::Memory(error)
    }
}

impl From<GuestInstructionError> for GuestMachineError {
    fn from(error: GuestInstructionError) -> Self {
        Self::Fetch(error)
    }
}

impl From<GuestMachineError> for GuestMachineRunError {
    fn from(error: GuestMachineError) -> Self {
        Self::Instruction(error)
    }
}

pub fn run_guest_machine(
    memory: &mut GuestMachineMemory,
    state: &mut GuestMachineState,
    instruction_limit: u64,
) -> Result<GuestMachineRunReport, GuestMachineRunError> {
    run_guest_machine_inner(memory, state, None, instruction_limit, None)
}

pub fn run_guest_machine_with_fcalls(
    memory: &mut GuestMachineMemory,
    state: &mut GuestMachineState,
    handler: &mut dyn GuestFcallHandler,
    instruction_limit: u64,
) -> Result<GuestMachineRunReport, GuestMachineRunError> {
    run_guest_machine_inner(memory, state, Some(handler), instruction_limit, None)
}

pub fn run_guest_machine_trace(
    memory: &mut GuestMachineMemory,
    state: &mut GuestMachineState,
    instruction_limit: u64,
) -> Result<GuestMachineExecutionTrace, GuestMachineRunError> {
    run_guest_machine_trace_inner(memory, state, None, instruction_limit)
}

pub fn run_guest_machine_trace_with_fcalls(
    memory: &mut GuestMachineMemory,
    state: &mut GuestMachineState,
    handler: &mut dyn GuestFcallHandler,
    instruction_limit: u64,
) -> Result<GuestMachineExecutionTrace, GuestMachineRunError> {
    run_guest_machine_trace_inner(memory, state, Some(handler), instruction_limit)
}

fn run_guest_machine_trace_inner(
    memory: &mut GuestMachineMemory,
    state: &mut GuestMachineState,
    handler: Option<&mut dyn GuestFcallHandler>,
    instruction_limit: u64,
) -> Result<GuestMachineExecutionTrace, GuestMachineRunError> {
    let mut reports = Vec::new();
    let run = run_guest_machine_inner(
        memory,
        state,
        handler,
        instruction_limit,
        Some(&mut reports),
    )?;
    Ok(GuestMachineExecutionTrace { run, reports })
}

fn run_guest_machine_inner(
    memory: &mut GuestMachineMemory,
    state: &mut GuestMachineState,
    mut handler: Option<&mut dyn GuestFcallHandler>,
    instruction_limit: u64,
    mut reports: Option<&mut Vec<GuestMachineReport>>,
) -> Result<GuestMachineRunReport, GuestMachineRunError> {
    let mut executed_instructions = 0_u64;
    loop {
        let prepared = prepare_current_guest_instruction(memory, state.pc())?;
        if prepared.instruction == RiscvInstruction::Ecall {
            return Ok(GuestMachineRunReport {
                executed_instructions,
                halt: GuestMachineHalt::Ecall {
                    address: state.pc(),
                },
            });
        }
        if executed_instructions == instruction_limit {
            return Err(GuestMachineRunError::InstructionLimitExceeded {
                instruction_limit,
                pc: state.pc(),
            });
        }
        let report = match handler.as_deref_mut() {
            Some(handler) => {
                advance_guest_machine_prepared_inner(memory, state, Some(handler), prepared)?
            }
            None => advance_guest_machine_prepared_inner(memory, state, None, prepared)?,
        };
        if let Some(reports) = reports.as_deref_mut() {
            reports.push(report);
        }
        executed_instructions += 1;
    }
}

pub fn advance_guest_machine(
    memory: &mut GuestMachineMemory,
    state: &mut GuestMachineState,
) -> Result<GuestMachineReport, GuestMachineError> {
    advance_guest_machine_inner(memory, state, None)
}

pub fn advance_guest_machine_with_fcalls(
    memory: &mut GuestMachineMemory,
    state: &mut GuestMachineState,
    handler: &mut dyn GuestFcallHandler,
) -> Result<GuestMachineReport, GuestMachineError> {
    advance_guest_machine_inner(memory, state, Some(handler))
}

pub(crate) fn prepare_current_guest_instruction(
    memory: &GuestMachineMemory,
    address: u64,
) -> Result<GuestMachinePreparedInstruction, GuestMachineError> {
    let (byte_len, instruction) = fetch_decode_guest_instruction(memory, address)?;
    Ok(GuestMachinePreparedInstruction {
        address,
        byte_len,
        instruction,
    })
}

pub(crate) fn advance_guest_machine_with_prepared_fcalls(
    memory: &mut GuestMachineMemory,
    state: &mut GuestMachineState,
    handler: &mut dyn GuestFcallHandler,
    prepared: GuestMachinePreparedInstruction,
) -> Result<GuestMachineReport, GuestMachineError> {
    advance_guest_machine_prepared_inner(memory, state, Some(handler), prepared)
}

fn advance_guest_machine_inner(
    memory: &mut GuestMachineMemory,
    state: &mut GuestMachineState,
    handler: Option<&mut dyn GuestFcallHandler>,
) -> Result<GuestMachineReport, GuestMachineError> {
    let address = state.pc();
    let prepared = prepare_current_guest_instruction(memory, address)?;
    advance_guest_machine_prepared_inner(memory, state, handler, prepared)
}

fn advance_guest_machine_prepared_inner(
    memory: &mut GuestMachineMemory,
    state: &mut GuestMachineState,
    handler: Option<&mut dyn GuestFcallHandler>,
    prepared: GuestMachinePreparedInstruction,
) -> Result<GuestMachineReport, GuestMachineError> {
    let address = state.pc();
    if prepared.address != address {
        return Err(GuestMachineError::PreparedInstructionPcMismatch {
            expected: address,
            found: prepared.address,
        });
    }
    let byte_len = prepared.byte_len;
    let instruction = prepared.instruction;
    let sequential_pc = address
        .checked_add(byte_len as u64)
        .ok_or(GuestMachineError::ProgramCounterOverflow { address, byte_len })?;
    let mut effects = GuestInstructionEffects::default();
    let checkpoint = GuestMachineStateCheckpoint::new(state, instruction);

    state.set_pc(sequential_pc);
    if let Err(error) = execute_guest_instruction(
        memory,
        address,
        sequential_pc,
        instruction,
        state,
        &mut effects,
        handler,
    ) {
        effects.restore_registers(state);
        checkpoint.restore(state);
        return Err(error);
    }
    state.retire_instruction();
    let next_pc = state.pc();

    Ok(GuestMachineReport {
        address,
        instruction_byte_len: byte_len,
        instruction,
        next_pc,
        register_writes: effects.register_writes,
        memory_accesses: effects.memory_accesses,
        precompile_memory_accesses: effects.precompile_memory_accesses.into_boxed_slice(),
        precompile_result: effects.precompile_result,
    })
}

pub(crate) fn decode_current_guest_instruction(
    memory: &GuestMachineMemory,
    address: u64,
) -> Result<RiscvInstruction, GuestMachineError> {
    Ok(prepare_current_guest_instruction(memory, address)?.instruction)
}

fn fetch_decode_guest_instruction(
    memory: &GuestMachineMemory,
    address: u64,
) -> Result<(usize, RiscvInstruction), GuestMachineError> {
    let fetched = memory.fetch_instruction(address)?;
    let byte_len = match fetched.byte_len() {
        Some(byte_len) => byte_len,
        None => {
            let RiscvEncodedInstruction::UnsupportedLong(halfword) = fetched.encoded else {
                unreachable!("instruction encoding without byte length must be explicit")
            };
            return Err(GuestMachineError::UnsupportedInstructionLength { address, halfword });
        }
    };
    let instruction = decode_guest_instruction(fetched);
    Ok((byte_len, instruction))
}

fn execute_guest_instruction(
    memory: &mut GuestMachineMemory,
    address: u64,
    sequential_pc: u64,
    instruction: RiscvInstruction,
    state: &mut GuestMachineState,
    effects: &mut GuestInstructionEffects,
    handler: Option<&mut dyn GuestFcallHandler>,
) -> Result<(), GuestMachineError> {
    if let Some(pending_dma) = state.take_pending_dma() {
        return execute_pending_dma(memory, address, instruction, state, effects, pending_dma);
    }

    match instruction {
        RiscvInstruction::Lui { rd, immediate } => {
            write_reported_register(state, effects, rd, immediate as u64);
        }
        RiscvInstruction::Auipc { rd, immediate } => {
            write_reported_register(state, effects, rd, address.wrapping_add_signed(immediate));
        }
        RiscvInstruction::Jal { rd, offset } => {
            write_reported_register(state, effects, rd, sequential_pc);
            state.set_pc(address.wrapping_add_signed(offset));
        }
        RiscvInstruction::Jalr { rd, rs1, offset } => {
            let target = state.read_decoded_register(rs1).wrapping_add_signed(offset) & !1;
            write_reported_register(state, effects, rd, sequential_pc);
            state.set_pc(target);
        }
        RiscvInstruction::Branch {
            kind,
            rs1,
            rs2,
            offset,
        } => {
            if branch_is_taken(
                kind,
                state.read_decoded_register(rs1),
                state.read_decoded_register(rs2),
            ) {
                state.set_pc(address.wrapping_add_signed(offset));
            }
        }
        RiscvInstruction::OpImm {
            kind,
            rd,
            rs1,
            immediate,
        } => {
            let value = execute_op_imm(kind, state.read_decoded_register(rs1), immediate).ok_or(
                GuestMachineError::UnsupportedInstruction {
                    address,
                    instruction,
                },
            )?;
            write_reported_register(state, effects, rd, value);
        }
        RiscvInstruction::Op { kind, rd, rs1, rs2 } => {
            let value = execute_op(
                kind,
                state.read_decoded_register(rs1),
                state.read_decoded_register(rs2),
            );
            write_reported_register(state, effects, rd, value);
        }
        RiscvInstruction::OpImm32 {
            kind,
            rd,
            rs1,
            immediate,
        } => {
            let value = execute_op_imm_32(kind, state.read_decoded_register(rs1), immediate);
            write_reported_register(state, effects, rd, value);
        }
        RiscvInstruction::Op32 { kind, rd, rs1, rs2 } => {
            let value = execute_op_32(
                kind,
                state.read_decoded_register(rs1),
                state.read_decoded_register(rs2),
            );
            write_reported_register(state, effects, rd, value);
        }
        RiscvInstruction::Load {
            kind,
            rd,
            rs1,
            offset,
        } => {
            let address = state.read_decoded_register(rs1).wrapping_add_signed(offset);
            let loaded = read_guest_load(memory, kind, address)?;
            effects.record_memory_read(address, loaded.byte_len, loaded.memory_value);
            write_reported_register(state, effects, rd, loaded.register_value);
        }
        RiscvInstruction::Store {
            kind,
            rs1,
            rs2,
            offset,
        } => {
            let address = state.read_decoded_register(rs1).wrapping_add_signed(offset);
            let byte_len = store_byte_len(kind);
            let value = state.read_decoded_register(rs2);
            write_guest_store(memory, kind, address, value)?;
            effects.record_memory_write(address, byte_len, low_bytes_value(value, byte_len));
            state.clear_reservation_if_overlaps(address, store_byte_len(kind));
        }
        RiscvInstruction::Amo {
            kind,
            width,
            rd,
            rs1,
            rs2,
            ..
        } => {
            let address = state.read_decoded_register(rs1);
            let loaded = read_guest_amo(memory, width, address)?;
            let stored = execute_amo(kind, width, loaded, state.read_decoded_register(rs2));
            write_guest_amo(memory, width, address, stored)?;
            let byte_len = amo_width_byte_len(width);
            effects.record_memory_read(address, byte_len, loaded);
            effects.record_memory_write(address, byte_len, low_bytes_value(stored, byte_len));
            state.clear_reservation_if_overlaps(address, amo_width_byte_len(width));
            write_reported_register(state, effects, rd, amo_result(width, loaded));
        }
        RiscvInstruction::LoadReserved { width, rd, rs1, .. } => {
            let address = state.read_decoded_register(rs1);
            let loaded = read_guest_amo(memory, width, address)?;
            effects.record_memory_read(address, amo_width_byte_len(width), loaded);
            write_reported_register(state, effects, rd, amo_result(width, loaded));
            state.set_reservation(address, width);
        }
        RiscvInstruction::StoreConditional {
            width,
            rd,
            rs1,
            rs2,
            ..
        } => {
            let address = state.read_decoded_register(rs1);
            validate_guest_amo_address(memory, width, address)?;
            if state.reservation_matches(address, width) {
                let value = state.read_decoded_register(rs2);
                write_guest_amo(memory, width, address, value)?;
                let byte_len = amo_width_byte_len(width);
                effects.record_memory_write(address, byte_len, low_bytes_value(value, byte_len));
                write_reported_register(state, effects, rd, 0);
            } else {
                write_reported_register(state, effects, rd, 1);
            }
            state.clear_reservation();
        }
        RiscvInstruction::CsrRead { csr, rd } => {
            write_reported_register(
                state,
                effects,
                rd,
                read_csr(csr, state.retired_instructions()),
            );
        }
        RiscvInstruction::ZiskPrecompile { kind, rs1, rd } => {
            let operand_address = state.read_decoded_register(rs1);
            let result =
                execute_precompile(memory, state, effects, kind, address, operand_address)?;
            effects.record_precompile_result(result);
            write_reported_register(state, effects, rd, result);
        }
        RiscvInstruction::ZiskDmaPrepare { kind, rs1 } => {
            state.set_pending_dma(kind, state.read_decoded_register(rs1));
        }
        RiscvInstruction::ZiskFcallParam { port, rs1 } => {
            state.push_fcall_param(port, state.read_decoded_register(rs1));
        }
        RiscvInstruction::ZiskFcallInvoke { function_id } => {
            let request = state.take_fcall_request(function_id);
            let Some(handler) = handler else {
                return Err(GuestMachineError::MissingFcallHandler {
                    address,
                    function_id,
                });
            };
            let response = handler.handle_fcall(request, memory).map_err(|source| {
                GuestMachineError::Fcall {
                    address,
                    function_id,
                    source,
                }
            })?;
            state.set_fcall_results(response.results);
        }
        RiscvInstruction::ZiskFcallResult { rd } => {
            let value = state
                .pop_fcall_result()
                .ok_or(GuestMachineError::MissingFcallResult { address })?;
            write_reported_register(state, effects, rd, value);
        }
        RiscvInstruction::Fence { .. } => {}
        RiscvInstruction::CompressedUnknown { .. }
        | RiscvInstruction::IllegalCompressed { .. }
        | RiscvInstruction::UnsupportedLong { .. }
        | RiscvInstruction::Ecall
        | RiscvInstruction::Ebreak
        | RiscvInstruction::Unknown { .. } => {
            return Err(GuestMachineError::UnsupportedInstruction {
                address,
                instruction,
            });
        }
    }

    Ok(())
}

fn write_reported_register(
    state: &mut GuestMachineState,
    effects: &mut GuestInstructionEffects,
    index: u8,
    value: u64,
) {
    if index == 0 {
        return;
    }
    let previous_value = state.read_decoded_register(index);
    state.write_nonzero_decoded_register(index, value);
    effects.record_register_write(index, previous_value, value);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GuestLoadResult {
    byte_len: usize,
    memory_value: u64,
    register_value: u64,
}

fn read_guest_load(
    memory: &GuestMachineMemory,
    kind: RiscvLoadKind,
    address: u64,
) -> Result<GuestLoadResult, GuestMachineError> {
    let (byte_len, memory_value, register_value) = match kind {
        RiscvLoadKind::Lb => {
            let mut bytes = [0_u8; 1];
            memory.read_range_into(address, &mut bytes)?;
            (1, u64::from(bytes[0]), i64::from(bytes[0] as i8) as u64)
        }
        RiscvLoadKind::Lh => {
            let mut bytes = [0_u8; 2];
            memory.read_range_into(address, &mut bytes)?;
            (
                2,
                u64::from(u16::from_le_bytes(bytes)),
                i64::from(i16::from_le_bytes(bytes)) as u64,
            )
        }
        RiscvLoadKind::Lw => {
            let mut bytes = [0_u8; 4];
            memory.read_range_into(address, &mut bytes)?;
            (
                4,
                u64::from(u32::from_le_bytes(bytes)),
                i64::from(i32::from_le_bytes(bytes)) as u64,
            )
        }
        RiscvLoadKind::Ld => {
            let mut bytes = [0_u8; 8];
            memory.read_range_into(address, &mut bytes)?;
            let value = u64::from_le_bytes(bytes);
            (8, value, value)
        }
        RiscvLoadKind::Lbu => {
            let mut bytes = [0_u8; 1];
            memory.read_range_into(address, &mut bytes)?;
            (1, u64::from(bytes[0]), u64::from(bytes[0]))
        }
        RiscvLoadKind::Lhu => {
            let mut bytes = [0_u8; 2];
            memory.read_range_into(address, &mut bytes)?;
            let value = u64::from(u16::from_le_bytes(bytes));
            (2, value, value)
        }
        RiscvLoadKind::Lwu => {
            let mut bytes = [0_u8; 4];
            memory.read_range_into(address, &mut bytes)?;
            let value = u64::from(u32::from_le_bytes(bytes));
            (4, value, value)
        }
    };
    Ok(GuestLoadResult {
        byte_len,
        memory_value,
        register_value,
    })
}

fn write_guest_store(
    memory: &mut GuestMachineMemory,
    kind: RiscvStoreKind,
    address: u64,
    value: u64,
) -> Result<(), GuestMachineError> {
    let bytes = value.to_le_bytes();
    match kind {
        RiscvStoreKind::Sb => memory.write_range(address, &bytes[..1])?,
        RiscvStoreKind::Sh => memory.write_range(address, &bytes[..2])?,
        RiscvStoreKind::Sw => memory.write_range(address, &bytes[..4])?,
        RiscvStoreKind::Sd => memory.write_range(address, &bytes)?,
    }
    Ok(())
}

fn store_byte_len(kind: RiscvStoreKind) -> usize {
    match kind {
        RiscvStoreKind::Sb => 1,
        RiscvStoreKind::Sh => 2,
        RiscvStoreKind::Sw => 4,
        RiscvStoreKind::Sd => 8,
    }
}

fn low_bytes_value(value: u64, byte_len: usize) -> u64 {
    if byte_len >= 8 {
        value
    } else {
        value & ((1_u64 << (byte_len * 8)) - 1)
    }
}

fn amo_width_byte_len(width: RiscvAmoWidth) -> usize {
    match width {
        RiscvAmoWidth::Word => 4,
        RiscvAmoWidth::Doubleword => 8,
    }
}

fn ensure_atomic_aligned(width: RiscvAmoWidth, address: u64) -> Result<(), GuestMachineError> {
    let byte_len = amo_width_byte_len(width) as u64;
    if address.is_multiple_of(byte_len) {
        Ok(())
    } else {
        Err(GuestMachineError::MisalignedAtomicAccess { address, width })
    }
}

fn validate_guest_amo_address(
    memory: &GuestMachineMemory,
    width: RiscvAmoWidth,
    address: u64,
) -> Result<(), GuestMachineError> {
    ensure_atomic_aligned(width, address)?;
    match width {
        RiscvAmoWidth::Word => {
            let mut bytes = [0_u8; 4];
            memory.read_range_into(address, &mut bytes)?;
        }
        RiscvAmoWidth::Doubleword => {
            let mut bytes = [0_u8; 8];
            memory.read_range_into(address, &mut bytes)?;
        }
    }
    Ok(())
}

fn read_guest_amo(
    memory: &GuestMachineMemory,
    width: RiscvAmoWidth,
    address: u64,
) -> Result<u64, GuestMachineError> {
    ensure_atomic_aligned(width, address)?;
    let value = match width {
        RiscvAmoWidth::Word => {
            let mut bytes = [0_u8; 4];
            memory.read_range_into(address, &mut bytes)?;
            u64::from(u32::from_le_bytes(bytes))
        }
        RiscvAmoWidth::Doubleword => {
            let mut bytes = [0_u8; 8];
            memory.read_range_into(address, &mut bytes)?;
            u64::from_le_bytes(bytes)
        }
    };
    Ok(value)
}

fn write_guest_amo(
    memory: &mut GuestMachineMemory,
    width: RiscvAmoWidth,
    address: u64,
    value: u64,
) -> Result<(), GuestMachineError> {
    ensure_atomic_aligned(width, address)?;
    match width {
        RiscvAmoWidth::Word => memory.write_range(address, &(value as u32).to_le_bytes())?,
        RiscvAmoWidth::Doubleword => memory.write_range(address, &value.to_le_bytes())?,
    }
    Ok(())
}

fn execute_amo(kind: RiscvAmoKind, width: RiscvAmoWidth, loaded: u64, operand: u64) -> u64 {
    match width {
        RiscvAmoWidth::Word => u64::from(execute_amo_word(kind, loaded as u32, operand as u32)),
        RiscvAmoWidth::Doubleword => execute_amo_doubleword(kind, loaded, operand),
    }
}

fn execute_amo_word(kind: RiscvAmoKind, loaded: u32, operand: u32) -> u32 {
    match kind {
        RiscvAmoKind::Add => loaded.wrapping_add(operand),
        RiscvAmoKind::Swap => operand,
        RiscvAmoKind::Xor => loaded ^ operand,
        RiscvAmoKind::Or => loaded | operand,
        RiscvAmoKind::And => loaded & operand,
        RiscvAmoKind::Min => (loaded as i32).min(operand as i32) as u32,
        RiscvAmoKind::Max => (loaded as i32).max(operand as i32) as u32,
        RiscvAmoKind::Minu => loaded.min(operand),
        RiscvAmoKind::Maxu => loaded.max(operand),
    }
}

fn execute_amo_doubleword(kind: RiscvAmoKind, loaded: u64, operand: u64) -> u64 {
    match kind {
        RiscvAmoKind::Add => loaded.wrapping_add(operand),
        RiscvAmoKind::Swap => operand,
        RiscvAmoKind::Xor => loaded ^ operand,
        RiscvAmoKind::Or => loaded | operand,
        RiscvAmoKind::And => loaded & operand,
        RiscvAmoKind::Min => (loaded as i64).min(operand as i64) as u64,
        RiscvAmoKind::Max => (loaded as i64).max(operand as i64) as u64,
        RiscvAmoKind::Minu => loaded.min(operand),
        RiscvAmoKind::Maxu => loaded.max(operand),
    }
}

fn amo_result(width: RiscvAmoWidth, loaded: u64) -> u64 {
    match width {
        RiscvAmoWidth::Word => sign_extend_word(loaded as u32),
        RiscvAmoWidth::Doubleword => loaded,
    }
}

fn execute_pending_dma(
    memory: &mut GuestMachineMemory,
    address: u64,
    instruction: RiscvInstruction,
    state: &mut GuestMachineState,
    effects: &mut GuestInstructionEffects,
    pending: GuestDmaPrepare,
) -> Result<(), GuestMachineError> {
    match instruction {
        RiscvInstruction::Op {
            kind: RiscvOpKind::Add,
            rd,
            rs1,
            rs2,
        } => {
            let dst = state.read_decoded_register(rs1);
            let count = state.read_decoded_register(rs2);
            execute_dma(
                memory,
                state,
                effects,
                GuestDmaExecute {
                    pending,
                    dst,
                    count,
                    rd,
                    fill_byte: None,
                },
            )
        }
        RiscvInstruction::OpImm {
            kind: RiscvOpImmKind::Addi,
            rd,
            rs1,
            immediate,
        } => {
            if pending.kind == RiscvDmaKind::Memset {
                let count = state.read_decoded_register(rs1);
                execute_dma(
                    memory,
                    state,
                    effects,
                    GuestDmaExecute {
                        pending,
                        dst: pending.first_arg,
                        count,
                        rd,
                        fill_byte: Some(immediate as u8),
                    },
                )
            } else {
                let dst = state.read_decoded_register(rs1);
                execute_dma(
                    memory,
                    state,
                    effects,
                    GuestDmaExecute {
                        pending,
                        dst,
                        count: immediate as u64,
                        rd,
                        fill_byte: None,
                    },
                )
            }
        }
        _ => Err(GuestMachineError::UnsupportedInstruction {
            address,
            instruction,
        }),
    }
}

fn execute_dma(
    memory: &mut GuestMachineMemory,
    state: &mut GuestMachineState,
    effects: &mut GuestInstructionEffects,
    request: GuestDmaExecute,
) -> Result<(), GuestMachineError> {
    let GuestDmaExecute {
        pending,
        dst,
        count,
        rd,
        fill_byte,
    } = request;
    let result = match pending.kind {
        RiscvDmaKind::Memcpy => {
            dma_memcpy(memory, state, dst, pending.first_arg, count)?;
            dst
        }
        RiscvDmaKind::Memcmp => {
            let (result, effective_count) = dma_memcmp(memory, dst, pending.first_arg, count)?;
            state.record_dma_memcmp_proof_value_flags(
                dst,
                pending.first_arg,
                effective_count,
                result != 0,
            );
            write_reported_register(state, effects, rd, result);
            return Ok(());
        }
        RiscvDmaKind::Memset => {
            let fill_byte = fill_byte.unwrap_or(0);
            dma_memset(memory, state, dst, count, fill_byte)?;
            dst
        }
        RiscvDmaKind::Inputcpy => {
            dma_inputcpy(memory, state, dst, count)?;
            dst
        }
    };
    state.record_dma_proof_value_flags(pending.kind, dst, pending.first_arg, count);
    write_reported_register(state, effects, rd, result);
    Ok(())
}

fn dma_memcpy(
    memory: &mut GuestMachineMemory,
    state: &mut GuestMachineState,
    dst: u64,
    src: u64,
    count: u64,
) -> Result<(), GuestMachineError> {
    let count = dma_count_to_usize(src, count)?;
    if dst == src || count == 0 {
        return Ok(());
    }
    let mut bytes = vec![0_u8; count];
    memory.read_range_into(src, &mut bytes)?;
    state.clear_reservation_if_overlaps(dst, count);
    memory.write_range(dst, &bytes)?;
    Ok(())
}

fn dma_memcmp(
    memory: &GuestMachineMemory,
    lhs: u64,
    rhs: u64,
    count: u64,
) -> Result<(u64, u64), GuestMachineError> {
    let count = dma_count_to_usize(lhs, count)?;
    if count == 0 {
        return Ok((0, 0));
    }
    let mut lhs_bytes = vec![0_u8; count];
    let mut rhs_bytes = vec![0_u8; count];
    memory.read_range_into(lhs, &mut lhs_bytes)?;
    memory.read_range_into(rhs, &mut rhs_bytes)?;
    for (index, (lhs_byte, rhs_byte)) in lhs_bytes.into_iter().zip(rhs_bytes).enumerate() {
        if lhs_byte != rhs_byte {
            let effective_count = u64::try_from(index + 1).unwrap_or(u64::MAX);
            return Ok((
                ((lhs_byte as i64) - (rhs_byte as i64)) as u64,
                effective_count,
            ));
        }
    }
    Ok((0, u64::try_from(count).unwrap_or(u64::MAX)))
}

fn dma_memset(
    memory: &mut GuestMachineMemory,
    state: &mut GuestMachineState,
    dst: u64,
    count: u64,
    fill_byte: u8,
) -> Result<(), GuestMachineError> {
    let count = dma_count_to_usize(dst, count)?;
    if count == 0 {
        return Ok(());
    }
    let bytes = vec![fill_byte; count];
    state.clear_reservation_if_overlaps(dst, count);
    memory.write_range(dst, &bytes)?;
    Ok(())
}

fn dma_inputcpy(
    memory: &mut GuestMachineMemory,
    state: &mut GuestMachineState,
    dst: u64,
    count: u64,
) -> Result<(), GuestMachineError> {
    let count = dma_count_to_usize(dst, count)?;
    if count == 0 {
        return Ok(());
    }
    let bytes = state.pop_fcall_result_bytes(state.pc(), count)?;
    state.clear_reservation_if_overlaps(dst, count);
    memory.write_range(dst, &bytes)?;
    Ok(())
}

fn dma_count_to_usize(address: u64, count: u64) -> Result<usize, GuestMemoryError> {
    usize::try_from(count).map_err(|_| GuestMemoryError::AddressRangeOverflow {
        address,
        byte_len: usize::MAX,
    })
}

fn dma_loop_count(dst: u64, count: u64) -> u64 {
    let dst_offset = dst & 0x07;
    let pending = if dst_offset == 0 {
        count
    } else {
        let pre_count = 8 - dst_offset;
        count.saturating_sub(pre_count)
    };
    pending >> 3
}

fn dma_memcmp_loop_count(dst: u64, count: u64, mismatch: bool) -> u64 {
    let loop_count = dma_loop_count(dst, count);
    let post_count = dma_post_count(dst, count);
    if mismatch && post_count == 0 {
        loop_count.saturating_sub(1)
    } else {
        loop_count
    }
}

fn dma_post_count(dst: u64, count: u64) -> u64 {
    let dst_offset = dst & 0x07;
    if dst_offset > 0 && count <= 8 - dst_offset {
        return 0;
    }
    let aligned_count = if dst_offset == 0 {
        count
    } else {
        count - (8 - dst_offset)
    };
    aligned_count & 0x07
}

fn dma_dst_is_aligned_with_src(dst: u64, src: u64) -> bool {
    (dst & 0x07) == (src & 0x07)
}

fn branch_is_taken(kind: RiscvBranchKind, lhs: u64, rhs: u64) -> bool {
    match kind {
        RiscvBranchKind::Beq => lhs == rhs,
        RiscvBranchKind::Bne => lhs != rhs,
        RiscvBranchKind::Blt => (lhs as i64) < (rhs as i64),
        RiscvBranchKind::Bge => (lhs as i64) >= (rhs as i64),
        RiscvBranchKind::Bltu => lhs < rhs,
        RiscvBranchKind::Bgeu => lhs >= rhs,
    }
}

pub(crate) fn fixed_csr_value(csr: RiscvCsr) -> Option<u64> {
    match csr {
        RiscvCsr::Misa => Some(RV64IMAC_MISA),
        RiscvCsr::Marchid => Some(ZISK_ARCHITECTURE_ID),
        RiscvCsr::Mvendorid | RiscvCsr::Mimpid | RiscvCsr::Mhartid => Some(0),
        RiscvCsr::Mcycle
        | RiscvCsr::Minstret
        | RiscvCsr::Cycle
        | RiscvCsr::Time
        | RiscvCsr::Instret
        | RiscvCsr::Mcycleh
        | RiscvCsr::Minstreth
        | RiscvCsr::Cycleh
        | RiscvCsr::Timeh
        | RiscvCsr::Instreth => None,
    }
}

fn read_csr(csr: RiscvCsr, retired_instructions: u64) -> u64 {
    match fixed_csr_value(csr) {
        Some(value) => value,
        None => match csr {
            RiscvCsr::Mcycle
            | RiscvCsr::Minstret
            | RiscvCsr::Cycle
            | RiscvCsr::Time
            | RiscvCsr::Instret => retired_instructions,
            RiscvCsr::Mcycleh
            | RiscvCsr::Minstreth
            | RiscvCsr::Cycleh
            | RiscvCsr::Timeh
            | RiscvCsr::Instreth => retired_instructions >> 32,
            RiscvCsr::Misa
            | RiscvCsr::Mvendorid
            | RiscvCsr::Marchid
            | RiscvCsr::Mimpid
            | RiscvCsr::Mhartid => unreachable!("fixed CSRs are handled above"),
        },
    }
}

fn execute_op_imm(kind: RiscvOpImmKind, rs1: u64, immediate: i64) -> Option<u64> {
    let value = match kind {
        RiscvOpImmKind::Addi => rs1.wrapping_add_signed(immediate),
        RiscvOpImmKind::Slti => u64::from((rs1 as i64) < immediate),
        RiscvOpImmKind::Sltiu => u64::from(rs1 < immediate as u64),
        RiscvOpImmKind::Xori => rs1 ^ immediate as u64,
        RiscvOpImmKind::Ori => rs1 | immediate as u64,
        RiscvOpImmKind::Andi => rs1 & immediate as u64,
        RiscvOpImmKind::Slli => rs1.wrapping_shl((immediate as u32) & 0x3f),
        RiscvOpImmKind::Srli => rs1.wrapping_shr((immediate as u32) & 0x3f),
        RiscvOpImmKind::Srai => ((rs1 as i64) >> ((immediate as u32) & 0x3f)) as u64,
    };
    Some(value)
}

fn execute_op_imm_32(kind: RiscvOpImm32Kind, rs1: u64, immediate: i64) -> u64 {
    match kind {
        RiscvOpImm32Kind::Addiw => sign_extend_word(rs1.wrapping_add_signed(immediate) as u32),
        RiscvOpImm32Kind::Slliw => {
            sign_extend_word((rs1 as u32).wrapping_shl((immediate as u32) & 0x1f))
        }
        RiscvOpImm32Kind::Srliw => {
            sign_extend_word((rs1 as u32).wrapping_shr((immediate as u32) & 0x1f))
        }
        RiscvOpImm32Kind::Sraiw => {
            sign_extend_word(((rs1 as u32 as i32) >> ((immediate as u32) & 0x1f)) as u32)
        }
    }
}

fn execute_op(kind: RiscvOpKind, rs1: u64, rs2: u64) -> u64 {
    match kind {
        RiscvOpKind::Add => rs1.wrapping_add(rs2),
        RiscvOpKind::Sub => rs1.wrapping_sub(rs2),
        RiscvOpKind::Sll => rs1.wrapping_shl((rs2 as u32) & 0x3f),
        RiscvOpKind::Slt => u64::from((rs1 as i64) < (rs2 as i64)),
        RiscvOpKind::Sltu => u64::from(rs1 < rs2),
        RiscvOpKind::Xor => rs1 ^ rs2,
        RiscvOpKind::Srl => rs1.wrapping_shr((rs2 as u32) & 0x3f),
        RiscvOpKind::Sra => ((rs1 as i64) >> ((rs2 as u32) & 0x3f)) as u64,
        RiscvOpKind::Or => rs1 | rs2,
        RiscvOpKind::And => rs1 & rs2,
        RiscvOpKind::Mul => rs1.wrapping_mul(rs2),
        RiscvOpKind::Mulh => (((rs1 as i64 as i128) * (rs2 as i64 as i128)) >> 64) as u64,
        RiscvOpKind::Mulhsu => (((rs1 as i64 as i128) * (rs2 as i128)) >> 64) as u64,
        RiscvOpKind::Mulhu => (((rs1 as u128) * (rs2 as u128)) >> 64) as u64,
        RiscvOpKind::Div => signed_divide(rs1 as i64, rs2 as i64) as u64,
        RiscvOpKind::Divu => {
            if rs2 == 0 {
                u64::MAX
            } else {
                rs1 / rs2
            }
        }
        RiscvOpKind::Rem => signed_remainder(rs1 as i64, rs2 as i64) as u64,
        RiscvOpKind::Remu => {
            if rs2 == 0 {
                rs1
            } else {
                rs1 % rs2
            }
        }
    }
}

fn execute_op_32(kind: RiscvOp32Kind, rs1: u64, rs2: u64) -> u64 {
    match kind {
        RiscvOp32Kind::Addw => sign_extend_word((rs1 as u32).wrapping_add(rs2 as u32)),
        RiscvOp32Kind::Subw => sign_extend_word((rs1 as u32).wrapping_sub(rs2 as u32)),
        RiscvOp32Kind::Sllw => sign_extend_word((rs1 as u32).wrapping_shl((rs2 as u32) & 0x1f)),
        RiscvOp32Kind::Srlw => sign_extend_word((rs1 as u32).wrapping_shr((rs2 as u32) & 0x1f)),
        RiscvOp32Kind::Sraw => {
            sign_extend_word(((rs1 as u32 as i32) >> ((rs2 as u32) & 0x1f)) as u32)
        }
        RiscvOp32Kind::Mulw => sign_extend_word((rs1 as u32).wrapping_mul(rs2 as u32)),
        RiscvOp32Kind::Divw => signed_divide_word(rs1 as u32 as i32, rs2 as u32 as i32),
        RiscvOp32Kind::Divuw => {
            if rs2 as u32 == 0 {
                u64::MAX
            } else {
                sign_extend_word((rs1 as u32) / (rs2 as u32))
            }
        }
        RiscvOp32Kind::Remw => signed_remainder_word(rs1 as u32 as i32, rs2 as u32 as i32),
        RiscvOp32Kind::Remuw => {
            if rs2 as u32 == 0 {
                sign_extend_word(rs1 as u32)
            } else {
                sign_extend_word((rs1 as u32) % (rs2 as u32))
            }
        }
    }
}

fn sign_extend_word(value: u32) -> u64 {
    i64::from(value as i32) as u64
}

fn signed_divide(dividend: i64, divisor: i64) -> i64 {
    if divisor == 0 {
        -1
    } else if dividend == i64::MIN && divisor == -1 {
        i64::MIN
    } else {
        dividend / divisor
    }
}

fn signed_remainder(dividend: i64, divisor: i64) -> i64 {
    if divisor == 0 {
        dividend
    } else if dividend == i64::MIN && divisor == -1 {
        0
    } else {
        dividend % divisor
    }
}

fn signed_divide_word(dividend: i32, divisor: i32) -> u64 {
    let quotient = if divisor == 0 {
        -1
    } else if dividend == i32::MIN && divisor == -1 {
        i32::MIN
    } else {
        dividend / divisor
    };
    sign_extend_word(quotient as u32)
}

fn signed_remainder_word(dividend: i32, divisor: i32) -> u64 {
    let remainder = if divisor == 0 {
        dividend
    } else if dividend == i32::MIN && divisor == -1 {
        0
    } else {
        dividend % divisor
    };
    sign_extend_word(remainder as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::guest_memory::load_guest_memory_image;
    use lzvm_artifacts::guest_image::parse_guest_image;

    const TEST_ENTRY: u64 = 0x8000_0000;

    fn sample_guest_image() -> Vec<u8> {
        let mut bytes = vec![0_u8; 64];
        bytes[0..4].copy_from_slice(b"\x7fELF");
        bytes[4] = 2;
        bytes[5] = 1;
        bytes[6] = 1;
        bytes[16..18].copy_from_slice(&2_u16.to_le_bytes());
        bytes[18..20].copy_from_slice(&243_u16.to_le_bytes());
        bytes[20..24].copy_from_slice(&1_u32.to_le_bytes());
        bytes[24..32].copy_from_slice(&TEST_ENTRY.to_le_bytes());
        bytes[32..40].copy_from_slice(&64_u64.to_le_bytes());
        bytes[52..54].copy_from_slice(&64_u16.to_le_bytes());
        bytes
    }

    fn guest_machine_memory_with_words(words: &[u32]) -> GuestMachineMemory {
        let mut code = Vec::with_capacity(words.len() * 4);
        for word in words {
            code.extend_from_slice(&word.to_le_bytes());
        }
        let mut header = [0_u8; 56];
        header[0..4].copy_from_slice(&1_u32.to_le_bytes());
        header[4..8].copy_from_slice(&5_u32.to_le_bytes());
        header[8..16].copy_from_slice(&120_u64.to_le_bytes());
        header[16..24].copy_from_slice(&TEST_ENTRY.to_le_bytes());
        header[24..32].copy_from_slice(&TEST_ENTRY.to_le_bytes());
        header[32..40].copy_from_slice(&(code.len() as u64).to_le_bytes());
        header[40..48].copy_from_slice(&(code.len() as u64).to_le_bytes());
        header[48..56].copy_from_slice(&0x1000_u64.to_le_bytes());

        let mut image = sample_guest_image();
        image[32..40].copy_from_slice(&64_u64.to_le_bytes());
        image[54..56].copy_from_slice(&56_u16.to_le_bytes());
        image[56..58].copy_from_slice(&1_u16.to_le_bytes());
        image.extend_from_slice(&header);
        image.extend_from_slice(&code);
        let info = parse_guest_image(&image).expect("guest image should parse");
        let memory_image =
            load_guest_memory_image(&image, &info).expect("guest memory should load");
        GuestMachineMemory::from_image(&memory_image)
    }

    fn addi(rd: u8, rs1: u8, immediate: i16) -> u32 {
        (((immediate as i32 as u32) & 0x0fff) << 20)
            | (u32::from(rs1) << 15)
            | (u32::from(rd) << 7)
            | 0x13
    }

    #[test]
    fn reads_high_counter_csr_halves() {
        let retired_instructions = (3_u64 << 32) | 17;

        for csr in [
            RiscvCsr::Mcycleh,
            RiscvCsr::Minstreth,
            RiscvCsr::Cycleh,
            RiscvCsr::Timeh,
            RiscvCsr::Instreth,
        ] {
            assert_eq!(read_csr(csr, retired_instructions), 3);
        }
    }

    #[test]
    fn prepared_advance_matches_regular_advance() {
        let mut regular_memory = guest_machine_memory_with_words(&[addi(1, 0, 7), 0x0000_0073]);
        let mut prepared_memory = regular_memory.clone();
        let mut regular_state = GuestMachineState::new(regular_memory.entry_address());
        let mut prepared_state = regular_state.clone();

        let expected = advance_guest_machine(&mut regular_memory, &mut regular_state)
            .expect("regular advance should succeed");
        let prepared = prepare_current_guest_instruction(&prepared_memory, prepared_state.pc())
            .expect("instruction should prepare");
        let actual = advance_guest_machine_prepared_inner(
            &mut prepared_memory,
            &mut prepared_state,
            None,
            prepared,
        )
        .expect("prepared advance should succeed");

        assert_eq!(actual, expected);
        assert_eq!(prepared_state, regular_state);
        assert_eq!(prepared_memory, regular_memory);
    }

    #[test]
    fn register_write_effect_restores_old_value_without_changing_reported_write() {
        let mut state = GuestMachineState::new(TEST_ENTRY);
        state
            .set_register(1, 42)
            .expect("test register should be writable");
        let mut effects = GuestInstructionEffects::default();

        write_reported_register(&mut state, &mut effects, 1, 99);
        assert_eq!(state.register(1), Some(99));
        assert_eq!(
            effects.register_writes.as_slice(),
            &[GuestRegisterWrite {
                index: 1,
                value: 99,
            }]
        );

        effects.restore_registers(&mut state);
        assert_eq!(state.register(1), Some(42));
    }

    #[test]
    fn prepared_advance_rejects_stale_program_counter() {
        let mut memory = guest_machine_memory_with_words(&[addi(1, 0, 7), addi(2, 0, 9)]);
        let mut state = GuestMachineState::new(memory.entry_address());
        let prepared = prepare_current_guest_instruction(&memory, state.pc())
            .expect("instruction should prepare");
        state.set_pc(TEST_ENTRY + 4);

        assert_eq!(
            advance_guest_machine_prepared_inner(&mut memory, &mut state, None, prepared),
            Err(GuestMachineError::PreparedInstructionPcMismatch {
                expected: TEST_ENTRY + 4,
                found: TEST_ENTRY,
            })
        );
        assert_eq!(state.register(1), Some(0));
        assert_eq!(state.pc(), TEST_ENTRY + 4);
    }
}
