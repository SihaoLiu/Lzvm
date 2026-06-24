use std::{path::PathBuf, process::Command};

const SMALL_PREFIX: &str = "LZVM_REAL_SMALL_PARITY";
const LARGE_PREFIX: &str = "LZVM_REAL_LARGE_PARITY";
const REQUIRED_SUFFIXES: &[&str] = &[
    "SETUP",
    "BLOCK_INPUT",
    "PROGRAM_IMAGE_CACHE",
    "INPUT_DATA",
    "GUEST_IMAGE",
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

fn test_dir(name: &str) -> std::path::PathBuf {
    workspace_root().join(format!("temp/{name}-{}", std::process::id()))
}

struct ProofFixture {
    dir: PathBuf,
    fake_bin: PathBuf,
    setup: PathBuf,
    block_input: PathBuf,
    cache: PathBuf,
    input_data: PathBuf,
    guest: PathBuf,
    tmp_dir: PathBuf,
}

impl ProofFixture {
    fn new(name: &str) -> Self {
        let dir = test_dir(name);
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("fixture dir should be created");
        let fake_bin = write_fixture(&dir, "lzvm");
        let setup = dir.join("setup");
        std::fs::create_dir_all(&setup).expect("setup dir should be created");
        let block_input = write_fixture(&dir, "block.input");
        let cache = write_fixture(&dir, "program-image.cache");
        let input_data = write_fixture(&dir, "input-data.bin");
        let guest = write_fixture(&dir, "guest.elf");
        let tmp_dir = dir.join("tmp");

        Self {
            dir,
            fake_bin,
            setup,
            block_input,
            cache,
            input_data,
            guest,
            tmp_dir,
        }
    }

    fn apply_env(&self, command: &mut Command, prefix: &str) {
        command
            .env(format!("{prefix}_BIN"), &self.fake_bin)
            .env(format!("{prefix}_SETUP"), &self.setup)
            .env(format!("{prefix}_BLOCK_INPUT"), &self.block_input)
            .env(format!("{prefix}_PROGRAM_IMAGE_CACHE"), &self.cache)
            .env(format!("{prefix}_INPUT_DATA"), &self.input_data)
            .env(format!("{prefix}_GUEST_IMAGE"), &self.guest)
            .env(format!("{prefix}_TMP_DIR"), &self.tmp_dir);
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
        stdout.contains("large_runs=3"),
        "self-test should run the large command: {stdout}"
    );
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
        .arg("dry run");
    fixture.apply_env(&mut command, SMALL_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch dry-run should run");
    let tmp_dir_created = fixture.tmp_dir.exists();
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
    assert!(!tmp_dir_created, "dry-run should not create TMPDIR");
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
        stdout.contains("--require-text verify_proof_status=ok"),
        "runner command should require the external proof verify marker: {stdout}"
    );
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
    assert!(
        stdout.contains("verify proof --eth-block-input")
            && stdout.contains("{batch_dir}/small-{run_padded}.proof/proof.bin")
            && stdout.contains("{batch_dir}/small-{run_padded}.proof/eth-block-public-values.bin")
            && stdout.contains("verify_proof_status=ok"),
        "small command should run an external proof verification after proving: {stdout}"
    );
    assert!(
        stdout.contains("LZVM_GUEST_PC_TRACE_COMMIT_PIPELINE=1"),
        "combined mode should set its mode environment: {stdout}"
    );
    assert!(
        stdout.contains("TMPDIR={tmp_dir}"),
        "small command should use the per-run temp dir token: {stdout}"
    );
    assert!(
        !stdout.contains(&fixture.tmp_dir.display().to_string()),
        "dry-run command should not use the configured TMPDIR as a shared run dir: {stdout}"
    );
    assert!(
        stdout.contains(&format!("'{}'", fixture.fake_bin.display())),
        "binary path containing spaces should be shell-quoted: {stdout}"
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
        !stdout.contains("--large-max-avg-s"),
        "large target threshold should not be passed when only small is selected: {stdout}"
    );
}

#[test]
fn eth_proof_timing_batch_run_does_not_create_configured_tmp_dir() {
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
    let tmp_dir_created = fixture.tmp_dir.exists();
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
        "actual run should not create the configured TMPDIR"
    );
    assert!(
        runner_args.contains("TMPDIR={tmp_dir}"),
        "runner should receive the per-run temp token: {runner_args}"
    );
    assert!(
        runner_args.contains("verify proof --eth-block-input")
            && runner_args.contains("verify_proof_status=ok"),
        "runner should receive a prove-then-verify command: {runner_args}"
    );
    assert!(
        !runner_args.contains(&fixture.tmp_dir.display().to_string()),
        "runner command should not use the configured TMPDIR as a shared run dir: {runner_args}"
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
            && !stdout.contains("--require-text verify_proof_status=ok"),
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
    let tmp_dir_created = fixture.tmp_dir.exists();
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    fixture.cleanup();

    assert!(
        success,
        "env check should pass for a complete small config: stderr={stderr}"
    );
    assert!(!tmp_dir_created, "env check should not create TMPDIR");
    assert!(stdout.contains("status=ok\n"), "{stdout}");
    assert!(stdout.contains("small=ready\n"), "{stdout}");
    assert!(stdout.contains("small_mode=combined\n"), "{stdout}");
    assert!(stdout.contains("small_verify_proof=true\n"), "{stdout}");
    assert!(stdout.contains("small_trace_limit=120000000\n"), "{stdout}");
    assert!(stdout.contains("small_block_input="), "{stdout}");
    assert!(stdout.contains("small_tmp_dir="), "{stdout}");
}

#[test]
fn eth_proof_timing_batch_prints_env_template_without_config() {
    let mut command = Command::new(script_path());
    command
        .arg("--suite")
        .arg("small")
        .arg("--print-env-template");
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
        stdout.contains("# cargo build --release -p lzvm-cli --bin lzvm"),
        "env template should include the default binary build command: {stdout}"
    );
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
        stdout.contains("export LZVM_REAL_SMALL_PARITY_TMP_DIR=temp/tmp")
            && stdout.contains("export LZVM_REAL_SMALL_PARITY_TRACE_LIMIT=120000000"),
        "small template should include optional defaults: {stdout}"
    );
    assert!(
        !stdout.contains(LARGE_PREFIX),
        "small template should not include the large prefix: {stdout}"
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

    assert!(
        !output.status.success(),
        "env check should fail when required config is missing"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("proof environment is incomplete"),
        "env check should explain missing configuration: stderr={}",
        String::from_utf8_lossy(&output.stderr)
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
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    fixture.cleanup();

    assert!(!success, "env check should fail when bin is missing");
    assert!(
        stderr.contains("path does not exist"),
        "env check should explain the missing bin: stderr={stderr}"
    );
}

#[test]
fn eth_proof_timing_batch_rejects_tmp_dir_outside_temp() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-tmp-outside");
    let outside_tmp = workspace_root().join(format!(
        "target/eth-proof-timing-batch-tmp-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&outside_tmp);
    let mut command = Command::new(script_path());
    command.arg("--suite").arg("small").arg("--check-env");
    fixture.apply_env(&mut command, SMALL_PREFIX);
    command.env(format!("{SMALL_PREFIX}_TMP_DIR"), &outside_tmp);

    let output = command
        .output()
        .expect("ETH proof timing batch env check should run");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let outside_created = outside_tmp.exists();
    let tmp_dir_created = fixture.tmp_dir.exists();
    let _ = std::fs::remove_dir_all(&outside_tmp);
    fixture.cleanup();

    assert!(!success, "env check should reject TMPDIR outside temp");
    assert!(
        stderr.contains("_TMP_DIR must be under"),
        "TMPDIR rejection should explain the temp boundary: stderr={stderr}"
    );
    assert!(
        !outside_created,
        "rejected TMPDIR should not be created outside temp"
    );
    assert!(
        !tmp_dir_created,
        "rejected env check should not create the fixture TMPDIR"
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
    let tmp_dir_created = fixture.tmp_dir.exists();
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
        "available dry-run should not create TMPDIR"
    );
    assert!(
        stdout.contains("large_command=env -u LZVM_GUEST_PC_TRACE_PARALLEL_LOWER"),
        "available suite should include the configured large command: {stdout}"
    );
    assert!(
        stdout.contains("verify proof --eth-block-input"),
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
