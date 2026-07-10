use std::collections::VecDeque;
use std::fmt;
use std::mem::MaybeUninit;
use std::time::{Duration, Instant};

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
pub(crate) use memory::GuestMachineMemoryOverlaySnapshot;
use precompile::execute_precompile;

const GUEST_REGISTER_COUNT: usize = 32;
const GUEST_INSTRUCTION_CACHE_ENTRY_COUNT: usize = 1 << 18;
const GUEST_INSTRUCTION_CACHE_ENTRY_BITS_ENV: &str = "LZVM_GUEST_INSTRUCTION_CACHE_ENTRY_BITS";
const GUEST_INSTRUCTION_CACHE_MIN_ENTRY_BITS: u32 = 10;
const GUEST_INSTRUCTION_CACHE_MAX_ENTRY_BITS: u32 = 22;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuestFcallMemoryEffect {
    Unknown,
    ReadOnly,
    WriteRange { address: u64, byte_len: usize },
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
    /// Declares memory writes from a successful call. Implementations must return `Unknown`
    /// unless every possible write is covered by the returned effect.
    fn memory_effect(&self, _function_id: u16) -> GuestFcallMemoryEffect {
        GuestFcallMemoryEffect::Unknown
    }

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
    pub byte_len: u8,
    pub value: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuestInlineEffectList<T> {
    entries: GuestInlineEffectEntries<T>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum GuestInlineEffectEntries<T> {
    Empty,
    One(T),
    Many(Box<[T]>),
}

impl<T> Default for GuestInlineEffectList<T> {
    fn default() -> Self {
        Self {
            entries: GuestInlineEffectEntries::Empty,
        }
    }
}

impl<T> GuestInlineEffectList<T> {
    pub fn push(&mut self, value: T) {
        self.entries = match std::mem::replace(&mut self.entries, GuestInlineEffectEntries::Empty) {
            GuestInlineEffectEntries::Empty => GuestInlineEffectEntries::One(value),
            GuestInlineEffectEntries::One(first) => {
                let mut entries = Vec::with_capacity(2);
                entries.push(first);
                entries.push(value);
                GuestInlineEffectEntries::Many(entries.into_boxed_slice())
            }
            GuestInlineEffectEntries::Many(entries) => {
                let mut entries = entries.into_vec();
                entries.push(value);
                GuestInlineEffectEntries::Many(entries.into_boxed_slice())
            }
        };
    }

    pub fn as_slice(&self) -> &[T] {
        match &self.entries {
            GuestInlineEffectEntries::Empty => &[],
            GuestInlineEffectEntries::One(value) => std::slice::from_ref(value),
            GuestInlineEffectEntries::Many(entries) => entries,
        }
    }

    pub fn len(&self) -> usize {
        self.as_slice().len()
    }

    pub fn is_empty(&self) -> bool {
        self.as_slice().is_empty()
    }
}

impl<T> From<Vec<T>> for GuestInlineEffectList<T> {
    fn from(entries: Vec<T>) -> Self {
        match entries.len() {
            0 => Self::default(),
            1 => {
                let mut entries = entries.into_iter();
                Self {
                    entries: GuestInlineEffectEntries::One(
                        entries.next().expect("single entry should be present"),
                    ),
                }
            }
            _ => Self {
                entries: GuestInlineEffectEntries::Many(entries.into_boxed_slice()),
            },
        }
    }
}

impl<T> std::ops::Deref for GuestInlineEffectList<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<'a, T> IntoIterator for &'a GuestInlineEffectList<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.as_slice().iter()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuestRegisterWriteList {
    entry: GuestRegisterWrite,
}

impl Default for GuestRegisterWriteList {
    fn default() -> Self {
        Self {
            entry: GuestRegisterWrite { index: 0, value: 0 },
        }
    }
}

impl GuestRegisterWriteList {
    fn one(value: GuestRegisterWrite) -> Self {
        debug_assert_ne!(value.index, 0);
        Self { entry: value }
    }

    fn push(&mut self, value: GuestRegisterWrite) {
        debug_assert_ne!(value.index, 0);
        if self.entry.index != 0 {
            panic!("guest instruction reported more than one register write");
        }
        self.entry = value;
    }

    pub fn as_slice(&self) -> &[GuestRegisterWrite] {
        if self.entry.index == 0 {
            &[]
        } else {
            std::slice::from_ref(&self.entry)
        }
    }

    pub fn single(&self) -> Option<GuestRegisterWrite> {
        (self.entry.index != 0).then_some(self.entry)
    }

    pub fn len(&self) -> usize {
        usize::from(self.entry.index != 0)
    }

    pub fn is_empty(&self) -> bool {
        self.entry.index == 0
    }
}

impl From<Vec<GuestRegisterWrite>> for GuestRegisterWriteList {
    fn from(entries: Vec<GuestRegisterWrite>) -> Self {
        match entries.as_slice() {
            [] => Self::default(),
            [entry] => Self::one(*entry),
            _ => panic!("guest register write list supports at most one entry"),
        }
    }
}

impl std::ops::Deref for GuestRegisterWriteList {
    type Target = [GuestRegisterWrite];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<'a> IntoIterator for &'a GuestRegisterWriteList {
    type Item = &'a GuestRegisterWrite;
    type IntoIter = std::slice::Iter<'a, GuestRegisterWrite>;

    fn into_iter(self) -> Self::IntoIter {
        self.as_slice().iter()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct GuestRegisterWriteValue {
    value: u64,
}

impl GuestRegisterWriteValue {
    pub(crate) fn new(value: u64) -> Self {
        Self { value }
    }

    fn from_register_writes(register_writes: GuestRegisterWriteList) -> Self {
        Self::new(register_writes.entry.value)
    }

    fn register_writes(self, instruction: RiscvInstruction) -> GuestRegisterWriteList {
        guest_instruction_register_write_index(instruction).map_or_else(
            GuestRegisterWriteList::default,
            |index| {
                GuestRegisterWriteList::one(GuestRegisterWrite {
                    index,
                    value: self.value,
                })
            },
        )
    }

    #[inline(always)]
    fn register_writes_with_index(self, index: Option<u8>) -> GuestRegisterWriteList {
        match index {
            Some(index) if index != 0 => GuestRegisterWriteList::one(GuestRegisterWrite {
                index,
                value: self.value,
            }),
            _ => GuestRegisterWriteList::default(),
        }
    }
}

type GuestRegisterRollbackList = SmallVec<[(u8, u64); 1]>;
pub type GuestPrecompileMemoryAccessList = Box<[GuestMemoryAccess]>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuestMemoryAccessList {
    entries: GuestMemoryAccessEntries,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum GuestMemoryAccessEntries {
    Empty,
    One(GuestMemoryAccess),
    Many(Box<[GuestMemoryAccess]>),
    Precompile(Box<GuestPrecompileReportEffects>),
}

impl Default for GuestMemoryAccessList {
    fn default() -> Self {
        Self {
            entries: GuestMemoryAccessEntries::Empty,
        }
    }
}

impl GuestMemoryAccessList {
    fn one(value: GuestMemoryAccess) -> Self {
        Self {
            entries: GuestMemoryAccessEntries::One(value),
        }
    }

    pub fn push(&mut self, value: GuestMemoryAccess) {
        self.entries = match std::mem::replace(&mut self.entries, GuestMemoryAccessEntries::Empty) {
            GuestMemoryAccessEntries::Empty => GuestMemoryAccessEntries::One(value),
            GuestMemoryAccessEntries::One(first) => {
                let mut entries = Vec::with_capacity(2);
                entries.push(first);
                entries.push(value);
                GuestMemoryAccessEntries::Many(entries.into_boxed_slice())
            }
            GuestMemoryAccessEntries::Many(entries) => {
                let mut entries = entries.into_vec();
                entries.push(value);
                GuestMemoryAccessEntries::Many(entries.into_boxed_slice())
            }
            GuestMemoryAccessEntries::Precompile(mut effects) => {
                let mut entries = std::mem::take(&mut effects.normal_memory_accesses).into_vec();
                entries.push(value);
                effects.normal_memory_accesses = entries.into_boxed_slice();
                GuestMemoryAccessEntries::Precompile(effects)
            }
        };
    }

    pub fn as_slice(&self) -> &[GuestMemoryAccess] {
        match &self.entries {
            GuestMemoryAccessEntries::Empty => &[],
            GuestMemoryAccessEntries::One(value) => std::slice::from_ref(value),
            GuestMemoryAccessEntries::Many(entries) => entries,
            GuestMemoryAccessEntries::Precompile(effects) => {
                effects.normal_memory_accesses.as_ref()
            }
        }
    }

    pub fn len(&self) -> usize {
        self.as_slice().len()
    }

    pub fn is_empty(&self) -> bool {
        self.as_slice().is_empty()
    }

    fn with_precompile_effects(
        memory_accesses: Self,
        precompile_effects: Option<Box<GuestPrecompileReportEffects>>,
    ) -> Self {
        match precompile_effects {
            Some(effects) => Self {
                entries: GuestMemoryAccessEntries::Precompile(
                    effects.with_normal_memory_accesses(memory_accesses),
                ),
            },
            None => memory_accesses,
        }
    }

    fn into_boxed_slice(self) -> Box<[GuestMemoryAccess]> {
        match self.entries {
            GuestMemoryAccessEntries::Empty => Vec::new().into_boxed_slice(),
            GuestMemoryAccessEntries::One(value) => vec![value].into_boxed_slice(),
            GuestMemoryAccessEntries::Many(entries) => entries,
            GuestMemoryAccessEntries::Precompile(effects) => effects.into_normal_memory_accesses(),
        }
    }

    #[inline(always)]
    fn precompile_memory_accesses(&self) -> &[GuestMemoryAccess] {
        match &self.entries {
            GuestMemoryAccessEntries::Precompile(effects) => effects.memory_accesses.as_ref(),
            _ => &[],
        }
    }

    fn precompile_result(&self) -> Option<u64> {
        match &self.entries {
            GuestMemoryAccessEntries::Precompile(effects) => effects.result,
            _ => None,
        }
    }

    #[cfg(test)]
    fn precompile_effects_mut(&mut self) -> Option<&mut GuestPrecompileReportEffects> {
        match &mut self.entries {
            GuestMemoryAccessEntries::Precompile(effects) => Some(effects),
            _ => None,
        }
    }

    #[cfg(test)]
    fn replace_precompile_effects(
        &mut self,
        precompile_effects: Option<Box<GuestPrecompileReportEffects>>,
    ) {
        let memory_accesses = std::mem::take(self);
        *self = Self::with_precompile_effects(memory_accesses, precompile_effects);
    }
}

impl From<Vec<GuestMemoryAccess>> for GuestMemoryAccessList {
    fn from(entries: Vec<GuestMemoryAccess>) -> Self {
        match entries.len() {
            0 => Self::default(),
            1 => {
                let mut entries = entries.into_iter();
                Self {
                    entries: GuestMemoryAccessEntries::One(
                        entries.next().expect("single entry should be present"),
                    ),
                }
            }
            _ => Self {
                entries: GuestMemoryAccessEntries::Many(entries.into_boxed_slice()),
            },
        }
    }
}

impl std::ops::Deref for GuestMemoryAccessList {
    type Target = [GuestMemoryAccess];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<'a> IntoIterator for &'a GuestMemoryAccessList {
    type Item = &'a GuestMemoryAccess;
    type IntoIter = std::slice::Iter<'a, GuestMemoryAccess>;

    fn into_iter(self) -> Self::IntoIter {
        self.as_slice().iter()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuestPrecompileReportEffects {
    pub memory_accesses: GuestPrecompileMemoryAccessList,
    pub result: Option<u64>,
    normal_memory_accesses: GuestPrecompileMemoryAccessList,
}

impl GuestPrecompileReportEffects {
    pub fn from_parts(
        memory_accesses: GuestPrecompileMemoryAccessList,
        result: Option<u64>,
    ) -> Option<Box<Self>> {
        if memory_accesses.is_empty() && result.is_none() {
            None
        } else {
            Some(Box::new(Self {
                memory_accesses,
                result,
                normal_memory_accesses: Vec::new().into_boxed_slice(),
            }))
        }
    }

    pub fn from_vec(
        memory_accesses: Vec<GuestMemoryAccess>,
        result: Option<u64>,
    ) -> Option<Box<Self>> {
        if memory_accesses.is_empty() && result.is_none() {
            None
        } else {
            Some(Box::new(Self {
                memory_accesses: memory_accesses.into_boxed_slice(),
                result,
                normal_memory_accesses: Vec::new().into_boxed_slice(),
            }))
        }
    }

    fn with_normal_memory_accesses(
        mut self: Box<Self>,
        memory_accesses: GuestMemoryAccessList,
    ) -> Box<Self> {
        self.normal_memory_accesses = memory_accesses.into_boxed_slice();
        self
    }

    fn into_normal_memory_accesses(self: Box<Self>) -> GuestPrecompileMemoryAccessList {
        let GuestPrecompileReportEffects {
            normal_memory_accesses,
            ..
        } = *self;
        normal_memory_accesses
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct GuestInstructionEffects {
    register_writes: GuestRegisterWriteList,
    register_rollback: GuestRegisterRollbackList,
    memory_accesses: GuestMemoryAccessList,
    has_memory_write: bool,
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
            byte_len: guest_memory_access_byte_len(byte_len),
            value,
        });
    }

    fn record_precompile_memory_write(&mut self, address: u64, byte_len: usize, value: u64) {
        self.has_memory_write = true;
        self.precompile_memory_accesses.push(GuestMemoryAccess {
            kind: GuestMemoryAccessKind::Write,
            address,
            byte_len: guest_memory_access_byte_len(byte_len),
            value,
        });
    }

    fn record_memory_read(&mut self, address: u64, byte_len: usize, value: u64) {
        self.memory_accesses.push(GuestMemoryAccess {
            kind: GuestMemoryAccessKind::Read,
            address,
            byte_len: guest_memory_access_byte_len(byte_len),
            value,
        });
    }

    fn record_memory_write(&mut self, address: u64, byte_len: usize, value: u64) {
        self.has_memory_write = true;
        self.memory_accesses.push(GuestMemoryAccess {
            kind: GuestMemoryAccessKind::Write,
            address,
            byte_len: guest_memory_access_byte_len(byte_len),
            value,
        });
    }

    fn report_shape(&self, instruction: RiscvInstruction) -> GuestMachineReportShape {
        GuestMachineReportShape {
            instruction,
            has_memory_write: self.has_memory_write,
        }
    }
}

fn guest_memory_access_byte_len(byte_len: usize) -> u8 {
    u8::try_from(byte_len).expect("guest memory access byte length should fit in u8")
}

fn guest_instruction_byte_len(byte_len: usize) -> u8 {
    u8::try_from(byte_len).expect("guest instruction byte length should fit in u8")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuestMachineReport {
    pub(crate) address_and_instruction_len: u64,
    pub instruction: RiscvInstruction,
    pub next_pc: u64,
    pub(crate) register_write_value: GuestRegisterWriteValue,
    pub memory_accesses: GuestMemoryAccessList,
}

impl GuestMachineReport {
    #[inline(always)]
    pub fn new(
        address: u64,
        instruction_byte_len: u8,
        instruction: RiscvInstruction,
        next_pc: u64,
        register_writes: GuestRegisterWriteList,
        memory_accesses: GuestMemoryAccessList,
        precompile_effects: Option<Box<GuestPrecompileReportEffects>>,
    ) -> Self {
        Self {
            address_and_instruction_len: pack_report_address_and_instruction_len(
                address,
                instruction_byte_len,
            ),
            instruction,
            next_pc,
            register_write_value: GuestRegisterWriteValue::from_register_writes(register_writes),
            memory_accesses: GuestMemoryAccessList::with_precompile_effects(
                memory_accesses,
                precompile_effects,
            ),
        }
    }

    #[inline(always)]
    fn new_single_effect(
        address: u64,
        instruction_byte_len: u8,
        instruction: RiscvInstruction,
        next_pc: u64,
        register_write: Option<GuestRegisterWrite>,
        memory_access: Option<GuestMemoryAccess>,
    ) -> Self {
        Self {
            address_and_instruction_len: pack_report_address_and_instruction_len(
                address,
                instruction_byte_len,
            ),
            instruction,
            next_pc,
            register_write_value: register_write
                .map(|write| GuestRegisterWriteValue::new(write.value))
                .unwrap_or_default(),
            memory_accesses: memory_access
                .map(GuestMemoryAccessList::one)
                .unwrap_or_default(),
        }
    }

    #[inline(always)]
    pub fn address(&self) -> u64 {
        self.address_and_instruction_len & !1
    }

    #[inline(always)]
    pub fn instruction_byte_len(&self) -> u8 {
        2 + ((self.address_and_instruction_len & 1) as u8 * 2)
    }

    #[inline(always)]
    pub fn precompile_memory_accesses(&self) -> &[GuestMemoryAccess] {
        self.memory_accesses.precompile_memory_accesses()
    }

    pub fn precompile_result(&self) -> Option<u64> {
        self.memory_accesses.precompile_result()
    }

    pub fn register_writes(&self) -> GuestRegisterWriteList {
        self.register_write_value.register_writes(self.instruction)
    }

    #[inline(always)]
    pub(crate) fn register_writes_with_index(&self, index: Option<u8>) -> GuestRegisterWriteList {
        self.register_write_value.register_writes_with_index(index)
    }

    #[cfg(test)]
    pub(crate) fn precompile_effects_mut(&mut self) -> Option<&mut GuestPrecompileReportEffects> {
        self.memory_accesses.precompile_effects_mut()
    }

    #[cfg(test)]
    pub(crate) fn replace_precompile_effects(
        &mut self,
        precompile_effects: Option<Box<GuestPrecompileReportEffects>>,
    ) {
        self.memory_accesses
            .replace_precompile_effects(precompile_effects);
    }
}

#[inline(always)]
pub(crate) fn pack_report_address_and_instruction_len(
    address: u64,
    instruction_byte_len: u8,
) -> u64 {
    debug_assert_eq!(address & 1, 0);
    debug_assert!(matches!(instruction_byte_len, 2 | 4));
    address | u64::from(instruction_byte_len == 4)
}

fn guest_instruction_register_write_index(instruction: RiscvInstruction) -> Option<u8> {
    let index = match instruction {
        RiscvInstruction::Lui { rd, .. }
        | RiscvInstruction::Auipc { rd, .. }
        | RiscvInstruction::Jal { rd, .. }
        | RiscvInstruction::Jalr { rd, .. }
        | RiscvInstruction::Load { rd, .. }
        | RiscvInstruction::OpImm { rd, .. }
        | RiscvInstruction::OpImm32 { rd, .. }
        | RiscvInstruction::Op { rd, .. }
        | RiscvInstruction::Op32 { rd, .. }
        | RiscvInstruction::Amo { rd, .. }
        | RiscvInstruction::LoadReserved { rd, .. }
        | RiscvInstruction::StoreConditional { rd, .. }
        | RiscvInstruction::CsrRead { rd, .. }
        | RiscvInstruction::ZiskPrecompile { rd, .. }
        | RiscvInstruction::ZiskFcallResult { rd } => rd,
        _ => 0,
    };
    (index != 0).then_some(index)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GuestMachineReportShape {
    pub(crate) instruction: RiscvInstruction,
    pub(crate) has_memory_write: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GuestMachineAdvanceReport {
    pub(crate) report: GuestMachineReport,
    pub(crate) shape: GuestMachineReportShape,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct GuestMachineAdvanceTiming {
    pub(crate) setup_duration: Duration,
    pub(crate) execute_duration: Duration,
    pub(crate) report_duration: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GuestMachineAdvancePath {
    Fast,
    Generic,
}

impl Default for GuestMachineAdvancePath {
    fn default() -> Self {
        Self::Generic
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GuestMachinePreparedInstruction {
    address: u64,
    byte_len: u8,
    instruction: RiscvInstruction,
}

impl GuestMachinePreparedInstruction {
    pub(crate) fn instruction(self) -> RiscvInstruction {
        self.instruction
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct GuestInstructionCacheStats {
    pub(crate) hit_count: usize,
    pub(crate) miss_count: usize,
    pub(crate) clear_count: usize,
    pub(crate) fcall_clear_count: usize,
    pub(crate) dma_clear_count: usize,
    pub(crate) write_invalidation_range_count: usize,
    pub(crate) write_invalidation_skipped_range_count: usize,
    pub(crate) write_invalidation_probe_count: usize,
    pub(crate) invalidated_entry_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GuestInstructionCacheClearReason {
    FcallInvoke,
    DmaPrepare,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GuestInstructionCacheUpdate {
    None,
    Clear(GuestInstructionCacheClearReason),
    InvalidateWriteRange { address: u64, byte_len: usize },
}

impl GuestInstructionCacheStats {
    pub(crate) fn record_clear_reason(&mut self, reason: GuestInstructionCacheClearReason) {
        match reason {
            GuestInstructionCacheClearReason::FcallInvoke => {
                self.fcall_clear_count += 1;
            }
            GuestInstructionCacheClearReason::DmaPrepare => {
                self.dma_clear_count += 1;
            }
        }
    }
}

#[derive(Debug)]
pub(crate) struct GuestInstructionCache {
    entries: Box<[GuestInstructionCacheEntry]>,
    entry_mask: usize,
    generation: u32,
    min_cached_address: u64,
    max_cached_address: u64,
}

#[derive(Debug, Clone, Copy)]
struct GuestInstructionCacheEntry {
    generation: u32,
    prepared: GuestMachinePreparedInstruction,
}

impl Default for GuestInstructionCacheEntry {
    fn default() -> Self {
        Self {
            generation: 0,
            prepared: GuestMachinePreparedInstruction {
                address: 0,
                byte_len: 2,
                instruction: RiscvInstruction::Ecall,
            },
        }
    }
}

impl Default for GuestInstructionCache {
    fn default() -> Self {
        let entry_count = guest_instruction_cache_entry_count();
        Self {
            entries: vec![GuestInstructionCacheEntry::default(); entry_count].into_boxed_slice(),
            entry_mask: entry_count - 1,
            generation: 1,
            min_cached_address: u64::MAX,
            max_cached_address: 0,
        }
    }
}

impl GuestInstructionCache {
    #[inline(always)]
    pub(crate) fn prepare(
        &mut self,
        memory: &GuestMachineMemory,
        address: u64,
    ) -> Result<GuestMachinePreparedInstruction, GuestMachineError> {
        let index = self.index(address);
        let entry = &self.entries[index];
        if entry.generation == self.generation && entry.prepared.address == address {
            return Ok(entry.prepared);
        }
        let prepared = prepare_current_guest_instruction(memory, address)?;
        self.entries[index] = GuestInstructionCacheEntry {
            generation: self.generation,
            prepared,
        };
        self.record_cached_address(address);
        Ok(prepared)
    }

    #[inline(always)]
    pub(crate) fn prepare_with_stats(
        &mut self,
        memory: &GuestMachineMemory,
        address: u64,
        stats: &mut GuestInstructionCacheStats,
    ) -> Result<GuestMachinePreparedInstruction, GuestMachineError> {
        let index = self.index(address);
        let entry = &self.entries[index];
        if entry.generation == self.generation && entry.prepared.address == address {
            stats.hit_count += 1;
            return Ok(entry.prepared);
        }
        stats.miss_count += 1;
        let prepared = prepare_current_guest_instruction(memory, address)?;
        self.entries[index] = GuestInstructionCacheEntry {
            generation: self.generation,
            prepared,
        };
        self.record_cached_address(address);
        Ok(prepared)
    }

    #[inline]
    pub(crate) fn clear(&mut self) {
        match self.generation.checked_add(1) {
            Some(next) => {
                self.generation = next;
                self.clear_cached_address_range();
            }
            None => {
                self.generation = 1;
                self.entries
                    .iter_mut()
                    .for_each(|entry| entry.generation = 0);
                self.clear_cached_address_range();
            }
        }
    }

    #[inline]
    pub(crate) fn clear_with_stats(&mut self, stats: &mut GuestInstructionCacheStats) {
        stats.clear_count += 1;
        self.clear();
    }

    pub(crate) fn invalidate_report(&mut self, report: &GuestMachineReport) {
        for access in report
            .memory_accesses
            .iter()
            .chain(report.precompile_memory_accesses())
        {
            if access.kind == GuestMemoryAccessKind::Write {
                self.invalidate_range(access.address, usize::from(access.byte_len));
            }
        }
    }

    pub(crate) fn invalidate_report_shape(
        &mut self,
        report: &GuestMachineReport,
        shape: GuestMachineReportShape,
    ) {
        if let Some((address, byte_len)) = common_instruction_write_range(report, shape) {
            self.invalidate_range(address, byte_len);
        } else if shape.has_memory_write {
            self.invalidate_report(report);
        }
    }

    pub(crate) fn invalidate_report_shape_with_stats(
        &mut self,
        report: &GuestMachineReport,
        shape: GuestMachineReportShape,
        stats: &mut GuestInstructionCacheStats,
    ) {
        if let Some((address, byte_len)) = common_instruction_write_range(report, shape) {
            self.invalidate_range_with_stats(address, byte_len, stats);
        } else if shape.has_memory_write {
            self.invalidate_report_with_stats(report, stats);
        }
    }

    #[inline]
    pub(crate) fn invalidate_write_range(&mut self, address: u64, byte_len: usize) {
        self.invalidate_range(address, byte_len);
    }

    #[inline]
    pub(crate) fn invalidate_write_range_with_stats(
        &mut self,
        address: u64,
        byte_len: usize,
        stats: &mut GuestInstructionCacheStats,
    ) {
        self.invalidate_range_with_stats(address, byte_len, stats);
    }

    fn invalidate_report_with_stats(
        &mut self,
        report: &GuestMachineReport,
        stats: &mut GuestInstructionCacheStats,
    ) {
        for access in report
            .memory_accesses
            .iter()
            .chain(report.precompile_memory_accesses())
        {
            if access.kind == GuestMemoryAccessKind::Write {
                self.invalidate_range_with_stats(
                    access.address,
                    usize::from(access.byte_len),
                    stats,
                );
            }
        }
    }

    #[inline]
    fn invalidate_range(&mut self, address: u64, byte_len: usize) {
        if byte_len == 0 {
            return;
        }
        let Some(end) = address.checked_add(byte_len as u64) else {
            self.clear();
            return;
        };
        let start = address.saturating_sub(2) & !1;
        if !self.cached_range_can_overlap(start, end) {
            return;
        }
        let mut pc = start.max(self.min_cached_address & !1);
        let end = end.min(self.max_cached_address.saturating_add(2));
        while pc < end {
            self.invalidate_address(pc);
            let Some(next) = pc.checked_add(2) else {
                break;
            };
            pc = next;
        }
    }

    #[inline]
    fn invalidate_range_with_stats(
        &mut self,
        address: u64,
        byte_len: usize,
        stats: &mut GuestInstructionCacheStats,
    ) {
        if byte_len == 0 {
            return;
        }
        stats.write_invalidation_range_count += 1;
        let Some(end) = address.checked_add(byte_len as u64) else {
            self.clear_with_stats(stats);
            return;
        };
        let start = address.saturating_sub(2) & !1;
        if !self.cached_range_can_overlap(start, end) {
            stats.write_invalidation_skipped_range_count += 1;
            return;
        }
        let mut pc = start.max(self.min_cached_address & !1);
        let end = end.min(self.max_cached_address.saturating_add(2));
        while pc < end {
            stats.write_invalidation_probe_count += 1;
            if self.invalidate_address(pc) {
                stats.invalidated_entry_count += 1;
            }
            let Some(next) = pc.checked_add(2) else {
                break;
            };
            pc = next;
        }
    }

    fn invalidate_address(&mut self, address: u64) -> bool {
        let index = self.index(address);
        if self.entries[index].generation == self.generation
            && self.entries[index].prepared.address == address
        {
            self.entries[index].generation = 0;
            return true;
        }
        false
    }

    #[inline]
    fn record_cached_address(&mut self, address: u64) {
        self.min_cached_address = self.min_cached_address.min(address);
        self.max_cached_address = self.max_cached_address.max(address);
    }

    #[inline]
    fn clear_cached_address_range(&mut self) {
        self.min_cached_address = u64::MAX;
        self.max_cached_address = 0;
    }

    #[inline]
    fn cached_range_can_overlap(&self, start: u64, end: u64) -> bool {
        self.min_cached_address <= self.max_cached_address
            && start <= self.max_cached_address
            && end > self.min_cached_address
    }

    #[inline(always)]
    fn index(&self, address: u64) -> usize {
        ((address >> 1) as usize) & self.entry_mask
    }
}

#[inline(always)]
fn common_instruction_write_range(
    report: &GuestMachineReport,
    shape: GuestMachineReportShape,
) -> Option<(u64, usize)> {
    if !shape.has_memory_write {
        return None;
    }
    let access = match &report.memory_accesses.entries {
        GuestMemoryAccessEntries::One(access) => access,
        GuestMemoryAccessEntries::Many(accesses)
            if matches!(shape.instruction, RiscvInstruction::Amo { .. }) =>
        {
            accesses.last()?
        }
        GuestMemoryAccessEntries::Empty
        | GuestMemoryAccessEntries::Many(_)
        | GuestMemoryAccessEntries::Precompile(_) => return None,
    };
    debug_assert_eq!(access.kind, GuestMemoryAccessKind::Write);
    (access.kind == GuestMemoryAccessKind::Write)
        .then_some((access.address, usize::from(access.byte_len)))
}

impl GuestInstructionCacheUpdate {
    #[inline]
    pub(crate) fn apply_or_invalidate_report(
        self,
        instruction_cache: &mut GuestInstructionCache,
        report: &GuestMachineReport,
    ) {
        match self {
            Self::None => instruction_cache.invalidate_report(report),
            Self::Clear(_) => instruction_cache.clear(),
            Self::InvalidateWriteRange { address, byte_len } => {
                instruction_cache.invalidate_write_range(address, byte_len);
            }
        }
    }

    #[inline]
    pub(crate) fn apply_or_invalidate_report_shape(
        self,
        instruction_cache: &mut GuestInstructionCache,
        report: &GuestMachineReport,
        shape: GuestMachineReportShape,
    ) {
        match self {
            Self::None => instruction_cache.invalidate_report_shape(report, shape),
            Self::Clear(_) => instruction_cache.clear(),
            Self::InvalidateWriteRange { address, byte_len } => {
                instruction_cache.invalidate_write_range(address, byte_len);
            }
        }
    }

    #[inline]
    pub(crate) fn apply_or_invalidate_report_shape_with_stats(
        self,
        instruction_cache: &mut GuestInstructionCache,
        report: &GuestMachineReport,
        shape: GuestMachineReportShape,
        stats: &mut GuestInstructionCacheStats,
    ) {
        match self {
            Self::None => {
                instruction_cache.invalidate_report_shape_with_stats(report, shape, stats)
            }
            Self::Clear(reason) => {
                stats.record_clear_reason(reason);
                instruction_cache.clear_with_stats(stats);
            }
            Self::InvalidateWriteRange { address, byte_len } => {
                instruction_cache.invalidate_write_range_with_stats(address, byte_len, stats);
            }
        }
    }
}

fn guest_instruction_cache_entry_count() -> usize {
    let Some(bits) = std::env::var(GUEST_INSTRUCTION_CACHE_ENTRY_BITS_ENV)
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .filter(|bits| {
            (GUEST_INSTRUCTION_CACHE_MIN_ENTRY_BITS..=GUEST_INSTRUCTION_CACHE_MAX_ENTRY_BITS)
                .contains(bits)
        })
    else {
        return GUEST_INSTRUCTION_CACHE_ENTRY_COUNT;
    };
    1_usize << bits
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
    let mut instruction_cache = GuestInstructionCache::default();
    loop {
        let prepared = instruction_cache.prepare(memory, state.pc())?;
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
        let instruction_cache_update = match handler.as_deref() {
            Some(handler) => instruction_cache_update_for_instruction_with_handler(
                state,
                prepared.instruction,
                handler,
            ),
            None => instruction_cache_update_for_instruction(state, prepared.instruction),
        };
        let report = match handler.as_deref_mut() {
            Some(handler) => {
                advance_guest_machine_prepared_inner(memory, state, Some(handler), prepared)?
            }
            None => advance_guest_machine_prepared_inner(memory, state, None, prepared)?,
        };
        instruction_cache_update.apply_or_invalidate_report(&mut instruction_cache, &report);
        if let Some(reports) = reports.as_deref_mut() {
            reports.push(report);
        }
        executed_instructions += 1;
    }
}

#[inline]
pub(crate) fn instruction_cache_update_for_instruction(
    state: &GuestMachineState,
    instruction: RiscvInstruction,
) -> GuestInstructionCacheUpdate {
    if matches!(instruction, RiscvInstruction::ZiskFcallInvoke { .. }) {
        return GuestInstructionCacheUpdate::Clear(GuestInstructionCacheClearReason::FcallInvoke);
    }
    instruction_cache_update_for_non_fcall_instruction(state, instruction)
}

#[inline]
pub(crate) fn instruction_cache_update_for_instruction_with_handler(
    state: &GuestMachineState,
    instruction: RiscvInstruction,
    handler: &dyn GuestFcallHandler,
) -> GuestInstructionCacheUpdate {
    if let RiscvInstruction::ZiskFcallInvoke { function_id } = instruction {
        return match handler.memory_effect(function_id) {
            GuestFcallMemoryEffect::Unknown => {
                GuestInstructionCacheUpdate::Clear(GuestInstructionCacheClearReason::FcallInvoke)
            }
            GuestFcallMemoryEffect::ReadOnly => GuestInstructionCacheUpdate::None,
            GuestFcallMemoryEffect::WriteRange { address, byte_len } => {
                GuestInstructionCacheUpdate::InvalidateWriteRange { address, byte_len }
            }
        };
    }
    instruction_cache_update_for_non_fcall_instruction(state, instruction)
}

#[inline]
fn instruction_cache_update_for_non_fcall_instruction(
    state: &GuestMachineState,
    instruction: RiscvInstruction,
) -> GuestInstructionCacheUpdate {
    let Some(pending) = state.pending_dma else {
        return GuestInstructionCacheUpdate::None;
    };
    if !matches!(
        pending.kind,
        RiscvDmaKind::Memcpy | RiscvDmaKind::Memset | RiscvDmaKind::Inputcpy
    ) {
        return GuestInstructionCacheUpdate::None;
    }
    let Some((address, byte_len)) = pending_dma_write_range(state, pending, instruction) else {
        return GuestInstructionCacheUpdate::Clear(GuestInstructionCacheClearReason::DmaPrepare);
    };
    let Ok(byte_len) = usize::try_from(byte_len) else {
        return GuestInstructionCacheUpdate::Clear(GuestInstructionCacheClearReason::DmaPrepare);
    };
    if byte_len == 0 || (pending.kind == RiscvDmaKind::Memcpy && address == pending.first_arg) {
        return GuestInstructionCacheUpdate::None;
    }
    GuestInstructionCacheUpdate::InvalidateWriteRange { address, byte_len }
}

#[inline]
fn pending_dma_write_range(
    state: &GuestMachineState,
    pending: GuestDmaPrepare,
    instruction: RiscvInstruction,
) -> Option<(u64, u64)> {
    match instruction {
        RiscvInstruction::Op {
            kind: RiscvOpKind::Add,
            rs1,
            rs2,
            ..
        } => Some((
            state.read_decoded_register(rs1),
            state.read_decoded_register(rs2),
        )),
        RiscvInstruction::OpImm {
            kind: RiscvOpImmKind::Addi,
            rs1,
            immediate,
            ..
        } => {
            if pending.kind == RiscvDmaKind::Memset {
                Some((pending.first_arg, state.read_decoded_register(rs1)))
            } else {
                Some((state.read_decoded_register(rs1), immediate as u64))
            }
        }
        _ => None,
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
        byte_len: guest_instruction_byte_len(byte_len),
        instruction,
    })
}

pub(crate) fn advance_guest_machine_with_prepared_fcalls_report_shape(
    memory: &mut GuestMachineMemory,
    state: &mut GuestMachineState,
    handler: &mut dyn GuestFcallHandler,
    prepared: GuestMachinePreparedInstruction,
) -> Result<GuestMachineAdvanceReport, GuestMachineError> {
    advance_guest_machine_prepared_inner_with_report_shape(
        memory,
        state,
        Some(handler),
        prepared,
        None,
        None,
    )
}

pub(crate) fn advance_guest_machine_with_prepared_fcalls_report_shape_timed(
    memory: &mut GuestMachineMemory,
    state: &mut GuestMachineState,
    handler: &mut dyn GuestFcallHandler,
    prepared: GuestMachinePreparedInstruction,
    timing: &mut GuestMachineAdvanceTiming,
) -> Result<GuestMachineAdvanceReport, GuestMachineError> {
    advance_guest_machine_prepared_inner_with_report_shape(
        memory,
        state,
        Some(handler),
        prepared,
        Some(timing),
        None,
    )
}

pub(crate) fn advance_guest_machine_with_prepared_fcalls_report_shape_path(
    memory: &mut GuestMachineMemory,
    state: &mut GuestMachineState,
    handler: &mut dyn GuestFcallHandler,
    prepared: GuestMachinePreparedInstruction,
) -> Result<(GuestMachineAdvanceReport, GuestMachineAdvancePath), GuestMachineError> {
    let mut path = GuestMachineAdvancePath::default();
    let advanced = advance_guest_machine_prepared_inner_with_report_shape(
        memory,
        state,
        Some(handler),
        prepared,
        None,
        Some(&mut path),
    )?;
    Ok((advanced, path))
}

pub(crate) fn advance_guest_machine_with_prepared_fcalls_report_shape_path_timed(
    memory: &mut GuestMachineMemory,
    state: &mut GuestMachineState,
    handler: &mut dyn GuestFcallHandler,
    prepared: GuestMachinePreparedInstruction,
    timing: &mut GuestMachineAdvanceTiming,
) -> Result<(GuestMachineAdvanceReport, GuestMachineAdvancePath), GuestMachineError> {
    let mut path = GuestMachineAdvancePath::default();
    let advanced = advance_guest_machine_prepared_inner_with_report_shape(
        memory,
        state,
        Some(handler),
        prepared,
        Some(timing),
        Some(&mut path),
    )?;
    Ok((advanced, path))
}

pub(crate) fn advance_guest_machine_with_prepared_fcalls_report_shape_at_pc_into(
    memory: &mut GuestMachineMemory,
    state: &mut GuestMachineState,
    handler: &mut dyn GuestFcallHandler,
    pc: u64,
    prepared: GuestMachinePreparedInstruction,
    report: &mut MaybeUninit<GuestMachineReport>,
) -> Result<GuestMachineReportShape, GuestMachineError> {
    advance_guest_machine_prepared_inner_report_shape_at_pc_into(
        memory,
        state,
        Some(handler),
        pc,
        prepared,
        report,
        None,
        None,
    )
}

pub(crate) fn advance_guest_machine_with_prepared_fcalls_report_shape_at_pc_into_timed(
    memory: &mut GuestMachineMemory,
    state: &mut GuestMachineState,
    handler: &mut dyn GuestFcallHandler,
    pc: u64,
    prepared: GuestMachinePreparedInstruction,
    report: &mut MaybeUninit<GuestMachineReport>,
    timing: &mut GuestMachineAdvanceTiming,
) -> Result<GuestMachineReportShape, GuestMachineError> {
    advance_guest_machine_prepared_inner_report_shape_at_pc_into(
        memory,
        state,
        Some(handler),
        pc,
        prepared,
        report,
        Some(timing),
        None,
    )
}

pub(crate) fn advance_guest_machine_with_prepared_fcalls_report_shape_path_at_pc_into(
    memory: &mut GuestMachineMemory,
    state: &mut GuestMachineState,
    handler: &mut dyn GuestFcallHandler,
    pc: u64,
    prepared: GuestMachinePreparedInstruction,
    report: &mut MaybeUninit<GuestMachineReport>,
) -> Result<(GuestMachineReportShape, GuestMachineAdvancePath), GuestMachineError> {
    let mut path = GuestMachineAdvancePath::default();
    let shape = advance_guest_machine_prepared_inner_report_shape_at_pc_into(
        memory,
        state,
        Some(handler),
        pc,
        prepared,
        report,
        None,
        Some(&mut path),
    )?;
    Ok((shape, path))
}

pub(crate) fn advance_guest_machine_with_prepared_fcalls_report_shape_path_at_pc_into_timed(
    memory: &mut GuestMachineMemory,
    state: &mut GuestMachineState,
    handler: &mut dyn GuestFcallHandler,
    pc: u64,
    prepared: GuestMachinePreparedInstruction,
    report: &mut MaybeUninit<GuestMachineReport>,
    timing: &mut GuestMachineAdvanceTiming,
) -> Result<(GuestMachineReportShape, GuestMachineAdvancePath), GuestMachineError> {
    let mut path = GuestMachineAdvancePath::default();
    let shape = advance_guest_machine_prepared_inner_report_shape_at_pc_into(
        memory,
        state,
        Some(handler),
        pc,
        prepared,
        report,
        Some(timing),
        Some(&mut path),
    )?;
    Ok((shape, path))
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
    Ok(advance_guest_machine_prepared_inner_with_report_shape(
        memory, state, handler, prepared, None, None,
    )?
    .report)
}

fn advance_guest_machine_prepared_inner_with_report_shape(
    memory: &mut GuestMachineMemory,
    state: &mut GuestMachineState,
    handler: Option<&mut dyn GuestFcallHandler>,
    prepared: GuestMachinePreparedInstruction,
    timing: Option<&mut GuestMachineAdvanceTiming>,
    path: Option<&mut GuestMachineAdvancePath>,
) -> Result<GuestMachineAdvanceReport, GuestMachineError> {
    let mut report = MaybeUninit::uninit();
    let shape = advance_guest_machine_prepared_inner_report_shape_into(
        memory,
        state,
        handler,
        prepared,
        &mut report,
        timing,
        path,
    )?;
    Ok(GuestMachineAdvanceReport {
        // SAFETY: advance_guest_machine_prepared_inner_report_shape_into only returns Ok after
        // writing a complete GuestMachineReport into the output slot.
        report: unsafe { report.assume_init() },
        shape,
    })
}

fn advance_guest_machine_prepared_inner_report_shape_into(
    memory: &mut GuestMachineMemory,
    state: &mut GuestMachineState,
    handler: Option<&mut dyn GuestFcallHandler>,
    prepared: GuestMachinePreparedInstruction,
    report: &mut MaybeUninit<GuestMachineReport>,
    timing: Option<&mut GuestMachineAdvanceTiming>,
    path: Option<&mut GuestMachineAdvancePath>,
) -> Result<GuestMachineReportShape, GuestMachineError> {
    let pc = state.pc();
    advance_guest_machine_prepared_inner_report_shape_at_pc_into(
        memory, state, handler, pc, prepared, report, timing, path,
    )
}

fn advance_guest_machine_prepared_inner_report_shape_at_pc_into(
    memory: &mut GuestMachineMemory,
    state: &mut GuestMachineState,
    handler: Option<&mut dyn GuestFcallHandler>,
    pc: u64,
    prepared: GuestMachinePreparedInstruction,
    report: &mut MaybeUninit<GuestMachineReport>,
    mut timing: Option<&mut GuestMachineAdvanceTiming>,
    mut path: Option<&mut GuestMachineAdvancePath>,
) -> Result<GuestMachineReportShape, GuestMachineError> {
    if prepared.address != pc {
        return Err(GuestMachineError::PreparedInstructionPcMismatch {
            expected: pc,
            found: prepared.address,
        });
    }
    let address = pc;
    let byte_len = prepared.byte_len;
    let instruction = prepared.instruction;
    let sequential_pc = address.checked_add(u64::from(byte_len)).ok_or(
        GuestMachineError::ProgramCounterOverflow {
            address,
            byte_len: usize::from(byte_len),
        },
    )?;
    let fast_path_started = advance_timing_started(&timing);
    match try_advance_guest_machine_report_fast_path(
        memory,
        state,
        address,
        byte_len,
        sequential_pc,
        instruction,
        report,
    ) {
        Ok(Some(shape)) => {
            if let Some(path) = path.as_deref_mut() {
                *path = GuestMachineAdvancePath::Fast;
            }
            record_advance_timing(fast_path_started, timing.as_deref_mut(), |timing| {
                &mut timing.execute_duration
            });
            return Ok(shape);
        }
        Ok(None) => {
            if let Some(path) = path.as_deref_mut() {
                *path = GuestMachineAdvancePath::Generic;
            }
            record_advance_timing(fast_path_started, timing.as_deref_mut(), |timing| {
                &mut timing.setup_duration
            });
        }
        Err(error) => {
            record_advance_timing(fast_path_started, timing.as_deref_mut(), |timing| {
                &mut timing.execute_duration
            });
            return Err(error);
        }
    }
    let advanced = advance_guest_machine_prepared_inner_report_shape_at_pc_generic(
        memory,
        state,
        handler,
        address,
        byte_len,
        sequential_pc,
        instruction,
        timing,
    )?;
    report.write(advanced.report);
    Ok(advanced.shape)
}

#[allow(clippy::too_many_arguments)]
fn advance_guest_machine_prepared_inner_report_shape_at_pc_generic(
    memory: &mut GuestMachineMemory,
    state: &mut GuestMachineState,
    handler: Option<&mut dyn GuestFcallHandler>,
    address: u64,
    byte_len: u8,
    sequential_pc: u64,
    instruction: RiscvInstruction,
    mut timing: Option<&mut GuestMachineAdvanceTiming>,
) -> Result<GuestMachineAdvanceReport, GuestMachineError> {
    let setup_started = advance_timing_started(&timing);
    let mut effects = GuestInstructionEffects::default();
    let checkpoint = GuestMachineStateCheckpoint::new(state, instruction);

    state.set_pc(sequential_pc);
    record_advance_timing(setup_started, timing.as_deref_mut(), |timing| {
        &mut timing.setup_duration
    });
    let execute_started = advance_timing_started(&timing);
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
        record_advance_timing(execute_started, timing.as_deref_mut(), |timing| {
            &mut timing.execute_duration
        });
        return Err(error);
    }
    state.retire_instruction();
    let next_pc = state.pc();
    record_advance_timing(execute_started, timing.as_deref_mut(), |timing| {
        &mut timing.execute_duration
    });
    let report_started = advance_timing_started(&timing);
    let shape = effects.report_shape(instruction);

    let report = GuestMachineReport::new(
        address,
        byte_len,
        instruction,
        next_pc,
        effects.register_writes,
        effects.memory_accesses,
        GuestPrecompileReportEffects::from_vec(
            effects.precompile_memory_accesses,
            effects.precompile_result,
        ),
    );
    record_advance_timing(report_started, timing.as_deref_mut(), |timing| {
        &mut timing.report_duration
    });
    Ok(GuestMachineAdvanceReport { report, shape })
}

fn try_advance_guest_machine_report_fast_path(
    memory: &mut GuestMachineMemory,
    state: &mut GuestMachineState,
    address: u64,
    byte_len: u8,
    sequential_pc: u64,
    instruction: RiscvInstruction,
    report: &mut MaybeUninit<GuestMachineReport>,
) -> Result<Option<GuestMachineReportShape>, GuestMachineError> {
    if let Some(pending_dma) = state.pending_dma {
        let checkpoint = GuestMachineStateCheckpoint::new(state, instruction);
        state.pending_dma = None;
        state.set_pc(sequential_pc);
        let register_write =
            match execute_fast_pending_dma(memory, address, instruction, state, pending_dma) {
                Ok(register_write) => register_write,
                Err(error) => {
                    checkpoint.restore(state);
                    return Err(error);
                }
            };
        state.retire_instruction();
        report.write(GuestMachineReport::new_single_effect(
            address,
            byte_len,
            instruction,
            sequential_pc,
            register_write,
            None,
        ));
        return Ok(Some(GuestMachineReportShape {
            instruction,
            has_memory_write: false,
        }));
    }

    let mut next_pc = sequential_pc;
    let mut register_write = None;
    let mut memory_access = None;
    let mut has_memory_write = false;
    match instruction {
        RiscvInstruction::Lui { rd, immediate } => {
            register_write = write_fast_reported_register(state, rd, immediate as u64);
        }
        RiscvInstruction::Auipc { rd, immediate } => {
            register_write = write_fast_reported_register(
                state,
                rd,
                address.wrapping_add_signed(i64::from(immediate)),
            );
        }
        RiscvInstruction::Jal { rd, offset } => {
            if rd != 0 {
                register_write = write_fast_reported_register(state, rd, sequential_pc);
            }
            next_pc = address.wrapping_add_signed(i64::from(offset));
        }
        RiscvInstruction::Jalr { rd, rs1, offset } => {
            let target = state
                .read_decoded_register(rs1)
                .wrapping_add_signed(i64::from(offset))
                & !1;
            if rd != 0 {
                register_write = write_fast_reported_register(state, rd, sequential_pc);
            }
            next_pc = target;
        }
        RiscvInstruction::Branch {
            kind,
            rs1,
            rs2,
            offset,
        } => {
            let taken = if rs1 == rs2 {
                same_register_branch_is_taken(kind)
            } else {
                branch_is_taken(
                    kind,
                    state.read_decoded_register(rs1),
                    state.read_decoded_register(rs2),
                )
            };
            if taken {
                next_pc = address.wrapping_add_signed(i64::from(offset));
            }
        }
        RiscvInstruction::OpImm {
            kind: RiscvOpImmKind::Addi,
            rd,
            rs1,
            immediate,
        } => {
            if rd != 0 {
                let value = state
                    .read_decoded_register(rs1)
                    .wrapping_add_signed(i64::from(immediate));
                register_write = write_fast_reported_register(state, rd, value);
            }
        }
        RiscvInstruction::OpImm {
            kind,
            rd,
            rs1,
            immediate,
        } => {
            if rd != 0 {
                let value = execute_op_imm(kind, state.read_decoded_register(rs1), immediate);
                register_write = write_fast_reported_register(state, rd, value);
            }
        }
        RiscvInstruction::Op { kind, rd, rs1, rs2 } => {
            if rd != 0 {
                let lhs = state.read_decoded_register(rs1);
                let rhs = if rs1 == rs2 {
                    lhs
                } else {
                    state.read_decoded_register(rs2)
                };
                let value = execute_op(kind, lhs, rhs);
                register_write = write_fast_reported_register(state, rd, value);
            }
        }
        RiscvInstruction::OpImm32 {
            kind: RiscvOpImm32Kind::Addiw,
            rd,
            rs1,
            immediate,
        } => {
            if rd != 0 {
                let value = sign_extend_word(
                    state
                        .read_decoded_register(rs1)
                        .wrapping_add_signed(i64::from(immediate)) as u32,
                );
                register_write = write_fast_reported_register(state, rd, value);
            }
        }
        RiscvInstruction::OpImm32 {
            kind,
            rd,
            rs1,
            immediate,
        } => {
            if rd != 0 {
                let value = execute_op_imm_32(kind, state.read_decoded_register(rs1), immediate);
                register_write = write_fast_reported_register(state, rd, value);
            }
        }
        RiscvInstruction::Op32 { kind, rd, rs1, rs2 } => {
            if rd != 0 {
                let lhs = state.read_decoded_register(rs1);
                let rhs = if rs1 == rs2 {
                    lhs
                } else {
                    state.read_decoded_register(rs2)
                };
                let value = execute_op_32(kind, lhs, rhs);
                register_write = write_fast_reported_register(state, rd, value);
            }
        }
        RiscvInstruction::Load {
            kind,
            rd,
            rs1,
            offset,
        } => {
            let read_address = state
                .read_decoded_register(rs1)
                .wrapping_add_signed(i64::from(offset));
            let loaded = read_guest_load(memory, kind, read_address)?;
            memory_access = Some(GuestMemoryAccess {
                kind: GuestMemoryAccessKind::Read,
                address: read_address,
                byte_len: guest_memory_access_byte_len(loaded.byte_len),
                value: loaded.memory_value,
            });
            register_write = write_fast_reported_register(state, rd, loaded.register_value);
        }
        RiscvInstruction::Store {
            kind,
            rs1,
            rs2,
            offset,
        } => {
            let write_address = state
                .read_decoded_register(rs1)
                .wrapping_add_signed(i64::from(offset));
            let value = state.read_decoded_register(rs2);
            let byte_len = write_guest_store(memory, kind, write_address, value)?;
            let stored_value = low_bytes_value(value, byte_len);
            has_memory_write = true;
            memory_access = Some(GuestMemoryAccess {
                kind: GuestMemoryAccessKind::Write,
                address: write_address,
                byte_len: guest_memory_access_byte_len(byte_len),
                value: stored_value,
            });
            state.clear_reservation_if_overlaps(write_address, byte_len);
        }
        RiscvInstruction::Amo {
            kind,
            width,
            rd,
            rs1,
            rs2,
            ..
        } => {
            let write_address = state.read_decoded_register(rs1);
            let loaded = read_guest_amo(memory, width, write_address)?;
            let stored = execute_amo(kind, width, loaded, state.read_decoded_register(rs2));
            write_guest_amo(memory, width, write_address, stored)?;
            let access_byte_len = amo_width_byte_len(width);
            let read_access = GuestMemoryAccess {
                kind: GuestMemoryAccessKind::Read,
                address: write_address,
                byte_len: guest_memory_access_byte_len(access_byte_len),
                value: loaded,
            };
            let write_access = GuestMemoryAccess {
                kind: GuestMemoryAccessKind::Write,
                address: write_address,
                byte_len: guest_memory_access_byte_len(access_byte_len),
                value: low_bytes_value(stored, access_byte_len),
            };
            let register_write = write_fast_reported_register(state, rd, amo_result(width, loaded));
            state.clear_reservation_if_overlaps(write_address, access_byte_len);
            state.set_pc(next_pc);
            state.retire_instruction();
            report.write(GuestMachineReport::new(
                address,
                byte_len,
                instruction,
                next_pc,
                register_write
                    .map(GuestRegisterWriteList::one)
                    .unwrap_or_default(),
                GuestMemoryAccessList::from(vec![read_access, write_access]),
                None,
            ));
            return Ok(Some(GuestMachineReportShape {
                instruction,
                has_memory_write: true,
            }));
        }
        RiscvInstruction::LoadReserved { width, rd, rs1, .. } => {
            let read_address = state.read_decoded_register(rs1);
            let loaded = read_guest_amo(memory, width, read_address)?;
            memory_access = Some(GuestMemoryAccess {
                kind: GuestMemoryAccessKind::Read,
                address: read_address,
                byte_len: guest_memory_access_byte_len(amo_width_byte_len(width)),
                value: loaded,
            });
            register_write = write_fast_reported_register(state, rd, amo_result(width, loaded));
            state.set_reservation(read_address, width);
        }
        RiscvInstruction::StoreConditional {
            width,
            rd,
            rs1,
            rs2,
            ..
        } => {
            let write_address = state.read_decoded_register(rs1);
            validate_guest_amo_address(memory, width, write_address)?;
            if state.reservation_matches(write_address, width) {
                let value = state.read_decoded_register(rs2);
                write_guest_amo(memory, width, write_address, value)?;
                let byte_len = amo_width_byte_len(width);
                has_memory_write = true;
                memory_access = Some(GuestMemoryAccess {
                    kind: GuestMemoryAccessKind::Write,
                    address: write_address,
                    byte_len: guest_memory_access_byte_len(byte_len),
                    value: low_bytes_value(value, byte_len),
                });
                register_write = write_fast_reported_register(state, rd, 0);
            } else {
                register_write = write_fast_reported_register(state, rd, 1);
            }
            state.clear_reservation();
        }
        RiscvInstruction::CsrRead { csr, rd } => {
            register_write = write_fast_reported_register(
                state,
                rd,
                read_csr(csr, state.retired_instructions()),
            );
        }
        RiscvInstruction::ZiskDmaPrepare { kind, rs1 } => {
            state.set_pending_dma(kind, state.read_decoded_register(rs1));
        }
        RiscvInstruction::ZiskFcallParam { port, rs1 } => {
            state.push_fcall_param(port, state.read_decoded_register(rs1));
        }
        RiscvInstruction::ZiskFcallResult { rd } => {
            let value = state
                .pop_fcall_result()
                .ok_or(GuestMachineError::MissingFcallResult { address })?;
            register_write = write_fast_reported_register(state, rd, value);
        }
        RiscvInstruction::Fence { .. } => {}
        _ => return Ok(None),
    }

    state.set_pc(next_pc);
    state.retire_instruction();
    report.write(GuestMachineReport::new_single_effect(
        address,
        byte_len,
        instruction,
        next_pc,
        register_write,
        memory_access,
    ));
    Ok(Some(GuestMachineReportShape {
        instruction,
        has_memory_write,
    }))
}

fn write_fast_reported_register(
    state: &mut GuestMachineState,
    index: u8,
    value: u64,
) -> Option<GuestRegisterWrite> {
    if index == 0 {
        return None;
    }
    state.write_nonzero_decoded_register(index, value);
    Some(GuestRegisterWrite { index, value })
}

#[inline(always)]
fn execute_fast_pending_dma(
    memory: &mut GuestMachineMemory,
    address: u64,
    instruction: RiscvInstruction,
    state: &mut GuestMachineState,
    pending: GuestDmaPrepare,
) -> Result<Option<GuestRegisterWrite>, GuestMachineError> {
    match instruction {
        RiscvInstruction::Op {
            kind: RiscvOpKind::Add,
            rd,
            rs1,
            rs2,
        } => {
            let dst = state.read_decoded_register(rs1);
            let count = state.read_decoded_register(rs2);
            execute_fast_dma(
                memory,
                state,
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
                execute_fast_dma(
                    memory,
                    state,
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
                execute_fast_dma(
                    memory,
                    state,
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

#[inline(always)]
fn execute_fast_dma(
    memory: &mut GuestMachineMemory,
    state: &mut GuestMachineState,
    request: GuestDmaExecute,
) -> Result<Option<GuestRegisterWrite>, GuestMachineError> {
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
            return Ok(write_fast_reported_register(state, rd, result));
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
    Ok(write_fast_reported_register(state, rd, result))
}

fn advance_timing_started(timing: &Option<&mut GuestMachineAdvanceTiming>) -> Option<Instant> {
    timing.as_ref().map(|_| Instant::now())
}

fn record_advance_timing(
    started: Option<Instant>,
    timing: Option<&mut GuestMachineAdvanceTiming>,
    field: impl FnOnce(&mut GuestMachineAdvanceTiming) -> &mut Duration,
) {
    if let (Some(started), Some(timing)) = (started, timing) {
        *field(timing) += started.elapsed();
    }
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
            write_reported_register(
                state,
                effects,
                rd,
                address.wrapping_add_signed(i64::from(immediate)),
            );
        }
        RiscvInstruction::Jal { rd, offset } => {
            write_reported_register(state, effects, rd, sequential_pc);
            state.set_pc(address.wrapping_add_signed(i64::from(offset)));
        }
        RiscvInstruction::Jalr { rd, rs1, offset } => {
            let target = state
                .read_decoded_register(rs1)
                .wrapping_add_signed(i64::from(offset))
                & !1;
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
                state.set_pc(address.wrapping_add_signed(i64::from(offset)));
            }
        }
        RiscvInstruction::OpImm {
            kind,
            rd,
            rs1,
            immediate,
        } => {
            let value = execute_op_imm(kind, state.read_decoded_register(rs1), immediate);
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
            let address = state
                .read_decoded_register(rs1)
                .wrapping_add_signed(i64::from(offset));
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
            let address = state
                .read_decoded_register(rs1)
                .wrapping_add_signed(i64::from(offset));
            let value = state.read_decoded_register(rs2);
            let byte_len = write_guest_store(memory, kind, address, value)?;
            effects.record_memory_write(address, byte_len, low_bytes_value(value, byte_len));
            state.clear_reservation_if_overlaps(address, byte_len);
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
            state.clear_reservation_if_overlaps(address, byte_len);
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

#[inline(always)]
fn read_guest_load(
    memory: &GuestMachineMemory,
    kind: RiscvLoadKind,
    address: u64,
) -> Result<GuestLoadResult, GuestMachineError> {
    let byte_len = guest_load_byte_len(kind);
    let memory_value = memory.read_u64_le(address, byte_len)?;
    let register_value = match kind {
        RiscvLoadKind::Lb => i64::from(memory_value as u8 as i8) as u64,
        RiscvLoadKind::Lh => i64::from(memory_value as u16 as i16) as u64,
        RiscvLoadKind::Lw => i64::from(memory_value as u32 as i32) as u64,
        RiscvLoadKind::Ld | RiscvLoadKind::Lbu | RiscvLoadKind::Lhu | RiscvLoadKind::Lwu => {
            memory_value
        }
    };
    Ok(GuestLoadResult {
        byte_len,
        memory_value,
        register_value,
    })
}

#[inline(always)]
fn guest_load_byte_len(kind: RiscvLoadKind) -> usize {
    match kind {
        RiscvLoadKind::Lb | RiscvLoadKind::Lbu => 1,
        RiscvLoadKind::Lh | RiscvLoadKind::Lhu => 2,
        RiscvLoadKind::Lw | RiscvLoadKind::Lwu => 4,
        RiscvLoadKind::Ld => 8,
    }
}

fn write_guest_store(
    memory: &mut GuestMachineMemory,
    kind: RiscvStoreKind,
    address: u64,
    value: u64,
) -> Result<usize, GuestMachineError> {
    let byte_len = match kind {
        RiscvStoreKind::Sb => {
            memory.write_u64_le::<1>(address, value)?;
            1
        }
        RiscvStoreKind::Sh => {
            memory.write_u64_le::<2>(address, value)?;
            2
        }
        RiscvStoreKind::Sw => {
            memory.write_u64_le::<4>(address, value)?;
            4
        }
        RiscvStoreKind::Sd => {
            memory.write_u64_le::<8>(address, value)?;
            8
        }
    };
    Ok(byte_len)
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
            memory.read_u64_le(address, 4)?;
        }
        RiscvAmoWidth::Doubleword => {
            memory.read_u64_le(address, 8)?;
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
        RiscvAmoWidth::Word => memory.read_u64_le(address, 4)?,
        RiscvAmoWidth::Doubleword => memory.read_u64_le(address, 8)?,
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
        RiscvAmoWidth::Word => memory.write_u64_le::<4>(address, value)?,
        RiscvAmoWidth::Doubleword => memory.write_u64_le::<8>(address, value)?,
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

fn same_register_branch_is_taken(kind: RiscvBranchKind) -> bool {
    matches!(
        kind,
        RiscvBranchKind::Beq | RiscvBranchKind::Bge | RiscvBranchKind::Bgeu
    )
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

fn execute_op_imm(kind: RiscvOpImmKind, rs1: u64, immediate: i32) -> u64 {
    match kind {
        RiscvOpImmKind::Addi => rs1.wrapping_add_signed(i64::from(immediate)),
        RiscvOpImmKind::Slti => u64::from((rs1 as i64) < i64::from(immediate)),
        RiscvOpImmKind::Sltiu => u64::from(rs1 < immediate as u64),
        RiscvOpImmKind::Xori => rs1 ^ immediate as u64,
        RiscvOpImmKind::Ori => rs1 | immediate as u64,
        RiscvOpImmKind::Andi => rs1 & immediate as u64,
        RiscvOpImmKind::Slli => rs1.wrapping_shl((immediate as u32) & 0x3f),
        RiscvOpImmKind::Srli => rs1.wrapping_shr((immediate as u32) & 0x3f),
        RiscvOpImmKind::Srai => ((rs1 as i64) >> ((immediate as u32) & 0x3f)) as u64,
    }
}

fn execute_op_imm_32(kind: RiscvOpImm32Kind, rs1: u64, immediate: i32) -> u64 {
    match kind {
        RiscvOpImm32Kind::Addiw => {
            sign_extend_word(rs1.wrapping_add_signed(i64::from(immediate)) as u32)
        }
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

    struct TestEnvVarGuard {
        name: &'static str,
        previous: Option<std::ffi::OsString>,
    }

    struct DeclaredMemoryEffectFcallHandler(GuestFcallMemoryEffect);

    impl GuestFcallHandler for DeclaredMemoryEffectFcallHandler {
        fn memory_effect(&self, _function_id: u16) -> GuestFcallMemoryEffect {
            self.0
        }

        fn handle_fcall(
            &mut self,
            _request: GuestFcallRequest,
            _memory: &mut GuestMachineMemory,
        ) -> Result<GuestFcallResponse, GuestFcallError> {
            Ok(GuestFcallResponse {
                results: Vec::new(),
            })
        }
    }

    impl TestEnvVarGuard {
        fn set(name: &'static str, value: &str) -> Self {
            let previous = std::env::var_os(name);
            std::env::set_var(name, value);
            Self { name, previous }
        }

        fn unset(name: &'static str) -> Self {
            let previous = std::env::var_os(name);
            std::env::remove_var(name);
            Self { name, previous }
        }
    }

    impl Drop for TestEnvVarGuard {
        fn drop(&mut self) {
            match &self.previous {
                Some(value) => std::env::set_var(self.name, value),
                None => std::env::remove_var(self.name),
            }
        }
    }

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

    fn sw(rs2: u8, rs1: u8, immediate: i16) -> u32 {
        let immediate = (immediate as i32 as u32) & 0x0fff;
        ((immediate >> 5) << 25)
            | (u32::from(rs2) << 20)
            | (u32::from(rs1) << 15)
            | (0x2 << 12)
            | ((immediate & 0x1f) << 7)
            | 0x23
    }

    fn jal(rd: u8, offset: i32) -> u32 {
        let offset = offset as u32;
        ((offset >> 20) & 0x1) << 31
            | ((offset >> 1) & 0x03ff) << 21
            | ((offset >> 11) & 0x1) << 20
            | ((offset >> 12) & 0x00ff) << 12
            | (u32::from(rd) << 7)
            | 0x6f
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
    fn register_write_list_uses_x0_sentinel_slot() {
        let empty = GuestRegisterWriteList::default();
        assert!(empty.is_empty());
        assert_eq!(empty.as_slice(), &[]);
        assert_eq!(empty.single(), None);

        let write = GuestRegisterWrite { index: 3, value: 7 };
        let one = GuestRegisterWriteList::one(write);
        assert_eq!(one.len(), 1);
        assert_eq!(one.as_slice(), &[write]);
        assert_eq!(one.single(), Some(write));
        assert_eq!(
            std::mem::size_of::<GuestRegisterWriteList>(),
            std::mem::size_of::<GuestRegisterWrite>()
        );
        assert!(std::mem::size_of::<GuestMachineReport>() <= 72);
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

    fn report_fast_path_test_state() -> GuestMachineState {
        let mut state = GuestMachineState::new(TEST_ENTRY);
        for (index, value) in [
            (1, 5),
            (2, 5),
            (3, 0x0123_4567_89ab_cdef),
            (4, 0xfedc_ba98_7654_3210),
            (6, TEST_ENTRY),
            (7, TEST_ENTRY + 32),
        ] {
            state
                .set_register(index, value)
                .expect("test register should set");
        }
        state.retired_instructions = (3_u64 << 32) | 17;
        state.set_fcall_results(vec![0x0123_4567_89ab_cdef]);
        state
    }

    fn advance_guest_machine_prepared_inner_with_report_shape_generic_for_test(
        memory: &mut GuestMachineMemory,
        state: &mut GuestMachineState,
        handler: Option<&mut dyn GuestFcallHandler>,
        prepared: GuestMachinePreparedInstruction,
        timing: Option<&mut GuestMachineAdvanceTiming>,
    ) -> Result<GuestMachineAdvanceReport, GuestMachineError> {
        let pc = state.pc();
        if prepared.address != pc {
            return Err(GuestMachineError::PreparedInstructionPcMismatch {
                expected: pc,
                found: prepared.address,
            });
        }
        let sequential_pc = pc.checked_add(u64::from(prepared.byte_len)).ok_or(
            GuestMachineError::ProgramCounterOverflow {
                address: pc,
                byte_len: usize::from(prepared.byte_len),
            },
        )?;
        advance_guest_machine_prepared_inner_report_shape_at_pc_generic(
            memory,
            state,
            handler,
            pc,
            prepared.byte_len,
            sequential_pc,
            prepared.instruction,
            timing,
        )
    }

    fn assert_report_fast_path_matches_generic_with_words(
        instruction: RiscvInstruction,
        words: &[u32],
    ) {
        assert_report_fast_path_matches_generic_with_words_and_state(instruction, words, |_| {});
    }

    fn assert_report_fast_path_matches_generic_with_words_and_state(
        instruction: RiscvInstruction,
        words: &[u32],
        configure_state: impl Fn(&mut GuestMachineState),
    ) {
        let mut fast_memory = guest_machine_memory_with_words(words);
        let mut generic_memory = fast_memory.clone();
        let mut direct_memory = fast_memory.clone();
        let mut fast_state = report_fast_path_test_state();
        configure_state(&mut fast_state);
        let mut generic_state = fast_state.clone();
        let mut direct_state = fast_state.clone();
        let prepared = GuestMachinePreparedInstruction {
            address: TEST_ENTRY,
            byte_len: 4,
            instruction,
        };
        let mut direct_report = MaybeUninit::new(GuestMachineReport::new(
            TEST_ENTRY,
            4,
            instruction,
            TEST_ENTRY + 4,
            GuestRegisterWriteList::default(),
            GuestMemoryAccessList::default(),
            None,
        ));
        let direct_shape = try_advance_guest_machine_report_fast_path(
            &mut direct_memory,
            &mut direct_state,
            TEST_ENTRY,
            4,
            TEST_ENTRY + 4,
            instruction,
            &mut direct_report,
        )
        .expect("direct report fast path should not fail")
        .expect("instruction should use report fast path");
        let fast = advance_guest_machine_prepared_inner_with_report_shape(
            &mut fast_memory,
            &mut fast_state,
            None,
            prepared,
            None,
            None,
        )
        .expect("fast prepared advance should succeed");
        let mut timing = GuestMachineAdvanceTiming::default();
        let generic = advance_guest_machine_prepared_inner_with_report_shape_generic_for_test(
            &mut generic_memory,
            &mut generic_state,
            None,
            prepared,
            Some(&mut timing),
        )
        .expect("generic prepared advance should succeed");

        assert_eq!(fast, generic);
        assert_eq!(direct_shape, generic.shape);
        assert_eq!(fast_state, generic_state);
        assert_eq!(direct_state, generic_state);
        assert_eq!(fast_memory, generic_memory);
        assert_eq!(direct_memory, generic_memory);
    }

    fn assert_report_fast_path_matches_generic(instruction: RiscvInstruction) {
        assert_report_fast_path_matches_generic_with_words(instruction, &[0x0000_0073]);
    }

    #[test]
    fn timed_prepared_advance_keeps_report_fast_path_enabled() {
        let instruction = RiscvInstruction::Lui {
            rd: 8,
            immediate: 0x1234_5000,
        };
        let mut memory = guest_machine_memory_with_words(&[0x0000_0073]);
        let mut state = report_fast_path_test_state();
        let prepared = GuestMachinePreparedInstruction {
            address: TEST_ENTRY,
            byte_len: 4,
            instruction,
        };
        let mut timing = GuestMachineAdvanceTiming::default();

        let advanced = advance_guest_machine_prepared_inner_with_report_shape(
            &mut memory,
            &mut state,
            None,
            prepared,
            Some(&mut timing),
            None,
        )
        .expect("timed fast-path advance should succeed");

        assert_eq!(advanced.shape.instruction, instruction);
        assert_eq!(state.pc(), TEST_ENTRY + 4);
        assert_eq!(timing.setup_duration, Duration::ZERO);
        assert_eq!(timing.report_duration, Duration::ZERO);
    }

    fn assert_pending_dma_fast_path_matches_generic(
        pending: GuestDmaPrepare,
        instruction: RiscvInstruction,
        configure_state: impl Fn(&mut GuestMachineState),
    ) {
        let mut fast_memory = guest_machine_memory_with_words(&[
            0x0302_0100,
            0x0706_0504,
            0x0b0a_0908,
            0x0f0e_0d0c,
            0x1312_1110,
            0x1716_1514,
            0x1b1a_1918,
            0x1f1e_1d1c,
            0x2322_2120,
            0x2726_2524,
            0x2b2a_2928,
            0x2f2e_2d2c,
        ]);
        let mut generic_memory = fast_memory.clone();
        let mut fast_state = GuestMachineState::new(TEST_ENTRY);
        fast_state.pending_dma = Some(pending);
        configure_state(&mut fast_state);
        let mut generic_state = fast_state.clone();
        let prepared = GuestMachinePreparedInstruction {
            address: TEST_ENTRY,
            byte_len: 4,
            instruction,
        };
        let fast = advance_guest_machine_prepared_inner_with_report_shape(
            &mut fast_memory,
            &mut fast_state,
            None,
            prepared,
            None,
            None,
        )
        .expect("fast prepared advance should succeed");
        let mut timing = GuestMachineAdvanceTiming::default();
        let generic = advance_guest_machine_prepared_inner_with_report_shape_generic_for_test(
            &mut generic_memory,
            &mut generic_state,
            None,
            prepared,
            Some(&mut timing),
        )
        .expect("generic prepared advance should succeed");

        assert_eq!(fast, generic);
        assert_eq!(fast_state, generic_state);
        assert_eq!(fast_memory, generic_memory);
    }

    #[test]
    fn prepared_report_fast_path_matches_timed_generic_advance() {
        for instruction in [
            RiscvInstruction::Lui {
                rd: 8,
                immediate: 0x1234_5000,
            },
            RiscvInstruction::Auipc {
                rd: 8,
                immediate: 16,
            },
            RiscvInstruction::Jal { rd: 8, offset: 16 },
            RiscvInstruction::Jalr {
                rd: 8,
                rs1: 7,
                offset: 3,
            },
            RiscvInstruction::Branch {
                kind: RiscvBranchKind::Beq,
                rs1: 1,
                rs2: 2,
                offset: 16,
            },
            RiscvInstruction::Branch {
                kind: RiscvBranchKind::Bne,
                rs1: 1,
                rs2: 2,
                offset: 16,
            },
            RiscvInstruction::OpImm {
                kind: RiscvOpImmKind::Slli,
                rd: 8,
                rs1: 3,
                immediate: 4,
            },
            RiscvInstruction::Op {
                kind: RiscvOpKind::Mulhu,
                rd: 8,
                rs1: 3,
                rs2: 4,
            },
            RiscvInstruction::OpImm32 {
                kind: RiscvOpImm32Kind::Sraiw,
                rd: 8,
                rs1: 3,
                immediate: 7,
            },
            RiscvInstruction::Op32 {
                kind: RiscvOp32Kind::Addw,
                rd: 8,
                rs1: 3,
                rs2: 4,
            },
            RiscvInstruction::Load {
                kind: RiscvLoadKind::Lwu,
                rd: 8,
                rs1: 6,
                offset: 0,
            },
            RiscvInstruction::Amo {
                kind: RiscvAmoKind::Add,
                width: RiscvAmoWidth::Word,
                rd: 8,
                rs1: 6,
                rs2: 3,
                acquire: false,
                release: false,
            },
            RiscvInstruction::LoadReserved {
                width: RiscvAmoWidth::Word,
                rd: 8,
                rs1: 6,
                acquire: false,
                release: false,
            },
            RiscvInstruction::Store {
                kind: RiscvStoreKind::Sw,
                rs1: 6,
                rs2: 3,
                offset: 0,
            },
            RiscvInstruction::CsrRead {
                csr: RiscvCsr::Cycle,
                rd: 8,
            },
            RiscvInstruction::ZiskDmaPrepare {
                kind: RiscvDmaKind::Memcpy,
                rs1: 6,
            },
            RiscvInstruction::ZiskFcallParam { port: 1, rs1: 6 },
            RiscvInstruction::ZiskFcallResult { rd: 8 },
            RiscvInstruction::Fence {
                kind: crate::guest_instruction::RiscvFenceKind::Fence,
                mode: 0,
                predecessor: 0,
                successor: 0,
            },
        ] {
            assert_report_fast_path_matches_generic(instruction);
        }
    }

    #[test]
    fn prepared_same_register_branches_match_timed_generic_advance() {
        for kind in [
            RiscvBranchKind::Beq,
            RiscvBranchKind::Bne,
            RiscvBranchKind::Blt,
            RiscvBranchKind::Bge,
            RiscvBranchKind::Bltu,
            RiscvBranchKind::Bgeu,
        ] {
            assert_report_fast_path_matches_generic(RiscvInstruction::Branch {
                kind,
                rs1: 1,
                rs2: 1,
                offset: 16,
            });
        }
    }

    #[test]
    fn prepared_addiw_fast_path_matches_timed_generic_advance() {
        assert_report_fast_path_matches_generic(RiscvInstruction::OpImm32 {
            kind: RiscvOpImm32Kind::Addiw,
            rd: 8,
            rs1: 3,
            immediate: -7,
        });
    }

    #[test]
    fn prepared_same_register_ops_match_timed_generic_advance() {
        for kind in [
            RiscvOpKind::Add,
            RiscvOpKind::Sub,
            RiscvOpKind::Sll,
            RiscvOpKind::Slt,
            RiscvOpKind::Sltu,
            RiscvOpKind::Xor,
            RiscvOpKind::Srl,
            RiscvOpKind::Sra,
            RiscvOpKind::Or,
            RiscvOpKind::And,
            RiscvOpKind::Mul,
            RiscvOpKind::Mulh,
            RiscvOpKind::Mulhsu,
            RiscvOpKind::Mulhu,
            RiscvOpKind::Div,
            RiscvOpKind::Divu,
            RiscvOpKind::Rem,
            RiscvOpKind::Remu,
        ] {
            assert_report_fast_path_matches_generic(RiscvInstruction::Op {
                kind,
                rd: 8,
                rs1: 6,
                rs2: 6,
            });
        }

        for kind in [
            RiscvOp32Kind::Addw,
            RiscvOp32Kind::Subw,
            RiscvOp32Kind::Sllw,
            RiscvOp32Kind::Srlw,
            RiscvOp32Kind::Sraw,
            RiscvOp32Kind::Mulw,
            RiscvOp32Kind::Divw,
            RiscvOp32Kind::Divuw,
            RiscvOp32Kind::Remw,
            RiscvOp32Kind::Remuw,
        ] {
            assert_report_fast_path_matches_generic(RiscvInstruction::Op32 {
                kind,
                rd: 8,
                rs1: 6,
                rs2: 6,
            });
        }
    }

    #[test]
    fn prepared_load_width_fast_paths_match_timed_generic_advance() {
        for (kind, words) in [
            (RiscvLoadKind::Lb, &[0x0000_0080, 0][..]),
            (RiscvLoadKind::Lh, &[0x0000_8000, 0][..]),
            (RiscvLoadKind::Lw, &[0x8000_0000, 0][..]),
            (RiscvLoadKind::Ld, &[0xffff_ffff, 0x8000_0000][..]),
            (RiscvLoadKind::Lbu, &[0x0000_00ff, 0][..]),
            (RiscvLoadKind::Lhu, &[0x0000_ffff, 0][..]),
            (RiscvLoadKind::Lwu, &[0xffff_ffff, 0][..]),
        ] {
            assert_report_fast_path_matches_generic_with_words(
                RiscvInstruction::Load {
                    kind,
                    rd: 8,
                    rs1: 6,
                    offset: 0,
                },
                words,
            );
        }
    }

    #[test]
    fn prepared_store_width_fast_paths_match_timed_generic_advance() {
        for kind in [
            RiscvStoreKind::Sb,
            RiscvStoreKind::Sh,
            RiscvStoreKind::Sw,
            RiscvStoreKind::Sd,
        ] {
            assert_report_fast_path_matches_generic_with_words(
                RiscvInstruction::Store {
                    kind,
                    rs1: 6,
                    rs2: 3,
                    offset: 0,
                },
                &[0, 0],
            );
        }
    }

    #[test]
    fn prepared_atomic_width_fast_paths_match_timed_generic_advance() {
        for (width, words) in [
            (RiscvAmoWidth::Word, &[0x8000_0073, 0][..]),
            (RiscvAmoWidth::Doubleword, &[0x89ab_cdef, 0x0123_4567][..]),
        ] {
            assert_report_fast_path_matches_generic_with_words(
                RiscvInstruction::Amo {
                    kind: RiscvAmoKind::Add,
                    width,
                    rd: 8,
                    rs1: 6,
                    rs2: 3,
                    acquire: false,
                    release: false,
                },
                words,
            );
            assert_report_fast_path_matches_generic_with_words(
                RiscvInstruction::LoadReserved {
                    width,
                    rd: 8,
                    rs1: 6,
                    acquire: false,
                    release: false,
                },
                words,
            );
        }
    }

    #[test]
    fn prepared_store_conditional_fast_paths_match_timed_generic_advance() {
        let instruction = RiscvInstruction::StoreConditional {
            width: RiscvAmoWidth::Doubleword,
            rd: 8,
            rs1: 6,
            rs2: 3,
            acquire: false,
            release: false,
        };
        assert_report_fast_path_matches_generic_with_words(instruction, &[0, 0]);
        assert_report_fast_path_matches_generic_with_words_and_state(
            instruction,
            &[0, 0],
            |state| state.set_reservation(TEST_ENTRY, RiscvAmoWidth::Doubleword),
        );
    }

    #[test]
    fn prepared_zero_destination_fast_paths_match_timed_generic_advance() {
        for instruction in [
            RiscvInstruction::Lui {
                rd: 0,
                immediate: 0x1234_5000,
            },
            RiscvInstruction::Jal { rd: 0, offset: 16 },
            RiscvInstruction::Jalr {
                rd: 0,
                rs1: 7,
                offset: 3,
            },
            RiscvInstruction::OpImm {
                kind: RiscvOpImmKind::Addi,
                rd: 0,
                rs1: 3,
                immediate: 4,
            },
            RiscvInstruction::Load {
                kind: RiscvLoadKind::Lwu,
                rd: 0,
                rs1: 6,
                offset: 0,
            },
            RiscvInstruction::CsrRead {
                csr: RiscvCsr::Cycle,
                rd: 0,
            },
        ] {
            assert_report_fast_path_matches_generic(instruction);
        }
    }

    #[test]
    fn prepared_pending_dma_fast_path_matches_timed_generic_advance() {
        assert_pending_dma_fast_path_matches_generic(
            GuestDmaPrepare {
                kind: RiscvDmaKind::Memcpy,
                first_arg: TEST_ENTRY + 32,
            },
            RiscvInstruction::Op {
                kind: RiscvOpKind::Add,
                rd: 8,
                rs1: 6,
                rs2: 2,
            },
            |state| {
                state.set_register(2, 8).expect("test register should set");
                state
                    .set_register(6, TEST_ENTRY + 16)
                    .expect("test register should set");
            },
        );
        assert_pending_dma_fast_path_matches_generic(
            GuestDmaPrepare {
                kind: RiscvDmaKind::Memcmp,
                first_arg: TEST_ENTRY + 24,
            },
            RiscvInstruction::Op {
                kind: RiscvOpKind::Add,
                rd: 8,
                rs1: 6,
                rs2: 2,
            },
            |state| {
                state.set_register(2, 8).expect("test register should set");
                state
                    .set_register(6, TEST_ENTRY + 16)
                    .expect("test register should set");
            },
        );
        assert_pending_dma_fast_path_matches_generic(
            GuestDmaPrepare {
                kind: RiscvDmaKind::Memset,
                first_arg: TEST_ENTRY + 16,
            },
            RiscvInstruction::OpImm {
                kind: RiscvOpImmKind::Addi,
                rd: 8,
                rs1: 2,
                immediate: 0x5a,
            },
            |state| {
                state.set_register(2, 8).expect("test register should set");
            },
        );
        assert_pending_dma_fast_path_matches_generic(
            GuestDmaPrepare {
                kind: RiscvDmaKind::Inputcpy,
                first_arg: 0,
            },
            RiscvInstruction::OpImm {
                kind: RiscvOpImmKind::Addi,
                rd: 8,
                rs1: 6,
                immediate: 8,
            },
            |state| {
                state
                    .set_register(6, TEST_ENTRY + 16)
                    .expect("test register should set");
                state.set_fcall_results(vec![0x0123_4567_89ab_cdef]);
            },
        );
    }

    #[test]
    fn run_cache_invalidates_stored_instruction() {
        let mut memory =
            guest_machine_memory_with_words(&[addi(1, 0, 0x73), sw(1, 2, 0), jal(0, -8)]);
        let mut state = GuestMachineState::new(memory.entry_address());
        state
            .set_register(2, TEST_ENTRY)
            .expect("test register should be writable");

        let run = run_guest_machine(&mut memory, &mut state, 16).expect("run should halt");

        assert_eq!(
            run,
            GuestMachineRunReport {
                executed_instructions: 3,
                halt: GuestMachineHalt::Ecall {
                    address: TEST_ENTRY,
                },
            }
        );
    }

    #[test]
    fn prepared_advance_returns_report_shape_at_execution_boundary() {
        let mut memory = guest_machine_memory_with_words(&[addi(1, 0, 7), 0x0000_0073]);
        let mut state = GuestMachineState::new(memory.entry_address());
        let prepared = prepare_current_guest_instruction(&memory, state.pc())
            .expect("instruction should prepare");

        let advanced = advance_guest_machine_prepared_inner_with_report_shape(
            &mut memory,
            &mut state,
            None,
            prepared,
            None,
            None,
        )
        .expect("prepared advance should succeed");

        assert_eq!(
            advanced.shape,
            GuestMachineReportShape {
                instruction: RiscvInstruction::OpImm {
                    kind: RiscvOpImmKind::Addi,
                    rd: 1,
                    rs1: 0,
                    immediate: 7,
                },
                has_memory_write: false,
            }
        );
        assert_eq!(advanced.report.instruction, advanced.shape.instruction);
    }

    #[test]
    fn prepared_advance_report_shape_tracks_store_conditional_write_success() {
        let instruction = RiscvInstruction::StoreConditional {
            width: RiscvAmoWidth::Doubleword,
            rd: 3,
            rs1: 1,
            rs2: 2,
            acquire: false,
            release: false,
        };
        let prepared = GuestMachinePreparedInstruction {
            address: TEST_ENTRY,
            byte_len: 4,
            instruction,
        };

        let mut success_memory = guest_machine_memory_with_words(&[addi(0, 0, 0), 0x0000_0073]);
        let mut success_state = GuestMachineState::new(TEST_ENTRY);
        success_state
            .set_register(1, TEST_ENTRY)
            .expect("test register should be writable");
        success_state
            .set_register(2, 0x1122_3344_5566_7788)
            .expect("test register should be writable");
        success_state.set_reservation(TEST_ENTRY, RiscvAmoWidth::Doubleword);
        let success = advance_guest_machine_prepared_inner_with_report_shape(
            &mut success_memory,
            &mut success_state,
            None,
            prepared,
            None,
            None,
        )
        .expect("successful store conditional should advance");
        assert_eq!(
            success.shape,
            GuestMachineReportShape {
                instruction,
                has_memory_write: true,
            }
        );

        let mut failure_memory = guest_machine_memory_with_words(&[addi(0, 0, 0), 0x0000_0073]);
        let mut failure_state = GuestMachineState::new(TEST_ENTRY);
        failure_state
            .set_register(1, TEST_ENTRY)
            .expect("test register should be writable");
        failure_state
            .set_register(2, 0x1122_3344_5566_7788)
            .expect("test register should be writable");
        let failure = advance_guest_machine_prepared_inner_with_report_shape(
            &mut failure_memory,
            &mut failure_state,
            None,
            prepared,
            None,
            None,
        )
        .expect("failed store conditional should advance");
        assert_eq!(
            failure.shape,
            GuestMachineReportShape {
                instruction,
                has_memory_write: false,
            }
        );
    }

    #[test]
    fn report_shape_tracks_precompile_memory_writes() {
        let mut effects = GuestInstructionEffects::default();
        effects.record_precompile_memory_write(TEST_ENTRY, 4, 0x73);

        let shape = effects.report_shape(RiscvInstruction::Fence {
            kind: crate::guest_instruction::RiscvFenceKind::Fence,
            mode: 0,
            predecessor: 0,
            successor: 0,
        });

        assert!(shape.has_memory_write);
    }

    #[test]
    fn instruction_cache_entry_count_uses_bounded_env_bits() {
        let _env_lock = crate::CUDA_TEST_ENV_LOCK
            .lock()
            .expect("test env lock should not be poisoned");
        {
            let _bits_env = TestEnvVarGuard::unset(GUEST_INSTRUCTION_CACHE_ENTRY_BITS_ENV);
            let cache = GuestInstructionCache::default();
            assert_eq!(guest_instruction_cache_entry_count(), 1 << 18);
            assert_eq!(cache.entries.len(), 1 << 18);
            assert_eq!(cache.entry_mask, (1 << 18) - 1);
        }
        {
            let _bits_env = TestEnvVarGuard::set(GUEST_INSTRUCTION_CACHE_ENTRY_BITS_ENV, "18");
            let cache = GuestInstructionCache::default();
            assert_eq!(guest_instruction_cache_entry_count(), 1 << 18);
            assert_eq!(cache.entries.len(), 1 << 18);
            assert_eq!(cache.entry_mask, (1 << 18) - 1);
        }
        {
            let _bits_env = TestEnvVarGuard::set(GUEST_INSTRUCTION_CACHE_ENTRY_BITS_ENV, "99");
            let cache = GuestInstructionCache::default();
            assert_eq!(
                guest_instruction_cache_entry_count(),
                GUEST_INSTRUCTION_CACHE_ENTRY_COUNT
            );
            assert_eq!(cache.entries.len(), GUEST_INSTRUCTION_CACHE_ENTRY_COUNT);
            assert_eq!(cache.entry_mask, GUEST_INSTRUCTION_CACHE_ENTRY_COUNT - 1);
        }
    }

    #[test]
    fn fcall_cache_update_uses_declared_memory_effect() {
        let state = GuestMachineState::new(TEST_ENTRY);
        let instruction = RiscvInstruction::ZiskFcallInvoke { function_id: 7 };

        assert_eq!(
            instruction_cache_update_for_instruction(&state, instruction),
            GuestInstructionCacheUpdate::Clear(GuestInstructionCacheClearReason::FcallInvoke)
        );
        assert_eq!(
            instruction_cache_update_for_instruction_with_handler(
                &state,
                instruction,
                &DeclaredMemoryEffectFcallHandler(GuestFcallMemoryEffect::ReadOnly),
            ),
            GuestInstructionCacheUpdate::None
        );
        assert_eq!(
            instruction_cache_update_for_instruction_with_handler(
                &state,
                instruction,
                &DeclaredMemoryEffectFcallHandler(GuestFcallMemoryEffect::WriteRange {
                    address: TEST_ENTRY + 0x1000,
                    byte_len: 64,
                }),
            ),
            GuestInstructionCacheUpdate::InvalidateWriteRange {
                address: TEST_ENTRY + 0x1000,
                byte_len: 64,
            }
        );
    }

    #[test]
    fn instruction_cache_shape_gate_preserves_write_invalidation() {
        let mut memory = guest_machine_memory_with_words(&[addi(1, 0, 7), 0x0000_0073]);
        let mut cache = GuestInstructionCache::default();
        let initial = cache
            .prepare(&memory, TEST_ENTRY)
            .expect("instruction should prepare");
        assert_eq!(
            initial.instruction,
            RiscvInstruction::OpImm {
                kind: RiscvOpImmKind::Addi,
                rd: 1,
                rs1: 0,
                immediate: 7,
            }
        );
        write_guest_store(&mut memory, RiscvStoreKind::Sw, TEST_ENTRY, 0x0000_0073)
            .expect("test store should update instruction memory");

        let report = GuestMachineReport {
            address_and_instruction_len:
                crate::guest_machine::pack_report_address_and_instruction_len(TEST_ENTRY, 4),
            instruction: RiscvInstruction::Store {
                kind: RiscvStoreKind::Sw,
                rs1: 1,
                rs2: 2,
                offset: 0,
            },
            next_pc: TEST_ENTRY + 4,
            register_write_value: GuestRegisterWriteValue::default(),
            memory_accesses: vec![GuestMemoryAccess {
                kind: GuestMemoryAccessKind::Write,
                address: TEST_ENTRY,
                byte_len: 4,
                value: 0x0000_0073,
            }]
            .into(),
        };
        cache.invalidate_report_shape(
            &report,
            GuestMachineReportShape {
                instruction: report.instruction,
                has_memory_write: false,
            },
        );
        assert_eq!(
            cache
                .prepare(&memory, TEST_ENTRY)
                .expect("cached instruction should still prepare")
                .instruction,
            initial.instruction
        );

        cache.invalidate_report_shape(
            &report,
            GuestMachineReportShape {
                instruction: report.instruction,
                has_memory_write: true,
            },
        );
        assert_eq!(
            cache
                .prepare(&memory, TEST_ENTRY)
                .expect("invalidated instruction should prepare")
                .instruction,
            RiscvInstruction::Ecall
        );
    }

    #[test]
    fn instruction_cache_keeps_entries_for_disjoint_memory_writes() {
        let mut memory = guest_machine_memory_with_words(&[addi(1, 0, 7), 0x0000_0073]);
        let mut cache = GuestInstructionCache::default();
        let initial = cache
            .prepare(&memory, TEST_ENTRY)
            .expect("instruction should prepare");
        write_guest_store(&mut memory, RiscvStoreKind::Sw, TEST_ENTRY, 0x0000_0073)
            .expect("test store should update instruction memory");

        let report = GuestMachineReport {
            address_and_instruction_len:
                crate::guest_machine::pack_report_address_and_instruction_len(TEST_ENTRY, 4),
            instruction: RiscvInstruction::Store {
                kind: RiscvStoreKind::Sw,
                rs1: 1,
                rs2: 2,
                offset: 0,
            },
            next_pc: TEST_ENTRY + 4,
            register_write_value: GuestRegisterWriteValue::default(),
            memory_accesses: vec![GuestMemoryAccess {
                kind: GuestMemoryAccessKind::Write,
                address: TEST_ENTRY + 0x1000,
                byte_len: 4,
                value: 0x0000_0073,
            }]
            .into(),
        };

        cache.invalidate_report_shape(
            &report,
            GuestMachineReportShape {
                instruction: report.instruction,
                has_memory_write: true,
            },
        );
        assert_eq!(
            cache
                .prepare(&memory, TEST_ENTRY)
                .expect("disjoint write should preserve cached instruction")
                .instruction,
            initial.instruction
        );
    }

    #[test]
    fn common_instruction_write_range_uses_final_atomic_access() {
        let instruction = RiscvInstruction::Amo {
            kind: RiscvAmoKind::Add,
            width: RiscvAmoWidth::Doubleword,
            rd: 3,
            rs1: 4,
            rs2: 5,
            acquire: false,
            release: false,
        };
        let report = GuestMachineReport::new(
            TEST_ENTRY,
            4,
            instruction,
            TEST_ENTRY + 4,
            GuestRegisterWriteList::default(),
            vec![
                GuestMemoryAccess {
                    kind: GuestMemoryAccessKind::Read,
                    address: TEST_ENTRY + 0x1000,
                    byte_len: 8,
                    value: 7,
                },
                GuestMemoryAccess {
                    kind: GuestMemoryAccessKind::Write,
                    address: TEST_ENTRY + 0x1000,
                    byte_len: 8,
                    value: 9,
                },
            ]
            .into(),
            None,
        );

        assert_eq!(
            common_instruction_write_range(
                &report,
                GuestMachineReportShape {
                    instruction,
                    has_memory_write: true,
                },
            ),
            Some((TEST_ENTRY + 0x1000, 8))
        );
        assert_eq!(
            common_instruction_write_range(
                &report,
                GuestMachineReportShape {
                    instruction,
                    has_memory_write: false,
                },
            ),
            None
        );

        let mut cache = GuestInstructionCache::default();
        let mut stats = GuestInstructionCacheStats::default();
        cache.invalidate_report_shape_with_stats(
            &report,
            GuestMachineReportShape {
                instruction,
                has_memory_write: true,
            },
            &mut stats,
        );
        assert_eq!(stats.write_invalidation_range_count, 1);
        assert_eq!(stats.write_invalidation_skipped_range_count, 1);
    }

    #[test]
    fn instruction_cache_dma_update_invalidates_overlapping_write_range() {
        let mut memory = guest_machine_memory_with_words(&[addi(1, 0, 7), 0x0000_0073]);
        let mut cache = GuestInstructionCache::default();
        cache
            .prepare(&memory, TEST_ENTRY)
            .expect("instruction should prepare");
        write_guest_store(&mut memory, RiscvStoreKind::Sw, TEST_ENTRY, 0x0000_0073)
            .expect("test store should update instruction memory");
        let mut state = GuestMachineState::new(TEST_ENTRY);
        state.pending_dma = Some(GuestDmaPrepare {
            kind: RiscvDmaKind::Memset,
            first_arg: TEST_ENTRY,
        });
        state.set_register(2, 4).expect("test register should set");

        let update = instruction_cache_update_for_instruction(
            &state,
            RiscvInstruction::OpImm {
                kind: RiscvOpImmKind::Addi,
                rd: 8,
                rs1: 2,
                immediate: 0,
            },
        );
        let GuestInstructionCacheUpdate::InvalidateWriteRange { address, byte_len } = update else {
            panic!("DMA write should use targeted invalidation: {update:?}");
        };
        let mut stats = GuestInstructionCacheStats::default();
        cache.invalidate_write_range_with_stats(address, byte_len, &mut stats);

        assert_eq!(stats.clear_count, 0);
        assert_eq!(stats.write_invalidation_range_count, 1);
        assert_eq!(stats.invalidated_entry_count, 1);
        assert_eq!(
            cache
                .prepare(&memory, TEST_ENTRY)
                .expect("invalidated instruction should prepare")
                .instruction,
            RiscvInstruction::Ecall
        );
    }

    #[test]
    fn instruction_cache_dma_update_skips_disjoint_write_range() {
        let memory = guest_machine_memory_with_words(&[addi(1, 0, 7), 0x0000_0073]);
        let mut cache = GuestInstructionCache::default();
        let initial = cache
            .prepare(&memory, TEST_ENTRY)
            .expect("instruction should prepare");
        let mut state = GuestMachineState::new(TEST_ENTRY);
        state.pending_dma = Some(GuestDmaPrepare {
            kind: RiscvDmaKind::Memset,
            first_arg: TEST_ENTRY + 0x1000,
        });
        state.set_register(2, 4).expect("test register should set");

        let update = instruction_cache_update_for_instruction(
            &state,
            RiscvInstruction::OpImm {
                kind: RiscvOpImmKind::Addi,
                rd: 8,
                rs1: 2,
                immediate: 0,
            },
        );
        let GuestInstructionCacheUpdate::InvalidateWriteRange { address, byte_len } = update else {
            panic!("DMA write should use targeted invalidation: {update:?}");
        };
        let mut stats = GuestInstructionCacheStats::default();
        cache.invalidate_write_range_with_stats(address, byte_len, &mut stats);

        assert_eq!(stats.clear_count, 0);
        assert_eq!(stats.write_invalidation_range_count, 1);
        assert_eq!(stats.write_invalidation_skipped_range_count, 1);
        assert_eq!(stats.write_invalidation_probe_count, 0);
        assert_eq!(
            cache
                .prepare(&memory, TEST_ENTRY)
                .expect("disjoint write should preserve cached instruction")
                .instruction,
            initial.instruction
        );
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
