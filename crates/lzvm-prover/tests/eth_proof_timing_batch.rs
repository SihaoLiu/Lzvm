use std::{
    io::{Seek, SeekFrom, Write},
    path::PathBuf,
    process::Command,
};

const SMALL_PREFIX: &str = "LZVM_REAL_SMALL_PARITY";
const LARGE_PREFIX: &str = "LZVM_REAL_LARGE_PARITY";
const REQUIRED_SUFFIXES: &[&str] = &[
    "SETUP",
    "BLOCK_INPUT",
    "PROGRAM_IMAGE_CACHE",
    "INPUT_DATA",
    "GUEST_IMAGE",
];
const VERIFY_REQUIRED_TEXTS: &[&str] = &[
    "verify_proof_status=ok",
    "artifact_public_input_match=ok",
    "artifact_proof_match=ok",
    "eth_block_input_match=ok",
    "program_image_cache_match=ok",
    "framed_guest_input_match=ok",
    "pipeline_input_bindings=ok",
];
const EXPECTED_ARTIFACT_HELP_ITEMS: &[(&str, &str)] = &[
    (
        "setup",
        "Use an existing setup key directory, or run: target/release/lzvm setup generate-key [--backend cpu|cuda] <setup-dir>",
    ),
    (
        "block_input",
        "Write BLOCK_INPUT from block RLP, hex, RPC JSON, and optional receipts: target/release/lzvm eth write-block-input [--hex|--rpc-json] [--receipts <receipts-rlp>|--receipts-rpc-json <receipts-json>] <block> <out>",
    ),
    (
        "public_input_block",
        "Write BLOCK_INPUT from ETH public input: target/release/lzvm eth write-public-block-input [--allow-trailing] [--receipts-rpc-json <receipts-json>] <public-input> <out>",
    ),
    (
        "program_image_cache",
        "Write PROGRAM_IMAGE_CACHE: target/release/lzvm setup write-program-image-cache [--backend cpu|cuda] --setup-dir <setup-dir> <program-bin> <guest-image> <root-bin> <trace-rows> <trace-columns> <blowup-factor> <arity> <out-cache>",
    ),
    (
        "input_data",
        "INPUT_DATA must be framed guest stdin consumed by the guest image.",
    ),
    (
        "guest_image",
        "GUEST_IMAGE must be the guest executable used to produce the matching framed input.",
    ),
];

fn workspace_root() -> &'static std::path::Path {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("workspace root should resolve")
}

fn script_path() -> std::path::PathBuf {
    workspace_root().join("scripts/run-eth-proof-timing-batch.py")
}

fn proof_timing_batch_script_path() -> std::path::PathBuf {
    workspace_root().join("scripts/run-proof-timing-batch.py")
}

fn test_dir(name: &str) -> std::path::PathBuf {
    workspace_root().join(format!("temp/{name}-{}", std::process::id()))
}

#[cfg(unix)]
fn make_executable(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = std::fs::metadata(path)
        .expect("executable fixture metadata should read")
        .permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(path, permissions).expect("executable fixture mode should update");
}

#[cfg(not(unix))]
fn make_executable(_path: &std::path::Path) {}

fn age_file_to_unix_epoch(path: &std::path::Path) {
    let status = Command::new("python3")
        .arg("-c")
        .arg("import os, sys; os.utime(sys.argv[1], (1, 1))")
        .arg(path)
        .status()
        .expect("python should update fixture mtime");
    assert!(status.success(), "fixture mtime update should succeed");
}

fn prepend_path(command: &mut Command, path: &std::path::Path) {
    let old_path = std::env::var_os("PATH").unwrap_or_default();
    let mut paths = std::env::split_paths(&old_path).collect::<Vec<_>>();
    paths.insert(0, path.to_path_buf());
    let joined = std::env::join_paths(paths).expect("fixture PATH should join");
    command.env("PATH", joined);
}

struct ProofFixture {
    dir: PathBuf,
    fake_bin: PathBuf,
    setup: PathBuf,
    block_input: PathBuf,
    cache: PathBuf,
    input_data: PathBuf,
    guest: PathBuf,
    shared_tmp_dir: PathBuf,
}

impl ProofFixture {
    fn new(name: &str) -> Self {
        let dir = test_dir(name);
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("fixture dir should be created");
        let fake_bin = write_fake_lzvm(&dir, "lzvm");
        make_executable(&fake_bin);
        let setup = dir.join("setup");
        std::fs::create_dir_all(&setup).expect("setup dir should be created");
        let block_input = write_fixture(&dir, "block.input");
        let cache = write_fixture(&dir, "program-image.cache");
        let input_data = write_framed_fixture(&dir, "input-data.bin", b"fixture");
        let guest = write_fixture(&dir, "guest.elf");
        let shared_tmp_dir = dir.join("tmp");

        Self {
            dir,
            fake_bin,
            setup,
            block_input,
            cache,
            input_data,
            guest,
            shared_tmp_dir,
        }
    }

    fn apply_env(&self, command: &mut Command, prefix: &str) {
        command
            .env(format!("{prefix}_BIN"), &self.fake_bin)
            .env(format!("{prefix}_SETUP"), &self.setup)
            .env(format!("{prefix}_BLOCK_INPUT"), &self.block_input)
            .env(format!("{prefix}_PROGRAM_IMAGE_CACHE"), &self.cache)
            .env(format!("{prefix}_INPUT_DATA"), &self.input_data)
            .env(format!("{prefix}_GUEST_IMAGE"), &self.guest);
    }

    fn cleanup(&self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn clear_env(command: &mut Command, prefix: &str) {
    for suffix in REQUIRED_SUFFIXES
        .iter()
        .chain(["BIN", "TMP_DIR", "TRACE_LIMIT"].iter())
    {
        command.env_remove(format!("{prefix}_{}", *suffix));
    }
}

fn assert_artifact_help_stderr(stderr: &str) {
    for (name, text) in EXPECTED_ARTIFACT_HELP_ITEMS {
        let expected = format!("artifact_help_{name}={text}");
        assert!(
            stderr.contains(&expected),
            "missing artifact help line {expected:?}: stderr={stderr}"
        );
    }
}

fn assert_artifact_template_help(template: &str) {
    assert!(
        template.contains("# artifact helpers:"),
        "template should introduce artifact helpers: {template}"
    );
    for (_name, text) in EXPECTED_ARTIFACT_HELP_ITEMS {
        let expected = format!("# {text}");
        assert!(
            template.contains(&expected),
            "missing artifact template line {expected:?}: template={template}"
        );
    }
}

fn verify_command_tail(command_text: &str) -> &str {
    command_text
        .find("verify proof --eth-block-input")
        .map(|index| &command_text[index..])
        .unwrap_or("")
}

fn has_required_text_arg(command_text: &str, marker: &str) -> bool {
    command_text.contains(&format!("--require-text {marker}"))
        || command_text.contains(&format!("--require-text\n{marker}"))
}

fn assert_verify_required_text_args(command_text: &str, context: &str) {
    for marker in VERIFY_REQUIRED_TEXTS {
        assert!(
            has_required_text_arg(command_text, marker),
            "{context} should require verify marker {marker}: {command_text}"
        );
    }
}

fn profile_command_dry_run(command_text: &str) -> String {
    let marker = " -- sh -lc ";
    let marker_index = command_text
        .find(marker)
        .expect("profile command should contain the profiled command separator");
    let dry_run_command = format!(
        "{} --dry-run{}",
        &command_text[..marker_index],
        &command_text[marker_index..]
    );
    let output = Command::new("sh")
        .arg("-lc")
        .arg(&dry_run_command)
        .current_dir(workspace_root())
        .output()
        .expect("generated profile command dry-run should execute");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(
        success,
        "generated profile command should parse under dry-run: command={dry_run_command} stderr={stderr}"
    );
    stdout
}

#[test]
fn eth_proof_timing_batch_self_test_runs() {
    let output = Command::new(script_path())
        .arg("--self-test")
        .output()
        .expect("ETH proof timing batch self-test should run");

    assert!(
        output.status.success(),
        "ETH proof timing batch self-test should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(
        stdout.contains("small_runs=3"),
        "self-test should run the small command: {stdout}"
    );
    assert!(
        stdout.contains("small_stable_runs=3"),
        "self-test should report stable small runs: {stdout}"
    );
    assert!(
        stdout.contains("small_stable_avg_s=1.002"),
        "self-test should report the stable small average: {stdout}"
    );
    assert!(
        stdout.contains("small_stable_spread_s=0.002")
            && stdout.contains("small_stable_relative_spread=0.001996"),
        "self-test should report the stable small spread: {stdout}"
    );
    assert!(
        stdout.contains("small_timing_summaries=3"),
        "self-test should summarize small timing logs: {stdout}"
    );
    assert!(
        stdout.contains("large_runs=3"),
        "self-test should run the large command: {stdout}"
    );
    assert!(
        stdout.contains("large_stable_runs=3"),
        "self-test should report stable large runs: {stdout}"
    );
    assert!(
        stdout.contains("large_stable_avg_s=2.002"),
        "self-test should report the stable large average: {stdout}"
    );
    assert!(
        stdout.contains("large_stable_spread_s=0.002")
            && stdout.contains("large_stable_relative_spread=0.000999"),
        "self-test should report the stable large spread: {stdout}"
    );
    assert!(
        stdout.contains("large_timing_summaries=3"),
        "self-test should summarize large timing logs: {stdout}"
    );
}

#[test]
fn proof_timing_batch_reports_excluded_noisy_runs() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-excluded-runs");
    let command_path = fixture.dir.join("emit-timing.py");
    std::fs::write(
        &command_path,
        r#"#!/usr/bin/env python3
import os
run = int(os.environ["LZVM_TIMING_BATCH_RUN"])
totals = {1: 1000, 2: 9000, 3: 1001, 4: 1002}
print(f"timing_total_ms={totals[run]}")
"#,
    )
    .expect("timing fixture command should write");
    make_executable(&command_path);

    let output = Command::new(proof_timing_batch_script_path())
        .arg("--runs")
        .arg("3")
        .arg("--max-runs")
        .arg("4")
        .arg("--small-command")
        .arg(&command_path)
        .arg("--small-timeout")
        .arg("10")
        .arg("--max-relative-spread")
        .arg("0.01")
        .arg("--work-dir")
        .arg(fixture.dir.join("runs"))
        .arg("--path")
        .arg(fixture.dir.join("improve-log.csv"))
        .arg("--summary")
        .arg("excluded run diagnostic")
        .output()
        .expect("proof timing batch should run");

    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(
        success,
        "proof timing batch should accept stable runs after excluding the noisy run: stderr={stderr}"
    );
    assert!(
        stdout.contains("small_runs=4")
            && stdout.contains("small_stable_runs=3")
            && stdout.contains("small_stable_avg_s=1.001")
            && stdout.contains("small_excluded_runs=1")
            && stdout.contains("small_excluded_timing_s=9.000"),
        "stdout should report the excluded noisy run and the stable average: {stdout}"
    );

    let batch_json_line = stdout
        .lines()
        .find(|line| line.starts_with("batch_json="))
        .expect("stdout should report batch_json path");
    let batch_json_path = PathBuf::from(
        batch_json_line
            .strip_prefix("batch_json=")
            .expect("batch_json line should have prefix"),
    );
    let batch_json =
        std::fs::read_to_string(batch_json_path).expect("batch json should be readable");
    assert!(
        batch_json.contains("\"small_excluded_run_count\": 1")
            && batch_json.contains("\"small_excluded_timing_s\": [\n    9.0\n  ]")
            && batch_json
                .contains("\"small_stable_timing_s\": [\n    1.0,\n    1.001,\n    1.002\n  ]"),
        "batch json should preserve stable and excluded timing samples: {batch_json}"
    );

    fixture.cleanup();
}

#[test]
fn eth_proof_timing_batch_dry_run_builds_small_command_from_env() {
    let fixture = ProofFixture::new("eth proof timing batch dry run");
    let mut command = Command::new(script_path());
    command
        .arg("--suite")
        .arg("small")
        .arg("--dry-run")
        .arg("--work-dir")
        .arg(fixture.dir.join("runs"))
        .arg("--path")
        .arg(fixture.dir.join("improve-log.csv"))
        .arg("--summary")
        .arg("dry run")
        .env("LZVM_GUEST_TRACE_RUNNER_DETAIL_TIMING", "1")
        .env("LZVM_GUEST_TRACE_RUNNER_DETAIL_TIMING_SAMPLE_STRIDE", "5")
        .env("LZVM_GUEST_TRACE_DETAIL_TIMING", "1")
        .env("LZVM_GUEST_TRACE_DETAIL_TIMING_SAMPLE_STRIDE", "7")
        .env("LZVM_GUEST_TRACE_SHAPE_TIMING", "1")
        .env("LZVM_GUEST_TRACE_SHAPE_TIMING_SAMPLE_STRIDE", "11")
        .env("CUDA_VISIBLE_DEVICES", "");
    fixture.apply_env(&mut command, SMALL_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch dry-run should run");
    let tmp_dir_created = fixture.shared_tmp_dir.exists();
    let work_dir_created = fixture.dir.join("runs").exists();
    let improve_log_created = fixture.dir.join("improve-log.csv").exists();
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    fixture.cleanup();

    assert!(
        success,
        "dry-run should build a small command: stderr={stderr}"
    );
    assert!(
        !tmp_dir_created,
        "dry-run should not create a shared TMPDIR"
    );
    assert!(!work_dir_created, "dry-run should not create a work dir");
    assert!(
        !improve_log_created,
        "dry-run should not create the improve log"
    );
    assert!(
        stdout.contains("--require-proof-output"),
        "runner command should require proof markers: {stdout}"
    );
    assert!(
        stdout.contains("--max-runs 5")
            && stdout.contains("runs=3\n")
            && stdout.contains("max_runs=5\n"),
        "runner command should reserve replacement attempts by default: {stdout}"
    );
    assert!(
        stdout.contains("--small-max-avg-s 10.0")
            && stdout.contains("small_target_max_avg_s=10.0\n"),
        "runner command should enforce the default small target: {stdout}"
    );
    assert!(
        !stdout.contains("--append-max-average-rejections"),
        "rejected average logging should stay off by default: {stdout}"
    );
    assert!(
        stdout.contains("append_max_average_rejections=false\n"),
        "dry-run metadata should report default rejected average logging: {stdout}"
    );
    assert_verify_required_text_args(&stdout, "runner command");
    assert!(
        stdout.contains("small_command=env -u LZVM_GUEST_PC_TRACE_PARALLEL_LOWER"),
        "small command should clear pipeline environment: {stdout}"
    );
    assert!(
        stdout.contains("{batch_dir}/small-{run_padded}.proof"),
        "small command should use a unique per-run output directory: {stdout}"
    );
    assert!(
        stdout.contains("prove witness --guest-pc-trace 120000000 --timings"),
        "small command should invoke the real proof subcommand: {stdout}"
    );
    assert!(
        stdout.contains("--eth-block-input")
            && stdout.contains("--program-image-cache")
            && stdout.contains("--input-data"),
        "small command should pass proof inputs: {stdout}"
    );
    let verify_command = verify_command_tail(&stdout);
    assert!(
        verify_command.contains("verify proof --eth-block-input")
            && verify_command.contains("--program-image-cache")
            && verify_command.contains("--input-data")
            && verify_command.contains("{batch_dir}/small-{run_padded}.proof/proof.bin")
            && verify_command
                .contains("{batch_dir}/small-{run_padded}.proof/eth-block-public-values.bin")
            && verify_command.contains("verify_proof_status=ok"),
        "small command should run an external proof verification after proving: {stdout}"
    );
    assert!(
        stdout.contains("&& env -u LZVM_GUEST_PC_TRACE_PARALLEL_LOWER"),
        "external proof verification should clear pipeline environment controls: {stdout}"
    );
    assert!(
        stdout.contains("-u LZVM_GUEST_TRACE_DETAIL_TIMING")
            && stdout.contains("-u LZVM_GUEST_TRACE_DETAIL_TIMING_SAMPLE_STRIDE")
            && stdout.contains("-u LZVM_GUEST_TRACE_RUNNER_DETAIL_TIMING")
            && stdout.contains("-u LZVM_GUEST_TRACE_RUNNER_DETAIL_TIMING_SAMPLE_STRIDE")
            && stdout.contains("-u LZVM_GUEST_TRACE_SHAPE_TIMING")
            && stdout.contains("-u LZVM_GUEST_TRACE_SHAPE_TIMING_SAMPLE_STRIDE"),
        "prove and verify commands should clear ambient trace diagnostic controls: {stdout}"
    );
    assert!(
        !stdout.contains("LZVM_GUEST_TRACE_RUNNER_DETAIL_TIMING=1")
            && !stdout.contains("LZVM_GUEST_TRACE_RUNNER_DETAIL_TIMING_SAMPLE_STRIDE=5")
            && !stdout.contains("LZVM_GUEST_TRACE_DETAIL_TIMING=1")
            && !stdout.contains("LZVM_GUEST_TRACE_DETAIL_TIMING_SAMPLE_STRIDE=7")
            && !stdout.contains("LZVM_GUEST_TRACE_SHAPE_TIMING=1")
            && !stdout.contains("LZVM_GUEST_TRACE_SHAPE_TIMING_SAMPLE_STRIDE=11")
            && stdout.contains("trace_shape_timing=false\n")
            && stdout.contains("trace_runner_detail_timing=false\n")
            && stdout.contains("trace_runner_detail_timing_sample_stride=\n")
            && stdout.contains("trace_detail_timing=false\n")
            && stdout.contains("trace_detail_timing_sample_stride=\n"),
        "diagnostic trace timing should stay off unless requested: {stdout}"
    );
    assert!(
        stdout.contains("small_mode=default\n")
            && !stdout.contains("LZVM_GUEST_PC_TRACE_PARALLEL_LOWER=1")
            && !stdout.contains("LZVM_GUEST_PC_TRACE_COMMIT_PIPELINE=1"),
        "default mode should avoid opt-in pipeline environment: {stdout}"
    );
    for assignment in [
        "LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_WORKERS=2",
        "LZVM_GUEST_PC_TRACE_SEGMENT_COMMIT_WORKERS=1",
    ] {
        assert!(
            !stdout.contains(assignment),
            "dry-run command should leave worker sizing to backend defaults: {stdout}"
        );
    }
    assert!(
        stdout.contains("TMPDIR={tmp_dir}"),
        "small command should use the per-run temp dir token: {stdout}"
    );
    assert!(
        !stdout.contains("CUDA_VISIBLE_DEVICES="),
        "dry-run command should not inject a GPU selector by default: {stdout}"
    );
    assert!(
        !stdout.contains(&fixture.shared_tmp_dir.display().to_string()),
        "dry-run command should not use a shared TMPDIR: {stdout}"
    );
    assert!(
        stdout.contains(&format!("'{}'", fixture.fake_bin.display())),
        "binary path containing spaces should be shell-quoted: {stdout}"
    );
}

#[test]
fn eth_proof_timing_batch_dry_run_passes_proof_tuning_flags_to_prove() {
    let fixture = ProofFixture::new("eth proof timing batch dry run proof tuning");
    let mut command = Command::new(script_path());
    command
        .arg("--suite")
        .arg("small")
        .arg("--dry-run")
        .arg("--gpu-preallocate")
        .arg("--minimal-memory")
        .arg("--no-pack-trace")
        .arg("--gpu-streams")
        .arg("7")
        .arg("--witness-thread-pools")
        .arg("5")
        .arg("--stored-witnesses")
        .arg("3")
        .arg("--work-dir")
        .arg(fixture.dir.join("runs"))
        .arg("--path")
        .arg(fixture.dir.join("improve-log.csv"))
        .arg("--summary")
        .arg("dry run")
        .env("CUDA_VISIBLE_DEVICES", "");
    fixture.apply_env(&mut command, SMALL_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch dry-run should run");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    fixture.cleanup();

    assert!(
        success,
        "dry-run should build a small command: stderr={stderr}"
    );
    assert!(
        stdout.contains("gpu_preallocate=true\n"),
        "dry-run summary should report GPU preallocation: {stdout}"
    );
    assert!(
        stdout.contains("minimal_memory=true\n")
            && stdout.contains("pack_trace=false\n")
            && stdout.contains("gpu_streams=7\n")
            && stdout.contains("witness_thread_pools=5\n")
            && stdout.contains("stored_witnesses=3\n"),
        "dry-run summary should report proof tuning flags: {stdout}"
    );
    assert!(
        stdout.contains(
            "prove witness --guest-pc-trace 120000000 --timings --gpu-preallocate \
             --minimal-memory --no-pack-trace --gpu-streams 7 --witness-thread-pools 5 \
             --stored-witnesses 3"
        ),
        "small command should pass proof tuning flags to proving: {stdout}"
    );
    let small_command = stdout
        .lines()
        .find(|line| line.starts_with("small_command="))
        .expect("small command should be printed");
    let verify_command = small_command
        .split(" && ")
        .nth(1)
        .expect("small command should include verification");
    assert!(
        !verify_command.contains("--gpu-preallocate")
            && !verify_command.contains("--minimal-memory")
            && !verify_command.contains("--no-pack-trace")
            && !verify_command.contains("--gpu-streams")
            && !verify_command.contains("--witness-thread-pools")
            && !verify_command.contains("--stored-witnesses"),
        "verify command should not receive proof-only tuning flags: {stdout}"
    );
}

#[test]
fn eth_proof_timing_batch_dry_run_applies_worker_overrides() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-worker-overrides");
    let mut command = Command::new(script_path());
    command
        .arg("--suite")
        .arg("small")
        .arg("--small-mode")
        .arg("combined")
        .arg("--dry-run")
        .arg("--parallel-lower-workers")
        .arg("6")
        .arg("--parallel-lower-job-queue")
        .arg("12")
        .arg("--segment-commit-workers")
        .arg("3")
        .arg("--summary")
        .arg("worker overrides");
    fixture.apply_env(&mut command, SMALL_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch dry-run should run");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    fixture.cleanup();

    assert!(
        success,
        "dry-run should build worker override command: stderr={stderr}"
    );
    for assignment in [
        "LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_WORKERS=6",
        "LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_JOB_QUEUE=12",
        "LZVM_GUEST_PC_TRACE_SEGMENT_COMMIT_WORKERS=3",
    ] {
        assert!(
            stdout.contains(assignment),
            "dry-run command should apply worker override {assignment}: {stdout}"
        );
    }
    assert!(
        stdout.contains("parallel_lower_workers=6\n")
            && stdout.contains("parallel_lower_job_queue=12\n")
            && stdout.contains("segment_commit_workers=3\n")
            && stdout.contains("small_mode=combined\n")
            && stdout.contains("LZVM_GUEST_PC_TRACE_PARALLEL_LOWER=1")
            && stdout.contains("LZVM_GUEST_PC_TRACE_COMMIT_PIPELINE=1"),
        "dry-run metadata should report worker overrides: {stdout}"
    );
}

#[test]
fn eth_proof_timing_batch_dry_run_pins_external_source_opening_batch_size() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-external-source-batch");
    let mut command = Command::new(script_path());
    command
        .arg("--suite")
        .arg("small")
        .arg("--dry-run")
        .arg("--external-source-opening-batch-size")
        .arg("8")
        .arg("--summary")
        .arg("external source batch")
        .env("LZVM_WITNESS_OPENING_EXTERNAL_SOURCE_BATCH_SIZE", "16")
        .env(
            "LZVM_CUDA_TRACE_OUTPUT_EXTERNAL_SOURCE_OPENING_BATCH_SIZE",
            "32",
        );
    fixture.apply_env(&mut command, SMALL_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch dry-run should run");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    fixture.cleanup();

    assert!(
        success,
        "dry-run should build external source batch command: stderr={stderr}"
    );
    assert!(
        stdout.contains("LZVM_WITNESS_OPENING_EXTERNAL_SOURCE_BATCH_SIZE=8"),
        "dry-run command should pin explicit external source batch size: {stdout}"
    );
    assert!(
        stdout.contains("external_source_opening_batch_size=8\n"),
        "dry-run metadata should report explicit external source batch size: {stdout}"
    );
    assert!(
        !stdout.contains("LZVM_WITNESS_OPENING_EXTERNAL_SOURCE_BATCH_SIZE=16")
            && !stdout.contains("LZVM_CUDA_TRACE_OUTPUT_EXTERNAL_SOURCE_OPENING_BATCH_SIZE=32"),
        "ambient external source batch env should not leak into generated commands: {stdout}"
    );
}

#[test]
fn eth_proof_timing_batch_dry_run_clears_ambient_external_source_opening_batch_size() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-clear-external-source-batch");
    let mut command = Command::new(script_path());
    command
        .arg("--suite")
        .arg("small")
        .arg("--dry-run")
        .arg("--summary")
        .arg("clear external source batch")
        .env("LZVM_WITNESS_OPENING_EXTERNAL_SOURCE_BATCH_SIZE", "16")
        .env(
            "LZVM_CUDA_TRACE_OUTPUT_EXTERNAL_SOURCE_OPENING_BATCH_SIZE",
            "32",
        );
    fixture.apply_env(&mut command, SMALL_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch dry-run should run");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    fixture.cleanup();

    assert!(
        success,
        "dry-run should clear ambient external source batch env: stderr={stderr}"
    );
    assert!(
        stdout.contains("-u LZVM_WITNESS_OPENING_EXTERNAL_SOURCE_BATCH_SIZE")
            && stdout.contains("-u LZVM_CUDA_TRACE_OUTPUT_EXTERNAL_SOURCE_OPENING_BATCH_SIZE"),
        "generated commands should clear ambient external source batch env: {stdout}"
    );
    assert!(
        stdout.contains("external_source_opening_batch_size=\n"),
        "dry-run metadata should show no explicit external source batch size: {stdout}"
    );
    assert!(
        !stdout.contains("LZVM_WITNESS_OPENING_EXTERNAL_SOURCE_BATCH_SIZE=16")
            && !stdout.contains("LZVM_CUDA_TRACE_OUTPUT_EXTERNAL_SOURCE_OPENING_BATCH_SIZE=32")
            && !stdout.contains("--external-source-opening-batch-size"),
        "ambient external source batch size should not be copied into commands: {stdout}"
    );
}

#[test]
fn eth_proof_timing_batch_check_env_preserves_external_source_opening_batch_next_command() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-next-external-source-batch");
    let mut command = Command::new(script_path());
    command
        .arg("--suite")
        .arg("small")
        .arg("--check-env")
        .arg("--external-source-opening-batch-size")
        .arg("8");
    fixture.apply_env(&mut command, SMALL_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch check-env should run");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    fixture.cleanup();

    assert!(
        success,
        "check-env should pass with external source batch size: stderr={stderr}"
    );
    let next_run = stdout
        .lines()
        .find(|line| line.starts_with("next_run_command="))
        .expect("check-env should print next run command");
    assert!(
        next_run.contains("--external-source-opening-batch-size 8"),
        "next run command should preserve explicit external source batch size: {stdout}"
    );
}

#[test]
fn eth_proof_timing_batch_dry_run_pins_cross_segment_root_window() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-root-window");
    let mut command = Command::new(script_path());
    command
        .arg("--suite")
        .arg("small")
        .arg("--dry-run")
        .arg("--cross-segment-root-window")
        .arg("16")
        .arg("--summary")
        .arg("root window")
        .env("LZVM_CUDA_GUEST_PC_CROSS_SEGMENT_ROOT_WINDOW", "8");
    fixture.apply_env(&mut command, SMALL_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch dry-run should run");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    fixture.cleanup();

    assert!(
        success,
        "dry-run should build cross-segment root window command: stderr={stderr}"
    );
    assert!(
        stdout.contains("LZVM_CUDA_GUEST_PC_CROSS_SEGMENT_ROOT_WINDOW=16"),
        "dry-run command should pin explicit cross-segment root window: {stdout}"
    );
    assert!(
        stdout.contains("cross_segment_root_window=16\n"),
        "dry-run metadata should report explicit cross-segment root window: {stdout}"
    );
    assert!(
        !stdout.contains("LZVM_CUDA_GUEST_PC_CROSS_SEGMENT_ROOT_WINDOW=8"),
        "ambient cross-segment root window should not leak into generated commands: {stdout}"
    );
}

#[test]
fn eth_proof_timing_batch_dry_run_clears_ambient_cross_segment_root_window() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-clear-root-window");
    let mut command = Command::new(script_path());
    command
        .arg("--suite")
        .arg("small")
        .arg("--dry-run")
        .arg("--summary")
        .arg("clear root window")
        .env("LZVM_CUDA_GUEST_PC_CROSS_SEGMENT_ROOT_WINDOW", "8");
    fixture.apply_env(&mut command, SMALL_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch dry-run should run");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    fixture.cleanup();

    assert!(
        success,
        "dry-run should clear ambient cross-segment root window: stderr={stderr}"
    );
    assert!(
        stdout.contains("-u LZVM_CUDA_GUEST_PC_CROSS_SEGMENT_ROOT_WINDOW"),
        "generated commands should clear ambient cross-segment root window: {stdout}"
    );
    assert!(
        stdout.contains("cross_segment_root_window=\n"),
        "dry-run metadata should show no explicit cross-segment root window: {stdout}"
    );
    assert!(
        !stdout.contains("LZVM_CUDA_GUEST_PC_CROSS_SEGMENT_ROOT_WINDOW=8")
            && !stdout.contains("--cross-segment-root-window"),
        "ambient cross-segment root window should not be copied into commands: {stdout}"
    );
}

#[test]
fn eth_proof_timing_batch_dry_run_clears_ambient_cross_segment_root_disable() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-clear-root-disable");
    let mut command = Command::new(script_path());
    command
        .arg("--suite")
        .arg("small")
        .arg("--dry-run")
        .arg("--summary")
        .arg("clear root disable")
        .env("LZVM_CUDA_GUEST_PC_CROSS_SEGMENT_ROOTS", "0");
    fixture.apply_env(&mut command, SMALL_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch dry-run should run");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    fixture.cleanup();

    assert!(
        success,
        "dry-run should clear ambient cross-segment root disable: stderr={stderr}"
    );
    assert!(
        stdout.contains("-u LZVM_CUDA_GUEST_PC_CROSS_SEGMENT_ROOTS"),
        "generated commands should clear ambient cross-segment root disable: {stdout}"
    );
    assert!(
        !stdout.contains("LZVM_CUDA_GUEST_PC_CROSS_SEGMENT_ROOTS=0"),
        "ambient cross-segment root disable should not be copied into commands: {stdout}"
    );
}

#[test]
fn eth_proof_timing_batch_check_env_preserves_cross_segment_root_window_next_command() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-next-root-window");
    let mut command = Command::new(script_path());
    command
        .arg("--suite")
        .arg("small")
        .arg("--check-env")
        .arg("--cross-segment-root-window")
        .arg("16");
    fixture.apply_env(&mut command, SMALL_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch check-env should run");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    fixture.cleanup();

    assert!(
        success,
        "check-env should pass with cross-segment root window: stderr={stderr}"
    );
    let next_run = stdout
        .lines()
        .find(|line| line.starts_with("next_run_command="))
        .expect("check-env should print next run command");
    assert!(
        next_run.contains("--cross-segment-root-window 16"),
        "next run command should preserve explicit cross-segment root window: {stdout}"
    );
}

#[test]
fn eth_proof_timing_batch_dry_run_applies_worker_overrides_in_default_mode() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-default-worker-overrides");
    let mut command = Command::new(script_path());
    command
        .arg("--suite")
        .arg("small")
        .arg("--small-mode")
        .arg("default")
        .arg("--dry-run")
        .arg("--parallel-lower-workers")
        .arg("6")
        .arg("--parallel-lower-job-queue")
        .arg("12")
        .arg("--summary")
        .arg("default worker overrides");
    fixture.apply_env(&mut command, SMALL_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch dry-run should run");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    fixture.cleanup();

    assert!(
        success,
        "dry-run should build default worker override command: stderr={stderr}"
    );
    assert!(
        stdout.contains("LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_WORKERS=6")
            && stdout.contains("LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_JOB_QUEUE=12")
            && !stdout.contains("LZVM_GUEST_PC_TRACE_PARALLEL_LOWER=1"),
        "default mode should pass sizing overrides without forcing parallel lower: {stdout}"
    );
    assert!(
        stdout.contains("small_mode=default\n")
            && stdout.contains("parallel_lower_workers=6\n")
            && stdout.contains("parallel_lower_job_queue=12\n"),
        "dry-run metadata should report default worker overrides: {stdout}"
    );
}

#[test]
fn eth_proof_timing_batch_dry_run_enables_owned_streaming_lower() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-owned-streaming");
    let mut command = Command::new(script_path());
    command
        .arg("--suite")
        .arg("small")
        .arg("--dry-run")
        .arg("--owned-streaming-lower")
        .arg("--summary")
        .arg("owned streaming");
    fixture.apply_env(&mut command, SMALL_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch dry-run should run");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    fixture.cleanup();

    assert!(
        success,
        "dry-run should build owned streaming lower command: stderr={stderr}"
    );
    assert!(
        stdout.contains("owned_streaming_lower=true\n"),
        "dry-run metadata should report owned streaming lower: {stdout}"
    );
    assert!(
        stdout.contains("LZVM_CUDA_GUEST_PC_OWNED_STREAMING_LOWER=1"),
        "prove command should enable owned streaming lower after clearing inherited controls: {stdout}"
    );
}

#[test]
fn eth_proof_timing_batch_dry_run_can_request_trace_diagnostic_timing() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-trace-diagnostic");
    let mut command = Command::new(script_path());
    command
        .arg("--suite")
        .arg("small")
        .arg("--dry-run")
        .arg("--trace-shape-timing")
        .arg("--trace-detail-timing-sample-stride")
        .arg("4096")
        .arg("--summary")
        .arg("trace diagnostic");
    fixture.apply_env(&mut command, SMALL_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch dry-run should run");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    fixture.cleanup();

    assert!(
        success,
        "dry-run should build trace diagnostic command: stderr={stderr}"
    );
    assert!(
        stdout.contains("LZVM_GUEST_TRACE_SHAPE_TIMING=1")
            && stdout.contains("LZVM_GUEST_TRACE_DETAIL_TIMING=1")
            && stdout.contains("LZVM_GUEST_TRACE_DETAIL_TIMING_SAMPLE_STRIDE=4096"),
        "prove command should enable requested trace diagnostic timing: {stdout}"
    );
    assert!(
        stdout.contains("trace_shape_timing=true\n")
            && stdout.contains("trace_detail_timing=true\n")
            && stdout.contains("trace_detail_timing_sample_stride=4096\n"),
        "dry-run metadata should report effective trace diagnostic timing: {stdout}"
    );
}

#[test]
fn eth_proof_timing_batch_dry_run_can_request_trace_shape_sample_timing() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-trace-shape-sample");
    let mut command = Command::new(script_path());
    command
        .arg("--suite")
        .arg("small")
        .arg("--dry-run")
        .arg("--trace-shape-timing-sample-stride")
        .arg("4096")
        .arg("--summary")
        .arg("trace shape sample");
    fixture.apply_env(&mut command, SMALL_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch dry-run should run");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    fixture.cleanup();

    assert!(
        success,
        "dry-run should build trace shape sample command: stderr={stderr}"
    );
    assert!(
        stdout.contains("LZVM_GUEST_TRACE_SHAPE_TIMING_SAMPLE_STRIDE=4096")
            && !stdout.contains("LZVM_GUEST_TRACE_SHAPE_TIMING=1"),
        "prove command should enable sampled shape timing without full shape timing: {stdout}"
    );
    assert!(
        stdout.contains("trace_shape_timing=false\n")
            && stdout.contains("trace_shape_timing_sample_stride=4096\n"),
        "dry-run metadata should report sampled trace shape timing: {stdout}"
    );
}

#[test]
fn eth_proof_timing_batch_dry_run_can_request_trace_runner_detail_sample_timing() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-trace-runner-detail");
    let mut command = Command::new(script_path());
    command
        .arg("--suite")
        .arg("small")
        .arg("--dry-run")
        .arg("--trace-runner-detail-timing-sample-stride")
        .arg("4096")
        .arg("--summary")
        .arg("trace runner detail");
    fixture.apply_env(&mut command, SMALL_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch dry-run should run");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    fixture.cleanup();

    assert!(
        success,
        "dry-run should build trace runner detail command: stderr={stderr}"
    );
    assert!(
        stdout.contains("LZVM_GUEST_TRACE_RUNNER_DETAIL_TIMING=1")
            && stdout.contains("LZVM_GUEST_TRACE_RUNNER_DETAIL_TIMING_SAMPLE_STRIDE=4096"),
        "prove command should enable sampled runner detail timing: {stdout}"
    );
    assert!(
        stdout.contains("trace_runner_detail_timing=true\n")
            && stdout.contains("trace_runner_detail_timing_sample_stride=4096\n"),
        "dry-run metadata should report sampled runner detail timing: {stdout}"
    );
}

#[test]
fn eth_proof_timing_batch_target_thresholds_follow_selected_suite() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-target-thresholds");
    let mut command = Command::new(script_path());
    command
        .arg("--suite")
        .arg("small")
        .arg("--dry-run")
        .arg("--enforce-targets")
        .arg("--max-runs")
        .arg("5")
        .arg("--summary")
        .arg("target thresholds");
    fixture.apply_env(&mut command, SMALL_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch dry-run should run");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    fixture.cleanup();

    assert!(
        success,
        "dry-run should build target threshold command: stderr={stderr}"
    );
    assert!(
        stdout.contains("--small-max-avg-s 10.0"),
        "small target threshold should be passed to the runner: {stdout}"
    );
    assert!(
        stdout.contains("--max-runs 5"),
        "rerun cap should be passed to the runner: {stdout}"
    );
    assert!(
        stdout.contains("suite=small\n")
            && stdout.contains("selected=small\n")
            && stdout.contains("runs=3\n")
            && stdout.contains("max_runs=5\n")
            && stdout.contains("verify_proof=true\n")
            && stdout.contains("small_mode=default\n")
            && stdout.contains("small_target_max_avg_s=10.0\n"),
        "dry-run metadata should report the effective target configuration: {stdout}"
    );
    assert!(
        !stdout.contains("--large-max-avg-s"),
        "large target threshold should not be passed when only small is selected: {stdout}"
    );
    assert!(
        !stdout.contains("large_target_max_avg_s="),
        "dry-run metadata should only report selected suites: {stdout}"
    );
}

#[test]
fn eth_proof_timing_batch_dry_run_can_request_rejected_average_logging() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-append-rejected");
    let mut command = Command::new(script_path());
    command
        .arg("--suite")
        .arg("small")
        .arg("--dry-run")
        .arg("--append-max-average-rejections")
        .arg("--enforce-targets")
        .arg("--summary")
        .arg("append rejected");
    fixture.apply_env(&mut command, SMALL_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch dry-run should run");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    fixture.cleanup();

    assert!(
        success,
        "dry-run should build rejected average logging command: stderr={stderr}"
    );
    assert!(
        stdout.contains("--append-max-average-rejections"),
        "rejected average logging should be passed to the runner: {stdout}"
    );
    assert!(
        stdout.contains("append_max_average_rejections=true\n"),
        "dry-run metadata should report rejected average logging: {stdout}"
    );
    assert!(
        stdout.contains("--small-max-avg-s 10.0"),
        "target threshold should still be passed with rejected average logging: {stdout}"
    );
}

#[test]
fn eth_proof_timing_batch_dry_run_can_request_seed_discovery_modes() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-seed-discovery");
    let mut command = Command::new(script_path());
    command
        .arg("--suite")
        .arg("large")
        .arg("--dry-run")
        .arg("--seed-discovery")
        .arg("--seed-discovery-streaming-device-lower")
        .arg("--summary")
        .arg("seed discovery");
    fixture.apply_env(&mut command, LARGE_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch dry-run should run");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    fixture.cleanup();

    assert!(
        success,
        "dry-run should build seed discovery command: stderr={stderr}"
    );
    assert!(
        stdout.contains("-u LZVM_GUEST_PC_TRACE_SEED_DISCOVERY")
            && stdout.contains("-u LZVM_GUEST_PC_TRACE_SEED_DISCOVERY_STREAMING_DEVICE_LOWER"),
        "seed discovery knobs should be cleared before explicit command env: {stdout}"
    );
    assert!(
        stdout.contains("LZVM_GUEST_PC_TRACE_SEED_DISCOVERY=1")
            && stdout.contains("LZVM_GUEST_PC_TRACE_SEED_DISCOVERY_STREAMING_DEVICE_LOWER=1"),
        "seed discovery knobs should be explicit command env: {stdout}"
    );
    assert!(
        stdout.contains("seed_discovery=true\n")
            && stdout.contains("seed_discovery_streaming_device_lower=true\n"),
        "dry-run metadata should report seed discovery modes: {stdout}"
    );
}

#[test]
fn eth_proof_timing_batch_skip_targets_omits_default_thresholds() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-skip-targets");
    let mut command = Command::new(script_path());
    command
        .arg("--suite")
        .arg("small")
        .arg("--dry-run")
        .arg("--skip-targets")
        .arg("--summary")
        .arg("skip targets");
    fixture.apply_env(&mut command, SMALL_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch dry-run should run");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    fixture.cleanup();

    assert!(
        success,
        "dry-run should allow skipping default targets: stderr={stderr}"
    );
    assert!(
        !stdout.contains("--small-max-avg-s") && stdout.contains("small_target_max_avg_s=\n"),
        "skip-targets should omit default target thresholds: {stdout}"
    );
}

#[test]
fn eth_proof_timing_batch_run_uses_runner_tmpdir_token() {
    let fixture = ProofFixture::new("eth proof timing batch actual run");
    let runner = fixture.dir.join("fake-runner.py");
    let runner_args_path = fixture.dir.join("runner-args.txt");
    std::fs::write(
        &runner,
        format!(
            "import pathlib\nimport sys\npathlib.Path({:?}).write_text('\\n'.join(sys.argv[1:]) + '\\n', encoding='utf-8')\n",
            runner_args_path.display().to_string()
        ),
    )
    .expect("fake runner should write");
    let mut command = Command::new(script_path());
    command
        .arg("--suite")
        .arg("small")
        .arg("--runner")
        .arg(&runner)
        .arg("--work-dir")
        .arg(fixture.dir.join("runs"))
        .arg("--path")
        .arg(fixture.dir.join("improve-log.csv"))
        .arg("--summary")
        .arg("actual run");
    fixture.apply_env(&mut command, SMALL_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch run should invoke the runner");
    let tmp_dir_created = fixture.shared_tmp_dir.exists();
    let runner_args =
        std::fs::read_to_string(&runner_args_path).expect("fake runner args should read");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    fixture.cleanup();

    assert!(
        success,
        "actual run should invoke the runner: stderr={stderr}"
    );
    assert!(
        !tmp_dir_created,
        "actual run should not create a shared TMPDIR"
    );
    assert!(
        runner_args.contains("TMPDIR={tmp_dir}"),
        "runner should receive the per-run temp token: {runner_args}"
    );
    assert_verify_required_text_args(&runner_args, "runner");
    let verify_command = verify_command_tail(&runner_args);
    assert!(
        verify_command.contains("verify proof --eth-block-input")
            && verify_command.contains("--program-image-cache")
            && verify_command.contains("--input-data")
            && verify_command.contains("verify_proof_status=ok"),
        "runner should receive a prove-then-verify command: {runner_args}"
    );
    assert!(
        runner_args.contains("&& env -u LZVM_GUEST_PC_TRACE_PARALLEL_LOWER"),
        "runner verify command should clear pipeline environment controls: {runner_args}"
    );
    assert!(
        !runner_args.contains(&fixture.shared_tmp_dir.display().to_string()),
        "runner command should not use a shared TMPDIR: {runner_args}"
    );
}

#[test]
fn eth_proof_timing_batch_skip_verify_omits_external_verify() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-skip-verify");
    let mut command = Command::new(script_path());
    command
        .arg("--suite")
        .arg("small")
        .arg("--dry-run")
        .arg("--skip-verify-proof")
        .arg("--summary")
        .arg("skip verify");
    fixture.apply_env(&mut command, SMALL_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch dry-run should run");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    fixture.cleanup();

    assert!(
        success,
        "dry-run should allow skipping verify: stderr={stderr}"
    );
    assert!(
        !stdout.contains("verify proof --eth-block-input")
            && !stdout.contains("verify_proof_status=ok")
            && !stdout.contains("--require-text verify_proof_status=ok")
            && !stdout.contains("artifact_public_input_match=ok")
            && !stdout.contains("artifact_proof_match=ok")
            && !stdout.contains("eth_block_input_match=ok")
            && !stdout.contains("program_image_cache_match=ok")
            && !stdout.contains("framed_guest_input_match=ok")
            && !stdout.contains("pipeline_input_bindings=ok"),
        "skip mode should omit external proof verification: {stdout}"
    );
}

#[test]
fn eth_proof_timing_batch_check_env_reports_ready_paths() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-check-env");
    let mut command = Command::new(script_path());
    command.arg("--suite").arg("small").arg("--check-env");
    fixture.apply_env(&mut command, SMALL_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch env check should run");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    fixture.cleanup();

    assert!(
        success,
        "env check should pass for a complete small config: stderr={stderr}"
    );
    assert!(stdout.contains("status=ok\n"), "{stdout}");
    assert!(stdout.contains("small=ready\n"), "{stdout}");
    assert!(stdout.contains("small_mode=default\n"), "{stdout}");
    assert!(stdout.contains("small_verify_proof=true\n"), "{stdout}");
    assert!(
        stdout.contains("small_verify_required_text=verify_proof_status=ok\n")
            && stdout.contains("small_verify_required_text=artifact_public_input_match=ok\n")
            && stdout.contains("small_verify_required_text=artifact_proof_match=ok\n")
            && stdout.contains("small_verify_required_text=eth_block_input_match=ok\n")
            && stdout.contains("small_verify_required_text=program_image_cache_match=ok\n")
            && stdout.contains("small_verify_required_text=framed_guest_input_match=ok\n")
            && stdout.contains("small_verify_required_text=pipeline_input_bindings=ok\n"),
        "env check should report required proof verification markers: {stdout}"
    );
    assert!(stdout.contains("small_trace_limit=120000000\n"), "{stdout}");
    assert!(stdout.contains("small_block_input="), "{stdout}");
    assert!(!stdout.contains("small_tmp_dir="), "{stdout}");
    let expected_next_base = concat!(
        "scripts/run-eth-proof-timing-batch.py --suite small --small-mode default ",
        "--large-mode default --runs 3 --max-runs 5 --small-timeout 60.0 ",
        "--large-timeout 180.0 --max-relative-spread 0.1 ",
        "--work-dir temp/proof-timing-batch --path temp/improve-log.csv"
    );
    assert!(
        stdout.contains(&format!(
            "next_preflight_command={expected_next_base} --check-env --check-profile-tools\n"
        )),
        "env check should report the next preflight command: {stdout}"
    );
    assert!(
        stdout.contains(&format!(
            "next_profile_command={expected_next_base} --print-profile-commands\n"
        )),
        "env check should report the next profile command: {stdout}"
    );
    assert!(
        stdout.contains(&format!(
            "next_run_command={expected_next_base} --summary 'real proof timing'\n"
        )),
        "env check should report the next timing command: {stdout}"
    );
}

#[test]
fn eth_proof_timing_batch_check_env_preserves_cli_binary_override() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-cli-bin-override");
    let override_bin = write_fake_lzvm(&fixture.dir, "override lzvm");
    make_executable(&override_bin);
    let override_rel = override_bin
        .strip_prefix(workspace_root())
        .expect("override binary path should be under workspace")
        .display()
        .to_string();
    let mut command = Command::new(script_path());
    command
        .arg("--suite")
        .arg("small")
        .arg("--check-env")
        .arg("--small-bin")
        .arg(&override_bin);
    fixture.apply_env(&mut command, SMALL_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch env check should accept CLI binary override");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    fixture.cleanup();

    assert!(
        success,
        "env check should pass with a CLI binary override: stderr={stderr}"
    );
    assert!(
        stdout.contains(&format!("small_bin={}", override_bin.display())),
        "env check should report the CLI-selected binary: {stdout}"
    );
    assert!(
        stdout.contains(&format!("--small-bin '{}'", override_rel)),
        "follow-up commands should preserve the CLI binary override: {stdout}"
    );
    assert!(
        !stdout.contains(&format!("small_bin={}", fixture.fake_bin.display())),
        "env binary should not override the CLI-selected binary: {stdout}"
    );
}

#[test]
fn eth_proof_timing_batch_check_env_preserves_trace_diagnostic_next_commands() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-check-env-trace-diagnostic");
    let mut command = Command::new(script_path());
    command
        .arg("--suite")
        .arg("small")
        .arg("--check-env")
        .arg("--trace-shape-timing")
        .arg("--trace-shape-timing-sample-stride")
        .arg("2048")
        .arg("--trace-runner-detail-timing-sample-stride")
        .arg("1024")
        .arg("--trace-detail-timing-sample-stride")
        .arg("4096");
    fixture.apply_env(&mut command, SMALL_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch env check should run");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let preflight_line = stdout
        .lines()
        .find(|line| line.starts_with("next_preflight_command="))
        .unwrap_or("");
    let profile_line = stdout
        .lines()
        .find(|line| line.starts_with("next_profile_command="))
        .unwrap_or("");
    let run_line = stdout
        .lines()
        .find(|line| line.starts_with("next_run_command="))
        .unwrap_or("");
    fixture.cleanup();

    assert!(
        success,
        "env check should pass for trace diagnostic flags: stderr={stderr}"
    );
    for line in [preflight_line, profile_line, run_line] {
        assert!(
            line.contains("--trace-shape-timing")
                && line.contains("--trace-shape-timing-sample-stride 2048")
                && line.contains("--trace-runner-detail-timing")
                && line.contains("--trace-runner-detail-timing-sample-stride 1024")
                && line.contains("--trace-detail-timing")
                && line.contains("--trace-detail-timing-sample-stride 4096"),
            "next command should preserve trace diagnostic flags: {stdout}"
        );
    }
}

#[test]
fn eth_proof_timing_batch_check_env_skip_verify_omits_required_markers() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-check-env-skip-verify");
    let mut command = Command::new(script_path());
    command
        .arg("--suite")
        .arg("small")
        .arg("--check-env")
        .arg("--skip-verify-proof");
    fixture.apply_env(&mut command, SMALL_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch env check should run");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    fixture.cleanup();

    assert!(
        success,
        "env check should pass for skip verify mode: stderr={stderr}"
    );
    assert!(stdout.contains("small_verify_proof=false\n"), "{stdout}");
    assert!(
        !stdout.contains("small_verify_required_text="),
        "skip verify env check should not report required markers: {stdout}"
    );
}

#[test]
fn eth_proof_timing_batch_check_env_reports_profile_tool_status() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-check-env-profile-tools");
    let profile_dir = fixture.dir.join("profiles");
    let nsys_path = write_fixture(&fixture.dir, "custom-nsys");
    make_executable(&nsys_path);
    let ncu_path = fixture.dir.join("missing-ncu");
    let profile_rel = profile_dir
        .strip_prefix(workspace_root())
        .expect("profile path should be under workspace")
        .display()
        .to_string();
    let nsys_rel = nsys_path
        .strip_prefix(workspace_root())
        .expect("nsys path should be under workspace")
        .display()
        .to_string();
    let mut command = Command::new(script_path());
    command
        .arg("--suite")
        .arg("small")
        .arg("--check-env")
        .arg("--profile-tool")
        .arg("both")
        .arg("--profile-output-dir")
        .arg(&profile_dir)
        .arg("--nsys-command")
        .arg(&nsys_path)
        .arg("--ncu-command")
        .arg(&ncu_path);
    fixture.apply_env(&mut command, SMALL_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch env check should run");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let profile_created = profile_dir.exists();
    fixture.cleanup();

    assert!(
        success,
        "env check should report profiler diagnostics without requiring every tool: stderr={stderr}"
    );
    assert!(
        stdout.contains("status=ok\n") && stdout.contains("small=ready\n"),
        "env check should still report proof readiness: {stdout}"
    );
    assert!(
        stdout.contains(&format!("profile_output_dir={profile_rel}\n"))
            && stdout.contains("profile_tool=both\n"),
        "env check should report the selected profile target: {stdout}"
    );
    assert!(
        stdout.contains("nsys_profiler_source=arg\n")
            && stdout.contains(&format!("nsys_profiler_command={}\n", nsys_path.display()))
            && stdout.contains("nsys_profiler_status=ready\n")
            && stdout.contains(&format!("nsys_profiler_resolved={nsys_rel}\n")),
        "env check should report the ready profiler path: {stdout}"
    );
    assert!(
        stdout.contains("ncu_profiler_source=arg\n")
            && stdout.contains(&format!("ncu_profiler_command={}\n", ncu_path.display()))
            && stdout.contains("ncu_profiler_status=missing\n")
            && !stdout.contains("ncu_profiler_resolved="),
        "env check should report missing optional profiler tools without a false resolved path: {stdout}"
    );
    assert!(
        !profile_created,
        "env check should not create profile output directories"
    );
}

#[test]
fn eth_proof_timing_batch_check_env_resolves_relative_profile_tools_from_workspace() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-check-env-profile-relative");
    let profile_dir = fixture.dir.join("profiles");
    let mut command = Command::new(script_path());
    command
        .arg("--suite")
        .arg("small")
        .arg("--check-env")
        .arg("--profile-output-dir")
        .arg(&profile_dir)
        .arg("--nsys-command")
        .arg("scripts/run-proof-profile.py");
    fixture.apply_env(&mut command, SMALL_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch env check should run");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let profile_created = profile_dir.exists();
    fixture.cleanup();

    assert!(
        success,
        "env check should accept workspace-relative profiler paths used by generated profile commands: stderr={stderr}"
    );
    assert!(
        stdout.contains("status=ok\n") && stdout.contains("small=ready\n"),
        "env check should still validate proof envs: {stdout}"
    );
    assert!(
        stdout.contains("nsys_profiler_source=arg\n")
            && stdout.contains("nsys_profiler_command=scripts/run-proof-profile.py\n")
            && stdout.contains("nsys_profiler_status=ready\n")
            && stdout.contains("nsys_profiler_resolved=scripts/run-proof-profile.py\n"),
        "env check should resolve relative profiler paths from the generated profile cwd: {stdout}"
    );
    assert!(
        !profile_created,
        "env check should not create profile output directories"
    );
}

#[test]
fn eth_proof_timing_batch_check_profile_tools_runs_without_proof_env() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-profile-tools-ready");
    let profile_dir = fixture.dir.join("profiles");
    let tool_path = write_fixture(&fixture.dir, "custom-nsys");
    make_executable(&tool_path);
    let mut command = Command::new(script_path());
    command
        .arg("--check-profile-tools")
        .arg("--profile-output-dir")
        .arg(&profile_dir)
        .arg("--nsys-command")
        .arg(&tool_path);
    clear_env(&mut command, SMALL_PREFIX);
    clear_env(&mut command, LARGE_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch profile tool check should run");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let profile_created = profile_dir.exists();
    fixture.cleanup();

    assert!(
        success,
        "profile tool check should not require proof envs: stderr={stderr}"
    );
    assert!(
        stdout.contains("profile_tool=nsys\n")
            && stdout.contains("nsys_profiler_source=arg\n")
            && stdout.contains(&format!("nsys_profiler_command={}\n", tool_path.display()))
            && stdout.contains("nsys_profiler_status=ready\n"),
        "profile tool check should report the ready profiler: {stdout}"
    );
    assert!(
        !stdout.contains("status=ok\n") && !stdout.contains("small=ready\n"),
        "profile tool check should not validate proof envs: {stdout}"
    );
    assert!(
        !profile_created,
        "profile tool check should not create profile output directories"
    );
}

#[test]
fn eth_proof_timing_batch_combined_check_still_validates_proof_env() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-combined-check-missing-env");
    let profile_dir = fixture.dir.join("profiles");
    let tool_path = write_fixture(&fixture.dir, "custom-nsys");
    make_executable(&tool_path);
    let mut command = Command::new(script_path());
    command
        .arg("--suite")
        .arg("small")
        .arg("--check-env")
        .arg("--check-profile-tools")
        .arg("--profile-output-dir")
        .arg(&profile_dir)
        .arg("--nsys-command")
        .arg(&tool_path);
    clear_env(&mut command, SMALL_PREFIX);
    clear_env(&mut command, LARGE_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch combined env check should run");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let profile_created = profile_dir.exists();
    fixture.cleanup();

    assert!(
        !success,
        "combined env/profile check should fail when proof inputs are missing"
    );
    assert!(
        stdout.contains("profile_tool=nsys\n")
            && stdout.contains("nsys_profiler_status=ready\n")
            && !stdout.contains("status=ok\n"),
        "combined check should report profiler readiness before proof env failure: {stdout}"
    );
    assert!(
        stderr.contains("small proof environment is incomplete"),
        "combined check should not skip proof env validation: stderr={stderr}"
    );
    assert_artifact_help_stderr(&stderr);
    assert!(
        !profile_created,
        "combined check should not create profile output directories"
    );
}

#[test]
fn eth_proof_timing_batch_combined_check_requires_profile_tools() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-combined-check-missing-tool");
    let profile_dir = fixture.dir.join("profiles");
    let mut command = Command::new(script_path());
    command
        .arg("--suite")
        .arg("small")
        .arg("--check-env")
        .arg("--check-profile-tools")
        .arg("--profile-output-dir")
        .arg(&profile_dir)
        .arg("--nsys-command")
        .arg(fixture.dir.join("missing-nsys"));
    fixture.apply_env(&mut command, SMALL_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch combined missing tool check should run");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let profile_created = profile_dir.exists();
    fixture.cleanup();

    assert!(
        !success,
        "combined env/profile check should fail when the requested profiler is missing"
    );
    assert!(
        stdout.contains("profile_tool=nsys\n")
            && stdout.contains("nsys_profiler_status=missing\n")
            && !stdout.contains("status=ok\n"),
        "combined check should report the missing profiler without proof-ready status: {stdout}"
    );
    assert!(
        stderr.contains("profile tool preflight failed"),
        "combined check should explain profiler readiness failure: stderr={stderr}"
    );
    assert!(
        !profile_created,
        "combined check should not create profile output directories"
    );
}

#[test]
fn eth_proof_timing_batch_check_env_does_not_require_profile_tools_by_default() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-env-check-missing-tool");
    let profile_dir = fixture.dir.join("profiles");
    let mut command = Command::new(script_path());
    command
        .arg("--suite")
        .arg("small")
        .arg("--check-env")
        .arg("--profile-output-dir")
        .arg(&profile_dir)
        .arg("--nsys-command")
        .arg(fixture.dir.join("missing-nsys"));
    fixture.apply_env(&mut command, SMALL_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch proof-only env check should run");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let profile_created = profile_dir.exists();
    fixture.cleanup();

    assert!(
        success,
        "proof-only env check should not fail on a missing profiler: stderr={stderr}"
    );
    assert!(
        stdout.contains("profile_tool=nsys\n")
            && stdout.contains("nsys_profiler_status=missing\n")
            && stdout.contains("status=ok\n")
            && stdout.contains("small=ready\n"),
        "proof-only env check should report missing profiler but still validate proof inputs: {stdout}"
    );
    assert!(
        !profile_created,
        "proof-only env check should not create profile output directories"
    );
}

#[test]
fn eth_proof_timing_batch_check_profile_tools_uses_env_profiler_command() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-profile-tools-env");
    let profile_dir = fixture.dir.join("profiles");
    let tool_path = write_fixture(&fixture.dir, "env-nsys");
    make_executable(&tool_path);
    let mut command = Command::new(script_path());
    command
        .arg("--check-profile-tools")
        .arg("--profile-output-dir")
        .arg(&profile_dir)
        .env("LZVM_NSYS_COMMAND", &tool_path);
    clear_env(&mut command, SMALL_PREFIX);
    clear_env(&mut command, LARGE_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch env profile tool check should run");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let profile_created = profile_dir.exists();
    fixture.cleanup();

    assert!(
        success,
        "env profile tool check should not require proof envs: stderr={stderr}"
    );
    assert!(
        stdout.contains("profile_tool=nsys\n")
            && stdout.contains("nsys_profiler_source=env\n")
            && stdout.contains(&format!("nsys_profiler_command={}\n", tool_path.display()))
            && stdout.contains("nsys_profiler_status=ready\n")
            && stdout
                .contains("nsys_profiler_resolved=temp/eth-proof-timing-batch-profile-tools-env-"),
        "profile tool check should report the env-selected profiler: {stdout}"
    );
    assert!(
        !profile_created,
        "env profile tool check should not create profile output directories"
    );
}

#[test]
fn eth_proof_timing_batch_check_profile_tools_resolves_bare_command_from_path() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-profile-tools-path");
    let profile_dir = fixture.dir.join("profiles");
    let bin_dir = fixture.dir.join("bin");
    std::fs::create_dir_all(&bin_dir).expect("fixture bin dir should be created");
    let tool_path = write_fixture(&bin_dir, "nsys");
    make_executable(&tool_path);
    let mut command = Command::new(script_path());
    command
        .arg("--check-profile-tools")
        .arg("--profile-output-dir")
        .arg(&profile_dir);
    clear_env(&mut command, SMALL_PREFIX);
    clear_env(&mut command, LARGE_PREFIX);
    command
        .env_remove("LZVM_NSYS_COMMAND")
        .env_remove("LZVM_NCU_COMMAND");
    prepend_path(&mut command, &bin_dir);

    let output = command
        .output()
        .expect("ETH proof timing batch PATH profile tool check should run");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let profile_created = profile_dir.exists();
    fixture.cleanup();

    assert!(
        success,
        "PATH profile tool check should not require proof envs: stderr={stderr}"
    );
    assert!(
        stdout.contains("profile_tool=nsys\n")
            && stdout.contains("nsys_profiler_source=path\n")
            && stdout.contains("nsys_profiler_command=nsys\n")
            && stdout.contains("nsys_profiler_status=ready\n")
            && stdout
                .contains("nsys_profiler_resolved=temp/eth-proof-timing-batch-profile-tools-path-"),
        "profile tool check should resolve bare profiler commands from PATH: {stdout}"
    );
    assert!(
        !profile_created,
        "PATH profile tool check should not create profile output directories"
    );
}

#[test]
fn eth_proof_timing_batch_check_profile_tools_fails_when_selected_tool_is_missing() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-profile-tools-missing");
    let profile_dir = fixture.dir.join("profiles");
    let tool_path = fixture.dir.join("missing-ncu");
    let mut command = Command::new(script_path());
    command
        .arg("--check-profile-tools")
        .arg("--profile-tool")
        .arg("ncu")
        .arg("--profile-output-dir")
        .arg(&profile_dir)
        .arg("--ncu-command")
        .arg(&tool_path);
    clear_env(&mut command, SMALL_PREFIX);
    clear_env(&mut command, LARGE_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch missing profile tool check should run");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let profile_created = profile_dir.exists();
    fixture.cleanup();

    assert!(!success, "missing selected profile tool should fail");
    assert!(
        stderr.is_empty(),
        "missing profile tool should report status on stdout only: stderr={stderr}"
    );
    assert!(
        stdout.contains("profile_tool=ncu\n")
            && stdout.contains("ncu_profiler_source=arg\n")
            && stdout.contains(&format!("ncu_profiler_command={}\n", tool_path.display()))
            && stdout.contains("ncu_profiler_status=missing\n")
            && !stdout.contains("ncu_profiler_resolved="),
        "missing profile tool check should report the unresolved profiler: {stdout}"
    );
    assert!(
        !stdout.contains("proof environment"),
        "profile tool check should not require proof envs: {stdout}"
    );
    assert!(
        !profile_created,
        "missing profile tool check should not create profile output directories"
    );
}

#[test]
fn eth_proof_timing_batch_check_profile_tools_fails_when_one_selected_tool_is_missing() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-profile-tools-both-missing");
    let profile_dir = fixture.dir.join("profiles");
    let nsys_path = write_fixture(&fixture.dir, "custom-nsys");
    make_executable(&nsys_path);
    let ncu_path = fixture.dir.join("missing-ncu");
    let mut command = Command::new(script_path());
    command
        .arg("--check-profile-tools")
        .arg("--profile-tool")
        .arg("both")
        .arg("--profile-output-dir")
        .arg(&profile_dir)
        .arg("--nsys-command")
        .arg(&nsys_path)
        .arg("--ncu-command")
        .arg(&ncu_path);
    clear_env(&mut command, SMALL_PREFIX);
    clear_env(&mut command, LARGE_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch mixed profile tool check should run");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let profile_created = profile_dir.exists();
    fixture.cleanup();

    assert!(
        !success,
        "profile tool check should fail when any selected tool is missing"
    );
    assert!(
        stderr.is_empty(),
        "mixed profile tool check should report status on stdout only: stderr={stderr}"
    );
    assert!(
        stdout.contains("profile_tool=both\n")
            && stdout.contains("nsys_profiler_status=ready\n")
            && stdout.contains("ncu_profiler_status=missing\n"),
        "mixed profile tool check should report both selected tool statuses: {stdout}"
    );
    assert!(
        !profile_created,
        "mixed profile tool check should not create profile output directories"
    );
}

#[test]
fn eth_proof_timing_batch_check_gpu_memory_reports_ready_status() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-gpu-memory-ready");
    let smi_path = write_executable_script(
        &fixture.dir,
        "nvidia-smi-ready",
        "#!/usr/bin/env python3\nprint('0, 24576, 4096, 20480')\n",
    );
    let mut command = Command::new(script_path());
    command
        .arg("--check-gpu-memory")
        .arg("--min-gpu-free-mib")
        .arg("1024")
        .arg("--nvidia-smi-command")
        .arg(&smi_path);
    command.env_remove("CUDA_VISIBLE_DEVICES");
    clear_env(&mut command, SMALL_PREFIX);
    clear_env(&mut command, LARGE_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch GPU memory check should run");
    let success = output.status.success();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    fixture.cleanup();

    assert!(success, "GPU memory check should pass: stderr={stderr}");
    assert!(
        stdout.contains("gpu_memory_source=arg\n")
            && stdout.contains(&format!("gpu_memory_command={}\n", smi_path.display()))
            && stdout.contains("gpu_memory_min_free_mib=1024\n")
            && stdout.contains("gpu_memory_device_count=1\n")
            && stdout.contains("gpu_memory_selected_index=0\n")
            && stdout.contains("gpu_memory_free_mib=20480\n")
            && stdout.contains("gpu_memory_status=ready\n"),
        "GPU memory check should report ready capacity: {stdout}"
    );
    assert!(
        !stdout.contains("status=ok\n") && !stdout.contains("proof environment"),
        "standalone GPU memory check should not require proof inputs: {stdout}"
    );
}

#[test]
fn eth_proof_timing_batch_check_gpu_memory_fails_when_free_memory_is_low() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-gpu-memory-low");
    let smi_path = write_executable_script(
        &fixture.dir,
        "nvidia-smi-low",
        "#!/usr/bin/env python3\nprint('0, 24576, 24288, 288')\n",
    );
    let mut command = Command::new(script_path());
    command
        .arg("--check-gpu-memory")
        .arg("--min-gpu-free-mib")
        .arg("1024")
        .arg("--nvidia-smi-command")
        .arg(&smi_path);
    command.env_remove("CUDA_VISIBLE_DEVICES");
    clear_env(&mut command, SMALL_PREFIX);
    clear_env(&mut command, LARGE_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch low GPU memory check should run");
    let success = output.status.success();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    fixture.cleanup();

    assert!(
        !success,
        "GPU memory check should fail when free memory is below the configured floor"
    );
    assert!(
        stderr.is_empty(),
        "low GPU memory should report status on stdout only: stderr={stderr}"
    );
    assert!(
        stdout.contains("gpu_memory_free_mib=288\n")
            && stdout.contains("gpu_memory_min_free_mib=1024\n")
            && stdout.contains("gpu_memory_status=low\n"),
        "GPU memory check should explain low free memory: {stdout}"
    );
}

#[test]
fn eth_proof_timing_batch_rejects_gpu_memory_wait_without_check() {
    let mut command = Command::new(script_path());
    command
        .arg("--gpu-memory-wait-timeout-s")
        .arg("1")
        .arg("--gpu-memory-wait-poll-s")
        .arg("0.1");
    clear_env(&mut command, SMALL_PREFIX);
    clear_env(&mut command, LARGE_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch GPU memory wait validation should run");
    let success = output.status.success();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    assert!(
        !success,
        "GPU memory wait flags should require the GPU memory check"
    );
    assert!(
        stdout.is_empty(),
        "GPU memory wait validation should fail before printing status: {stdout}"
    );
    assert!(
        stderr.contains("--gpu-memory-wait-* requires --check-gpu-memory"),
        "GPU memory wait validation should explain the required flag: stderr={stderr}"
    );
}

#[test]
fn eth_proof_timing_batch_check_gpu_memory_waits_until_ready() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-gpu-memory-wait");
    let state_path = fixture.dir.join("smi-count");
    let smi_source = format!(
        concat!(
            "#!/usr/bin/env python3\n",
            "from pathlib import Path\n",
            "state = Path(r'{}')\n",
            "count = int(state.read_text()) if state.exists() else 0\n",
            "state.write_text(str(count + 1))\n",
            "if count == 0:\n",
            "    print('0, 24576, 24288, 288')\n",
            "else:\n",
            "    print('0, 24576, 4096, 20480')\n",
        ),
        state_path.display()
    );
    let smi_path = write_executable_script(&fixture.dir, "nvidia-smi-wait", &smi_source);
    let mut command = Command::new(script_path());
    command
        .arg("--check-gpu-memory")
        .arg("--min-gpu-free-mib")
        .arg("1024")
        .arg("--gpu-memory-wait-timeout-s")
        .arg("2")
        .arg("--gpu-memory-wait-poll-s")
        .arg("0.01")
        .arg("--nvidia-smi-command")
        .arg(&smi_path);
    command.env_remove("CUDA_VISIBLE_DEVICES");
    clear_env(&mut command, SMALL_PREFIX);
    clear_env(&mut command, LARGE_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch waiting GPU memory check should run");
    let success = output.status.success();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let attempt_count =
        std::fs::read_to_string(&state_path).expect("GPU memory fixture count should write");
    fixture.cleanup();

    assert!(
        success,
        "GPU memory check should pass after waiting: stderr={stderr}"
    );
    assert_eq!(
        attempt_count, "2",
        "GPU memory wait should retry after the initial low-memory sample"
    );
    assert!(
        stdout.contains("gpu_memory_wait_timeout_s=2.0\n")
            && stdout.contains("gpu_memory_wait_poll_s=0.01\n")
            && stdout.contains("gpu_memory_wait_attempt=1\n")
            && stdout.contains("gpu_memory_status=low\n")
            && stdout.contains("gpu_memory_wait_attempt=2\n")
            && stdout.contains("gpu_memory_status=ready\n")
            && stdout.contains("gpu_memory_wait_status=ready\n"),
        "GPU memory wait should report low and ready samples: {stdout}"
    );
}

#[test]
fn eth_proof_timing_batch_check_gpu_memory_uses_default_cuda_device() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-gpu-memory-default-device");
    let smi_path = write_executable_script(
        &fixture.dir,
        "nvidia-smi-multi",
        "#!/usr/bin/env python3\nprint('0, GPU-low, 24576, 24288, 288')\nprint('1, GPU-free, 24576, 4096, 20480')\n",
    );
    let mut command = Command::new(script_path());
    command
        .arg("--check-gpu-memory")
        .arg("--min-gpu-free-mib")
        .arg("1024")
        .arg("--nvidia-smi-command")
        .arg(&smi_path);
    command.env_remove("CUDA_VISIBLE_DEVICES");
    clear_env(&mut command, SMALL_PREFIX);
    clear_env(&mut command, LARGE_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch multi-GPU memory check should run");
    let success = output.status.success();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    fixture.cleanup();

    assert!(
        !success,
        "GPU memory check should not pass by selecting a non-default free GPU"
    );
    assert!(
        stderr.is_empty(),
        "default GPU memory check should report status on stdout only: stderr={stderr}"
    );
    assert!(
        stdout.contains("gpu_memory_device_count=2\n")
            && stdout.contains("gpu_memory_selected_index=0\n")
            && stdout.contains("gpu_memory_selected_uuid=GPU-low\n")
            && stdout.contains("gpu_memory_free_mib=288\n")
            && stdout.contains("gpu_memory_status=low\n"),
        "GPU memory check should inspect CUDA device 0 by default: {stdout}"
    );
}

#[test]
fn eth_proof_timing_batch_check_gpu_memory_uses_first_visible_cuda_device() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-gpu-memory-visible-device");
    let smi_path = write_executable_script(
        &fixture.dir,
        "nvidia-smi-visible",
        "#!/usr/bin/env python3\nprint('0, GPU-low, 24576, 24288, 288')\nprint('1, GPU-free, 24576, 4096, 20480')\n",
    );
    let mut command = Command::new(script_path());
    command
        .arg("--check-gpu-memory")
        .arg("--min-gpu-free-mib")
        .arg("1024")
        .arg("--nvidia-smi-command")
        .arg(&smi_path)
        .env("CUDA_VISIBLE_DEVICES", "1,0");
    clear_env(&mut command, SMALL_PREFIX);
    clear_env(&mut command, LARGE_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch visible GPU memory check should run");
    let success = output.status.success();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    fixture.cleanup();

    assert!(
        success,
        "GPU memory check should use the first CUDA-visible device: stderr={stderr}"
    );
    assert!(
        stdout.contains("gpu_memory_cuda_visible_devices=1,0\n")
            && stdout.contains("gpu_memory_device_count=2\n")
            && stdout.contains("gpu_memory_selected_index=1\n")
            && stdout.contains("gpu_memory_selected_uuid=GPU-free\n")
            && stdout.contains("gpu_memory_free_mib=20480\n")
            && stdout.contains("gpu_memory_status=ready\n"),
        "GPU memory check should inspect the first CUDA-visible GPU: {stdout}"
    );
}

#[test]
fn eth_proof_timing_batch_check_env_fails_when_gpu_memory_is_low() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-check-env-gpu-memory-low");
    let smi_path = write_executable_script(
        &fixture.dir,
        "nvidia-smi-low",
        "#!/usr/bin/env python3\nprint('0, 24576, 24288, 288')\n",
    );
    let mut command = Command::new(script_path());
    command
        .arg("--suite")
        .arg("small")
        .arg("--check-env")
        .arg("--check-gpu-memory")
        .arg("--min-gpu-free-mib")
        .arg("1024")
        .arg("--nvidia-smi-command")
        .arg(&smi_path);
    command.env_remove("CUDA_VISIBLE_DEVICES");
    fixture.apply_env(&mut command, SMALL_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch env GPU memory check should run");
    let success = output.status.success();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    fixture.cleanup();

    assert!(
        !success,
        "env check should fail when GPU memory is below the configured floor"
    );
    assert!(
        stdout.contains("gpu_memory_status=low\n") && !stdout.contains("status=ok\n"),
        "env check should report low GPU memory without reporting ready status: {stdout}"
    );
    assert!(
        stderr.contains("GPU memory preflight failed"),
        "env check should explain the preflight failure: stderr={stderr}"
    );
}

#[test]
fn eth_proof_timing_batch_check_env_gpu_memory_ready_preserves_next_commands() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-check-env-gpu-memory-ready");
    let smi_path = write_executable_script(
        &fixture.dir,
        "nvidia-smi-ready",
        "#!/usr/bin/env python3\nprint('0, GPU-free, 24576, 4096, 20480')\n",
    );
    let smi_rel = smi_path
        .strip_prefix(workspace_root())
        .expect("smi path should be under workspace")
        .display()
        .to_string();
    let mut command = Command::new(script_path());
    command
        .arg("--suite")
        .arg("small")
        .arg("--check-env")
        .arg("--check-gpu-memory")
        .arg("--min-gpu-free-mib")
        .arg("2048")
        .arg("--gpu-memory-wait-timeout-s")
        .arg("30")
        .arg("--gpu-memory-wait-poll-s")
        .arg("0.5")
        .arg("--nvidia-smi-command")
        .arg(&smi_path)
        .arg("--skip-targets");
    command.env_remove("CUDA_VISIBLE_DEVICES");
    fixture.apply_env(&mut command, SMALL_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch env GPU memory check should run");
    let success = output.status.success();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let preflight_line = stdout
        .lines()
        .find(|line| line.starts_with("next_preflight_command="))
        .unwrap_or("");
    let profile_line = stdout
        .lines()
        .find(|line| line.starts_with("next_profile_command="))
        .unwrap_or("");
    let run_line = stdout
        .lines()
        .find(|line| line.starts_with("next_run_command="))
        .unwrap_or("");
    fixture.cleanup();

    assert!(
        success,
        "env check should pass when GPU memory meets the configured floor: stderr={stderr}"
    );
    assert!(
        stdout.contains("gpu_memory_status=ready\n") && stdout.contains("status=ok\n"),
        "env check should report ready GPU memory and proof env status: {stdout}"
    );
    for line in [preflight_line, profile_line, run_line] {
        assert!(
            line.contains("--check-gpu-memory")
                && line.contains("--min-gpu-free-mib 2048")
                && line.contains("--gpu-memory-wait-timeout-s 30.0")
                && line.contains("--gpu-memory-wait-poll-s 0.5")
                && line.contains(&format!("--nvidia-smi-command {smi_rel}")),
            "next command should preserve GPU memory preflight flags: {stdout}"
        );
    }
    assert!(
        preflight_line.contains("--check-env --check-profile-tools"),
        "preflight command should keep the combined env and profile-tool checks: {stdout}"
    );
}

#[test]
fn eth_proof_timing_batch_prints_env_template_without_config() {
    let mut command = Command::new(script_path());
    command
        .arg("--suite")
        .arg("small")
        .arg("--print-env-template")
        .env("CUDA_VISIBLE_DEVICES", "");
    clear_env(&mut command, SMALL_PREFIX);
    clear_env(&mut command, LARGE_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch env template should run");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");

    assert!(
        success,
        "env template should not require configured inputs: stderr={stderr}"
    );
    assert!(
        stderr.is_empty(),
        "env template should not warn about missing inputs: stderr={stderr}"
    );
    assert!(
        stdout.contains("# cargo build --release -p lzvm-cli --bin lzvm --features cuda"),
        "env template should include the default binary build command: {stdout}"
    );
    assert_artifact_template_help(&stdout);
    assert!(
        stdout.contains("export LZVM_REAL_SMALL_PARITY_BIN=target/release/lzvm"),
        "small template should include the default binary path: {stdout}"
    );
    assert!(
        stdout.contains("export LZVM_REAL_SMALL_PARITY_SETUP=")
            && stdout.contains("export LZVM_REAL_SMALL_PARITY_BLOCK_INPUT=")
            && stdout.contains("export LZVM_REAL_SMALL_PARITY_PROGRAM_IMAGE_CACHE=")
            && stdout.contains("export LZVM_REAL_SMALL_PARITY_INPUT_DATA=")
            && stdout.contains("export LZVM_REAL_SMALL_PARITY_GUEST_IMAGE="),
        "small template should include every required input: {stdout}"
    );
    assert!(
        stdout.contains("# INPUT_DATA must be framed guest stdin"),
        "env template should explain the guest stdin format: {stdout}"
    );
    assert!(
        stdout.contains("export LZVM_REAL_SMALL_PARITY_TRACE_LIMIT=120000000"),
        "small template should include optional trace limit default: {stdout}"
    );
    assert!(
        stdout.contains("# optional GPU selection for reproducible timing and profiling")
            && stdout.contains("# export CUDA_VISIBLE_DEVICES=0")
            && !stdout.contains("\nexport CUDA_VISIBLE_DEVICES="),
        "env template should document GPU selection without hiding devices by default: {stdout}"
    );
    assert!(
        !stdout.contains("LZVM_REAL_SMALL_PARITY_TMP_DIR"),
        "small template should not expose a shared TMPDIR: {stdout}"
    );
    assert!(
        !stdout.contains(LARGE_PREFIX),
        "small template should not include the large prefix: {stdout}"
    );
}

#[test]
fn eth_proof_timing_batch_writes_env_template_under_temp() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-write-env-template");
    let template_path = fixture.dir.join("real-proof.env");
    let runner_path = fixture.dir.join("custom runner.py");
    let nsys_path = fixture.dir.join("custom nsys");
    let ncu_path = fixture.dir.join("custom ncu");
    let smi_path = fixture.dir.join("custom nvidia-smi");
    let nsys_trace = "cpu,nvtx";
    let ncu_set = "full";
    let ncu_target_processes = "application";
    let template_rel = template_path
        .strip_prefix(workspace_root())
        .expect("template path should be under workspace")
        .display()
        .to_string();
    let runner_rel = runner_path
        .strip_prefix(workspace_root())
        .expect("runner path should be under workspace")
        .display()
        .to_string();
    let nsys_rel = nsys_path
        .strip_prefix(workspace_root())
        .expect("nsys path should be under workspace")
        .display()
        .to_string();
    let ncu_rel = ncu_path
        .strip_prefix(workspace_root())
        .expect("ncu path should be under workspace")
        .display()
        .to_string();
    let smi_rel = smi_path
        .strip_prefix(workspace_root())
        .expect("smi path should be under workspace")
        .display()
        .to_string();
    let mut command = Command::new(script_path());
    command
        .arg("--suite")
        .arg("both")
        .arg("--small-mode")
        .arg("pipeline")
        .arg("--large-mode")
        .arg("work-units")
        .arg("--runs")
        .arg("3")
        .arg("--max-runs")
        .arg("5")
        .arg("--work-dir")
        .arg(fixture.dir.join("runs"))
        .arg("--path")
        .arg(fixture.dir.join("improve-log.csv"))
        .arg("--runner")
        .arg(&runner_path)
        .arg("--nsys-command")
        .arg(&nsys_path)
        .arg("--ncu-command")
        .arg(&ncu_path)
        .arg("--nsys-trace")
        .arg(nsys_trace)
        .arg("--ncu-set")
        .arg(ncu_set)
        .arg("--ncu-target-processes")
        .arg(ncu_target_processes)
        .arg("--check-gpu-memory")
        .arg("--min-gpu-free-mib")
        .arg("16384")
        .arg("--nvidia-smi-command")
        .arg(&smi_path)
        .arg("--skip-nsys-export")
        .arg("--gpu-preallocate")
        .arg("--minimal-memory")
        .arg("--no-pack-trace")
        .arg("--gpu-streams")
        .arg("7")
        .arg("--witness-thread-pools")
        .arg("5")
        .arg("--stored-witnesses")
        .arg("3")
        .arg("--profile-arg=--kernel-name-base=demangled")
        .arg("--commit")
        .arg("&&")
        .arg("--write-env-template")
        .arg(&template_path)
        .env("CUDA_VISIBLE_DEVICES", "1,0");
    clear_env(&mut command, SMALL_PREFIX);
    clear_env(&mut command, LARGE_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch env template should write");
    let success = output.status.success();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let template = std::fs::read_to_string(&template_path).expect("env template should be written");
    let profile_tool_prefix = format!(
        "next_profile_tool_check_command=scripts/run-eth-proof-timing-batch.py --suite both --small-mode pipeline --large-mode work-units --runs 3 --env-file {template_rel}"
    );
    let profile_tool_line = stdout
        .lines()
        .find(|line| line.starts_with("next_profile_tool_check_command="))
        .unwrap_or("");
    let preflight_prefix = format!(
        "next_preflight_command=scripts/run-eth-proof-timing-batch.py --suite both --small-mode pipeline --large-mode work-units --runs 3 --env-file {template_rel}"
    );
    let preflight_line = stdout
        .lines()
        .find(|line| line.starts_with("next_preflight_command="))
        .unwrap_or("");
    fixture.cleanup();

    assert!(
        success,
        "env template should write under temp: stderr={stderr}"
    );
    assert!(
        stderr.is_empty(),
        "env template write should not warn: stderr={stderr}"
    );
    assert!(
        stdout.contains(&format!("env_template={template_rel}\n")),
        "env template path should be reported: {stdout}"
    );
    assert!(
        stdout.contains(&format!(
            "next_check_command=scripts/run-eth-proof-timing-batch.py --suite both --small-mode pipeline --large-mode work-units --runs 3 --env-file {template_rel}"
        )),
        "env template should report a check command: {stdout}"
    );
    assert!(
        profile_tool_line.starts_with(&profile_tool_prefix)
            && profile_tool_line.contains("--check-profile-tools")
            && !profile_tool_line.contains("--check-env"),
        "env template should report a proof-independent profile tool check command: {stdout}"
    );
    assert!(
        preflight_line.starts_with(&preflight_prefix)
            && preflight_line.contains("--check-env --check-profile-tools"),
        "env template should report a combined proof and profile preflight command: {stdout}"
    );
    assert!(
        stdout.contains(&format!(
            "next_profile_command=scripts/run-eth-proof-timing-batch.py --suite both --small-mode pipeline --large-mode work-units --runs 3 --env-file {template_rel}"
        )),
        "env template should report a profile command: {stdout}"
    );
    assert!(
        stdout.contains(&format!(
            "next_run_command=scripts/run-eth-proof-timing-batch.py --suite both --small-mode pipeline --large-mode work-units --runs 3 --env-file {template_rel}"
        )),
        "env template should report a run command: {stdout}"
    );
    assert!(
        stdout.contains("--max-runs 5") && stdout.contains("--summary 'real proof timing'"),
        "run command should preserve the retry cap and summary placeholder: {stdout}"
    );
    for proof_arg in [
        "--gpu-preallocate",
        "--minimal-memory",
        "--no-pack-trace",
        "--gpu-streams 7",
        "--witness-thread-pools 5",
        "--stored-witnesses 3",
    ] {
        assert!(
            stdout.contains(proof_arg),
            "next commands should preserve proof tuning argument {proof_arg}: {stdout}"
        );
    }
    assert!(
        stdout.contains(&format!("--runner '{runner_rel}'")),
        "env template command should preserve custom runner paths: {stdout}"
    );
    assert!(
        stdout.contains(&format!("--nsys-command '{nsys_rel}'"))
            && stdout.contains(&format!("--ncu-command '{ncu_rel}'")),
        "env template command should preserve profiler executable paths: {stdout}"
    );
    assert!(
        stdout.contains("--check-gpu-memory")
            && stdout.contains("--min-gpu-free-mib 16384")
            && stdout.contains(&format!("--nvidia-smi-command '{smi_rel}'")),
        "env template command should preserve GPU memory preflight settings: {stdout}"
    );
    assert!(
        stdout.contains(&format!("--nsys-trace {nsys_trace}"))
            && stdout.contains(&format!("--ncu-set {ncu_set}"))
            && stdout.contains(&format!("--ncu-target-processes {ncu_target_processes}"))
            && stdout.contains("--skip-nsys-export"),
        "env template command should preserve profiler tuning flags: {stdout}"
    );
    assert!(
        stdout.contains("--profile-arg=--kernel-name-base=demangled")
            && !stdout.contains("--profile-arg --kernel-name-base=demangled"),
        "env template command should preserve profiler passthrough args: {stdout}"
    );
    assert!(
        stdout.contains("--commit '&&'"),
        "env template command should quote commit values that look like shell syntax: {stdout}"
    );
    assert!(
        template.contains("# run with --small-mode pipeline")
            && template.contains("# run with --large-mode work-units"),
        "template should record selected modes: {template}"
    );
    assert_artifact_template_help(&template);
    assert!(
        template.contains("# GPU selection captured from the current environment")
            && template.contains("export CUDA_VISIBLE_DEVICES=1,0"),
        "template should preserve explicit GPU selection for reproducible timing: {template}"
    );
    assert!(
        template.contains("export LZVM_REAL_SMALL_PARITY_SETUP=")
            && template.contains("export LZVM_REAL_LARGE_PARITY_SETUP="),
        "template should include both selected suites: {template}"
    );
    assert!(
        template
            .lines()
            .filter(|line| *line == "# INPUT_DATA must be framed guest stdin")
            .count()
            == 2,
        "template should explain the guest stdin format for both suites: {template}"
    );
    assert!(
        !template.contains("TMP_DIR"),
        "template should not expose a shared TMPDIR: {template}"
    );
}

#[test]
fn eth_proof_timing_batch_env_template_reuses_existing_env_file_values() {
    let fixture = ProofFixture::new("eth proof timing batch env template prefill");
    let env_path = fixture.dir.join("partial.env");
    let template_path = fixture.dir.join("partial.template.env");
    std::fs::write(
        &env_path,
        format!(
            "export {SMALL_PREFIX}_BIN='{}'\nexport {SMALL_PREFIX}_SETUP='{}'\nexport {SMALL_PREFIX}_TRACE_LIMIT=42\n",
            fixture.fake_bin.display(),
            fixture.setup.display(),
        ),
    )
    .expect("partial env file should write");
    let expected_bin = format!("export {SMALL_PREFIX}_BIN='{}'", fixture.fake_bin.display());
    let expected_setup = format!("export {SMALL_PREFIX}_SETUP='{}'", fixture.setup.display());
    let expected_trace_limit = format!("export {SMALL_PREFIX}_TRACE_LIMIT=42");
    let mut print_command = Command::new(script_path());
    print_command
        .arg("--suite")
        .arg("small")
        .arg("--env-file")
        .arg(&env_path)
        .arg("--print-env-template");
    clear_env(&mut print_command, SMALL_PREFIX);
    clear_env(&mut print_command, LARGE_PREFIX);

    let print_output = print_command
        .output()
        .expect("ETH proof timing batch env template should print");
    let print_success = print_output.status.success();
    let print_stdout = String::from_utf8(print_output.stdout).expect("stdout should be utf-8");
    let print_stderr = String::from_utf8_lossy(&print_output.stderr).into_owned();

    let mut command = Command::new(script_path());
    command
        .arg("--suite")
        .arg("small")
        .arg("--env-file")
        .arg(&env_path)
        .arg("--write-env-template")
        .arg(&template_path);
    clear_env(&mut command, SMALL_PREFIX);
    clear_env(&mut command, LARGE_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch env template should write");
    let success = output.status.success();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let template = std::fs::read_to_string(&template_path).expect("env template should be written");
    fixture.cleanup();

    assert!(
        success,
        "env template should write from a partial env file: stderr={stderr}"
    );
    assert!(
        stdout.contains("next_check_command="),
        "env template write should still report follow-up commands: {stdout}"
    );
    assert!(
        print_success,
        "env template should print from a partial env file: stderr={print_stderr}"
    );
    assert!(
        print_stdout.contains(&expected_bin)
            && print_stdout.contains(&expected_setup)
            && print_stdout.contains(&expected_trace_limit),
        "printed env template should keep values already present in the env file: {print_stdout}"
    );
    assert!(
        print_stderr.is_empty(),
        "env template print should not warn: {print_stderr}"
    );
    assert!(
        template.contains(&expected_bin)
            && template.contains(&expected_setup)
            && template.contains(&expected_trace_limit),
        "env template should keep values already present in the env file: {template}"
    );
    assert!(
        template.contains(&format!("export {SMALL_PREFIX}_BLOCK_INPUT=\n"))
            && template.contains(&format!("export {SMALL_PREFIX}_PROGRAM_IMAGE_CACHE=\n"))
            && template.contains(&format!("export {SMALL_PREFIX}_INPUT_DATA=\n"))
            && template.contains(&format!("export {SMALL_PREFIX}_GUEST_IMAGE=\n")),
        "env template should leave missing required paths blank: {template}"
    );
    assert!(stderr.is_empty(), "env template should not warn: {stderr}");
}

#[test]
fn eth_proof_timing_batch_missing_env_file_suggests_template_command() {
    let fixture = ProofFixture::new("eth proof timing batch missing env file");
    let env_path = fixture.dir.join("missing.env");
    let mut command = Command::new(script_path());
    command
        .arg("--suite")
        .arg("small")
        .arg("--check-env")
        .arg("--env-file")
        .arg(&env_path);
    clear_env(&mut command, SMALL_PREFIX);
    clear_env(&mut command, LARGE_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch missing env-file check should run");
    let success = output.status.success();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    fixture.cleanup();

    assert!(!success, "missing env file should fail");
    assert!(
        stdout.is_empty(),
        "missing env-file rejection should happen before partial diagnostics: {stdout}"
    );
    assert!(
        stderr.contains("--env-file path does not exist")
            && stderr.contains("create a template with:")
            && stderr.contains("scripts/run-eth-proof-timing-batch.py")
            && stderr.contains("--write-env-template")
            && stderr.contains("missing.env"),
        "missing env-file rejection should include a template recovery command: stderr={stderr}"
    );
}

#[test]
fn eth_proof_timing_batch_env_file_configures_dry_run() {
    let fixture = ProofFixture::new("eth proof timing batch env file");
    let env_path = fixture.dir.join("proof.env");
    std::fs::write(
        &env_path,
        format!(
            concat!(
                "# proof timing env\n",
                "export {prefix}_BIN='{bin}'\n",
                "export {prefix}_SETUP='{setup}'\n",
                "export {prefix}_BLOCK_INPUT='{block_input}'\n",
                "export {prefix}_PROGRAM_IMAGE_CACHE='{cache}'\n",
                "export {prefix}_INPUT_DATA='{input_data}'\n",
                "export {prefix}_GUEST_IMAGE='{guest}'\n",
                "export {prefix}_TRACE_LIMIT=42\n",
                "export CUDA_VISIBLE_DEVICES=1,0\n",
            ),
            prefix = SMALL_PREFIX,
            bin = fixture.fake_bin.display(),
            setup = fixture.setup.display(),
            block_input = fixture.block_input.display(),
            cache = fixture.cache.display(),
            input_data = fixture.input_data.display(),
            guest = fixture.guest.display(),
        ),
    )
    .expect("env file should write");
    let mut command = Command::new(script_path());
    command
        .arg("--suite")
        .arg("small")
        .arg("--dry-run")
        .arg("--env-file")
        .arg(&env_path)
        .arg("--summary")
        .arg("env file");
    clear_env(&mut command, SMALL_PREFIX);
    clear_env(&mut command, LARGE_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch env-file dry-run should run");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    fixture.cleanup();

    assert!(
        success,
        "env-file dry-run should build a command from the env file: stderr={stderr}"
    );
    assert!(
        stdout.contains("prove witness --guest-pc-trace 42 --timings")
            && stdout.contains("TMPDIR={tmp_dir} CUDA_VISIBLE_DEVICES=1,0"),
        "env-file dry-run should pin the prove command to the configured GPU selector: {stdout}"
    );
    let verify_command = verify_command_tail(&stdout);
    assert!(
        verify_command.contains("TMPDIR={tmp_dir} CUDA_VISIBLE_DEVICES=1,0")
            && verify_command.contains("verify proof --eth-block-input"),
        "env-file dry-run should pin the verify command to the configured GPU selector: {stdout}"
    );
    assert!(
        stdout.contains("selected=small\n")
            && stdout.contains("small_mode=default\n")
            && stdout.contains("&& env -u LZVM_GUEST_PC_TRACE_PARALLEL_LOWER"),
        "env-file dry-run should load proof paths and trace limit: {stdout}"
    );
    assert_verify_required_text_args(&stdout, "env-file runner command");
}

#[test]
fn eth_proof_timing_batch_env_file_clears_ambient_proof_env() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-env-file-clears-ambient");
    let env_path = fixture.dir.join("large-only.env");
    std::fs::write(
        &env_path,
        format!(
            concat!(
                "export {prefix}_BIN='{bin}'\n",
                "export {prefix}_SETUP='{setup}'\n",
                "export {prefix}_BLOCK_INPUT='{block_input}'\n",
                "export {prefix}_PROGRAM_IMAGE_CACHE='{cache}'\n",
                "export {prefix}_INPUT_DATA='{input_data}'\n",
                "export {prefix}_GUEST_IMAGE='{guest}'\n",
            ),
            prefix = LARGE_PREFIX,
            bin = fixture.fake_bin.display(),
            setup = fixture.setup.display(),
            block_input = fixture.block_input.display(),
            cache = fixture.cache.display(),
            input_data = fixture.input_data.display(),
            guest = fixture.guest.display(),
        ),
    )
    .expect("env file should write");
    let mut command = Command::new(script_path());
    command
        .arg("--suite")
        .arg("available")
        .arg("--dry-run")
        .arg("--env-file")
        .arg(&env_path)
        .arg("--summary")
        .arg("env file");
    fixture.apply_env(&mut command, SMALL_PREFIX);
    clear_env(&mut command, LARGE_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch env-file dry-run should run");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    fixture.cleanup();

    assert!(
        success,
        "large-only env file should build an available command: stderr={stderr}"
    );
    assert!(
        stdout.contains("selected=large\n") && stdout.contains("large_command="),
        "env file should select the configured large workload: {stdout}"
    );
    assert!(
        !stdout.contains("selected=small")
            && !stdout.contains("small_command=")
            && !stdout.contains("--small-command"),
        "env file should not inherit ambient small workload settings: {stdout}"
    );
}

#[test]
fn eth_proof_timing_batch_env_file_configures_profile_and_gpu_tools() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-env-file-tools");
    let env_path = fixture.dir.join("tools.env");
    let nsys_path = write_fixture(&fixture.dir, "env-nsys");
    let ncu_path = write_fixture(&fixture.dir, "env-ncu");
    let smi_path = write_executable_script(
        &fixture.dir,
        "env-nvidia-smi",
        "#!/usr/bin/env python3\nprint('0, GPU-low, 24576, 24288, 288')\nprint('1, GPU-free, 24576, 4096, 20480')\n",
    );
    make_executable(&nsys_path);
    make_executable(&ncu_path);
    std::fs::write(
        &env_path,
        format!(
            concat!(
                "export LZVM_NSYS_COMMAND='{nsys}'\n",
                "export LZVM_NCU_COMMAND='{ncu}'\n",
                "export LZVM_NVIDIA_SMI_COMMAND='{smi}'\n",
                "export CUDA_VISIBLE_DEVICES=1,0\n",
            ),
            nsys = nsys_path.display(),
            ncu = ncu_path.display(),
            smi = smi_path.display(),
        ),
    )
    .expect("env file should write");
    let mut command = Command::new(script_path());
    command
        .arg("--check-profile-tools")
        .arg("--profile-tool")
        .arg("both")
        .arg("--check-gpu-memory")
        .arg("--min-gpu-free-mib")
        .arg("1024")
        .arg("--env-file")
        .arg(&env_path)
        .env_remove("LZVM_NSYS_COMMAND")
        .env_remove("LZVM_NCU_COMMAND")
        .env_remove("LZVM_NVIDIA_SMI_COMMAND")
        .env_remove("CUDA_VISIBLE_DEVICES");
    clear_env(&mut command, SMALL_PREFIX);
    clear_env(&mut command, LARGE_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch env-file tool check should run");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    fixture.cleanup();

    assert!(
        success,
        "env-file tool check should load allowed tool vars: stderr={stderr}"
    );
    assert!(
        stdout.contains("profile_tool=both\n")
            && stdout.contains("nsys_profiler_source=env\n")
            && stdout.contains(&format!("nsys_profiler_command={}\n", nsys_path.display()))
            && stdout.contains("ncu_profiler_source=env\n")
            && stdout.contains(&format!("ncu_profiler_command={}\n", ncu_path.display()))
            && stdout.contains("gpu_memory_source=env\n")
            && stdout.contains(&format!("gpu_memory_command={}\n", smi_path.display()))
            && stdout.contains("gpu_memory_cuda_visible_devices=1,0\n")
            && stdout.contains("gpu_memory_selected_index=1\n")
            && stdout.contains("gpu_memory_status=ready\n"),
        "env-file tool check should use allowed tool vars: {stdout}"
    );
}

#[test]
fn eth_proof_timing_batch_env_file_rejects_unapproved_env_names() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-env-file-unapproved");
    let env_path = fixture.dir.join("unapproved.env");
    std::fs::write(&env_path, "PATH=/shadow\n").expect("env file should write");
    let mut command = Command::new(script_path());
    command
        .arg("--check-profile-tools")
        .arg("--env-file")
        .arg(&env_path);
    clear_env(&mut command, SMALL_PREFIX);
    clear_env(&mut command, LARGE_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch env-file rejection should run");
    let success = output.status.success();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    fixture.cleanup();

    assert!(!success, "unapproved env name should be rejected");
    assert!(
        stdout.is_empty(),
        "env-file rejection should happen before partial diagnostics: {stdout}"
    );
    assert!(
        stderr.contains("env name is not allowed in --env-file: PATH"),
        "env-file rejection should explain the rejected name: stderr={stderr}"
    );
}

#[test]
fn eth_proof_timing_batch_env_file_rejects_duplicate_names() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-env-file-duplicate");
    let env_path = fixture.dir.join("duplicate.env");
    std::fs::write(
        &env_path,
        format!("export {SMALL_PREFIX}_TRACE_LIMIT=42\nexport {SMALL_PREFIX}_TRACE_LIMIT=43\n"),
    )
    .expect("env file should write");
    let mut command = Command::new(script_path());
    command
        .arg("--check-profile-tools")
        .arg("--env-file")
        .arg(&env_path);
    clear_env(&mut command, SMALL_PREFIX);
    clear_env(&mut command, LARGE_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch env-file duplicate rejection should run");
    let success = output.status.success();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    fixture.cleanup();

    assert!(!success, "duplicate env name should be rejected");
    assert!(
        stdout.is_empty(),
        "duplicate env-file rejection should happen before partial diagnostics: {stdout}"
    );
    assert!(
        stderr.contains(&format!(
            "duplicate env name in --env-file: {SMALL_PREFIX}_TRACE_LIMIT first set on line 1"
        )),
        "duplicate env-file rejection should identify the repeated name: stderr={stderr}"
    );
}

#[test]
fn eth_proof_timing_batch_rejects_env_file_outside_temp() {
    let env_path = workspace_root().join(format!(
        "target/eth-proof-timing-batch-env-file-outside-{}.env",
        std::process::id()
    ));
    std::fs::write(&env_path, "").expect("outside env file should write");
    let mut command = Command::new(script_path());
    command
        .arg("--suite")
        .arg("small")
        .arg("--check-env")
        .arg("--env-file")
        .arg(&env_path);
    clear_env(&mut command, SMALL_PREFIX);
    clear_env(&mut command, LARGE_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch env-file boundary check should run");
    let success = output.status.success();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let _ = std::fs::remove_file(&env_path);

    assert!(!success, "env-file outside temp should be rejected");
    assert!(
        stdout.is_empty(),
        "env-file rejection should happen before partial diagnostics: {stdout}"
    );
    assert!(
        stderr.contains("--env-file must be under"),
        "env-file rejection should explain the temp boundary: stderr={stderr}"
    );
}

#[cfg(unix)]
#[test]
fn eth_proof_timing_batch_rejects_symlinked_env_file() {
    use std::os::unix::fs::symlink;

    let fixture = ProofFixture::new("eth-proof-timing-batch-env-file-symlink");
    let redirected = fixture.dir.join("redirected.env");
    let env_path = fixture.dir.join("linked.env");
    std::fs::write(&redirected, "PATH=bad\n").expect("redirected env file should write");
    symlink(&redirected, &env_path).expect("env-file symlink fixture should be created");
    let mut command = Command::new(script_path());
    command
        .arg("--suite")
        .arg("small")
        .arg("--check-env")
        .arg("--env-file")
        .arg(&env_path);
    clear_env(&mut command, SMALL_PREFIX);
    clear_env(&mut command, LARGE_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch env-file symlink check should run");
    let success = output.status.success();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let redirected_text =
        std::fs::read_to_string(&redirected).expect("redirected env file should read");
    fixture.cleanup();

    assert!(!success, "env-file symlink should be rejected");
    assert!(
        stdout.is_empty(),
        "env-file symlink rejection should happen before partial diagnostics: {stdout}"
    );
    assert!(
        stderr.contains("--env-file must not be a symlink"),
        "env-file symlink rejection should explain the path constraint: stderr={stderr}"
    );
    assert_eq!(
        redirected_text, "PATH=bad\n",
        "rejected env-file should not modify a symlink target"
    );
}

#[cfg(unix)]
#[test]
fn eth_proof_timing_batch_rejects_dangling_symlinked_env_file() {
    use std::os::unix::fs::symlink;

    let fixture = ProofFixture::new("eth-proof-timing-batch-env-file-dangling-symlink");
    let env_path = fixture.dir.join("linked.env");
    symlink(fixture.dir.join("missing.env"), &env_path)
        .expect("dangling env-file symlink fixture should be created");
    let mut command = Command::new(script_path());
    command
        .arg("--suite")
        .arg("small")
        .arg("--check-env")
        .arg("--env-file")
        .arg(&env_path);
    clear_env(&mut command, SMALL_PREFIX);
    clear_env(&mut command, LARGE_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch dangling env-file symlink check should run");
    let success = output.status.success();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    fixture.cleanup();

    assert!(!success, "dangling env-file symlink should be rejected");
    assert!(
        stdout.is_empty(),
        "dangling env-file symlink rejection should happen before partial diagnostics: {stdout}"
    );
    assert!(
        stderr.contains("--env-file must not be a symlink"),
        "dangling env-file symlink rejection should explain the path constraint: stderr={stderr}"
    );
    assert!(
        !stderr.contains("--env-file path does not exist"),
        "dangling env-file symlink should not be treated as a missing file: stderr={stderr}"
    );
}

#[test]
fn eth_proof_timing_batch_env_template_preserves_env_profiler_commands() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-write-env-profile-env");
    let template_path = fixture.dir.join("real-proof.env");
    let nsys_path = fixture.dir.join("env nsys");
    let ncu_path = fixture.dir.join("env ncu");
    let nsys_rel = nsys_path
        .strip_prefix(workspace_root())
        .expect("nsys path should be under workspace")
        .display()
        .to_string();
    let ncu_rel = ncu_path
        .strip_prefix(workspace_root())
        .expect("ncu path should be under workspace")
        .display()
        .to_string();
    let mut command = Command::new(script_path());
    command
        .arg("--suite")
        .arg("both")
        .arg("--profile-tool")
        .arg("both")
        .arg("--write-env-template")
        .arg(&template_path)
        .env("LZVM_NSYS_COMMAND", &nsys_path)
        .env("LZVM_NCU_COMMAND", &ncu_path);
    clear_env(&mut command, SMALL_PREFIX);
    clear_env(&mut command, LARGE_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch env template should write");
    let success = output.status.success();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let template = std::fs::read_to_string(&template_path).expect("env template should be written");
    fixture.cleanup();

    assert!(
        success,
        "env template should preserve env profiler commands: stderr={stderr}"
    );
    assert!(
        stdout.contains(&format!("--nsys-command '{nsys_rel}'"))
            && stdout.contains(&format!("--ncu-command '{ncu_rel}'")),
        "next commands should embed profiler commands selected from env: {stdout}"
    );
    assert!(
        stdout.contains("next_profile_tool_check_command=")
            && stdout.contains("next_profile_command=")
            && stdout.contains("next_run_command="),
        "env template should report every next command with preserved profiler settings: {stdout}"
    );
    assert!(
        !template.contains("LZVM_NSYS_COMMAND") && !template.contains("LZVM_NCU_COMMAND"),
        "proof env template should not require ambient profiler env exports: {template}"
    );
}

#[test]
fn eth_proof_timing_batch_prints_profile_commands_from_env() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-profile-commands");
    let profile_dir = fixture.dir.join("profiles");
    let nsys_path = fixture.dir.join("custom nsys");
    let ncu_path = fixture.dir.join("custom ncu");
    let smi_path = write_executable_script(
        &fixture.dir,
        "nvidia-smi-ready",
        "#!/usr/bin/env python3\nprint('0, GPU-free, 24576, 4096, 20480')\n",
    );
    let nsys_trace = "cpu,nvtx";
    let ncu_set = "full";
    let ncu_target_processes = "application";
    let min_gpu_free_mib = "4096";
    let nsys_rel = nsys_path
        .strip_prefix(workspace_root())
        .expect("nsys path should be under workspace")
        .display()
        .to_string();
    let ncu_rel = ncu_path
        .strip_prefix(workspace_root())
        .expect("ncu path should be under workspace")
        .display()
        .to_string();
    let mut command = Command::new(script_path());
    command
        .arg("--suite")
        .arg("small")
        .arg("--small-mode")
        .arg("stream-pipeline")
        .arg("--parallel-lower-workers")
        .arg("6")
        .arg("--parallel-lower-job-queue")
        .arg("12")
        .arg("--segment-commit-workers")
        .arg("3")
        .arg("--profile-tool")
        .arg("both")
        .arg("--profile-output-dir")
        .arg(&profile_dir)
        .arg("--nsys-command")
        .arg(&nsys_path)
        .arg("--ncu-command")
        .arg(&ncu_path)
        .arg("--nsys-trace")
        .arg(nsys_trace)
        .arg("--ncu-set")
        .arg(ncu_set)
        .arg("--ncu-target-processes")
        .arg(ncu_target_processes)
        .arg("--skip-nsys-export")
        .arg("--check-gpu-memory")
        .arg("--min-gpu-free-mib")
        .arg(min_gpu_free_mib)
        .arg("--gpu-memory-wait-timeout-s")
        .arg("30")
        .arg("--gpu-memory-wait-poll-s")
        .arg("0.5")
        .arg("--nvidia-smi-command")
        .arg(&smi_path)
        .arg("--gpu-preallocate")
        .arg("--minimal-memory")
        .arg("--no-pack-trace")
        .arg("--gpu-streams")
        .arg("7")
        .arg("--witness-thread-pools")
        .arg("5")
        .arg("--stored-witnesses")
        .arg("3")
        .arg("--profile-arg=--kernel-name-base=demangled")
        .arg("--profile-arg=--launch-skip=1")
        .arg("--print-profile-commands")
        .env_remove("CUDA_VISIBLE_DEVICES");
    fixture.apply_env(&mut command, SMALL_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch profile command should print");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let nsys_command = stdout
        .lines()
        .find(|line| line.starts_with("small_nsys_profile_command="))
        .expect("nsys profile command should be printed");
    let ncu_command = stdout
        .lines()
        .find(|line| line.starts_with("small_ncu_profile_command="))
        .expect("ncu profile command should be printed");
    let profile_created = profile_dir.exists();
    fixture.cleanup();

    assert!(
        success,
        "profile command output should use configured env: stderr={stderr}"
    );
    assert!(
        stdout.contains("small_nsys_profile_command=scripts/run-proof-profile.py --tool nsys")
            && stdout.contains("small_ncu_profile_command=scripts/run-proof-profile.py --tool ncu"),
        "profile command output should include both selected tools: {stdout}"
    );
    assert!(
        stdout.contains(&format!("--nsys-command '{nsys_rel}'"))
            && stdout.contains(&format!("--ncu-command '{ncu_rel}'")),
        "profile command output should route custom profiler executables through the matching tool: {stdout}"
    );
    assert!(
        stdout.contains(&format!("--nsys-trace {nsys_trace}"))
            && stdout.contains(&format!("--ncu-set {ncu_set}"))
            && stdout.contains(&format!("--ncu-target-processes {ncu_target_processes}"))
            && stdout.contains("--skip-nsys-export"),
        "profile command output should route profiler tuning flags through the matching tool: {stdout}"
    );
    assert!(
        stdout.contains("gpu_memory_status=ready\n")
            && nsys_command.contains("--check-gpu-memory")
            && nsys_command.contains(&format!("--min-gpu-free-mib {min_gpu_free_mib}"))
            && nsys_command.contains("--nvidia-smi-command")
            && !nsys_command.contains("--gpu-memory-wait-timeout-s")
            && !nsys_command.contains("--gpu-memory-wait-poll-s")
            && ncu_command.contains("--check-gpu-memory")
            && ncu_command.contains(&format!("--min-gpu-free-mib {min_gpu_free_mib}"))
            && ncu_command.contains("--nvidia-smi-command")
            && !ncu_command.contains("--gpu-memory-wait-timeout-s")
            && !ncu_command.contains("--gpu-memory-wait-poll-s"),
        "profile command output should preserve supported GPU memory preflight flags without unsupported wait flags: {stdout}"
    );
    assert!(
        nsys_command.contains("--skip-nsys-export")
            && !nsys_command.contains("--summarize")
            && nsys_command.contains("--proof-timing-summary")
            && nsys_command.contains("--require-proof-timing-summary"),
        "nsys command should avoid the downstream-rejected summarize plus skip-export combination while still requiring timing evidence: {stdout}"
    );
    assert!(
        ncu_command.contains("--summarize")
            && ncu_command.contains("--require-proof-timing-summary")
            && !ncu_command.contains("--skip-nsys-export"),
        "ncu command should still request summary files without nsys-only flags: {stdout}"
    );
    assert!(
        stdout.contains("--profile-arg=--kernel-name-base=demangled")
            && stdout.contains("--profile-arg=--launch-skip=1")
            && !stdout.contains("--profile-arg --kernel-name-base=demangled")
            && !stdout.contains("--profile-arg --launch-skip=1"),
        "profile command output should pass profiler-specific args through: {stdout}"
    );
    assert!(
        stdout.contains("--output-dir")
            && stdout.contains("profiles/small-stream-pipeline/nsys")
            && stdout.contains("profiles/small-stream-pipeline/ncu")
            && stdout.contains("--name small-stream-pipeline"),
        "profile command output should isolate profile artifacts by suite, mode, and tool: {stdout}"
    );
    assert!(
        stdout.contains("sh -lc")
            && stdout.contains("prove witness --guest-pc-trace 120000000 --timings")
            && stdout.contains("--gpu-preallocate")
            && stdout.contains("--minimal-memory")
            && stdout.contains("--no-pack-trace")
            && stdout.contains("--gpu-streams 7")
            && stdout.contains("--witness-thread-pools 5")
            && stdout.contains("--stored-witnesses 3")
            && stdout.contains("verify proof --eth-block-input")
            && stdout.contains("&& env -u LZVM_GUEST_PC_TRACE_PARALLEL_LOWER")
            && stdout.contains("LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_WORKERS=6")
            && stdout.contains("LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_JOB_QUEUE=12")
            && stdout.contains("LZVM_GUEST_PC_TRACE_SEGMENT_COMMIT_WORKERS=3")
            && stdout.contains(" TMPDIR=")
            && stdout.contains("verify_proof_status=ok"),
        "profile command should wrap the same prove-then-verify shell command with managed worker overrides and TMPDIR: {stdout}"
    );
    let profile_verify_command = nsys_command
        .split(" && env ")
        .nth(1)
        .expect("profile command should include verification");
    assert!(
        !profile_verify_command.contains("--gpu-preallocate")
            && !profile_verify_command.contains("--minimal-memory")
            && !profile_verify_command.contains("--no-pack-trace")
            && !profile_verify_command.contains("--gpu-streams")
            && !profile_verify_command.contains("--witness-thread-pools")
            && !profile_verify_command.contains("--stored-witnesses"),
        "profile verify command should not receive proof-only tuning flags: {stdout}"
    );
    assert!(
        stdout.contains("small-profile.proof")
            && stdout.contains("TMPDIR=")
            && !stdout.contains("{batch_dir}")
            && !stdout.contains("{tmp_dir}")
            && !stdout.contains("{run_padded}"),
        "profile command should expand timing batch tokens to concrete temp paths: {stdout}"
    );
    assert!(
        !profile_created,
        "printing profile commands should not create profile output directories"
    );
}

#[test]
fn eth_proof_timing_batch_profile_commands_preserve_env_profiler_commands() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-profile-env-tools");
    let profile_dir = fixture.dir.join("profiles");
    let nsys_path = fixture.dir.join("env nsys");
    let ncu_path = fixture.dir.join("env ncu");
    let nsys_rel = nsys_path
        .strip_prefix(workspace_root())
        .expect("nsys path should be under workspace")
        .display()
        .to_string();
    let ncu_rel = ncu_path
        .strip_prefix(workspace_root())
        .expect("ncu path should be under workspace")
        .display()
        .to_string();
    let mut command = Command::new(script_path());
    command
        .arg("--suite")
        .arg("small")
        .arg("--profile-tool")
        .arg("both")
        .arg("--profile-output-dir")
        .arg(&profile_dir)
        .arg("--print-profile-commands")
        .env("LZVM_NSYS_COMMAND", &nsys_path)
        .env("LZVM_NCU_COMMAND", &ncu_path)
        .env("CUDA_VISIBLE_DEVICES", "1,0");
    fixture.apply_env(&mut command, SMALL_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch profile command should print");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let nsys_command = stdout
        .lines()
        .find(|line| line.starts_with("small_nsys_profile_command="))
        .expect("nsys profile command should be printed");
    let ncu_command = stdout
        .lines()
        .find(|line| line.starts_with("small_ncu_profile_command="))
        .expect("ncu profile command should be printed");
    let profile_created = profile_dir.exists();
    fixture.cleanup();

    assert!(
        success,
        "profile command output should preserve env profiler commands: stderr={stderr}"
    );
    assert!(
        stdout.contains(&format!("--nsys-command '{nsys_rel}'"))
            && stdout.contains(&format!("--ncu-command '{ncu_rel}'")),
        "profile command output should embed profiler commands selected from env: {stdout}"
    );
    assert!(
        nsys_command.starts_with(
            "small_nsys_profile_command=env CUDA_VISIBLE_DEVICES=1,0 scripts/run-proof-profile.py"
        ) && ncu_command.starts_with(
            "small_ncu_profile_command=env CUDA_VISIBLE_DEVICES=1,0 scripts/run-proof-profile.py"
        ),
        "profile command output should preserve explicit GPU selection in copied commands: {stdout}"
    );
    assert!(
        !stdout.contains("LZVM_NSYS_COMMAND") && !stdout.contains("LZVM_NCU_COMMAND"),
        "profile command output should not depend on ambient profiler env names: {stdout}"
    );
    assert!(
        !profile_created,
        "printing profile commands should not create profile output directories"
    );
}

#[test]
fn eth_proof_timing_batch_profile_commands_ignore_empty_cuda_visible_devices() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-profile-empty-cuda-visible-devices");
    let profile_dir = fixture.dir.join("profiles");
    let mut command = Command::new(script_path());
    command
        .arg("--suite")
        .arg("small")
        .arg("--profile-tool")
        .arg("both")
        .arg("--profile-output-dir")
        .arg(&profile_dir)
        .arg("--print-profile-commands")
        .env("CUDA_VISIBLE_DEVICES", "");
    fixture.apply_env(&mut command, SMALL_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch profile command should print");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let nsys_command = stdout
        .lines()
        .find(|line| line.starts_with("small_nsys_profile_command="))
        .expect("nsys profile command should be printed");
    let ncu_command = stdout
        .lines()
        .find(|line| line.starts_with("small_ncu_profile_command="))
        .expect("ncu profile command should be printed");
    let profile_created = profile_dir.exists();
    fixture.cleanup();

    assert!(
        success,
        "profile command output should ignore an empty GPU selector: stderr={stderr}"
    );
    assert!(
        nsys_command.starts_with("small_nsys_profile_command=scripts/run-proof-profile.py")
            && ncu_command.starts_with("small_ncu_profile_command=scripts/run-proof-profile.py")
            && !stdout.contains("CUDA_VISIBLE_DEVICES="),
        "profile command output should not emit an empty GPU selector: {stdout}"
    );
    assert!(
        !profile_created,
        "printing profile commands should not create profile output directories"
    );
}

#[test]
fn eth_proof_timing_batch_profile_commands_skip_verify_when_requested() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-profile-skip-verify");
    let mut command = Command::new(script_path());
    command
        .arg("--suite")
        .arg("small")
        .arg("--profile-tool")
        .arg("both")
        .arg("--parallel-lower-workers")
        .arg("6")
        .arg("--parallel-lower-job-queue")
        .arg("12")
        .arg("--segment-commit-workers")
        .arg("3")
        .arg("--profile-arg=--kernel-name-base=demangled")
        .arg("--profile-arg=--launch-skip=1")
        .arg("--skip-verify-proof")
        .arg("--print-profile-commands");
    fixture.apply_env(&mut command, SMALL_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch profile command should print");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let nsys_command = stdout
        .lines()
        .find(|line| line.starts_with("small_nsys_profile_command="))
        .expect("nsys profile command should be printed");
    let ncu_command = stdout
        .lines()
        .find(|line| line.starts_with("small_ncu_profile_command="))
        .expect("ncu profile command should be printed");
    let nsys_command_text = nsys_command
        .strip_prefix("small_nsys_profile_command=")
        .expect("nsys profile command should have the expected key");
    let ncu_command_text = ncu_command
        .strip_prefix("small_ncu_profile_command=")
        .expect("ncu profile command should have the expected key");
    let nsys_dry_run = profile_command_dry_run(nsys_command_text);
    let ncu_dry_run = profile_command_dry_run(ncu_command_text);
    fixture.cleanup();

    assert!(
        success,
        "profile command output should support skip verify: stderr={stderr}"
    );
    assert!(
        stdout.contains("small_nsys_profile_command=")
            && stdout.contains("small_ncu_profile_command="),
        "profile commands should still be printed: {stdout}"
    );
    assert!(
        stdout.contains("LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_WORKERS=6")
            && stdout.contains("LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_JOB_QUEUE=12")
            && stdout.contains("LZVM_GUEST_PC_TRACE_SEGMENT_COMMIT_WORKERS=3"),
        "skip verify profile commands should preserve worker overrides: {stdout}"
    );
    assert!(
        stdout.contains("--profile-arg=--kernel-name-base=demangled")
            && stdout.contains("--profile-arg=--launch-skip=1")
            && !stdout.contains("--profile-arg --kernel-name-base=demangled")
            && !stdout.contains("--profile-arg --launch-skip=1"),
        "skip verify profile commands should emit copy-paste-safe profiler args: {stdout}"
    );
    assert!(
        nsys_dry_run.contains("--kernel-name-base=demangled")
            && nsys_dry_run.contains("--launch-skip=1")
            && ncu_dry_run.contains("--kernel-name-base=demangled")
            && ncu_dry_run.contains("--launch-skip=1"),
        "generated profile commands should parse profiler args under dry-run: nsys={nsys_dry_run} ncu={ncu_dry_run}"
    );
    assert!(
        nsys_dry_run.contains("LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_WORKERS=6")
            && nsys_dry_run.contains("LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_JOB_QUEUE=12")
            && nsys_dry_run.contains("LZVM_GUEST_PC_TRACE_SEGMENT_COMMIT_WORKERS=3")
            && ncu_dry_run.contains("LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_WORKERS=6")
            && ncu_dry_run.contains("LZVM_GUEST_PC_TRACE_PARALLEL_LOWER_JOB_QUEUE=12")
            && ncu_dry_run.contains("LZVM_GUEST_PC_TRACE_SEGMENT_COMMIT_WORKERS=3"),
        "generated profile commands should preserve worker overrides under dry-run: nsys={nsys_dry_run} ncu={ncu_dry_run}"
    );
    assert!(
        !stdout.contains("verify proof --eth-block-input")
            && !stdout.contains("verify_proof_status=ok"),
        "skip verify profile command should omit external verification: {stdout}"
    );
}

#[test]
fn eth_proof_timing_batch_rejects_profile_output_dir_outside_temp() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-profile-outside");
    let outside_profile_dir = workspace_root().join(format!(
        "eth-proof-timing-batch-profile-outside-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&outside_profile_dir);
    let mut command = Command::new(script_path());
    command
        .arg("--suite")
        .arg("small")
        .arg("--profile-output-dir")
        .arg(&outside_profile_dir)
        .arg("--print-profile-commands");
    fixture.apply_env(&mut command, SMALL_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch profile command should reject outside temp");
    let success = output.status.success();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let outside_created = outside_profile_dir.exists();
    let _ = std::fs::remove_dir_all(&outside_profile_dir);
    fixture.cleanup();

    assert!(
        !success,
        "profile commands should reject output dirs outside temp"
    );
    assert!(
        stdout.is_empty(),
        "failed profile command output should not print commands: {stdout}"
    );
    assert!(
        stderr.contains("--profile-output-dir must be under"),
        "profile output dir rejection should explain the temp boundary: stderr={stderr}"
    );
    assert!(
        !outside_created,
        "rejected profile output dir should not be created outside temp"
    );
}

#[test]
fn eth_proof_timing_batch_check_env_rejects_profile_output_dir_outside_temp() {
    let outside_profile_dir = workspace_root().join(format!(
        "eth-proof-timing-batch-check-env-profile-outside-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&outside_profile_dir);
    let mut command = Command::new(script_path());
    command
        .arg("--suite")
        .arg("small")
        .arg("--check-env")
        .arg("--profile-output-dir")
        .arg(&outside_profile_dir);

    let output = command
        .output()
        .expect("ETH proof timing batch env check should reject outside profile dir");
    let success = output.status.success();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let outside_created = outside_profile_dir.exists();
    let _ = std::fs::remove_dir_all(&outside_profile_dir);

    assert!(
        !success,
        "env check should reject profile dirs outside temp"
    );
    assert!(
        stdout.is_empty(),
        "failed profile env check should not print partial diagnostics: {stdout}"
    );
    assert!(
        stderr.contains("--profile-output-dir must be under"),
        "profile output dir rejection should explain the temp boundary: stderr={stderr}"
    );
    assert!(
        !outside_created,
        "rejected profile output dir should not be created outside temp"
    );
}

#[test]
fn eth_proof_timing_batch_rejects_env_template_command_paths_outside_temp() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-env-template-command-paths");
    let work_template_path = fixture.dir.join("bad-work.env");
    let log_template_path = fixture.dir.join("bad-log.env");
    let outside_work_dir = workspace_root().join(format!(
        "eth-proof-timing-batch-work-outside-{}",
        std::process::id()
    ));
    let outside_log_path = workspace_root().join(format!(
        "eth-proof-timing-batch-log-outside-{}.csv",
        std::process::id()
    ));

    let mut work_command = Command::new(script_path());
    work_command
        .arg("--suite")
        .arg("small")
        .arg("--work-dir")
        .arg(&outside_work_dir)
        .arg("--write-env-template")
        .arg(&work_template_path);
    let work_output = work_command
        .output()
        .expect("ETH proof timing batch env template should reject outside work dir");

    let mut log_command = Command::new(script_path());
    log_command
        .arg("--suite")
        .arg("small")
        .arg("--path")
        .arg(&outside_log_path)
        .arg("--write-env-template")
        .arg(&log_template_path);
    let log_output = log_command
        .output()
        .expect("ETH proof timing batch env template should reject outside log path");

    let work_success = work_output.status.success();
    let work_stdout = String::from_utf8(work_output.stdout).expect("stdout should be utf-8");
    let work_stderr = String::from_utf8_lossy(&work_output.stderr).into_owned();
    let log_success = log_output.status.success();
    let log_stdout = String::from_utf8(log_output.stdout).expect("stdout should be utf-8");
    let log_stderr = String::from_utf8_lossy(&log_output.stderr).into_owned();
    let work_template_created = work_template_path.exists();
    let log_template_created = log_template_path.exists();
    let outside_work_created = outside_work_dir.exists();
    let outside_log_created = outside_log_path.exists();
    let _ = std::fs::remove_dir_all(&outside_work_dir);
    let _ = std::fs::remove_file(&outside_log_path);
    fixture.cleanup();

    assert!(
        !work_success,
        "env template should reject work dirs outside temp"
    );
    assert!(
        work_stdout.is_empty(),
        "failed work-dir template write should not report commands: {work_stdout}"
    );
    assert!(
        work_stderr.contains("--work-dir must be under"),
        "env template should explain the work-dir constraint: stderr={work_stderr}"
    );
    assert!(
        !work_template_created,
        "rejected work-dir template should not be created"
    );
    assert!(
        !outside_work_created,
        "rejected work dir should not be created outside temp"
    );
    assert!(
        !log_success,
        "env template should reject improve log paths outside temp"
    );
    assert!(
        log_stdout.is_empty(),
        "failed log-path template write should not report commands: {log_stdout}"
    );
    assert!(
        log_stderr.contains("--path must be under"),
        "env template should explain the log path constraint: stderr={log_stderr}"
    );
    assert!(
        !log_template_created,
        "rejected log-path template should not be created"
    );
    assert!(
        !outside_log_created,
        "rejected log path should not be created outside temp"
    );
}

#[test]
fn eth_proof_timing_batch_rejects_env_template_outside_temp() {
    let template_path = workspace_root().join(format!(
        "eth-proof-timing-batch-env-template-{}.env",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&template_path);
    let mut command = Command::new(script_path());
    command
        .arg("--suite")
        .arg("small")
        .arg("--write-env-template")
        .arg(&template_path);

    let output = command
        .output()
        .expect("ETH proof timing batch env template should reject outside temp");
    let success = output.status.success();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let template_created = template_path.exists();
    let _ = std::fs::remove_file(&template_path);

    assert!(!success, "env template should reject paths outside temp");
    assert!(
        stdout.is_empty(),
        "failed env template write should not report commands: {stdout}"
    );
    assert!(
        stderr.contains("--write-env-template must be under"),
        "env template should explain the path constraint: stderr={stderr}"
    );
    assert!(
        !template_created,
        "rejected env template path should not be created"
    );
}

#[cfg(unix)]
#[test]
fn eth_proof_timing_batch_rejects_symlinked_env_template() {
    use std::os::unix::fs::symlink;

    let fixture = ProofFixture::new("eth-proof-timing-batch-env-template-symlink");
    let template_path = fixture.dir.join("template.env");
    let redirected = fixture.dir.join("redirected.env");
    std::fs::write(&redirected, "sentinel\n").expect("redirect target should write");
    symlink(&redirected, &template_path).expect("template symlink fixture should be created");
    let mut command = Command::new(script_path());
    command
        .arg("--suite")
        .arg("small")
        .arg("--write-env-template")
        .arg(&template_path);

    let output = command
        .output()
        .expect("ETH proof timing batch env template should reject symlink");
    let success = output.status.success();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let redirected_text =
        std::fs::read_to_string(&redirected).expect("redirect target should remain readable");
    fixture.cleanup();

    assert!(!success, "env template should reject symlinked paths");
    assert!(
        stdout.is_empty(),
        "failed env template write should not report commands: {stdout}"
    );
    assert!(
        stderr.contains("--write-env-template must not be a symlink"),
        "env template symlink rejection should explain the path constraint: stderr={stderr}"
    );
    assert_eq!(
        redirected_text, "sentinel\n",
        "rejected env template should not overwrite a symlink target"
    );
}

#[test]
fn eth_proof_timing_batch_check_env_rejects_missing_config() {
    let mut command = Command::new(script_path());
    command.arg("--suite").arg("small").arg("--check-env");
    clear_env(&mut command, SMALL_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch env check should run");
    let success = output.status.success();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    assert!(
        !success,
        "env check should fail when required config is missing"
    );
    assert!(
        !stdout.contains("status=ok"),
        "failed env check should not report ok: {stdout}"
    );
    assert!(
        stderr.contains("proof environment is incomplete"),
        "env check should explain missing configuration: stderr={stderr}"
    );
    assert_artifact_help_stderr(&stderr);
    assert!(
        stderr.contains("next_env_template_command=scripts/run-eth-proof-timing-batch.py --suite small")
            && stderr.contains("--write-env-template temp/real-proof.env"),
        "env check should report the env-template command needed to configure real inputs: stderr={stderr}"
    );
}

#[test]
fn eth_proof_timing_batch_incomplete_env_file_suggests_sibling_template_path() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-incomplete-env-file");
    let env_path = fixture.dir.join("real-proof.env.next");
    let template_path = fixture.dir.join("real-proof.env.template.next");
    std::fs::write(
        &env_path,
        format!(
            "export {SMALL_PREFIX}_SETUP=\nexport {SMALL_PREFIX}_BLOCK_INPUT=\nexport {SMALL_PREFIX}_PROGRAM_IMAGE_CACHE=\nexport {SMALL_PREFIX}_INPUT_DATA=\nexport {SMALL_PREFIX}_GUEST_IMAGE=\n"
        ),
    )
    .expect("incomplete env file should write");
    let env_rel = env_path
        .strip_prefix(workspace_root())
        .expect("env path should be under workspace")
        .display()
        .to_string();
    let template_rel = template_path
        .strip_prefix(workspace_root())
        .expect("template path should be under workspace")
        .display()
        .to_string();
    let mut command = Command::new(script_path());
    command
        .arg("--suite")
        .arg("small")
        .arg("--check-env")
        .arg("--env-file")
        .arg(&env_path);
    clear_env(&mut command, SMALL_PREFIX);
    clear_env(&mut command, LARGE_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch incomplete env-file check should run");
    let success = output.status.success();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    fixture.cleanup();

    assert!(!success, "incomplete env file should fail");
    assert!(
        !stdout.contains("status=ok"),
        "failed env-file check should not report ok: {stdout}"
    );
    assert!(
        stderr.contains("proof environment is incomplete"),
        "env-file check should explain missing configuration: stderr={stderr}"
    );
    assert_artifact_help_stderr(&stderr);
    assert!(
        stderr.contains(&format!("--env-file {env_rel}"))
            && stderr.contains(&format!("--write-env-template {template_rel}"))
            && !stderr.contains(&format!("--write-env-template {env_rel}"))
            && !stderr.contains("--write-env-template temp/real-proof.env"),
        "env-file check should suggest a sibling template path without overwriting the checked env file: stderr={stderr}"
    );
}

#[test]
fn eth_proof_timing_batch_check_env_reports_template_command_when_no_env_is_available() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-check-env-no-env");
    let profile_dir = fixture.dir.join("profiles");
    let nsys_path = write_fixture(&fixture.dir, "env nsys");
    let ncu_path = write_fixture(&fixture.dir, "env ncu");
    make_executable(&nsys_path);
    make_executable(&ncu_path);
    let profile_rel = profile_dir
        .strip_prefix(workspace_root())
        .expect("profile path should be under workspace")
        .display()
        .to_string();
    let nsys_rel = nsys_path
        .strip_prefix(workspace_root())
        .expect("nsys path should be under workspace")
        .display()
        .to_string();
    let ncu_rel = ncu_path
        .strip_prefix(workspace_root())
        .expect("ncu path should be under workspace")
        .display()
        .to_string();
    let template_path = workspace_root().join("temp/real-proof.env");
    let _ = std::fs::remove_file(&template_path);
    let mut command = Command::new(script_path());
    command
        .arg("--suite")
        .arg("available")
        .arg("--check-env")
        .arg("--profile-tool")
        .arg("both")
        .arg("--profile-output-dir")
        .arg(&profile_dir)
        .arg("--nsys-command")
        .arg(&nsys_path)
        .arg("--ncu-command")
        .arg(&ncu_path);
    clear_env(&mut command, SMALL_PREFIX);
    clear_env(&mut command, LARGE_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch env check should run");
    let success = output.status.success();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let template_created = template_path.exists();
    fixture.cleanup();

    assert!(
        !success,
        "env check should fail when no proof env is available"
    );
    assert!(
        stdout.contains("profile_tool=both\n")
            && stdout.contains("nsys_profiler_status=ready\n")
            && stdout.contains("ncu_profiler_status=ready\n")
            && !stdout.contains("status=ok\n"),
        "failed env check should still report profiler readiness without proof readiness: {stdout}"
    );
    assert!(
        stderr.contains("no proof environments available; missing"),
        "env check should explain that no proof env is available: stderr={stderr}"
    );
    assert_artifact_help_stderr(&stderr);
    assert!(
        stderr.contains(
            "next_env_template_command=scripts/run-eth-proof-timing-batch.py --suite available"
        ) && stderr.contains("--profile-tool both")
            && stderr.contains(&format!("--profile-output-dir {profile_rel}"))
            && stderr.contains(&format!("--nsys-command '{nsys_rel}'"))
            && stderr.contains(&format!("--ncu-command '{ncu_rel}'"))
            && stderr.contains("--write-env-template temp/real-proof.env"),
        "env check should preserve profiling options in the env-template command: stderr={stderr}"
    );
    assert!(
        !template_created,
        "check-env should not create the suggested env template"
    );
}

#[test]
fn eth_proof_timing_batch_no_available_env_file_suggests_sibling_template_path() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-no-available-env-file");
    let env_path = fixture.dir.join("real-proof.resolved.env");
    let template_path = fixture.dir.join("real-proof.resolved.template.env");
    std::fs::write(
        &env_path,
        format!(
            "export {SMALL_PREFIX}_TRACE_LIMIT=120000000\nexport {LARGE_PREFIX}_TRACE_LIMIT=600000000\n"
        ),
    )
    .expect("env file should write");
    let env_rel = env_path
        .strip_prefix(workspace_root())
        .expect("env path should be under workspace")
        .display()
        .to_string();
    let template_rel = template_path
        .strip_prefix(workspace_root())
        .expect("template path should be under workspace")
        .display()
        .to_string();
    let mut command = Command::new(script_path());
    command
        .arg("--suite")
        .arg("available")
        .arg("--check-env")
        .arg("--env-file")
        .arg(&env_path)
        .arg("--skip-targets");
    clear_env(&mut command, SMALL_PREFIX);
    clear_env(&mut command, LARGE_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch env-file check should run");
    let success = output.status.success();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let template_created = template_path.exists();
    fixture.cleanup();

    assert!(
        !success,
        "env-file check should fail when no proof env is available"
    );
    assert!(
        !stdout.contains("status=ok\n"),
        "failed env-file check should not report proof readiness: {stdout}"
    );
    assert!(
        stderr.contains("no proof environments available; missing"),
        "env-file check should explain that no proof env is available: stderr={stderr}"
    );
    assert_artifact_help_stderr(&stderr);
    assert!(
        stderr.contains(&format!("--env-file {env_rel}"))
            && stderr.contains(&format!("--write-env-template {template_rel}"))
            && !stderr.contains(&format!("--write-env-template {env_rel}"))
            && !stderr.contains("--write-env-template temp/real-proof.env"),
        "env-file check should suggest a sibling template path without overwriting the checked env file: stderr={stderr}"
    );
    assert!(
        !template_created,
        "check-env should not create the suggested env template"
    );
}

#[test]
fn eth_proof_timing_batch_check_env_rejects_missing_bin() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-missing-bin");
    let mut command = Command::new(script_path());
    command.arg("--suite").arg("small").arg("--check-env");
    fixture.apply_env(&mut command, SMALL_PREFIX);
    command.env(
        format!("{SMALL_PREFIX}_BIN"),
        fixture.dir.join("missing-lzvm"),
    );

    let output = command
        .output()
        .expect("ETH proof timing batch env check should run");
    let success = output.status.success();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    fixture.cleanup();

    assert!(!success, "env check should fail when bin is missing");
    assert!(
        !stdout.contains("status=ok"),
        "failed env check should not report ok: {stdout}"
    );
    assert!(
        stderr.contains("path does not exist"),
        "env check should explain the missing bin: stderr={stderr}"
    );
}

#[test]
fn eth_proof_timing_batch_check_env_rejects_bad_bin_types() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-bad-bin");

    let mut dir_command = Command::new(script_path());
    dir_command.arg("--suite").arg("small").arg("--check-env");
    fixture.apply_env(&mut dir_command, SMALL_PREFIX);
    dir_command.env(format!("{SMALL_PREFIX}_BIN"), &fixture.setup);
    let dir_output = dir_command
        .output()
        .expect("ETH proof timing batch env check should run");

    let plain_bin = write_fixture(&fixture.dir, "plain-lzvm");
    let mut plain_command = Command::new(script_path());
    plain_command.arg("--suite").arg("small").arg("--check-env");
    fixture.apply_env(&mut plain_command, SMALL_PREFIX);
    plain_command.env(format!("{SMALL_PREFIX}_BIN"), &plain_bin);
    let plain_output = plain_command
        .output()
        .expect("ETH proof timing batch env check should run");

    let dir_success = dir_output.status.success();
    let dir_stdout = String::from_utf8(dir_output.stdout).expect("stdout should be utf-8");
    let dir_stderr = String::from_utf8_lossy(&dir_output.stderr).into_owned();
    let plain_success = plain_output.status.success();
    let plain_stdout = String::from_utf8(plain_output.stdout).expect("stdout should be utf-8");
    let plain_stderr = String::from_utf8_lossy(&plain_output.stderr).into_owned();
    fixture.cleanup();

    assert!(!dir_success, "env check should reject directory bin paths");
    assert!(
        !dir_stdout.contains("status=ok"),
        "failed env check should not report ok: {dir_stdout}"
    );
    assert!(
        dir_stderr.contains("_BIN must be a file"),
        "env check should explain bin path type: stderr={dir_stderr}"
    );
    assert!(
        !plain_success,
        "env check should reject non-executable bin paths"
    );
    assert!(
        !plain_stdout.contains("status=ok"),
        "failed env check should not report ok: {plain_stdout}"
    );
    assert!(
        plain_stderr.contains("_BIN must be executable"),
        "env check should explain bin executability: stderr={plain_stderr}"
    );
}

#[test]
fn eth_proof_timing_batch_check_env_rejects_old_bin_by_default() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-old-bin");
    age_file_to_unix_epoch(&fixture.fake_bin);
    let mut command = Command::new(script_path());
    command.arg("--suite").arg("small").arg("--check-env");
    fixture.apply_env(&mut command, SMALL_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch env check should run");
    let success = output.status.success();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    fixture.cleanup();

    assert!(!success, "env check should reject an old proof binary");
    assert!(
        !stdout.contains("status=ok"),
        "failed env check should not report ok: {stdout}"
    );
    assert!(
        stderr.contains("freshness check failed") && stderr.contains("--allow-stale-bin"),
        "env check should explain the old binary and opt-out: stderr={stderr}"
    );
}

#[test]
fn eth_proof_timing_batch_check_env_allows_old_bin_with_explicit_flag() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-allow-old-bin");
    age_file_to_unix_epoch(&fixture.fake_bin);
    let mut command = Command::new(script_path());
    command
        .arg("--suite")
        .arg("small")
        .arg("--check-env")
        .arg("--allow-stale-bin");
    fixture.apply_env(&mut command, SMALL_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch env check should run");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    fixture.cleanup();

    assert!(
        success,
        "env check should allow old proof binary with explicit flag: stderr={stderr}"
    );
    assert!(stdout.contains("status=ok\n"), "{stdout}");
    assert!(
        stdout.contains("--allow-stale-bin"),
        "follow-up commands should preserve old-binary opt-out: {stdout}"
    );
}

#[test]
fn eth_proof_timing_batch_check_env_rejects_bad_trace_limits() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-bad-trace-limit");

    let mut text_command = Command::new(script_path());
    text_command.arg("--suite").arg("small").arg("--check-env");
    fixture.apply_env(&mut text_command, SMALL_PREFIX);
    text_command.env(format!("{SMALL_PREFIX}_TRACE_LIMIT"), "not-a-number");
    let text_output = text_command
        .output()
        .expect("ETH proof timing batch env check should run");

    let mut zero_command = Command::new(script_path());
    zero_command.arg("--suite").arg("small").arg("--check-env");
    fixture.apply_env(&mut zero_command, SMALL_PREFIX);
    zero_command.env(format!("{SMALL_PREFIX}_TRACE_LIMIT"), "0");
    let zero_output = zero_command
        .output()
        .expect("ETH proof timing batch env check should run");

    let text_success = text_output.status.success();
    let text_stdout = String::from_utf8(text_output.stdout).expect("stdout should be utf-8");
    let text_stderr = String::from_utf8_lossy(&text_output.stderr).into_owned();
    let zero_success = zero_output.status.success();
    let zero_stdout = String::from_utf8(zero_output.stdout).expect("stdout should be utf-8");
    let zero_stderr = String::from_utf8_lossy(&zero_output.stderr).into_owned();
    fixture.cleanup();

    assert!(!text_success, "env check should reject text trace limits");
    assert!(
        !text_stdout.contains("status=ok"),
        "failed env check should not report ok: {text_stdout}"
    );
    assert!(
        text_stderr.contains("_TRACE_LIMIT must be a positive integer"),
        "env check should explain text trace limit: stderr={text_stderr}"
    );
    assert!(!zero_success, "env check should reject zero trace limits");
    assert!(
        !zero_stdout.contains("status=ok"),
        "failed env check should not report ok: {zero_stdout}"
    );
    assert!(
        zero_stderr.contains("_TRACE_LIMIT must be a positive integer"),
        "env check should explain zero trace limit: stderr={zero_stderr}"
    );
}

#[test]
fn eth_proof_timing_batch_check_env_rejects_wrong_input_types() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-input-types");

    let mut setup_command = Command::new(script_path());
    setup_command.arg("--suite").arg("small").arg("--check-env");
    fixture.apply_env(&mut setup_command, SMALL_PREFIX);
    setup_command.env(format!("{SMALL_PREFIX}_SETUP"), &fixture.block_input);
    let setup_output = setup_command
        .output()
        .expect("ETH proof timing batch env check should run");

    let mut block_command = Command::new(script_path());
    block_command.arg("--suite").arg("small").arg("--check-env");
    fixture.apply_env(&mut block_command, SMALL_PREFIX);
    block_command.env(format!("{SMALL_PREFIX}_BLOCK_INPUT"), &fixture.setup);
    let block_output = block_command
        .output()
        .expect("ETH proof timing batch env check should run");

    let setup_success = setup_output.status.success();
    let setup_stdout = String::from_utf8(setup_output.stdout).expect("stdout should be utf-8");
    let setup_stderr = String::from_utf8_lossy(&setup_output.stderr).into_owned();
    let block_success = block_output.status.success();
    let block_stdout = String::from_utf8(block_output.stdout).expect("stdout should be utf-8");
    let block_stderr = String::from_utf8_lossy(&block_output.stderr).into_owned();
    fixture.cleanup();

    assert!(!setup_success, "env check should reject file setup paths");
    assert!(
        !setup_stdout.contains("status=ok"),
        "failed env check should not report ok: {setup_stdout}"
    );
    assert!(
        setup_stderr.contains("_SETUP must be a directory"),
        "env check should explain setup path type: stderr={setup_stderr}"
    );
    assert!(
        !block_success,
        "env check should reject directory input paths"
    );
    assert!(
        !block_stdout.contains("status=ok"),
        "failed env check should not report ok: {block_stdout}"
    );
    assert!(
        block_stderr.contains("_BLOCK_INPUT must be a file"),
        "env check should explain input path type: stderr={block_stderr}"
    );
}

#[test]
fn eth_proof_timing_batch_check_env_rejects_malformed_block_input() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-malformed-block-input");
    std::fs::write(&fixture.block_input, b"malformed fixture")
        .expect("block fixture should update");
    std::fs::write(
        &fixture.fake_bin,
        r#"#!/usr/bin/env python3
import pathlib
import sys
args = sys.argv[1:]
if args[:2] == ["eth", "block-input-summary"]:
    if pathlib.Path(args[-1]).read_bytes() == b"malformed fixture":
        sys.stderr.write("canonical block parser rejected malformed input\n")
        sys.exit(1)
    print("status=ok")
    sys.exit(0)
if args[:2] == ["setup", "program-image-cache-summary"]:
    print("status=ok")
    sys.exit(0)
sys.exit(0)
"#,
    )
    .expect("malformed block fake should write");
    make_executable(&fixture.fake_bin);
    let mut command = Command::new(script_path());
    command.arg("--suite").arg("small").arg("--check-env");
    fixture.apply_env(&mut command, SMALL_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch env check should run");
    let success = output.status.success();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    fixture.cleanup();

    assert!(!success, "env check should reject malformed block input");
    assert!(
        !stdout.contains("status=ok"),
        "failed env check should not report ok: {stdout}"
    );
    assert!(
        stderr.contains("_BLOCK_INPUT artifact is invalid: semantic summary failed")
            && stderr.contains("canonical block parser rejected malformed input"),
        "env check should explain malformed block input: stderr={stderr}"
    );
}

#[test]
fn eth_proof_timing_batch_check_env_rejects_malformed_program_image_cache() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-malformed-program-cache");
    std::fs::write(&fixture.cache, b"malformed fixture").expect("cache fixture should update");
    std::fs::write(
        &fixture.fake_bin,
        r#"#!/usr/bin/env python3
import pathlib
import sys
args = sys.argv[1:]
if args[:2] == ["eth", "block-input-summary"]:
    print("status=ok")
    sys.exit(0)
if args[:2] == ["setup", "program-image-cache-summary"]:
    if pathlib.Path(args[-1]).read_bytes() == b"malformed fixture":
        sys.stderr.write("canonical cache parser rejected malformed input\n")
        sys.exit(1)
    print("status=ok")
    sys.exit(0)
sys.exit(0)
"#,
    )
    .expect("malformed cache fake should write");
    make_executable(&fixture.fake_bin);
    let mut command = Command::new(script_path());
    command.arg("--suite").arg("small").arg("--check-env");
    fixture.apply_env(&mut command, SMALL_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch env check should run");
    let success = output.status.success();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    fixture.cleanup();

    assert!(
        !success,
        "env check should reject malformed program-image cache"
    );
    assert!(
        !stdout.contains("status=ok"),
        "failed env check should not report ok: {stdout}"
    );
    assert!(
        stderr.contains("_PROGRAM_IMAGE_CACHE artifact is invalid: semantic summary failed")
            && stderr.contains("canonical cache parser rejected malformed input"),
        "env check should explain malformed program-image cache: stderr={stderr}"
    );
}

#[test]
fn eth_proof_timing_batch_check_env_rejects_block_input_semantic_summary_failure() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-block-summary-failure");
    std::fs::write(
        &fixture.fake_bin,
        r#"#!/usr/bin/env python3
import sys
args = sys.argv[1:]
if args[:2] == ["eth", "block-input-summary"]:
    sys.stderr.write("canonical block parse failed\n")
    sys.exit(1)
if args[:2] == ["setup", "program-image-cache-summary"]:
    print("status=ok")
    sys.exit(0)
sys.exit(0)
"#,
    )
    .expect("semantic failure fake should write");
    make_executable(&fixture.fake_bin);
    let mut command = Command::new(script_path());
    command.arg("--suite").arg("small").arg("--check-env");
    fixture.apply_env(&mut command, SMALL_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch env check should run");
    let success = output.status.success();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    fixture.cleanup();

    assert!(
        !success,
        "env check should reject block input semantic summary failures"
    );
    assert!(
        !stdout.contains("status=ok"),
        "failed env check should not report ok: {stdout}"
    );
    assert!(
        stderr.contains("_BLOCK_INPUT artifact is invalid: semantic summary failed")
            && stderr.contains("canonical block parse failed"),
        "env check should explain block semantic summary failure: stderr={stderr}"
    );
}

#[test]
fn eth_proof_timing_batch_check_env_rejects_program_cache_semantic_summary_failure() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-cache-summary-failure");
    std::fs::write(
        &fixture.fake_bin,
        r#"#!/usr/bin/env python3
import sys
args = sys.argv[1:]
if args[:2] == ["eth", "block-input-summary"]:
    print("status=ok")
    sys.exit(0)
if args[:2] == ["setup", "program-image-cache-summary"]:
    sys.stderr.write("canonical cache parse failed\n")
    sys.exit(1)
sys.exit(0)
"#,
    )
    .expect("semantic failure fake should write");
    make_executable(&fixture.fake_bin);
    let mut command = Command::new(script_path());
    command.arg("--suite").arg("small").arg("--check-env");
    fixture.apply_env(&mut command, SMALL_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch env check should run");
    let success = output.status.success();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    fixture.cleanup();

    assert!(
        !success,
        "env check should reject program cache semantic summary failures"
    );
    assert!(
        !stdout.contains("status=ok"),
        "failed env check should not report ok: {stdout}"
    );
    assert!(
        stderr.contains("_PROGRAM_IMAGE_CACHE artifact is invalid: semantic summary failed")
            && stderr.contains("canonical cache parse failed"),
        "env check should explain cache semantic summary failure: stderr={stderr}"
    );
}

#[test]
fn eth_proof_timing_batch_check_env_rejects_malformed_input_data() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-malformed-input-data");
    std::fs::write(&fixture.input_data, [1_u8, 2, 3]).expect("input fixture should update");
    let mut command = Command::new(script_path());
    command.arg("--suite").arg("small").arg("--check-env");
    fixture.apply_env(&mut command, SMALL_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch env check should run");
    let success = output.status.success();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    fixture.cleanup();

    assert!(
        !success,
        "env check should reject malformed framed input data"
    );
    assert!(
        !stdout.contains("status=ok"),
        "failed env check should not report ok: {stdout}"
    );
    assert!(
        stderr.contains("_INPUT_DATA framed input is invalid"),
        "env check should explain malformed input data: stderr={stderr}"
    );
}

#[test]
fn eth_proof_timing_batch_check_env_rejects_truncated_input_padding() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-truncated-input-padding");
    let mut input_data = Vec::new();
    input_data.extend_from_slice(&1_u64.to_le_bytes());
    input_data.push(9);
    std::fs::write(&fixture.input_data, input_data).expect("input fixture should update");
    let mut command = Command::new(script_path());
    command.arg("--suite").arg("small").arg("--check-env");
    fixture.apply_env(&mut command, SMALL_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch env check should run");
    let success = output.status.success();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    fixture.cleanup();

    assert!(
        !success,
        "env check should reject truncated framed input padding"
    );
    assert!(
        !stdout.contains("status=ok"),
        "failed env check should not report ok: {stdout}"
    );
    assert!(
        stderr.contains("_INPUT_DATA framed input is invalid: truncated chunk padding"),
        "env check should explain truncated input padding: stderr={stderr}"
    );
}

#[test]
fn eth_proof_timing_batch_check_env_rejects_empty_input_data() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-empty-input-data");
    std::fs::write(&fixture.input_data, []).expect("input fixture should update");
    let mut command = Command::new(script_path());
    command.arg("--suite").arg("small").arg("--check-env");
    fixture.apply_env(&mut command, SMALL_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch env check should run");
    let success = output.status.success();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    fixture.cleanup();

    assert!(!success, "env check should reject empty framed input data");
    assert!(
        !stdout.contains("status=ok"),
        "failed env check should not report ok: {stdout}"
    );
    assert!(
        stderr.contains("_INPUT_DATA framed input is invalid: empty input"),
        "env check should explain empty input data: stderr={stderr}"
    );
}

#[test]
fn eth_proof_timing_batch_check_env_accepts_sparse_large_input_data() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-sparse-large-input-data");
    let sparse_input = write_sparse_framed_fixture(&fixture.dir, "large-input-data.bin", 1 << 32);
    let mut command = Command::new(script_path());
    command.arg("--suite").arg("small").arg("--check-env");
    fixture.apply_env(&mut command, SMALL_PREFIX);
    command.env(format!("{SMALL_PREFIX}_INPUT_DATA"), &sparse_input);

    let output = command
        .output()
        .expect("ETH proof timing batch env check should run");
    let success = output.status.success();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let input_len = std::fs::metadata(&sparse_input)
        .expect("sparse input metadata should read")
        .len();
    fixture.cleanup();

    assert!(
        success,
        "env check should validate sparse large framed input without reading the payload: stderr={stderr}"
    );
    assert!(
        stdout.contains("status=ok\n"),
        "env check should report ready status: {stdout}"
    );
    assert!(stderr.is_empty(), "env check should not warn: {stderr}");
    assert!(
        input_len > u32::MAX as u64,
        "sparse fixture should cover payloads larger than a 32-bit byte count"
    );
}

#[test]
fn eth_proof_timing_batch_ignores_legacy_tmp_dir_env() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-legacy-tmp-env");
    let legacy_tmp = workspace_root().join(format!(
        "target/eth-proof-timing-batch-legacy-tmp-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&legacy_tmp);
    let mut command = Command::new(script_path());
    command.arg("--suite").arg("small").arg("--check-env");
    fixture.apply_env(&mut command, SMALL_PREFIX);
    command.env(format!("{SMALL_PREFIX}_TMP_DIR"), &legacy_tmp);

    let output = command
        .output()
        .expect("ETH proof timing batch env check should run");
    let success = output.status.success();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let legacy_created = legacy_tmp.exists();
    let tmp_dir_created = fixture.shared_tmp_dir.exists();
    let _ = std::fs::remove_dir_all(&legacy_tmp);
    fixture.cleanup();

    assert!(
        success,
        "legacy TMPDIR env should not affect env check: stderr={stderr}"
    );
    assert!(
        stdout.contains("status=ok\n"),
        "env check should report ready status: {stdout}"
    );
    assert!(
        !stdout.contains("small_tmp_dir="),
        "env check should not expose a shared TMPDIR: {stdout}"
    );
    assert!(stderr.is_empty(), "env check should not warn: {stderr}");
    assert!(
        !legacy_created,
        "ignored legacy TMPDIR should not be created outside temp"
    );
    assert!(
        !tmp_dir_created,
        "env check should not create a shared TMPDIR"
    );
}

#[test]
fn eth_proof_timing_batch_available_suite_uses_only_configured_large_env() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-available-large");
    let mut command = Command::new(script_path());
    command
        .arg("--suite")
        .arg("available")
        .arg("--dry-run")
        .arg("--summary")
        .arg("available");
    clear_env(&mut command, SMALL_PREFIX);
    fixture.apply_env(&mut command, LARGE_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch available dry-run should run");
    let tmp_dir_created = fixture.shared_tmp_dir.exists();
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    fixture.cleanup();

    assert!(
        success,
        "available dry-run should build a large command: stderr={stderr}"
    );
    assert!(
        !tmp_dir_created,
        "available dry-run should not create a shared TMPDIR"
    );
    assert!(
        stdout.contains("large_command=env -u LZVM_GUEST_PC_TRACE_PARALLEL_LOWER"),
        "available suite should include the configured large command: {stdout}"
    );
    assert_verify_required_text_args(&stdout, "available runner command");
    let verify_command = verify_command_tail(&stdout);
    assert!(
        verify_command.contains("verify proof --eth-block-input")
            && verify_command.contains("--program-image-cache")
            && verify_command.contains("--input-data")
            && verify_command.contains("verify_proof_status=ok"),
        "available suite should include external verification in the large command: {stdout}"
    );
    assert!(
        stdout.contains("{batch_dir}/large-{run_padded}.proof"),
        "large command should use a unique per-run output directory: {stdout}"
    );
    assert!(
        !stdout.contains("small_command="),
        "available suite should not include an unconfigured small command: {stdout}"
    );
}

fn write_fixture(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, b"fixture").expect("fixture should write");
    path
}

fn write_fake_lzvm(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    std::fs::write(
        &path,
        r#"#!/usr/bin/env python3
import pathlib
import sys

args = sys.argv[1:]
if args[:2] in (["eth", "block-input-summary"], ["setup", "program-image-cache-summary"]):
    print("status=ok")
    sys.exit(0)
if args[:2] == ["verify", "proof"]:
    print("status=ok")
    print("verify_proof_status=ok")
    print("artifact_public_input_match=ok")
    print("artifact_proof_match=ok")
    print("eth_block_input_match=ok")
    print("program_image_cache_match=ok")
    print("framed_guest_input_match=ok")
    print("pipeline_input_bindings=ok")
    sys.exit(0)
if args[:2] == ["prove", "witness"] and len(args) >= 2:
    output_dir = pathlib.Path(args[-2])
    output_dir.mkdir(parents=True, exist_ok=True)
    (output_dir / "proof.bin").write_bytes(b"proof")
    (output_dir / "eth-block-public-values.bin").write_bytes(b"public")
    print("status=ok")
    print("verify_outputs=true")
    print("timing_total_ms=1000")
    sys.exit(0)
sys.stderr.write("unsupported fake lzvm command\n")
sys.exit(2)
"#,
    )
    .expect("fake lzvm fixture should write");
    path
}

fn write_framed_fixture(dir: &std::path::Path, name: &str, payload: &[u8]) -> std::path::PathBuf {
    let path = dir.join(name);
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&(payload.len() as u64).to_le_bytes());
    bytes.extend_from_slice(payload);
    bytes.resize(bytes.len().next_multiple_of(8), 0);
    std::fs::write(&path, bytes).expect("framed fixture should write");
    path
}

fn write_sparse_framed_fixture(
    dir: &std::path::Path,
    name: &str,
    payload_len: u64,
) -> std::path::PathBuf {
    let path = dir.join(name);
    let payload_offset = 8_u64;
    let payload_end = payload_offset
        .checked_add(payload_len)
        .expect("sparse fixture length should fit");
    let padding_len = (8 - (payload_end % 8)) % 8;
    let file_len = payload_end
        .checked_add(padding_len)
        .expect("sparse fixture padded length should fit");
    let mut file = std::fs::File::create(&path).expect("sparse framed fixture should create");
    file.write_all(&payload_len.to_le_bytes())
        .expect("sparse framed fixture header should write");
    file.seek(SeekFrom::Start(file_len - 1))
        .expect("sparse framed fixture should seek to final byte");
    file.write_all(&[0])
        .expect("sparse framed fixture final byte should write");
    path
}

fn write_executable_script(dir: &std::path::Path, name: &str, source: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, source).expect("script fixture should write");
    make_executable(&path);
    path
}
