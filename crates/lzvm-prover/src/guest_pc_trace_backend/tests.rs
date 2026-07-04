use super::*;
use crate::guest_instruction::{
    decode_riscv_instruction, RiscvAmoKind, RiscvAmoWidth, RiscvBranchKind, RiscvCsr,
    RiscvFenceKind, RiscvInstruction, RiscvLoadKind, RiscvOp32Kind, RiscvOpImm32Kind,
    RiscvOpImmKind, RiscvOpKind, RiscvPrecompileKind, RiscvStoreKind,
};
use crate::guest_machine::GuestMachineReportShape;
use crate::guest_machine::GuestPrecompileReportEffects;
use crate::witness_layout::{derive_witness_trace_layout, WitnessTraceLayout};
use crate::witness_loader::WitnessComputeContext;
use crate::witness_trace::parse_witness_trace;
use crate::ProveUnitSchedule;
use lzvm_artifacts::guest_image::{parse_guest_image, GuestImageInfo};
use lzvm_artifacts::key_directory::KeyUnitKind;
use lzvm_artifacts::pcs_plan::PcsFriLayer;
use lzvm_artifacts::setup_info::CommitmentColumn;
use lzvm_field::Felt;

struct GuestPcTraceEnvLock;

static GUEST_PC_TRACE_ENV_LOCK: GuestPcTraceEnvLock = GuestPcTraceEnvLock;

impl GuestPcTraceEnvLock {
    fn lock(&self) -> std::sync::LockResult<std::sync::MutexGuard<'static, ()>> {
        crate::CUDA_TEST_ENV_LOCK.lock()
    }
}

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
fn guest_pc_trace_thread_spawn_error_preserves_source() {
    let error = GuestPcTraceBackendError::ThreadSpawn {
        name: "lzvm-gp-test",
        source: std::io::Error::new(std::io::ErrorKind::WouldBlock, "thread limit"),
    };

    assert!(error
        .to_string()
        .contains("guest PC trace backend thread lzvm-gp-test failed to spawn"));
    let source =
        std::error::Error::source(&error).expect("thread spawn source should be preserved");
    let io_source = source
        .downcast_ref::<std::io::Error>()
        .expect("thread spawn source should remain an I/O error");
    assert_eq!(io_source.kind(), std::io::ErrorKind::WouldBlock);
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
    let shape_env = EnvGuard::new("LZVM_GUEST_TRACE_SHAPE_TIMING_SAMPLE_STRIDE");
    let full_shape_env = EnvGuard::new("LZVM_GUEST_TRACE_SHAPE_TIMING");
    assert_eq!(guest_pc_trace_detail_timing_sample_stride(), 1);
    assert_eq!(guest_pc_trace_shape_timing_sample_stride(), None);

    env.set("0");
    assert_eq!(guest_pc_trace_detail_timing_sample_stride(), 1);
    shape_env.set("0");
    assert_eq!(guest_pc_trace_shape_timing_sample_stride(), None);

    env.set("not-a-number");
    assert_eq!(guest_pc_trace_detail_timing_sample_stride(), 1);
    shape_env.set("not-a-number");
    assert_eq!(guest_pc_trace_shape_timing_sample_stride(), None);

    env.set("17");
    assert_eq!(guest_pc_trace_detail_timing_sample_stride(), 17);
    shape_env.set("17");
    assert_eq!(guest_pc_trace_shape_timing_sample_stride(), Some(17));

    #[cfg(feature = "cuda")]
    {
        let timing_config = ZiskMainTraceLowerTimingConfig::from_env();
        assert!(timing_config.row_timing_enabled);
        assert!(timing_config.shape_timing_for_report(0));
        assert!(!timing_config.shape_timing_for_report(16));
        assert!(timing_config.shape_timing_for_report(17));

        full_shape_env.set("1");
        let full_timing_config = ZiskMainTraceLowerTimingConfig::from_env();
        assert!(full_timing_config.shape_timing);
        assert_eq!(full_timing_config.shape_sample_stride, None);
        assert!(full_timing_config.shape_timing_for_report(16));
    }

    env.clear();
    shape_env.clear();
    full_shape_env.clear();
    assert_eq!(guest_pc_trace_detail_timing_sample_stride(), 1);
    assert_eq!(guest_pc_trace_shape_timing_sample_stride(), None);
}

#[test]
fn guest_pc_trace_runner_detail_timing_ignores_unexecuted_row_fit_probe() {
    let _env_lock = GUEST_PC_TRACE_ENV_LOCK
        .lock()
        .expect("guest PC trace env lock should not be poisoned");
    let _detail_env = TestEnvVarGuard::set("LZVM_GUEST_TRACE_RUNNER_DETAIL_TIMING", "1");
    let _stride_env =
        TestEnvVarGuard::set("LZVM_GUEST_TRACE_RUNNER_DETAIL_TIMING_SAMPLE_STRIDE", "1");
    let dir = repo_temp_dir("guest-pc-runner-detail-row-fit");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let guest_image_bytes = sample_guest_image_with_words(&[riscv_amo_add_d(1, 2, 3)]);
    std::fs::write(&guest_image, &guest_image_bytes).expect("guest image should be written");
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    let context = WitnessComputeContext {
        guest_image: Some(&guest_image),
        guest_image_info: Some(&guest_image_info),
        trace_layout: None,
    };
    let (mut memory, mut state, mut fcall_handler) =
        load_guest_pc_trace_machine(context, &[]).expect("guest machine should load");
    let mut instruction_cache = GuestInstructionCache::default();
    let mut timing = GuestPcTraceStreamTiming::default();

    let result = run_guest_pc_trace_segment_slice_with_cache(
        &mut memory,
        &mut state,
        &mut fcall_handler,
        32,
        1,
        &mut instruction_cache,
        Some(&mut timing),
    );
    let Err(error) = result else {
        panic!("oversized first report should fail before advance");
    };
    std::fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert!(matches!(
        error,
        GuestPcTraceBackendError::InvalidPcTraceLayout { .. }
    ));
    assert_eq!(timing.runner_detail_sample_count(), 0);
    assert_eq!(timing.runner_detail_duration(), std::time::Duration::ZERO);
    assert_eq!(
        timing.runner_prepare_instruction_duration(),
        std::time::Duration::ZERO
    );
    assert_eq!(
        timing.runner_pre_boundary_duration(),
        std::time::Duration::ZERO
    );
    assert_eq!(
        timing.runner_cache_policy_duration(),
        std::time::Duration::ZERO
    );
    assert_eq!(
        timing.runner_advance_setup_duration(),
        std::time::Duration::ZERO
    );
    assert_eq!(
        timing.runner_advance_execute_duration(),
        std::time::Duration::ZERO
    );
    assert_eq!(
        timing.runner_advance_report_duration(),
        std::time::Duration::ZERO
    );
    assert_eq!(
        timing.runner_post_boundary_duration(),
        std::time::Duration::ZERO
    );
    assert_eq!(
        timing.runner_counter_update_duration(),
        std::time::Duration::ZERO
    );
}

#[test]
fn guest_pc_trace_runner_path_counts_stay_zero_when_disabled() {
    let _env_lock = GUEST_PC_TRACE_ENV_LOCK
        .lock()
        .expect("guest PC trace env lock should not be poisoned");
    let _path_env = TestEnvVarGuard::unset("LZVM_GUEST_TRACE_RUNNER_PATH_TIMING");
    let _cache_env = TestEnvVarGuard::unset("LZVM_GUEST_TRACE_RUNNER_CACHE_STATS");
    let dir = repo_temp_dir("guest-pc-runner-path-counts-disabled");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let guest_image_bytes =
        sample_guest_image_with_words(&[riscv_addi(1, 0, 7), riscv_addi(2, 1, 3), 0x0000_0073]);
    std::fs::write(&guest_image, &guest_image_bytes).expect("guest image should be written");
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    let context = WitnessComputeContext {
        guest_image: Some(&guest_image),
        guest_image_info: Some(&guest_image_info),
        trace_layout: None,
    };
    let (mut memory, mut state, mut fcall_handler) =
        load_guest_pc_trace_machine(context, &[]).expect("guest machine should load");
    let mut instruction_cache = GuestInstructionCache::default();
    let mut timing = GuestPcTraceStreamTiming::default();

    let slice = run_guest_pc_trace_segment_slice_with_cache(
        &mut memory,
        &mut state,
        &mut fcall_handler,
        32,
        16,
        &mut instruction_cache,
        Some(&mut timing),
    )
    .expect("guest trace slice should run");
    std::fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(slice.report_count, 2);
    assert_eq!(timing.runner_advance_fast_path_count(), 0);
    assert_eq!(timing.runner_advance_generic_fallback_count(), 0);
    assert_eq!(timing.runner_instruction_cache_hit_count(), 0);
    assert_eq!(timing.runner_instruction_cache_miss_count(), 0);
    assert_eq!(timing.runner_instruction_cache_clear_count(), 0);
    assert_eq!(timing.runner_instruction_cache_fcall_clear_count(), 0);
    assert_eq!(timing.runner_instruction_cache_dma_clear_count(), 0);
    assert_eq!(
        timing.runner_instruction_cache_write_invalidation_range_count(),
        0
    );
    assert_eq!(
        timing.runner_instruction_cache_write_invalidation_skipped_range_count(),
        0
    );
    assert_eq!(
        timing.runner_instruction_cache_write_invalidation_probe_count(),
        0
    );
    assert_eq!(timing.runner_instruction_cache_invalidated_entry_count(), 0);
}

#[test]
fn guest_pc_trace_runner_cache_stats_count_hits_when_enabled() {
    let _env_lock = GUEST_PC_TRACE_ENV_LOCK
        .lock()
        .expect("guest PC trace env lock should not be poisoned");
    let _cache_env = TestEnvVarGuard::set("LZVM_GUEST_TRACE_RUNNER_CACHE_STATS", "1");
    let dir = repo_temp_dir("guest-pc-runner-cache-stats");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let guest_image_bytes = sample_guest_image_with_words(&[0x0000_006f]);
    std::fs::write(&guest_image, &guest_image_bytes).expect("guest image should be written");
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    let context = WitnessComputeContext {
        guest_image: Some(&guest_image),
        guest_image_info: Some(&guest_image_info),
        trace_layout: None,
    };
    let (mut memory, mut state, mut fcall_handler) =
        load_guest_pc_trace_machine(context, &[]).expect("guest machine should load");
    let mut instruction_cache = GuestInstructionCache::default();
    let mut timing = GuestPcTraceStreamTiming::default();

    let slice = run_guest_pc_trace_segment_slice_with_cache(
        &mut memory,
        &mut state,
        &mut fcall_handler,
        6,
        32,
        &mut instruction_cache,
        Some(&mut timing),
    )
    .expect("guest trace slice should run");
    std::fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(slice.report_count, 6);
    assert_eq!(timing.runner_instruction_cache_miss_count(), 1);
    assert_eq!(timing.runner_instruction_cache_hit_count(), 6);
    assert_eq!(timing.runner_instruction_cache_clear_count(), 0);
    assert_eq!(timing.runner_instruction_cache_fcall_clear_count(), 0);
    assert_eq!(timing.runner_instruction_cache_dma_clear_count(), 0);
    assert_eq!(
        timing.runner_instruction_cache_write_invalidation_range_count(),
        0
    );
}

#[test]
fn guest_pc_trace_report_chunk_capacity_defaults_to_large_batches() {
    let _env_lock = GUEST_PC_TRACE_ENV_LOCK
        .lock()
        .expect("guest PC trace env lock should not be poisoned");
    let _capacity_env = TestEnvVarGuard::unset("LZVM_GUEST_PC_TRACE_REPORT_CHUNK_CAPACITY");

    assert_eq!(guest_pc_trace_report_chunk_capacity(), 65_536);

    std::env::set_var("LZVM_GUEST_PC_TRACE_REPORT_CHUNK_CAPACITY", "1");
    assert_eq!(guest_pc_trace_report_chunk_capacity(), 1);

    std::env::set_var("LZVM_GUEST_PC_TRACE_REPORT_CHUNK_CAPACITY", "0");
    assert_eq!(guest_pc_trace_report_chunk_capacity(), 65_536);
}

#[test]
fn guest_pc_trace_segment_queue_capacity_defaults_to_measured_overlap_depth() {
    let _env_lock = GUEST_PC_TRACE_ENV_LOCK
        .lock()
        .expect("guest PC trace env lock should not be poisoned");
    let _queue_env = TestEnvVarGuard::unset("LZVM_GUEST_PC_TRACE_SEGMENT_QUEUE");

    assert_eq!(guest_pc_trace_segment_queue_capacity(), 2);
}

#[test]
fn guest_pc_trace_segment_queue_capacity_uses_runtime_override() {
    let _env_lock = GUEST_PC_TRACE_ENV_LOCK
        .lock()
        .expect("guest PC trace env lock should not be poisoned");
    let _queue_env = TestEnvVarGuard::unset("LZVM_GUEST_PC_TRACE_SEGMENT_QUEUE");

    std::env::set_var("LZVM_GUEST_PC_TRACE_SEGMENT_QUEUE", "5");
    assert_eq!(guest_pc_trace_segment_queue_capacity(), 5);
}

#[test]
fn guest_pc_trace_parallel_lower_job_queue_capacity_is_bounded_and_configurable() {
    let _env_lock = GUEST_PC_TRACE_ENV_LOCK
        .lock()
        .expect("guest PC trace env lock should not be poisoned");
    let _queue_env = TestEnvVarGuard::unset("LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_JOB_QUEUE");

    assert_eq!(guest_pc_trace_parallel_lower_job_queue_capacity(0), 4);
    assert_eq!(guest_pc_trace_parallel_lower_job_queue_capacity(2), 4);
    assert_eq!(guest_pc_trace_parallel_lower_job_queue_capacity(8), 8);
    assert_eq!(guest_pc_trace_parallel_lower_job_queue_capacity(32), 16);

    std::env::set_var("LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_JOB_QUEUE", "9");
    assert_eq!(guest_pc_trace_parallel_lower_job_queue_capacity(32), 9);

    std::env::set_var("LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_JOB_QUEUE", "0");
    assert_eq!(guest_pc_trace_parallel_lower_job_queue_capacity(8), 8);
}

#[test]
fn guest_pc_trace_seed_discovery_stays_opt_in() {
    let _env_lock = GUEST_PC_TRACE_ENV_LOCK
        .lock()
        .expect("guest PC trace env lock should not be poisoned");
    let _discovery_env = TestEnvVarGuard::unset("LZVM_GUEST_PC_TRACE_SEED_DISCOVERY");

    assert!(!guest_pc_trace_seed_discovery_enabled());

    std::env::set_var("LZVM_GUEST_PC_TRACE_SEED_DISCOVERY", "1");
    assert!(guest_pc_trace_seed_discovery_enabled());

    std::env::set_var("LZVM_GUEST_PC_TRACE_SEED_DISCOVERY", "0");
    assert!(!guest_pc_trace_seed_discovery_enabled());
}

#[test]
#[cfg(feature = "cuda")]
fn guest_pc_trace_seed_discovery_streaming_device_lower_stays_opt_in() {
    let _env_lock = GUEST_PC_TRACE_ENV_LOCK
        .lock()
        .expect("guest PC trace env lock should not be poisoned");
    let _streaming_env =
        TestEnvVarGuard::unset("LZVM_GUEST_PC_TRACE_SEED_DISCOVERY_STREAMING_DEVICE_LOWER");

    assert!(!guest_pc_trace_seed_discovery_streaming_device_lower_enabled());

    std::env::set_var(
        "LZVM_GUEST_PC_TRACE_SEED_DISCOVERY_STREAMING_DEVICE_LOWER",
        "1",
    );
    assert!(guest_pc_trace_seed_discovery_streaming_device_lower_enabled());

    std::env::set_var(
        "LZVM_GUEST_PC_TRACE_SEED_DISCOVERY_STREAMING_DEVICE_LOWER",
        "0",
    );
    assert!(!guest_pc_trace_seed_discovery_streaming_device_lower_enabled());
}

#[test]
fn guest_pc_trace_weighted_contiguous_chunks_isolate_heavy_segments() {
    let weights = [1_usize, 1, 1, 1, 100, 1, 1, 1, 1, 1, 1, 1];

    let ranges = guest_pc_trace_weighted_contiguous_chunk_ranges(&weights, 3, |weight| *weight);

    assert_eq!(ranges, vec![0..4, 4..5, 5..12]);
    assert!(ranges.iter().all(|range| !range.is_empty()));
    assert_eq!(ranges.first().map(|range| range.start), Some(0));
    assert_eq!(ranges.last().map(|range| range.end), Some(weights.len()));
}

#[test]
fn guest_pc_trace_seed_discovery_scans_without_retaining_reports() {
    let _env_lock = GUEST_PC_TRACE_ENV_LOCK
        .lock()
        .expect("guest PC trace env lock should not be poisoned");
    let _mirror_env = TestEnvVarGuard::set("LZVM_GUEST_PC_TRACE_SEED_MIRROR", "1");
    let dir = repo_temp_dir("guest-pc-seed-discovery");
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
    let unit = sample_unit_with_zisk_main_columns_rows(2);
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");
    let context = WitnessComputeContext {
        guest_image: Some(&guest_image),
        guest_image_info: Some(&guest_image_info),
        trace_layout: Some(&layout),
    };
    let mut serial_pending = Vec::new();
    let serial =
        produce_guest_pc_trace_pending_slices(32, context, &[], layout.row_count(), |segment| {
            serial_pending.push(segment);
            Ok(())
        })
        .expect("serial seed mirror should produce pending slices");
    let discovered = discover_guest_pc_trace_segment_seeds(32, context, &[], layout.row_count())
        .expect("seed discovery should scan the same segments");
    let (mut expected_memory, mut expected_state, mut expected_fcall_handler) =
        load_guest_pc_trace_machine(context, &[]).expect("expected machine should load");
    let mut expected_machine_states = Vec::new();
    let mut expected_input_mapped_states = Vec::new();
    for expected in &serial_pending {
        expected_machine_states.push(expected_state.clone());
        expected_input_mapped_states.push(expected_fcall_handler.input_data_was_mapped());
        let slice = run_guest_pc_trace_segment_slice(
            &mut expected_memory,
            &mut expected_state,
            &mut expected_fcall_handler,
            expected.runner_remaining_instruction_limit,
            layout.row_count(),
        )
        .expect("expected machine segment should run");
        assert_eq!(
            slice.executed_instructions,
            expected.executed_instruction_count
        );
        assert_eq!(slice.trace_rows, expected.trace_row_count);
    }
    std::fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(discovered.proof_values, serial.proof_values);
    assert_eq!(discovered.segments.len(), serial_pending.len());
    assert_eq!(
        discovered.timing.seed_direct_lift_success_count(),
        serial_pending.len().saturating_sub(1)
    );
    assert_eq!(discovered.timing.seed_full_advance_count(), 0);
    assert_eq!(discovered.timing.trace_runner_report_buffer_capacity(), 0);
    for (discovered, expected) in discovered.segments.iter().zip(&serial_pending) {
        assert_eq!(
            &discovered.seed,
            expected
                .seed
                .as_deref()
                .expect("serial seed mirror should attach segment seeds")
        );
        assert_eq!(
            discovered.trace_instance_index,
            expected.trace_instance_index
        );
        assert_eq!(
            discovered.executed_instruction_count,
            expected.executed_instruction_count
        );
        assert_eq!(discovered.trace_row_count, expected.trace_row_count);
        assert_eq!(discovered.report_count, expected.report_count);
        assert_eq!(
            discovered.runner_remaining_instruction_limit,
            expected.runner_remaining_instruction_limit
        );
        assert_eq!(
            &discovered.machine_state,
            &expected_machine_states[usize::try_from(discovered.trace_instance_index)
                .expect("trace instance index should fit")]
        );
        assert_eq!(
            discovered.fcall_state.input_data_was_mapped(),
            expected_input_mapped_states[usize::try_from(discovered.trace_instance_index)
                .expect("trace instance index should fit")]
        );
        assert_eq!(discovered.terminal_pc, expected.terminal_pc);
        assert_eq!(
            discovered.lookahead_instruction,
            expected.lookahead_instruction
        );
        assert_eq!(discovered.is_last_segment, expected.is_last_segment);
    }
    for (index, discovery) in discovered.segments.iter().enumerate() {
        if discovery.is_last_segment {
            assert!(discovery.next_seed.is_none());
            continue;
        }
        let next_discovered = discovery
            .next_seed
            .as_ref()
            .expect("non-terminal discovery segment should carry its next seed");
        assert_eq!(
            next_discovered,
            &discovered.segments[index + 1].seed,
            "discovery next seed should match the following segment seed"
        );
        let expected = &serial_pending[index];
        let expected_seed = expected
            .seed
            .as_deref()
            .expect("serial seed mirror should attach segment seeds");
        let expected_lowered = lower_guest_pc_trace_seeded_pending_segment_with_output_mode(
            &layout,
            expected,
            expected_seed,
            None,
            false,
            None,
        )
        .expect("serial pending segment should lower");
        assert_eq!(next_discovered, &expected_lowered.next_seed);
    }
}

#[test]
fn guest_pc_trace_seed_discovery_tracks_input_mapping_boundaries() {
    let _env_lock = GUEST_PC_TRACE_ENV_LOCK
        .lock()
        .expect("guest PC trace env lock should not be poisoned");
    let _mirror_env = TestEnvVarGuard::set("LZVM_GUEST_PC_TRACE_SEED_MIRROR", "1");
    let dir = repo_temp_dir("guest-pc-seed-discovery-input-map");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let guest_image_bytes = sample_guest_image_with_words(&[
        riscv_lui(1, 0x40000),
        riscv_addi(1, 1, 7),
        riscv_zisk_fcall_param(0, 1),
        riscv_zisk_fcall_invoke(crate::zisk_fcalls::ZISK_INPUT_READY_FCALL_ID),
        riscv_addi(2, 0, 5),
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
    let input = framed_stdin_chunk(b"abc");
    let discovered = discover_guest_pc_trace_segment_seeds(32, context, &input, layout.row_count())
        .expect("seed discovery should scan the input-ready program");
    let (mut expected_memory, mut expected_state, mut expected_fcall_handler) =
        load_guest_pc_trace_machine(context, &input).expect("expected machine should load");
    let mut expected_input_mapped_states = Vec::new();
    for discovered_segment in &discovered.segments {
        expected_input_mapped_states.push(expected_fcall_handler.input_data_was_mapped());
        run_guest_pc_trace_segment_slice(
            &mut expected_memory,
            &mut expected_state,
            &mut expected_fcall_handler,
            discovered_segment.runner_remaining_instruction_limit,
            layout.row_count(),
        )
        .expect("expected input-ready segment should run");
    }
    let discovered_input_mapped_states = discovered
        .segments
        .iter()
        .map(|segment| segment.fcall_state.input_data_was_mapped())
        .collect::<Vec<_>>();
    assert_eq!(discovered_input_mapped_states, expected_input_mapped_states);
    assert!(
        discovered_input_mapped_states
            .windows(2)
            .any(|window| window == [false, true]),
        "input-ready fcall should flip the boundary state between segments"
    );

    let rebuilt = discovered
        .segments
        .iter()
        .map(|segment| {
            let (mut rebuilt_memory, _, _) =
                load_guest_pc_trace_machine(context, &input).expect("replay machine should load");
            let handler = segment
                .fcall_state
                .rebuild_input_handler_with_memory(&input, &mut rebuilt_memory)
                .expect("boundary fcall state should rebuild");
            if segment.fcall_state.input_data_was_mapped() {
                let mut expected = vec![0; std::mem::size_of::<u64>() + input.len()];
                let mut rebuilt = vec![0; expected.len()];
                expected_memory
                    .read_range_into(ZISK_INPUT_ADDRESS, &mut expected)
                    .expect("expected mapped input image should read");
                rebuilt_memory
                    .read_range_into(ZISK_INPUT_ADDRESS, &mut rebuilt)
                    .expect("rebuilt mapped input image should read");
                assert_eq!(rebuilt, expected);
            }
            handler.input_data_was_mapped()
        })
        .collect::<Vec<_>>();
    assert_eq!(rebuilt, discovered_input_mapped_states);
    std::fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn guest_pc_trace_seed_discovery_lifts_fcall_result_boundary() {
    let _env_lock = GUEST_PC_TRACE_ENV_LOCK
        .lock()
        .expect("guest PC trace env lock should not be poisoned");
    let _mirror_env = TestEnvVarGuard::set("LZVM_GUEST_PC_TRACE_SEED_MIRROR", "1");
    let dir = repo_temp_dir("guest-pc-seed-discovery-fcall-result");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let data_offset = 64_u16;
    let mut words = vec![
        riscv_auipc(10, 0),
        riscv_addi(10, 10, data_offset as i16),
        riscv_addi(11, 0, 2),
        riscv_zisk_fcall_param(0, 11),
        riscv_zisk_fcall_param(2, 10),
        riscv_addi(10, 10, 32),
        riscv_zisk_fcall_param(2, 10),
        riscv_zisk_fcall_invoke(crate::zisk_fcalls::ZISK_MSB_POS_256_FCALL_ID),
        riscv_zisk_fcall_result(12),
        riscv_addi(13, 12, 1),
        0x0000_0073,
    ];
    while words.len() * std::mem::size_of::<u32>() < usize::from(data_offset) {
        words.push(0);
    }
    for value in [0_u64, 1 << 9, 0, 0, 0, 0, 0, 0] {
        let bytes = value.to_le_bytes();
        words.push(u32::from_le_bytes(bytes[..4].try_into().expect("low word")));
        words.push(u32::from_le_bytes(
            bytes[4..].try_into().expect("high word"),
        ));
    }
    let guest_image_bytes = sample_guest_image_with_words(&words);
    std::fs::write(&guest_image, &guest_image_bytes).expect("guest image should be written");
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    let unit = sample_unit_with_zisk_main_columns_rows(3);
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");
    let context = WitnessComputeContext {
        guest_image: Some(&guest_image),
        guest_image_info: Some(&guest_image_info),
        trace_layout: Some(&layout),
    };
    let mut serial_pending = Vec::new();
    produce_guest_pc_trace_pending_slices(32, context, &[], layout.row_count(), |segment| {
        serial_pending.push(segment);
        Ok(())
    })
    .expect("serial pending slices should produce");
    assert!(
        serial_pending
            .iter()
            .any(|segment| segment.terminal_pc == ENTRY + 9 * 4),
        "fixture should end a non-final segment after the fcall result"
    );

    let discovered = discover_guest_pc_trace_segment_seeds(32, context, &[], layout.row_count())
        .expect("seed discovery should lift the fcall-result boundary");
    std::fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(discovered.segments.len(), serial_pending.len());
    for (discovered, expected) in discovered.segments.iter().zip(&serial_pending) {
        assert_eq!(
            &discovered.seed,
            expected
                .seed
                .as_deref()
                .expect("serial seed mirror should attach segment seeds")
        );
        assert_eq!(discovered.terminal_pc, expected.terminal_pc);
    }
}

#[test]
fn guest_pc_trace_seed_discovery_restores_written_memory_boundaries() {
    let _env_lock = GUEST_PC_TRACE_ENV_LOCK
        .lock()
        .expect("guest PC trace env lock should not be poisoned");
    let _mirror_env = TestEnvVarGuard::set("LZVM_GUEST_PC_TRACE_SEED_MIRROR", "1");
    let dir = repo_temp_dir("guest-pc-seed-discovery-memory-boundary");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let guest_image_info = write_store_load_replay_guest_image(&guest_image);
    let layout = store_load_replay_layout();
    let context = WitnessComputeContext {
        guest_image: Some(&guest_image),
        guest_image_info: Some(&guest_image_info),
        trace_layout: Some(&layout),
    };
    let discovered = discover_guest_pc_trace_segment_seeds(32, context, &[], layout.row_count())
        .expect("seed discovery should scan the store-load fixture");
    assert!(
        discovered.segments.len() >= 2,
        "fixture should split after the store"
    );
    assert_eq!(discovered.segments[0].terminal_pc, ENTRY + 4 * 4);

    let (mut expected_memory, mut expected_state, mut expected_fcall_handler) =
        load_guest_pc_trace_machine(context, &[]).expect("expected machine should load");
    run_guest_pc_trace_segment_slice(
        &mut expected_memory,
        &mut expected_state,
        &mut expected_fcall_handler,
        discovered.segments[0].runner_remaining_instruction_limit,
        layout.row_count(),
    )
    .expect("expected first segment should run");

    let boundary = &discovered.segments[1];
    let mut expected_second_memory = expected_memory.clone();
    let mut expected_second_state = expected_state.clone();
    let mut expected_second_fcall_handler = ZiskInputFcallHandler::new(&[])
        .expect("expected second segment fcall handler should build");
    let expected_second = run_guest_pc_trace_segment_slice(
        &mut expected_second_memory,
        &mut expected_second_state,
        &mut expected_second_fcall_handler,
        boundary.runner_remaining_instruction_limit,
        layout.row_count(),
    )
    .expect("expected second segment should run");

    let replay = replay_guest_pc_trace_segment_from_snapshot(
        boundary
            .replay_snapshot(context, &[])
            .expect("boundary replay snapshot should rebuild"),
        boundary.runner_remaining_instruction_limit,
        layout.row_count(),
    )
    .expect("replayed second segment should run");
    std::fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(replay.slice.reports, expected_second.reports);
    assert_eq!(replay.state, expected_second_state);
    assert_eq!(replay.memory, expected_second_memory);
}

#[test]
fn guest_pc_trace_seed_discovery_builds_replayable_pending_segments() {
    let _env_lock = GUEST_PC_TRACE_ENV_LOCK
        .lock()
        .expect("guest PC trace env lock should not be poisoned");
    let _mirror_env = TestEnvVarGuard::set("LZVM_GUEST_PC_TRACE_SEED_MIRROR", "1");
    let dir = repo_temp_dir("guest-pc-seed-discovery-replayable-pending");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let guest_image_info = write_store_load_replay_guest_image(&guest_image);
    let layout = store_load_replay_layout();
    let context = WitnessComputeContext {
        guest_image: Some(&guest_image),
        guest_image_info: Some(&guest_image_info),
        trace_layout: Some(&layout),
    };
    let mut serial_pending = Vec::new();
    let serial =
        produce_guest_pc_trace_pending_slices(32, context, &[], layout.row_count(), |segment| {
            serial_pending.push(segment);
            Ok(())
        })
        .expect("serial pending slices should produce");
    let discovered = discover_guest_pc_trace_segment_seeds(32, context, &[], layout.row_count())
        .expect("seed discovery should scan the store-load fixture");
    let mut replayable = discovered
        .replayable_pending_segments(context, &[])
        .expect("seed discovery should build replayable pending segments");
    std::fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(discovered.proof_values, serial.proof_values);
    assert_eq!(replayable.len(), serial_pending.len());
    let mut timing = GuestPcTraceStreamTiming::default();
    for (rebuilt, expected) in replayable.iter_mut().zip(&serial_pending) {
        assert_eq!(rebuilt.trace_instance_index, expected.trace_instance_index);
        assert_eq!(
            rebuilt.executed_instruction_count,
            expected.executed_instruction_count
        );
        assert_eq!(rebuilt.trace_row_count, expected.trace_row_count);
        assert_eq!(
            rebuilt.runner_remaining_instruction_limit,
            expected.runner_remaining_instruction_limit
        );
        assert_eq!(rebuilt.report_count, expected.report_count);
        assert!(rebuilt.reports.is_empty());
        assert!(rebuilt.reports_elided);
        assert_eq!(rebuilt.terminal_pc, expected.terminal_pc);
        assert_eq!(
            rebuilt.lookahead_instruction,
            expected.lookahead_instruction
        );
        assert_eq!(rebuilt.is_last_segment, expected.is_last_segment);
        assert_eq!(rebuilt.seed, expected.seed);
        assert!(rebuilt.replay_snapshot.is_some());

        replay_guest_pc_trace_pending_segment_reports(&layout, rebuilt, &mut timing)
            .expect("replayable pending segment should restore reports");
        assert_eq!(rebuilt.reports, expected.reports);
        let seed = rebuilt
            .seed
            .as_deref()
            .expect("replayable pending segment should carry a seed");
        let replay_lowered = lower_guest_pc_trace_seeded_pending_segment_with_output_mode(
            &layout, rebuilt, seed, None, false, None,
        )
        .expect("replayable pending segment should lower");
        let serial_lowered = lower_guest_pc_trace_seeded_pending_segment_with_output_mode(
            &layout, expected, seed, None, false, None,
        )
        .expect("serial pending segment should lower");
        assert_eq!(replay_lowered.next_seed, serial_lowered.next_seed);
        assert_eq!(replay_lowered.segment.trace, serial_lowered.segment.trace);
        assert_eq!(
            replay_lowered.segment.unit_values,
            serial_lowered.segment.unit_values
        );
    }
    assert_eq!(
        timing.parallel_lower_snapshot_replay_count(),
        replayable.len()
    );
}

#[test]
fn guest_pc_trace_seed_discovery_lowers_replayable_pending_segments() {
    let _env_lock = GUEST_PC_TRACE_ENV_LOCK
        .lock()
        .expect("guest PC trace env lock should not be poisoned");
    let _mirror_env = TestEnvVarGuard::set("LZVM_GUEST_PC_TRACE_SEED_MIRROR", "1");
    let dir = repo_temp_dir("guest-pc-seed-discovery-lower-replayable-pending");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let guest_image_info = write_store_load_replay_guest_image(&guest_image);
    let layout = store_load_replay_layout();
    let context = WitnessComputeContext {
        guest_image: Some(&guest_image),
        guest_image_info: Some(&guest_image_info),
        trace_layout: Some(&layout),
    };
    let mut serial_pending = Vec::new();
    produce_guest_pc_trace_pending_slices(32, context, &[], layout.row_count(), |segment| {
        serial_pending.push(segment);
        Ok(())
    })
    .expect("serial pending slices should produce");
    let discovered = discover_guest_pc_trace_segment_seeds(32, context, &[], layout.row_count())
        .expect("seed discovery should scan the store-load fixture");
    let mut timing = GuestPcTraceStreamTiming::default();
    let replay_lowered = discovered
        .lower_replayable_pending_segments(&layout, context, &[], None, 2, Some(&mut timing))
        .expect("seed discovery should lower replayable pending segments");
    std::fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    let serial_lowered =
        lower_guest_pc_trace_seeded_pending_segments_with_workers(&layout, serial_pending, None, 2)
            .expect("serial pending segments should lower");
    assert_eq!(replay_lowered.len(), serial_lowered.len());
    for (replay, serial) in replay_lowered.iter().zip(&serial_lowered) {
        assert_eq!(replay.next_seed, serial.next_seed);
        assert_eq!(
            replay.segment.trace_instance_index,
            serial.segment.trace_instance_index
        );
        assert_eq!(
            replay.segment.trace_source_prefix_rows,
            serial.segment.trace_source_prefix_rows
        );
        assert_eq!(replay.segment.trace, serial.segment.trace);
        assert_eq!(replay.segment.unit_values, serial.segment.unit_values);
    }
    assert_eq!(
        timing.parallel_lower_snapshot_replay_count(),
        replay_lowered.len()
    );
}

#[test]
#[cfg(feature = "cuda")]
fn guest_pc_trace_seed_discovery_streaming_device_lower_matches_serial_lower() {
    let _env_lock = GUEST_PC_TRACE_ENV_LOCK
        .lock()
        .expect("guest PC trace env lock should not be poisoned");
    let _mirror_env = TestEnvVarGuard::set("LZVM_GUEST_PC_TRACE_SEED_MIRROR", "1");
    let dir = repo_temp_dir("guest-pc-seed-discovery-streaming-device-lower");
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
    let mut serial_pending = Vec::new();
    produce_guest_pc_trace_pending_slices(32, context, &[], layout.row_count(), |segment| {
        serial_pending.push(segment);
        Ok(())
    })
    .expect("serial pending slices should produce");
    let discovered = discover_guest_pc_trace_segment_seeds(32, context, &[], layout.row_count())
        .expect("seed discovery should scan the device fixture");
    let serial_lowered =
        lower_guest_pc_trace_seeded_pending_segments_with_workers(&layout, serial_pending, None, 2)
            .expect("serial pending segments should lower");
    let mut streaming_timing = GuestPcTraceStreamTiming::default();
    let streaming_lowered = discovered
        .lower_streaming_device_segments(
            &layout,
            context,
            &[],
            None,
            2,
            Some(&mut streaming_timing),
        )
        .expect("seed discovery streaming lower should produce segments");
    std::fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(streaming_lowered.len(), serial_lowered.len());
    for (streaming, serial) in streaming_lowered.iter().zip(&serial_lowered) {
        assert_eq!(streaming.next_seed, serial.next_seed);
        assert_eq!(
            streaming.segment.trace_instance_index,
            serial.segment.trace_instance_index
        );
        assert_eq!(
            streaming.segment.trace_source_prefix_rows,
            serial.segment.trace_source_prefix_rows
        );
        assert_eq!(
            streaming.segment.device_segment_material,
            serial.segment.device_segment_material
        );
        assert_eq!(streaming.segment.unit_values, serial.segment.unit_values);
        assert_eq!(streaming.segment.proof_values, serial.segment.proof_values);
    }
    assert_eq!(streaming_timing.parallel_lower_snapshot_replay_count(), 0);
    assert_eq!(
        streaming_timing.parallel_lower_stream_segment_count(),
        streaming_lowered.len()
    );
}

#[test]
#[cfg(feature = "cuda")]
fn guest_pc_trace_seed_discovery_streaming_device_lower_emits_ordered_segments() {
    let _env_lock = GUEST_PC_TRACE_ENV_LOCK
        .lock()
        .expect("guest PC trace env lock should not be poisoned");
    let _mirror_env = TestEnvVarGuard::set("LZVM_GUEST_PC_TRACE_SEED_MIRROR", "1");
    let dir = repo_temp_dir("guest-pc-seed-discovery-streaming-device-emit");
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
    let mut serial_pending = Vec::new();
    produce_guest_pc_trace_pending_slices(32, context, &[], layout.row_count(), |segment| {
        serial_pending.push(segment);
        Ok(())
    })
    .expect("serial pending slices should produce");
    let discovered = discover_guest_pc_trace_segment_seeds(32, context, &[], layout.row_count())
        .expect("seed discovery should scan the device fixture");
    let serial_lowered =
        lower_guest_pc_trace_seeded_pending_segments_with_workers(&layout, serial_pending, None, 2)
            .expect("serial pending segments should lower");
    let mut timing = GuestPcTraceStreamTiming::default();
    let mut emitted = Vec::new();
    lower_guest_pc_trace_seed_discovery_streaming_device_segments_emit_with_timing(
        &layout,
        &discovered.segments,
        context,
        &[],
        None,
        2,
        Some(&mut timing),
        |lowered| {
            emitted.push(lowered);
            Ok(())
        },
    )
    .expect("streaming discovery lower should emit ordered segments");
    std::fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(emitted.len(), serial_lowered.len());
    for (emitted, serial) in emitted.iter().zip(&serial_lowered) {
        assert_eq!(
            emitted.segment.trace_instance_index,
            serial.segment.trace_instance_index
        );
        assert_eq!(emitted.next_seed, serial.next_seed);
        assert_eq!(
            emitted.segment.device_segment_material,
            serial.segment.device_segment_material
        );
        assert_eq!(emitted.segment.unit_values, serial.segment.unit_values);
    }
    assert_eq!(timing.parallel_lower_emitted_count(), emitted.len());
    assert_eq!(timing.parallel_lower_stream_segment_count(), emitted.len());
    assert_eq!(timing.parallel_lower_snapshot_replay_count(), 0);
}

#[test]
fn guest_pc_trace_fcall_fixture_words_decode() {
    assert_eq!(
        decode_riscv_instruction(riscv_zisk_fcall_param(0, 1)),
        RiscvInstruction::ZiskFcallParam { port: 0, rs1: 1 }
    );
    assert_eq!(
        decode_riscv_instruction(riscv_zisk_fcall_invoke(
            crate::zisk_fcalls::ZISK_INPUT_READY_FCALL_ID
        )),
        RiscvInstruction::ZiskFcallInvoke {
            function_id: crate::zisk_fcalls::ZISK_INPUT_READY_FCALL_ID
        }
    );
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
fn copy_row_shape_counts_track_source_and_store_classes() {
    let mut timing = GuestPcTraceStreamTiming::default();
    let register_copy = zisk_main_base_instruction(
        0x8000_0100,
        ZiskMainSource::Immediate(0),
        ZiskMainSource::Register(1),
        ZiskMainOp::CopyB,
        ZiskMainStore::Register(2),
        4,
    );
    let memory_copy = zisk_main_base_instruction(
        0x8000_0104,
        ZiskMainSource::Immediate(0),
        ZiskMainSource::Memory(64),
        ZiskMainOp::CopyB,
        ZiskMainStore::Memory(128),
        4,
    );
    let indirect_copy = zisk_main_base_instruction(
        0x8000_0108,
        ZiskMainSource::Immediate(0),
        ZiskMainSource::Indirect(8),
        ZiskMainOp::CopyB,
        ZiskMainStore::None,
        4,
    );

    for instruction in [&register_copy, &memory_copy, &indirect_copy] {
        record_trace_lowered_row_shape(&mut timing, instruction);
    }

    assert_eq!(timing.trace_copy_row_count(), 3);
    assert_eq!(timing.trace_copy_memory_source_row_count(), 2);
    assert_eq!(timing.trace_copy_indirect_memory_row_count(), 1);
    assert_eq!(timing.trace_copy_register_store_row_count(), 1);
    assert_eq!(timing.trace_copy_memory_store_row_count(), 1);
    assert_eq!(timing.trace_copy_no_store_row_count(), 1);
    assert_eq!(timing.trace_copy_no_memory_row_count(), 1);
}

#[test]
fn row_shape_top_patterns_track_exact_source_store_mix() {
    let mut timing = GuestPcTraceStreamTiming::default();
    let hot_copy = zisk_main_base_instruction(
        0x8000_0200,
        ZiskMainSource::Immediate(0),
        ZiskMainSource::Register(1),
        ZiskMainOp::CopyB,
        ZiskMainStore::Register(2),
        4,
    );
    let second_copy = zisk_main_base_instruction(
        0x8000_0204,
        ZiskMainSource::Register(3),
        ZiskMainSource::Indirect(8),
        ZiskMainOp::CopyB,
        ZiskMainStore::Memory(128),
        4,
    );
    let cold_external = zisk_main_base_instruction(
        0x8000_0208,
        ZiskMainSource::Immediate(1),
        ZiskMainSource::Immediate(2),
        ZiskMainOp::Add,
        ZiskMainStore::None,
        4,
    );
    let fourth_shape = zisk_main_base_instruction(
        0x8000_020c,
        ZiskMainSource::Register(4),
        ZiskMainSource::Register(5),
        ZiskMainOp::Sub,
        ZiskMainStore::Register(6),
        4,
    );
    let late_hot = zisk_main_base_instruction(
        0x8000_0210,
        ZiskMainSource::Memory(256),
        ZiskMainSource::Immediate(7),
        ZiskMainOp::Add,
        ZiskMainStore::Memory(264),
        4,
    );

    for instruction in [
        &hot_copy,
        &second_copy,
        &hot_copy,
        &cold_external,
        &fourth_shape,
        &late_hot,
        &second_copy,
        &hot_copy,
        &late_hot,
        &late_hot,
        &late_hot,
    ] {
        record_trace_lowered_row_shape(&mut timing, instruction);
    }

    let top = timing.trace_row_shape_top_patterns();
    assert_eq!(
        top[0],
        (main_row_shape_pattern_id(&late_hot), 4),
        "late-arriving high-frequency row pattern should be counted exactly"
    );
    assert_eq!(
        top[1],
        (main_row_shape_pattern_id(&hot_copy), 3),
        "second most common exact row pattern should be retained"
    );
    assert_eq!(
        top[2],
        (main_row_shape_pattern_id(&second_copy), 2),
        "third most common exact row pattern should be retained"
    );
}

#[test]
fn source_value_kind_timing_counts_reads_and_durations() {
    let mut timing = GuestPcTraceStreamTiming::default();
    record_trace_report_source_read_timing(
        &mut timing,
        ZiskMainSource::Immediate(7),
        std::time::Duration::from_nanos(10),
        false,
    );
    record_trace_report_source_read_timing(
        &mut timing,
        ZiskMainSource::Register(1),
        std::time::Duration::from_nanos(20),
        false,
    );
    record_trace_report_source_read_timing(
        &mut timing,
        ZiskMainSource::Memory(64),
        std::time::Duration::from_nanos(30),
        false,
    );
    record_trace_report_source_read_timing(
        &mut timing,
        ZiskMainSource::Indirect(8),
        std::time::Duration::from_nanos(40),
        false,
    );
    record_trace_report_source_read_timing(
        &mut timing,
        ZiskMainSource::LastC,
        std::time::Duration::from_nanos(50),
        false,
    );

    assert_eq!(timing.trace_report_source_immediate_read_count(), 1);
    assert_eq!(timing.trace_report_source_register_read_count(), 1);
    assert_eq!(timing.trace_report_source_memory_read_count(), 1);
    assert_eq!(timing.trace_report_source_indirect_read_count(), 1);
    assert_eq!(timing.trace_report_source_last_c_read_count(), 1);
    assert_eq!(
        timing.trace_report_source_immediate_read_duration(),
        std::time::Duration::from_nanos(10)
    );
    assert_eq!(
        timing.trace_report_source_register_read_duration(),
        std::time::Duration::from_nanos(20)
    );
    assert_eq!(
        timing.trace_report_source_memory_read_duration(),
        std::time::Duration::from_nanos(30)
    );
    assert_eq!(
        timing.trace_report_source_indirect_read_duration(),
        std::time::Duration::from_nanos(40)
    );
    assert_eq!(
        timing.trace_report_source_last_c_read_duration(),
        std::time::Duration::from_nanos(50)
    );
}

#[test]
fn copy_source_value_kind_timing_is_tracked_separately() {
    let mut timing = GuestPcTraceStreamTiming::default();
    record_trace_report_source_read_timing(
        &mut timing,
        ZiskMainSource::Memory(64),
        std::time::Duration::from_nanos(30),
        true,
    );
    record_trace_report_source_read_timing(
        &mut timing,
        ZiskMainSource::Indirect(8),
        std::time::Duration::from_nanos(40),
        true,
    );
    record_trace_report_source_read_timing(
        &mut timing,
        ZiskMainSource::Register(1),
        std::time::Duration::from_nanos(20),
        true,
    );

    assert_eq!(timing.trace_report_source_memory_read_count(), 1);
    assert_eq!(timing.trace_report_source_indirect_read_count(), 1);
    assert_eq!(timing.trace_copy_source_memory_read_count(), 1);
    assert_eq!(timing.trace_copy_source_indirect_read_count(), 1);
    assert_eq!(
        timing.trace_copy_source_memory_read_duration(),
        std::time::Duration::from_nanos(30)
    );
    assert_eq!(
        timing.trace_copy_source_indirect_read_duration(),
        std::time::Duration::from_nanos(40)
    );
}

#[test]
fn rejects_add256_precompile_memory_access_address_mismatch() {
    let mut report = add256_report();
    report
        .precompile_effects_mut()
        .expect("Add256 report should carry precompile effects")
        .memory_accesses[4]
        .address += 8;

    let error = validate_main_precompile_memory_accesses(
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

    validate_main_precompile_memory_accesses_if_required(
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
    report.replace_precompile_effects(GuestPrecompileReportEffects::from_parts(
        vec![memory_read(64, 7)].into(),
        None,
    ));
    let instruction = lower_guest_report(&report).expect("report should lower");

    let error = validate_main_precompile_memory_accesses_if_required(
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
    report.replace_precompile_effects(GuestPrecompileReportEffects::from_parts(
        Vec::new().into(),
        Some(1),
    ));
    let instruction = lower_guest_report(&report).expect("report should lower");

    let error = validate_main_precompile_memory_accesses_if_required(
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
    assert!(!main_precompile_memory_validation_required(
        &instruction,
        ZiskMainReportEffects::from_report(&report),
    ));

    let mut non_precompile_with_access = addi_report();
    non_precompile_with_access.replace_precompile_effects(
        GuestPrecompileReportEffects::from_parts(vec![memory_read(64, 7)].into(), None),
    );
    let instruction = lower_guest_report(&non_precompile_with_access).expect("report should lower");
    assert!(main_precompile_memory_validation_required(
        &instruction,
        ZiskMainReportEffects::from_report(&non_precompile_with_access),
    ));

    let mut precompile_without_access = add256_report();
    precompile_without_access.replace_precompile_effects(GuestPrecompileReportEffects::from_parts(
        Vec::new().into(),
        Some(1),
    ));
    let instruction = lower_guest_report(&precompile_without_access).expect("report should lower");
    assert!(main_precompile_memory_validation_required(
        &instruction,
        ZiskMainReportEffects::from_report(&precompile_without_access),
    ));
}

#[test]
fn builds_zisk_main_segment_trace_without_serialized_roundtrip() {
    let _env_lock = GUEST_PC_TRACE_ENV_LOCK
        .lock()
        .expect("guest PC trace env lock should not be poisoned");
    let _detail_env = TestEnvVarGuard::unset("LZVM_GUEST_TRACE_DETAIL_TIMING");
    let _shape_env = TestEnvVarGuard::unset("LZVM_GUEST_TRACE_SHAPE_TIMING");
    let _shape_sample_env = TestEnvVarGuard::unset("LZVM_GUEST_TRACE_SHAPE_TIMING_SAMPLE_STRIDE");
    let unit = sample_main_trace_unit_rows(4);
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");
    let reports = [
        addi_report_at(0x8000_0000, 3, 0, 7, 7),
        GuestMachineReport {
            address_and_instruction_len:
                crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0004, 4),
            instruction: RiscvInstruction::Store {
                kind: RiscvStoreKind::Sd,
                rs1: 0,
                rs2: 3,
                offset: 16,
            },
            next_pc: 0x8000_0008,
            register_write_value: GuestRegisterWriteValue::default(),
            memory_accesses: vec![memory_write(16, 7)].into(),
        },
        GuestMachineReport {
            address_and_instruction_len:
                crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0008, 4),
            instruction: RiscvInstruction::ZiskDmaPrepare {
                kind: RiscvDmaKind::Memcpy,
                rs1: 5,
            },
            next_pc: 0x8000_000c,
            register_write_value: GuestRegisterWriteValue::default(),
            memory_accesses: Vec::new().into(),
        },
        GuestMachineReport {
            address_and_instruction_len:
                crate::guest_machine::pack_report_address_and_instruction_len(0x8000_000c, 4),
            instruction: RiscvInstruction::Op {
                kind: RiscvOpKind::Add,
                rd: 10,
                rs1: 5,
                rs2: 3,
            },
            next_pc: 0x8000_0010,
            register_write_value: GuestRegisterWriteValue::new(11),
            memory_accesses: Vec::new().into(),
        },
    ];
    let mut timing = GuestPcTraceStreamTiming::default();
    let mut initial_state = ZiskMainTraceState::new();
    initial_state.registers[5] = 11;

    let written = build_layout_zisk_main_trace_segment(
        &layout,
        &reports,
        reports[3].next_pc,
        &initial_state,
        None,
        ZiskMainTraceSegmentInfo {
            trace_instance_index: 0,
            is_last_segment: true,
            previous_c: 0,
        },
        Some(&mut timing),
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
        Some(Felt::from_canonical(reports[0].address()).expect("canonical pc"))
    );

    let mut bytes = vec![0; written.output.produced_len];
    serialize_trace_to_output(trace, written.output.produced_len, &mut bytes)
        .expect("trace should serialize");
    let parsed = parse_witness_trace(&bytes, layout.row_count(), layout.column_count())
        .expect("serialized trace should parse");
    assert_eq!(&parsed, trace);
    let fallback_instruction = ZiskMainInstruction {
        pc: reports[1].address(),
        a: ZiskMainSource::Immediate(0),
        b: ZiskMainSource::Register(3),
        op: ZiskMainOp::CopyB,
        store: ZiskMainStore::Indirect(16),
        store_pc: false,
        set_pc: false,
        jmp_offset1: 4,
        jmp_offset2: 4,
        ind_width: 8,
        m32: false,
        is_external_op: false,
        is_precompiled: false,
    };
    assert_eq!(timing.trace_main_report_fast_path_count(), 2);
    assert_eq!(timing.trace_main_report_simple_copy_fast_path_count(), 1);
    assert_eq!(timing.trace_main_report_no_memory_fast_path_count(), 1);
    assert_eq!(timing.trace_main_report_generic_fallback_count(), 1);
    assert_eq!(
        timing.trace_main_report_generic_fallback_shape_top_patterns()[0],
        (main_row_shape_pattern_id(&fallback_instruction), 1)
    );
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
    let report =
        GuestMachineReport {
            address_and_instruction_len:
                crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0000, 4),
            instruction: RiscvInstruction::Jal { rd: 1, offset: 16 },
            next_pc: 0x8000_0010,
            register_write_value: GuestRegisterWriteValue::new(0x8000_0004),
            memory_accesses: Vec::new().into(),
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
    let report =
        GuestMachineReport {
            address_and_instruction_len:
                crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0000, 4),
            instruction: RiscvInstruction::Store {
                kind: RiscvStoreKind::Sd,
                rs1: 1,
                rs2: 2,
                offset: 8,
            },
            next_pc: 0x8000_0004,
            register_write_value: GuestRegisterWriteValue::default(),
            memory_accesses: vec![GuestMemoryAccess {
                kind: GuestMemoryAccessKind::Write,
                address: 0x1008,
                byte_len: 8,
                value: 0x1234_5678_9abc_def0,
            }]
            .into(),
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
    let taken =
        GuestMachineReport {
            address_and_instruction_len:
                crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0000, 4),
            instruction: RiscvInstruction::Branch {
                kind: RiscvBranchKind::Beq,
                rs1: 1,
                rs2: 2,
                offset: 16,
            },
            next_pc: 0x8000_0010,
            register_write_value: GuestRegisterWriteValue::default(),
            memory_accesses: Vec::new().into(),
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
fn replay_only_runner_seed_snapshot_elides_runner_report_buffer() {
    let _env_lock = GUEST_PC_TRACE_ENV_LOCK
        .lock()
        .expect("guest PC trace env lock should not be poisoned");
    let _parallel_env = TestEnvVarGuard::set("LZVM_GUEST_PC_TRACE_PARALLEL_LOWER", "1");
    let _worker_env = TestEnvVarGuard::set("LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_WORKERS", "2");
    let _replay_only_env =
        TestEnvVarGuard::set("LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_REPLAY_ONLY", "1");
    let dir = repo_temp_dir("guest-pc-replay-only-runner-report-elision");
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

    let produced = produce_guest_pc_trace_pending_slices(
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
    .expect("replay-only pending slices should produce");
    std::fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert!(pending.len() >= 3);
    assert!(pending.iter().all(|segment| segment.reports_elided));
    assert!(pending.iter().all(|segment| segment.reports.is_empty()));
    assert!(pending.iter().all(|segment| segment.report_count > 0));
    assert!(pending
        .iter()
        .all(|segment| segment.replay_snapshot.is_some()));
    assert_eq!(
        produced.timing.segment_replay_snapshot_capture_count(),
        pending.len()
    );
    assert!(produced.timing.segment_replay_snapshot_capture_duration() > std::time::Duration::ZERO);
    assert_eq!(produced.timing.trace_runner_report_buffer_capacity(), 0);
    assert!(produced.timing.seed_direct_lift_attempt_count() > 0);
    assert_eq!(
        produced.timing.seed_direct_lift_attempt_count(),
        produced.timing.seed_direct_lift_success_count()
    );
}

#[test]
fn elided_runner_seed_snapshot_does_not_retain_alu_last_report() {
    let _env_lock = GUEST_PC_TRACE_ENV_LOCK
        .lock()
        .expect("guest PC trace env lock should not be poisoned");
    let dir = repo_temp_dir("guest-pc-elided-runner-alu-last-report");
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
    let context = WitnessComputeContext {
        guest_image: Some(&guest_image),
        guest_image_info: Some(&guest_image_info),
        trace_layout: Some(&layout),
    };
    let mut serial_pending = Vec::new();
    produce_guest_pc_trace_pending_slices(16, context, &[], layout.row_count(), |segment| {
        serial_pending.push(segment);
        Ok(())
    })
    .expect("serial pending slices should produce");
    let expected = serial_pending
        .into_iter()
        .next()
        .expect("fixture should produce a first segment");
    let current_seed = ZiskMainSegmentSeed::new();
    let serial_lowered = lower_guest_pc_trace_seeded_pending_segment_with_output_mode(
        &layout,
        &expected,
        &current_seed,
        None,
        false,
        None,
    )
    .expect("serial pending segment should lower");

    let (mut memory, mut state, mut fcall_handler) =
        load_guest_pc_trace_machine(context, &[]).expect("guest trace machine should load");
    let mut boundary_snapshot = ZiskMainRunnerBoundarySnapshot::new(&current_seed);
    let slice = run_guest_pc_trace_segment_slice_with_elided_reports_and_boundary_snapshot(
        &mut memory,
        &mut state,
        &mut fcall_handler,
        16,
        layout.row_count(),
        &mut boundary_snapshot,
    )
    .expect("elided segment should run");
    std::fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(
        slice.executed_instructions,
        expected.executed_instruction_count
    );
    assert_eq!(slice.trace_rows, expected.trace_row_count);
    assert_eq!(
        slice.status,
        GuestMachineTraceSliceStatus::Paused {
            pc: expected.terminal_pc,
            instruction: expected
                .lookahead_instruction
                .expect("paused segment should have lookahead"),
        }
    );
    assert!(slice.reports.is_empty());
    assert_eq!(slice.report_count, expected.report_count);

    let segment = ZiskMainTraceSegmentInfo {
        trace_instance_index: expected.trace_instance_index,
        is_last_segment: expected.is_last_segment,
        previous_c: current_seed.previous_c,
    };
    let lifted = try_lift_zisk_main_next_segment_seed_from_runner_boundary_snapshot(
        layout.row_count(),
        segment,
        ZiskMainRunnerBoundarySeedInput {
            reports: &[],
            report_count: slice.report_count,
            last_report_shape: slice.last_report_shape,
            lookahead_instruction: expected.lookahead_instruction,
            runner_state: &state,
            current_seed: &current_seed,
            boundary_snapshot: &boundary_snapshot,
        },
    )
    .expect("shape-only ALU seed lift should evaluate")
    .expect("shape-only ALU seed lift should succeed");

    assert_eq!(lifted, serial_lowered.next_seed);
}

#[test]
fn elided_runner_seed_snapshot_does_not_return_amo_last_report_after_snapshot() {
    let _env_lock = GUEST_PC_TRACE_ENV_LOCK
        .lock()
        .expect("guest PC trace env lock should not be poisoned");
    let dir = repo_temp_dir("guest-pc-elided-runner-amo-last-report");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture directory should be created");
    let guest_image = dir.join("guest.elf");
    let guest_image_bytes = sample_guest_image_with_words(&[riscv_amo_add_d(1, 1, 2), 0x0000_0073]);
    std::fs::write(&guest_image, &guest_image_bytes).expect("guest image should be written");
    let guest_image_info = parse_guest_image(&guest_image_bytes).expect("guest image should parse");
    let unit = sample_unit_with_zisk_main_columns_rows(4);
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");
    let context = WitnessComputeContext {
        guest_image: Some(&guest_image),
        guest_image_info: Some(&guest_image_info),
        trace_layout: Some(&layout),
    };
    let current_seed = {
        let mut seed = ZiskMainSegmentSeed::new();
        seed.initial_state.registers[1] = 0x9000;
        seed.initial_state.registers[2] = 0x10;
        seed
    };
    let (mut memory, mut state, mut fcall_handler) =
        load_guest_pc_trace_machine(context, &[]).expect("guest trace machine should load");
    memory
        .map_initialized_range(0x9000, 0x1234_5678_9abc_def0_u64.to_le_bytes().to_vec())
        .expect("test AMO data should map");
    state
        .set_register(1, 0x9000)
        .expect("test register should be writable");
    state
        .set_register(2, 0x10)
        .expect("test register should be writable");
    let mut boundary_snapshot = ZiskMainRunnerBoundarySnapshot::new(&current_seed);

    let slice = run_guest_pc_trace_segment_slice_with_elided_reports_and_boundary_snapshot(
        &mut memory,
        &mut state,
        &mut fcall_handler,
        16,
        layout.row_count(),
        &mut boundary_snapshot,
    )
    .expect("elided AMO segment should run");
    std::fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(slice.report_count, 1);
    assert_eq!(slice.trace_rows, 4);
    assert!(slice.reports.is_empty());
    assert_eq!(
        boundary_snapshot
            .internal_memory
            .get(zisk_internal_register_address_u64(ZISK_AMO_TEMP_REGISTER)),
        Some(0x1234_5678_9abc_def0)
    );
}

#[test]
fn live_report_chunk_runner_matches_serial_slice_without_returning_reports() {
    let _env_lock = GUEST_PC_TRACE_ENV_LOCK
        .lock()
        .expect("guest PC trace env lock should not be poisoned");
    let _cache_env = TestEnvVarGuard::set("LZVM_GUEST_TRACE_RUNNER_CACHE_STATS", "1");
    let dir = repo_temp_dir("guest-pc-live-report-chunk-runner");
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
    let unit = sample_unit_with_zisk_main_columns_rows(2);
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");
    let context = WitnessComputeContext {
        guest_image: Some(&guest_image),
        guest_image_info: Some(&guest_image_info),
        trace_layout: Some(&layout),
    };
    let (mut serial_memory, mut serial_state, mut serial_fcall_handler) =
        load_guest_pc_trace_machine(context, &[]).expect("serial guest trace machine should load");
    let (mut live_memory, mut live_state, mut live_fcall_handler) =
        load_guest_pc_trace_machine(context, &[]).expect("live guest trace machine should load");

    let serial = run_guest_pc_trace_segment_slice(
        &mut serial_memory,
        &mut serial_state,
        &mut serial_fcall_handler,
        32,
        layout.row_count(),
    )
    .expect("serial segment should run");
    let mut live_reports = Vec::new();
    let mut instruction_cache = GuestInstructionCache::default();
    let mut live_timing = GuestPcTraceStreamTiming::default();
    let live = run_guest_pc_trace_segment_slice_with_live_report_chunks(
        &mut live_memory,
        &mut live_state,
        &mut live_fcall_handler,
        32,
        layout.row_count(),
        &mut instruction_cache,
        None,
        true,
        &mut live_timing,
        |report| {
            live_reports.push(report);
            Ok(())
        },
    )
    .expect("live report chunk segment should run");
    std::fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(live.executed_instructions, serial.executed_instructions);
    assert_eq!(live.trace_rows, serial.trace_rows);
    assert_eq!(live.status, serial.status);
    assert_eq!(
        live_timing.runner_advance_fast_path_count()
            + live_timing.runner_advance_generic_fallback_count(),
        live.report_count
    );
    assert!(live_timing.runner_instruction_cache_miss_count() > 0);
    assert!(
        live_timing.runner_instruction_cache_hit_count()
            + live_timing.runner_instruction_cache_miss_count()
            >= live.report_count
    );
    assert_eq!(live.report_count, serial.report_count);
    assert_eq!(live.reports, Vec::new());
    assert_eq!(live.report_capacity, 0);
    assert_eq!(live_reports, serial.reports);
    assert_eq!(live_memory, serial_memory);
    assert_eq!(live_state, serial_state);
}

#[test]
fn live_pending_segment_messages_reassemble_to_serial_lower_output() {
    let _env_lock = GUEST_PC_TRACE_ENV_LOCK
        .lock()
        .expect("guest PC trace env lock should not be poisoned");
    let _mirror_env = TestEnvVarGuard::set("LZVM_GUEST_PC_TRACE_SEED_MIRROR", "1");
    let dir = repo_temp_dir("guest-pc-live-pending-messages");
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
    let unit = sample_unit_with_zisk_main_columns_rows(2);
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");
    let context = WitnessComputeContext {
        guest_image: Some(&guest_image),
        guest_image_info: Some(&guest_image_info),
        trace_layout: Some(&layout),
    };
    let mut serial_pending = Vec::new();
    produce_guest_pc_trace_pending_slices(32, context, &[], layout.row_count(), |segment| {
        serial_pending.push(segment);
        Ok(())
    })
    .expect("serial pending slices should produce");
    let expected = serial_pending
        .into_iter()
        .next()
        .expect("fixture should produce a first segment");

    let (mut live_memory, mut live_state, mut live_fcall_handler) =
        load_guest_pc_trace_machine(context, &[]).expect("live guest trace machine should load");
    let mut messages = Vec::new();
    let mut instruction_cache = GuestInstructionCache::default();
    let mut live_timing = GuestPcTraceStreamTiming::default();
    let emitted = emit_guest_pc_trace_live_pending_segment_messages(
        &mut live_memory,
        &mut live_state,
        &mut live_fcall_handler,
        expected.trace_instance_index,
        expected.runner_remaining_instruction_limit,
        layout.row_count(),
        expected.seed.clone(),
        None,
        None,
        &mut instruction_cache,
        false,
        &mut live_timing,
        false,
        1,
        |message| {
            messages.push(message);
            Ok(())
        },
    )
    .expect("live pending segment messages should emit");
    std::fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(
        emitted.executed_instruction_count,
        expected.executed_instruction_count
    );
    assert_eq!(emitted.trace_row_count, expected.trace_row_count);
    assert_eq!(emitted.terminal_pc, expected.terminal_pc);
    assert_eq!(emitted.is_last_segment, expected.is_last_segment);
    assert!(!emitted.needs_terminal_segment);
    assert!(
        matches!(
            messages.first(),
            Some(GuestPcTracePendingSegmentMessage::ReportChunk(_))
        ),
        "live pending messages should emit report chunks before the segment metadata is known"
    );

    let mut pending_chunks = BTreeMap::new();
    let mut chunk_timing = GuestPcTraceStreamTiming::default();
    let mut started = None;
    let mut rebuilt = None;
    for message in messages {
        match message {
            GuestPcTracePendingSegmentMessage::ReportChunk(chunk) => {
                receive_guest_pc_trace_pending_report_chunk(
                    *chunk,
                    &mut pending_chunks,
                    &mut chunk_timing,
                );
            }
            GuestPcTracePendingSegmentMessage::SegmentStarted(pending) => {
                assert!(started.replace(*pending).is_none());
            }
            GuestPcTracePendingSegmentMessage::SegmentFinished(finish) => {
                let pending = started
                    .take()
                    .expect("segment finish should follow segment metadata");
                assert_eq!(finish.trace_instance_index, pending.trace_instance_index);
                rebuilt = Some(
                    finish_guest_pc_trace_chunked_pending_segment(pending, &mut pending_chunks)
                        .expect("live chunks should reassemble"),
                );
            }
            _ => panic!("live segment helper should only emit chunked segment messages"),
        }
    }
    validate_guest_pc_trace_no_pending_report_chunks(&pending_chunks)
        .expect("all live chunks should be consumed");
    let rebuilt = rebuilt.expect("live messages should include a segment finish");

    assert_eq!(rebuilt.trace_instance_index, expected.trace_instance_index);
    assert_eq!(
        rebuilt.executed_instruction_count,
        expected.executed_instruction_count
    );
    assert_eq!(rebuilt.trace_row_count, expected.trace_row_count);
    assert_eq!(
        rebuilt.runner_remaining_instruction_limit,
        expected.runner_remaining_instruction_limit
    );
    assert_eq!(rebuilt.report_count, expected.report_count);
    assert_eq!(rebuilt.reports, expected.reports);
    assert_eq!(rebuilt.reports_elided, expected.reports_elided);
    assert_eq!(rebuilt.terminal_pc, expected.terminal_pc);
    assert_eq!(
        rebuilt.lookahead_instruction,
        expected.lookahead_instruction
    );
    assert_eq!(rebuilt.is_last_segment, expected.is_last_segment);
    assert_eq!(rebuilt.seed, expected.seed);

    let seed = expected
        .seed
        .as_deref()
        .expect("seed mirror should attach a segment seed");
    let serial_lowered = lower_guest_pc_trace_seeded_pending_segment_with_output_mode(
        &layout, &expected, seed, None, false, None,
    )
    .expect("serial pending segment should lower");
    let live_lowered = lower_guest_pc_trace_seeded_pending_segment_with_output_mode(
        &layout, &rebuilt, seed, None, false, None,
    )
    .expect("rebuilt live pending segment should lower");
    assert_eq!(live_lowered.next_seed, serial_lowered.next_seed);
    assert_eq!(
        live_lowered.segment.trace_instance_index,
        serial_lowered.segment.trace_instance_index
    );
    assert_eq!(
        live_lowered.segment.trace_source_prefix_rows,
        serial_lowered.segment.trace_source_prefix_rows
    );
    #[cfg(feature = "cuda")]
    assert_eq!(
        live_lowered.segment.device_segment_material,
        serial_lowered.segment.device_segment_material
    );
    assert_eq!(live_lowered.segment.trace, serial_lowered.segment.trace);
    assert_eq!(
        live_lowered.segment.unit_values,
        serial_lowered.segment.unit_values
    );
}

#[test]
fn live_pending_segment_boundary_snapshot_lifts_next_seed_without_retained_reports() {
    let _env_lock = GUEST_PC_TRACE_ENV_LOCK
        .lock()
        .expect("guest PC trace env lock should not be poisoned");
    let _mirror_env = TestEnvVarGuard::set("LZVM_GUEST_PC_TRACE_SEED_MIRROR", "1");
    let dir = repo_temp_dir("guest-pc-live-boundary-seed");
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
    let unit = sample_unit_with_zisk_main_columns_rows(2);
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");
    let context = WitnessComputeContext {
        guest_image: Some(&guest_image),
        guest_image_info: Some(&guest_image_info),
        trace_layout: Some(&layout),
    };
    let mut serial_pending = Vec::new();
    produce_guest_pc_trace_pending_slices(32, context, &[], layout.row_count(), |segment| {
        serial_pending.push(segment);
        Ok(())
    })
    .expect("serial pending slices should produce");
    let expected = serial_pending
        .into_iter()
        .next()
        .expect("fixture should produce a first segment");
    assert!(!expected.is_last_segment);
    let current_seed = expected
        .seed
        .as_deref()
        .expect("seed mirror should attach a segment seed")
        .clone();
    let serial_lowered = lower_guest_pc_trace_seeded_pending_segment_with_output_mode(
        &layout,
        &expected,
        &current_seed,
        None,
        false,
        None,
    )
    .expect("serial pending segment should lower");

    let (mut live_memory, mut live_state, mut live_fcall_handler) =
        load_guest_pc_trace_machine(context, &[]).expect("live guest trace machine should load");
    let mut boundary_snapshot = ZiskMainRunnerBoundarySnapshot::new(&current_seed);
    let mut messages = Vec::new();
    let mut instruction_cache = GuestInstructionCache::default();
    let mut live_timing = GuestPcTraceStreamTiming::default();
    let emitted = emit_guest_pc_trace_live_pending_segment_messages(
        &mut live_memory,
        &mut live_state,
        &mut live_fcall_handler,
        expected.trace_instance_index,
        expected.runner_remaining_instruction_limit,
        layout.row_count(),
        expected.seed.clone(),
        None,
        Some(&mut boundary_snapshot),
        &mut instruction_cache,
        false,
        &mut live_timing,
        false,
        1,
        |message| {
            messages.push(message);
            Ok(())
        },
    )
    .expect("live pending segment messages should emit with a boundary snapshot");
    std::fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert!(matches!(
        messages.first(),
        Some(GuestPcTracePendingSegmentMessage::ReportChunk(_))
    ));
    assert_eq!(emitted.report_count, expected.report_count);
    assert_eq!(
        emitted.lookahead_instruction,
        expected.lookahead_instruction
    );

    let segment = ZiskMainTraceSegmentInfo {
        trace_instance_index: expected.trace_instance_index,
        is_last_segment: expected.is_last_segment,
        previous_c: current_seed.previous_c,
    };
    let lifted = try_lift_zisk_main_next_segment_seed_from_runner_boundary_snapshot(
        layout.row_count(),
        segment,
        ZiskMainRunnerBoundarySeedInput {
            reports: &[],
            report_count: emitted.report_count,
            last_report_shape: emitted.last_report_shape,
            lookahead_instruction: emitted.lookahead_instruction,
            runner_state: &live_state,
            current_seed: &current_seed,
            boundary_snapshot: &boundary_snapshot,
        },
    )
    .expect("live boundary seed lift should evaluate")
    .expect("live boundary seed lift should succeed");

    assert_eq!(lifted, serial_lowered.next_seed);
}

#[test]
fn live_pending_segment_can_emit_stream_start_before_chunks() {
    let _env_lock = GUEST_PC_TRACE_ENV_LOCK
        .lock()
        .expect("guest PC trace env lock should not be poisoned");
    let _mirror_env = TestEnvVarGuard::set("LZVM_GUEST_PC_TRACE_SEED_MIRROR", "1");
    let dir = repo_temp_dir("guest-pc-live-early-stream-start");
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
    let unit = sample_unit_with_zisk_main_columns_rows(2);
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");
    let context = WitnessComputeContext {
        guest_image: Some(&guest_image),
        guest_image_info: Some(&guest_image_info),
        trace_layout: Some(&layout),
    };
    let mut serial_pending = Vec::new();
    produce_guest_pc_trace_pending_slices(32, context, &[], layout.row_count(), |segment| {
        serial_pending.push(segment);
        Ok(())
    })
    .expect("serial pending slices should produce");
    let expected = serial_pending
        .into_iter()
        .next()
        .expect("fixture should produce a first segment");
    let expected_seed = expected
        .seed
        .as_deref()
        .expect("seed mirror should attach a segment seed")
        .clone();

    let (mut live_memory, mut live_state, mut live_fcall_handler) =
        load_guest_pc_trace_machine(context, &[]).expect("live guest trace machine should load");
    let mut messages = Vec::new();
    let mut instruction_cache = GuestInstructionCache::default();
    let mut live_timing = GuestPcTraceStreamTiming::default();
    emit_guest_pc_trace_live_pending_segment_messages(
        &mut live_memory,
        &mut live_state,
        &mut live_fcall_handler,
        expected.trace_instance_index,
        expected.runner_remaining_instruction_limit,
        layout.row_count(),
        expected.seed.clone(),
        None,
        None,
        &mut instruction_cache,
        false,
        &mut live_timing,
        true,
        1,
        |message| {
            messages.push(message);
            Ok(())
        },
    )
    .expect("live pending segment messages should emit early stream start");
    std::fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    let Some(GuestPcTracePendingSegmentMessage::SegmentStreamStarted(start)) = messages.first()
    else {
        panic!("live pending messages should emit stream start before report chunks");
    };
    assert_eq!(start.trace_instance_index, expected.trace_instance_index);
    assert_eq!(
        start.runner_remaining_instruction_limit,
        expected.runner_remaining_instruction_limit
    );
    assert_eq!(
        start.seed.as_deref(),
        Some(&expected_seed),
        "early stream start should carry the segment seed"
    );
    assert!(matches!(
        messages.get(1),
        Some(GuestPcTracePendingSegmentMessage::ReportChunk(_))
    ));
}

#[test]
fn live_report_chunk_producer_preserves_segment_output_when_enabled() {
    let _env_lock = GUEST_PC_TRACE_ENV_LOCK
        .lock()
        .expect("guest PC trace env lock should not be poisoned");
    let _live_env = TestEnvVarGuard::set("LZVM_GUEST_PC_TRACE_LIVE_REPORT_CHUNKS", "1");
    let _chunk_capacity_env =
        TestEnvVarGuard::set("LZVM_GUEST_PC_TRACE_REPORT_CHUNK_CAPACITY", "1");
    let dir = repo_temp_dir("guest-pc-live-report-producer");
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
    let unit = sample_unit_with_zisk_main_columns_rows(2);
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");
    let context = WitnessComputeContext {
        guest_image: Some(&guest_image),
        guest_image_info: Some(&guest_image_info),
        trace_layout: Some(&layout),
    };
    let expected =
        compute_guest_pc_trace_segments(32, context, &[]).expect("serial segments should compute");
    let mut emitted = Vec::new();
    let produced = produce_guest_pc_trace_segments(32, context, &[], None, |segment| {
        emitted.push(segment);
        Ok(())
    })
    .expect("live report chunk producer should produce segments");
    std::fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(emitted.len(), expected.len());
    assert!(
        produced.timing.trace_report_chunk_sent_count() > 0,
        "live report chunk producer should emit report chunks"
    );
    assert_eq!(produced.timing.trace_stream_start_sent_count(), 0);
    assert_eq!(
        produced.timing.trace_report_chunk_sent_count(),
        produced.timing.trace_report_chunk_received_count()
    );
    assert_eq!(
        produced.timing.trace_report_chunk_report_count(),
        expected
            .iter()
            .map(|segment| segment.trace_source_prefix_rows)
            .sum::<usize>()
    );
    for (emitted, expected) in emitted.iter().zip(expected.iter()) {
        assert_eq!(emitted.trace_instance_index, expected.trace_instance_index);
        assert_eq!(
            emitted.trace_source_prefix_rows,
            expected.trace_source_prefix_rows
        );
        #[cfg(feature = "cuda")]
        assert_eq!(
            emitted.device_segment_material,
            expected.device_segment_material
        );
        assert_eq!(emitted.trace, expected.trace);
        assert_eq!(emitted.unit_values, expected.unit_values);
    }
}

#[test]
fn live_report_chunk_producer_supports_trusted_runner_seed_snapshot() {
    let _env_lock = GUEST_PC_TRACE_ENV_LOCK
        .lock()
        .expect("guest PC trace env lock should not be poisoned");
    let _live_env = TestEnvVarGuard::set("LZVM_GUEST_PC_TRACE_LIVE_REPORT_CHUNKS", "1");
    let _chunk_capacity_env =
        TestEnvVarGuard::set("LZVM_GUEST_PC_TRACE_REPORT_CHUNK_CAPACITY", "1");
    let _snapshot_env = TestEnvVarGuard::set("LZVM_GUEST_PC_TRACE_RUNNER_SEED_SNAPSHOT", "1");
    let _trusted_env =
        TestEnvVarGuard::set("LZVM_GUEST_PC_TRACE_RUNNER_SEED_SNAPSHOT_TRUSTED", "1");
    let dir = repo_temp_dir("guest-pc-live-report-trusted-seed");
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
    let unit = sample_unit_with_zisk_main_columns_rows(2);
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");
    let context = WitnessComputeContext {
        guest_image: Some(&guest_image),
        guest_image_info: Some(&guest_image_info),
        trace_layout: Some(&layout),
    };
    let expected =
        compute_guest_pc_trace_segments(32, context, &[]).expect("serial segments should compute");
    let mut emitted = Vec::new();
    let produced = produce_guest_pc_trace_segments(32, context, &[], None, |segment| {
        emitted.push(segment);
        Ok(())
    })
    .expect("live report chunk producer should support trusted runner seeds");
    std::fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(emitted.len(), expected.len());
    assert!(produced.timing.trace_report_chunk_sent_count() > 0);
    assert_eq!(
        produced.timing.trace_report_chunk_sent_count(),
        produced.timing.trace_report_chunk_received_count()
    );
    assert_eq!(
        produced.timing.seed_direct_lift_attempt_count(),
        produced.timing.seed_direct_lift_success_count()
    );
    assert!(produced.timing.seed_direct_lift_success_count() > 0);
    for (emitted, expected) in emitted.iter().zip(expected.iter()) {
        assert_eq!(emitted.trace_instance_index, expected.trace_instance_index);
        assert_eq!(
            emitted.trace_source_prefix_rows,
            expected.trace_source_prefix_rows
        );
        #[cfg(feature = "cuda")]
        assert_eq!(
            emitted.device_segment_material,
            expected.device_segment_material
        );
        assert_eq!(emitted.trace, expected.trace);
        assert_eq!(emitted.unit_values, expected.unit_values);
    }
}

#[test]
fn live_report_chunk_producer_records_stream_start_when_enabled() {
    let _env_lock = GUEST_PC_TRACE_ENV_LOCK
        .lock()
        .expect("guest PC trace env lock should not be poisoned");
    let _live_env = TestEnvVarGuard::set("LZVM_GUEST_PC_TRACE_LIVE_REPORT_CHUNKS", "1");
    let _stream_start_env = TestEnvVarGuard::set("LZVM_GUEST_PC_TRACE_LIVE_STREAM_START", "1");
    let _chunk_capacity_env =
        TestEnvVarGuard::set("LZVM_GUEST_PC_TRACE_REPORT_CHUNK_CAPACITY", "1");
    let _snapshot_env = TestEnvVarGuard::set("LZVM_GUEST_PC_TRACE_RUNNER_SEED_SNAPSHOT", "1");
    let _trusted_env =
        TestEnvVarGuard::set("LZVM_GUEST_PC_TRACE_RUNNER_SEED_SNAPSHOT_TRUSTED", "1");
    let dir = repo_temp_dir("guest-pc-live-stream-start-producer");
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
    let unit = sample_unit_with_zisk_main_columns_rows(2);
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");
    let context = WitnessComputeContext {
        guest_image: Some(&guest_image),
        guest_image_info: Some(&guest_image_info),
        trace_layout: Some(&layout),
    };
    let expected =
        compute_guest_pc_trace_segments(32, context, &[]).expect("serial segments should compute");
    let mut emitted = Vec::new();
    let produced = produce_guest_pc_trace_segments(32, context, &[], None, |segment| {
        emitted.push(segment);
        Ok(())
    })
    .expect("live report chunk producer should support early stream starts");
    std::fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(emitted.len(), expected.len());
    assert_eq!(
        produced.timing.trace_stream_start_sent_count(),
        expected.len(),
        "stream-start messages should be sent once per trace segment"
    );
    assert!(produced.timing.trace_report_chunk_sent_count() > 0);
    assert_eq!(
        produced.timing.trace_report_chunk_sent_count(),
        produced.timing.trace_report_chunk_received_count()
    );
    for (emitted, expected) in emitted.iter().zip(expected.iter()) {
        assert_eq!(emitted.trace_instance_index, expected.trace_instance_index);
        assert_eq!(
            emitted.trace_source_prefix_rows,
            expected.trace_source_prefix_rows
        );
        #[cfg(feature = "cuda")]
        assert_eq!(
            emitted.device_segment_material,
            expected.device_segment_material
        );
        assert_eq!(emitted.trace, expected.trace);
        assert_eq!(emitted.unit_values, expected.unit_values);
    }
}

#[test]
fn live_report_chunk_parallel_lower_matches_serial_output() {
    let _env_lock = GUEST_PC_TRACE_ENV_LOCK
        .lock()
        .expect("guest PC trace env lock should not be poisoned");
    let _seed_mirror_env = TestEnvVarGuard::unset("LZVM_GUEST_PC_TRACE_SEED_MIRROR");
    let _segment_replay_env = TestEnvVarGuard::unset("LZVM_GUEST_PC_TRACE_SEGMENT_REPLAY");
    let _segment_replay_snapshot_env =
        TestEnvVarGuard::unset("LZVM_GUEST_PC_TRACE_SEGMENT_REPLAY_SNAPSHOT");
    let _parallel_replay_env =
        TestEnvVarGuard::unset("LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_REPLAY_ONLY");
    let _live_env = TestEnvVarGuard::set("LZVM_GUEST_PC_TRACE_LIVE_REPORT_CHUNKS", "1");
    let _chunk_capacity_env =
        TestEnvVarGuard::set("LZVM_GUEST_PC_TRACE_REPORT_CHUNK_CAPACITY", "1");
    let _parallel_env = TestEnvVarGuard::set("LZVM_GUEST_PC_TRACE_PARALLEL_LOWER", "1");
    let _worker_env = TestEnvVarGuard::set("LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_WORKERS", "2");
    let dir = repo_temp_dir("guest-pc-live-report-parallel-lower");
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
    let expected =
        compute_guest_pc_trace_segments(32, context, &[]).expect("serial segments should compute");
    let mut emitted = Vec::new();
    let produced = produce_guest_pc_trace_segments(32, context, &[], None, |segment| {
        emitted.push(segment);
        Ok(())
    })
    .expect("live report chunk parallel lower should produce segments");
    std::fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert!(expected.len() >= 3);
    assert_eq!(emitted.len(), expected.len());
    assert_eq!(produced.timing.parallel_lower_worker_count(), 2);
    assert_eq!(
        produced.timing.parallel_lower_dispatched_count(),
        expected.len()
    );
    assert_eq!(
        produced.timing.parallel_lower_received_count(),
        expected.len()
    );
    assert_eq!(
        produced.timing.parallel_lower_emitted_count(),
        expected.len()
    );
    assert!(produced.timing.trace_report_chunk_sent_count() > 0);
    assert_eq!(
        produced.timing.trace_report_chunk_sent_count(),
        produced.timing.trace_report_chunk_received_count()
    );
    assert!(produced.timing.seed_direct_lift_success_count() > 0);
    for (emitted, expected) in emitted.iter().zip(expected.iter()) {
        assert_eq!(emitted.trace_instance_index, expected.trace_instance_index);
        assert_eq!(
            emitted.trace_source_prefix_rows,
            expected.trace_source_prefix_rows
        );
        #[cfg(feature = "cuda")]
        assert_eq!(
            emitted.device_segment_material,
            expected.device_segment_material
        );
        assert_eq!(emitted.trace, expected.trace);
        assert_eq!(emitted.unit_values, expected.unit_values);
    }
}

#[test]
#[cfg(feature = "cuda")]
fn live_report_chunk_parallel_lower_streams_chunks_to_workers() {
    let _env_lock = GUEST_PC_TRACE_ENV_LOCK
        .lock()
        .expect("guest PC trace env lock should not be poisoned");
    let _seed_mirror_env = TestEnvVarGuard::unset("LZVM_GUEST_PC_TRACE_SEED_MIRROR");
    let _segment_replay_env = TestEnvVarGuard::unset("LZVM_GUEST_PC_TRACE_SEGMENT_REPLAY");
    let _segment_replay_snapshot_env =
        TestEnvVarGuard::unset("LZVM_GUEST_PC_TRACE_SEGMENT_REPLAY_SNAPSHOT");
    let _parallel_replay_env =
        TestEnvVarGuard::unset("LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_REPLAY_ONLY");
    let _live_env = TestEnvVarGuard::set("LZVM_GUEST_PC_TRACE_LIVE_REPORT_CHUNKS", "1");
    let _stream_start_env = TestEnvVarGuard::set("LZVM_GUEST_PC_TRACE_LIVE_STREAM_START", "1");
    let _stream_worker_env =
        TestEnvVarGuard::set("LZVM_GUEST_PC_TRACE_PARALLEL_STREAM_CHUNKS", "1");
    let _chunk_capacity_env =
        TestEnvVarGuard::set("LZVM_GUEST_PC_TRACE_REPORT_CHUNK_CAPACITY", "1");
    let _parallel_env = TestEnvVarGuard::set("LZVM_GUEST_PC_TRACE_PARALLEL_LOWER", "1");
    let _worker_env = TestEnvVarGuard::set("LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_WORKERS", "2");
    assert!(guest_pc_trace_parallel_stream_chunks_enabled());
    let dir = repo_temp_dir("guest-pc-live-report-parallel-stream");
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
    let unit = sample_unit_with_zisk_main_device_columns_rows(2);
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");
    let context = WitnessComputeContext {
        guest_image: Some(&guest_image),
        guest_image_info: Some(&guest_image_info),
        trace_layout: Some(&layout),
    };
    let expected =
        compute_guest_pc_trace_segments(32, context, &[]).expect("serial segments should compute");
    let mut emitted = Vec::new();
    let produced = produce_guest_pc_trace_segments(32, context, &[], None, |segment| {
        emitted.push(segment);
        Ok(())
    })
    .expect("live report chunk parallel lower should stream chunks to workers");
    std::fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert!(expected
        .iter()
        .any(|segment| segment.device_segment_material.is_some()));
    assert_eq!(emitted.len(), expected.len());
    assert_eq!(produced.timing.parallel_lower_worker_count(), 2);
    assert_eq!(
        produced.timing.parallel_lower_dispatched_count(),
        expected.len()
    );
    assert_eq!(
        produced.timing.parallel_lower_received_count(),
        expected.len()
    );
    assert_eq!(
        produced.timing.trace_stream_start_sent_count(),
        expected.len()
    );
    assert_eq!(
        produced.timing.parallel_lower_stream_chunk_count(),
        produced.timing.trace_report_chunk_sent_count(),
        "worker streaming should consume all live chunks without dispatcher reassembly"
    );
    assert!(
        produced
            .timing
            .parallel_lower_stream_chunk_process_duration()
            > std::time::Duration::ZERO,
        "worker streaming should record chunk processing time separately from dispatch wait"
    );
    assert_eq!(
        produced.timing.parallel_lower_stream_retained_report_count(),
        0,
        "worker streaming should use replay snapshots for terminal fallback without retaining every streamed report"
    );
    assert!(produced.timing.parallel_lower_stream_segment_count() > 0);
    assert_eq!(
        produced.timing.owned_streaming_lower_segment_count(),
        produced.timing.parallel_lower_stream_segment_count()
    );
    for (emitted, expected) in emitted.iter().zip(expected.iter()) {
        assert_eq!(emitted.trace_instance_index, expected.trace_instance_index);
        assert_eq!(
            emitted.trace_source_prefix_rows,
            expected.trace_source_prefix_rows
        );
        assert_eq!(
            emitted.device_segment_material,
            expected.device_segment_material
        );
        assert_eq!(emitted.trace, expected.trace);
        assert_eq!(emitted.unit_values, expected.unit_values);
    }
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
    let lowered = lower_guest_pc_trace_seeded_pending_segment_with_output_mode(
        &layout,
        second,
        second_seed,
        None,
        false,
        None,
    )
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
            lower_guest_pc_trace_seeded_pending_segment_with_output_mode(
                &layout, segment, seed, None, false, None,
            )
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

fn sample_main_trace_unit_rows(row_count: u64) -> ProveUnitSchedule {
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

#[test]
fn pending_work_units_parallel_lower_without_replay_snapshots() {
    let _env_lock = GUEST_PC_TRACE_ENV_LOCK
        .lock()
        .expect("guest PC trace env lock should not be poisoned");
    let _mirror_env = TestEnvVarGuard::unset("LZVM_GUEST_PC_TRACE_SEED_MIRROR");
    let _snapshot_env = TestEnvVarGuard::unset("LZVM_GUEST_PC_TRACE_RUNNER_SEED_SNAPSHOT");
    let _trusted_env = TestEnvVarGuard::unset("LZVM_GUEST_PC_TRACE_RUNNER_SEED_SNAPSHOT_TRUSTED");
    let _parallel_env = TestEnvVarGuard::unset("LZVM_GUEST_PC_TRACE_PARALLEL_LOWER");
    let _work_units_env =
        TestEnvVarGuard::set("LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_WORK_UNITS", "1");
    let _worker_env = TestEnvVarGuard::set("LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_WORKERS", "2");
    let _replay_env = TestEnvVarGuard::unset("LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_REPLAY_ONLY");
    let _segment_replay_env = TestEnvVarGuard::unset("LZVM_GUEST_PC_TRACE_SEGMENT_REPLAY");
    let _snapshot_replay_env =
        TestEnvVarGuard::unset("LZVM_GUEST_PC_TRACE_SEGMENT_REPLAY_SNAPSHOT");
    let dir = repo_temp_dir("guest-pc-pending-work-units");
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
    let unit = sample_main_trace_unit_rows(2);
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
    assert!(pending.iter().all(|segment| !segment.reports_elided));
    assert!(pending
        .iter()
        .all(|segment| segment.replay_snapshot.is_none()));
    assert!(pending
        .iter()
        .all(|segment| segment.reports.len() == segment.report_count));

    let mut serial = Vec::new();
    for segment in &pending {
        let seed = segment
            .seed
            .as_deref()
            .expect("work-unit pending segment should carry its own seed");
        serial.push(
            lower_guest_pc_trace_seeded_pending_segment_with_output_mode(
                &layout, segment, seed, None, false, None,
            )
            .expect("serial seeded segment should lower"),
        );
    }

    let work_units = pending
        .into_iter()
        .map(GuestPcTraceParallelLowerWorkUnit::try_from)
        .collect::<Result<Vec<_>, _>>()
        .expect("retained-report pending segments should convert into work units");
    assert!(work_units
        .iter()
        .all(|unit| unit.reports.len() == unit.report_count));
    assert!(work_units.iter().all(|unit| !unit.reports_elided));

    let parallel =
        lower_guest_pc_trace_parallel_lower_work_units_with_workers(&layout, work_units, None, 2)
            .expect("parallel work units should lower");

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
fn work_unit_env_stream_matches_serial_without_replay() {
    let _env_lock = GUEST_PC_TRACE_ENV_LOCK
        .lock()
        .expect("guest PC trace env lock should not be poisoned");
    let _mirror_env = TestEnvVarGuard::unset("LZVM_GUEST_PC_TRACE_SEED_MIRROR");
    let _snapshot_env = TestEnvVarGuard::unset("LZVM_GUEST_PC_TRACE_RUNNER_SEED_SNAPSHOT");
    let _trusted_env = TestEnvVarGuard::unset("LZVM_GUEST_PC_TRACE_RUNNER_SEED_SNAPSHOT_TRUSTED");
    let _parallel_env = TestEnvVarGuard::unset("LZVM_GUEST_PC_TRACE_PARALLEL_LOWER");
    let _work_units_env =
        TestEnvVarGuard::set("LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_WORK_UNITS", "1");
    let _worker_env = TestEnvVarGuard::set("LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_WORKERS", "2");
    let _replay_env = TestEnvVarGuard::unset("LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_REPLAY_ONLY");
    let _parallel_snapshot_env =
        TestEnvVarGuard::unset("LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_REPLAY_SNAPSHOT");
    let _segment_replay_env = TestEnvVarGuard::unset("LZVM_GUEST_PC_TRACE_SEGMENT_REPLAY");
    let _snapshot_replay_env =
        TestEnvVarGuard::unset("LZVM_GUEST_PC_TRACE_SEGMENT_REPLAY_SNAPSHOT");
    let dir = repo_temp_dir("guest-pc-work-unit-stream");
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
    let unit = sample_main_trace_unit_rows(2);
    let layout = derive_witness_trace_layout(&unit).expect("layout should derive");
    let context = WitnessComputeContext {
        guest_image: Some(&guest_image),
        guest_image_info: Some(&guest_image_info),
        trace_layout: Some(&layout),
    };

    let serial = compute_guest_pc_trace_segments(32, context, &[])
        .expect("serial guest PC trace should compute");
    let mut streamed = Vec::new();
    let stream = produce_guest_pc_trace_segments(32, context, &[], None, |segment| {
        streamed.push(segment);
        Ok(())
    })
    .expect("work-unit stream should produce");
    std::fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert!(serial.len() >= 3);
    assert_eq!(streamed.len(), serial.len());
    assert_eq!(stream.proof_values, serial[0].proof_values);
    assert_eq!(stream.timing.parallel_lower_worker_count(), 2);
    assert_eq!(stream.timing.segment_replay_count(), 0);
    assert_eq!(stream.timing.segment_replay_snapshot_capture_count(), 0);
    assert_eq!(stream.timing.parallel_lower_snapshot_replay_count(), 0);
    assert_eq!(stream.timing.parallel_lower_report_elided_count(), 0);
    assert_eq!(
        stream.timing.parallel_lower_dispatched_count(),
        serial.len()
    );
    assert_eq!(stream.timing.parallel_lower_received_count(), serial.len());
    assert_eq!(stream.timing.parallel_lower_emitted_count(), serial.len());
    for (streamed, serial) in streamed.iter().zip(serial.iter()) {
        assert_eq!(streamed.trace_instance_index, serial.trace_instance_index);
        assert_eq!(
            streamed.trace_source_prefix_rows,
            serial.trace_source_prefix_rows
        );
        #[cfg(feature = "cuda")]
        assert_eq!(
            streamed.device_segment_material,
            serial.device_segment_material
        );
        assert_eq!(streamed.trace, serial.trace);
        assert_eq!(streamed.unit_values, serial.unit_values);
    }
}

#[cfg(feature = "cuda")]
#[test]
fn seeded_pending_segment_owned_streaming_lower_matches_borrowed_output() {
    let _env_lock = GUEST_PC_TRACE_ENV_LOCK
        .lock()
        .expect("guest PC trace env lock should not be poisoned");
    let _mirror_env = TestEnvVarGuard::set("LZVM_GUEST_PC_TRACE_SEED_MIRROR", "1");
    let dir = repo_temp_dir("guest-pc-owned-streaming-seeded-lower");
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
        .into_iter()
        .find(|segment| !segment.is_last_segment)
        .expect("fixture should have a non-terminal segment");
    let seed = segment
        .seed
        .as_deref()
        .expect("seeded pending segment should carry its own seed")
        .clone();
    let borrowed = lower_guest_pc_trace_seeded_pending_segment_with_output_mode(
        &layout, &segment, &seed, None, false, None,
    )
    .expect("borrowed seeded segment should lower");
    let mut owned_timing = GuestPcTraceStreamTiming::default();
    let owned = lower_guest_pc_trace_owned_streaming_pending_segment(
        &layout,
        segment,
        &seed,
        None,
        false,
        Some(&mut owned_timing),
    )
    .expect("owned streaming seeded segment should lower");

    assert_eq!(owned.next_seed, borrowed.next_seed);
    assert_eq!(owned_timing.owned_streaming_lower_segment_count(), 1);
    assert_eq!(
        owned.segment.trace_instance_index,
        borrowed.segment.trace_instance_index
    );
    assert_eq!(
        owned.segment.trace_source_prefix_rows,
        borrowed.segment.trace_source_prefix_rows
    );
    assert_eq!(
        owned.segment.device_segment_material,
        borrowed.segment.device_segment_material
    );
    assert_eq!(owned.segment.trace, None);
    assert_eq!(owned.segment.unit_values, borrowed.segment.unit_values);
    assert_eq!(owned.segment.proof_values, borrowed.segment.proof_values);
}

#[cfg(feature = "cuda")]
#[test]
fn live_chunks_before_segment_start_lower_with_traceless_output() {
    let _env_lock = GUEST_PC_TRACE_ENV_LOCK
        .lock()
        .expect("guest PC trace env lock should not be poisoned");
    let _mirror_env = TestEnvVarGuard::set("LZVM_GUEST_PC_TRACE_SEED_MIRROR", "1");
    let _traceless_env = TestEnvVarGuard::set("LZVM_CUDA_GUEST_PC_TRACELESS_SEGMENT_OUTPUT", "1");
    let dir = repo_temp_dir("guest-pc-live-chunks-before-start");
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
    let mut serial_pending = Vec::new();
    produce_guest_pc_trace_pending_slices(32, context, &[], layout.row_count(), |segment| {
        serial_pending.push(segment);
        Ok(())
    })
    .expect("serial pending slices should produce");
    let expected = serial_pending
        .into_iter()
        .next()
        .expect("fixture should produce a first segment");
    let seed = expected
        .seed
        .as_deref()
        .expect("seed mirror should attach a segment seed")
        .clone();
    let expected_lowered = lower_guest_pc_trace_owned_streaming_pending_segment(
        &layout, expected, &seed, None, false, None,
    )
    .expect("expected segment should lower");

    let (mut live_memory, mut live_state, mut live_fcall_handler) =
        load_guest_pc_trace_machine(context, &[]).expect("live guest trace machine should load");
    let mut instruction_cache = GuestInstructionCache::default();
    let (sender, receiver) = mpsc::sync_channel(8);
    emit_guest_pc_trace_live_pending_segment_messages(
        &mut live_memory,
        &mut live_state,
        &mut live_fcall_handler,
        expected_lowered.segment.trace_instance_index,
        32,
        layout.row_count(),
        Some(Box::new(seed.clone())),
        None,
        None,
        &mut instruction_cache,
        false,
        1,
        |message| {
            sender.send(message).expect("live message should send");
            Ok(())
        },
    )
    .expect("live pending segment messages should emit");
    sender
        .send(GuestPcTracePendingSegmentMessage::Complete(Box::new(
            GuestPcTraceStreamResult {
                proof_values: Vec::new(),
                timing: GuestPcTraceStreamTiming::default(),
            },
        )))
        .expect("completion message should send");
    drop(sender);
    std::fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    let mut timing = GuestPcTraceStreamTiming::default();
    let mut emitted = Vec::new();
    lower_guest_pc_trace_pending_segments(
        32,
        &layout,
        receiver,
        None,
        &mut timing,
        &mut |segment| {
            emitted.push(segment);
            Ok(())
        },
    )
    .expect("lowerer should accept live chunks sent before segment metadata");

    assert_eq!(emitted.len(), 1);
    let emitted = emitted.pop().expect("one segment should be emitted");
    assert_eq!(
        emitted.trace_instance_index,
        expected_lowered.segment.trace_instance_index
    );
    assert_eq!(
        emitted.trace_source_prefix_rows,
        expected_lowered.segment.trace_source_prefix_rows
    );
    assert_eq!(
        emitted.device_segment_material,
        expected_lowered.segment.device_segment_material
    );
    assert_eq!(emitted.trace, None);
    assert_eq!(emitted.unit_values, expected_lowered.segment.unit_values);
    assert_eq!(emitted.proof_values, expected_lowered.segment.proof_values);
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
    let _work_units_env = TestEnvVarGuard::unset("LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_WORK_UNITS");
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
fn parallel_lower_work_units_selects_parallel_lower_without_replay_elision() {
    let _env_lock = GUEST_PC_TRACE_ENV_LOCK
        .lock()
        .expect("guest PC trace env lock should not be poisoned");
    let _parallel_env = TestEnvVarGuard::unset("LZVM_GUEST_PC_TRACE_PARALLEL_LOWER");
    let _work_units_env =
        TestEnvVarGuard::set("LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_WORK_UNITS", "1");
    let _workers_env = TestEnvVarGuard::set("LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_WORKERS", "2");
    let _replay_only_env = TestEnvVarGuard::unset("LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_REPLAY_ONLY");
    let _replay_snapshot_env =
        TestEnvVarGuard::unset("LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_REPLAY_SNAPSHOT");
    let _snapshot_env = TestEnvVarGuard::unset("LZVM_GUEST_PC_TRACE_RUNNER_SEED_SNAPSHOT");
    let _trusted_env = TestEnvVarGuard::unset("LZVM_GUEST_PC_TRACE_RUNNER_SEED_SNAPSHOT_TRUSTED");

    assert!(guest_pc_trace_parallel_lower_work_units_enabled());
    assert!(guest_pc_trace_parallel_lower_enabled());
    assert_eq!(guest_pc_trace_parallel_lower_worker_count(), Some(2));
    assert!(guest_pc_trace_runner_seed_snapshot_enabled());
    assert!(guest_pc_trace_runner_seed_snapshot_trusted_enabled());
    assert!(!guest_pc_trace_parallel_lower_report_elision_enabled());
    assert!(!guest_pc_trace_parallel_lower_replay_snapshot_enabled());

    let mode = GuestPcTraceParallelLowerMode::from_env();
    assert!(mode.work_units);
    assert!(!mode.replay_snapshot);
}

#[test]
#[cfg(feature = "cuda")]
fn large_runtime_auto_work_units_select_parallel_lower() {
    let _env_lock = GUEST_PC_TRACE_ENV_LOCK
        .lock()
        .expect("guest PC trace env lock should not be poisoned");
    let _parallel_env = TestEnvVarGuard::unset("LZVM_GUEST_PC_TRACE_PARALLEL_LOWER");
    let _work_units_env = TestEnvVarGuard::unset("LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_WORK_UNITS");
    let _auto_lower_env = TestEnvVarGuard::unset("LZVM_GUEST_PC_TRACE_AUTO_PARALLEL_LOWER");
    let _auto_env = TestEnvVarGuard::unset("LZVM_GUEST_PC_TRACE_AUTO_PARALLEL_LOWER_WORK_UNITS");
    let _snapshot_env = TestEnvVarGuard::unset("LZVM_GUEST_PC_TRACE_RUNNER_SEED_SNAPSHOT");
    let _trusted_env = TestEnvVarGuard::unset("LZVM_GUEST_PC_TRACE_RUNNER_SEED_SNAPSHOT_TRUSTED");

    assert!(!guest_pc_trace_parallel_lower_enabled());
    assert!(!guest_pc_trace_parallel_lower_enabled_for_limit(5_000_000));
    assert!(!guest_pc_trace_parallel_lower_enabled_for_limit(49_999_999));
    assert!(!guest_pc_trace_parallel_lower_enabled_for_limit(50_000_000));

    std::env::set_var("LZVM_GUEST_PC_TRACE_AUTO_PARALLEL_LOWER_WORK_UNITS", "1");
    assert!(guest_pc_trace_parallel_lower_enabled_for_limit(50_000_000));
    assert!(guest_pc_trace_parallel_lower_work_units_enabled_for_limit(
        50_000_000
    ));

    let seed_mode = GuestPcTraceRunnerSeedMode::from_runtime(50_000_000);
    assert!(seed_mode.snapshot);
    assert!(seed_mode.trusted);

    let mode = GuestPcTraceParallelLowerMode::from_runtime(50_000_000);
    assert!(mode.work_units);

    std::env::set_var("LZVM_GUEST_PC_TRACE_AUTO_PARALLEL_LOWER_WORK_UNITS", "0");
    assert!(!guest_pc_trace_parallel_lower_enabled_for_limit(50_000_000));
    assert!(!GuestPcTraceParallelLowerMode::from_runtime(50_000_000).work_units);
}

#[test]
#[cfg(feature = "cuda")]
fn large_runtime_auto_lower_uses_bounded_workers() {
    let _env_lock = GUEST_PC_TRACE_ENV_LOCK
        .lock()
        .expect("guest PC trace env lock should not be poisoned");
    let _parallel_env = TestEnvVarGuard::unset("LZVM_GUEST_PC_TRACE_PARALLEL_LOWER");
    let _work_units_env = TestEnvVarGuard::unset("LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_WORK_UNITS");
    let _auto_lower_env = TestEnvVarGuard::unset("LZVM_GUEST_PC_TRACE_AUTO_PARALLEL_LOWER");
    let _workers_env = TestEnvVarGuard::unset("LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_WORKERS");
    let _snapshot_env = TestEnvVarGuard::unset("LZVM_GUEST_PC_TRACE_RUNNER_SEED_SNAPSHOT");
    let _trusted_env = TestEnvVarGuard::unset("LZVM_GUEST_PC_TRACE_RUNNER_SEED_SNAPSHOT_TRUSTED");

    assert!(!guest_pc_trace_auto_parallel_lower_selected(599_999_999));
    assert!(!guest_pc_trace_parallel_lower_enabled_for_limit(
        599_999_999
    ));
    assert!(guest_pc_trace_auto_parallel_lower_selected(600_000_000));
    assert!(guest_pc_trace_parallel_lower_enabled_for_limit(600_000_000));
    assert_eq!(
        guest_pc_trace_parallel_lower_worker_count_for_limit(600_000_000),
        Some(2)
    );

    let seed_mode = GuestPcTraceRunnerSeedMode::from_runtime(600_000_000);
    assert!(seed_mode.snapshot);
    assert!(seed_mode.trusted);

    std::env::set_var("LZVM_GUEST_PC_TRACE_AUTO_PARALLEL_LOWER", "0");
    assert!(!guest_pc_trace_auto_parallel_lower_selected(600_000_000));
    assert!(!guest_pc_trace_parallel_lower_enabled_for_limit(
        600_000_000
    ));

    std::env::set_var("LZVM_GUEST_PC_TRACE_AUTO_PARALLEL_LOWER", "1");
    std::env::set_var("LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_WORKERS", "3");
    assert_eq!(
        guest_pc_trace_parallel_lower_worker_count_for_limit(600_000_000),
        Some(3)
    );

    std::env::set_var("LZVM_GUEST_PC_TRACE_PARALLEL_LOWER", "0");
    assert!(!guest_pc_trace_auto_parallel_lower_selected(600_000_000));
    assert!(!guest_pc_trace_parallel_lower_enabled_for_limit(
        600_000_000
    ));
}

#[test]
#[cfg(not(feature = "cuda"))]
fn large_runtime_auto_work_units_stay_disabled_without_gpu_feature() {
    let _env_lock = GUEST_PC_TRACE_ENV_LOCK
        .lock()
        .expect("guest PC trace env lock should not be poisoned");
    let _parallel_env = TestEnvVarGuard::unset("LZVM_GUEST_PC_TRACE_PARALLEL_LOWER");
    let _work_units_env = TestEnvVarGuard::unset("LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_WORK_UNITS");
    let _auto_lower_env = TestEnvVarGuard::unset("LZVM_GUEST_PC_TRACE_AUTO_PARALLEL_LOWER");
    let _auto_env = TestEnvVarGuard::unset("LZVM_GUEST_PC_TRACE_AUTO_PARALLEL_LOWER_WORK_UNITS");
    let _snapshot_env = TestEnvVarGuard::unset("LZVM_GUEST_PC_TRACE_RUNNER_SEED_SNAPSHOT");
    let _trusted_env = TestEnvVarGuard::unset("LZVM_GUEST_PC_TRACE_RUNNER_SEED_SNAPSHOT_TRUSTED");

    assert!(!guest_pc_trace_parallel_lower_enabled_for_limit(50_000_000));
    assert!(!guest_pc_trace_parallel_lower_work_units_enabled_for_limit(
        50_000_000
    ));

    let seed_mode = GuestPcTraceRunnerSeedMode::from_runtime(50_000_000);
    assert!(!seed_mode.snapshot);
    assert!(!seed_mode.trusted);
    assert!(!GuestPcTraceParallelLowerMode::from_runtime(50_000_000).work_units);
}

#[test]
fn commit_pipeline_does_not_enable_parallel_lower() {
    let _env_lock = GUEST_PC_TRACE_ENV_LOCK
        .lock()
        .expect("guest PC trace env lock should not be poisoned");
    let _parallel_env = TestEnvVarGuard::unset("LZVM_GUEST_PC_TRACE_PARALLEL_LOWER");
    let _work_units_env = TestEnvVarGuard::unset("LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_WORK_UNITS");
    let _pipeline_env = TestEnvVarGuard::set("LZVM_GUEST_PC_TRACE_COMMIT_PIPELINE", "1");

    assert!(!guest_pc_trace_parallel_lower_enabled());
    assert_eq!(guest_pc_trace_parallel_lower_worker_count(), None);
    assert!(!guest_pc_trace_runner_seed_snapshot_enabled());
    assert!(!guest_pc_trace_runner_seed_snapshot_trusted_enabled());
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
        None,
    )
    .expect("dispatcher should skip a full worker queue");

    assert_eq!(first_receiver.try_recv(), Ok(10));
    assert_eq!(second_receiver.try_recv(), Ok(20));
    assert_eq!(next_worker, 0);
}

#[test]
fn parallel_lower_job_dispatch_records_full_queue_backpressure() {
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    sender
        .send(10_u32)
        .expect("worker queue should accept setup job");
    let mut next_worker = 0_usize;
    let mut timing = GuestPcTraceStreamTiming::default();
    let drain = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(5));
        let previous = receiver
            .recv()
            .expect("drain thread should receive setup job");
        let dispatched = receiver
            .recv()
            .expect("drain thread should receive dispatched job");
        (previous, dispatched)
    });

    dispatch_guest_pc_trace_parallel_lower_job(
        &[sender],
        &mut next_worker,
        20_u32,
        Some(&mut timing),
    )
    .expect("dispatcher should block until a full worker queue drains");

    assert_eq!(drain.join().expect("drain thread should finish"), (10, 20));
    assert_eq!(timing.parallel_lower_dispatch_blocked_count(), 1);
    assert!(
        timing.parallel_lower_dispatch_wait_duration() > std::time::Duration::ZERO,
        "full worker queue dispatch should record nonzero wait"
    );
}

#[test]
fn parallel_lower_result_send_records_full_queue_backpressure() {
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    sender
        .send(GuestPcTraceParallelLowerMessage::Complete {
            stream: Box::new(GuestPcTraceStreamResult {
                proof_values: Vec::new(),
                timing: GuestPcTraceStreamTiming::default(),
            }),
            dispatched_count: 0,
            timing: GuestPcTraceStreamTiming::default(),
        })
        .expect("result queue should accept setup message");
    let drain = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(5));
        let _previous = receiver
            .recv()
            .expect("drain thread should receive setup message");
        receiver
            .recv()
            .expect("drain thread should receive result message")
    });
    let seed = ZiskMainSegmentSeed::new();
    let result = GuestPcTraceSeededLoweredSegment {
        seed: seed.clone(),
        lowered: GuestPcTraceLoweredSegment {
            segment: GuestPcTraceSegmentTrace {
                trace_instance_index: 0,
                trace_source_prefix_rows: 0,
                #[cfg(feature = "cuda")]
                device_segment_material: None,
                trace: None,
                unit_values: Vec::new(),
                proof_values: Vec::new(),
            },
            next_seed: seed,
        },
    };

    assert!(send_guest_pc_trace_parallel_lower_segment_result(
        &sender,
        0,
        Ok(result),
        GuestPcTraceStreamTiming::default(),
    ));

    let message = drain.join().expect("drain thread should finish");
    match message {
        GuestPcTraceParallelLowerMessage::Segment { timing, .. } => {
            assert!(
                timing.parallel_lower_result_send_wait_duration() > std::time::Duration::ZERO,
                "full result queue send should record nonzero wait"
            );
        }
        _ => panic!("drain thread should receive a segment result"),
    }
}

#[cfg(feature = "cuda")]
#[test]
fn fixed_worker_stream_chunk_dispatch_records_full_queue_backpressure() {
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    sender
        .send(GuestPcTraceParallelLowerJob::StreamChunk(Box::new(
            GuestPcTracePendingReportChunk {
                trace_instance_index: 0,
                reports: Vec::new(),
            },
        )))
        .expect("worker queue should accept setup job");
    let mut timing = GuestPcTraceStreamTiming::default();
    let drain = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(5));
        let _previous = receiver
            .recv()
            .expect("drain thread should receive setup job");
        let dispatched = receiver
            .recv()
            .expect("drain thread should receive dispatched job");
        match dispatched {
            GuestPcTraceParallelLowerJob::StreamChunk(chunk) => chunk.trace_instance_index,
            _ => panic!("drain thread should receive a stream chunk job"),
        }
    });

    let result = send_guest_pc_trace_parallel_lower_job_to_worker(
        &[sender],
        0,
        GuestPcTraceParallelLowerJob::StreamChunk(Box::new(GuestPcTracePendingReportChunk {
            trace_instance_index: 1,
            reports: Vec::new(),
        })),
        Some(&mut timing),
    );
    assert!(
        result.is_ok(),
        "fixed worker dispatcher should block until a full worker queue drains"
    );

    assert_eq!(drain.join().expect("drain thread should finish"), 1);
    assert_eq!(timing.parallel_lower_dispatch_blocked_count(), 1);
    assert!(
        timing.parallel_lower_stream_chunk_dispatch_wait_duration() > std::time::Duration::ZERO,
        "full fixed worker stream chunk dispatch should record nonzero wait"
    );
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
    let report =
        GuestMachineReport {
            address_and_instruction_len:
                crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0000, 4),
            instruction: RiscvInstruction::ZiskDmaPrepare {
                kind: RiscvDmaKind::Memcpy,
                rs1: 5,
            },
            next_pc: 0x8000_0004,
            register_write_value: GuestRegisterWriteValue::default(),
            memory_accesses: Vec::new().into(),
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
    let report =
        GuestMachineReport {
            address_and_instruction_len:
                crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0000, 4),
            instruction: RiscvInstruction::ZiskDmaPrepare {
                kind: RiscvDmaKind::Memcpy,
                rs1: 5,
            },
            next_pc: 0x8000_0004,
            register_write_value: GuestRegisterWriteValue::default(),
            memory_accesses: Vec::new().into(),
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
    let report =
        GuestMachineReport {
            address_and_instruction_len:
                crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0000, 4),
            instruction: RiscvInstruction::Store {
                kind: RiscvStoreKind::Sd,
                rs1: 1,
                rs2: 2,
                offset: 8,
            },
            next_pc: 0x8000_0004,
            register_write_value: GuestRegisterWriteValue::default(),
            memory_accesses: vec![GuestMemoryAccess {
                kind: GuestMemoryAccessKind::Write,
                address: 0x1008,
                byte_len: 8,
                value: 0xfeed_face_cafe_babe,
            }]
            .into(),
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
    let report =
        GuestMachineReport {
            address_and_instruction_len:
                crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0000, 4),
            instruction: RiscvInstruction::Store {
                kind: RiscvStoreKind::Sb,
                rs1: 10,
                rs2: 12,
                offset: 17,
            },
            next_pc: 0x8000_0004,
            register_write_value: GuestRegisterWriteValue::default(),
            memory_accesses: vec![GuestMemoryAccess {
                kind: GuestMemoryAccessKind::Write,
                address: 0x1011,
                byte_len: 1,
                value: 0xf0,
            }]
            .into(),
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
    let report =
        GuestMachineReport {
            address_and_instruction_len:
                crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0000, 4),
            instruction: RiscvInstruction::Store {
                kind: RiscvStoreKind::Sb,
                rs1: 10,
                rs2: 12,
                offset: 17,
            },
            next_pc: 0x8000_0004,
            register_write_value: GuestRegisterWriteValue::default(),
            memory_accesses: vec![GuestMemoryAccess {
                kind: GuestMemoryAccessKind::Write,
                address: 0x1011,
                byte_len: 1,
                value: 0xf0,
            }]
            .into(),
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
        ZiskMainRunnerBoundarySeedInput::from_reports(
            std::slice::from_ref(&report),
            None,
            &runner_state,
            &current_seed,
            &boundary_snapshot,
        ),
        0x1234_5678_9abc_def0,
    )
    .expect("narrow store boundary c should come from runner state registers");

    assert_eq!(lifted.previous_c, 0x1234_5678_9abc_def0);
    assert_eq!(lifted.initial_state.last_c, 0x1234_5678_9abc_def0);
    assert_eq!(lifted.initial_state.registers[12], 0x1234_5678_9abc_def0);
}

#[test]
fn runner_boundary_seed_snapshot_uses_report_shape_for_store_boundary_without_retained_report() {
    let current_seed = ZiskMainSegmentSeed::new();
    let mut runner_state = GuestMachineState::new(0x8000_0004);
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

    let lifted = try_lift_zisk_main_next_segment_seed_from_runner_boundary_snapshot(
        1,
        segment,
        ZiskMainRunnerBoundarySeedInput {
            reports: &[],
            report_count: 1,
            last_report_shape: Some(GuestMachineReportShape {
                instruction: RiscvInstruction::Store {
                    kind: RiscvStoreKind::Sb,
                    rs1: 10,
                    rs2: 12,
                    offset: 17,
                },
                has_memory_write: true,
            }),
            lookahead_instruction: Some(RiscvInstruction::OpImm {
                kind: RiscvOpImmKind::Addi,
                rd: 0,
                rs1: 0,
                immediate: 0,
            }),
            runner_state: &runner_state,
            current_seed: &current_seed,
            boundary_snapshot: &boundary_snapshot,
        },
    )
    .expect("shape-only store boundary seed lift should evaluate")
    .expect("shape-only store boundary should use runner source register");

    assert_eq!(lifted.previous_c, 0x1234_5678_9abc_def0);
    assert_eq!(lifted.initial_state.last_c, 0x1234_5678_9abc_def0);
    assert_eq!(lifted.initial_state.registers[12], 0x1234_5678_9abc_def0);
}

#[test]
fn runner_boundary_seed_snapshot_uses_report_shape_for_register_boundary_without_retained_report() {
    let current_seed = ZiskMainSegmentSeed::new();
    let mut runner_state = GuestMachineState::new(0x8000_0004);
    runner_state
        .set_register(12, 0x1234_5678_9abc_def0)
        .expect("destination register should set");
    let boundary_snapshot = ZiskMainRunnerBoundarySnapshot::new(&current_seed);
    let segment = ZiskMainTraceSegmentInfo {
        trace_instance_index: 0,
        is_last_segment: false,
        previous_c: 0,
    };

    let lifted = try_lift_zisk_main_next_segment_seed_from_runner_boundary_snapshot(
        1,
        segment,
        ZiskMainRunnerBoundarySeedInput {
            reports: &[],
            report_count: 1,
            last_report_shape: Some(GuestMachineReportShape {
                instruction: RiscvInstruction::OpImm {
                    kind: RiscvOpImmKind::Addi,
                    rd: 12,
                    rs1: 0,
                    immediate: 7,
                },
                has_memory_write: false,
            }),
            lookahead_instruction: Some(RiscvInstruction::OpImm {
                kind: RiscvOpImmKind::Addi,
                rd: 0,
                rs1: 0,
                immediate: 0,
            }),
            runner_state: &runner_state,
            current_seed: &current_seed,
            boundary_snapshot: &boundary_snapshot,
        },
    )
    .expect("shape-only register boundary seed lift should evaluate")
    .expect("shape-only register boundary should use runner destination register");

    assert_eq!(lifted.previous_c, 0x1234_5678_9abc_def0);
    assert_eq!(lifted.initial_state.last_c, 0x1234_5678_9abc_def0);
    assert_eq!(lifted.initial_state.registers[12], 0x1234_5678_9abc_def0);
}

#[test]
fn runner_boundary_seed_snapshot_uses_report_shape_for_load_boundary_without_retained_report() {
    let current_seed = ZiskMainSegmentSeed::new();
    let mut runner_state = GuestMachineState::new(0x8000_0004);
    runner_state
        .set_register(28, 0x7f)
        .expect("destination register should set");
    let boundary_snapshot = ZiskMainRunnerBoundarySnapshot::new(&current_seed);
    let segment = ZiskMainTraceSegmentInfo {
        trace_instance_index: 0,
        is_last_segment: false,
        previous_c: 0,
    };

    let lifted = try_lift_zisk_main_next_segment_seed_from_runner_boundary_snapshot(
        1,
        segment,
        ZiskMainRunnerBoundarySeedInput {
            reports: &[],
            report_count: 1,
            last_report_shape: Some(GuestMachineReportShape {
                instruction: RiscvInstruction::Load {
                    kind: RiscvLoadKind::Lbu,
                    rd: 28,
                    rs1: 12,
                    offset: 107,
                },
                has_memory_write: false,
            }),
            lookahead_instruction: Some(RiscvInstruction::OpImm {
                kind: RiscvOpImmKind::Slli,
                rd: 10,
                rs1: 10,
                immediate: 8,
            }),
            runner_state: &runner_state,
            current_seed: &current_seed,
            boundary_snapshot: &boundary_snapshot,
        },
    )
    .expect("shape-only load boundary seed lift should evaluate")
    .expect("shape-only load boundary should use runner destination register");

    assert_eq!(lifted.previous_c, 0x7f);
    assert_eq!(lifted.initial_state.last_c, 0x7f);
    assert_eq!(lifted.initial_state.registers[28], 0x7f);
}

#[test]
fn runner_boundary_seed_snapshot_uses_report_shape_for_jalr_boundary_without_retained_report() {
    let current_seed = ZiskMainSegmentSeed::new();
    let runner_state = GuestMachineState::new(0x8000_1234);
    let boundary_snapshot = ZiskMainRunnerBoundarySnapshot::new(&current_seed);
    let segment = ZiskMainTraceSegmentInfo {
        trace_instance_index: 0,
        is_last_segment: false,
        previous_c: 0,
    };

    let lifted = try_lift_zisk_main_next_segment_seed_from_runner_boundary_snapshot(
        1,
        segment,
        ZiskMainRunnerBoundarySeedInput {
            reports: &[],
            report_count: 1,
            last_report_shape: Some(GuestMachineReportShape {
                instruction: RiscvInstruction::Jalr {
                    rd: 0,
                    rs1: 1,
                    offset: 4,
                },
                has_memory_write: false,
            }),
            lookahead_instruction: Some(RiscvInstruction::Load {
                kind: RiscvLoadKind::Ld,
                rd: 10,
                rs1: 2,
                offset: 600,
            }),
            runner_state: &runner_state,
            current_seed: &current_seed,
            boundary_snapshot: &boundary_snapshot,
        },
    )
    .expect("shape-only JALR boundary seed lift should evaluate")
    .expect("shape-only JALR boundary should use runner next PC");

    assert_eq!(lifted.previous_c, 0x8000_1230);
    assert_eq!(lifted.initial_state.last_c, 0x8000_1230);
    assert_eq!(lifted.initial_state.next_pc, 0x8000_1234);
}

#[test]
fn runner_boundary_seed_snapshot_uses_report_shape_for_branch_boundary_without_retained_report() {
    let current_seed = ZiskMainSegmentSeed::new();
    let report =
        GuestMachineReport {
            address_and_instruction_len:
                crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0000, 4),
            instruction: RiscvInstruction::Branch {
                kind: RiscvBranchKind::Bne,
                rs1: 22,
                rs2: 11,
                offset: 84,
            },
            next_pc: 0x8000_0054,
            register_write_value: GuestRegisterWriteValue::default(),
            memory_accesses: Vec::new().into(),
        };
    let runner_state = GuestMachineState::new(report.next_pc);
    let mut boundary_snapshot = ZiskMainRunnerBoundarySnapshot::new(&current_seed);
    record_zisk_main_runner_boundary_snapshot(
        &mut boundary_snapshot,
        Some(&report),
        None,
        Some(RiscvInstruction::Op {
            kind: RiscvOpKind::Add,
            rd: 11,
            rs1: 19,
            rs2: 23,
        }),
        runner_state.registers(),
    )
    .expect("boundary snapshot should record branch report context");
    let segment = ZiskMainTraceSegmentInfo {
        trace_instance_index: 0,
        is_last_segment: false,
        previous_c: 0,
    };

    let lifted = try_lift_zisk_main_next_segment_seed_from_runner_boundary_snapshot(
        1,
        segment,
        ZiskMainRunnerBoundarySeedInput {
            reports: &[],
            report_count: 1,
            last_report_shape: Some(GuestMachineReportShape {
                instruction: report.instruction,
                has_memory_write: false,
            }),
            lookahead_instruction: Some(RiscvInstruction::Op {
                kind: RiscvOpKind::Add,
                rd: 11,
                rs1: 19,
                rs2: 23,
            }),
            runner_state: &runner_state,
            current_seed: &current_seed,
            boundary_snapshot: &boundary_snapshot,
        },
    )
    .expect("shape-only branch boundary seed lift should evaluate")
    .expect("shape-only branch boundary should use recorded report context");

    assert_eq!(lifted.previous_c, 0);
    assert_eq!(lifted.initial_state.last_c, 0);
    assert_eq!(lifted.initial_state.next_pc, report.next_pc);
}

#[test]
fn runner_boundary_seed_snapshot_uses_report_shape_for_jal_boundary_without_retained_report() {
    let current_seed = ZiskMainSegmentSeed::new();
    let runner_state = GuestMachineState::new(0x8000_001c);
    let boundary_snapshot = ZiskMainRunnerBoundarySnapshot::new(&current_seed);
    let segment = ZiskMainTraceSegmentInfo {
        trace_instance_index: 0,
        is_last_segment: false,
        previous_c: 0,
    };

    let lifted = try_lift_zisk_main_next_segment_seed_from_runner_boundary_snapshot(
        1,
        segment,
        ZiskMainRunnerBoundarySeedInput {
            reports: &[],
            report_count: 1,
            last_report_shape: Some(GuestMachineReportShape {
                instruction: RiscvInstruction::Jal { rd: 0, offset: 28 },
                has_memory_write: false,
            }),
            lookahead_instruction: Some(RiscvInstruction::Load {
                kind: RiscvLoadKind::Ld,
                rd: 10,
                rs1: 19,
                offset: 8,
            }),
            runner_state: &runner_state,
            current_seed: &current_seed,
            boundary_snapshot: &boundary_snapshot,
        },
    )
    .expect("shape-only JAL boundary seed lift should evaluate")
    .expect("shape-only JAL boundary should use flag result");

    assert_eq!(lifted.previous_c, 0);
    assert_eq!(lifted.initial_state.last_c, 0);
    assert_eq!(lifted.initial_state.next_pc, 0x8000_001c);
}

#[test]
fn runner_boundary_seed_snapshot_uses_report_shape_for_auipc_boundary_without_retained_report() {
    let current_seed = ZiskMainSegmentSeed::new();
    let mut runner_state = GuestMachineState::new(0x8000_0004);
    runner_state
        .set_register(12, 0x8005_4000)
        .expect("destination register should set");
    let boundary_snapshot = ZiskMainRunnerBoundarySnapshot::new(&current_seed);
    let segment = ZiskMainTraceSegmentInfo {
        trace_instance_index: 0,
        is_last_segment: false,
        previous_c: 0,
    };

    let lifted = try_lift_zisk_main_next_segment_seed_from_runner_boundary_snapshot(
        1,
        segment,
        ZiskMainRunnerBoundarySeedInput {
            reports: &[],
            report_count: 1,
            last_report_shape: Some(GuestMachineReportShape {
                instruction: RiscvInstruction::Auipc {
                    rd: 12,
                    immediate: 344064,
                },
                has_memory_write: false,
            }),
            lookahead_instruction: Some(RiscvInstruction::Load {
                kind: RiscvLoadKind::Ld,
                rd: 16,
                rs1: 11,
                offset: -328,
            }),
            runner_state: &runner_state,
            current_seed: &current_seed,
            boundary_snapshot: &boundary_snapshot,
        },
    )
    .expect("shape-only AUIPC boundary seed lift should evaluate")
    .expect("shape-only AUIPC boundary should use flag result");

    assert_eq!(lifted.previous_c, 0);
    assert_eq!(lifted.initial_state.last_c, 0);
    assert_eq!(lifted.initial_state.registers[12], 0x8005_4000);
}

#[test]
fn runner_boundary_seed_snapshot_uses_report_shape_for_keccak_precompile_boundary_without_retained_report(
) {
    let current_seed = ZiskMainSegmentSeed::new();
    let mut runner_state = GuestMachineState::new(0x8027_0014);
    runner_state
        .set_register(10, 0x9000_0000)
        .expect("precompile operand register should set");
    let boundary_snapshot = ZiskMainRunnerBoundarySnapshot::new(&current_seed);
    let segment = ZiskMainTraceSegmentInfo {
        trace_instance_index: 0,
        is_last_segment: false,
        previous_c: 0,
    };

    let lifted = try_lift_zisk_main_next_segment_seed_from_runner_boundary_snapshot(
        1,
        segment,
        ZiskMainRunnerBoundarySeedInput {
            reports: &[],
            report_count: 1,
            last_report_shape: Some(GuestMachineReportShape {
                instruction: RiscvInstruction::ZiskPrecompile {
                    kind: RiscvPrecompileKind::Keccak,
                    rs1: 10,
                    rd: 0,
                },
                has_memory_write: false,
            }),
            lookahead_instruction: Some(RiscvInstruction::OpImm {
                kind: RiscvOpImmKind::Addi,
                rd: 11,
                rs1: 2,
                immediate: 440,
            }),
            runner_state: &runner_state,
            current_seed: &current_seed,
            boundary_snapshot: &boundary_snapshot,
        },
    )
    .expect("shape-only Keccak precompile boundary seed lift should evaluate")
    .expect("shape-only Keccak precompile boundary should use fixed result");

    assert_eq!(lifted.previous_c, 0);
    assert_eq!(lifted.initial_state.last_c, 0);
    assert_eq!(lifted.initial_state.next_pc, 0x8027_0014);
}

#[test]
fn runner_boundary_seed_snapshot_uses_report_shape_for_pending_dma_without_retained_report() {
    let current_seed = ZiskMainSegmentSeed::new();
    let runner_state = GuestMachineState::new(0x8000_0004);
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
            reports: &[],
            report_count: 1,
            last_report_shape: Some(GuestMachineReportShape {
                instruction: RiscvInstruction::ZiskDmaPrepare {
                    kind: RiscvDmaKind::Memcpy,
                    rs1: 5,
                },
                has_memory_write: false,
            }),
            lookahead_instruction: Some(RiscvInstruction::OpImm {
                kind: RiscvOpImmKind::Addi,
                rd: 0,
                rs1: 0,
                immediate: 0,
            }),
            runner_state: &runner_state,
            current_seed: &current_seed,
            boundary_snapshot: &boundary_snapshot,
        },
        0,
    )
    .expect("shape-only pending DMA seed lift should succeed");

    assert_eq!(lifted.previous_c, 0);
    assert_eq!(lifted.initial_state.last_c, 0);
    assert_eq!(
        lifted.initial_state.pending_dma,
        Some(ZiskMainPendingDma {
            kind: RiscvDmaKind::Memcpy,
            first_arg_reg: 5,
        })
    );
}

#[test]
fn runner_boundary_seed_snapshot_uses_dma_prepare_lookahead_boundary_without_retained_report() {
    let current_seed = ZiskMainSegmentSeed::new();
    let mut runner_state = GuestMachineState::new(0x8000_0004);
    runner_state
        .set_register(12, 0xfeed_face_cafe_babe)
        .expect("lookahead source register should set");
    let shape = GuestMachineReportShape {
        instruction: RiscvInstruction::ZiskDmaPrepare {
            kind: RiscvDmaKind::Memcpy,
            rs1: 11,
        },
        has_memory_write: false,
    };
    let lookahead = RiscvInstruction::Op {
        kind: RiscvOpKind::Add,
        rd: 0,
        rs1: 10,
        rs2: 12,
    };
    let mut boundary_snapshot = ZiskMainRunnerBoundarySnapshot::new(&current_seed);
    boundary_snapshot
        .record_report_shape(shape, Some(lookahead), runner_state.registers())
        .expect("boundary snapshot should record DMA prepare scratch");

    let lifted = try_lift_zisk_main_next_segment_seed_from_runner_boundary_snapshot(
        1,
        ZiskMainTraceSegmentInfo {
            trace_instance_index: 0,
            is_last_segment: false,
            previous_c: current_seed.previous_c,
        },
        ZiskMainRunnerBoundarySeedInput {
            reports: &[],
            report_count: 1,
            last_report_shape: Some(shape),
            lookahead_instruction: Some(lookahead),
            runner_state: &runner_state,
            current_seed: &current_seed,
            boundary_snapshot: &boundary_snapshot,
        },
    )
    .expect("DMA prepare boundary seed lift should evaluate")
    .expect("DMA prepare lookahead boundary should use extra-params source");

    assert_eq!(lifted.previous_c, 0xfeed_face_cafe_babe);
    assert_eq!(lifted.initial_state.last_c, 0xfeed_face_cafe_babe);
    assert_eq!(
        lifted.initial_state.pending_dma,
        Some(ZiskMainPendingDma {
            kind: RiscvDmaKind::Memcpy,
            first_arg_reg: 11,
        })
    );
    assert_eq!(
        lifted
            .initial_state
            .internal_memory
            .get(ZISK_EXTRA_PARAMS_ADDRESS),
        Some(0xfeed_face_cafe_babe)
    );
}

#[test]
fn runner_boundary_seed_snapshot_uses_store_conditional_register_row_boundary_without_retained_report(
) {
    let current_seed = ZiskMainSegmentSeed::new();
    let runner_state = GuestMachineState::new(0x8000_0004);
    let boundary_snapshot = ZiskMainRunnerBoundarySnapshot::new(&current_seed);
    let segment = ZiskMainTraceSegmentInfo {
        trace_instance_index: 0,
        is_last_segment: false,
        previous_c: current_seed.previous_c,
    };

    let lifted = try_lift_zisk_main_next_segment_seed_from_runner_boundary_snapshot(
        2,
        segment,
        ZiskMainRunnerBoundarySeedInput {
            reports: &[],
            report_count: 1,
            last_report_shape: Some(GuestMachineReportShape {
                instruction: RiscvInstruction::StoreConditional {
                    width: RiscvAmoWidth::Doubleword,
                    rd: 7,
                    rs1: 10,
                    rs2: 11,
                    acquire: false,
                    release: false,
                },
                has_memory_write: true,
            }),
            lookahead_instruction: Some(RiscvInstruction::OpImm {
                kind: RiscvOpImmKind::Addi,
                rd: 0,
                rs1: 0,
                immediate: 0,
            }),
            runner_state: &runner_state,
            current_seed: &current_seed,
            boundary_snapshot: &boundary_snapshot,
        },
    )
    .expect("store-conditional boundary seed lift should evaluate")
    .expect("successful store-conditional register row should have c=0");

    assert_eq!(lifted.previous_c, 0);
    assert_eq!(lifted.initial_state.last_c, 0);
}

#[test]
fn runner_boundary_seed_snapshot_uses_store_conditional_source_boundary_without_retained_report() {
    let current_seed = ZiskMainSegmentSeed::new();
    let mut runner_state = GuestMachineState::new(0x8000_0004);
    runner_state
        .set_register(11, 0xdead_beef_cafe_f00d)
        .expect("source register should set");
    let boundary_snapshot = ZiskMainRunnerBoundarySnapshot::new(&current_seed);
    let segment = ZiskMainTraceSegmentInfo {
        trace_instance_index: 0,
        is_last_segment: false,
        previous_c: current_seed.previous_c,
    };

    let lifted = try_lift_zisk_main_next_segment_seed_from_runner_boundary_snapshot(
        1,
        segment,
        ZiskMainRunnerBoundarySeedInput {
            reports: &[],
            report_count: 1,
            last_report_shape: Some(GuestMachineReportShape {
                instruction: RiscvInstruction::StoreConditional {
                    width: RiscvAmoWidth::Doubleword,
                    rd: 0,
                    rs1: 10,
                    rs2: 11,
                    acquire: false,
                    release: false,
                },
                has_memory_write: true,
            }),
            lookahead_instruction: Some(RiscvInstruction::OpImm {
                kind: RiscvOpImmKind::Addi,
                rd: 0,
                rs1: 0,
                immediate: 0,
            }),
            runner_state: &runner_state,
            current_seed: &current_seed,
            boundary_snapshot: &boundary_snapshot,
        },
    )
    .expect("store-conditional boundary seed lift should evaluate")
    .expect("zero-destination store-conditional should use source register boundary");

    assert_eq!(lifted.previous_c, 0xdead_beef_cafe_f00d);
    assert_eq!(lifted.initial_state.last_c, 0xdead_beef_cafe_f00d);
}

#[test]
fn runner_boundary_seed_snapshot_uses_retained_report_for_zero_register_load_boundary() {
    let current_seed = ZiskMainSegmentSeed::new();
    let runner_state = GuestMachineState::new(0x8000_0004);
    let report =
        GuestMachineReport {
            address_and_instruction_len:
                crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0000, 4),
            instruction: RiscvInstruction::Load {
                kind: RiscvLoadKind::Lbu,
                rd: 0,
                rs1: 10,
                offset: 5,
            },
            next_pc: 0x8000_0004,
            register_write_value: GuestRegisterWriteValue::default(),
            memory_accesses: vec![GuestMemoryAccess {
                kind: GuestMemoryAccessKind::Read,
                address: 0x1005,
                byte_len: 1,
                value: 0xab,
            }]
            .into(),
        };

    let lifted = try_lift_zisk_main_next_segment_seed_from_runner_boundary(
        1,
        ZiskMainTraceSegmentInfo {
            trace_instance_index: 0,
            is_last_segment: false,
            previous_c: current_seed.previous_c,
        },
        std::slice::from_ref(&report),
        None,
        &runner_state,
        &current_seed,
    )
    .expect("retained load boundary seed lift should evaluate")
    .expect("retained load boundary should use report memory read");

    assert_eq!(lifted.previous_c, 0xab);
    assert_eq!(lifted.initial_state.last_c, 0xab);
}

#[test]
fn runner_boundary_seed_snapshot_uses_retained_report_for_zero_register_sign_extend_load_boundary()
{
    let current_seed = ZiskMainSegmentSeed::new();
    let runner_state = GuestMachineState::new(0x8000_0004);
    let report =
        GuestMachineReport {
            address_and_instruction_len:
                crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0000, 4),
            instruction: RiscvInstruction::Load {
                kind: RiscvLoadKind::Lb,
                rd: 0,
                rs1: 10,
                offset: 5,
            },
            next_pc: 0x8000_0004,
            register_write_value: GuestRegisterWriteValue::default(),
            memory_accesses: vec![GuestMemoryAccess {
                kind: GuestMemoryAccessKind::Read,
                address: 0x1005,
                byte_len: 1,
                value: 0xff,
            }]
            .into(),
        };

    let lifted = try_lift_zisk_main_next_segment_seed_from_runner_boundary(
        1,
        ZiskMainTraceSegmentInfo {
            trace_instance_index: 0,
            is_last_segment: false,
            previous_c: current_seed.previous_c,
        },
        std::slice::from_ref(&report),
        None,
        &runner_state,
        &current_seed,
    )
    .expect("retained sign-extended load boundary seed lift should evaluate")
    .expect("retained sign-extended load boundary should use report memory read");

    assert_eq!(lifted.previous_c, u64::MAX);
    assert_eq!(lifted.initial_state.last_c, u64::MAX);
}

#[test]
fn runner_boundary_seed_snapshot_uses_pending_dma_add_register_boundary_without_retained_report() {
    let current_seed = ZiskMainSegmentSeed::new();
    let mut runner_state = GuestMachineState::new(0x8000_0008);
    runner_state
        .set_register(9, 0x9000_1234_5678_abcd)
        .expect("destination register should set");
    let mut boundary_snapshot = ZiskMainRunnerBoundarySnapshot::new(&current_seed);
    boundary_snapshot.record_report_shape_state(GuestMachineReportShape {
        instruction: RiscvInstruction::ZiskDmaPrepare {
            kind: RiscvDmaKind::Memcpy,
            rs1: 5,
        },
        has_memory_write: false,
    });
    let execute_shape = GuestMachineReportShape {
        instruction: RiscvInstruction::Op {
            kind: RiscvOpKind::Add,
            rd: 9,
            rs1: 10,
            rs2: 11,
        },
        has_memory_write: true,
    };
    boundary_snapshot.record_report_shape_state(execute_shape);

    let lifted = try_lift_zisk_main_next_segment_seed_from_runner_boundary_snapshot(
        2,
        ZiskMainTraceSegmentInfo {
            trace_instance_index: 0,
            is_last_segment: false,
            previous_c: current_seed.previous_c,
        },
        ZiskMainRunnerBoundarySeedInput {
            reports: &[],
            report_count: 2,
            last_report_shape: Some(execute_shape),
            lookahead_instruction: Some(RiscvInstruction::OpImm {
                kind: RiscvOpImmKind::Addi,
                rd: 0,
                rs1: 0,
                immediate: 0,
            }),
            runner_state: &runner_state,
            current_seed: &current_seed,
            boundary_snapshot: &boundary_snapshot,
        },
    )
    .expect("pending-DMA boundary seed lift should evaluate")
    .expect("pending-DMA add boundary should use runner destination register");

    assert_eq!(lifted.previous_c, 0x9000_1234_5678_abcd);
    assert_eq!(lifted.initial_state.last_c, 0x9000_1234_5678_abcd);
    assert_eq!(lifted.initial_state.pending_dma, None);
}

#[test]
fn runner_boundary_seed_snapshot_uses_pending_dma_addi_register_boundary_without_retained_report() {
    let current_seed = ZiskMainSegmentSeed::new();
    let mut runner_state = GuestMachineState::new(0x8000_0008);
    runner_state
        .set_register(14, 0xfeed_face_cafe_babe)
        .expect("destination register should set");
    let mut boundary_snapshot = ZiskMainRunnerBoundarySnapshot::new(&current_seed);
    boundary_snapshot.record_report_shape_state(GuestMachineReportShape {
        instruction: RiscvInstruction::ZiskDmaPrepare {
            kind: RiscvDmaKind::Memset,
            rs1: 5,
        },
        has_memory_write: false,
    });
    let execute_shape = GuestMachineReportShape {
        instruction: RiscvInstruction::OpImm {
            kind: RiscvOpImmKind::Addi,
            rd: 14,
            rs1: 10,
            immediate: 7,
        },
        has_memory_write: true,
    };
    boundary_snapshot.record_report_shape_state(execute_shape);

    let lifted = try_lift_zisk_main_next_segment_seed_from_runner_boundary_snapshot(
        2,
        ZiskMainTraceSegmentInfo {
            trace_instance_index: 0,
            is_last_segment: false,
            previous_c: current_seed.previous_c,
        },
        ZiskMainRunnerBoundarySeedInput {
            reports: &[],
            report_count: 2,
            last_report_shape: Some(execute_shape),
            lookahead_instruction: Some(RiscvInstruction::OpImm {
                kind: RiscvOpImmKind::Addi,
                rd: 0,
                rs1: 0,
                immediate: 0,
            }),
            runner_state: &runner_state,
            current_seed: &current_seed,
            boundary_snapshot: &boundary_snapshot,
        },
    )
    .expect("pending-DMA boundary seed lift should evaluate")
    .expect("pending-DMA addi boundary should use runner destination register");

    assert_eq!(lifted.previous_c, 0xfeed_face_cafe_babe);
    assert_eq!(lifted.initial_state.last_c, 0xfeed_face_cafe_babe);
    assert_eq!(lifted.initial_state.pending_dma, None);
}

#[test]
fn runner_boundary_seed_snapshot_uses_pending_dma_add_zero_register_destination() {
    let current_seed = ZiskMainSegmentSeed::new();
    let mut runner_state = GuestMachineState::new(0x8000_0008);
    runner_state
        .set_register(10, 0x9000_1234_5678_abcd)
        .expect("destination source register should set");
    let mut boundary_snapshot = ZiskMainRunnerBoundarySnapshot::new(&current_seed);
    boundary_snapshot.record_report_shape_state(GuestMachineReportShape {
        instruction: RiscvInstruction::ZiskDmaPrepare {
            kind: RiscvDmaKind::Memcpy,
            rs1: 5,
        },
        has_memory_write: false,
    });
    let execute_shape = GuestMachineReportShape {
        instruction: RiscvInstruction::Op {
            kind: RiscvOpKind::Add,
            rd: 0,
            rs1: 10,
            rs2: 11,
        },
        has_memory_write: true,
    };
    boundary_snapshot.record_report_shape_state(execute_shape);

    let result = try_lift_zisk_main_next_segment_seed_from_runner_boundary_snapshot(
        2,
        ZiskMainTraceSegmentInfo {
            trace_instance_index: 0,
            is_last_segment: false,
            previous_c: current_seed.previous_c,
        },
        ZiskMainRunnerBoundarySeedInput {
            reports: &[],
            report_count: 2,
            last_report_shape: Some(execute_shape),
            lookahead_instruction: Some(RiscvInstruction::OpImm {
                kind: RiscvOpImmKind::Addi,
                rd: 0,
                rs1: 0,
                immediate: 0,
            }),
            runner_state: &runner_state,
            current_seed: &current_seed,
            boundary_snapshot: &boundary_snapshot,
        },
    )
    .expect("pending-DMA boundary seed lift should evaluate");

    let lifted = result.expect("zero-destination pending DMA should use destination source");
    assert_eq!(lifted.previous_c, 0x9000_1234_5678_abcd);
    assert_eq!(lifted.initial_state.last_c, 0x9000_1234_5678_abcd);
    assert_eq!(lifted.initial_state.pending_dma, None);
}

#[test]
fn runner_boundary_seed_snapshot_keeps_pending_memcmp_zero_register_boundary_unavailable() {
    let current_seed = ZiskMainSegmentSeed::new();
    let runner_state = GuestMachineState::new(0x8000_0008);
    let mut boundary_snapshot = ZiskMainRunnerBoundarySnapshot::new(&current_seed);
    boundary_snapshot.record_report_shape_state(GuestMachineReportShape {
        instruction: RiscvInstruction::ZiskDmaPrepare {
            kind: RiscvDmaKind::Memcmp,
            rs1: 5,
        },
        has_memory_write: false,
    });
    let execute_shape = GuestMachineReportShape {
        instruction: RiscvInstruction::Op {
            kind: RiscvOpKind::Add,
            rd: 0,
            rs1: 10,
            rs2: 11,
        },
        has_memory_write: true,
    };
    boundary_snapshot.record_report_shape_state(execute_shape);

    let result = try_lift_zisk_main_next_segment_seed_from_runner_boundary_snapshot(
        2,
        ZiskMainTraceSegmentInfo {
            trace_instance_index: 0,
            is_last_segment: false,
            previous_c: current_seed.previous_c,
        },
        ZiskMainRunnerBoundarySeedInput {
            reports: &[],
            report_count: 2,
            last_report_shape: Some(execute_shape),
            lookahead_instruction: Some(RiscvInstruction::OpImm {
                kind: RiscvOpImmKind::Addi,
                rd: 0,
                rs1: 0,
                immediate: 0,
            }),
            runner_state: &runner_state,
            current_seed: &current_seed,
            boundary_snapshot: &boundary_snapshot,
        },
    )
    .expect("pending-DMA boundary seed lift should evaluate");

    assert_eq!(
        result,
        Err(ZiskMainDirectSeedLiftMissReason::BoundaryCUnavailable)
    );
}

#[test]
fn runner_boundary_seed_snapshot_uses_snapshot_for_amo_boundary_without_retained_report() {
    let current_seed = ZiskMainSegmentSeed::new();
    let mut runner_state = GuestMachineState::new(0x8000_0004);
    runner_state
        .set_register(1, 0x1234_5678_9abc_def0)
        .expect("test register should be writable");
    let mut boundary_snapshot = ZiskMainRunnerBoundarySnapshot::new(&current_seed);
    boundary_snapshot
        .internal_memory
        .insert(
            zisk_internal_register_address_u64(ZISK_AMO_TEMP_REGISTER),
            0x1234_5678_9abc_def0,
        )
        .expect("AMO scratch address should be supported");

    let lifted = try_lift_zisk_main_next_segment_seed_from_runner_boundary_snapshot(
        4,
        ZiskMainTraceSegmentInfo {
            trace_instance_index: 0,
            is_last_segment: false,
            previous_c: current_seed.previous_c,
        },
        ZiskMainRunnerBoundarySeedInput {
            reports: &[],
            report_count: 1,
            last_report_shape: Some(GuestMachineReportShape {
                instruction: RiscvInstruction::Amo {
                    kind: RiscvAmoKind::Add,
                    width: RiscvAmoWidth::Doubleword,
                    rd: 1,
                    rs1: 1,
                    rs2: 2,
                    acquire: false,
                    release: false,
                },
                has_memory_write: true,
            }),
            lookahead_instruction: Some(RiscvInstruction::OpImm {
                kind: RiscvOpImmKind::Addi,
                rd: 0,
                rs1: 0,
                immediate: 0,
            }),
            runner_state: &runner_state,
            current_seed: &current_seed,
            boundary_snapshot: &boundary_snapshot,
        },
    )
    .expect("snapshot-backed AMO seed lift should evaluate")
    .expect("snapshot-backed AMO seed lift should succeed");

    assert_eq!(lifted.initial_state.last_c, 0x1234_5678_9abc_def0);
    assert_eq!(
        lifted
            .initial_state
            .internal_memory
            .get(zisk_internal_register_address_u64(ZISK_AMO_TEMP_REGISTER)),
        Some(0x1234_5678_9abc_def0)
    );
}

#[test]
fn runner_pre_boundary_snapshot_skips_redundant_amo_report_replay() {
    let current_seed = ZiskMainSegmentSeed::new();
    let runner_state = GuestMachineState::new(0x8000_0004);
    let mut boundary_snapshot = ZiskMainRunnerBoundarySnapshot::new(&current_seed);
    boundary_snapshot
        .internal_memory
        .insert(
            zisk_internal_register_address_u64(ZISK_AMO_TEMP_REGISTER),
            0x1234_5678_9abc_def0,
        )
        .expect("AMO scratch address should be supported");
    let report =
        GuestMachineReport {
            address_and_instruction_len:
                crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0000, 4),
            instruction: RiscvInstruction::Amo {
                kind: RiscvAmoKind::Add,
                width: RiscvAmoWidth::Doubleword,
                rd: 1,
                rs1: 1,
                rs2: 2,
                acquire: false,
                release: false,
            },
            next_pc: 0x8000_0004,
            register_write_value: GuestRegisterWriteValue::default(),
            memory_accesses: Vec::new().into(),
        };

    record_zisk_main_runner_pre_boundary_snapshot(
        &mut boundary_snapshot,
        Some(&report),
        None,
        Some(RiscvInstruction::OpImm {
            kind: RiscvOpImmKind::Addi,
            rd: 0,
            rs1: 0,
            immediate: 0,
        }),
        runner_state.registers(),
    )
    .expect("pre-boundary snapshot should not replay AMO scratch from the previous report");

    assert_eq!(
        boundary_snapshot
            .internal_memory
            .get(zisk_internal_register_address_u64(ZISK_AMO_TEMP_REGISTER)),
        Some(0x1234_5678_9abc_def0)
    );
}

#[test]
fn runner_pre_boundary_snapshot_keeps_dma_extra_params_update() {
    let current_seed = ZiskMainSegmentSeed::new();
    let mut runner_state = GuestMachineState::new(0x8000_0004);
    runner_state
        .set_register(9, 0xfeed_face_cafe_babe)
        .expect("test register should be writable");
    let mut boundary_snapshot = ZiskMainRunnerBoundarySnapshot::new(&current_seed);
    let report =
        GuestMachineReport {
            address_and_instruction_len:
                crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0000, 4),
            instruction: RiscvInstruction::ZiskDmaPrepare {
                kind: RiscvDmaKind::Memcpy,
                rs1: 5,
            },
            next_pc: 0x8000_0004,
            register_write_value: GuestRegisterWriteValue::default(),
            memory_accesses: Vec::new().into(),
        };

    record_zisk_main_runner_pre_boundary_snapshot(
        &mut boundary_snapshot,
        Some(&report),
        None,
        Some(RiscvInstruction::Op {
            kind: RiscvOpKind::Add,
            rd: 3,
            rs1: 4,
            rs2: 9,
        }),
        runner_state.registers(),
    )
    .expect("pre-boundary snapshot should keep DMA extra-params scratch update");

    assert_eq!(
        boundary_snapshot
            .internal_memory
            .get(ZISK_EXTRA_PARAMS_ADDRESS),
        Some(0xfeed_face_cafe_babe)
    );
}

#[test]
fn live_report_chunk_finish_emits_amo_boundary_without_returning_last_report() {
    let report =
        GuestMachineReport {
            address_and_instruction_len:
                crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0000, 4),
            instruction: RiscvInstruction::Amo {
                kind: RiscvAmoKind::Add,
                width: RiscvAmoWidth::Doubleword,
                rd: 1,
                rs1: 1,
                rs2: 2,
                acquire: false,
                release: false,
            },
            next_pc: 0x8000_0004,
            register_write_value: GuestRegisterWriteValue::new(0x1234_5678_9abc_def0),
            memory_accesses: vec![
                memory_read(0x1000, 0x1234_5678_9abc_def0),
                memory_write(0x1000, 0x1234_5678_9abc_f000),
            ]
            .into(),
        };
    let shape = guest_machine_report_shape_from_report(&report);
    let mut emitted = Vec::new();

    let slice = finish_guest_pc_trace_live_report_chunk_segment_slice(
        Some(report.clone()),
        1,
        &mut |report| {
            emitted.push(report);
            Ok(())
        },
        1,
        4,
        GuestMachineTraceSliceStatus::Paused {
            pc: 0x8000_0004,
            instruction: RiscvInstruction::OpImm {
                kind: RiscvOpImmKind::Addi,
                rd: 0,
                rs1: 0,
                immediate: 0,
            },
        },
        Some(shape),
    )
    .expect("live chunk finish should emit AMO report");

    assert_eq!(emitted, vec![report]);
    assert_eq!(slice.last_report_shape, Some(shape));
}

#[test]
fn runner_boundary_snapshot_does_not_route_amo_report_after_scratch_snapshot() {
    let report =
        GuestMachineReport {
            address_and_instruction_len:
                crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0000, 4),
            instruction: RiscvInstruction::Amo {
                kind: RiscvAmoKind::Add,
                width: RiscvAmoWidth::Doubleword,
                rd: 1,
                rs1: 1,
                rs2: 2,
                acquire: false,
                release: false,
            },
            next_pc: 0x8000_0004,
            register_write_value: GuestRegisterWriteValue::new(0x1234_5678_9abc_def0),
            memory_accesses: vec![
                memory_read(0x1000, 0x1234_5678_9abc_def0),
                memory_write(0x1000, 0x1234_5678_9abc_f000),
            ]
            .into(),
        };
    let shape = guest_machine_report_shape_from_report(&report);

    assert!(
        zisk_main_runner_boundary_report_for_shape(Some(&report), Some(shape)).is_none(),
        "AMO scratch should be captured before the boundary update instead of routing the full report through the boundary"
    );
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
        register_writes: Vec::new().into(),
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
        register_writes: Vec::new().into(),
        memory_accesses: &accesses,
        precompile_memory_accesses: &[],
        precompile_result: None,
    };
    let state = ZiskMainTraceState::new();
    let report = addi_report();

    let error = zisk_main_source_value(
        9,
        ZiskMainSource::Memory(64),
        &state,
        &report,
        effects,
        None,
        0,
        0,
    )
    .expect_err("source values should consume the expected memory access position");

    assert!(error.to_string().contains("expected Read at 64"));
}

#[test]
fn ordered_memory_access_value_returns_value_after_order_validation() {
    let accesses = [memory_read(64, 96), memory_read(104, 13)];
    let effects = ZiskMainReportEffects {
        register_writes: Vec::new().into(),
        memory_accesses: &accesses,
        precompile_memory_accesses: &[],
        precompile_result: None,
    };

    assert_eq!(
        ordered_memory_access_value(9, effects, 1, GuestMemoryAccessKind::Read, 104, 8)
            .expect("ordered access value should be returned"),
        13
    );

    let error = ordered_memory_access_value(9, effects, 0, GuestMemoryAccessKind::Read, 104, 8)
        .expect_err("value helper should keep ordered access validation");
    assert!(error.to_string().contains("expected Read at 104"));
}

#[test]
fn load_copy_fast_path_parts_match_generic_lowering() {
    let report =
        GuestMachineReport {
            address_and_instruction_len:
                crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0000, 4),
            instruction: RiscvInstruction::Load {
                kind: RiscvLoadKind::Ld,
                rd: 3,
                rs1: 2,
                offset: 8,
            },
            next_pc: 0x8000_0004,
            register_write_value: GuestRegisterWriteValue::new(0xaa55),
            memory_accesses: vec![memory_read(0x108, 0xaa55)].into(),
        };

    let (instruction, a_index, b_offset, store_index) =
        load_copy_indirect_register_store_fast_path_parts(3, &report)
            .expect("fast path detection should succeed")
            .expect("ld from register base into register should match");

    assert_eq!(a_index, 2);
    assert_eq!(b_offset, 8);
    assert_eq!(store_index, 3);
    assert_eq!(
        instruction,
        lower_guest_report(&report).expect("generic lowering should match")
    );
}

#[test]
fn load_copy_fast_path_parts_fall_back_for_non_dominant_loads() {
    let mut report =
        GuestMachineReport {
            address_and_instruction_len:
                crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0000, 4),
            instruction: RiscvInstruction::Load {
                kind: RiscvLoadKind::Lw,
                rd: 3,
                rs1: 2,
                offset: 8,
            },
            next_pc: 0x8000_0004,
            register_write_value: GuestRegisterWriteValue::new(0xaa55),
            memory_accesses: vec![memory_read(0x108, 0xaa55)].into(),
        };
    assert!(
        load_copy_indirect_register_store_fast_path_parts(3, &report)
            .expect("signed load should fall back")
            .is_none()
    );

    report.instruction = RiscvInstruction::Load {
        kind: RiscvLoadKind::Ld,
        rd: 0,
        rs1: 2,
        offset: 8,
    };
    assert!(
        load_copy_indirect_register_store_fast_path_parts(3, &report)
            .expect("zero destination should fall back")
            .is_none()
    );

    report.instruction = RiscvInstruction::Load {
        kind: RiscvLoadKind::Ld,
        rd: 3,
        rs1: 2,
        offset: 8,
    };
    report.next_pc = 0x8000_0008;
    assert!(
        load_copy_indirect_register_store_fast_path_parts(3, &report)
            .expect("non-sequential load should fall back")
            .is_none()
    );
}

#[test]
fn load_reserved_fast_path_parts_match_generic_lowering() {
    for (width, expect_copy) in [
        (RiscvAmoWidth::Word, false),
        (RiscvAmoWidth::Doubleword, true),
    ] {
        let report = GuestMachineReport {
            address_and_instruction_len:
                crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0000, 4),
            instruction: RiscvInstruction::LoadReserved {
                width,
                rd: 3,
                rs1: 2,
                acquire: true,
                release: false,
            },
            next_pc: 0x8000_0004,
            register_write_value: GuestRegisterWriteValue::new(0xaa55),
            memory_accesses: vec![memory_read(0x100, 0xaa55)].into(),
        };
        let parts = load_reserved_indirect_register_store_fast_path_parts(3, &report)
            .expect("fast path detection should succeed")
            .expect("load-reserved from register base into register should match");
        let instruction = match (expect_copy, parts) {
            (true, MainReportFastPathParts::LoadCopy(instruction, ..))
            | (false, MainReportFastPathParts::LoadSignExtend(instruction, ..)) => instruction,
            _ => panic!("load-reserved should route through the expected load fast path"),
        };
        assert_eq!(
            instruction,
            lower_guest_report(&report).expect("generic lowering should match")
        );
    }
}

#[test]
fn load_sign_extend_fast_path_parts_match_generic_lowering() {
    let mut access = memory_read(0x108, 0xffff_ff80);
    access.byte_len = 4;
    let report =
        GuestMachineReport {
            address_and_instruction_len:
                crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0000, 4),
            instruction: RiscvInstruction::Load {
                kind: RiscvLoadKind::Lw,
                rd: 3,
                rs1: 2,
                offset: 8,
            },
            next_pc: 0x8000_0004,
            register_write_value: GuestRegisterWriteValue::new(0xffff_ffff_ffff_ff80),
            memory_accesses: vec![access].into(),
        };

    let (instruction, a_index, b_offset, store_index) =
        load_sign_extend_indirect_register_store_fast_path_parts(3, &report)
            .expect("fast path detection should succeed")
            .expect("signed load from register base into register should match");

    assert_eq!(a_index, 2);
    assert_eq!(b_offset, 8);
    assert_eq!(store_index, 3);
    assert_eq!(
        instruction,
        lower_guest_report(&report).expect("generic lowering should match")
    );
    assert_eq!(
        sign_extend_indirect_register_store_fast_path_parts(
            &instruction,
            ZiskMainReportEffects::from_report(&report),
        ),
        Some((2, 8, 3))
    );
}

#[test]
fn load_sign_extend_fast_path_parts_fall_back_for_non_dominant_loads() {
    let mut access = memory_read(0x108, 0xaa55);
    access.byte_len = 4;
    let mut report =
        GuestMachineReport {
            address_and_instruction_len:
                crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0000, 4),
            instruction: RiscvInstruction::Load {
                kind: RiscvLoadKind::Lwu,
                rd: 3,
                rs1: 2,
                offset: 8,
            },
            next_pc: 0x8000_0004,
            register_write_value: GuestRegisterWriteValue::new(0xaa55),
            memory_accesses: vec![access].into(),
        };
    assert!(
        load_sign_extend_indirect_register_store_fast_path_parts(3, &report)
            .expect("unsigned load should fall back")
            .is_none()
    );

    report.instruction = RiscvInstruction::Load {
        kind: RiscvLoadKind::Lw,
        rd: 0,
        rs1: 2,
        offset: 8,
    };
    assert!(
        load_sign_extend_indirect_register_store_fast_path_parts(3, &report)
            .expect("zero destination should fall back")
            .is_none()
    );

    report.instruction = RiscvInstruction::Load {
        kind: RiscvLoadKind::Lw,
        rd: 3,
        rs1: 2,
        offset: 8,
    };
    report.next_pc = 0x8000_0008;
    assert!(
        load_sign_extend_indirect_register_store_fast_path_parts(3, &report)
            .expect("non-sequential load should fall back")
            .is_none()
    );
}

#[test]
fn store_copy_fast_path_parts_match_generic_lowering() {
    let report =
        GuestMachineReport {
            address_and_instruction_len:
                crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0000, 4),
            instruction: RiscvInstruction::Store {
                kind: RiscvStoreKind::Sd,
                rs1: 2,
                rs2: 3,
                offset: 8,
            },
            next_pc: 0x8000_0004,
            register_write_value: GuestRegisterWriteValue::default(),
            memory_accesses: vec![memory_write(0x108, 0xaa55)].into(),
        };

    let parts = store_copy_indirect_store_fast_path_parts(3, &report)
        .expect("fast path detection should succeed")
        .expect("register store through register base should match");
    let MainReportFastPathParts::StoreCopy(instruction, a_index, b_index, store_offset) = parts
    else {
        panic!("register store should route to register-source store copy");
    };

    assert_eq!(a_index, 2);
    assert_eq!(b_index, 3);
    assert_eq!(store_offset, 8);
    assert_eq!(
        instruction,
        lower_guest_report(&report).expect("generic lowering should match")
    );

    let zero_report =
        GuestMachineReport {
            address_and_instruction_len:
                crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0000, 4),
            instruction: RiscvInstruction::Store {
                kind: RiscvStoreKind::Sd,
                rs1: 2,
                rs2: 0,
                offset: 8,
            },
            next_pc: 0x8000_0004,
            register_write_value: GuestRegisterWriteValue::default(),
            memory_accesses: vec![memory_write(0x108, 0)].into(),
        };
    let parts = store_copy_indirect_store_fast_path_parts(3, &zero_report)
        .expect("fast path detection should succeed")
        .expect("zero store through register base should match");
    let MainReportFastPathParts::StoreImmediateCopy(instruction, a_index, b, store_offset) = parts
    else {
        panic!("zero store should route to immediate-source store copy");
    };

    assert_eq!(a_index, 2);
    assert_eq!(b, 0);
    assert_eq!(store_offset, 8);
    assert_eq!(
        instruction,
        lower_guest_report(&zero_report).expect("generic lowering should match")
    );
}

#[test]
fn store_copy_fast_path_parts_fall_back_for_non_dominant_stores() {
    let mut report =
        GuestMachineReport {
            address_and_instruction_len:
                crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0000, 4),
            instruction: RiscvInstruction::Store {
                kind: RiscvStoreKind::Sd,
                rs1: 0,
                rs2: 3,
                offset: 8,
            },
            next_pc: 0x8000_0004,
            register_write_value: GuestRegisterWriteValue::default(),
            memory_accesses: vec![memory_write(0x108, 0xaa55)].into(),
        };
    assert!(store_copy_indirect_store_fast_path_parts(3, &report)
        .expect("zero base register should fall back")
        .is_none());

    report.instruction = RiscvInstruction::Store {
        kind: RiscvStoreKind::Sd,
        rs1: 2,
        rs2: 3,
        offset: 8,
    };
    report.replace_precompile_effects(GuestPrecompileReportEffects::from_parts(
        vec![memory_read(64, 7)].into(),
        None,
    ));
    assert!(store_copy_indirect_store_fast_path_parts(3, &report)
        .expect("rows with precompile accesses should fall back")
        .is_none());
    report.replace_precompile_effects(None);

    report.instruction = RiscvInstruction::Store {
        kind: RiscvStoreKind::Sd,
        rs1: 2,
        rs2: 3,
        offset: 8,
    };
    report.next_pc = 0x8000_0008;
    assert!(store_copy_indirect_store_fast_path_parts(3, &report)
        .expect("non-sequential store should fall back")
        .is_none());
}

#[test]
fn copy_register_indirect_store_fast_path_preserves_row_effects() {
    let accesses = [memory_write(0x108, 0xaa55)];
    let effects = ZiskMainReportEffects {
        register_writes: Vec::new().into(),
        memory_accesses: &accesses,
        precompile_memory_accesses: &[],
        precompile_result: None,
    };
    let instruction = ZiskMainInstruction {
        pc: 0x8000_0000,
        a: ZiskMainSource::Register(2),
        b: ZiskMainSource::Register(3),
        op: ZiskMainOp::CopyB,
        store: ZiskMainStore::Indirect(8),
        store_pc: false,
        set_pc: false,
        jmp_offset1: 4,
        jmp_offset2: 4,
        ind_width: 8,
        m32: false,
        is_external_op: false,
        is_precompiled: false,
    };
    let mut state = ZiskMainTraceState::new();
    state.registers[2] = 0x100;
    state.registers[3] = 0xaa55;
    state.register_mem_steps[2] = 33;
    state.register_mem_steps[3] = 44;
    let mut context = ZiskMainReportValidationContext::new(
        None,
        16,
        ZiskMainTraceSegmentInfo {
            trace_instance_index: 0,
            is_last_segment: false,
            previous_c: 0,
        },
    )
    .expect("context should initialize");
    let mut visited = None;
    apply_copy_register_indirect_store_fast_path(
        3,
        instruction,
        effects,
        0x8000_0004,
        2,
        3,
        8,
        &mut state,
        &mut context,
        &mut |row, values, timing| {
            assert_eq!(row, 3);
            assert!(timing.is_none());
            visited = Some(values);
            Ok(())
        },
    )
    .expect("dominant store row shape should take fast path");

    assert_eq!(state.registers[2], 0x100);
    assert_eq!(state.registers[3], 0xaa55);
    assert_eq!(state.last_c, 0xaa55);
    assert_eq!(state.next_pc, 0x8000_0004);
    assert_eq!(state.register_mem_steps[2], 13);
    assert_eq!(state.register_mem_steps[3], 14);
    let values = visited.expect("fast path should emit row values");
    assert_eq!(values.a, 0x100);
    assert_eq!(values.b, 0xaa55);
    assert_eq!(values.c, 0xaa55);
    assert!(!values.flag);
    assert_eq!(values.register_accesses.a_prev_mem_step, Some(33));
    assert_eq!(values.register_accesses.b_prev_mem_step, Some(44));
    assert_eq!(values.register_accesses.store_prev_mem_step, None);
    assert_eq!(values.register_accesses.store_prev_value, None);
}

#[test]
fn copy_indirect_no_store_fast_path_preserves_row_effects() {
    let accesses = [memory_read(0x108, 0xaa55)];
    let effects = ZiskMainReportEffects {
        register_writes: Vec::new().into(),
        memory_accesses: &accesses,
        precompile_memory_accesses: &[],
        precompile_result: None,
    };
    let instruction = ZiskMainInstruction {
        pc: 0x8000_0000,
        a: ZiskMainSource::Register(2),
        b: ZiskMainSource::Indirect(8),
        op: ZiskMainOp::CopyB,
        store: ZiskMainStore::None,
        store_pc: false,
        set_pc: false,
        jmp_offset1: 4,
        jmp_offset2: 4,
        ind_width: 8,
        m32: false,
        is_external_op: false,
        is_precompiled: false,
    };
    let mut state = ZiskMainTraceState::new();
    state.registers[2] = 0x100;
    state.register_mem_steps[2] = 33;
    let mut context = ZiskMainReportValidationContext::new(
        None,
        16,
        ZiskMainTraceSegmentInfo {
            trace_instance_index: 0,
            is_last_segment: false,
            previous_c: 0,
        },
    )
    .expect("context should initialize");
    let mut visited = None;
    apply_copy_indirect_no_store_fast_path(
        3,
        instruction,
        effects,
        0x8000_0004,
        2,
        8,
        8,
        &mut state,
        &mut context,
        &mut |row, values, timing| {
            assert_eq!(row, 3);
            assert!(timing.is_none());
            visited = Some(values);
            Ok(())
        },
    )
    .expect("load row with no store should take fast path");

    assert_eq!(state.registers[2], 0x100);
    assert_eq!(state.last_c, 0xaa55);
    assert_eq!(state.next_pc, 0x8000_0004);
    assert_eq!(state.register_mem_steps[2], 13);
    let values = visited.expect("fast path should emit row values");
    assert_eq!(values.a, 0x100);
    assert_eq!(values.b, 0xaa55);
    assert_eq!(values.c, 0xaa55);
    assert!(!values.flag);
    assert_eq!(values.register_accesses.a_prev_mem_step, Some(33));
    assert_eq!(values.register_accesses.b_prev_mem_step, None);
    assert_eq!(values.register_accesses.store_prev_mem_step, None);
    assert_eq!(values.register_accesses.store_prev_value, None);
}

#[test]
fn precompile_no_store_fast_path_preserves_row_effects() {
    let report = fixed_precompile_report(0x8000_0000, RiscvPrecompileKind::Keccak);
    let instruction = lower_guest_report(&report).expect("generic lowering should succeed");
    let effects = ZiskMainReportEffects::from_report(&report);
    let mut state = ZiskMainTraceState::new();
    state.registers[2] = 0x1000;
    state.register_mem_steps[2] = 44;
    let mut context = ZiskMainReportValidationContext::new(
        None,
        16,
        ZiskMainTraceSegmentInfo {
            trace_instance_index: 0,
            is_last_segment: false,
            previous_c: 0,
        },
    )
    .expect("context should initialize");
    let mut visited = None;

    apply_precompile_no_store_fast_path(
        3,
        &report,
        instruction,
        effects,
        0x8000_0004,
        Some(2),
        &mut state,
        &mut context,
        &mut |row, values, timing| {
            assert_eq!(row, 3);
            assert!(timing.is_none());
            visited = Some(values);
            Ok(())
        },
    )
    .expect("fixed precompile row with no store should take fast path");

    assert_eq!(state.registers[2], 0x1000);
    assert_eq!(state.last_c, 0);
    assert_eq!(state.next_pc, 0x8000_0004);
    assert_eq!(state.register_mem_steps[2], 14);
    let values = visited.expect("fast path should emit row values");
    assert_eq!(values.a, 0);
    assert_eq!(values.b, 0x1000);
    assert_eq!(values.c, 0);
    assert!(!values.flag);
    assert_eq!(values.register_accesses.a_prev_mem_step, None);
    assert_eq!(values.register_accesses.b_prev_mem_step, Some(44));
    assert_eq!(values.register_accesses.store_prev_mem_step, None);
    assert_eq!(values.register_accesses.store_prev_value, None);
}

#[test]
fn fixed_precompile_no_store_fast_path_rejects_add256_and_register_store() {
    let add256 = add256_report();
    assert!(
        fixed_precompile_no_store_fast_path_parts(3, &add256)
            .expect("Add256 detector should return cleanly")
            .is_none(),
        "Add256 has a material result and must use generic lowering"
    );

    let mut stored = fixed_precompile_report(0x8000_0000, RiscvPrecompileKind::Keccak);
    stored.instruction = RiscvInstruction::ZiskPrecompile {
        kind: RiscvPrecompileKind::Keccak,
        rs1: 2,
        rd: 3,
    };
    assert!(
        fixed_precompile_no_store_fast_path_parts(3, &stored)
            .expect("stored fixed precompile detector should return cleanly")
            .is_none(),
        "fixed precompile rows with a register destination must use generic lowering"
    );
}

#[test]
fn copy_immediate_indirect_store_fast_path_preserves_row_effects() {
    let accesses = [memory_write(0x108, 0x1122_3344_5566_7788)];
    let effects = ZiskMainReportEffects {
        register_writes: Vec::new().into(),
        memory_accesses: &accesses,
        precompile_memory_accesses: &[],
        precompile_result: None,
    };
    let instruction = ZiskMainInstruction {
        pc: 0x8000_0000,
        a: ZiskMainSource::Register(2),
        b: ZiskMainSource::Immediate(0x1122_3344_5566_7788),
        op: ZiskMainOp::CopyB,
        store: ZiskMainStore::Indirect(8),
        store_pc: false,
        set_pc: false,
        jmp_offset1: 4,
        jmp_offset2: 4,
        ind_width: 8,
        m32: false,
        is_external_op: false,
        is_precompiled: false,
    };
    let parts = copy_immediate_indirect_store_fast_path_parts(&instruction, effects)
        .expect("immediate store through register base should match");
    assert_eq!(parts, (2, 0x1122_3344_5566_7788, 8));

    let mut state = ZiskMainTraceState::new();
    state.registers[2] = 0x100;
    state.register_mem_steps[2] = 33;
    let mut context = ZiskMainReportValidationContext::new(
        None,
        16,
        ZiskMainTraceSegmentInfo {
            trace_instance_index: 0,
            is_last_segment: false,
            previous_c: 0,
        },
    )
    .expect("context should initialize");
    let mut visited = None;
    apply_copy_immediate_indirect_store_fast_path(
        3,
        instruction,
        effects,
        0x8000_0004,
        2,
        0x1122_3344_5566_7788,
        8,
        &mut state,
        &mut context,
        &mut |row, values, timing| {
            assert_eq!(row, 3);
            assert!(timing.is_none());
            visited = Some(values);
            Ok(())
        },
    )
    .expect("immediate store row should take fast path");

    assert_eq!(state.registers[2], 0x100);
    assert_eq!(state.last_c, 0x1122_3344_5566_7788);
    assert_eq!(state.next_pc, 0x8000_0004);
    assert_eq!(state.register_mem_steps[2], 13);
    let values = visited.expect("fast path should emit row values");
    assert_eq!(values.a, 0x100);
    assert_eq!(values.b, 0x1122_3344_5566_7788);
    assert_eq!(values.c, 0x1122_3344_5566_7788);
    assert!(!values.flag);
    assert_eq!(values.register_accesses.a_prev_mem_step, Some(33));
    assert_eq!(values.register_accesses.b_prev_mem_step, None);
    assert_eq!(values.register_accesses.store_prev_mem_step, None);
    assert_eq!(values.register_accesses.store_prev_value, None);
}

#[test]
fn simple_copy_fast_path_parts_match_generic_lowering() {
    let reports = [
        GuestMachineReport {
            address_and_instruction_len:
                crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0000, 4),
            instruction: RiscvInstruction::OpImm {
                kind: RiscvOpImmKind::Addi,
                rd: 3,
                rs1: 2,
                immediate: 0,
            },
            next_pc: 0x8000_0004,
            register_write_value: GuestRegisterWriteValue::new(0xaa55),
            memory_accesses: vec![].into(),
        },
        GuestMachineReport {
            address_and_instruction_len:
                crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0004, 4),
            instruction: RiscvInstruction::OpImm {
                kind: RiscvOpImmKind::Addi,
                rd: 4,
                rs1: 0,
                immediate: -7,
            },
            next_pc: 0x8000_0008,
            register_write_value: GuestRegisterWriteValue::new((-7_i64) as u64),
            memory_accesses: vec![].into(),
        },
        GuestMachineReport {
            address_and_instruction_len:
                crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0008, 4),
            instruction: RiscvInstruction::Lui {
                rd: 5,
                immediate: 0x1234_5000,
            },
            next_pc: 0x8000_000c,
            register_write_value: GuestRegisterWriteValue::new(0x1234_5000),
            memory_accesses: vec![].into(),
        },
    ];

    for report in reports {
        let (instruction, _b_index, store_index) =
            simple_copy_register_store_fast_path_parts(3, &report)
                .expect("fast path detection should succeed")
                .expect("simple register copy should match");
        assert!(store_index != 0);
        assert_eq!(
            instruction,
            lower_guest_report(&report).expect("generic lowering should match")
        );
    }
}

#[test]
fn simple_copy_fast_path_parts_fall_back_for_non_copy_rows() {
    let mut report =
        GuestMachineReport {
            address_and_instruction_len:
                crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0000, 4),
            instruction: RiscvInstruction::OpImm {
                kind: RiscvOpImmKind::Addi,
                rd: 3,
                rs1: 2,
                immediate: 8,
            },
            next_pc: 0x8000_0004,
            register_write_value: GuestRegisterWriteValue::new(0xaa55),
            memory_accesses: vec![].into(),
        };
    assert!(simple_copy_register_store_fast_path_parts(3, &report)
        .expect("real add should fall back")
        .is_none());

    report.instruction = RiscvInstruction::OpImm {
        kind: RiscvOpImmKind::Addi,
        rd: 0,
        rs1: 2,
        immediate: 0,
    };
    assert!(simple_copy_register_store_fast_path_parts(3, &report)
        .expect("zero destination should fall back")
        .is_none());

    report.instruction = RiscvInstruction::OpImm {
        kind: RiscvOpImmKind::Addi,
        rd: 3,
        rs1: 2,
        immediate: 0,
    };
    report.next_pc = 0x8000_0008;
    assert!(simple_copy_register_store_fast_path_parts(3, &report)
        .expect("non-sequential copy should fall back")
        .is_none());
}

#[test]
fn report_level_fast_path_parts_routes_representative_rows() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum ReportLevelRoute {
        LoadCopy,
        LoadSignExtend,
        StoreCopy,
        NoMemory,
        SimpleCopy,
        FcallResult,
        Jump,
    }

    let mut signed_load = memory_read(0x108, 0xffff_ff80);
    signed_load.byte_len = 4;
    let mut reserved_word_load = memory_read(0x100, 0xffff_ff80);
    reserved_word_load.byte_len = 4;
    let cases = vec![
        (
            GuestMachineReport {
                address_and_instruction_len:
                    crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0000, 4),
                instruction: RiscvInstruction::Load {
                    kind: RiscvLoadKind::Ld,
                    rd: 3,
                    rs1: 2,
                    offset: 8,
                },
                next_pc: 0x8000_0004,
                register_write_value: GuestRegisterWriteValue::new(0xaa55),
                memory_accesses: vec![memory_read(0x108, 0xaa55)].into(),
            },
            ReportLevelRoute::LoadCopy,
        ),
        (
            GuestMachineReport {
                address_and_instruction_len:
                    crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0034, 4),
                instruction: RiscvInstruction::Load {
                    kind: RiscvLoadKind::Ld,
                    rd: 0,
                    rs1: 2,
                    offset: 8,
                },
                next_pc: 0x8000_0038,
                register_write_value: GuestRegisterWriteValue::default(),
                memory_accesses: vec![memory_read(0x108, 0xaa55)].into(),
            },
            ReportLevelRoute::LoadCopy,
        ),
        (
            GuestMachineReport {
                address_and_instruction_len:
                    crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0004, 4),
                instruction: RiscvInstruction::Load {
                    kind: RiscvLoadKind::Lw,
                    rd: 3,
                    rs1: 2,
                    offset: 8,
                },
                next_pc: 0x8000_0008,
                register_write_value: GuestRegisterWriteValue::new(0xffff_ffff_ffff_ff80),
                memory_accesses: vec![signed_load].into(),
            },
            ReportLevelRoute::LoadSignExtend,
        ),
        (
            GuestMachineReport {
                address_and_instruction_len:
                    crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0038, 4),
                instruction: RiscvInstruction::LoadReserved {
                    width: RiscvAmoWidth::Doubleword,
                    rd: 4,
                    rs1: 2,
                    acquire: false,
                    release: false,
                },
                next_pc: 0x8000_003c,
                register_write_value: GuestRegisterWriteValue::new(0xaa55),
                memory_accesses: vec![memory_read(0x100, 0xaa55)].into(),
            },
            ReportLevelRoute::LoadCopy,
        ),
        (
            GuestMachineReport {
                address_and_instruction_len:
                    crate::guest_machine::pack_report_address_and_instruction_len(0x8000_003c, 4),
                instruction: RiscvInstruction::LoadReserved {
                    width: RiscvAmoWidth::Word,
                    rd: 4,
                    rs1: 2,
                    acquire: false,
                    release: false,
                },
                next_pc: 0x8000_0040,
                register_write_value: GuestRegisterWriteValue::new(0xffff_ffff_ffff_ff80),
                memory_accesses: vec![reserved_word_load].into(),
            },
            ReportLevelRoute::LoadSignExtend,
        ),
        (
            GuestMachineReport {
                address_and_instruction_len:
                    crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0008, 4),
                instruction: RiscvInstruction::Store {
                    kind: RiscvStoreKind::Sd,
                    rs1: 2,
                    rs2: 3,
                    offset: 8,
                },
                next_pc: 0x8000_000c,
                register_write_value: GuestRegisterWriteValue::default(),
                memory_accesses: vec![memory_write(0x108, 0xaa55)].into(),
            },
            ReportLevelRoute::StoreCopy,
        ),
        (
            GuestMachineReport {
                address_and_instruction_len:
                    crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0028, 4),
                instruction: RiscvInstruction::Store {
                    kind: RiscvStoreKind::Sd,
                    rs1: 2,
                    rs2: 0,
                    offset: 8,
                },
                next_pc: 0x8000_002c,
                register_write_value: GuestRegisterWriteValue::default(),
                memory_accesses: vec![memory_write(0x108, 0)].into(),
            },
            ReportLevelRoute::StoreCopy,
        ),
        (
            GuestMachineReport {
                address_and_instruction_len:
                    crate::guest_machine::pack_report_address_and_instruction_len(0x8000_000c, 4),
                instruction: RiscvInstruction::Auipc {
                    rd: 5,
                    immediate: 0x40,
                },
                next_pc: 0x8000_0010,
                register_write_value: GuestRegisterWriteValue::new(0x8000_004c),
                memory_accesses: vec![].into(),
            },
            ReportLevelRoute::Jump,
        ),
        (
            GuestMachineReport {
                address_and_instruction_len:
                    crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0010, 4),
                instruction: RiscvInstruction::Jal { rd: 5, offset: 16 },
                next_pc: 0x8000_0020,
                register_write_value: GuestRegisterWriteValue::new(0x8000_0014),
                memory_accesses: vec![].into(),
            },
            ReportLevelRoute::Jump,
        ),
        (
            GuestMachineReport {
                address_and_instruction_len:
                    crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0014, 4),
                instruction: RiscvInstruction::Branch {
                    kind: RiscvBranchKind::Beq,
                    rs1: 2,
                    rs2: 3,
                    offset: 16,
                },
                next_pc: 0x8000_0018,
                register_write_value: GuestRegisterWriteValue::default(),
                memory_accesses: vec![].into(),
            },
            ReportLevelRoute::NoMemory,
        ),
        (
            GuestMachineReport {
                address_and_instruction_len:
                    crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0018, 4),
                instruction: RiscvInstruction::Fence {
                    kind: RiscvFenceKind::Fence,
                    mode: 0,
                    predecessor: 0,
                    successor: 0,
                },
                next_pc: 0x8000_001c,
                register_write_value: GuestRegisterWriteValue::default(),
                memory_accesses: vec![].into(),
            },
            ReportLevelRoute::NoMemory,
        ),
        (
            fixed_precompile_report(0x8000_0048, RiscvPrecompileKind::Keccak),
            ReportLevelRoute::NoMemory,
        ),
        (
            GuestMachineReport {
                address_and_instruction_len:
                    crate::guest_machine::pack_report_address_and_instruction_len(0x8000_001c, 4),
                instruction: RiscvInstruction::OpImm {
                    kind: RiscvOpImmKind::Addi,
                    rd: 3,
                    rs1: 2,
                    immediate: 0,
                },
                next_pc: 0x8000_0020,
                register_write_value: GuestRegisterWriteValue::new(0xaa55),
                memory_accesses: vec![].into(),
            },
            ReportLevelRoute::SimpleCopy,
        ),
        (
            GuestMachineReport {
                address_and_instruction_len:
                    crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0020, 4),
                instruction: RiscvInstruction::ZiskFcallResult { rd: 6 },
                next_pc: 0x8000_0024,
                register_write_value: GuestRegisterWriteValue::new(0xfeed_face),
                memory_accesses: vec![].into(),
            },
            ReportLevelRoute::FcallResult,
        ),
        (
            GuestMachineReport {
                address_and_instruction_len:
                    crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0024, 4),
                instruction: RiscvInstruction::OpImm {
                    kind: RiscvOpImmKind::Addi,
                    rd: 3,
                    rs1: 2,
                    immediate: 8,
                },
                next_pc: 0x8000_0028,
                register_write_value: GuestRegisterWriteValue::new(0xaa5d),
                memory_accesses: vec![].into(),
            },
            ReportLevelRoute::NoMemory,
        ),
    ];

    let mut timing = GuestPcTraceStreamTiming::default();
    for (report, expected_route) in cases {
        let expected_instruction = lower_guest_report(&report).expect("lowering should succeed");
        let mut next_instruction = || None;
        let parts = report_level_fast_path_parts(3, &report, &mut next_instruction)
            .expect("routing should not fail")
            .expect("row should route to a fast path");
        timing.record_main_report_fast_path(&parts);
        let (actual_route, actual_instruction) = match parts {
            MainReportFastPathParts::LoadCopy(instruction, ..) => {
                (ReportLevelRoute::LoadCopy, instruction)
            }
            MainReportFastPathParts::LoadNoStore(instruction, ..) => {
                (ReportLevelRoute::LoadCopy, instruction)
            }
            MainReportFastPathParts::LoadSignExtend(instruction, ..) => {
                (ReportLevelRoute::LoadSignExtend, instruction)
            }
            MainReportFastPathParts::NoMemory(instruction, _) => {
                (ReportLevelRoute::NoMemory, instruction)
            }
            MainReportFastPathParts::PrecompileNoStore(instruction, ..) => {
                (ReportLevelRoute::NoMemory, instruction)
            }
            MainReportFastPathParts::InternalMemoryCopy(instruction, ..) => {
                (ReportLevelRoute::NoMemory, instruction)
            }
            MainReportFastPathParts::StoreCopy(instruction, ..) => {
                (ReportLevelRoute::StoreCopy, instruction)
            }
            MainReportFastPathParts::StoreImmediateCopy(instruction, ..) => {
                (ReportLevelRoute::StoreCopy, instruction)
            }
            MainReportFastPathParts::SimpleCopy(instruction, ..) => {
                (ReportLevelRoute::SimpleCopy, instruction)
            }
            MainReportFastPathParts::FcallResult(instruction, ..) => {
                (ReportLevelRoute::FcallResult, instruction)
            }
            MainReportFastPathParts::Jump(instruction, _) => (ReportLevelRoute::Jump, instruction),
        };
        assert_eq!(actual_route, expected_route);
        assert_eq!(actual_instruction, expected_instruction);
    }
    let prepare_report =
        GuestMachineReport {
            address_and_instruction_len:
                crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0030, 4),
            instruction: RiscvInstruction::ZiskDmaPrepare {
                kind: RiscvDmaKind::Memcpy,
                rs1: 5,
            },
            next_pc: 0x8000_0034,
            register_write_value: GuestRegisterWriteValue::default(),
            memory_accesses: vec![].into(),
        };
    let prepare_lookahead = RiscvInstruction::Op {
        kind: RiscvOpKind::Add,
        rd: 6,
        rs1: 5,
        rs2: 3,
    };
    let expected_instruction = lower_dma_prepare_report(
        3,
        lower_guest_report(&prepare_report).expect("lowering should succeed"),
        RiscvDmaKind::Memcpy,
        Some(prepare_lookahead),
    )
    .expect("DMA prepare lowering should succeed");
    let mut next_instruction = || Some(prepare_lookahead);
    let parts = report_level_fast_path_parts(3, &prepare_report, &mut next_instruction)
        .expect("routing should not fail")
        .expect("DMA prepare row should route to a fast path");
    timing.record_main_report_fast_path(&parts);
    let MainReportFastPathParts::InternalMemoryCopy(instruction, b_index, store_address) = parts
    else {
        panic!("DMA prepare row should route to internal memory copy");
    };
    assert_eq!(instruction, expected_instruction);
    assert_eq!(b_index, 3);
    assert_eq!(store_address, ZISK_EXTRA_PARAMS_ADDRESS);
    let base_prepare_report =
        GuestMachineReport {
            address_and_instruction_len:
                crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0040, 4),
            instruction: RiscvInstruction::ZiskDmaPrepare {
                kind: RiscvDmaKind::Inputcpy,
                rs1: 5,
            },
            next_pc: 0x8000_0044,
            register_write_value: GuestRegisterWriteValue::default(),
            memory_accesses: vec![].into(),
        };
    let base_lookahead = RiscvInstruction::OpImm {
        kind: RiscvOpImmKind::Addi,
        rd: 6,
        rs1: 5,
        immediate: 8,
    };
    let expected_instruction = lower_dma_prepare_report(
        3,
        lower_guest_report(&base_prepare_report).expect("lowering should succeed"),
        RiscvDmaKind::Inputcpy,
        Some(base_lookahead),
    )
    .expect("DMA prepare lowering should succeed");
    let mut next_instruction = || Some(base_lookahead);
    let parts = report_level_fast_path_parts(3, &base_prepare_report, &mut next_instruction)
        .expect("routing should not fail")
        .expect("DMA prepare row should route to a fast path");
    timing.record_main_report_fast_path(&parts);
    let MainReportFastPathParts::NoMemory(instruction, parts) = parts else {
        panic!("DMA prepare row should route to no-memory copy");
    };
    assert_eq!(instruction, expected_instruction);
    assert_eq!(parts.a_index, None);
    assert_eq!(parts.b_index, Some(5));
    assert_eq!(parts.store_index, None);
    timing.record_main_report_generic_fallback();

    assert_eq!(timing.trace_main_report_fast_path_count(), 17);
    assert_eq!(timing.trace_main_report_generic_fallback_count(), 1);
    assert_eq!(timing.trace_main_report_load_copy_fast_path_count(), 3);
    assert_eq!(
        timing.trace_main_report_load_sign_extend_fast_path_count(),
        2
    );
    assert_eq!(timing.trace_main_report_store_copy_fast_path_count(), 2);
    assert_eq!(timing.trace_main_report_jump_fast_path_count(), 2);
    assert_eq!(timing.trace_main_report_no_memory_fast_path_count(), 6);
    assert_eq!(timing.trace_main_report_simple_copy_fast_path_count(), 1);
    assert_eq!(timing.trace_main_report_fcall_result_fast_path_count(), 1);
}

#[test]
fn fast_path_report_effects_with_known_register_index_match_report_effects() {
    let writing_report =
        GuestMachineReport {
            address_and_instruction_len:
                crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0000, 4),
            instruction: RiscvInstruction::OpImm {
                kind: RiscvOpImmKind::Addi,
                rd: 3,
                rs1: 2,
                immediate: 8,
            },
            next_pc: 0x8000_0004,
            register_write_value: GuestRegisterWriteValue::new(0xaa5d),
            memory_accesses: vec![].into(),
        };
    let store_report =
        GuestMachineReport {
            address_and_instruction_len:
                crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0004, 4),
            instruction: RiscvInstruction::Store {
                kind: RiscvStoreKind::Sd,
                rs1: 2,
                rs2: 3,
                offset: 16,
            },
            next_pc: 0x8000_0008,
            register_write_value: GuestRegisterWriteValue::new(0xdead_beef),
            memory_accesses: vec![GuestMemoryAccess {
                kind: GuestMemoryAccessKind::Write,
                address: 0x1010,
                byte_len: 8,
                value: 0xaa55,
            }]
            .into(),
        };
    let precompile_report = fixed_precompile_report(0x8000_0008, RiscvPrecompileKind::Keccak);

    for (report, register_index) in [
        (&writing_report, Some(3)),
        (&store_report, None),
        (&precompile_report, None),
    ] {
        let derived = ZiskMainReportEffects::from_report(report);
        let known = ZiskMainReportEffects::from_report_with_register_index(report, register_index);
        assert_eq!(known.register_writes, derived.register_writes);
        assert_eq!(known.memory_accesses, derived.memory_accesses);
        assert_eq!(
            known.precompile_memory_accesses,
            derived.precompile_memory_accesses
        );
        assert_eq!(known.precompile_result, derived.precompile_result);
    }
}

#[test]
fn fcall_result_fast_path_parts_match_generic_lowering() {
    let report =
        GuestMachineReport {
            address_and_instruction_len:
                crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0000, 4),
            instruction: RiscvInstruction::ZiskFcallResult { rd: 10 },
            next_pc: 0x8000_0004,
            register_write_value: GuestRegisterWriteValue::new(0xfeed_face),
            memory_accesses: vec![].into(),
        };

    let (instruction, store_index) = fcall_result_register_store_fast_path_parts(3, &report)
        .expect("fast path detection should succeed")
        .expect("returned word into a register should match");

    assert_eq!(store_index, 10);
    assert_eq!(
        instruction,
        lower_guest_report(&report).expect("generic lowering should match")
    );
}

#[test]
fn fcall_result_register_store_fast_path_preserves_row_effects() {
    let report =
        GuestMachineReport {
            address_and_instruction_len:
                crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0000, 4),
            instruction: RiscvInstruction::ZiskFcallResult { rd: 7 },
            next_pc: 0x8000_0004,
            register_write_value: GuestRegisterWriteValue::new(0x1122_3344_5566_7788),
            memory_accesses: vec![].into(),
        };
    let instruction = lower_guest_report(&report).expect("generic lowering should succeed");
    let effects = ZiskMainReportEffects::from_report(&report);
    let mut state = ZiskMainTraceState::new();
    state.registers[7] = 0xaa55;
    state.register_mem_steps[7] = 44;
    let mut context = ZiskMainReportValidationContext::new(
        None,
        16,
        ZiskMainTraceSegmentInfo {
            trace_instance_index: 0,
            is_last_segment: false,
            previous_c: 0,
        },
    )
    .expect("context should initialize");
    let mut visited = None;
    apply_fcall_result_register_store_fast_path(
        3,
        instruction,
        effects,
        0x8000_0004,
        7,
        &mut state,
        &mut context,
        &mut |row, values, timing| {
            assert_eq!(row, 3);
            assert!(timing.is_none());
            visited = Some(values);
            Ok(())
        },
    )
    .expect("returned word row should take fast path");

    assert_eq!(state.registers[7], 0x1122_3344_5566_7788);
    assert_eq!(state.last_c, 0x1122_3344_5566_7788);
    assert_eq!(state.next_pc, 0x8000_0004);
    assert_eq!(state.register_mem_steps[7], 15);
    let values = visited.expect("fast path should emit row values");
    assert_eq!(values.a, 0);
    assert_eq!(values.b, 0x1122_3344_5566_7788);
    assert_eq!(values.c, 0x1122_3344_5566_7788);
    assert!(!values.flag);
    assert_eq!(values.register_accesses.a_prev_mem_step, None);
    assert_eq!(values.register_accesses.b_prev_mem_step, None);
    assert_eq!(values.register_accesses.store_prev_mem_step, Some(44));
    assert_eq!(values.register_accesses.store_prev_value, Some(0xaa55));
}

#[test]
fn simple_copy_register_store_fast_path_preserves_row_effects() {
    let writes = [GuestRegisterWrite {
        index: 3,
        value: 0xaa55,
    }];
    let effects = ZiskMainReportEffects {
        register_writes: writes.to_vec().into(),
        memory_accesses: &[],
        precompile_memory_accesses: &[],
        precompile_result: None,
    };
    let instruction = ZiskMainInstruction {
        pc: 0x8000_0000,
        a: ZiskMainSource::Immediate(0),
        b: ZiskMainSource::Register(3),
        op: ZiskMainOp::CopyB,
        store: ZiskMainStore::Register(3),
        store_pc: false,
        set_pc: false,
        jmp_offset1: 4,
        jmp_offset2: 4,
        ind_width: 0,
        m32: false,
        is_external_op: false,
        is_precompiled: false,
    };
    let mut state = ZiskMainTraceState::new();
    state.registers[3] = 0xaa55;
    state.register_mem_steps[3] = 44;
    let mut context = ZiskMainReportValidationContext::new(
        None,
        16,
        ZiskMainTraceSegmentInfo {
            trace_instance_index: 0,
            is_last_segment: false,
            previous_c: 0,
        },
    )
    .expect("context should initialize");
    let mut visited = None;
    apply_simple_copy_register_store_fast_path(
        3,
        instruction,
        effects,
        0x8000_0004,
        Some(3),
        3,
        &mut state,
        &mut context,
        &mut |row, values, timing| {
            assert_eq!(row, 3);
            assert!(timing.is_none());
            visited = Some(values);
            Ok(())
        },
    )
    .expect("simple copy row should take fast path");

    assert_eq!(state.registers[3], 0xaa55);
    assert_eq!(state.last_c, 0xaa55);
    assert_eq!(state.next_pc, 0x8000_0004);
    assert_eq!(state.register_mem_steps[3], 15);
    let values = visited.expect("fast path should emit row values");
    assert_eq!(values.a, 0);
    assert_eq!(values.b, 0xaa55);
    assert_eq!(values.c, 0xaa55);
    assert!(!values.flag);
    assert_eq!(values.register_accesses.a_prev_mem_step, None);
    assert_eq!(values.register_accesses.b_prev_mem_step, Some(44));
    assert_eq!(values.register_accesses.store_prev_mem_step, Some(14));
    assert_eq!(values.register_accesses.store_prev_value, Some(0xaa55));
}

#[test]
fn internal_memory_copy_fast_path_requires_store_columns() {
    let effects = ZiskMainReportEffects {
        register_writes: Vec::new().into(),
        memory_accesses: &[],
        precompile_memory_accesses: &[],
        precompile_result: None,
    };
    let instruction = ZiskMainInstruction {
        pc: 0x8000_0000,
        a: ZiskMainSource::Immediate(0),
        b: ZiskMainSource::Register(3),
        op: ZiskMainOp::CopyB,
        store: ZiskMainStore::Memory(ZISK_EXTRA_PARAMS_ADDRESS as i64),
        store_pc: false,
        set_pc: false,
        jmp_offset1: 0,
        jmp_offset2: 4,
        ind_width: 0,
        m32: false,
        is_external_op: false,
        is_precompiled: false,
    };
    let mut state = ZiskMainTraceState::new();
    state.registers[3] = 0xaa55;
    let mut context = ZiskMainReportValidationContext {
        columns: None,
        row_count: 16,
        row_mem_step_cursor: GuestPcTraceRowMemStepCursor::new(16, 0)
            .expect("cursor should initialize"),
        b_memory_source_columns_available: true,
        indirect_memory_columns_available: true,
        memory_store_columns_available: false,
    };
    let mut visited = false;
    let error = apply_internal_memory_copy_fast_path(
        3,
        instruction,
        effects,
        0x8000_0004,
        3,
        ZISK_EXTRA_PARAMS_ADDRESS,
        &mut state,
        &mut context,
        &mut |_, _, _| {
            visited = true;
            Ok(())
        },
    )
    .expect_err("copy should reject missing store columns");

    assert!(matches!(
        error,
        GuestPcTraceBackendError::InvalidPcTraceLayout { .. }
    ));
    assert!(!visited);
    assert_eq!(state.internal_memory.get(ZISK_EXTRA_PARAMS_ADDRESS), None);
    assert_eq!(state.last_c, 0);
    assert_eq!(state.next_pc, 0);
}

#[test]
fn sign_extend_indirect_register_store_fast_path_preserves_row_effects() {
    let accesses = [GuestMemoryAccess {
        kind: GuestMemoryAccessKind::Read,
        address: 0x108,
        byte_len: 4,
        value: 0xffff_ff80,
    }];
    let writes = [GuestRegisterWrite {
        index: 3,
        value: 0xffff_ffff_ffff_ff80,
    }];
    let effects = ZiskMainReportEffects {
        register_writes: writes.to_vec().into(),
        memory_accesses: &accesses,
        precompile_memory_accesses: &[],
        precompile_result: None,
    };
    let instruction = ZiskMainInstruction {
        pc: 0x8000_0000,
        a: ZiskMainSource::Register(2),
        b: ZiskMainSource::Indirect(8),
        op: ZiskMainOp::SignExtendW,
        store: ZiskMainStore::Register(3),
        store_pc: false,
        set_pc: false,
        jmp_offset1: 4,
        jmp_offset2: 4,
        ind_width: 4,
        m32: false,
        is_external_op: true,
        is_precompiled: false,
    };
    let mut state = ZiskMainTraceState::new();
    state.registers[2] = 0x100;
    state.registers[3] = 0x77;
    state.register_mem_steps[2] = 33;
    state.register_mem_steps[3] = 44;
    let mut context = ZiskMainReportValidationContext::new(
        None,
        16,
        ZiskMainTraceSegmentInfo {
            trace_instance_index: 0,
            is_last_segment: false,
            previous_c: 0,
        },
    )
    .expect("context should initialize");
    let mut visited = None;
    apply_sign_extend_indirect_register_store_fast_path(
        3,
        instruction,
        effects,
        0x8000_0004,
        2,
        8,
        3,
        &mut state,
        &mut context,
        &mut |row, values, timing| {
            assert_eq!(row, 3);
            assert!(timing.is_none());
            visited = Some(values);
            Ok(())
        },
    )
    .expect("signed load row should take fast path");

    assert_eq!(state.registers[3], 0xffff_ffff_ffff_ff80);
    assert_eq!(state.last_c, 0xffff_ffff_ffff_ff80);
    assert_eq!(state.next_pc, 0x8000_0004);
    assert_eq!(state.register_mem_steps[2], 13);
    assert_eq!(state.register_mem_steps[3], 15);
    let values = visited.expect("fast path should emit row values");
    assert_eq!(values.a, 0x100);
    assert_eq!(values.b, 0xffff_ff80);
    assert_eq!(values.c, 0xffff_ffff_ffff_ff80);
    assert!(!values.flag);
    assert_eq!(values.register_accesses.a_prev_mem_step, Some(33));
    assert_eq!(values.register_accesses.b_prev_mem_step, None);
    assert_eq!(values.register_accesses.store_prev_mem_step, Some(44));
    assert_eq!(values.register_accesses.store_prev_value, Some(0x77));
}

#[test]
fn no_memory_external_fast_path_parts_match_generic_lowering() {
    let report =
        GuestMachineReport {
            address_and_instruction_len:
                crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0000, 4),
            instruction: RiscvInstruction::OpImm {
                kind: RiscvOpImmKind::Ori,
                rd: 3,
                rs1: 2,
                immediate: 0xff,
            },
            next_pc: 0x8000_0004,
            register_write_value: GuestRegisterWriteValue::new(0x1ff),
            memory_accesses: vec![].into(),
        };
    let instruction = lower_guest_report(&report).expect("generic lowering should succeed");

    assert_eq!(
        no_memory_external_fast_path_parts(
            &instruction,
            ZiskMainReportEffects::from_report(&report)
        ),
        Some(ZiskMainNoMemoryFastPathParts {
            a_index: Some(2),
            b_index: None,
            store_index: Some(3),
        })
    );
}

#[test]
fn arithmetic_fast_path_parts_match_generic_lowering() {
    let mut reports = Vec::new();
    for (variant, kind) in [
        RiscvOpImmKind::Addi,
        RiscvOpImmKind::Slti,
        RiscvOpImmKind::Sltiu,
        RiscvOpImmKind::Xori,
        RiscvOpImmKind::Ori,
        RiscvOpImmKind::Andi,
        RiscvOpImmKind::Slli,
        RiscvOpImmKind::Srli,
        RiscvOpImmKind::Srai,
    ]
    .into_iter()
    .enumerate()
    {
        reports.push(GuestMachineReport {
            address_and_instruction_len:
                crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0000, 4),
            instruction: RiscvInstruction::OpImm {
                kind,
                rd: 3,
                rs1: 2,
                immediate: i32::try_from(variant + 1).expect("variant index should fit"),
            },
            next_pc: 0x8000_0004,
            register_write_value: GuestRegisterWriteValue::new(0),
            memory_accesses: vec![].into(),
        });
    }
    for (variant, kind) in [
        RiscvOpImm32Kind::Addiw,
        RiscvOpImm32Kind::Slliw,
        RiscvOpImm32Kind::Srliw,
        RiscvOpImm32Kind::Sraiw,
    ]
    .into_iter()
    .enumerate()
    {
        reports.push(GuestMachineReport {
            address_and_instruction_len:
                crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0000, 4),
            instruction: RiscvInstruction::OpImm32 {
                kind,
                rd: 4,
                rs1: 5,
                immediate: i32::try_from(variant + 1).expect("variant index should fit"),
            },
            next_pc: 0x8000_0004,
            register_write_value: GuestRegisterWriteValue::new(0),
            memory_accesses: vec![].into(),
        });
    }
    for (variant, kind) in [
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
    ]
    .into_iter()
    .enumerate()
    {
        reports.push(GuestMachineReport {
            address_and_instruction_len:
                crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0000, 4),
            instruction: RiscvInstruction::Op {
                kind,
                rd: 6,
                rs1: 7,
                rs2: u8::try_from(8 + variant).expect("variant index should fit"),
            },
            next_pc: 0x8000_0004,
            register_write_value: GuestRegisterWriteValue::new(0),
            memory_accesses: vec![].into(),
        });
    }
    for (variant, kind) in [
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
    ]
    .into_iter()
    .enumerate()
    {
        reports.push(GuestMachineReport {
            address_and_instruction_len:
                crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0000, 4),
            instruction: RiscvInstruction::Op32 {
                kind,
                rd: 9,
                rs1: 10,
                rs2: u8::try_from(11 + variant).expect("variant index should fit"),
            },
            next_pc: 0x8000_0004,
            register_write_value: GuestRegisterWriteValue::new(0),
            memory_accesses: vec![].into(),
        });
    }

    for report in reports {
        let expected = lower_guest_report(&report).expect("generic lowering should succeed");
        let (actual, parts) = arithmetic_fast_path_parts(3, &report)
            .expect("arithmetic matcher should not fail")
            .expect("arithmetic row should match");
        assert_eq!(actual, expected);
        assert_eq!(
            Some(parts),
            no_memory_external_fast_path_parts(
                &expected,
                ZiskMainReportEffects::from_report(&report)
            )
        );
    }
}

#[test]
fn arithmetic_fast_path_skips_noop_register_ops() {
    let report =
        GuestMachineReport {
            address_and_instruction_len:
                crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0000, 4),
            instruction: RiscvInstruction::Op {
                kind: RiscvOpKind::Add,
                rd: 0,
                rs1: 7,
                rs2: 8,
            },
            next_pc: 0x8000_0004,
            register_write_value: GuestRegisterWriteValue::default(),
            memory_accesses: vec![].into(),
        };
    let lowered = lower_guest_report(&report).expect("generic lowering should succeed");

    assert_eq!(lowered.op, ZiskMainOp::Flag);
    assert!(arithmetic_fast_path_parts(3, &report)
        .expect("arithmetic matcher should not fail")
        .is_none());
}

#[test]
fn arithmetic_fast_path_preserves_row_effects() {
    let writes = [GuestRegisterWrite {
        index: 6,
        value: 19,
    }];
    let effects = ZiskMainReportEffects {
        register_writes: writes.to_vec().into(),
        memory_accesses: &[],
        precompile_memory_accesses: &[],
        precompile_result: None,
    };
    let report =
        GuestMachineReport {
            address_and_instruction_len:
                crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0000, 4),
            instruction: RiscvInstruction::Op {
                kind: RiscvOpKind::Add,
                rd: 6,
                rs1: 7,
                rs2: 8,
            },
            next_pc: 0x8000_0004,
            register_write_value: GuestRegisterWriteValue::new(19),
            memory_accesses: vec![].into(),
        };
    let (instruction, parts) = arithmetic_fast_path_parts(3, &report)
        .expect("arithmetic matcher should not fail")
        .expect("arithmetic row should match");
    let mut state = ZiskMainTraceState::new();
    state.registers[6] = 0x77;
    state.registers[7] = 11;
    state.registers[8] = 8;
    state.register_mem_steps[6] = 55;
    state.register_mem_steps[7] = 33;
    state.register_mem_steps[8] = 44;
    let mut context = ZiskMainReportValidationContext::new(
        None,
        16,
        ZiskMainTraceSegmentInfo {
            trace_instance_index: 0,
            is_last_segment: false,
            previous_c: 0,
        },
    )
    .expect("context should initialize");
    let mut visited = None;
    apply_no_memory_fast_path(
        3,
        instruction,
        effects,
        report.next_pc,
        parts,
        &mut state,
        &mut context,
        &mut |row, values, timing| {
            assert_eq!(row, 3);
            assert!(timing.is_none());
            visited = Some(values);
            Ok(())
        },
    )
    .expect("arithmetic row should take fast path");

    assert_eq!(state.registers[6], 19);
    assert_eq!(state.last_c, 19);
    assert_eq!(state.next_pc, 0x8000_0004);
    assert_eq!(state.register_mem_steps[7], 13);
    assert_eq!(state.register_mem_steps[8], 14);
    assert_eq!(state.register_mem_steps[6], 15);
    let values = visited.expect("fast path should emit row values");
    assert_eq!(values.a, 11);
    assert_eq!(values.b, 8);
    assert_eq!(values.c, 19);
    assert!(!values.flag);
    assert_eq!(values.register_accesses.a_prev_mem_step, Some(33));
    assert_eq!(values.register_accesses.b_prev_mem_step, Some(44));
    assert_eq!(values.register_accesses.store_prev_mem_step, Some(55));
    assert_eq!(values.register_accesses.store_prev_value, Some(0x77));
}

#[test]
fn branch_fast_path_parts_match_generic_lowering() {
    let reports =
        [
            RiscvBranchKind::Beq,
            RiscvBranchKind::Bne,
            RiscvBranchKind::Blt,
            RiscvBranchKind::Bge,
            RiscvBranchKind::Bltu,
            RiscvBranchKind::Bgeu,
        ]
        .into_iter()
        .enumerate()
        .map(|(variant, kind)| GuestMachineReport {
            address_and_instruction_len:
                crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0000, 4),
            instruction: RiscvInstruction::Branch {
                kind,
                rs1: u8::try_from(variant + 1).expect("variant index should fit"),
                rs2: u8::try_from(variant + 7).expect("variant index should fit"),
                offset: 16 - i32::try_from(variant).expect("variant index should fit") * 4,
            },
            next_pc: 0x8000_0004,
            register_write_value: GuestRegisterWriteValue::default(),
            memory_accesses: vec![].into(),
        });

    for report in reports {
        let expected = lower_guest_report(&report).expect("generic lowering should succeed");
        let (actual, parts) = branch_fast_path_parts(3, &report)
            .expect("branch matcher should not fail")
            .expect("branch should match");
        assert_eq!(actual, expected);
        assert_eq!(
            Some(parts),
            no_memory_external_fast_path_parts(
                &expected,
                ZiskMainReportEffects::from_report(&report)
            )
        );
    }
}

#[test]
fn taken_branch_fast_path_preserves_row_effects() {
    let effects = ZiskMainReportEffects {
        register_writes: Vec::new().into(),
        memory_accesses: &[],
        precompile_memory_accesses: &[],
        precompile_result: None,
    };
    let report =
        GuestMachineReport {
            address_and_instruction_len:
                crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0000, 4),
            instruction: RiscvInstruction::Branch {
                kind: RiscvBranchKind::Beq,
                rs1: 2,
                rs2: 3,
                offset: 16,
            },
            next_pc: 0x8000_0010,
            register_write_value: GuestRegisterWriteValue::default(),
            memory_accesses: vec![].into(),
        };
    let (instruction, parts) = branch_fast_path_parts(3, &report)
        .expect("branch matcher should not fail")
        .expect("branch should match");
    let mut state = ZiskMainTraceState::new();
    state.registers[2] = 0x55;
    state.registers[3] = 0x55;
    state.register_mem_steps[2] = 33;
    state.register_mem_steps[3] = 44;
    let mut context = ZiskMainReportValidationContext::new(
        None,
        16,
        ZiskMainTraceSegmentInfo {
            trace_instance_index: 0,
            is_last_segment: false,
            previous_c: 0,
        },
    )
    .expect("context should initialize");
    let mut visited = None;
    apply_no_memory_fast_path(
        3,
        instruction,
        effects,
        report.next_pc,
        parts,
        &mut state,
        &mut context,
        &mut |row, values, timing| {
            assert_eq!(row, 3);
            assert!(timing.is_none());
            visited = Some(values);
            Ok(())
        },
    )
    .expect("taken branch row should take fast path");

    assert_eq!(state.last_c, 1);
    assert_eq!(state.next_pc, 0x8000_0010);
    assert_eq!(state.register_mem_steps[2], 13);
    assert_eq!(state.register_mem_steps[3], 14);
    let values = visited.expect("fast path should emit row values");
    assert_eq!(values.a, 0x55);
    assert_eq!(values.b, 0x55);
    assert_eq!(values.c, 1);
    assert!(values.flag);
    assert_eq!(values.register_accesses.a_prev_mem_step, Some(33));
    assert_eq!(values.register_accesses.b_prev_mem_step, Some(44));
    assert_eq!(values.register_accesses.store_prev_mem_step, None);
    assert_eq!(values.register_accesses.store_prev_value, None);
}

#[test]
fn fallthrough_branch_fast_path_preserves_row_effects() {
    let effects = ZiskMainReportEffects {
        register_writes: Vec::new().into(),
        memory_accesses: &[],
        precompile_memory_accesses: &[],
        precompile_result: None,
    };
    let report =
        GuestMachineReport {
            address_and_instruction_len:
                crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0000, 4),
            instruction: RiscvInstruction::Branch {
                kind: RiscvBranchKind::Beq,
                rs1: 4,
                rs2: 5,
                offset: 16,
            },
            next_pc: 0x8000_0004,
            register_write_value: GuestRegisterWriteValue::default(),
            memory_accesses: vec![].into(),
        };
    let (instruction, parts) = branch_fast_path_parts(3, &report)
        .expect("branch matcher should not fail")
        .expect("branch should match");
    let mut state = ZiskMainTraceState::new();
    state.registers[4] = 0x55;
    state.registers[5] = 0x56;
    state.register_mem_steps[4] = 35;
    state.register_mem_steps[5] = 45;
    let mut context = ZiskMainReportValidationContext::new(
        None,
        16,
        ZiskMainTraceSegmentInfo {
            trace_instance_index: 0,
            is_last_segment: false,
            previous_c: 0,
        },
    )
    .expect("context should initialize");
    let mut visited = None;
    apply_no_memory_fast_path(
        3,
        instruction,
        effects,
        report.next_pc,
        parts,
        &mut state,
        &mut context,
        &mut |row, values, timing| {
            assert_eq!(row, 3);
            assert!(timing.is_none());
            visited = Some(values);
            Ok(())
        },
    )
    .expect("fallthrough branch row should take fast path");

    assert_eq!(state.last_c, 0);
    assert_eq!(state.next_pc, 0x8000_0004);
    assert_eq!(state.register_mem_steps[4], 13);
    assert_eq!(state.register_mem_steps[5], 14);
    let values = visited.expect("fast path should emit row values");
    assert_eq!(values.a, 0x55);
    assert_eq!(values.b, 0x56);
    assert_eq!(values.c, 0);
    assert!(!values.flag);
    assert_eq!(values.register_accesses.a_prev_mem_step, Some(35));
    assert_eq!(values.register_accesses.b_prev_mem_step, Some(45));
    assert_eq!(values.register_accesses.store_prev_mem_step, None);
    assert_eq!(values.register_accesses.store_prev_value, None);
}

#[test]
fn jump_fast_path_parts_match_generic_lowering() {
    let reports = [
        GuestMachineReport {
            address_and_instruction_len:
                crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0000, 4),
            instruction: RiscvInstruction::Jal { rd: 5, offset: 16 },
            next_pc: 0x8000_0010,
            register_write_value: GuestRegisterWriteValue::new(0x8000_0004),
            memory_accesses: vec![].into(),
        },
        GuestMachineReport {
            address_and_instruction_len:
                crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0000, 4),
            instruction: RiscvInstruction::Jalr {
                rd: 6,
                rs1: 7,
                offset: -8,
            },
            next_pc: 0x8000_00f8,
            register_write_value: GuestRegisterWriteValue::new(0x8000_0004),
            memory_accesses: vec![].into(),
        },
        GuestMachineReport {
            address_and_instruction_len:
                crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0000, 4),
            instruction: RiscvInstruction::Jal { rd: 0, offset: 12 },
            next_pc: 0x8000_000c,
            register_write_value: GuestRegisterWriteValue::default(),
            memory_accesses: vec![].into(),
        },
    ];

    for report in reports {
        let expected = lower_guest_report(&report).expect("generic lowering should succeed");
        let (actual, _parts) = jump_fast_path_parts(3, &report)
            .expect("jump matcher should not fail")
            .expect("jump should match");
        assert_eq!(actual, expected);
    }
}

#[test]
fn pc_relative_fast_path_parts_match_generic_lowering() {
    let reports = [
        GuestMachineReport {
            address_and_instruction_len:
                crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0000, 4),
            instruction: RiscvInstruction::Auipc {
                rd: 5,
                immediate: 0x40,
            },
            next_pc: 0x8000_0004,
            register_write_value: GuestRegisterWriteValue::new(0x8000_0040),
            memory_accesses: vec![].into(),
        },
        GuestMachineReport {
            address_and_instruction_len:
                crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0020, 4),
            instruction: RiscvInstruction::Auipc {
                rd: 0,
                immediate: -16,
            },
            next_pc: 0x8000_0024,
            register_write_value: GuestRegisterWriteValue::default(),
            memory_accesses: vec![].into(),
        },
    ];

    for report in reports {
        let expected = lower_guest_report(&report).expect("generic lowering should succeed");
        let (actual, _parts) = pc_relative_fast_path_parts(3, &report)
            .expect("pc-relative matcher should not fail")
            .expect("pc-relative row should match");
        assert_eq!(actual, expected);
    }
}

#[test]
fn special_no_memory_fast_path_parts_match_generic_lowering() {
    let reports = [
        GuestMachineReport {
            address_and_instruction_len:
                crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0000, 4),
            instruction: RiscvInstruction::Fence {
                kind: RiscvFenceKind::Fence,
                mode: 0,
                predecessor: 0,
                successor: 0,
            },
            next_pc: 0x8000_0004,
            register_write_value: GuestRegisterWriteValue::default(),
            memory_accesses: vec![].into(),
        },
        GuestMachineReport {
            address_and_instruction_len:
                crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0004, 4),
            instruction: RiscvInstruction::CsrRead {
                csr: RiscvCsr::Mvendorid,
                rd: 3,
            },
            next_pc: 0x8000_0008,
            register_write_value: GuestRegisterWriteValue::new(
                fixed_csr_value(RiscvCsr::Mvendorid).expect("fixed CSR should exist"),
            ),
            memory_accesses: vec![].into(),
        },
        GuestMachineReport {
            address_and_instruction_len:
                crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0008, 4),
            instruction: RiscvInstruction::CsrRead {
                csr: RiscvCsr::Cycle,
                rd: 0,
            },
            next_pc: 0x8000_000c,
            register_write_value: GuestRegisterWriteValue::default(),
            memory_accesses: vec![].into(),
        },
        GuestMachineReport {
            address_and_instruction_len:
                crate::guest_machine::pack_report_address_and_instruction_len(0x8000_000c, 4),
            instruction: RiscvInstruction::ZiskFcallParam { port: 2, rs1: 7 },
            next_pc: 0x8000_0010,
            register_write_value: GuestRegisterWriteValue::default(),
            memory_accesses: vec![].into(),
        },
        GuestMachineReport {
            address_and_instruction_len:
                crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0010, 4),
            instruction: RiscvInstruction::ZiskFcallInvoke { function_id: 9 },
            next_pc: 0x8000_0014,
            register_write_value: GuestRegisterWriteValue::default(),
            memory_accesses: vec![].into(),
        },
        GuestMachineReport {
            address_and_instruction_len:
                crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0014, 4),
            instruction: RiscvInstruction::Lui {
                rd: 0,
                immediate: 0x1234_5000,
            },
            next_pc: 0x8000_0018,
            register_write_value: GuestRegisterWriteValue::default(),
            memory_accesses: vec![].into(),
        },
        GuestMachineReport {
            address_and_instruction_len:
                crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0018, 4),
            instruction: RiscvInstruction::OpImm {
                kind: RiscvOpImmKind::Addi,
                rd: 0,
                rs1: 7,
                immediate: -5,
            },
            next_pc: 0x8000_001c,
            register_write_value: GuestRegisterWriteValue::default(),
            memory_accesses: vec![].into(),
        },
    ];

    for report in reports {
        let expected = lower_guest_report(&report).expect("generic lowering should succeed");
        let (actual, _parts) = special_no_memory_fast_path_parts(3, &report)
            .expect("special no-memory matcher should not fail")
            .expect("special no-memory row should match");
        assert_eq!(actual, expected);
    }
}

#[test]
fn special_no_memory_fast_path_parts_fall_back_for_unsupported_rows() {
    let reports = [
        GuestMachineReport {
            address_and_instruction_len:
                crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0000, 4),
            instruction: RiscvInstruction::CsrRead {
                csr: RiscvCsr::Cycle,
                rd: 3,
            },
            next_pc: 0x8000_0004,
            register_write_value: GuestRegisterWriteValue::new(0),
            memory_accesses: vec![].into(),
        },
        GuestMachineReport {
            address_and_instruction_len:
                crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0000, 4),
            instruction: RiscvInstruction::ZiskFcallParam { port: 20, rs1: 7 },
            next_pc: 0x8000_0004,
            register_write_value: GuestRegisterWriteValue::default(),
            memory_accesses: vec![].into(),
        },
    ];

    for report in reports {
        assert!(special_no_memory_fast_path_parts(3, &report)
            .expect("unsupported special row should not fail during matching")
            .is_none());
    }
}

#[test]
fn pc_relative_fast_path_preserves_row_effects() {
    let writes = [GuestRegisterWrite {
        index: 5,
        value: 0x8000_0040,
    }];
    let effects = ZiskMainReportEffects {
        register_writes: writes.to_vec().into(),
        memory_accesses: &[],
        precompile_memory_accesses: &[],
        precompile_result: None,
    };
    let report =
        GuestMachineReport {
            address_and_instruction_len:
                crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0000, 4),
            instruction: RiscvInstruction::Auipc {
                rd: 5,
                immediate: 0x40,
            },
            next_pc: 0x8000_0004,
            register_write_value: GuestRegisterWriteValue::new(0x8000_0040),
            memory_accesses: vec![].into(),
        };
    let (instruction, parts) = pc_relative_fast_path_parts(3, &report)
        .expect("pc-relative matcher should not fail")
        .expect("pc-relative row should match");
    let mut state = ZiskMainTraceState::new();
    state.registers[5] = 0x77;
    state.register_mem_steps[5] = 44;
    let mut context = ZiskMainReportValidationContext::new(
        None,
        16,
        ZiskMainTraceSegmentInfo {
            trace_instance_index: 0,
            is_last_segment: false,
            previous_c: 0,
        },
    )
    .expect("context should initialize");
    let mut visited = None;
    apply_jump_fast_path(
        3,
        instruction,
        effects,
        report.next_pc,
        parts,
        &mut state,
        &mut context,
        &mut |row, values, timing| {
            assert_eq!(row, 3);
            assert!(timing.is_none());
            visited = Some(values);
            Ok(())
        },
    )
    .expect("pc-relative row should take fast path");

    assert_eq!(state.registers[5], 0x8000_0040);
    assert_eq!(state.last_c, 0);
    assert_eq!(state.next_pc, 0x8000_0004);
    assert_eq!(state.register_mem_steps[5], 15);
    let values = visited.expect("fast path should emit row values");
    assert_eq!(values.a, 0);
    assert_eq!(values.b, 0);
    assert_eq!(values.c, 0);
    assert!(values.flag);
    assert_eq!(values.register_accesses.a_prev_mem_step, None);
    assert_eq!(values.register_accesses.b_prev_mem_step, None);
    assert_eq!(values.register_accesses.store_prev_mem_step, Some(44));
    assert_eq!(values.register_accesses.store_prev_value, Some(0x77));
}

#[test]
fn linked_jump_fast_path_preserves_row_effects() {
    let writes = [GuestRegisterWrite {
        index: 6,
        value: 0x8000_0004,
    }];
    let effects = ZiskMainReportEffects {
        register_writes: writes.to_vec().into(),
        memory_accesses: &[],
        precompile_memory_accesses: &[],
        precompile_result: None,
    };
    let report =
        GuestMachineReport {
            address_and_instruction_len:
                crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0000, 4),
            instruction: RiscvInstruction::Jalr {
                rd: 6,
                rs1: 7,
                offset: -8,
            },
            next_pc: 0x8000_00f8,
            register_write_value: GuestRegisterWriteValue::new(0x8000_0004),
            memory_accesses: vec![].into(),
        };
    let (instruction, parts) = jump_fast_path_parts(3, &report)
        .expect("jump matcher should not fail")
        .expect("jump should match");
    let mut state = ZiskMainTraceState::new();
    state.registers[6] = 0x77;
    state.registers[7] = 0x8000_0100;
    state.register_mem_steps[6] = 44;
    state.register_mem_steps[7] = 33;
    let mut context = ZiskMainReportValidationContext::new(
        None,
        16,
        ZiskMainTraceSegmentInfo {
            trace_instance_index: 0,
            is_last_segment: false,
            previous_c: 0,
        },
    )
    .expect("context should initialize");
    let mut visited = None;
    apply_jump_fast_path(
        3,
        instruction,
        effects,
        report.next_pc,
        parts,
        &mut state,
        &mut context,
        &mut |row, values, timing| {
            assert_eq!(row, 3);
            assert!(timing.is_none());
            visited = Some(values);
            Ok(())
        },
    )
    .expect("linked jump row should take fast path");

    assert_eq!(state.registers[6], 0x8000_0004);
    assert_eq!(state.last_c, 0x8000_0100);
    assert_eq!(state.next_pc, 0x8000_00f8);
    assert_eq!(state.register_mem_steps[7], 14);
    assert_eq!(state.register_mem_steps[6], 15);
    let values = visited.expect("fast path should emit row values");
    assert_eq!(values.a, !1);
    assert_eq!(values.b, 0x8000_0100);
    assert_eq!(values.c, 0x8000_0100);
    assert!(!values.flag);
    assert_eq!(values.register_accesses.a_prev_mem_step, None);
    assert_eq!(values.register_accesses.b_prev_mem_step, Some(33));
    assert_eq!(values.register_accesses.store_prev_mem_step, Some(44));
    assert_eq!(values.register_accesses.store_prev_value, Some(0x77));
}

#[test]
fn x0_jump_fast_path_preserves_no_store_effects() {
    let effects = ZiskMainReportEffects {
        register_writes: Vec::new().into(),
        memory_accesses: &[],
        precompile_memory_accesses: &[],
        precompile_result: None,
    };
    let report =
        GuestMachineReport {
            address_and_instruction_len:
                crate::guest_machine::pack_report_address_and_instruction_len(0x8000_0000, 4),
            instruction: RiscvInstruction::Jal { rd: 0, offset: 12 },
            next_pc: 0x8000_000c,
            register_write_value: GuestRegisterWriteValue::default(),
            memory_accesses: vec![].into(),
        };
    let (instruction, parts) = jump_fast_path_parts(3, &report)
        .expect("jump matcher should not fail")
        .expect("jump should match");
    let mut state = ZiskMainTraceState::new();
    let mut context = ZiskMainReportValidationContext::new(
        None,
        16,
        ZiskMainTraceSegmentInfo {
            trace_instance_index: 0,
            is_last_segment: false,
            previous_c: 0,
        },
    )
    .expect("context should initialize");
    let mut visited = None;
    apply_jump_fast_path(
        3,
        instruction,
        effects,
        report.next_pc,
        parts,
        &mut state,
        &mut context,
        &mut |row, values, timing| {
            assert_eq!(row, 3);
            assert!(timing.is_none());
            visited = Some(values);
            Ok(())
        },
    )
    .expect("x0 jump row should take fast path");

    assert_eq!(state.last_c, 0);
    assert_eq!(state.next_pc, 0x8000_000c);
    let values = visited.expect("fast path should emit row values");
    assert_eq!(values.a, 0);
    assert_eq!(values.b, 0);
    assert_eq!(values.c, 0);
    assert!(values.flag);
    assert_eq!(values.register_accesses.a_prev_mem_step, None);
    assert_eq!(values.register_accesses.b_prev_mem_step, None);
    assert_eq!(values.register_accesses.store_prev_mem_step, None);
    assert_eq!(values.register_accesses.store_prev_value, None);
}

#[test]
fn no_memory_copy_fast_path_preserves_row_effects() {
    let writes = [GuestRegisterWrite {
        index: 3,
        value: 0xaa55,
    }];
    let effects = ZiskMainReportEffects {
        register_writes: writes.to_vec().into(),
        memory_accesses: &[],
        precompile_memory_accesses: &[],
        precompile_result: None,
    };
    let instruction = ZiskMainInstruction {
        pc: 0x8000_0000,
        a: ZiskMainSource::Immediate(0),
        b: ZiskMainSource::Register(2),
        op: ZiskMainOp::CopyB,
        store: ZiskMainStore::Register(3),
        store_pc: false,
        set_pc: false,
        jmp_offset1: 4,
        jmp_offset2: 4,
        ind_width: 0,
        m32: false,
        is_external_op: false,
        is_precompiled: false,
    };
    let parts =
        no_memory_copy_fast_path_parts(&instruction, effects).expect("no-memory copy should match");
    assert_eq!(
        parts,
        ZiskMainNoMemoryFastPathParts {
            a_index: None,
            b_index: Some(2),
            store_index: Some(3),
        }
    );

    let mut state = ZiskMainTraceState::new();
    state.registers[2] = 0xaa55;
    state.registers[3] = 0x77;
    state.register_mem_steps[2] = 33;
    state.register_mem_steps[3] = 44;
    let mut context = ZiskMainReportValidationContext::new(
        None,
        16,
        ZiskMainTraceSegmentInfo {
            trace_instance_index: 0,
            is_last_segment: false,
            previous_c: 0,
        },
    )
    .expect("context should initialize");
    let mut visited = None;
    apply_no_memory_fast_path(
        3,
        instruction,
        effects,
        0x8000_0004,
        parts,
        &mut state,
        &mut context,
        &mut |row, values, timing| {
            assert_eq!(row, 3);
            assert!(timing.is_none());
            visited = Some(values);
            Ok(())
        },
    )
    .expect("no-memory copy row should take fast path");

    assert_eq!(state.registers[3], 0xaa55);
    assert_eq!(state.last_c, 0xaa55);
    assert_eq!(state.next_pc, 0x8000_0004);
    assert_eq!(state.register_mem_steps[2], 14);
    assert_eq!(state.register_mem_steps[3], 15);
    let values = visited.expect("fast path should emit row values");
    assert_eq!(values.a, 0);
    assert_eq!(values.b, 0xaa55);
    assert_eq!(values.c, 0xaa55);
    assert!(!values.flag);
    assert_eq!(values.register_accesses.a_prev_mem_step, None);
    assert_eq!(values.register_accesses.b_prev_mem_step, Some(33));
    assert_eq!(values.register_accesses.store_prev_mem_step, Some(44));
    assert_eq!(values.register_accesses.store_prev_value, Some(0x77));
}

#[test]
fn no_memory_external_register_store_fast_path_preserves_row_effects() {
    let writes = [GuestRegisterWrite {
        index: 3,
        value: 0x1ff,
    }];
    let effects = ZiskMainReportEffects {
        register_writes: writes.to_vec().into(),
        memory_accesses: &[],
        precompile_memory_accesses: &[],
        precompile_result: None,
    };
    let instruction = ZiskMainInstruction {
        pc: 0x8000_0000,
        a: ZiskMainSource::Register(2),
        b: ZiskMainSource::Immediate(0xff),
        op: ZiskMainOp::Or,
        store: ZiskMainStore::Register(3),
        store_pc: false,
        set_pc: false,
        jmp_offset1: 4,
        jmp_offset2: 4,
        ind_width: 0,
        m32: false,
        is_external_op: true,
        is_precompiled: false,
    };
    let mut state = ZiskMainTraceState::new();
    state.registers[2] = 0x100;
    state.registers[3] = 0x77;
    state.register_mem_steps[2] = 33;
    state.register_mem_steps[3] = 44;
    let mut context = ZiskMainReportValidationContext::new(
        None,
        16,
        ZiskMainTraceSegmentInfo {
            trace_instance_index: 0,
            is_last_segment: false,
            previous_c: 0,
        },
    )
    .expect("context should initialize");
    let mut visited = None;
    apply_no_memory_fast_path(
        3,
        instruction,
        effects,
        0x8000_0004,
        ZiskMainNoMemoryFastPathParts {
            a_index: Some(2),
            b_index: None,
            store_index: Some(3),
        },
        &mut state,
        &mut context,
        &mut |row, values, timing| {
            assert_eq!(row, 3);
            assert!(timing.is_none());
            visited = Some(values);
            Ok(())
        },
    )
    .expect("no-memory register-store row should take fast path");

    assert_eq!(state.registers[3], 0x1ff);
    assert_eq!(state.last_c, 0x1ff);
    assert_eq!(state.next_pc, 0x8000_0004);
    assert_eq!(state.register_mem_steps[2], 13);
    assert_eq!(state.register_mem_steps[3], 15);
    let values = visited.expect("fast path should emit row values");
    assert_eq!(values.a, 0x100);
    assert_eq!(values.b, 0xff);
    assert_eq!(values.c, 0x1ff);
    assert!(!values.flag);
    assert_eq!(values.register_accesses.a_prev_mem_step, Some(33));
    assert_eq!(values.register_accesses.b_prev_mem_step, None);
    assert_eq!(values.register_accesses.store_prev_mem_step, Some(44));
    assert_eq!(values.register_accesses.store_prev_value, Some(0x77));
}

#[test]
fn no_memory_external_no_store_fast_path_preserves_source_steps() {
    let effects = ZiskMainReportEffects {
        register_writes: Vec::new().into(),
        memory_accesses: &[],
        precompile_memory_accesses: &[],
        precompile_result: None,
    };
    let instruction = ZiskMainInstruction {
        pc: 0x8000_0000,
        a: ZiskMainSource::Register(2),
        b: ZiskMainSource::Register(3),
        op: ZiskMainOp::Sub,
        store: ZiskMainStore::None,
        store_pc: false,
        set_pc: false,
        jmp_offset1: 4,
        jmp_offset2: 4,
        ind_width: 0,
        m32: false,
        is_external_op: true,
        is_precompiled: false,
    };
    let mut state = ZiskMainTraceState::new();
    state.registers[2] = 9;
    state.registers[3] = 4;
    state.register_mem_steps[2] = 33;
    state.register_mem_steps[3] = 44;
    let mut context = ZiskMainReportValidationContext::new(
        None,
        16,
        ZiskMainTraceSegmentInfo {
            trace_instance_index: 0,
            is_last_segment: false,
            previous_c: 0,
        },
    )
    .expect("context should initialize");
    let mut visited = None;
    apply_no_memory_fast_path(
        3,
        instruction,
        effects,
        0x8000_0004,
        ZiskMainNoMemoryFastPathParts {
            a_index: Some(2),
            b_index: Some(3),
            store_index: None,
        },
        &mut state,
        &mut context,
        &mut |row, values, timing| {
            assert_eq!(row, 3);
            assert!(timing.is_none());
            visited = Some(values);
            Ok(())
        },
    )
    .expect("no-memory no-store row should take fast path");

    assert_eq!(state.registers[2], 9);
    assert_eq!(state.registers[3], 4);
    assert_eq!(state.last_c, 5);
    assert_eq!(state.next_pc, 0x8000_0004);
    assert_eq!(state.register_mem_steps[2], 13);
    assert_eq!(state.register_mem_steps[3], 14);
    let values = visited.expect("fast path should emit row values");
    assert_eq!(values.a, 9);
    assert_eq!(values.b, 4);
    assert_eq!(values.c, 5);
    assert!(!values.flag);
    assert_eq!(values.register_accesses.a_prev_mem_step, Some(33));
    assert_eq!(values.register_accesses.b_prev_mem_step, Some(44));
    assert_eq!(values.register_accesses.store_prev_mem_step, None);
    assert_eq!(values.register_accesses.store_prev_value, None);
}

#[test]
fn copy_indirect_register_store_fast_path_preserves_row_effects() {
    let accesses = [memory_read(0x108, 0xaa55)];
    let writes = [GuestRegisterWrite {
        index: 3,
        value: 0xaa55,
    }];
    let lowered = ZiskMainLoweredReportRow {
        instruction: ZiskMainInstruction {
            pc: 0x8000_0000,
            a: ZiskMainSource::Register(2),
            b: ZiskMainSource::Indirect(8),
            op: ZiskMainOp::CopyB,
            store: ZiskMainStore::Register(3),
            store_pc: false,
            set_pc: false,
            jmp_offset1: 4,
            jmp_offset2: 4,
            ind_width: 8,
            m32: false,
            is_external_op: false,
            is_precompiled: false,
        },
        effects: ZiskMainReportEffects {
            register_writes: writes.to_vec().into(),
            memory_accesses: &accesses,
            precompile_memory_accesses: &[],
            precompile_result: None,
        },
        expected_next_pc: 0x8000_0004,
    };
    let mut state = ZiskMainTraceState::new();
    state.registers[2] = 0x100;
    state.registers[3] = 0x77;
    state.register_mem_steps[2] = 33;
    state.register_mem_steps[3] = 44;
    let mut context = ZiskMainReportValidationContext::new(
        None,
        16,
        ZiskMainTraceSegmentInfo {
            trace_instance_index: 0,
            is_last_segment: false,
            previous_c: 0,
        },
    )
    .expect("context should initialize");
    let mut visited = None;
    apply_copy_indirect_register_store_fast_path(
        3,
        lowered.instruction,
        lowered.effects,
        lowered.expected_next_pc,
        2,
        8,
        3,
        &mut state,
        &mut context,
        &mut |row, values, timing| {
            assert_eq!(row, 3);
            assert!(timing.is_none());
            visited = Some(values);
            Ok(())
        },
    )
    .expect("dominant row shape should take fast path");

    assert_eq!(state.registers[3], 0xaa55);
    assert_eq!(state.last_c, 0xaa55);
    assert_eq!(state.next_pc, 0x8000_0004);
    assert_eq!(state.register_mem_steps[2], 13);
    assert_eq!(state.register_mem_steps[3], 15);
    let values = visited.expect("fast path should emit row values");
    assert_eq!(values.a, 0x100);
    assert_eq!(values.b, 0xaa55);
    assert_eq!(values.c, 0xaa55);
    assert!(!values.flag);
    assert_eq!(values.register_accesses.a_prev_mem_step, Some(33));
    assert_eq!(values.register_accesses.b_prev_mem_step, None);
    assert_eq!(values.register_accesses.store_prev_mem_step, Some(44));
    assert_eq!(values.register_accesses.store_prev_value, Some(0x77));
}

#[test]
fn copy_indirect_register_store_fast_path_rejects_invalid_registers() {
    let accesses = [memory_read(0x108, 0xaa55)];
    let writes = [GuestRegisterWrite {
        index: 3,
        value: 0xaa55,
    }];
    let instruction = ZiskMainInstruction {
        pc: 0x8000_0000,
        a: ZiskMainSource::Register(2),
        b: ZiskMainSource::Indirect(8),
        op: ZiskMainOp::CopyB,
        store: ZiskMainStore::Register(3),
        store_pc: false,
        set_pc: false,
        jmp_offset1: 4,
        jmp_offset2: 4,
        ind_width: 8,
        m32: false,
        is_external_op: false,
        is_precompiled: false,
    };
    let effects = ZiskMainReportEffects {
        register_writes: writes.to_vec().into(),
        memory_accesses: &accesses,
        precompile_memory_accesses: &[],
        precompile_result: None,
    };
    let mut state = ZiskMainTraceState::new();
    state.registers[2] = 0x100;
    let mut context = ZiskMainReportValidationContext::new(
        None,
        16,
        ZiskMainTraceSegmentInfo {
            trace_instance_index: 0,
            is_last_segment: false,
            previous_c: 0,
        },
    )
    .expect("context should initialize");

    let source_error = apply_copy_indirect_register_store_fast_path(
        3,
        instruction.clone(),
        effects,
        0x8000_0004,
        0,
        8,
        3,
        &mut state,
        &mut context,
        &mut |_, _, _| Ok(()),
    )
    .expect_err("source register zero should be rejected");
    assert!(matches!(
        source_error,
        GuestPcTraceBackendError::UnsupportedZiskMainSource { row: 3 }
    ));

    let store_error = apply_copy_indirect_register_store_fast_path(
        3,
        instruction,
        effects,
        0x8000_0004,
        2,
        8,
        32,
        &mut state,
        &mut context,
        &mut |_, _, _| Ok(()),
    )
    .expect_err("destination register outside the valid window should be rejected");
    assert!(matches!(
        store_error,
        GuestPcTraceBackendError::UnsupportedZiskMainStore { row: 3 }
    ));
}

#[test]
fn zisk_main_source_value_reports_memory_access_count() {
    let accesses = [memory_read(64, 96), memory_read(104, 13)];
    let effects = ZiskMainReportEffects {
        register_writes: Vec::new().into(),
        memory_accesses: &accesses,
        precompile_memory_accesses: &[],
        precompile_result: None,
    };
    let mut state = ZiskMainTraceState::new();
    state.registers[1] = 7;
    let report = addi_report();

    let memory = zisk_main_source_value(
        9,
        ZiskMainSource::Memory(64),
        &state,
        &report,
        effects,
        None,
        0,
        0,
    )
    .expect("direct memory source should validate the ordered access");
    let indirect = zisk_main_source_value(
        9,
        ZiskMainSource::Indirect(8),
        &state,
        &report,
        effects,
        Some(96),
        8,
        1,
    )
    .expect("indirect memory source should validate the ordered access");
    let register = zisk_main_source_value(
        9,
        ZiskMainSource::Register(1),
        &state,
        &report,
        effects,
        None,
        0,
        0,
    )
    .expect("register source should not consume a memory access");

    assert_eq!(memory.value, 96);
    assert_eq!(memory.memory_access_count, 1);
    assert_eq!(indirect.value, 13);
    assert_eq!(indirect.memory_access_count, 1);
    assert_eq!(register.value, 7);
    assert_eq!(register.memory_access_count, 0);
}

#[test]
fn zisk_main_source_value_result_stays_narrow() {
    assert!(
        std::mem::size_of::<ZiskMainSourceValueResult>() <= 16,
        "source-value results are returned twice per lowered row and should not carry pointer-sized counters"
    );
}

#[test]
fn source_value_rejects_invalid_register_index() {
    let state = ZiskMainTraceState::new();
    let report = addi_report();
    let result = std::panic::catch_unwind(|| {
        zisk_main_source_value(
            9,
            ZiskMainSource::Register(32),
            &state,
            &report,
            ZiskMainReportEffects::empty(),
            None,
            0,
            0,
        )
    });

    assert!(
        result.is_ok(),
        "invalid register source should return an error instead of panicking"
    );
    let error = result
        .expect("source lookup should not panic")
        .expect_err("invalid register source should fail");
    assert!(error.to_string().contains("unsupported"));
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
        register_writes: Vec::new().into(),
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
        register_writes: Vec::new().into(),
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
        register_writes: Vec::new().into(),
        memory_accesses: &ordered_accesses,
        precompile_memory_accesses: &[],
        precompile_result: None,
    };

    validate_zisk_main_memory_accesses_after_source_values(9, &instruction, effects, 96, 13, 2)
        .expect("store after two validated source accesses should validate");

    let misplaced_store = [memory_read(64, 96), store_access, memory_read(72, 13)];
    let effects = ZiskMainReportEffects {
        register_writes: Vec::new().into(),
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
    let values = apply_zisk_main_register_access_values(
        row,
        &instruction,
        &mut state,
        row_base,
        Some(1),
        Some(1),
    )
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

#[test]
fn zisk_main_report_row_count_uses_lightweight_runner_shape() {
    let addi = GuestMachineReportShape {
        instruction: RiscvInstruction::OpImm {
            kind: RiscvOpImmKind::Addi,
            rd: 1,
            rs1: 0,
            immediate: 7,
        },
        has_memory_write: false,
    };
    assert_eq!(
        zisk_main_report_row_count_from_report_shape(0, addi)
            .expect("ordinary instruction should fit one row"),
        1
    );

    let amo_overlap = GuestMachineReportShape {
        instruction: RiscvInstruction::Amo {
            kind: RiscvAmoKind::Add,
            width: RiscvAmoWidth::Doubleword,
            rd: 1,
            rs1: 1,
            rs2: 2,
            acquire: false,
            release: false,
        },
        has_memory_write: true,
    };
    assert_eq!(
        zisk_main_report_row_count_from_report_shape(0, amo_overlap)
            .expect("overlapping AMO add should use scratch rows"),
        4
    );

    let store_conditional_write = GuestMachineReportShape {
        instruction: RiscvInstruction::StoreConditional {
            width: RiscvAmoWidth::Doubleword,
            rd: 3,
            rs1: 1,
            rs2: 2,
            acquire: false,
            release: false,
        },
        has_memory_write: true,
    };
    assert_eq!(
        zisk_main_report_row_count_from_report_shape(0, store_conditional_write)
            .expect("successful store conditional with rd should use result row"),
        2
    );

    let store_conditional_missing_write = GuestMachineReportShape {
        has_memory_write: false,
        ..store_conditional_write
    };
    assert!(
        zisk_main_report_row_count_from_report_shape(0, store_conditional_missing_write).is_err(),
        "store conditional row shape must preserve the write-success distinction"
    );
}

#[test]
fn main_row_capacity_check_only_near_segment_boundary() {
    assert!(!main_instruction_capacity_needs_exact_check(0, 4));
    assert!(!main_instruction_capacity_needs_exact_check(10, 14));
    assert!(main_instruction_capacity_needs_exact_check(0, 3));
    assert!(main_instruction_capacity_needs_exact_check(11, 14));
    assert!(main_instruction_capacity_needs_exact_check(14, 14));
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
    GuestMachineReport::new(
        0x8000_0000,
        4,
        RiscvInstruction::ZiskPrecompile {
            kind: RiscvPrecompileKind::Add256,
            rs1: 1,
            rd: 2,
        },
        0x8000_0004,
        vec![GuestRegisterWrite { index: 2, value: 1 }].into(),
        Vec::new().into(),
        GuestPrecompileReportEffects::from_parts(precompile_memory_accesses.into(), Some(1)),
    )
}

fn fixed_precompile_report(address: u64, kind: RiscvPrecompileKind) -> GuestMachineReport {
    let operand_address = 0x1000;
    let mut accesses = Vec::new();
    match kind {
        RiscvPrecompileKind::Keccak => {
            for index in 0..25 {
                accesses.push(memory_read(operand_address + index * 8, index));
            }
            for index in 0..25 {
                accesses.push(memory_write(operand_address + index * 8, index));
            }
        }
        RiscvPrecompileKind::Secp256k1Dbl => {
            for index in 0..8 {
                accesses.push(memory_read(operand_address + index * 8, index));
            }
            for index in 0..8 {
                accesses.push(memory_write(operand_address + index * 8, index));
            }
        }
        _ => panic!("test helper only supports direct-address fixed precompiles"),
    }
    GuestMachineReport::new(
        address,
        4,
        RiscvInstruction::ZiskPrecompile {
            kind,
            rs1: 2,
            rd: 0,
        },
        address + 4,
        Vec::new().into(),
        Vec::new().into(),
        GuestPrecompileReportEffects::from_parts(accesses.into(), Some(0)),
    )
}

fn addi_report() -> GuestMachineReport {
    addi_report_at(0x8000_0000, 1, 0, 7, 7)
}

fn addi_report_at(address: u64, rd: u8, rs1: u8, immediate: i16, value: u64) -> GuestMachineReport {
    GuestMachineReport {
        address_and_instruction_len: crate::guest_machine::pack_report_address_and_instruction_len(
            address, 4,
        ),
        instruction: RiscvInstruction::OpImm {
            kind: RiscvOpImmKind::Addi,
            rd,
            rs1,
            immediate: immediate.into(),
        },
        next_pc: address + 4,
        register_write_value: GuestRegisterWriteValue::new(value),
        memory_accesses: Vec::new().into(),
    }
}

#[cfg(feature = "cuda")]
fn add_report_at(address: u64, rd: u8, rs1: u8, rs2: u8, value: u64) -> GuestMachineReport {
    GuestMachineReport {
        address_and_instruction_len: crate::guest_machine::pack_report_address_and_instruction_len(
            address, 4,
        ),
        instruction: RiscvInstruction::Op {
            kind: RiscvOpKind::Add,
            rd,
            rs1,
            rs2,
        },
        next_pc: address + 4,
        register_write_value: GuestRegisterWriteValue::new(value),
        memory_accesses: Vec::new().into(),
    }
}

#[cfg(feature = "cuda")]
fn dma_prepare_report_at(address: u64, kind: RiscvDmaKind, rs1: u8) -> GuestMachineReport {
    GuestMachineReport {
        address_and_instruction_len: crate::guest_machine::pack_report_address_and_instruction_len(
            address, 4,
        ),
        instruction: RiscvInstruction::ZiskDmaPrepare { kind, rs1 },
        next_pc: address + 4,
        register_write_value: GuestRegisterWriteValue::default(),
        memory_accesses: Vec::new().into(),
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

fn write_store_load_replay_guest_image(guest_image: &std::path::Path) -> GuestImageInfo {
    let data_offset = 64_u16;
    let mut words = vec![
        riscv_auipc(1, 0),
        riscv_addi(1, 1, data_offset as i16),
        riscv_addi(2, 0, 123),
        riscv_sd(2, 1, 0),
        riscv_ld(3, 1, 0),
        riscv_addi(4, 3, 1),
        0x0000_0073,
    ];
    while words.len() * std::mem::size_of::<u32>() < usize::from(data_offset) {
        words.push(0);
    }
    words.extend_from_slice(&[0, 0]);
    let guest_image_bytes = sample_guest_image_with_words(&words);
    std::fs::write(guest_image, &guest_image_bytes).expect("guest image should be written");
    parse_guest_image(&guest_image_bytes).expect("guest image should parse")
}

fn store_load_replay_layout() -> WitnessTraceLayout {
    let unit = sample_unit_with_zisk_main_columns_rows(4);
    derive_witness_trace_layout(&unit).expect("layout should derive")
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

fn riscv_ld(rd: u8, rs1: u8, offset: i16) -> u32 {
    (((offset as i32 as u32) & 0x0fff) << 20)
        | (u32::from(rs1) << 15)
        | (3 << 12)
        | (u32::from(rd) << 7)
        | 0x03
}

fn riscv_sd(rs2: u8, rs1: u8, offset: i16) -> u32 {
    let immediate = (offset as i32 as u32) & 0x0fff;
    ((immediate >> 5) << 25)
        | (u32::from(rs2) << 20)
        | (u32::from(rs1) << 15)
        | (3 << 12)
        | ((immediate & 0x1f) << 7)
        | 0x23
}

fn riscv_auipc(rd: u8, upper: u32) -> u32 {
    (upper << 12) | (u32::from(rd) << 7) | 0x17
}

fn riscv_lui(rd: u8, upper: u32) -> u32 {
    (upper << 12) | (u32::from(rd) << 7) | 0x37
}

fn riscv_zisk_fcall_param(port: u8, rs1: u8) -> u32 {
    let csr = 0x08f0_u32 + u32::from(port);
    (csr << 20) | (u32::from(rs1) << 15) | (2 << 12) | 0x73
}

fn riscv_zisk_fcall_invoke(function_id: u16) -> u32 {
    let bank = u32::from(function_id / 32);
    let rs1 = u32::from(function_id % 32);
    ((0x08c0 + bank) << 20) | (rs1 << 15) | (5 << 12) | 0x73
}

fn riscv_zisk_fcall_result(rd: u8) -> u32 {
    (0x0ffe << 20) | (2 << 12) | (u32::from(rd) << 7) | 0x73
}

fn framed_stdin_chunk(payload: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&(payload.len() as u64).to_le_bytes());
    bytes.extend_from_slice(payload);
    bytes.resize(bytes.len().next_multiple_of(std::mem::size_of::<u64>()), 0);
    bytes
}

fn riscv_amo_add_d(rd: u8, rs1: u8, rs2: u8) -> u32 {
    (u32::from(rs2) << 20) | (u32::from(rs1) << 15) | (0b011 << 12) | (u32::from(rd) << 7) | 0x2f
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
