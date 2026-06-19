#![cfg(feature = "cuda")]

use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const PIPELINE_ENV: &str = "LZVM_GUEST_PC_TRACE_COMMIT_PIPELINE";
const LOWER_WORKERS_ENV: &str = "LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_WORKERS";
const COMMIT_WORKERS_ENV: &str = "LZVM_GUEST_PC_TRACE_SEGMENT_COMMIT_WORKERS";

#[test]
#[ignore]
fn real_small_trace_pipeline_preserves_proof_bytes() {
    let config = RealSmallParityConfig::from_env();
    let work_dir = config.work_dir.join(format!(
        "lzvm-real-small-trace-pipeline-parity-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&work_dir);
    fs::create_dir_all(&work_dir).expect("work directory should be created");
    fs::create_dir_all(&config.tmp_dir).expect("tmp directory should be created");

    let default = run_prove_witness(&config, &work_dir, "default", false);
    let pipeline = run_prove_witness(&config, &work_dir, "pipeline", true);

    assert_same_file(
        "proof.bin",
        &default.output_dir.join("proof.bin"),
        &pipeline.output_dir.join("proof.bin"),
    );
    assert_same_file(
        "eth-block-public-values.bin",
        &default.output_dir.join("eth-block-public-values.bin"),
        &pipeline.output_dir.join("eth-block-public-values.bin"),
    );

    fs::remove_dir_all(&work_dir).expect("work directory should be removed");
}

struct RealSmallParityConfig {
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

struct ProveRun {
    output_dir: PathBuf,
}

impl RealSmallParityConfig {
    fn from_env() -> Self {
        let workspace = workspace_root();
        let temp_dir = workspace.join("temp");
        Self {
            bin: optional_env_path("LZVM_REAL_SMALL_PARITY_BIN")
                .unwrap_or_else(|| workspace.join("target").join("release").join("lzvm")),
            setup_dir: required_env_path("LZVM_REAL_SMALL_PARITY_SETUP"),
            block_input: required_env_path("LZVM_REAL_SMALL_PARITY_BLOCK_INPUT"),
            program_image_cache: required_env_path("LZVM_REAL_SMALL_PARITY_PROGRAM_IMAGE_CACHE"),
            input_data: required_env_path("LZVM_REAL_SMALL_PARITY_INPUT_DATA"),
            guest_image: required_env_path("LZVM_REAL_SMALL_PARITY_GUEST_IMAGE"),
            trace_limit: env::var("LZVM_REAL_SMALL_PARITY_TRACE_LIMIT")
                .unwrap_or_else(|_| "120000000".to_owned()),
            work_dir: optional_env_path("LZVM_REAL_SMALL_PARITY_WORK_DIR")
                .unwrap_or_else(|| temp_dir.clone()),
            tmp_dir: optional_env_path("LZVM_REAL_SMALL_PARITY_TMP_DIR")
                .unwrap_or_else(|| temp_dir.join("tmp")),
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

fn run_prove_witness(
    config: &RealSmallParityConfig,
    work_dir: &Path,
    label: &str,
    pipeline_enabled: bool,
) -> ProveRun {
    assert!(
        config.bin.exists(),
        "binary path should exist: {}",
        config.bin.display()
    );
    let output_dir = work_dir.join(format!("{label}.proof"));
    let output = prove_command(config, &output_dir, pipeline_enabled)
        .output()
        .unwrap_or_else(|error| panic!("{label} prove command should run: {error}"));
    fs::write(work_dir.join(format!("{label}.stdout")), &output.stdout)
        .expect("stdout should be written");
    fs::write(work_dir.join(format!("{label}.stderr")), &output.stderr)
        .expect("stderr should be written");
    let output_text = assert_successful_proof(label, &output);
    if pipeline_enabled {
        assert_timing_equals(&output_text, "timing_guest_trace_parallel_lower_workers", 2);
        assert_timing_positive(
            &output_text,
            "timing_guest_trace_seed_direct_lift_successes",
        );
    } else {
        assert_timing_equals(&output_text, "timing_guest_trace_parallel_lower_workers", 0);
    }
    ProveRun { output_dir }
}

fn prove_command(
    config: &RealSmallParityConfig,
    output_dir: &Path,
    pipeline_enabled: bool,
) -> Command {
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
    if pipeline_enabled {
        command
            .env(PIPELINE_ENV, "1")
            .env(LOWER_WORKERS_ENV, "2")
            .env(COMMIT_WORKERS_ENV, "1");
    }
    command
}

fn clear_pipeline_env(command: &mut Command) {
    for name in [
        PIPELINE_ENV,
        LOWER_WORKERS_ENV,
        COMMIT_WORKERS_ENV,
        "LZVM_GUEST_PC_TRACE_PARALLEL_LOWER",
        "LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_REPLAY",
        "LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_REPLAY_ONLY",
        "LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_REPLAY_SNAPSHOT",
        "LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_WORKER_REPLAY",
    ] {
        command.env_remove(OsStr::new(name));
    }
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
