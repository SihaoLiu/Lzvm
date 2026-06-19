use super::*;
use crate::guest_instruction::{
    RiscvBranchKind, RiscvInstruction, RiscvOpImmKind, RiscvPrecompileKind, RiscvStoreKind,
};
use crate::witness_layout::derive_witness_trace_layout;
use crate::witness_loader::WitnessComputeContext;
use crate::witness_trace::parse_witness_trace;
use crate::ProveUnitSchedule;
use lzvm_artifacts::guest_image::parse_guest_image;
use lzvm_artifacts::key_directory::KeyUnitKind;
use lzvm_artifacts::pcs_plan::PcsFriLayer;
use lzvm_artifacts::setup_info::CommitmentColumn;
use lzvm_field::Felt;

static GUEST_PC_TRACE_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn internal_memory_tracks_only_supported_scratch_addresses() {
    let mut memory = ZiskMainInternalMemory::new();
    let amo_temp = zisk_internal_register_address_u64(ZISK_AMO_TEMP_REGISTER);

    assert_eq!(memory.get(ZISK_EXTRA_PARAMS_ADDRESS), None);
    assert_eq!(memory.get(amo_temp), None);

    memory
        .insert(ZISK_EXTRA_PARAMS_ADDRESS, 11)
        .expect("extra params scratch address should be supported");
    memory
        .insert(amo_temp, 22)
        .expect("AMO scratch address should be supported");

    assert_eq!(memory.get(ZISK_EXTRA_PARAMS_ADDRESS), Some(11));
    assert_eq!(memory.get(amo_temp), Some(22));
    assert_eq!(memory.get(0xdead_beef), None);
    assert!(
        memory.insert(0xdead_beef, 33).is_err(),
        "internal memory should not grow into a general map"
    );
}

struct TestEnvVarGuard {
    name: &'static str,
    previous: Option<std::ffi::OsString>,
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

#[test]
fn guest_trace_detail_timing_sample_stride_uses_positive_env_values() {
    let _env_lock = GUEST_PC_TRACE_ENV_LOCK
        .lock()
        .expect("guest PC trace env lock should not be poisoned");
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
fn trace_shape_run_counts_track_consecutive_row_classes() {
    let mut timing = GuestPcTraceStreamTiming::default();
    let external = zisk_main_base_instruction(
        0x8000_0000,
        ZiskMainSource::Immediate(1),
        ZiskMainSource::Immediate(2),
        ZiskMainOp::Add,
        ZiskMainStore::None,
        4,
    );
    let copy = zisk_main_base_instruction(
        0x8000_0004,
        ZiskMainSource::Immediate(0),
        ZiskMainSource::Immediate(3),
        ZiskMainOp::CopyB,
        ZiskMainStore::None,
        4,
    );
    let flag = zisk_main_base_instruction(
        0x8000_0008,
        ZiskMainSource::Immediate(0),
        ZiskMainSource::Immediate(0),
        ZiskMainOp::Flag,
        ZiskMainStore::None,
        4,
    );

    for instruction in [&external, &external, &copy, &copy, &copy, &flag, &external] {
        record_trace_lowered_row_shape(&mut timing, instruction);
    }

    assert_eq!(timing.trace_external_op_run_count(), 2);
    assert_eq!(timing.trace_external_op_max_run_count(), 2);
    assert_eq!(timing.trace_copy_run_count(), 1);
    assert_eq!(timing.trace_copy_max_run_count(), 3);
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
fn precompile_memory_validation_skips_empty_non_precompile_rows() {
    let report = addi_report();
    let instruction = lower_guest_report(&report).expect("report should lower");

    validate_zisk_main_precompile_memory_accesses_if_required(
        3,
        &report,
        &instruction,
        ZiskMainReportEffects::from_report(&report),
        0,
    )
    .expect("empty non-precompile rows should skip precompile memory validation");
}

#[test]
fn precompile_memory_validation_rejects_non_precompile_rows_with_accesses() {
    let mut report = addi_report();
    report.precompile_memory_accesses = vec![memory_read(64, 7)].into();
    let instruction = lower_guest_report(&report).expect("report should lower");

    let error = validate_zisk_main_precompile_memory_accesses_if_required(
        3,
        &report,
        &instruction,
        ZiskMainReportEffects::from_report(&report),
        0,
    )
    .expect_err("non-precompile rows with precompile accesses should fail");

    assert!(error.to_string().contains("non-precompile row reported"));
}

#[test]
fn precompile_memory_validation_rejects_precompile_rows_with_missing_accesses() {
    let mut report = add256_report();
    report.precompile_memory_accesses = Vec::new().into();
    let instruction = lower_guest_report(&report).expect("report should lower");

    let error = validate_zisk_main_precompile_memory_accesses_if_required(
        3,
        &report,
        &instruction,
        ZiskMainReportEffects::from_report(&report),
        64,
    )
    .expect_err("precompile rows with missing precompile accesses should fail");

    assert!(error
        .to_string()
        .contains("missing precompile memory access"));
}

#[test]
fn precompile_memory_validation_required_matches_row_shape() {
    let report = addi_report();
    let instruction = lower_guest_report(&report).expect("report should lower");
    assert!(!zisk_main_precompile_memory_validation_required(
        &instruction,
        ZiskMainReportEffects::from_report(&report),
    ));

    let mut non_precompile_with_access = addi_report();
    non_precompile_with_access.precompile_memory_accesses = vec![memory_read(64, 7)].into();
    let instruction = lower_guest_report(&non_precompile_with_access).expect("report should lower");
    assert!(zisk_main_precompile_memory_validation_required(
        &instruction,
        ZiskMainReportEffects::from_report(&non_precompile_with_access),
    ));

    let mut precompile_without_access = add256_report();
    precompile_without_access.precompile_memory_accesses = Vec::new().into();
    let instruction = lower_guest_report(&precompile_without_access).expect("report should lower");
    assert!(zisk_main_precompile_memory_validation_required(
        &instruction,
        ZiskMainReportEffects::from_report(&precompile_without_access),
    ));
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
fn zisk_main_streaming_unit_value_summary_matches_batch_reports() {
    let unit = sample_unit_with_zisk_main_columns_rows(2);
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");
    let reports = [
        addi_report_at(0x8000_0000, 1, 0, 7, 7),
        addi_report_at(0x8000_0004, 2, 1, 3, 10),
    ];
    let segment = ZiskMainTraceSegmentInfo {
        trace_instance_index: 0,
        is_last_segment: false,
        previous_c: 0,
    };
    let written = build_layout_zisk_main_trace_segment(
        &layout,
        &reports,
        0x8000_0008,
        &ZiskMainTraceState::new(),
        None,
        segment,
        None,
    )
    .expect("segment should build")
    .expect("layout should be supported");
    let mut summary = ZiskMainSegmentUnitValueSummary::new();
    for report in &reports {
        summary.push_report(report);
    }

    assert_eq!(
        summary.unit_values(
            layout.row_count(),
            written.trace_source_prefix_rows,
            0x8000_0008,
            &written.final_state,
            segment,
        ),
        written.output.unit_values
    );
}

#[test]
fn direct_boundary_c_uses_register_store_write_value() {
    let report = addi_report_at(0x8000_0000, 3, 0, 11, 11);
    let instruction = lower_guest_report(&report).expect("report should lower");

    assert_eq!(
        direct_zisk_main_report_boundary_c(&report, &instruction),
        Some(11)
    );
}

#[test]
fn direct_boundary_c_does_not_confuse_store_pc_write_with_c() {
    let report = GuestMachineReport {
        address: 0x8000_0000,
        instruction_byte_len: 4,
        instruction: RiscvInstruction::Jal { rd: 1, offset: 16 },
        next_pc: 0x8000_0010,
        register_writes: vec![GuestRegisterWrite {
            index: 1,
            value: 0x8000_0004,
        }]
        .into(),
        memory_accesses: Vec::new().into(),
        precompile_memory_accesses: Vec::new().into(),
        precompile_result: None,
    };
    let instruction = lower_guest_report(&report).expect("report should lower");
    assert!(instruction.store_pc);

    assert_ne!(
        direct_zisk_main_report_boundary_c(&report, &instruction),
        Some(0x8000_0004)
    );
    assert_eq!(
        direct_zisk_main_report_boundary_c(&report, &instruction),
        Some(0)
    );
}

#[test]
fn direct_boundary_c_uses_full_width_memory_store_value() {
    let report = GuestMachineReport {
        address: 0x8000_0000,
        instruction_byte_len: 4,
        instruction: RiscvInstruction::Store {
            kind: RiscvStoreKind::Sd,
            rs1: 1,
            rs2: 2,
            offset: 8,
        },
        next_pc: 0x8000_0004,
        register_writes: Vec::new().into(),
        memory_accesses: vec![GuestMemoryAccess {
            kind: GuestMemoryAccessKind::Write,
            address: 0x1008,
            byte_len: 8,
            value: 0x1234_5678_9abc_def0,
        }]
        .into(),
        precompile_memory_accesses: Vec::new().into(),
        precompile_result: None,
    };
    let instruction = lower_guest_report(&report).expect("report should lower");
    assert!(matches!(instruction.store, ZiskMainStore::Indirect(8)));

    assert_eq!(
        direct_zisk_main_report_boundary_c(&report, &instruction),
        Some(0x1234_5678_9abc_def0)
    );
}

#[test]
fn direct_boundary_c_uses_branch_next_pc_outcome() {
    let taken = GuestMachineReport {
        address: 0x8000_0000,
        instruction_byte_len: 4,
        instruction: RiscvInstruction::Branch {
            kind: RiscvBranchKind::Beq,
            rs1: 1,
            rs2: 2,
            offset: 16,
        },
        next_pc: 0x8000_0010,
        register_writes: Vec::new().into(),
        memory_accesses: Vec::new().into(),
        precompile_memory_accesses: Vec::new().into(),
        precompile_result: None,
    };
    let taken_instruction = lower_guest_report(&taken).expect("branch should lower");
    assert_eq!(
        direct_zisk_main_report_boundary_c(&taken, &taken_instruction),
        Some(1)
    );

    let not_taken = GuestMachineReport {
        next_pc: 0x8000_0004,
        ..taken
    };
    let not_taken_instruction = lower_guest_report(&not_taken).expect("branch should lower");
    assert_eq!(
        direct_zisk_main_report_boundary_c(&not_taken, &not_taken_instruction),
        Some(0)
    );
}

#[test]
fn guest_pc_trace_seed_mirror_attaches_pending_segment_seeds_when_enabled() {
    let _env_lock = GUEST_PC_TRACE_ENV_LOCK
        .lock()
        .expect("guest PC trace env lock should not be poisoned");
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

    let produced = produce_guest_pc_trace_pending_slices(
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
    assert!(pending
        .iter()
        .all(|segment| segment.replay_snapshot.is_none()));
    assert_eq!(produced.timing.seed_direct_lift_attempt_count(), 0);
    assert_eq!(produced.timing.seed_direct_lift_success_count(), 0);
    assert_eq!(produced.timing.seed_full_advance_count(), pending.len());
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
    let _env_lock = GUEST_PC_TRACE_ENV_LOCK
        .lock()
        .expect("guest PC trace env lock should not be poisoned");
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
    let _replay_env = EnvGuard::new("LZVM_GUEST_PC_TRACE_SEGMENT_REPLAY_SNAPSHOT", "1");
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
    assert!(pending
        .iter()
        .all(|segment| segment.replay_snapshot.is_some()));
    for segment in &pending {
        let replay = replay_guest_pc_trace_segment_from_snapshot(
            segment
                .replay_snapshot
                .clone()
                .expect("pending segment should carry replay snapshot"),
            16,
            layout.row_count(),
        )
        .expect("pending segment snapshot should replay");
        assert_eq!(replay.slice.reports, segment.reports);
    }
}

#[test]
fn guest_pc_trace_trusted_runner_seed_snapshot_produces_pending_seeds() {
    let _env_lock = GUEST_PC_TRACE_ENV_LOCK
        .lock()
        .expect("guest PC trace env lock should not be poisoned");
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

    let _snapshot_env = EnvGuard::new("LZVM_GUEST_PC_TRACE_RUNNER_SEED_SNAPSHOT", "1");
    let _trusted_env = EnvGuard::new("LZVM_GUEST_PC_TRACE_RUNNER_SEED_SNAPSHOT_TRUSTED", "1");
    let dir = repo_temp_dir("guest-pc-trusted-runner-seed-snapshot");
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

    let produced = produce_guest_pc_trace_pending_slices(
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
    .expect("trusted runner seed snapshot should produce pending slices");
    std::fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(pending.len(), 2);
    assert!(pending.iter().all(|segment| segment.seed.is_some()));
    assert_eq!(pending[1].seed.as_ref().unwrap().previous_c, 10);
    assert_eq!(produced.timing.seed_direct_lift_attempt_count(), 1);
    assert_eq!(produced.timing.seed_direct_lift_success_count(), 1);
    assert_eq!(produced.timing.seed_full_advance_count(), pending.len());
}

#[test]
fn seeded_pending_segment_lowers_without_prior_segment_state() {
    let _env_lock = GUEST_PC_TRACE_ENV_LOCK
        .lock()
        .expect("guest PC trace env lock should not be poisoned");
    let _mirror_env = TestEnvVarGuard::set("LZVM_GUEST_PC_TRACE_SEED_MIRROR", "1");
    let dir = repo_temp_dir("guest-pc-seeded-pending-lower");
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
    let second = &pending[1];
    let second_seed = second
        .seed
        .as_deref()
        .expect("second segment should carry its own seed");
    let lowered =
        lower_guest_pc_trace_seeded_pending_segment(&layout, second, second_seed, None, None)
            .expect("seeded segment should lower independently");
    let trace = lowered
        .segment
        .trace
        .as_ref()
        .expect("host trace should be built");
    let pc_column = layout.column(1, "pc").expect("pc column").trace_column();

    assert_eq!(lowered.segment.trace_instance_index, 1);
    assert_eq!(
        trace.value(0, pc_column),
        Some(Felt::from_canonical(0x8000_0008).expect("canonical pc"))
    );
    assert_eq!(lowered.next_seed.previous_c, 15);
}

#[test]
fn seeded_pending_segments_parallel_lower_matches_serial_output() {
    let _env_lock = GUEST_PC_TRACE_ENV_LOCK
        .lock()
        .expect("guest PC trace env lock should not be poisoned");
    let _mirror_env = TestEnvVarGuard::set("LZVM_GUEST_PC_TRACE_SEED_MIRROR", "1");
    let dir = repo_temp_dir("guest-pc-parallel-seeded-lower");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let guest_image_bytes = sample_guest_image_with_words(&[
        riscv_addi(1, 0, 7),
        riscv_addi(2, 1, 3),
        riscv_addi(3, 2, 5),
        riscv_addi(4, 3, 11),
        riscv_addi(5, 4, 13),
        0x0000_0073,
    ]);
    std::fs::write(&guest_image, &guest_image_bytes).expect("guest image should be written");
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    let unit = sample_unit_with_zisk_main_columns_rows(2);
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");
    let mut pending = Vec::new();

    produce_guest_pc_trace_pending_slices(
        32,
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

    assert!(pending.len() >= 3);
    assert!(pending.iter().all(|segment| segment.seed.is_some()));

    let mut serial = Vec::new();
    for segment in &pending {
        let seed = segment
            .seed
            .as_deref()
            .expect("seeded pending segment should carry its own seed");
        serial.push(
            lower_guest_pc_trace_seeded_pending_segment(&layout, segment, seed, None, None)
                .expect("serial seeded segment should lower"),
        );
    }

    let parallel =
        lower_guest_pc_trace_seeded_pending_segments_with_workers(&layout, pending, None, 2)
            .expect("parallel seeded segments should lower");

    assert_eq!(parallel.len(), serial.len());
    for (parallel, serial) in parallel.iter().zip(serial.iter()) {
        assert_eq!(parallel.next_seed, serial.next_seed);
        assert_eq!(
            parallel.segment.trace_instance_index,
            serial.segment.trace_instance_index
        );
        assert_eq!(
            parallel.segment.trace_source_prefix_rows,
            serial.segment.trace_source_prefix_rows
        );
        #[cfg(feature = "cuda")]
        assert_eq!(
            parallel.segment.device_segment_material,
            serial.segment.device_segment_material
        );
        assert_eq!(parallel.segment.trace, serial.segment.trace);
        assert_eq!(parallel.segment.unit_values, serial.segment.unit_values);
        assert_eq!(parallel.segment.proof_values, serial.segment.proof_values);
    }
}

#[test]
fn guest_pc_trace_segment_replay_snapshot_matches_serial_slice() {
    let _env_lock = GUEST_PC_TRACE_ENV_LOCK
        .lock()
        .expect("guest PC trace env lock should not be poisoned");
    let _replay_env = TestEnvVarGuard::unset("LZVM_GUEST_PC_TRACE_SEGMENT_REPLAY");
    assert!(
        !guest_pc_trace_segment_replay_enabled(),
        "segment replay should stay disabled by default"
    );
    std::env::set_var("LZVM_GUEST_PC_TRACE_SEGMENT_REPLAY", "1");
    assert!(
        guest_pc_trace_segment_replay_enabled(),
        "segment replay should have an explicit opt-in gate"
    );

    let dir = repo_temp_dir("guest-pc-segment-replay-snapshot");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let guest_image_bytes = sample_guest_image_with_words(&[
        riscv_addi(1, 0, 7),
        riscv_addi(2, 1, 3),
        riscv_addi(3, 2, 5),
        riscv_addi(4, 3, 11),
        riscv_addi(5, 4, 13),
        0x0000_0073,
    ]);
    std::fs::write(&guest_image, &guest_image_bytes).expect("guest image should be written");
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    let row_count = 2;
    let context = WitnessComputeContext {
        guest_image: Some(&guest_image),
        guest_image_info: Some(&guest_image_info),
        trace_layout: None,
    };
    let (mut memory, mut state, mut fcall_handler) =
        load_guest_pc_trace_machine(context, &[]).expect("guest trace machine should load");

    let first = run_guest_pc_trace_segment_slice(
        &mut memory,
        &mut state,
        &mut fcall_handler,
        32,
        row_count,
    )
    .expect("first segment should run");
    assert_eq!(first.trace_rows, row_count);

    let replay_snapshot =
        GuestPcTraceSegmentReplaySnapshot::capture(&memory, &state, &fcall_handler);
    let serial = run_guest_pc_trace_segment_slice(
        &mut memory,
        &mut state,
        &mut fcall_handler,
        32_u64.saturating_sub(first.executed_instructions),
        row_count,
    )
    .expect("serial segment should run");
    let replay = replay_guest_pc_trace_segment_from_snapshot(
        replay_snapshot,
        32_u64.saturating_sub(first.executed_instructions),
        row_count,
    )
    .expect("snapshot replay should run");
    std::fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(
        replay.slice.executed_instructions,
        serial.executed_instructions
    );
    assert_eq!(replay.slice.trace_rows, serial.trace_rows);
    assert_eq!(replay.slice.status, serial.status);
    assert_eq!(replay.slice.reports, serial.reports);
    assert_eq!(replay.memory, memory);
    assert_eq!(replay.state, state);
    assert!(replay.fcall_handler.equals_any(&fcall_handler));
}

#[test]
fn parallel_lower_env_stream_matches_serial_segments() {
    let _env_lock = GUEST_PC_TRACE_ENV_LOCK
        .lock()
        .expect("guest PC trace env lock should not be poisoned");
    let _mirror_env = TestEnvVarGuard::unset("LZVM_GUEST_PC_TRACE_SEED_MIRROR");
    let _snapshot_env = TestEnvVarGuard::unset("LZVM_GUEST_PC_TRACE_RUNNER_SEED_SNAPSHOT");
    let _trusted_env = TestEnvVarGuard::unset("LZVM_GUEST_PC_TRACE_RUNNER_SEED_SNAPSHOT_TRUSTED");
    let _parallel_env = TestEnvVarGuard::set("LZVM_GUEST_PC_TRACE_PARALLEL_LOWER", "1");
    let _worker_env = TestEnvVarGuard::set("LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_WORKERS", "2");
    let _replay_env = TestEnvVarGuard::set("LZVM_GUEST_PC_TRACE_SEGMENT_REPLAY", "1");
    let dir = repo_temp_dir("guest-pc-parallel-lower-stream");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let guest_image_bytes = sample_guest_image_with_words(&[
        riscv_addi(1, 0, 7),
        riscv_addi(2, 1, 3),
        riscv_addi(3, 2, 5),
        riscv_addi(4, 3, 11),
        riscv_addi(5, 4, 13),
        0x0000_0073,
    ]);
    std::fs::write(&guest_image, &guest_image_bytes).expect("guest image should be written");
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    let unit = sample_unit_with_zisk_main_columns_rows(2);
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");
    let context = WitnessComputeContext {
        guest_image: Some(&guest_image),
        guest_image_info: Some(&guest_image_info),
        trace_layout: Some(&layout),
    };

    let serial = compute_guest_pc_trace_segments(32, context, &[])
        .expect("serial guest PC trace should compute");
    let mut parallel = Vec::new();
    let stream = produce_guest_pc_trace_segments(32, context, &[], None, |segment| {
        parallel.push(segment);
        Ok(())
    })
    .expect("parallel guest PC trace stream should produce");
    std::fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert!(serial.len() >= 3);
    assert_eq!(parallel.len(), serial.len());
    assert_eq!(stream.proof_values, serial[0].proof_values);
    assert_eq!(stream.timing.parallel_lower_worker_count(), 2);
    assert_eq!(
        stream.timing.parallel_lower_dispatched_count(),
        serial.len()
    );
    assert_eq!(stream.timing.segment_replay_count(), serial.len());
    assert_eq!(stream.timing.parallel_lower_received_count(), serial.len());
    assert_eq!(stream.timing.parallel_lower_emitted_count(), serial.len());
    assert!(stream.timing.parallel_lower_max_reorder_count() <= serial.len());
    for (parallel, serial) in parallel.iter().zip(serial.iter()) {
        assert_eq!(parallel.trace_instance_index, serial.trace_instance_index);
        assert_eq!(
            parallel.trace_source_prefix_rows,
            serial.trace_source_prefix_rows
        );
        #[cfg(feature = "cuda")]
        assert_eq!(
            parallel.device_segment_material,
            serial.device_segment_material
        );
        assert_eq!(parallel.trace, serial.trace);
        assert_eq!(parallel.unit_values, serial.unit_values);
    }
}

#[test]
fn guest_pc_trace_default_runner_seed_snapshot_stays_disabled() {
    let _env_lock = GUEST_PC_TRACE_ENV_LOCK
        .lock()
        .expect("guest PC trace env lock should not be poisoned");
    let _mirror_env = TestEnvVarGuard::unset("LZVM_GUEST_PC_TRACE_SEED_MIRROR");
    let _snapshot_env = TestEnvVarGuard::unset("LZVM_GUEST_PC_TRACE_RUNNER_SEED_SNAPSHOT");
    let _trusted_env = TestEnvVarGuard::unset("LZVM_GUEST_PC_TRACE_RUNNER_SEED_SNAPSHOT_TRUSTED");
    let _parallel_env = TestEnvVarGuard::unset("LZVM_GUEST_PC_TRACE_PARALLEL_LOWER");
    let dir = repo_temp_dir("guest-pc-default-runner-seed-snapshot");
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
    .expect("default runner seed snapshot should stay optional");
    std::fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(pending.len(), 2);
    assert!(pending.iter().all(|segment| segment.seed.is_none()));
}

#[test]
fn parallel_lower_implies_trusted_runner_seed_snapshot() {
    let _env_lock = GUEST_PC_TRACE_ENV_LOCK
        .lock()
        .expect("guest PC trace env lock should not be poisoned");
    let _trusted_env = TestEnvVarGuard::unset("LZVM_GUEST_PC_TRACE_RUNNER_SEED_SNAPSHOT_TRUSTED");
    let _parallel_env = TestEnvVarGuard::set("LZVM_GUEST_PC_TRACE_PARALLEL_LOWER", "1");

    assert!(guest_pc_trace_runner_seed_snapshot_trusted_enabled());
}

#[test]
fn parallel_lower_job_dispatch_skips_full_worker_queue() {
    let (first_sender, first_receiver) = std::sync::mpsc::sync_channel(1);
    let (second_sender, second_receiver) = std::sync::mpsc::sync_channel(1);
    first_sender
        .send(10_u32)
        .expect("first worker queue should accept setup job");
    let mut next_worker = 0_usize;

    dispatch_guest_pc_trace_parallel_lower_job(
        &[first_sender, second_sender],
        &mut next_worker,
        20_u32,
    )
    .expect("dispatcher should skip a full worker queue");

    assert_eq!(first_receiver.try_recv(), Ok(10));
    assert_eq!(second_receiver.try_recv(), Ok(20));
    assert_eq!(next_worker, 0);
}

#[test]
fn trusted_runner_seed_snapshot_forces_reference_seed_when_validation_enabled() {
    assert!(guest_pc_trace_needs_full_seed_advance(
        true, true, true, false, true
    ));
    assert!(!guest_pc_trace_needs_full_seed_advance(
        true, true, false, false, true
    ));
    assert!(guest_pc_trace_needs_full_seed_advance(
        true, true, false, true, true
    ));
    assert!(guest_pc_trace_needs_full_seed_advance(
        true, true, false, false, false
    ));
    assert!(guest_pc_trace_needs_full_seed_advance(
        true, false, false, false, true
    ));
    assert!(!guest_pc_trace_needs_full_seed_advance(
        false, true, true, false, true
    ));
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
        precompile_memory_accesses: Vec::new().into(),
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
            .get(ZISK_EXTRA_PARAMS_ADDRESS),
        Some(0x20)
    );
}

#[test]
fn runner_boundary_snapshot_records_dma_prepare_scratch_incrementally() {
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
        precompile_memory_accesses: Vec::new().into(),
        precompile_result: None,
    };

    let mut snapshot = ZiskMainRunnerBoundarySnapshot::new(&current_seed);
    snapshot
        .record_report(
            &report,
            Some(RiscvInstruction::Op {
                kind: RiscvOpKind::Add,
                rd: 7,
                rs1: 8,
                rs2: 6,
            }),
            &current_seed.initial_state.registers,
        )
        .expect("boundary snapshot should record DMA scratch");

    assert_eq!(
        snapshot.internal_memory.get(ZISK_EXTRA_PARAMS_ADDRESS),
        Some(0x20)
    );
}

#[test]
fn runner_boundary_seed_snapshot_rejects_direct_previous_c_mismatch() {
    let current_seed = ZiskMainSegmentSeed::new();
    let report = addi_report_at(0x8000_0000, 3, 0, 11, 11);
    let mut runner_state = GuestMachineState::new(report.next_pc);
    runner_state
        .set_register(3, 11)
        .expect("destination register should set");
    let segment = ZiskMainTraceSegmentInfo {
        trace_instance_index: 0,
        is_last_segment: false,
        previous_c: 0,
    };

    let error = lift_zisk_main_next_segment_seed_from_runner_boundary(
        1,
        segment,
        std::slice::from_ref(&report),
        None,
        &runner_state,
        &current_seed,
        12,
    )
    .expect_err("directly derivable boundary c mismatch should reject");

    assert!(error.to_string().contains("direct boundary c"));
}

#[test]
fn runner_boundary_seed_snapshot_derives_full_width_store_boundary() {
    let mut current_seed = ZiskMainSegmentSeed::new();
    current_seed.initial_state.registers[1] = 0x1000;
    current_seed.initial_state.registers[2] = 0xfeed_face_cafe_babe;
    let report = GuestMachineReport {
        address: 0x8000_0000,
        instruction_byte_len: 4,
        instruction: RiscvInstruction::Store {
            kind: RiscvStoreKind::Sd,
            rs1: 1,
            rs2: 2,
            offset: 8,
        },
        next_pc: 0x8000_0004,
        register_writes: Vec::new().into(),
        memory_accesses: vec![GuestMemoryAccess {
            kind: GuestMemoryAccessKind::Write,
            address: 0x1008,
            byte_len: 8,
            value: 0xfeed_face_cafe_babe,
        }]
        .into(),
        precompile_memory_accesses: Vec::new().into(),
        precompile_result: None,
    };
    let mut runner_state = GuestMachineState::new(report.next_pc);
    runner_state
        .set_register(1, 0x1000)
        .expect("base register should set");
    runner_state
        .set_register(2, 0xfeed_face_cafe_babe)
        .expect("source register should set");
    let segment = ZiskMainTraceSegmentInfo {
        trace_instance_index: 0,
        is_last_segment: false,
        previous_c: 0,
    };

    let lifted = try_lift_zisk_main_next_segment_seed_from_runner_boundary(
        2,
        segment,
        std::slice::from_ref(&report),
        None,
        &runner_state,
        &current_seed,
    )
    .expect("runner boundary seed should lift")
    .expect("full-width store boundary should be direct");

    assert_eq!(lifted.previous_c, 0xfeed_face_cafe_babe);
    assert_eq!(lifted.initial_state.last_c, 0xfeed_face_cafe_babe);
    assert_eq!(lifted.initial_state.next_pc, report.next_pc);
    assert_eq!(lifted.initial_state.registers[1], 0x1000);
    assert_eq!(lifted.initial_state.registers[2], 0xfeed_face_cafe_babe);
}

#[test]
fn runner_boundary_seed_snapshot_derives_narrow_store_boundary_from_runner_register() {
    let mut current_seed = ZiskMainSegmentSeed::new();
    current_seed.initial_state.registers[10] = 0x1000;
    current_seed.initial_state.registers[12] = 0x1234_5678_9abc_def0;
    let report = GuestMachineReport {
        address: 0x8000_0000,
        instruction_byte_len: 4,
        instruction: RiscvInstruction::Store {
            kind: RiscvStoreKind::Sb,
            rs1: 10,
            rs2: 12,
            offset: 17,
        },
        next_pc: 0x8000_0004,
        register_writes: Vec::new().into(),
        memory_accesses: vec![GuestMemoryAccess {
            kind: GuestMemoryAccessKind::Write,
            address: 0x1011,
            byte_len: 1,
            value: 0xf0,
        }]
        .into(),
        precompile_memory_accesses: Vec::new().into(),
        precompile_result: None,
    };
    let mut runner_state = GuestMachineState::new(report.next_pc);
    runner_state
        .set_register(10, 0x1000)
        .expect("base register should set");
    runner_state
        .set_register(12, 0x1234_5678_9abc_def0)
        .expect("source register should set");
    let segment = ZiskMainTraceSegmentInfo {
        trace_instance_index: 0,
        is_last_segment: false,
        previous_c: 0,
    };

    let lifted = try_lift_zisk_main_next_segment_seed_from_runner_boundary(
        1,
        segment,
        std::slice::from_ref(&report),
        None,
        &runner_state,
        &current_seed,
    )
    .expect("runner boundary seed lift should evaluate")
    .expect("narrow store boundary c should come from the runner source register");

    assert_eq!(lifted.previous_c, 0x1234_5678_9abc_def0);
    assert_eq!(lifted.initial_state.last_c, 0x1234_5678_9abc_def0);
    assert_eq!(lifted.initial_state.next_pc, report.next_pc);
    assert_eq!(lifted.initial_state.registers[10], 0x1000);
    assert_eq!(lifted.initial_state.registers[12], 0x1234_5678_9abc_def0);
}

#[test]
fn runner_boundary_seed_snapshot_uses_runner_registers_for_narrow_store() {
    let current_seed = ZiskMainSegmentSeed::new();
    let report = GuestMachineReport {
        address: 0x8000_0000,
        instruction_byte_len: 4,
        instruction: RiscvInstruction::Store {
            kind: RiscvStoreKind::Sb,
            rs1: 10,
            rs2: 12,
            offset: 17,
        },
        next_pc: 0x8000_0004,
        register_writes: Vec::new().into(),
        memory_accesses: vec![GuestMemoryAccess {
            kind: GuestMemoryAccessKind::Write,
            address: 0x1011,
            byte_len: 1,
            value: 0xf0,
        }]
        .into(),
        precompile_memory_accesses: Vec::new().into(),
        precompile_result: None,
    };
    let mut runner_state = GuestMachineState::new(report.next_pc);
    runner_state
        .set_register(10, 0x1000)
        .expect("base register should set");
    runner_state
        .set_register(12, 0x1234_5678_9abc_def0)
        .expect("source register should set");
    let boundary_snapshot = ZiskMainRunnerBoundarySnapshot::new(&current_seed);
    let segment = ZiskMainTraceSegmentInfo {
        trace_instance_index: 0,
        is_last_segment: false,
        previous_c: 0,
    };

    let lifted = lift_zisk_main_next_segment_seed_from_runner_boundary_snapshot(
        1,
        segment,
        ZiskMainRunnerBoundarySeedInput {
            reports: std::slice::from_ref(&report),
            lookahead_instruction: None,
            runner_state: &runner_state,
            current_seed: &current_seed,
            boundary_snapshot: &boundary_snapshot,
        },
        0x1234_5678_9abc_def0,
    )
    .expect("narrow store boundary c should come from runner state registers");

    assert_eq!(lifted.previous_c, 0x1234_5678_9abc_def0);
    assert_eq!(lifted.initial_state.last_c, 0x1234_5678_9abc_def0);
    assert_eq!(lifted.initial_state.registers[12], 0x1234_5678_9abc_def0);
}

#[test]
fn direct_seed_lift_classifies_empty_segment_miss() {
    let miss = direct_zisk_main_segment_boundary_c(&[], None, &ZiskMainSegmentSeed::new(), None)
        .expect("direct boundary classification should evaluate")
        .expect_err("empty segments cannot expose a direct boundary c");

    assert_eq!(miss, ZiskMainDirectSeedLiftMissReason::EmptySegment);
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
fn zisk_main_device_descriptor_counts_unpaired_high_words() {
    let mut descriptors = ZiskMainDeviceTraceDescriptors::new(3, 39, 0x2000);
    let low_values = zisk_main_descriptor_trace_values(0x1000, 5, 6, 21, 22, 23, 24, 7);
    append_main_device_trace_descriptor(&mut descriptors, &low_values)
        .expect("low descriptor row should append");

    let mut high_values =
        zisk_main_descriptor_trace_values(0x1004, 5, 6, 21, 22, 23, 0x1_0000_0001, 0x2_0000_0002);
    high_values.a = 0x3_0000_0003;
    high_values.instruction.a = ZiskMainSource::Immediate(0x4_0000_0004);
    high_values.instruction.store = ZiskMainStore::Memory(0x5_0000_0005);
    append_main_device_trace_descriptor(&mut descriptors, &high_values)
        .expect("high descriptor row should append");

    assert_eq!(descriptors.unpaired_value_count(), 14);
    assert_eq!(descriptors.unpaired_high32_nonzero_count(), 5);
    assert_eq!(descriptors.unpaired_high32_nonzero_row_count(), 1);
    assert_eq!(
        descriptors.unpaired_high32_nonzero_field_counts(),
        [1, 0, 1, 1, 0, 1, 1]
    );
    assert_eq!(
        descriptors.unpaired_high32_nonzero_row_field_histogram(),
        [1, 0, 0, 0, 0, 1, 0, 0]
    );
}

#[test]
#[cfg(feature = "cuda")]
fn zisk_main_device_descriptor_uses_sparse_high_words() {
    let mut descriptors = ZiskMainDeviceTraceDescriptors::new_with_descriptor_words_and_stats(
        3,
        39,
        0x2000,
        ZISK_MAIN_DEVICE_TRACE_SPARSE_DESCRIPTOR_WORDS,
        true,
    );
    let low_values = zisk_main_descriptor_trace_values(0x1000, 5, 6, 21, 22, 23, 24, 7);
    append_main_device_trace_descriptor(&mut descriptors, &low_values)
        .expect("low sparse descriptor row should append");

    let mut high_values =
        zisk_main_descriptor_trace_values(0x1004, 5, 6, 21, 22, 23, 0x1_0000_0001, 0x2_0000_0002);
    high_values.a = 0x3_0000_0003;
    high_values.instruction.a = ZiskMainSource::Immediate(0x4_0000_0004);
    high_values.instruction.store = ZiskMainStore::Memory(0x5_0000_0005);
    append_main_device_trace_descriptor(&mut descriptors, &high_values)
        .expect("high sparse descriptor row should append");

    assert_eq!(descriptors.descriptor_word_count(), 9);
    assert_eq!(descriptors.words().len(), 18);
    assert_eq!(descriptors.sparse_high_words().len(), 3);
    assert_eq!(descriptors.words()[7] >> 32, 0);
    assert_eq!(descriptors.words()[9 + 7] >> 32, 0x6d);
    assert_eq!(
        descriptors.sparse_high_words(),
        &[0x3 | (0x2_u64 << 32), 0x4 | (0x5_u64 << 32), 0x1,]
    );
}

#[test]
#[cfg(feature = "cuda")]
fn zisk_main_sparse_descriptor_words_are_stack_packed() {
    fn assert_copy<T: Copy>(_: T) {}

    let mut values =
        zisk_main_descriptor_trace_values(0x1004, 5, 6, 21, 22, 23, 0x1_0000_0001, 0x2_0000_0002);
    values.a = 0x3_0000_0003;
    values.instruction.a = ZiskMainSource::Immediate(0x4_0000_0004);
    values.instruction.store = ZiskMainStore::Memory(0x5_0000_0005);
    let instruction = &values.instruction;
    let (_, a_payload) = zisk_main_device_trace_source_descriptor(instruction.a);
    let (_, b_payload) = zisk_main_device_trace_source_descriptor(instruction.b);
    let (_, store_payload) = zisk_main_device_trace_store_descriptor(&instruction.store);
    let control = 0x1234;

    let sparse = zisk_main_sparse_device_trace_descriptor_words(
        &values,
        a_payload,
        b_payload,
        store_payload,
        control,
        21,
        22,
        23,
        0x1_0000_0001,
        17,
    )
    .expect("sparse descriptor should pack on stack");

    assert_copy(sparse);
    assert_eq!(sparse.high_word_count, 3);
    assert_eq!(
        &sparse.high_words[..sparse.high_word_count],
        &[0x3 | (0x2_u64 << 32), 0x4 | (0x5_u64 << 32), 0x1,]
    );
    assert_eq!(sparse.words[7] >> 32, 0x6d);
    assert_eq!(sparse.words[8], 17);
}

#[test]
#[cfg(feature = "cuda")]
fn zisk_main_device_sparse_descriptor_falls_back_to_wide_words_when_values_do_not_fit() {
    let mut descriptors = ZiskMainDeviceTraceDescriptors::new_with_descriptor_words_and_stats(
        2,
        39,
        0x2000,
        ZISK_MAIN_DEVICE_TRACE_SPARSE_DESCRIPTOR_WORDS,
        true,
    );
    let mut first =
        zisk_main_descriptor_trace_values(0x1000, 5, 6, 21, 22, 23, 0x1_0000_0001, 0x2_0000_0002);
    first.a = 0x3_0000_0003;
    first.instruction.a = ZiskMainSource::Immediate(0x4_0000_0004);
    first.instruction.store = ZiskMainStore::Memory(0x5_0000_0005);
    append_main_device_trace_descriptor(&mut descriptors, &first)
        .expect("first sparse descriptor row should append");

    let second =
        zisk_main_descriptor_trace_values(0x1004, i64::from(i32::MAX) + 1, 6, 21, 22, 23, 24, 7);
    append_main_device_trace_descriptor(&mut descriptors, &second)
        .expect("wide fallback descriptor row should append");

    assert_eq!(descriptors.descriptor_word_count(), 14);
    assert!(descriptors.sparse_high_words().is_empty());
    assert_eq!(descriptors.words().len(), 28);
    assert_eq!(descriptors.words()[0], 0x3_0000_0003);
    assert_eq!(descriptors.words()[2], 0x2_0000_0002);
    assert_eq!(descriptors.words()[4], 0x4_0000_0004);
    assert_eq!(descriptors.words()[6], 0x5_0000_0005);
    assert_eq!(descriptors.words()[13], 0x1_0000_0001);
    assert_eq!(
        descriptors.words()[14 + 8],
        (i64::from(i32::MAX) + 1) as u64
    );
}

#[test]
#[cfg(feature = "cuda")]
fn streaming_device_segment_builder_matches_host_trace_device_material() {
    let _env_lock = GUEST_PC_TRACE_ENV_LOCK
        .lock()
        .expect("guest PC trace env lock should not be poisoned");
    let _mirror_env = TestEnvVarGuard::set("LZVM_GUEST_PC_TRACE_SEED_MIRROR", "1");
    let dir = repo_temp_dir("guest-pc-streaming-device-builder");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let guest_image_bytes = sample_guest_image_with_words(&[
        riscv_addi(1, 0, 7),
        riscv_addi(2, 1, 3),
        riscv_addi(3, 2, 5),
        riscv_addi(4, 3, 11),
        0x0000_0073,
    ]);
    std::fs::write(&guest_image, &guest_image_bytes).expect("guest image should be written");
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    let unit = sample_unit_with_zisk_main_device_columns_rows(2);
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");
    let mut pending = Vec::new();

    produce_guest_pc_trace_pending_slices(
        32,
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

    let segment = pending
        .iter()
        .find(|segment| !segment.is_last_segment)
        .expect("fixture should include a full non-final segment");
    let seed = segment
        .seed
        .as_deref()
        .expect("seed mirror should attach segment seed");
    let segment_info = ZiskMainTraceSegmentInfo {
        trace_instance_index: segment.trace_instance_index,
        is_last_segment: segment.is_last_segment,
        previous_c: seed.previous_c,
    };
    let host = build_layout_zisk_main_trace_segment(
        &layout,
        &segment.reports,
        segment.terminal_pc,
        &seed.initial_state,
        segment.lookahead_instruction,
        segment_info,
        None,
    )
    .expect("host trace should build")
    .expect("layout should support host trace");
    let mut streaming =
        ZiskMainStreamingDeviceSegmentBuilder::new(&layout, &seed.initial_state, segment_info)
            .expect("streaming builder should initialize")
            .expect("layout should support streaming device material");
    let timing_config = ZiskMainTraceLowerTimingConfig::from_env();
    for (report_index, report) in segment.reports.iter().enumerate() {
        streaming
            .push_report_at(
                report_index,
                report,
                || {
                    guest_report_next_instruction(
                        &segment.reports,
                        report_index,
                        segment.lookahead_instruction,
                    )
                },
                timing_config,
                None,
            )
            .expect("streaming report should append");
    }
    let streamed = streaming
        .finish(segment.terminal_pc, None)
        .expect("streaming material should finish");

    assert_device_build_matches_host_trace(&streamed, &host);

    let mut fed_streaming =
        ZiskMainStreamingDeviceSegmentBuilder::new(&layout, &seed.initial_state, segment_info)
            .expect("fed streaming builder should initialize")
            .expect("layout should support fed streaming device material");
    let mut feeder = ZiskMainStreamingDeviceReportFeeder::new(timing_config);
    for report in &segment.reports {
        feeder
            .push_report(&mut fed_streaming, report, None)
            .expect("streaming feeder should append report when lookahead is available");
    }
    feeder
        .finish(&mut fed_streaming, segment.lookahead_instruction, None)
        .expect("streaming feeder should flush final report");
    let fed = fed_streaming
        .finish(segment.terminal_pc, None)
        .expect("fed streaming material should finish");

    assert_device_build_matches_host_trace(&fed, &host);
}

#[test]
#[cfg(feature = "cuda")]
fn runner_streaming_device_material_matches_segment_lowering() {
    let _env_lock = GUEST_PC_TRACE_ENV_LOCK
        .lock()
        .expect("guest PC trace env lock should not be poisoned");
    let dir = repo_temp_dir("guest-pc-runner-streaming-device-builder");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let guest_image_bytes = sample_guest_image_with_words(&[
        riscv_addi(1, 0, 7),
        riscv_addi(2, 1, 3),
        riscv_addi(3, 2, 5),
        riscv_addi(4, 3, 11),
        0x0000_0073,
    ]);
    std::fs::write(&guest_image, &guest_image_bytes).expect("guest image should be written");
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    let unit = sample_unit_with_zisk_main_device_columns_rows(2);
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");
    let context = WitnessComputeContext {
        guest_image: Some(&guest_image),
        guest_image_info: Some(&guest_image_info),
        trace_layout: Some(&layout),
    };
    let (mut memory, mut state, mut fcall_handler) =
        load_guest_pc_trace_machine(context, &[]).expect("guest trace machine should load");
    let seed = ZiskMainSegmentSeed::new();
    let segment = ZiskMainTraceSegmentInfo {
        trace_instance_index: 0,
        is_last_segment: false,
        previous_c: seed.previous_c,
    };

    let streamed = run_guest_pc_trace_segment_slice_with_streaming_device_material(
        &layout,
        &seed.initial_state,
        segment,
        &mut memory,
        &mut state,
        &mut fcall_handler,
        32,
        layout.row_count(),
    )
    .expect("runner streaming material should build")
    .expect("layout should support runner streaming material");
    std::fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(streamed.slice.trace_rows, layout.row_count());
    let host = build_layout_zisk_main_trace_segment(
        &layout,
        &streamed.slice.reports,
        streamed.terminal_pc,
        &seed.initial_state,
        streamed.lookahead_instruction,
        segment,
        None,
    )
    .expect("host trace should build")
    .expect("layout should support host trace");
    assert_device_build_matches_host_trace(&streamed.device_build, &host);

    let expected_seed = advance_zisk_main_segment_seed(
        &layout,
        &streamed.slice.reports,
        streamed.terminal_pc,
        &seed,
        streamed.lookahead_instruction,
        segment,
    )
    .expect("reference seed should advance")
    .expect("layout should support seed advancement");
    assert_eq!(streamed.next_seed, expected_seed);
}

#[test]
#[cfg(feature = "cuda")]
fn streaming_device_report_feeder_matches_host_trace_for_dma_prepare_lookahead() {
    let _env_lock = GUEST_PC_TRACE_ENV_LOCK
        .lock()
        .expect("guest PC trace env lock should not be poisoned");
    let mut initial_state = ZiskMainTraceState::new();
    initial_state.registers[5] = 0x100;
    initial_state.registers[6] = 0x18;
    initial_state.registers[8] = 0x200;
    let prepare = dma_prepare_report_at(ENTRY, RiscvDmaKind::Memcpy, 5);
    let execute = add_report_at(ENTRY + 4, 7, 8, 6, 0x200);

    let unit = sample_unit_with_zisk_main_device_columns_rows(2);
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");
    assert_device_material_matches_host_trace(
        &layout,
        &[prepare.clone(), execute.clone()],
        execute.next_pc,
        &initial_state,
        None,
        ZiskMainTraceSegmentInfo {
            trace_instance_index: 0,
            is_last_segment: false,
            previous_c: 0,
        },
    );

    let boundary_unit = sample_unit_with_zisk_main_device_columns_rows(1);
    let boundary_layout =
        derive_witness_trace_layout(&boundary_unit).expect("boundary layout should derive");
    assert_device_material_matches_host_trace(
        &boundary_layout,
        std::slice::from_ref(&prepare),
        prepare.next_pc,
        &initial_state,
        Some(execute.instruction),
        ZiskMainTraceSegmentInfo {
            trace_instance_index: 1,
            is_last_segment: false,
            previous_c: 0,
        },
    );
}

#[cfg(feature = "cuda")]
fn assert_device_material_matches_host_trace(
    layout: &WitnessTraceLayout,
    reports: &[GuestMachineReport],
    terminal_pc: u64,
    initial_state: &ZiskMainTraceState,
    lookahead_instruction: Option<RiscvInstruction>,
    segment: ZiskMainTraceSegmentInfo,
) {
    let host = build_layout_zisk_main_trace_segment(
        layout,
        reports,
        terminal_pc,
        initial_state,
        lookahead_instruction,
        segment,
        None,
    )
    .expect("host trace should build")
    .expect("host trace layout should be supported");
    let trace_less = build_layout_zisk_main_trace_segment_device_material(
        layout,
        reports,
        terminal_pc,
        initial_state,
        lookahead_instruction,
        segment,
        None,
    )
    .expect("trace-less device material should build")
    .expect("trace-less device material should be supported");

    assert_device_build_matches_host_trace(&trace_less, &host);
}

#[cfg(feature = "cuda")]
fn assert_device_build_matches_host_trace(
    device: &GuestPcTraceDeviceSegmentBuild,
    host: &ZiskMainTraceSegmentWrite,
) {
    let host_material = host
        .device_segment_material
        .as_ref()
        .expect("host trace should include device material");
    assert_eq!(
        device.device_segment_material.trace_source_prefix_rows,
        host_material.trace_source_prefix_rows
    );
    assert_eq!(
        device
            .device_segment_material
            .device_trace_descriptors
            .words(),
        host_material.device_trace_descriptors.words()
    );
    assert_eq!(
        device
            .device_segment_material
            .device_trace_descriptors
            .sparse_high_words(),
        host_material.device_trace_descriptors.sparse_high_words()
    );
    assert_eq!(device.unit_values, host.output.unit_values);
    assert_eq!(device.final_state, host.final_state);
    assert_eq!(device.continuation_state, host.continuation_state);
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
fn zisk_main_source_value_requires_ordered_memory_access() {
    let accesses = [memory_read(72, 13), memory_read(64, 96)];
    let effects = ZiskMainReportEffects {
        register_writes: &[],
        memory_accesses: &accesses,
        precompile_memory_accesses: &[],
        precompile_result: None,
    };
    let state = ZiskMainTraceState::new();
    let report = addi_report();

    let error = zisk_main_source_value(ZiskMainSourceValueRequest {
        row: 9,
        source: ZiskMainSource::Memory(64),
        state: &state,
        report: &report,
        effects,
        base: None,
        ind_width: 0,
        memory_access_index: 0,
    })
    .expect_err("source values should consume the expected memory access position");

    assert!(error.to_string().contains("expected Read at 64"));
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
fn zisk_main_memory_access_validation_after_source_values_checks_store_position() {
    let mut instruction = zisk_main_base_instruction(
        0x8000_0000,
        ZiskMainSource::Memory(64),
        ZiskMainSource::Memory(72),
        ZiskMainOp::CopyB,
        ZiskMainStore::Indirect(0),
        4,
    );
    instruction.ind_width = 8;
    let store_access = memory_write(96, 13);
    let ordered_accesses = [memory_read(64, 96), memory_read(72, 13), store_access];
    let effects = ZiskMainReportEffects {
        register_writes: &[],
        memory_accesses: &ordered_accesses,
        precompile_memory_accesses: &[],
        precompile_result: None,
    };

    validate_zisk_main_memory_accesses_after_source_values(9, &instruction, effects, 96, 13, 2)
        .expect("store after two validated source accesses should validate");

    let misplaced_store = [memory_read(64, 96), store_access, memory_read(72, 13)];
    let effects = ZiskMainReportEffects {
        register_writes: &[],
        memory_accesses: &misplaced_store,
        precompile_memory_accesses: &[],
        precompile_result: None,
    };
    let error =
        validate_zisk_main_memory_accesses_after_source_values(9, &instruction, effects, 96, 13, 2)
            .expect_err("store access should remain checked after validated source accesses");

    assert!(error.to_string().contains("expected Write at 96"));
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

    let row_base = zisk_main_row_mem_step_base(row_count, segment.trace_instance_index, row)
        .expect("row mem-step base should fit");
    let values = apply_zisk_main_register_access_values(row, &instruction, &mut state, row_base)
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

#[test]
fn segment_mem_step_base_matches_row_base_helper() {
    for (row_count, trace_instance_index, rows) in [
        (1, 0, [0, 0, 0]),
        (100, 2, [0, 5, 99]),
        (120_000_000, 8, [0, 42, 119_999_999]),
    ] {
        let segment_base = zisk_main_segment_mem_step_base(row_count, trace_instance_index)
            .expect("segment mem-step base should fit");
        for row in rows {
            assert_eq!(
                zisk_main_row_mem_step_base_from_segment_base(segment_base, row)
                    .expect("precomputed row base should fit"),
                zisk_main_row_mem_step_base(row_count, trace_instance_index, row)
                    .expect("direct row base should fit")
            );
        }
    }
}

#[test]
fn row_mem_step_cursor_matches_direct_offset_helper() {
    let row_count = 120_000_000;
    let trace_instance_index = 8;
    let mut cursor = GuestPcTraceRowMemStepCursor::new(row_count, trace_instance_index)
        .expect("cursor should initialize for supported row count");

    for row in [0, 1, 42, 4_194_303, row_count - 1] {
        cursor
            .advance_to(row)
            .expect("cursor should advance to requested row");
        for offset in [
            ZISK_MAIN_A_MEM_STEP_OFFSET,
            ZISK_MAIN_B_MEM_STEP_OFFSET,
            ZISK_MAIN_STORE_MEM_STEP_OFFSET,
        ] {
            assert_eq!(
                cursor
                    .step(offset)
                    .expect("cursor offset mem-step should fit"),
                zisk_main_row_mem_step(row_count, trace_instance_index, row, offset)
                    .expect("direct row mem-step should fit")
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
        precompile_memory_accesses: precompile_memory_accesses.into(),
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
        precompile_memory_accesses: Vec::new().into(),
        precompile_result: None,
    }
}

#[cfg(feature = "cuda")]
fn add_report_at(address: u64, rd: u8, rs1: u8, rs2: u8, value: u64) -> GuestMachineReport {
    GuestMachineReport {
        address,
        instruction_byte_len: 4,
        instruction: RiscvInstruction::Op {
            kind: RiscvOpKind::Add,
            rd,
            rs1,
            rs2,
        },
        next_pc: address + 4,
        register_writes: vec![GuestRegisterWrite { index: rd, value }].into(),
        memory_accesses: Vec::new().into(),
        precompile_memory_accesses: Vec::new().into(),
        precompile_result: None,
    }
}

#[cfg(feature = "cuda")]
fn dma_prepare_report_at(address: u64, kind: RiscvDmaKind, rs1: u8) -> GuestMachineReport {
    GuestMachineReport {
        address,
        instruction_byte_len: 4,
        instruction: RiscvInstruction::ZiskDmaPrepare { kind, rs1 },
        next_pc: address + 4,
        register_writes: Vec::new().into(),
        memory_accesses: Vec::new().into(),
        precompile_memory_accesses: Vec::new().into(),
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

#[cfg(feature = "cuda")]
fn sample_unit_with_zisk_main_device_columns_rows(row_count: u64) -> ProveUnitSchedule {
    let mut unit = sample_unit_with_zisk_main_columns_rows(row_count);
    unit.stage_commit_widths = vec![39];
    unit.commitment_columns = vec![
        commitment_column("a", 1, 0, 2),
        commitment_column("b", 1, 2, 2),
        commitment_column("c", 1, 4, 2),
        commitment_column("flag", 1, 6, 1),
        commitment_column("pc", 1, 7, 1),
        commitment_column("a_src_imm", 1, 8, 1),
        commitment_column("a_src_mem", 1, 9, 1),
        commitment_column("a_offset_imm0", 1, 10, 1),
        commitment_column("a_imm1", 1, 11, 1),
        commitment_column("is_precompiled", 1, 12, 1),
        commitment_column("b_src_imm", 1, 13, 1),
        commitment_column("b_src_mem", 1, 14, 1),
        commitment_column("b_offset_imm0", 1, 15, 1),
        commitment_column("b_imm1", 1, 16, 1),
        commitment_column("b_src_ind", 1, 17, 1),
        commitment_column("ind_width", 1, 18, 1),
        commitment_column("is_external_op", 1, 19, 1),
        commitment_column("op", 1, 20, 1),
        commitment_column("store_pc", 1, 21, 1),
        commitment_column("store_mem", 1, 22, 1),
        commitment_column("store_ind", 1, 23, 1),
        commitment_column("store_offset", 1, 24, 1),
        commitment_column("set_pc", 1, 25, 1),
        commitment_column("jmp_offset1", 1, 26, 1),
        commitment_column("jmp_offset2", 1, 27, 1),
        commitment_column("m32", 1, 28, 1),
        commitment_column("addr1", 1, 29, 1),
        commitment_column("a_reg_prev_mem_step", 1, 30, 1),
        commitment_column("b_reg_prev_mem_step", 1, 31, 1),
        commitment_column("store_reg_prev_mem_step", 1, 32, 1),
        commitment_column("store_reg_prev_value", 1, 33, 2),
        commitment_column("a_src_reg", 1, 35, 1),
        commitment_column("b_src_reg", 1, 36, 1),
        commitment_column("store_reg", 1, 37, 1),
        commitment_column("known_zero", 1, 38, 1),
    ];
    unit
}
