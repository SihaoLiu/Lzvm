use super::*;
use crate::guest_instruction::{RiscvInstruction, RiscvOpImmKind, RiscvPrecompileKind};
use crate::witness_layout::derive_witness_trace_layout;
use crate::witness_loader::WitnessComputeContext;
use crate::witness_trace::parse_witness_trace;
use crate::ProveUnitSchedule;
use lzvm_artifacts::guest_image::parse_guest_image;
use lzvm_artifacts::key_directory::KeyUnitKind;
use lzvm_artifacts::pcs_plan::PcsFriLayer;
use lzvm_artifacts::setup_info::CommitmentColumn;
use lzvm_field::Felt;

#[test]
fn guest_trace_detail_timing_sample_stride_uses_positive_env_values() {
    struct EnvGuard {
        name: &'static str,
        previous: Option<std::ffi::OsString>,
    }

    impl EnvGuard {
        fn new(name: &'static str) -> Self {
            let previous = std::env::var_os(name);
            std::env::remove_var(name);
            Self { name, previous }
        }

        fn set(&self, value: &str) {
            std::env::set_var(self.name, value);
        }

        fn clear(&self) {
            std::env::remove_var(self.name);
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.previous {
                Some(value) => std::env::set_var(self.name, value),
                None => std::env::remove_var(self.name),
            }
        }
    }

    let env = EnvGuard::new("LZVM_GUEST_TRACE_DETAIL_TIMING_SAMPLE_STRIDE");
    assert_eq!(guest_pc_trace_detail_timing_sample_stride(), 1);

    env.set("0");
    assert_eq!(guest_pc_trace_detail_timing_sample_stride(), 1);

    env.set("not-a-number");
    assert_eq!(guest_pc_trace_detail_timing_sample_stride(), 1);

    env.set("17");
    assert_eq!(guest_pc_trace_detail_timing_sample_stride(), 17);

    env.clear();
    assert_eq!(guest_pc_trace_detail_timing_sample_stride(), 1);
}

#[test]
fn rejects_add256_precompile_memory_access_address_mismatch() {
    let mut report = add256_report();
    report.precompile_memory_accesses[4].address += 8;

    let error = validate_zisk_main_precompile_memory_accesses(
        3,
        &report,
        ZiskMainReportEffects::from_report(&report),
        64,
    )
    .expect_err("mismatched Add256 precompile memory access should fail");

    assert!(error.to_string().contains("precompile memory access 4"));
}

#[test]
fn builds_zisk_main_segment_trace_without_serialized_roundtrip() {
    let unit = sample_unit_with_zisk_main_columns_rows(2);
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");
    let report = addi_report();

    let written = build_layout_zisk_main_trace_segment(
        &layout,
        std::slice::from_ref(&report),
        report.next_pc,
        &ZiskMainTraceState::new(),
        None,
        ZiskMainTraceSegmentInfo {
            trace_instance_index: 0,
            is_last_segment: true,
            previous_c: 0,
        },
        None,
    )
    .expect("segment trace should build")
    .expect("Zisk Main layout should be supported");

    let trace = written.trace.as_ref().expect("host trace should be built");
    assert_eq!(trace.row_count(), layout.row_count());
    assert_eq!(trace.column_count(), layout.column_count());
    assert_eq!(
        written.output.produced_len,
        trace.values().len() * std::mem::size_of::<u64>()
    );
    let pc_column = layout.column(1, "pc").expect("pc column").trace_column();
    assert_eq!(
        trace.value(0, pc_column),
        Some(Felt::from_canonical(report.address).expect("canonical pc"))
    );

    let mut bytes = vec![0; written.output.produced_len];
    serialize_trace_to_output(trace, written.output.produced_len, &mut bytes)
        .expect("trace should serialize");
    let parsed = parse_witness_trace(&bytes, layout.row_count(), layout.column_count())
        .expect("serialized trace should parse");
    assert_eq!(&parsed, trace);
}

#[test]
fn zisk_main_trace_build_uses_resolved_column_targets() {
    let unit = sample_unit_with_zisk_main_columns_rows(2);
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");
    let report = addi_report();

    crate::witness_layout::reset_column_lookup_count();
    crate::witness_layout::reset_resolved_column_validation_count();
    let written = build_layout_zisk_main_trace_segment(
        &layout,
        std::slice::from_ref(&report),
        report.next_pc,
        &ZiskMainTraceState::new(),
        None,
        ZiskMainTraceSegmentInfo {
            trace_instance_index: 0,
            is_last_segment: true,
            previous_c: 0,
        },
        None,
    )
    .expect("segment trace should build")
    .expect("Zisk Main layout should be supported");

    assert_eq!(
        written
            .trace
            .as_ref()
            .expect("host trace should be built")
            .row_count(),
        layout.row_count()
    );
    assert_eq!(crate::witness_layout::column_lookup_count(), 0);
    assert_eq!(crate::witness_layout::resolved_column_validation_count(), 0);
}

#[test]
fn zisk_main_segment_seed_mirror_matches_written_continuation() {
    let unit = sample_unit_with_zisk_main_columns_rows(2);
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");
    let first_reports = [
        addi_report_at(0x8000_0000, 1, 0, 7, 7),
        addi_report_at(0x8000_0004, 2, 1, 3, 10),
    ];
    let second_reports = [addi_report_at(0x8000_0008, 3, 2, 5, 15)];
    let initial_seed = ZiskMainSegmentSeed::new();
    let first_segment = ZiskMainTraceSegmentInfo {
        trace_instance_index: 0,
        is_last_segment: false,
        previous_c: initial_seed.previous_c,
    };

    let first_written = build_layout_zisk_main_trace_segment(
        &layout,
        &first_reports,
        0x8000_0008,
        &initial_seed.initial_state,
        Some(second_reports[0].instruction),
        first_segment,
        None,
    )
    .expect("first segment should build")
    .expect("Zisk Main layout should be supported");
    let second_seed = advance_zisk_main_segment_seed(
        &layout,
        &first_reports,
        0x8000_0008,
        &initial_seed,
        Some(second_reports[0].instruction),
        first_segment,
    )
    .expect("first segment seed should advance")
    .expect("Zisk Main layout should be supported");

    assert_eq!(second_seed.initial_state, first_written.continuation_state);
    assert_eq!(second_seed.previous_c, first_written.final_state.last_c);

    let second_segment = ZiskMainTraceSegmentInfo {
        trace_instance_index: 1,
        is_last_segment: true,
        previous_c: second_seed.previous_c,
    };
    let second_written = build_layout_zisk_main_trace_segment(
        &layout,
        &second_reports,
        0x8000_000c,
        &second_seed.initial_state,
        None,
        second_segment,
        None,
    )
    .expect("second segment should build")
    .expect("Zisk Main layout should be supported");
    let terminal_seed = advance_zisk_main_segment_seed(
        &layout,
        &second_reports,
        0x8000_000c,
        &second_seed,
        None,
        second_segment,
    )
    .expect("second segment seed should advance")
    .expect("Zisk Main layout should be supported");

    assert_eq!(
        terminal_seed.initial_state,
        second_written.continuation_state
    );
    assert_eq!(terminal_seed.previous_c, second_written.final_state.last_c);
}

#[test]
fn guest_pc_trace_seed_mirror_attaches_pending_segment_seeds_when_enabled() {
    struct EnvGuard {
        name: &'static str,
        previous: Option<std::ffi::OsString>,
    }

    impl EnvGuard {
        fn new(name: &'static str, value: &str) -> Self {
            let previous = std::env::var_os(name);
            std::env::set_var(name, value);
            Self { name, previous }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.previous {
                Some(value) => std::env::set_var(self.name, value),
                None => std::env::remove_var(self.name),
            }
        }
    }

    let _env = EnvGuard::new("LZVM_GUEST_PC_TRACE_SEED_MIRROR", "1");
    let dir = repo_temp_dir("guest-pc-seed-mirror");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let guest_image_bytes = sample_guest_image_with_words(&[
        riscv_addi(1, 0, 7),
        riscv_addi(2, 1, 3),
        riscv_addi(3, 2, 5),
        0x0000_0073,
    ]);
    std::fs::write(&guest_image, &guest_image_bytes).expect("guest image should be written");
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    let unit = sample_unit_with_zisk_main_columns_rows(2);
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");
    let mut pending = Vec::new();

    produce_guest_pc_trace_pending_slices(
        16,
        WitnessComputeContext {
            guest_image: Some(&guest_image),
            guest_image_info: Some(&guest_image_info),
            trace_layout: Some(&layout),
        },
        &[],
        layout.row_count(),
        |segment| {
            pending.push(segment);
            Ok(())
        },
    )
    .expect("pending slices should produce");
    std::fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(pending.len(), 2);
    assert!(pending.iter().all(|segment| segment.seed.is_some()));
    assert_eq!(pending[0].seed.as_ref().unwrap().previous_c, 0);
    assert_eq!(pending[1].seed.as_ref().unwrap().previous_c, 10);
    assert_eq!(
        pending[1].seed.as_ref().unwrap().initial_state.registers[1],
        7
    );
    assert_eq!(
        pending[1].seed.as_ref().unwrap().initial_state.registers[2],
        10
    );
}

#[test]
fn guest_pc_trace_runner_seed_snapshot_matches_mirror_when_enabled() {
    struct EnvGuard {
        name: &'static str,
        previous: Option<std::ffi::OsString>,
    }

    impl EnvGuard {
        fn new(name: &'static str, value: &str) -> Self {
            let previous = std::env::var_os(name);
            std::env::set_var(name, value);
            Self { name, previous }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.previous {
                Some(value) => std::env::set_var(self.name, value),
                None => std::env::remove_var(self.name),
            }
        }
    }

    let _mirror_env = EnvGuard::new("LZVM_GUEST_PC_TRACE_SEED_MIRROR", "1");
    let _snapshot_env = EnvGuard::new("LZVM_GUEST_PC_TRACE_RUNNER_SEED_SNAPSHOT", "1");
    let dir = repo_temp_dir("guest-pc-runner-seed-snapshot");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let guest_image_bytes = sample_guest_image_with_words(&[
        riscv_addi(1, 0, 7),
        riscv_addi(2, 1, 3),
        riscv_addi(3, 2, 5),
        0x0000_0073,
    ]);
    std::fs::write(&guest_image, &guest_image_bytes).expect("guest image should be written");
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    let unit = sample_unit_with_zisk_main_columns_rows(2);
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");
    let mut pending = Vec::new();

    produce_guest_pc_trace_pending_slices(
        16,
        WitnessComputeContext {
            guest_image: Some(&guest_image),
            guest_image_info: Some(&guest_image_info),
            trace_layout: Some(&layout),
        },
        &[],
        layout.row_count(),
        |segment| {
            pending.push(segment);
            Ok(())
        },
    )
    .expect("runner seed snapshot should match mirrored seeds");
    std::fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(pending.len(), 2);
    assert!(pending.iter().all(|segment| segment.seed.is_some()));
}

#[test]
fn runner_boundary_seed_snapshot_carries_dma_prepare_scratch() {
    let mut current_seed = ZiskMainSegmentSeed::new();
    current_seed.initial_state.registers[5] = 0x1000;
    current_seed.initial_state.registers[6] = 0x20;
    let report = GuestMachineReport {
        address: 0x8000_0000,
        instruction_byte_len: 4,
        instruction: RiscvInstruction::ZiskDmaPrepare {
            kind: RiscvDmaKind::Memcpy,
            rs1: 5,
        },
        next_pc: 0x8000_0004,
        register_writes: Vec::new().into(),
        memory_accesses: Vec::new().into(),
        precompile_memory_accesses: Vec::new(),
        precompile_result: None,
    };
    let mut runner_state = GuestMachineState::new(report.next_pc);
    runner_state
        .set_register(5, 0x1000)
        .expect("source register should set");
    runner_state
        .set_register(6, 0x20)
        .expect("count register should set");
    let segment = ZiskMainTraceSegmentInfo {
        trace_instance_index: 0,
        is_last_segment: false,
        previous_c: 0,
    };

    let lifted = lift_zisk_main_next_segment_seed_from_runner_boundary(
        2,
        segment,
        std::slice::from_ref(&report),
        Some(RiscvInstruction::Op {
            kind: RiscvOpKind::Add,
            rd: 7,
            rs1: 8,
            rs2: 6,
        }),
        &runner_state,
        &current_seed,
        0x20,
    )
    .expect("runner boundary seed should lift");

    assert_eq!(lifted.previous_c, 0x20);
    assert_eq!(lifted.initial_state.last_c, 0x20);
    assert_eq!(lifted.initial_state.next_pc, report.next_pc);
    assert_eq!(
        lifted.initial_state.pending_dma,
        Some(ZiskMainPendingDma {
            kind: RiscvDmaKind::Memcpy,
            first_arg_reg: 5,
        })
    );
    assert_eq!(
        lifted
            .initial_state
            .internal_memory
            .get(&ZISK_EXTRA_PARAMS_ADDRESS)
            .copied(),
        Some(0x20)
    );
}

#[test]
#[cfg(feature = "cuda")]
fn zisk_main_device_descriptor_uses_compact_words_when_values_fit() {
    let mut descriptors = ZiskMainDeviceTraceDescriptors::new(2, 39, 0x2000);
    let values = zisk_main_descriptor_trace_values(0x1000, 5, 6, 21, 22, 23, 24, 7);

    append_main_device_trace_descriptor(&mut descriptors, &values)
        .expect("descriptor row should append");

    assert_eq!(descriptors.descriptor_word_count(), 11);
    assert_eq!(descriptors.words().len(), 11);
}

#[test]
#[cfg(feature = "cuda")]
fn main_descriptor_width_tracks_segment_mem_step_capacity() {
    assert_eq!(
        main_segment_descriptor_words(120_000_000, 0),
        ZISK_MAIN_DEVICE_TRACE_DESCRIPTOR_WORDS
    );
    assert_eq!(
        main_segment_descriptor_words(120_000_000, 9),
        ZISK_MAIN_DEVICE_TRACE_WIDE_DESCRIPTOR_WORDS
    );
}

#[test]
#[cfg(feature = "cuda")]
fn zisk_main_device_descriptor_falls_back_to_wide_words_when_values_do_not_fit() {
    let mut descriptors = ZiskMainDeviceTraceDescriptors::new(2, 39, 0x2000);
    let first = zisk_main_descriptor_trace_values(0x1000, 5, 6, 21, 22, 23, 24, 7);
    append_main_device_trace_descriptor(&mut descriptors, &first)
        .expect("first compact descriptor row should append");

    let second = zisk_main_descriptor_trace_values(
        0x1004,
        i64::from(i32::MAX) + 1,
        6,
        u64::from(u32::MAX) + 1,
        22,
        23,
        24,
        7,
    );
    append_main_device_trace_descriptor(&mut descriptors, &second)
        .expect("wide fallback descriptor row should append");

    assert_eq!(descriptors.descriptor_word_count(), 14);
    assert_eq!(descriptors.words().len(), 28);
    assert_eq!(descriptors.words()[8], 5_u64);
    assert_eq!(descriptors.words()[10], 21_u64);
    assert_eq!(
        descriptors.words()[14 + 8],
        (i64::from(i32::MAX) + 1) as u64
    );
    assert_eq!(descriptors.words()[14 + 10], u64::from(u32::MAX) + 1);
}

#[test]
fn matching_memory_access_rejects_duplicate_matches() {
    let accesses = [
        memory_read(64, 11),
        memory_write(96, 22),
        memory_read(64, 33),
    ];
    let effects = ZiskMainReportEffects {
        register_writes: &[],
        memory_accesses: &accesses,
        precompile_memory_accesses: &[],
        precompile_result: None,
    };

    let error = matching_memory_access(7, effects, GuestMemoryAccessKind::Read, 64, 8)
        .expect_err("duplicate matching memory accesses should fail");

    assert!(error.to_string().contains("multiple Read accesses at 64"));
}

#[test]
fn zisk_main_memory_access_validation_preserves_source_then_store_order() {
    let mut instruction = zisk_main_base_instruction(
        0x8000_0000,
        ZiskMainSource::Memory(64),
        ZiskMainSource::Memory(72),
        ZiskMainOp::CopyB,
        ZiskMainStore::Indirect(0),
        4,
    );
    instruction.ind_width = 8;
    let a_access = ExpectedMemoryAccess {
        kind: GuestMemoryAccessKind::Read,
        address: 64,
        byte_len: 8,
        value: 96,
    };
    let b_access = ExpectedMemoryAccess {
        kind: GuestMemoryAccessKind::Read,
        address: 72,
        byte_len: 8,
        value: 13,
    };
    let store_access = memory_write(96, 13);
    let ordered_accesses = [memory_read(64, 96), memory_read(72, 13), store_access];
    let effects = ZiskMainReportEffects {
        register_writes: &[],
        memory_accesses: &ordered_accesses,
        precompile_memory_accesses: &[],
        precompile_result: None,
    };

    validate_zisk_main_memory_accesses(
        9,
        &instruction,
        effects,
        96,
        13,
        Some(a_access),
        Some(b_access),
    )
    .expect("ordered source and store accesses should validate");

    let reordered_accesses = [memory_read(72, 13), memory_read(64, 96), store_access];
    let effects = ZiskMainReportEffects {
        register_writes: &[],
        memory_accesses: &reordered_accesses,
        precompile_memory_accesses: &[],
        precompile_result: None,
    };
    let error = validate_zisk_main_memory_accesses(
        9,
        &instruction,
        effects,
        96,
        13,
        Some(a_access),
        Some(b_access),
    )
    .expect_err("reordered source accesses should fail");

    assert!(error.to_string().contains("expected Read at 64"));
}

#[test]
fn register_mem_steps_preserve_same_register_access_order() {
    let row = 5;
    let row_count = 100;
    let segment = ZiskMainTraceSegmentInfo {
        trace_instance_index: 2,
        is_last_segment: false,
        previous_c: 0,
    };
    let mut state = ZiskMainTraceState::new();
    state.registers[1] = 99;
    state.register_mem_steps[1] = 42;
    let instruction = zisk_main_base_instruction(
        0x8000_0000,
        ZiskMainSource::Register(1),
        ZiskMainSource::Register(1),
        ZiskMainOp::Add,
        ZiskMainStore::Register(1),
        4,
    );

    let values =
        apply_zisk_main_register_access_values(row, &instruction, &mut state, row_count, segment)
            .expect("same-register accesses should validate");

    let a_step = zisk_main_row_mem_step(
        row_count,
        segment.trace_instance_index,
        row,
        ZISK_MAIN_A_MEM_STEP_OFFSET,
    )
    .expect("A mem step should fit");
    let b_step = zisk_main_row_mem_step(
        row_count,
        segment.trace_instance_index,
        row,
        ZISK_MAIN_B_MEM_STEP_OFFSET,
    )
    .expect("B mem step should fit");
    let store_step = zisk_main_row_mem_step(
        row_count,
        segment.trace_instance_index,
        row,
        ZISK_MAIN_STORE_MEM_STEP_OFFSET,
    )
    .expect("store mem step should fit");

    assert_eq!(values.a_prev_mem_step, Some(42));
    assert_eq!(values.b_prev_mem_step, Some(a_step));
    assert_eq!(values.store_prev_mem_step, Some(b_step));
    assert_eq!(values.store_prev_value, Some(99));
    assert_eq!(state.register_mem_steps[1], store_step);
}

#[test]
fn row_mem_step_base_matches_direct_offset_helper() {
    for (row_count, trace_instance_index, row) in
        [(1, 0, 0), (100, 2, 5), (120_000_000, 8, 119_999_999)]
    {
        let base = zisk_main_row_mem_step_base(row_count, trace_instance_index, row)
            .expect("row mem-step base should fit");
        for offset in [
            ZISK_MAIN_A_MEM_STEP_OFFSET,
            ZISK_MAIN_B_MEM_STEP_OFFSET,
            ZISK_MAIN_STORE_MEM_STEP_OFFSET,
            ZISK_MAIN_SPECIAL_MEM_STEP_OFFSET,
        ] {
            assert_eq!(
                zisk_main_mem_step_from_base(base, offset).expect("offset mem-step should fit"),
                zisk_main_row_mem_step(row_count, trace_instance_index, row, offset)
                    .expect("direct mem-step should fit")
            );
        }
    }
}

fn add256_report() -> GuestMachineReport {
    let params_address = 64;
    let a_address = 96;
    let b_address = 128;
    let c_address = 160;
    let mut precompile_memory_accesses = vec![
        memory_read(params_address, a_address),
        memory_read(params_address + 8, b_address),
        memory_read(params_address + 16, 0),
        memory_read(params_address + 24, c_address),
    ];
    precompile_memory_accesses.extend([
        memory_read(a_address, u64::MAX),
        memory_read(a_address + 8, u64::MAX),
        memory_read(a_address + 16, u64::MAX),
        memory_read(a_address + 24, u64::MAX),
        memory_read(b_address, 1),
        memory_read(b_address + 8, 0),
        memory_read(b_address + 16, 0),
        memory_read(b_address + 24, 0),
        memory_write(c_address, 0),
        memory_write(c_address + 8, 0),
        memory_write(c_address + 16, 0),
        memory_write(c_address + 24, 0),
    ]);
    GuestMachineReport {
        address: 0x8000_0000,
        instruction_byte_len: 4,
        instruction: RiscvInstruction::ZiskPrecompile {
            kind: RiscvPrecompileKind::Add256,
            rs1: 1,
            rd: 2,
        },
        next_pc: 0x8000_0004,
        register_writes: vec![GuestRegisterWrite { index: 2, value: 1 }].into(),
        memory_accesses: Vec::new().into(),
        precompile_memory_accesses,
        precompile_result: Some(1),
    }
}

fn addi_report() -> GuestMachineReport {
    addi_report_at(0x8000_0000, 1, 0, 7, 7)
}

fn addi_report_at(address: u64, rd: u8, rs1: u8, immediate: i16, value: u64) -> GuestMachineReport {
    GuestMachineReport {
        address,
        instruction_byte_len: 4,
        instruction: RiscvInstruction::OpImm {
            kind: RiscvOpImmKind::Addi,
            rd,
            rs1,
            immediate: immediate.into(),
        },
        next_pc: address + 4,
        register_writes: vec![GuestRegisterWrite { index: rd, value }].into(),
        memory_accesses: Vec::new().into(),
        precompile_memory_accesses: Vec::new(),
        precompile_result: None,
    }
}

#[cfg(feature = "cuda")]
#[allow(clippy::too_many_arguments)]
fn zisk_main_descriptor_trace_values(
    pc: u64,
    jmp_offset1: i64,
    jmp_offset2: i64,
    a_prev_mem_step: u64,
    b_prev_mem_step: u64,
    store_prev_mem_step: u64,
    store_prev_value: u64,
    c: u64,
) -> ZiskMainReportTraceValues {
    ZiskMainReportTraceValues {
        instruction: ZiskMainInstruction {
            pc,
            a: ZiskMainSource::Immediate(0x11),
            b: ZiskMainSource::Register(2),
            op: ZiskMainOp::Add,
            store: ZiskMainStore::Register(3),
            store_pc: false,
            set_pc: false,
            jmp_offset1,
            jmp_offset2,
            ind_width: 0,
            m32: false,
            is_external_op: true,
            is_precompiled: false,
        },
        a: 0x22,
        b: 0x33,
        c,
        flag: true,
        register_accesses: ZiskMainRegisterAccessValues {
            a_prev_mem_step: Some(a_prev_mem_step),
            b_prev_mem_step: Some(b_prev_mem_step),
            store_prev_mem_step: Some(store_prev_mem_step),
            store_prev_value: Some(store_prev_value),
        },
    }
}

fn memory_read(address: u64, value: u64) -> GuestMemoryAccess {
    GuestMemoryAccess {
        kind: GuestMemoryAccessKind::Read,
        address,
        byte_len: 8,
        value,
    }
}

fn memory_write(address: u64, value: u64) -> GuestMemoryAccess {
    GuestMemoryAccess {
        kind: GuestMemoryAccessKind::Write,
        address,
        byte_len: 8,
        value,
    }
}

fn repo_temp_dir(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("temp")
        .join(format!("lzvm-prover-{}-{name}", std::process::id()))
}

const ENTRY: u64 = 0x8000_0000;

fn sample_guest_image_with_words(words: &[u32]) -> Vec<u8> {
    let mut code = Vec::with_capacity(words.len() * 4);
    for word in words {
        code.extend_from_slice(&word.to_le_bytes());
    }
    let header = program_header(120, code.len() as u64);
    let mut image = sample_guest_image_with_program_headers(&[header]);
    image.extend_from_slice(&code);
    image
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
    bytes[24..32].copy_from_slice(&ENTRY.to_le_bytes());
    bytes[32..40].copy_from_slice(&64_u64.to_le_bytes());
    bytes[52..54].copy_from_slice(&64_u16.to_le_bytes());
    bytes
}

fn sample_guest_image_with_program_headers(program_headers: &[[u8; 56]]) -> Vec<u8> {
    let mut bytes = sample_guest_image();
    bytes[32..40].copy_from_slice(&64_u64.to_le_bytes());
    bytes[54..56].copy_from_slice(&56_u16.to_le_bytes());
    bytes[56..58].copy_from_slice(&(program_headers.len() as u16).to_le_bytes());
    for header in program_headers {
        bytes.extend_from_slice(header);
    }
    bytes
}

fn program_header(file_offset: u64, file_size: u64) -> [u8; 56] {
    let mut bytes = [0_u8; 56];
    bytes[0..4].copy_from_slice(&1_u32.to_le_bytes());
    bytes[4..8].copy_from_slice(&5_u32.to_le_bytes());
    bytes[8..16].copy_from_slice(&file_offset.to_le_bytes());
    bytes[16..24].copy_from_slice(&ENTRY.to_le_bytes());
    bytes[24..32].copy_from_slice(&ENTRY.to_le_bytes());
    bytes[32..40].copy_from_slice(&file_size.to_le_bytes());
    bytes[40..48].copy_from_slice(&file_size.to_le_bytes());
    bytes[48..56].copy_from_slice(&0x1000_u64.to_le_bytes());
    bytes
}

fn riscv_addi(rd: u8, rs1: u8, immediate: i16) -> u32 {
    (((immediate as i32 as u32) & 0x0fff) << 20)
        | (u32::from(rs1) << 15)
        | (u32::from(rd) << 7)
        | 0x13
}

fn commitment_column(
    name: &str,
    stage: u32,
    stage_position: u32,
    dimension: u32,
) -> CommitmentColumn {
    CommitmentColumn {
        name: name.to_owned(),
        stage,
        dimension,
        pols_map_id: 0,
        stage_id: stage.saturating_sub(1),
        stage_position,
        intermediate: false,
        lengths: Vec::new(),
    }
}

fn sample_unit_with_zisk_main_columns_rows(row_count: u64) -> ProveUnitSchedule {
    let base_domain_size = row_count;
    let base_domain_bits = row_count.next_power_of_two().ilog2();
    let extended_domain_bits = base_domain_bits + 1;
    ProveUnitSchedule {
        kind: KeyUnitKind::Basic,
        group_id: Some(0),
        unit_id: Some(0),
        group_name: Some("group-a".to_owned()),
        unit_name: Some("unit-a".to_owned()),
        base_domain_bits,
        extended_domain_bits,
        base_domain_size,
        extended_domain_size: base_domain_size * 2,
        blowup_factor: 2,
        query_count: 1,
        proof_of_work_bits: 0,
        merkle_tree_arity: 4,
        last_level_verification: 0,
        transcript_arity: Some(4),
        hash_commits: true,
        transcript_root_challenge_draws: vec![1, 1],
        challenge_count: 1,
        evaluation_value_count: 0,
        evaluation_map: Vec::new(),
        transcript_evaluation_challenge_draws: 1,
        constant_width: 0,
        stage_commit_widths: vec![27],
        commitment_columns: vec![
            commitment_column("a", 1, 0, 2),
            commitment_column("b", 1, 2, 2),
            commitment_column("c", 1, 4, 2),
            commitment_column("flag", 1, 6, 1),
            commitment_column("pc", 1, 7, 1),
            commitment_column("a_src_imm", 1, 8, 1),
            commitment_column("b_src_imm", 1, 9, 1),
            commitment_column("a_src_reg", 1, 10, 1),
            commitment_column("b_src_reg", 1, 11, 1),
            commitment_column("store_reg", 1, 12, 1),
            commitment_column("store_pc", 1, 13, 1),
            commitment_column("set_pc", 1, 14, 1),
            commitment_column("op", 1, 15, 1),
            commitment_column("jmp_offset1", 1, 16, 1),
            commitment_column("jmp_offset2", 1, 17, 1),
            commitment_column("m32", 1, 18, 1),
            commitment_column("is_external_op", 1, 19, 1),
            commitment_column("is_precompiled", 1, 20, 1),
            commitment_column("b_src_ind", 1, 21, 1),
            commitment_column("ind_width", 1, 22, 1),
            commitment_column("store_ind", 1, 23, 1),
            commitment_column("store_offset", 1, 24, 1),
            commitment_column("store_mem", 1, 25, 1),
            commitment_column("b_offset_imm0", 1, 26, 1),
        ],
        unit_value_map: Vec::new(),
        group_value_map: Vec::new(),
        opening_points: vec![0],
        fri_layers: vec![PcsFriLayer {
            input_bits: extended_domain_bits,
            output_bits: 1,
            folding_factor: 4,
        }],
        final_layer_bits: 1,
        fixed_bytes: 0,
        constant_tree_root: None,
        pcs_material_bytes: None,
        pcs_material_plan_digest: None,
        pcs_material_fixed_column_digest: None,
        pcs_material_constant_tree_digest: None,
        pcs_material_constant_tree_root: None,
        pcs_material_fixed_byte_count: None,
        pcs_material_constant_tree_byte_count: None,
        pcs_material_leaf_byte_count: None,
        pcs_material_node_byte_count: None,
    }
}
