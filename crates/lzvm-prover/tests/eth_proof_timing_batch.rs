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
const VERIFY_REQUIRED_TEXTS: &[&str] = &[
    "verify_proof_status=ok",
    "artifact_public_input_match=ok",
    "artifact_proof_match=ok",
    "eth_block_input_match=ok",
    "program_image_cache_match=ok",
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
        let fake_bin = write_fixture(&dir, "lzvm");
        make_executable(&fake_bin);
        let setup = dir.join("setup");
        std::fs::create_dir_all(&setup).expect("setup dir should be created");
        let block_input = write_fixture(&dir, "block.input");
        let cache = write_fixture(&dir, "program-image.cache");
        let input_data = write_fixture(&dir, "input-data.bin");
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
        stdout.contains("large_timing_summaries=3"),
        "self-test should summarize large timing logs: {stdout}"
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
            && verify_command.contains("{batch_dir}/small-{run_padded}.proof/proof.bin")
            && verify_command
                .contains("{batch_dir}/small-{run_padded}.proof/eth-block-public-values.bin")
            && verify_command.contains("verify_proof_status=ok"),
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
        !stdout.contains(&fixture.shared_tmp_dir.display().to_string()),
        "dry-run command should not use a shared TMPDIR: {stdout}"
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
            && stdout.contains("small_mode=combined\n")
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
            && verify_command.contains("verify_proof_status=ok"),
        "runner should receive a prove-then-verify command: {runner_args}"
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
            && !stdout.contains("program_image_cache_match=ok"),
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
    assert!(stdout.contains("small_mode=combined\n"), "{stdout}");
    assert!(stdout.contains("small_verify_proof=true\n"), "{stdout}");
    assert!(
        stdout.contains("small_verify_required_text=verify_proof_status=ok\n")
            && stdout.contains("small_verify_required_text=artifact_public_input_match=ok\n")
            && stdout.contains("small_verify_required_text=artifact_proof_match=ok\n")
            && stdout.contains("small_verify_required_text=eth_block_input_match=ok\n")
            && stdout.contains("small_verify_required_text=program_image_cache_match=ok\n"),
        "env check should report required proof verification markers: {stdout}"
    );
    assert!(stdout.contains("small_trace_limit=120000000\n"), "{stdout}");
    assert!(stdout.contains("small_block_input="), "{stdout}");
    assert!(!stdout.contains("small_tmp_dir="), "{stdout}");
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
        stdout.contains("export LZVM_REAL_SMALL_PARITY_TRACE_LIMIT=120000000"),
        "small template should include optional trace limit default: {stdout}"
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
        .arg("--skip-nsys-export")
        .arg("--profile-arg=--kernel-name-base=demangled")
        .arg("--commit")
        .arg("&&")
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
        stdout.contains(&format!("next_check_command=. {template_rel} && scripts/run-eth-proof-timing-batch.py --suite both")),
        "env template should report a check command: {stdout}"
    );
    assert!(
        stdout.contains(&format!("next_profile_tool_check_command=. {template_rel} && scripts/run-eth-proof-timing-batch.py --suite both"))
            && stdout.contains("--check-profile-tools"),
        "env template should report a proof-independent profile tool check command: {stdout}"
    );
    assert!(
        stdout.contains(&format!("next_profile_command=. {template_rel} && scripts/run-eth-proof-timing-batch.py --suite both")),
        "env template should report a profile command: {stdout}"
    );
    assert!(
        stdout.contains(&format!("next_run_command=. {template_rel} && scripts/run-eth-proof-timing-batch.py --suite both")),
        "env template should report a run command: {stdout}"
    );
    assert!(
        stdout.contains("--max-runs 5") && stdout.contains("--summary 'real proof timing'"),
        "run command should preserve the retry cap and summary placeholder: {stdout}"
    );
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
        stdout.contains(&format!("--nsys-trace {nsys_trace}"))
            && stdout.contains(&format!("--ncu-set {ncu_set}"))
            && stdout.contains(&format!("--ncu-target-processes {ncu_target_processes}"))
            && stdout.contains("--skip-nsys-export"),
        "env template command should preserve profiler tuning flags: {stdout}"
    );
    assert!(
        stdout.contains("--profile-arg --kernel-name-base=demangled"),
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
    assert!(
        template.contains("export LZVM_REAL_SMALL_PARITY_SETUP=")
            && template.contains("export LZVM_REAL_LARGE_PARITY_SETUP="),
        "template should include both selected suites: {template}"
    );
    assert!(
        !template.contains("TMP_DIR"),
        "template should not expose a shared TMPDIR: {template}"
    );
}

#[test]
fn eth_proof_timing_batch_prints_profile_commands_from_env() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-profile-commands");
    let profile_dir = fixture.dir.join("profiles");
    let nsys_path = fixture.dir.join("custom nsys");
    let ncu_path = fixture.dir.join("custom ncu");
    let nsys_trace = "cpu,nvtx";
    let ncu_set = "full";
    let ncu_target_processes = "application";
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
        .arg("--profile-arg=--kernel-name-base=demangled")
        .arg("--profile-arg=--launch-skip=1")
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
        nsys_command.contains("--skip-nsys-export") && !nsys_command.contains("--summarize"),
        "nsys command should avoid the downstream-rejected summarize plus skip-export combination: {stdout}"
    );
    assert!(
        ncu_command.contains("--summarize") && !ncu_command.contains("--skip-nsys-export"),
        "ncu command should still request summary files without nsys-only flags: {stdout}"
    );
    assert!(
        stdout.contains("--profile-arg --kernel-name-base=demangled")
            && stdout.contains("--profile-arg --launch-skip=1"),
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
            && stdout.contains("verify proof --eth-block-input")
            && stdout.contains("&& env TMPDIR=")
            && stdout.contains("verify_proof_status=ok"),
        "profile command should wrap the same prove-then-verify shell command with managed TMPDIR: {stdout}"
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
fn eth_proof_timing_batch_profile_commands_skip_verify_when_requested() {
    let fixture = ProofFixture::new("eth-proof-timing-batch-profile-skip-verify");
    let mut command = Command::new(script_path());
    command
        .arg("--suite")
        .arg("small")
        .arg("--skip-verify-proof")
        .arg("--print-profile-commands");
    fixture.apply_env(&mut command, SMALL_PREFIX);

    let output = command
        .output()
        .expect("ETH proof timing batch profile command should print");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    fixture.cleanup();

    assert!(
        success,
        "profile command output should support skip verify: stderr={stderr}"
    );
    assert!(
        stdout.contains("small_nsys_profile_command="),
        "profile command should still be printed: {stdout}"
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
