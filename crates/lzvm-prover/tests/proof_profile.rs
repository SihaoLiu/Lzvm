use std::process::Command;

fn workspace_root() -> &'static std::path::Path {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("workspace root should resolve")
}

fn script_path() -> std::path::PathBuf {
    workspace_root().join("scripts/run-proof-profile.py")
}

#[test]
fn proof_profile_self_test_runs() {
    let output = Command::new(script_path())
        .arg("--self-test")
        .output()
        .expect("proof profile self-test should run");

    assert!(
        output.status.success(),
        "proof profile self-test should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(
        stdout.contains("nsys_report=temp/proof-profile-self-test-")
            && stdout.contains("profile_tmp_dir=temp/proof-profile-self-test-")
            && stdout.contains("profile_target_tmp_dir=temp/proof-profile-self-test-")
            && stdout.contains("profile_log=temp/proof-profile-self-test-")
            && stdout.contains("profile_json_output=temp/proof-profile-self-test-")
            && stdout.contains("nsys_exported_sqlite=temp/proof-profile-self-test-")
            && stdout.contains("ncu_csv=temp/proof-profile-self-test-")
            && stdout.contains("ncu_kernel_summary=temp/proof-profile-self-test-")
            && stdout.contains("proof_timing_summary=temp/proof-profile-self-test-"),
        "self-test should report profiler output paths: {stdout}"
    );
    assert!(
        stdout.contains("proof_timing_summary=skipped_missing_keys=")
            && stdout.contains("timing_guest_stage_tree_commit_root_count")
            && stdout.contains("timing_guest_stage_tree_commit_root_materialization_groups")
            && stdout
                .contains("timing_guest_stage_tree_commit_root_materialization_max_group_size"),
        "self-test should skip incomplete proof timing summaries without failing: {stdout}"
    );
}

#[test]
fn proof_profile_nsys_dry_run_prints_summary_commands() {
    let output_dir = workspace_root().join(format!(
        "temp/proof-profile-nsys-dry-run-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&output_dir);

    let output = Command::new(script_path())
        .arg("--tool")
        .arg("nsys")
        .arg("--output-dir")
        .arg(&output_dir)
        .arg("--name")
        .arg("small-proof")
        .arg("--dry-run")
        .arg("--")
        .arg("python3")
        .arg("-c")
        .arg("print('timing_total_ms=1000')")
        .output()
        .expect("proof profile dry-run should run");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let output_dir_created = output_dir.exists();
    let _ = std::fs::remove_dir_all(&output_dir);

    assert!(
        success,
        "nsys proof profile dry-run should pass: stderr={stderr}"
    );
    assert!(
        stdout.contains("profile_command=")
            && stdout.contains("nsys profile")
            && stdout.contains("--trace=cuda,nvtx,osrt"),
        "dry-run should print the nsys profile command: {stdout}"
    );
    assert!(
        stdout.contains("nsys_report=temp/proof-profile-nsys-dry-run-")
            && stdout.contains("small-proof.nsys-rep"),
        "dry-run should report the nsys output path: {stdout}"
    );
    assert!(
        stdout.contains("profile_tmp_dir=temp/proof-profile-nsys-dry-run-")
            && stdout.contains("small-proof.tmp"),
        "dry-run should report the managed profiler TMPDIR: {stdout}"
    );
    assert!(
        stdout.contains("profile_target_tmp_dir=temp/proof-profile-nsys-dry-run-")
            && stdout.contains("small-proof.target.tmp"),
        "dry-run should report the profiled command TMPDIR: {stdout}"
    );
    assert!(
        stdout.contains("profile_stdout=temp/proof-profile-nsys-dry-run-")
            && stdout.contains("small-proof.profile.stdout")
            && stdout.contains("profile_stderr=temp/proof-profile-nsys-dry-run-")
            && stdout.contains("small-proof.profile.stderr")
            && stdout.contains("profile_log=temp/proof-profile-nsys-dry-run-")
            && stdout.contains("small-proof.profile.log")
            && stdout.contains("profile_json_output=temp/proof-profile-nsys-dry-run-")
            && stdout.contains("small-proof.profile.json")
            && stdout.contains("proof_timing_summary_output=temp/proof-profile-nsys-dry-run-")
            && stdout.contains("small-proof.proof-timing-summary.csv"),
        "dry-run should report captured profile output paths: {stdout}"
    );
    assert!(
        stdout.contains("nsys_export_command=")
            && stdout.contains(" export --type sqlite")
            && stdout.contains("nsys_cuda_kernel_summary_command=")
            && stdout.contains("nsys_cuda_kernel_summary_output=")
            && stdout.contains("nsys_cuda_sync_summary_command=")
            && stdout.contains("nsys_cuda_sync_summary_output=")
            && stdout.contains("nsys_cuda_copy_summary_command="),
        "dry-run should print follow-up summary commands: {stdout}"
    );
    assert!(
        !output_dir_created,
        "dry-run should not create the temp output directory"
    );
}

#[test]
fn proof_profile_ncu_dry_run_prints_csv_summary_command() {
    let output_dir = workspace_root().join(format!(
        "temp/proof-profile-ncu-dry-run-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&output_dir);

    let output = Command::new(script_path())
        .arg("--tool")
        .arg("ncu")
        .arg("--output-dir")
        .arg(&output_dir)
        .arg("--name")
        .arg("kernel-metrics")
        .arg("--profile-arg=--kernel-name-base=demangled")
        .arg("--dry-run")
        .arg("--")
        .arg("python3")
        .arg("-c")
        .arg("print('timing_total_ms=1000')")
        .output()
        .expect("proof profile dry-run should run");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let output_dir_created = output_dir.exists();
    let _ = std::fs::remove_dir_all(&output_dir);

    assert!(
        success,
        "ncu proof profile dry-run should pass: stderr={stderr}"
    );
    assert!(
        stdout.contains("profile_command=")
            && stdout.contains("ncu --target-processes all --set basic --page raw --csv")
            && stdout.contains("--kernel-name-base=demangled"),
        "dry-run should print the ncu profile command: {stdout}"
    );
    assert!(
        stdout.contains("ncu_report=temp/proof-profile-ncu-dry-run-")
            && stdout.contains("kernel-metrics.ncu-rep")
            && stdout.contains("ncu_csv=temp/proof-profile-ncu-dry-run-")
            && stdout.contains("kernel-metrics.ncu.csv"),
        "dry-run should report ncu report and CSV paths: {stdout}"
    );
    assert!(
        stdout.contains("profile_tmp_dir=temp/proof-profile-ncu-dry-run-")
            && stdout.contains("kernel-metrics.tmp"),
        "dry-run should report the managed profiler TMPDIR: {stdout}"
    );
    assert!(
        stdout.contains("profile_target_tmp_dir=temp/proof-profile-ncu-dry-run-")
            && stdout.contains("kernel-metrics.target.tmp"),
        "dry-run should report the profiled command TMPDIR: {stdout}"
    );
    assert!(
        stdout.contains("profile_stdout=temp/proof-profile-ncu-dry-run-")
            && stdout.contains("kernel-metrics.profile.stdout")
            && stdout.contains("profile_stderr=temp/proof-profile-ncu-dry-run-")
            && stdout.contains("kernel-metrics.profile.stderr")
            && stdout.contains("profile_log=temp/proof-profile-ncu-dry-run-")
            && stdout.contains("kernel-metrics.profile.log")
            && stdout.contains("profile_json_output=temp/proof-profile-ncu-dry-run-")
            && stdout.contains("kernel-metrics.profile.json")
            && stdout.contains("proof_timing_summary_output=temp/proof-profile-ncu-dry-run-")
            && stdout.contains("kernel-metrics.proof-timing-summary.csv"),
        "dry-run should report captured profile output paths: {stdout}"
    );
    assert!(
        stdout.contains("ncu_cuda_kernel_summary_command=scripts/ncu-cuda-kernel-summary.py"),
        "dry-run should print the ncu summary command: {stdout}"
    );
    assert!(
        stdout.contains("ncu_cuda_kernel_summary_output=temp/proof-profile-ncu-dry-run-"),
        "dry-run should print where the ncu summary will be written: {stdout}"
    );
    assert!(
        !output_dir_created,
        "dry-run should not create the temp output directory"
    );
}

#[test]
fn proof_profile_nsys_dry_run_uses_custom_export_command() {
    let output_dir = workspace_root().join(format!(
        "temp/proof-profile-custom-nsys-{}",
        std::process::id()
    ));
    let nsys_command = output_dir.join("custom nsys");
    let _ = std::fs::remove_dir_all(&output_dir);

    let output = Command::new(script_path())
        .arg("--tool")
        .arg("nsys")
        .arg("--nsys-command")
        .arg(&nsys_command)
        .arg("--output-dir")
        .arg(&output_dir)
        .arg("--name")
        .arg("custom-export")
        .arg("--dry-run")
        .arg("--")
        .arg("python3")
        .arg("-c")
        .arg("print('timing_total_ms=1000')")
        .output()
        .expect("proof profile dry-run should run with custom nsys");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let _ = std::fs::remove_dir_all(&output_dir);

    assert!(success, "custom nsys dry-run should pass: stderr={stderr}");
    assert!(
        stdout.contains(&format!(
            "profile_command='{}' profile",
            nsys_command.display()
        )) && stdout.contains(&format!(
            "nsys_export_command='{}' export --type sqlite",
            nsys_command.display()
        )),
        "custom nsys command should be used for profile and export commands: {stdout}"
    );
}

#[test]
fn proof_profile_ncu_dry_run_uses_custom_profile_command() {
    let output_dir = workspace_root().join(format!(
        "temp/proof-profile-custom-ncu-{}",
        std::process::id()
    ));
    let ncu_command = output_dir.join("custom ncu");
    let _ = std::fs::remove_dir_all(&output_dir);

    let output = Command::new(script_path())
        .arg("--tool")
        .arg("ncu")
        .arg("--ncu-command")
        .arg(&ncu_command)
        .arg("--output-dir")
        .arg(&output_dir)
        .arg("--name")
        .arg("custom-kernels")
        .arg("--dry-run")
        .arg("--")
        .arg("python3")
        .arg("-c")
        .arg("print('timing_total_ms=1000')")
        .output()
        .expect("proof profile dry-run should run with custom ncu");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let _ = std::fs::remove_dir_all(&output_dir);

    assert!(success, "custom ncu dry-run should pass: stderr={stderr}");
    assert!(
        stdout.contains(&format!(
            "profile_command='{}' --target-processes all --set basic",
            ncu_command.display()
        )) && stdout.contains("--page raw --csv")
            && stdout.contains("ncu_cuda_kernel_summary_command=scripts/ncu-cuda-kernel-summary.py"),
        "custom ncu command should be used for profile command while keeping summary output: {stdout}"
    );
}

#[cfg(unix)]
fn write_executable(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;

    std::fs::write(path, b"#!/bin/sh\nexit 0\n").expect("fixture executable should write");
    let mut permissions = std::fs::metadata(path)
        .expect("fixture executable metadata should read")
        .permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(path, permissions).expect("fixture executable mode should update");
}

#[cfg(unix)]
#[test]
fn proof_profile_check_tool_reports_ready_custom_profiler() {
    let output_dir = workspace_root().join(format!(
        "temp/proof-profile-check-tool-ready-{}",
        std::process::id()
    ));
    let tool_path = output_dir.join("custom-nsys");
    let _ = std::fs::remove_dir_all(&output_dir);
    std::fs::create_dir_all(&output_dir).expect("fixture dir should be created");
    write_executable(&tool_path);
    let _ = std::fs::remove_dir_all(output_dir.join("profiles"));

    let output = Command::new(script_path())
        .arg("--tool")
        .arg("nsys")
        .arg("--nsys-command")
        .arg(&tool_path)
        .arg("--output-dir")
        .arg(output_dir.join("profiles"))
        .arg("--check-tool")
        .output()
        .expect("proof profile tool check should run");
    let success = output.status.success();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let profile_dir_created = output_dir.join("profiles").exists();
    let _ = std::fs::remove_dir_all(&output_dir);

    assert!(success, "tool check should pass: stderr={stderr}");
    assert!(
        stdout.contains("tool=nsys\n")
            && stdout.contains("tool_source=arg\n")
            && stdout.contains(&format!("tool_command={}\n", tool_path.display()))
            && stdout.contains("tool_status=ready\n")
            && stdout.contains("tool_resolved=temp/proof-profile-check-tool-ready-")
            && stdout.contains("output_dir=temp/proof-profile-check-tool-ready-")
            && stdout.contains("cwd=.\n"),
        "tool check should report the selected profiler and output target: {stdout}"
    );
    assert!(
        !profile_dir_created,
        "tool check should not create profile output directories"
    );
}

#[test]
fn proof_profile_check_tool_reports_missing_profiler() {
    let output_dir = workspace_root().join(format!(
        "temp/proof-profile-check-tool-missing-{}",
        std::process::id()
    ));
    let tool_path = output_dir.join("missing-ncu");
    let _ = std::fs::remove_dir_all(&output_dir);

    let output = Command::new(script_path())
        .arg("--tool")
        .arg("ncu")
        .arg("--ncu-command")
        .arg(&tool_path)
        .arg("--output-dir")
        .arg(&output_dir)
        .arg("--check-tool")
        .output()
        .expect("proof profile missing tool check should run");
    let success = output.status.success();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let output_dir_created = output_dir.exists();
    let _ = std::fs::remove_dir_all(&output_dir);

    assert!(!success, "missing tool check should fail");
    assert!(
        stderr.is_empty(),
        "missing tool check should report status on stdout only: stderr={stderr}"
    );
    assert!(
        stdout.contains("tool=ncu\n")
            && stdout.contains("tool_source=arg\n")
            && stdout.contains(&format!("tool_command={}\n", tool_path.display()))
            && stdout.contains("tool_status=missing\n")
            && !stdout.contains("tool_resolved="),
        "missing tool check should report the unresolved profiler: {stdout}"
    );
    assert!(
        !output_dir_created,
        "missing tool check should not create profile output directories"
    );
}

#[test]
fn proof_profile_check_tool_respects_relative_command_cwd() {
    let output_dir = workspace_root().join(format!(
        "temp/proof-profile-check-tool-relative-cwd-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&output_dir);
    std::fs::create_dir_all(&output_dir).expect("fixture dir should be created");

    let output = Command::new(script_path())
        .arg("--tool")
        .arg("nsys")
        .arg("--nsys-command")
        .arg("scripts/run-proof-profile.py")
        .arg("--cwd")
        .arg(&output_dir)
        .arg("--output-dir")
        .arg(output_dir.join("profiles"))
        .arg("--check-tool")
        .output()
        .expect("proof profile relative command check should run");
    let success = output.status.success();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let profile_dir_created = output_dir.join("profiles").exists();
    let _ = std::fs::remove_dir_all(&output_dir);

    assert!(
        !success,
        "relative profiler path should be checked relative to --cwd, not the caller workspace"
    );
    assert!(
        stderr.is_empty(),
        "relative command miss should report status on stdout only: stderr={stderr}"
    );
    assert!(
        stdout.contains("tool=nsys\n")
            && stdout.contains("tool_source=arg\n")
            && stdout.contains("tool_command=scripts/run-proof-profile.py\n")
            && stdout.contains("tool_status=missing\n")
            && !stdout.contains("tool_resolved="),
        "relative command check should not report a workspace-relative false ready: {stdout}"
    );
    assert!(
        !profile_dir_created,
        "relative command check should not create profile output directories"
    );
}

#[test]
fn proof_profile_check_tool_rejects_output_dir_outside_temp() {
    let output_dir = workspace_root().join(format!(
        "target/proof-profile-check-tool-outside-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&output_dir);

    let output = Command::new(script_path())
        .arg("--output-dir")
        .arg(&output_dir)
        .arg("--check-tool")
        .output()
        .expect("proof profile tool check should reject outside output dir");
    let success = output.status.success();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let created = output_dir.exists();
    let _ = std::fs::remove_dir_all(&output_dir);

    assert!(
        !success,
        "tool check should reject output dirs outside temp"
    );
    assert!(
        stdout.is_empty(),
        "rejected tool check should not print partial diagnostics: {stdout}"
    );
    assert!(
        stderr.contains("--output-dir must be under"),
        "output-dir rejection should explain the temp boundary: stderr={stderr}"
    );
    assert!(
        !created,
        "rejected output dir should not be created outside temp"
    );
}

#[test]
fn proof_profile_rejects_output_dir_outside_temp() {
    let output_dir = workspace_root().join(format!(
        "target/proof-profile-outside-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&output_dir);

    let output = Command::new(script_path())
        .arg("--output-dir")
        .arg(&output_dir)
        .arg("--dry-run")
        .arg("--")
        .arg("python3")
        .arg("-c")
        .arg("print('timing_total_ms=1000')")
        .output()
        .expect("proof profile dry-run should reject output dir");
    let success = output.status.success();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let created = output_dir.exists();
    let _ = std::fs::remove_dir_all(&output_dir);

    assert!(
        !success,
        "proof profile should reject output dirs outside temp"
    );
    assert!(
        stdout.is_empty(),
        "rejected profile path should not print commands: {stdout}"
    );
    assert!(
        stderr.contains("--output-dir must be under"),
        "output-dir rejection should explain the temp boundary: stderr={stderr}"
    );
    assert!(
        !created,
        "rejected output dir should not be created outside temp"
    );
}

#[test]
fn proof_profile_rejects_nsys_summary_without_sqlite_export() {
    let output = Command::new(script_path())
        .arg("--tool")
        .arg("nsys")
        .arg("--summarize")
        .arg("--skip-nsys-export")
        .arg("--dry-run")
        .arg("--")
        .arg("python3")
        .arg("-c")
        .arg("print('timing_total_ms=1000')")
        .output()
        .expect("proof profile dry-run should reject incompatible nsys options");

    assert!(
        !output.status.success(),
        "nsys summary should require a SQLite export"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("--summarize requires nsys SQLite export"),
        "incompatible nsys options should explain the missing export: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn proof_profile_rejects_unsafe_profile_names() {
    let output = Command::new(script_path())
        .arg("--name")
        .arg("../bad")
        .arg("--dry-run")
        .arg("--")
        .arg("python3")
        .arg("-c")
        .arg("print('timing_total_ms=1000')")
        .output()
        .expect("proof profile dry-run should reject unsafe names");

    assert!(
        !output.status.success(),
        "proof profile should reject names with path separators"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("--name may contain only"),
        "unsafe name rejection should explain allowed characters: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
}
