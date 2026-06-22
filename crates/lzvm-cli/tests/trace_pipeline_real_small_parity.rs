#![cfg(feature = "cuda")]

use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use lzvm_artifacts::challenge_values_segment::CHALLENGE_VALUES_SEGMENT_ID;
use lzvm_artifacts::constant_opening_segment::CONSTANT_OPENING_SEGMENT_ID;
use lzvm_artifacts::pcs_evaluation_segment::PCS_EVALUATION_SEGMENT_ID;
use lzvm_artifacts::pcs_fri_segment::PCS_FRI_OPENING_SEGMENT_ID;
use lzvm_artifacts::pcs_material_segment::PCS_MATERIAL_MANIFEST_SEGMENT_ID;
use lzvm_artifacts::pcs_nonce_segment::PCS_QUERY_NONCE_SEGMENT_ID;
use lzvm_artifacts::pcs_proof_values_segment::PCS_PROOF_VALUES_SEGMENT_ID;
use lzvm_artifacts::pcs_query_segment::PCS_QUERY_PLAN_SEGMENT_ID;
use lzvm_artifacts::proof::{parse_proof_artifact, ProofArtifact, ProofSegment};
use lzvm_artifacts::witness_opening_segment::WITNESS_OPENING_SEGMENT_ID;
use lzvm_artifacts::witness_segment::{
    encode_witness_commitment_segment, parse_witness_commitment_segment, WitnessCommitmentSegment,
    WitnessCommitmentStageSegment, WITNESS_COMMITMENT_SEGMENT_BASE_ID,
};

const PARALLEL_LOWER_ENV: &str = "LZVM_GUEST_PC_TRACE_PARALLEL_LOWER";
const LOWER_WORKERS_ENV: &str = "LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_WORKERS";
const LOWER_JOB_QUEUE_ENV: &str = "LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_JOB_QUEUE";
const COMMIT_PIPELINE_ENV: &str = "LZVM_GUEST_PC_TRACE_COMMIT_PIPELINE";
const COMMIT_WORKERS_ENV: &str = "LZVM_GUEST_PC_TRACE_SEGMENT_COMMIT_WORKERS";
const COMMIT_ASYNC_SINGLE_ENV: &str = "LZVM_GUEST_PC_TRACE_SEGMENT_COMMIT_ASYNC_SINGLE";
const SEGMENT_REPLAY_ENV: &str = "LZVM_GUEST_PC_TRACE_SEGMENT_REPLAY";
const SEGMENT_REPLAY_SNAPSHOT_ENV: &str = "LZVM_GUEST_PC_TRACE_SEGMENT_REPLAY_SNAPSHOT";
const RUNNER_SEED_SNAPSHOT_ENV: &str = "LZVM_GUEST_PC_TRACE_RUNNER_SEED_SNAPSHOT";
const RUNNER_SEED_SNAPSHOT_TRUSTED_ENV: &str = "LZVM_GUEST_PC_TRACE_RUNNER_SEED_SNAPSHOT_TRUSTED";
const RUNNER_SEED_SNAPSHOT_VALIDATE_ENV: &str = "LZVM_GUEST_PC_TRACE_RUNNER_SEED_SNAPSHOT_VALIDATE";
const PARALLEL_REPLAY_SNAPSHOT_ENV: &str = "LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_REPLAY_SNAPSHOT";
const PARALLEL_REPLAY_ONLY_ENV: &str = "LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_REPLAY_ONLY";
const WORK_UNITS_ENV: &str = "LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_WORK_UNITS";
const LIVE_REPORT_CHUNKS_ENV: &str = "LZVM_GUEST_PC_TRACE_LIVE_REPORT_CHUNKS";
const LIVE_STREAM_START_ENV: &str = "LZVM_GUEST_PC_TRACE_LIVE_STREAM_START";
const PARALLEL_STREAM_CHUNKS_ENV: &str = "LZVM_GUEST_PC_TRACE_PARALLEL_STREAM_CHUNKS";
const REPORT_CHUNKS_ENV: &str = "LZVM_GUEST_PC_TRACE_REPORT_CHUNKS";
const REPORT_CHUNK_CAPACITY_ENV: &str = "LZVM_GUEST_PC_TRACE_REPORT_CHUNK_CAPACITY";
const OWNED_STREAMING_LOWER_ENV: &str = "LZVM_CUDA_GUEST_PC_OWNED_STREAMING_LOWER";
const REAL_SMALL_PARITY_ENV: RealParityEnv = RealParityEnv {
    bin: "LZVM_REAL_SMALL_PARITY_BIN",
    setup: "LZVM_REAL_SMALL_PARITY_SETUP",
    block_input: "LZVM_REAL_SMALL_PARITY_BLOCK_INPUT",
    program_image_cache: "LZVM_REAL_SMALL_PARITY_PROGRAM_IMAGE_CACHE",
    input_data: "LZVM_REAL_SMALL_PARITY_INPUT_DATA",
    guest_image: "LZVM_REAL_SMALL_PARITY_GUEST_IMAGE",
    trace_limit: "LZVM_REAL_SMALL_PARITY_TRACE_LIMIT",
    work_dir: "LZVM_REAL_SMALL_PARITY_WORK_DIR",
    tmp_dir: "LZVM_REAL_SMALL_PARITY_TMP_DIR",
};
const REAL_LARGE_PARITY_ENV: RealParityEnv = RealParityEnv {
    bin: "LZVM_REAL_LARGE_PARITY_BIN",
    setup: "LZVM_REAL_LARGE_PARITY_SETUP",
    block_input: "LZVM_REAL_LARGE_PARITY_BLOCK_INPUT",
    program_image_cache: "LZVM_REAL_LARGE_PARITY_PROGRAM_IMAGE_CACHE",
    input_data: "LZVM_REAL_LARGE_PARITY_INPUT_DATA",
    guest_image: "LZVM_REAL_LARGE_PARITY_GUEST_IMAGE",
    trace_limit: "LZVM_REAL_LARGE_PARITY_TRACE_LIMIT",
    work_dir: "LZVM_REAL_LARGE_PARITY_WORK_DIR",
    tmp_dir: "LZVM_REAL_LARGE_PARITY_TMP_DIR",
};

#[test]
#[ignore]
fn real_small_trace_pipeline_preserves_proof_bytes() {
    let config = RealParityConfig::from_env(REAL_SMALL_PARITY_ENV, "120000000");
    run_real_trace_pipeline_parity(
        &config,
        "small",
        &[
            RealParityMode::OwnedStreamingLower,
            RealParityMode::LiveReportChunks,
            RealParityMode::LiveTrustedSeedSnapshot,
            RealParityMode::LiveSeedPipeline,
            RealParityMode::LiveStreamPipeline,
            RealParityMode::AsyncSingleCommit,
            RealParityMode::TrustedSeedSnapshot,
            RealParityMode::SeedPipeline,
            RealParityMode::WorkUnitSeedPipeline,
            RealParityMode::ReplayOnlySeedPipeline,
            RealParityMode::SeedPipelineCommitWorkers,
        ],
    );
}

#[test]
#[ignore]
fn trace_pipeline_real_small_default_vs_work_units() {
    let config = RealParityConfig::from_env(REAL_SMALL_PARITY_ENV, "120000000");
    run_real_trace_pipeline_parity(
        &config,
        "small-work-units",
        &[RealParityMode::WorkUnitSeedPipeline],
    );
}

#[test]
#[ignore]
fn real_large_trace_pipeline_preserves_proof_bytes() {
    let config = RealParityConfig::from_env(REAL_LARGE_PARITY_ENV, "600000000");
    run_real_trace_pipeline_parity(
        &config,
        "large",
        &[
            RealParityMode::LiveStreamPipeline,
            RealParityMode::AsyncSingleCommit,
            RealParityMode::TrustedSeedSnapshot,
            RealParityMode::SeedPipeline,
            RealParityMode::ReplayOnlySeedPipeline,
            RealParityMode::SeedPipelineCommitWorkers,
        ],
    );
}

fn run_real_trace_pipeline_parity(
    config: &RealParityConfig,
    label: &str,
    parity_modes: &[RealParityMode],
) {
    let work_dir = config.work_dir.join(format!(
        "lzvm-real-{label}-trace-pipeline-parity-{}",
        std::process::id(),
    ));
    let _ = fs::remove_dir_all(&work_dir);
    fs::create_dir_all(&work_dir).expect("work directory should be created");
    fs::create_dir_all(&config.tmp_dir).expect("tmp directory should be created");

    let default = run_prove_witness(config, &work_dir, "default", RealParityMode::Default);
    for mode in parity_modes {
        let candidate = run_prove_witness(config, &work_dir, mode.label(), *mode);
        assert_same_proof_artifact_surfaces(
            &format!("{} proof.bin", mode.label()),
            &default.output_dir.join("proof.bin"),
            &candidate.output_dir.join("proof.bin"),
        );
        assert_same_file(
            &format!("{} proof.bin", mode.label()),
            &default.output_dir.join("proof.bin"),
            &candidate.output_dir.join("proof.bin"),
        );
        assert_same_file(
            &format!("{} eth-block-public-values.bin", mode.label()),
            &default.output_dir.join("eth-block-public-values.bin"),
            &candidate.output_dir.join("eth-block-public-values.bin"),
        );
    }

    fs::remove_dir_all(&work_dir).expect("work directory should be removed");
}

#[test]
fn proof_artifact_surfaces_cover_stage_roots_and_transcript_segments() {
    let proof = ProofArtifact {
        setup_hash: [1; 32],
        public_values_hash: [2; 32],
        segments: vec![
            ProofSegment {
                id: WITNESS_COMMITMENT_SEGMENT_BASE_ID,
                data: encode_witness_commitment_segment(&WitnessCommitmentSegment {
                    unit_index: 0,
                    input_byte_count: 11,
                    trace_rows: 16,
                    trace_columns: 39,
                    stages: vec![WitnessCommitmentStageSegment {
                        stage_index: 3,
                        arity: 16,
                        root: [4, 5, 6, 7],
                        tree_byte_count: 128,
                        tree_digest: [8; 32],
                    }],
                })
                .expect("witness commitment segment should encode"),
            },
            ProofSegment {
                id: PCS_QUERY_PLAN_SEGMENT_ID,
                data: b"query-plan".to_vec(),
            },
            ProofSegment {
                id: PCS_FRI_OPENING_SEGMENT_ID,
                data: b"fri-opening".to_vec(),
            },
        ],
    };

    let surfaces = proof_artifact_surfaces(&proof);

    assert_eq!(
        surfaces.stage_roots,
        vec![(WITNESS_COMMITMENT_SEGMENT_BASE_ID, 0, 3, [4, 5, 6, 7])]
    );
    assert_eq!(
        surfaces.transcript_segments,
        vec![
            (PCS_QUERY_PLAN_SEGMENT_ID, b"query-plan".to_vec()),
            (PCS_FRI_OPENING_SEGMENT_ID, b"fri-opening".to_vec()),
        ]
    );
}

struct RealParityConfig {
    bin: PathBuf,
    setup_dir: PathBuf,
    block_input: PathBuf,
    program_image_cache: PathBuf,
    input_data: PathBuf,
    guest_image: PathBuf,
    trace_limit: String,
    work_dir: PathBuf,
    tmp_dir: PathBuf,
}

#[derive(Clone, Copy)]
struct RealParityEnv {
    bin: &'static str,
    setup: &'static str,
    block_input: &'static str,
    program_image_cache: &'static str,
    input_data: &'static str,
    guest_image: &'static str,
    trace_limit: &'static str,
    work_dir: &'static str,
    tmp_dir: &'static str,
}

struct ProveRun {
    output_dir: PathBuf,
}

#[derive(Clone, Copy)]
enum RealParityMode {
    Default,
    OwnedStreamingLower,
    LiveReportChunks,
    LiveTrustedSeedSnapshot,
    LiveSeedPipeline,
    LiveStreamPipeline,
    AsyncSingleCommit,
    TrustedSeedSnapshot,
    SeedPipeline,
    WorkUnitSeedPipeline,
    ReplayOnlySeedPipeline,
    SeedPipelineCommitWorkers,
}

#[derive(Debug, PartialEq, Eq)]
struct ProofArtifactSurfaces {
    stage_roots: Vec<(u32, u32, u32, [u64; 4])>,
    transcript_segments: Vec<(u32, Vec<u8>)>,
}

impl RealParityMode {
    fn label(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::OwnedStreamingLower => "owned-streaming-lower",
            Self::LiveReportChunks => "live-report-chunks",
            Self::LiveTrustedSeedSnapshot => "live-report-chunks-trusted-seed",
            Self::LiveSeedPipeline => "live-report-chunks-pipeline",
            Self::LiveStreamPipeline => "live-report-chunks-stream-pipeline",
            Self::AsyncSingleCommit => "async-single-commit",
            Self::TrustedSeedSnapshot => "trusted-seed-snapshot",
            Self::SeedPipeline => "pipeline",
            Self::WorkUnitSeedPipeline => "work-units",
            Self::ReplayOnlySeedPipeline => "replay-only-pipeline",
            Self::SeedPipelineCommitWorkers => "combined",
        }
    }
}

impl RealParityConfig {
    fn from_env(env_names: RealParityEnv, default_trace_limit: &str) -> Self {
        let workspace = workspace_root();
        let temp_dir = workspace.join("temp");
        Self {
            bin: optional_env_path(env_names.bin)
                .unwrap_or_else(|| workspace.join("target").join("release").join("lzvm")),
            setup_dir: required_env_path(env_names.setup),
            block_input: required_env_path(env_names.block_input),
            program_image_cache: required_env_path(env_names.program_image_cache),
            input_data: required_env_path(env_names.input_data),
            guest_image: required_env_path(env_names.guest_image),
            trace_limit: env::var(env_names.trace_limit)
                .unwrap_or_else(|_| default_trace_limit.to_owned()),
            work_dir: optional_env_path(env_names.work_dir).unwrap_or_else(|| temp_dir.clone()),
            tmp_dir: optional_env_path(env_names.tmp_dir).unwrap_or_else(|| temp_dir.join("tmp")),
        }
    }
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root should be derivable")
        .to_path_buf()
}

fn optional_env_path(name: &str) -> Option<PathBuf> {
    env::var_os(name).map(PathBuf::from)
}

fn required_env_path(name: &str) -> PathBuf {
    let path = optional_env_path(name).unwrap_or_else(|| panic!("{name} must be set"));
    assert!(
        path.exists(),
        "{name} path should exist: {}",
        path.display()
    );
    path
}

#[test]
fn trusted_seed_snapshot_mode_sets_seed_env_without_parallel_lower() {
    let workspace = workspace_root();
    let config = RealParityConfig {
        bin: workspace.join("target").join("release").join("lzvm"),
        setup_dir: workspace.join("temp").join("setup"),
        block_input: workspace.join("temp").join("eth-block.input"),
        program_image_cache: workspace.join("temp").join("program-image.cache"),
        input_data: workspace.join("temp").join("input-data.bin"),
        guest_image: workspace.join("temp").join("guest.elf"),
        trace_limit: "1".to_owned(),
        work_dir: workspace.join("temp"),
        tmp_dir: workspace.join("temp").join("tmp"),
    };
    let command = prove_command(
        &config,
        &workspace.join("temp").join("trusted-seed-snapshot.proof"),
        RealParityMode::TrustedSeedSnapshot,
    );

    assert_command_env_equals(&command, RUNNER_SEED_SNAPSHOT_ENV, "1");
    assert_command_env_equals(&command, RUNNER_SEED_SNAPSHOT_TRUSTED_ENV, "1");
    assert_env_removed(&command, PARALLEL_LOWER_ENV);
    assert_env_removed(&command, LOWER_WORKERS_ENV);
    assert_env_removed(&command, PARALLEL_REPLAY_ONLY_ENV);
}

#[test]
fn trusted_seed_snapshot_timing_shape_requires_seed_lift_without_parallel_lower() {
    let output = [
        "timing_guest_trace_parallel_lower_workers=0",
        "timing_guest_trace_seed_direct_lift_successes=22",
        "timing_guest_trace_seed_full_advances=1",
    ]
    .join("\n");

    assert_trusted_seed_snapshot_timing_shape(&output);
}

#[test]
fn live_report_chunks_mode_sets_live_chunk_env_without_seed_or_parallel_lower() {
    let workspace = workspace_root();
    let config = RealParityConfig {
        bin: workspace.join("target").join("release").join("lzvm"),
        setup_dir: workspace.join("temp").join("setup"),
        block_input: workspace.join("temp").join("eth-block.input"),
        program_image_cache: workspace.join("temp").join("program-image.cache"),
        input_data: workspace.join("temp").join("input-data.bin"),
        guest_image: workspace.join("temp").join("guest.elf"),
        trace_limit: "1".to_owned(),
        work_dir: workspace.join("temp"),
        tmp_dir: workspace.join("temp").join("tmp"),
    };
    let command = prove_command(
        &config,
        &workspace.join("temp").join("live-report-chunks.proof"),
        RealParityMode::LiveReportChunks,
    );

    assert_command_env_equals(&command, LIVE_REPORT_CHUNKS_ENV, "1");
    assert_env_removed(&command, REPORT_CHUNK_CAPACITY_ENV);
    assert_env_removed(&command, PARALLEL_LOWER_ENV);
    assert_env_removed(&command, LOWER_WORKERS_ENV);
    assert_env_removed(&command, RUNNER_SEED_SNAPSHOT_ENV);
    assert_env_removed(&command, RUNNER_SEED_SNAPSHOT_TRUSTED_ENV);
}

#[test]
fn live_report_chunks_timing_shape_requires_chunk_counts_without_parallel_lower() {
    let output = [
        "timing_guest_trace_parallel_lower_workers=0",
        "timing_guest_trace_seed_direct_lift_successes=0",
        "timing_guest_trace_report_chunk_sent=4",
        "timing_guest_trace_report_chunk_received=4",
        "timing_guest_trace_report_chunk_reports=16",
    ]
    .join("\n");

    assert_live_report_chunks_timing_shape(&output);
}

#[test]
fn live_trusted_seed_snapshot_timing_shape_requires_chunks_and_seed_lift() {
    let output = [
        "timing_guest_trace_parallel_lower_workers=0",
        "timing_guest_trace_seed_direct_lift_successes=22",
        "timing_guest_trace_report_chunk_sent=4",
        "timing_guest_trace_report_chunk_received=4",
        "timing_guest_trace_report_chunk_reports=16",
    ]
    .join("\n");

    assert_live_trusted_seed_snapshot_timing_shape(&output);
}

#[test]
fn live_seed_pipeline_timing_shape_requires_chunks_and_parallel_lower() {
    let output = [
        "timing_guest_trace_parallel_lower_workers=2",
        "timing_guest_trace_seed_direct_lift_successes=22",
        "timing_guest_trace_seed_full_advances=1",
        "timing_guest_trace_parallel_lower_snapshot_replay_count=0",
        "timing_guest_trace_parallel_lower_report_elided_count=0",
        "timing_guest_trace_report_chunk_sent=4",
        "timing_guest_trace_report_chunk_received=4",
        "timing_guest_trace_report_chunk_reports=16",
    ]
    .join("\n");

    assert_live_seed_pipeline_timing_shape(&output);
}

#[test]
fn live_stream_pipeline_timing_shape_requires_worker_stream_counts() {
    let output = [
        "timing_guest_trace_parallel_lower_workers=2",
        "timing_guest_trace_seed_direct_lift_successes=22",
        "timing_guest_trace_seed_full_advances=1",
        "timing_guest_trace_parallel_lower_snapshot_replay_count=1",
        "timing_guest_trace_parallel_lower_report_elided_count=0",
        "timing_guest_trace_stream_start_sent=4",
        "timing_guest_trace_report_chunk_sent=4",
        "timing_guest_trace_report_chunk_received=4",
        "timing_guest_trace_report_chunk_reports=16",
        "timing_guest_trace_parallel_lower_stream_segments=3",
        "timing_guest_trace_parallel_lower_stream_chunks=4",
        "timing_guest_trace_parallel_lower_stream_retained_reports=0",
    ]
    .join("\n");

    assert_live_stream_pipeline_timing_shape(&output);
}

#[test]
fn live_trusted_seed_snapshot_mode_sets_live_chunk_and_seed_env_without_parallel_lower() {
    let workspace = workspace_root();
    let config = RealParityConfig {
        bin: workspace.join("target").join("release").join("lzvm"),
        setup_dir: workspace.join("temp").join("setup"),
        block_input: workspace.join("temp").join("eth-block.input"),
        program_image_cache: workspace.join("temp").join("program-image.cache"),
        input_data: workspace.join("temp").join("input-data.bin"),
        guest_image: workspace.join("temp").join("guest.elf"),
        trace_limit: "1".to_owned(),
        work_dir: workspace.join("temp"),
        tmp_dir: workspace.join("temp").join("tmp"),
    };
    let command = prove_command(
        &config,
        &workspace.join("temp").join("live-trusted-seed.proof"),
        RealParityMode::LiveTrustedSeedSnapshot,
    );

    assert_command_env_equals(&command, LIVE_REPORT_CHUNKS_ENV, "1");
    assert_env_removed(&command, REPORT_CHUNK_CAPACITY_ENV);
    assert_command_env_equals(&command, RUNNER_SEED_SNAPSHOT_ENV, "1");
    assert_command_env_equals(&command, RUNNER_SEED_SNAPSHOT_TRUSTED_ENV, "1");
    assert_env_removed(&command, PARALLEL_LOWER_ENV);
    assert_env_removed(&command, LOWER_WORKERS_ENV);
    assert_env_removed(&command, PARALLEL_REPLAY_ONLY_ENV);
}

#[test]
fn live_seed_pipeline_mode_sets_live_chunk_and_parallel_lower_env() {
    let workspace = workspace_root();
    let config = RealParityConfig {
        bin: workspace.join("target").join("release").join("lzvm"),
        setup_dir: workspace.join("temp").join("setup"),
        block_input: workspace.join("temp").join("eth-block.input"),
        program_image_cache: workspace.join("temp").join("program-image.cache"),
        input_data: workspace.join("temp").join("input-data.bin"),
        guest_image: workspace.join("temp").join("guest.elf"),
        trace_limit: "1".to_owned(),
        work_dir: workspace.join("temp"),
        tmp_dir: workspace.join("temp").join("tmp"),
    };
    let command = prove_command(
        &config,
        &workspace.join("temp").join("live-seed-pipeline.proof"),
        RealParityMode::LiveSeedPipeline,
    );

    assert_command_env_equals(&command, LIVE_REPORT_CHUNKS_ENV, "1");
    assert_env_removed(&command, REPORT_CHUNK_CAPACITY_ENV);
    assert_command_env_equals(&command, PARALLEL_LOWER_ENV, "1");
    assert_command_env_equals(&command, LOWER_WORKERS_ENV, "2");
    assert_env_removed(&command, PARALLEL_REPLAY_ONLY_ENV);
    assert_env_removed(&command, RUNNER_SEED_SNAPSHOT_ENV);
    assert_env_removed(&command, RUNNER_SEED_SNAPSHOT_TRUSTED_ENV);
}

#[test]
fn live_stream_pipeline_mode_sets_stream_chunk_and_parallel_lower_env() {
    let workspace = workspace_root();
    let config = RealParityConfig {
        bin: workspace.join("target").join("release").join("lzvm"),
        setup_dir: workspace.join("temp").join("setup"),
        block_input: workspace.join("temp").join("eth-block.input"),
        program_image_cache: workspace.join("temp").join("program-image.cache"),
        input_data: workspace.join("temp").join("input-data.bin"),
        guest_image: workspace.join("temp").join("guest.elf"),
        trace_limit: "1".to_owned(),
        work_dir: workspace.join("temp"),
        tmp_dir: workspace.join("temp").join("tmp"),
    };
    let command = prove_command(
        &config,
        &workspace.join("temp").join("live-stream-pipeline.proof"),
        RealParityMode::LiveStreamPipeline,
    );

    assert_command_env_equals(&command, LIVE_REPORT_CHUNKS_ENV, "1");
    assert_command_env_equals(&command, LIVE_STREAM_START_ENV, "1");
    assert_command_env_equals(&command, PARALLEL_STREAM_CHUNKS_ENV, "1");
    assert_env_removed(&command, REPORT_CHUNK_CAPACITY_ENV);
    assert_command_env_equals(&command, PARALLEL_LOWER_ENV, "1");
    assert_command_env_equals(&command, LOWER_WORKERS_ENV, "2");
    assert_env_removed(&command, PARALLEL_REPLAY_ONLY_ENV);
    assert_env_removed(&command, RUNNER_SEED_SNAPSHOT_ENV);
    assert_env_removed(&command, RUNNER_SEED_SNAPSHOT_TRUSTED_ENV);
}

fn run_prove_witness(
    config: &RealParityConfig,
    work_dir: &Path,
    label: &str,
    mode: RealParityMode,
) -> ProveRun {
    assert!(
        config.bin.exists(),
        "binary path should exist: {}",
        config.bin.display()
    );
    let output_dir = work_dir.join(format!("{label}.proof"));
    let output = prove_command(config, &output_dir, mode)
        .output()
        .unwrap_or_else(|error| panic!("{label} prove command should run: {error}"));
    fs::write(work_dir.join(format!("{label}.stdout")), &output.stdout)
        .expect("stdout should be written");
    fs::write(work_dir.join(format!("{label}.stderr")), &output.stderr)
        .expect("stderr should be written");
    let output_text = assert_successful_proof(label, &output);
    match mode {
        RealParityMode::Default => {
            assert_timing_equals(&output_text, "timing_guest_trace_parallel_lower_workers", 0);
            assert_timing_equals(
                &output_text,
                "timing_guest_segment_commit_effective_workers",
                1,
            );
        }
        RealParityMode::OwnedStreamingLower => {
            assert_timing_equals(&output_text, "timing_guest_trace_parallel_lower_workers", 0);
            assert_timing_equals(
                &output_text,
                "timing_guest_segment_commit_effective_workers",
                1,
            );
        }
        RealParityMode::LiveReportChunks => {
            assert_live_report_chunks_timing_shape(&output_text);
            assert_timing_equals(
                &output_text,
                "timing_guest_segment_commit_effective_workers",
                1,
            );
        }
        RealParityMode::LiveTrustedSeedSnapshot => {
            assert_live_trusted_seed_snapshot_timing_shape(&output_text);
            assert_timing_equals(
                &output_text,
                "timing_guest_segment_commit_effective_workers",
                1,
            );
        }
        RealParityMode::LiveSeedPipeline => {
            assert_live_seed_pipeline_timing_shape(&output_text);
            assert_timing_equals(
                &output_text,
                "timing_guest_segment_commit_effective_workers",
                1,
            );
        }
        RealParityMode::LiveStreamPipeline => {
            assert_live_stream_pipeline_timing_shape(&output_text);
            assert_timing_equals(
                &output_text,
                "timing_guest_segment_commit_effective_workers",
                1,
            );
        }
        RealParityMode::AsyncSingleCommit => {
            assert_timing_equals(&output_text, "timing_guest_trace_parallel_lower_workers", 0);
            assert_timing_equals(
                &output_text,
                "timing_guest_segment_commit_effective_workers",
                1,
            );
            assert_timing_at_least(
                &output_text,
                "timing_guest_segment_commit_worker_max_in_flight",
                1,
            );
        }
        RealParityMode::TrustedSeedSnapshot => {
            assert_trusted_seed_snapshot_timing_shape(&output_text);
            assert_timing_equals(
                &output_text,
                "timing_guest_segment_commit_effective_workers",
                1,
            );
        }
        RealParityMode::SeedPipeline => {
            assert_pipeline_timing_shape(&output_text);
            assert_timing_equals(
                &output_text,
                "timing_guest_segment_commit_effective_workers",
                1,
            );
        }
        RealParityMode::WorkUnitSeedPipeline => {
            assert_pipeline_timing_shape(&output_text);
            assert_timing_equals(
                &output_text,
                "timing_guest_segment_commit_effective_workers",
                1,
            );
        }
        RealParityMode::ReplayOnlySeedPipeline => {
            assert_replay_only_pipeline_timing_shape(&output_text);
            assert_timing_equals(
                &output_text,
                "timing_guest_segment_commit_effective_workers",
                1,
            );
        }
        RealParityMode::SeedPipelineCommitWorkers => {
            assert_pipeline_timing_shape(&output_text);
            assert_combined_pipeline_timing_shape(&output_text);
        }
    }
    ProveRun { output_dir }
}

fn prove_command(config: &RealParityConfig, output_dir: &Path, mode: RealParityMode) -> Command {
    let mut command = Command::new(&config.bin);
    command
        .arg("prove")
        .arg("witness")
        .arg("--guest-pc-trace")
        .arg(&config.trace_limit)
        .arg("--timings")
        .arg("--eth-block-input")
        .arg(&config.block_input)
        .arg("--program-image-cache")
        .arg(&config.program_image_cache)
        .arg("--input-data")
        .arg(&config.input_data)
        .arg(&config.setup_dir)
        .arg(output_dir)
        .arg(&config.guest_image)
        .env("TMPDIR", &config.tmp_dir);

    clear_pipeline_env(&mut command);
    match mode {
        RealParityMode::Default => {}
        RealParityMode::OwnedStreamingLower => {
            command.env(OWNED_STREAMING_LOWER_ENV, "1");
        }
        RealParityMode::LiveReportChunks => {
            command.env(LIVE_REPORT_CHUNKS_ENV, "1");
        }
        RealParityMode::LiveTrustedSeedSnapshot => {
            command
                .env(LIVE_REPORT_CHUNKS_ENV, "1")
                .env(RUNNER_SEED_SNAPSHOT_ENV, "1")
                .env(RUNNER_SEED_SNAPSHOT_TRUSTED_ENV, "1");
        }
        RealParityMode::LiveSeedPipeline => {
            command
                .env(LIVE_REPORT_CHUNKS_ENV, "1")
                .env(PARALLEL_LOWER_ENV, "1")
                .env(LOWER_WORKERS_ENV, "2");
        }
        RealParityMode::LiveStreamPipeline => {
            command
                .env(LIVE_REPORT_CHUNKS_ENV, "1")
                .env(LIVE_STREAM_START_ENV, "1")
                .env(PARALLEL_STREAM_CHUNKS_ENV, "1")
                .env(PARALLEL_LOWER_ENV, "1")
                .env(LOWER_WORKERS_ENV, "2");
        }
        RealParityMode::AsyncSingleCommit => {
            command
                .env(COMMIT_WORKERS_ENV, "1")
                .env(COMMIT_ASYNC_SINGLE_ENV, "1");
        }
        RealParityMode::TrustedSeedSnapshot => {
            command
                .env(RUNNER_SEED_SNAPSHOT_ENV, "1")
                .env(RUNNER_SEED_SNAPSHOT_TRUSTED_ENV, "1");
        }
        RealParityMode::SeedPipeline => {
            command
                .env(PARALLEL_LOWER_ENV, "1")
                .env(LOWER_WORKERS_ENV, "2")
                .env(COMMIT_WORKERS_ENV, "1");
        }
        RealParityMode::WorkUnitSeedPipeline => {
            command
                .env(PARALLEL_LOWER_ENV, "1")
                .env(LOWER_WORKERS_ENV, "2")
                .env(COMMIT_WORKERS_ENV, "1")
                .env(WORK_UNITS_ENV, "1");
        }
        RealParityMode::ReplayOnlySeedPipeline => {
            command
                .env(PARALLEL_LOWER_ENV, "1")
                .env(LOWER_WORKERS_ENV, "2")
                .env(COMMIT_WORKERS_ENV, "1")
                .env(PARALLEL_REPLAY_ONLY_ENV, "1");
        }
        RealParityMode::SeedPipelineCommitWorkers => {
            command
                .env(PARALLEL_LOWER_ENV, "1")
                .env(LOWER_WORKERS_ENV, "2")
                .env(COMMIT_PIPELINE_ENV, "1");
        }
    }
    command
}

fn clear_pipeline_env(command: &mut Command) {
    for name in [
        PARALLEL_LOWER_ENV,
        COMMIT_PIPELINE_ENV,
        LOWER_WORKERS_ENV,
        LOWER_JOB_QUEUE_ENV,
        COMMIT_WORKERS_ENV,
        COMMIT_ASYNC_SINGLE_ENV,
        SEGMENT_REPLAY_ENV,
        SEGMENT_REPLAY_SNAPSHOT_ENV,
        RUNNER_SEED_SNAPSHOT_ENV,
        RUNNER_SEED_SNAPSHOT_TRUSTED_ENV,
        RUNNER_SEED_SNAPSHOT_VALIDATE_ENV,
        "LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_REPLAY",
        PARALLEL_REPLAY_ONLY_ENV,
        PARALLEL_REPLAY_SNAPSHOT_ENV,
        WORK_UNITS_ENV,
        "LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_WORKER_REPLAY",
        LIVE_REPORT_CHUNKS_ENV,
        LIVE_STREAM_START_ENV,
        PARALLEL_STREAM_CHUNKS_ENV,
        REPORT_CHUNKS_ENV,
        REPORT_CHUNK_CAPACITY_ENV,
        OWNED_STREAMING_LOWER_ENV,
    ] {
        command.env_remove(OsStr::new(name));
    }
}

#[test]
fn clear_pipeline_env_removes_current_replay_controls() {
    let mut command = Command::new("lzvm");
    for name in [
        PARALLEL_LOWER_ENV,
        LOWER_WORKERS_ENV,
        LOWER_JOB_QUEUE_ENV,
        COMMIT_WORKERS_ENV,
        COMMIT_PIPELINE_ENV,
        COMMIT_ASYNC_SINGLE_ENV,
        SEGMENT_REPLAY_ENV,
        SEGMENT_REPLAY_SNAPSHOT_ENV,
        RUNNER_SEED_SNAPSHOT_ENV,
        RUNNER_SEED_SNAPSHOT_TRUSTED_ENV,
        RUNNER_SEED_SNAPSHOT_VALIDATE_ENV,
        PARALLEL_REPLAY_SNAPSHOT_ENV,
        PARALLEL_REPLAY_ONLY_ENV,
        WORK_UNITS_ENV,
        LIVE_REPORT_CHUNKS_ENV,
        LIVE_STREAM_START_ENV,
        PARALLEL_STREAM_CHUNKS_ENV,
        REPORT_CHUNKS_ENV,
        REPORT_CHUNK_CAPACITY_ENV,
        OWNED_STREAMING_LOWER_ENV,
    ] {
        command.env(name, "1");
    }

    clear_pipeline_env(&mut command);

    for name in [
        PARALLEL_LOWER_ENV,
        LOWER_WORKERS_ENV,
        LOWER_JOB_QUEUE_ENV,
        COMMIT_WORKERS_ENV,
        COMMIT_ASYNC_SINGLE_ENV,
        SEGMENT_REPLAY_ENV,
        SEGMENT_REPLAY_SNAPSHOT_ENV,
        RUNNER_SEED_SNAPSHOT_ENV,
        RUNNER_SEED_SNAPSHOT_TRUSTED_ENV,
        RUNNER_SEED_SNAPSHOT_VALIDATE_ENV,
        PARALLEL_REPLAY_SNAPSHOT_ENV,
        PARALLEL_REPLAY_ONLY_ENV,
        WORK_UNITS_ENV,
        LIVE_REPORT_CHUNKS_ENV,
        LIVE_STREAM_START_ENV,
        PARALLEL_STREAM_CHUNKS_ENV,
        REPORT_CHUNKS_ENV,
        REPORT_CHUNK_CAPACITY_ENV,
        OWNED_STREAMING_LOWER_ENV,
    ] {
        assert_env_removed(&command, name);
    }
}

#[test]
fn pipeline_timing_shape_requires_seed_ready_non_replay_lowering() {
    let output = [
        "timing_guest_trace_parallel_lower_workers=2",
        "timing_guest_trace_seed_direct_lift_successes=22",
        "timing_guest_trace_seed_full_advances=1",
        "timing_guest_trace_parallel_lower_snapshot_replay_count=0",
        "timing_guest_trace_parallel_lower_report_elided_count=0",
    ]
    .join("\n");

    assert_pipeline_timing_shape(&output);
}

#[test]
fn combined_pipeline_timing_shape_requires_cross_segment_commit_workers() {
    let output = [
        "timing_guest_trace_parallel_lower_workers=2",
        "timing_guest_trace_seed_direct_lift_successes=22",
        "timing_guest_trace_seed_full_advances=1",
        "timing_guest_trace_parallel_lower_snapshot_replay_count=0",
        "timing_guest_trace_parallel_lower_report_elided_count=0",
        "timing_guest_segment_commit_initial_workers=2",
        "timing_guest_segment_commit_effective_workers=2",
        "timing_guest_segment_commit_oom_retries=0",
        "timing_guest_segment_commit_worker_max_in_flight=2",
    ]
    .join("\n");

    assert_pipeline_timing_shape(&output);
    assert_combined_pipeline_timing_shape(&output);
}

#[test]
fn work_unit_pipeline_timing_shape_requires_non_replay_lowering() {
    let output = [
        "timing_guest_trace_parallel_lower_workers=2",
        "timing_guest_trace_seed_direct_lift_successes=22",
        "timing_guest_trace_seed_full_advances=1",
        "timing_guest_trace_parallel_lower_snapshot_replay_count=0",
        "timing_guest_trace_parallel_lower_report_elided_count=0",
    ]
    .join("\n");

    assert_pipeline_timing_shape(&output);
}

#[test]
fn replay_only_pipeline_timing_shape_requires_report_elision() {
    let output = [
        "timing_guest_trace_parallel_lower_workers=2",
        "timing_guest_trace_seed_direct_lift_successes=22",
        "timing_guest_trace_seed_full_advances=1",
        "timing_guest_trace_parallel_lower_snapshot_replay_count=23",
        "timing_guest_trace_parallel_lower_report_elided_count=23",
    ]
    .join("\n");

    assert_replay_only_pipeline_timing_shape(&output);
}

#[test]
fn combined_pipeline_timing_shape_allows_oom_retry_fallback() {
    let output = [
        "timing_guest_trace_parallel_lower_workers=2",
        "timing_guest_trace_seed_direct_lift_successes=119",
        "timing_guest_trace_seed_full_advances=1",
        "timing_guest_trace_parallel_lower_snapshot_replay_count=0",
        "timing_guest_trace_parallel_lower_report_elided_count=0",
        "timing_guest_segment_commit_initial_workers=2",
        "timing_guest_segment_commit_effective_workers=1",
        "timing_guest_segment_commit_oom_retries=1",
        "timing_guest_segment_commit_worker_max_in_flight=1",
    ]
    .join("\n");

    assert_pipeline_timing_shape(&output);
    assert_combined_pipeline_timing_shape(&output);
}

#[test]
fn combined_pipeline_mode_sets_commit_pipeline_env() {
    let workspace = workspace_root();
    let config = RealParityConfig {
        bin: workspace.join("target").join("release").join("lzvm"),
        setup_dir: workspace.join("temp").join("setup"),
        block_input: workspace.join("temp").join("eth-block.input"),
        program_image_cache: workspace.join("temp").join("program-image.cache"),
        input_data: workspace.join("temp").join("input-data.bin"),
        guest_image: workspace.join("temp").join("guest.elf"),
        trace_limit: "1".to_owned(),
        work_dir: workspace.join("temp"),
        tmp_dir: workspace.join("temp").join("tmp"),
    };

    let command = prove_command(
        &config,
        &workspace.join("temp").join("combined.proof"),
        RealParityMode::SeedPipelineCommitWorkers,
    );

    assert_command_env_equals(&command, PARALLEL_LOWER_ENV, "1");
    assert_command_env_equals(&command, LOWER_WORKERS_ENV, "2");
    assert_command_env_equals(&command, COMMIT_PIPELINE_ENV, "1");
    assert_env_removed(&command, COMMIT_WORKERS_ENV);
}

#[test]
fn replay_only_pipeline_mode_sets_report_elision_env() {
    let workspace = workspace_root();
    let config = RealParityConfig {
        bin: workspace.join("target").join("release").join("lzvm"),
        setup_dir: workspace.join("temp").join("setup"),
        block_input: workspace.join("temp").join("eth-block.input"),
        program_image_cache: workspace.join("temp").join("program-image.cache"),
        input_data: workspace.join("temp").join("input-data.bin"),
        guest_image: workspace.join("temp").join("guest.elf"),
        trace_limit: "1".to_owned(),
        work_dir: workspace.join("temp"),
        tmp_dir: workspace.join("temp").join("tmp"),
    };

    let command = prove_command(
        &config,
        &workspace.join("temp").join("replay-only.proof"),
        RealParityMode::ReplayOnlySeedPipeline,
    );

    assert_command_env_equals(&command, PARALLEL_LOWER_ENV, "1");
    assert_command_env_equals(&command, LOWER_WORKERS_ENV, "2");
    assert_command_env_equals(&command, COMMIT_WORKERS_ENV, "1");
    assert_command_env_equals(&command, PARALLEL_REPLAY_ONLY_ENV, "1");
}

#[test]
fn work_unit_pipeline_mode_sets_dedicated_env() {
    let workspace = workspace_root();
    let config = RealParityConfig {
        bin: workspace.join("target").join("release").join("lzvm"),
        setup_dir: workspace.join("temp").join("setup"),
        block_input: workspace.join("temp").join("eth-block.input"),
        program_image_cache: workspace.join("temp").join("program-image.cache"),
        input_data: workspace.join("temp").join("input-data.bin"),
        guest_image: workspace.join("temp").join("guest.elf"),
        trace_limit: "1".to_owned(),
        work_dir: workspace.join("temp"),
        tmp_dir: workspace.join("temp").join("tmp"),
    };

    let command = prove_command(
        &config,
        &workspace.join("temp").join("work-units.proof"),
        RealParityMode::WorkUnitSeedPipeline,
    );

    assert_command_env_equals(&command, PARALLEL_LOWER_ENV, "1");
    assert_command_env_equals(&command, LOWER_WORKERS_ENV, "2");
    assert_command_env_equals(&command, COMMIT_WORKERS_ENV, "1");
    assert_command_env_equals(&command, WORK_UNITS_ENV, "1");
    assert_env_removed(&command, PARALLEL_REPLAY_ONLY_ENV);
    assert_env_removed(&command, PARALLEL_REPLAY_SNAPSHOT_ENV);
}

fn assert_env_removed(command: &Command, name: &str) {
    let state = command
        .get_envs()
        .find(|(key, _)| *key == OsStr::new(name))
        .map(|(_, value)| value);
    assert!(
        matches!(state, Some(None)),
        "{name} should be explicitly removed, got {state:?}"
    );
}

fn assert_command_env_equals(command: &Command, name: &str, expected: &str) {
    let state = command
        .get_envs()
        .find(|(key, _)| *key == OsStr::new(name))
        .map(|(_, value)| value);
    assert_eq!(
        state,
        Some(Some(OsStr::new(expected))),
        "{name} should be set to {expected}, got {state:?}"
    );
}

fn assert_successful_proof(label: &str, output: &Output) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "{label} prove command failed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    let combined = format!("{stdout}{stderr}");
    assert!(
        combined.contains("status=ok"),
        "{label} prove command did not report status=ok\n{combined}"
    );
    assert!(
        combined.contains("verify_outputs=true"),
        "{label} prove command did not report verify_outputs=true\n{combined}"
    );
    combined
}

fn assert_timing_equals(output: &str, key: &str, expected: u64) {
    let actual = timing_value(output, key);
    assert_eq!(actual, expected, "{key} should equal {expected}");
}

fn assert_timing_positive(output: &str, key: &str) {
    let actual = timing_value(output, key);
    assert!(actual > 0, "{key} should be positive, got {actual}");
}

fn assert_timing_at_least(output: &str, key: &str, minimum: u64) {
    let actual = timing_value(output, key);
    assert!(
        actual >= minimum,
        "{key} should be at least {minimum}, got {actual}"
    );
}

fn assert_pipeline_timing_shape(output: &str) {
    assert_timing_equals(output, "timing_guest_trace_parallel_lower_workers", 2);
    assert_timing_positive(output, "timing_guest_trace_seed_direct_lift_successes");
    assert_timing_equals(output, "timing_guest_trace_seed_full_advances", 1);
    assert_timing_equals(
        output,
        "timing_guest_trace_parallel_lower_snapshot_replay_count",
        0,
    );
    assert_timing_equals(
        output,
        "timing_guest_trace_parallel_lower_report_elided_count",
        0,
    );
}

fn assert_trusted_seed_snapshot_timing_shape(output: &str) {
    assert_timing_equals(output, "timing_guest_trace_parallel_lower_workers", 0);
    assert_timing_positive(output, "timing_guest_trace_seed_direct_lift_successes");
    assert_timing_equals(output, "timing_guest_trace_seed_full_advances", 1);
}

fn assert_live_report_chunks_timing_shape(output: &str) {
    assert_timing_equals(output, "timing_guest_trace_parallel_lower_workers", 0);
    assert_timing_equals(output, "timing_guest_trace_seed_direct_lift_successes", 0);
    assert_timing_positive(output, "timing_guest_trace_report_chunk_sent");
    assert_timing_positive(output, "timing_guest_trace_report_chunk_received");
    assert_timing_positive(output, "timing_guest_trace_report_chunk_reports");
    assert_eq!(
        timing_value(output, "timing_guest_trace_report_chunk_sent"),
        timing_value(output, "timing_guest_trace_report_chunk_received"),
        "live report chunk sent and received counts should match"
    );
}

fn assert_live_trusted_seed_snapshot_timing_shape(output: &str) {
    assert_timing_equals(output, "timing_guest_trace_parallel_lower_workers", 0);
    assert_timing_positive(output, "timing_guest_trace_seed_direct_lift_successes");
    assert_timing_positive(output, "timing_guest_trace_report_chunk_sent");
    assert_timing_positive(output, "timing_guest_trace_report_chunk_received");
    assert_timing_positive(output, "timing_guest_trace_report_chunk_reports");
    assert_eq!(
        timing_value(output, "timing_guest_trace_report_chunk_sent"),
        timing_value(output, "timing_guest_trace_report_chunk_received"),
        "live report chunk sent and received counts should match"
    );
}

fn assert_live_seed_pipeline_timing_shape(output: &str) {
    assert_pipeline_timing_shape(output);
    assert_timing_positive(output, "timing_guest_trace_report_chunk_sent");
    assert_timing_positive(output, "timing_guest_trace_report_chunk_received");
    assert_timing_positive(output, "timing_guest_trace_report_chunk_reports");
    assert_eq!(
        timing_value(output, "timing_guest_trace_report_chunk_sent"),
        timing_value(output, "timing_guest_trace_report_chunk_received"),
        "live report chunk sent and received counts should match"
    );
}

fn assert_live_stream_pipeline_timing_shape(output: &str) {
    assert_timing_equals(output, "timing_guest_trace_parallel_lower_workers", 2);
    assert_timing_positive(output, "timing_guest_trace_seed_direct_lift_successes");
    assert_timing_equals(output, "timing_guest_trace_seed_full_advances", 1);
    assert_timing_equals(
        output,
        "timing_guest_trace_parallel_lower_snapshot_replay_count",
        1,
    );
    assert_timing_equals(
        output,
        "timing_guest_trace_parallel_lower_report_elided_count",
        0,
    );
    assert_timing_positive(output, "timing_guest_trace_report_chunk_sent");
    assert_timing_positive(output, "timing_guest_trace_report_chunk_received");
    assert_timing_positive(output, "timing_guest_trace_report_chunk_reports");
    assert_timing_positive(output, "timing_guest_trace_stream_start_sent");
    assert_timing_positive(output, "timing_guest_trace_parallel_lower_stream_segments");
    assert_timing_positive(output, "timing_guest_trace_parallel_lower_stream_chunks");
    assert_timing_equals(
        output,
        "timing_guest_trace_parallel_lower_stream_retained_reports",
        0,
    );
    assert_eq!(
        timing_value(output, "timing_guest_trace_parallel_lower_stream_chunks"),
        timing_value(output, "timing_guest_trace_report_chunk_received"),
        "worker stream chunk count should match received live chunks"
    );
}

fn assert_replay_only_pipeline_timing_shape(output: &str) {
    assert_timing_equals(output, "timing_guest_trace_parallel_lower_workers", 2);
    assert_timing_positive(output, "timing_guest_trace_seed_direct_lift_successes");
    assert_timing_equals(output, "timing_guest_trace_seed_full_advances", 1);
    assert_timing_positive(
        output,
        "timing_guest_trace_parallel_lower_snapshot_replay_count",
    );
    assert_timing_positive(
        output,
        "timing_guest_trace_parallel_lower_report_elided_count",
    );
}

fn assert_combined_pipeline_timing_shape(output: &str) {
    assert_timing_equals(output, "timing_guest_segment_commit_initial_workers", 2);
    let oom_retries = timing_value(output, "timing_guest_segment_commit_oom_retries");
    if oom_retries > 0 {
        assert_timing_equals(output, "timing_guest_segment_commit_effective_workers", 1);
        assert_timing_at_least(
            output,
            "timing_guest_segment_commit_worker_max_in_flight",
            1,
        );
    } else {
        assert_timing_equals(output, "timing_guest_segment_commit_effective_workers", 2);
        assert_timing_at_least(
            output,
            "timing_guest_segment_commit_worker_max_in_flight",
            2,
        );
    }
}

fn timing_value(output: &str, key: &str) -> u64 {
    let prefix = format!("{key}=");
    let value = output
        .lines()
        .find_map(|line| line.strip_prefix(&prefix))
        .unwrap_or_else(|| panic!("{key} should be present in timing output"));
    value
        .parse::<u64>()
        .unwrap_or_else(|error| panic!("{key} should parse as u64: {error}"))
}

fn assert_same_file(label: &str, left_path: &Path, right_path: &Path) {
    let left = fs::read(left_path)
        .unwrap_or_else(|error| panic!("{label} should read at {}: {error}", left_path.display()));
    let right = fs::read(right_path)
        .unwrap_or_else(|error| panic!("{label} should read at {}: {error}", right_path.display()));
    if left == right {
        return;
    }
    let first_diff = left
        .iter()
        .zip(&right)
        .position(|(left, right)| left != right);
    panic!(
        "{label} bytes differ: left_len={}, right_len={}, first_diff={first_diff:?}",
        left.len(),
        right.len()
    );
}

fn assert_same_proof_artifact_surfaces(label: &str, left_path: &Path, right_path: &Path) {
    let left = read_proof_artifact_surfaces(label, left_path);
    let right = read_proof_artifact_surfaces(label, right_path);
    assert_eq!(
        left.stage_roots, right.stage_roots,
        "{label} witness stage-root sequence should match"
    );
    assert_eq!(
        left.transcript_segments, right.transcript_segments,
        "{label} transcript-relevant segments should match"
    );
}

fn read_proof_artifact_surfaces(label: &str, path: &Path) -> ProofArtifactSurfaces {
    let bytes = fs::read(path)
        .unwrap_or_else(|error| panic!("{label} should read at {}: {error}", path.display()));
    let proof = parse_proof_artifact(&bytes)
        .unwrap_or_else(|error| panic!("{label} should parse at {}: {error}", path.display()));
    proof_artifact_surfaces(&proof)
}

fn proof_artifact_surfaces(proof: &ProofArtifact) -> ProofArtifactSurfaces {
    ProofArtifactSurfaces {
        stage_roots: witness_stage_root_sequence(proof),
        transcript_segments: transcript_relevant_proof_segments(proof),
    }
}

fn witness_stage_root_sequence(proof: &ProofArtifact) -> Vec<(u32, u32, u32, [u64; 4])> {
    let mut roots = Vec::new();
    for segment in &proof.segments {
        if segment.id < WITNESS_COMMITMENT_SEGMENT_BASE_ID
            || segment.id >= PCS_MATERIAL_MANIFEST_SEGMENT_ID
        {
            continue;
        }
        let witness = parse_witness_commitment_segment(&segment.data)
            .expect("witness commitment segment should parse");
        for stage in witness.stages {
            roots.push((
                segment.id,
                witness.unit_index,
                stage.stage_index,
                stage.root,
            ));
        }
    }
    roots.sort_unstable_by_key(|(segment_id, unit_index, stage_index, root)| {
        (*segment_id, *unit_index, *stage_index, *root)
    });
    roots
}

fn transcript_relevant_proof_segments(proof: &ProofArtifact) -> Vec<(u32, Vec<u8>)> {
    proof
        .segments
        .iter()
        .filter(|segment| {
            matches!(
                segment.id,
                PCS_MATERIAL_MANIFEST_SEGMENT_ID
                    | PCS_QUERY_PLAN_SEGMENT_ID
                    | CONSTANT_OPENING_SEGMENT_ID
                    | WITNESS_OPENING_SEGMENT_ID
                    | PCS_FRI_OPENING_SEGMENT_ID
                    | PCS_QUERY_NONCE_SEGMENT_ID
                    | PCS_EVALUATION_SEGMENT_ID
                    | PCS_PROOF_VALUES_SEGMENT_ID
                    | CHALLENGE_VALUES_SEGMENT_ID
            )
        })
        .map(|segment| (segment.id, segment.data.clone()))
        .collect()
}
