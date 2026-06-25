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
    write_executable_script(path, "#!/bin/sh\nexit 0\n");
}

#[cfg(unix)]
fn write_executable_script(path: &std::path::Path, contents: &str) {
    use std::os::unix::fs::PermissionsExt;

    std::fs::write(path, contents).expect("fixture executable should write");
    let mut permissions = std::fs::metadata(path)
        .expect("fixture executable metadata should read")
        .permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(path, permissions).expect("fixture executable mode should update");
}

fn prepend_path(command: &mut Command, path: &std::path::Path) {
    let old_path = std::env::var_os("PATH").unwrap_or_default();
    let mut paths = std::env::split_paths(&old_path).collect::<Vec<_>>();
    paths.insert(0, path.to_path_buf());
    let joined = std::env::join_paths(paths).expect("fixture PATH should join");
    command.env("PATH", joined);
}

#[cfg(unix)]
#[test]
fn proof_profile_check_gpu_memory_reports_ready_status() {
    let output_dir = workspace_root().join(format!(
        "temp/proof-profile-gpu-memory-ready-{}",
        std::process::id()
    ));
    let smi_path = output_dir.join("nvidia-smi-ready");
    let _ = std::fs::remove_dir_all(&output_dir);
    std::fs::create_dir_all(&output_dir).expect("fixture dir should be created");
    write_executable_script(
        &smi_path,
        "#!/usr/bin/env python3\nprint('0, GPU-free, 24576, 4096, 20480')\n",
    );

    let output = Command::new(script_path())
        .arg("--check-gpu-memory")
        .arg("--min-gpu-free-mib")
        .arg("1024")
        .arg("--nvidia-smi-command")
        .arg(&smi_path)
        .env_remove("CUDA_VISIBLE_DEVICES")
        .output()
        .expect("proof profile GPU memory check should run");
    let success = output.status.success();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let profile_dir_created = output_dir.join("profiles").exists();
    let _ = std::fs::remove_dir_all(&output_dir);

    assert!(success, "GPU memory check should pass: stderr={stderr}");
    assert!(
        stdout.contains("gpu_memory_source=arg\n")
            && stdout.contains(&format!("gpu_memory_command={}\n", smi_path.display()))
            && stdout.contains("gpu_memory_min_free_mib=1024\n")
            && stdout.contains("gpu_memory_device_count=1\n")
            && stdout.contains("gpu_memory_selected_index=0\n")
            && stdout.contains("gpu_memory_selected_uuid=GPU-free\n")
            && stdout.contains("gpu_memory_free_mib=20480\n")
            && stdout.contains("gpu_memory_status=ready\n"),
        "GPU memory check should report ready capacity: {stdout}"
    );
    assert!(
        !stdout.contains("profile_command="),
        "standalone GPU memory check should not require a profiled command: {stdout}"
    );
    assert!(
        !profile_dir_created,
        "GPU memory check should not create profile output directories"
    );
}

#[cfg(unix)]
#[test]
fn proof_profile_check_gpu_memory_fails_when_free_memory_is_low() {
    let output_dir = workspace_root().join(format!(
        "temp/proof-profile-gpu-memory-low-{}",
        std::process::id()
    ));
    let smi_path = output_dir.join("nvidia-smi-low");
    let _ = std::fs::remove_dir_all(&output_dir);
    std::fs::create_dir_all(&output_dir).expect("fixture dir should be created");
    write_executable_script(
        &smi_path,
        "#!/usr/bin/env python3\nprint('0, GPU-low, 24576, 24288, 288')\n",
    );

    let output = Command::new(script_path())
        .arg("--check-gpu-memory")
        .arg("--min-gpu-free-mib")
        .arg("1024")
        .arg("--nvidia-smi-command")
        .arg(&smi_path)
        .env_remove("CUDA_VISIBLE_DEVICES")
        .output()
        .expect("proof profile low GPU memory check should run");
    let success = output.status.success();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let _ = std::fs::remove_dir_all(&output_dir);

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

#[cfg(unix)]
#[test]
fn proof_profile_check_gpu_memory_uses_first_visible_cuda_device() {
    let output_dir = workspace_root().join(format!(
        "temp/proof-profile-gpu-memory-visible-device-{}",
        std::process::id()
    ));
    let smi_path = output_dir.join("nvidia-smi-visible");
    let _ = std::fs::remove_dir_all(&output_dir);
    std::fs::create_dir_all(&output_dir).expect("fixture dir should be created");
    write_executable_script(
        &smi_path,
        "#!/usr/bin/env python3\nprint('0, GPU-low, 24576, 24288, 288')\nprint('1, GPU-free, 24576, 4096, 20480')\n",
    );

    let output = Command::new(script_path())
        .arg("--check-gpu-memory")
        .arg("--min-gpu-free-mib")
        .arg("1024")
        .arg("--nvidia-smi-command")
        .arg(&smi_path)
        .env("CUDA_VISIBLE_DEVICES", "1,0")
        .output()
        .expect("proof profile visible GPU memory check should run");
    let success = output.status.success();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let profile_dir_created = output_dir.join("profiles").exists();
    let _ = std::fs::remove_dir_all(&output_dir);

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
    assert!(
        !stdout.contains("profile_command="),
        "standalone GPU memory check should not require a profiled command: {stdout}"
    );
    assert!(
        !profile_dir_created,
        "GPU memory check should not create profile output directories"
    );
}

#[cfg(unix)]
#[test]
fn proof_profile_check_tool_can_include_gpu_memory_preflight() {
    let output_dir = workspace_root().join(format!(
        "temp/proof-profile-check-tool-gpu-memory-{}",
        std::process::id()
    ));
    let tool_path = output_dir.join("custom-nsys");
    let smi_path = output_dir.join("nvidia-smi-ready");
    let _ = std::fs::remove_dir_all(&output_dir);
    std::fs::create_dir_all(&output_dir).expect("fixture dir should be created");
    write_executable(&tool_path);
    write_executable_script(
        &smi_path,
        "#!/usr/bin/env python3\nprint('0, GPU-free, 24576, 4096, 20480')\n",
    );

    let output = Command::new(script_path())
        .arg("--tool")
        .arg("nsys")
        .arg("--nsys-command")
        .arg(&tool_path)
        .arg("--output-dir")
        .arg(output_dir.join("profiles"))
        .arg("--check-tool")
        .arg("--check-gpu-memory")
        .arg("--nvidia-smi-command")
        .arg(&smi_path)
        .env_remove("CUDA_VISIBLE_DEVICES")
        .output()
        .expect("proof profile tool and GPU memory check should run");
    let success = output.status.success();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let profile_dir_created = output_dir.join("profiles").exists();
    let _ = std::fs::remove_dir_all(&output_dir);

    assert!(success, "combined check should pass: stderr={stderr}");
    assert!(
        stdout.contains("tool_status=ready\n") && stdout.contains("gpu_memory_status=ready\n"),
        "combined check should report both tool and GPU readiness: {stdout}"
    );
    assert!(
        !profile_dir_created,
        "combined check should not create profile output directories"
    );
}

#[cfg(unix)]
#[test]
fn proof_profile_records_gpu_memory_preflight_in_json() {
    let output_dir = workspace_root().join(format!(
        "temp/proof-profile-json-gpu-memory-{}",
        std::process::id()
    ));
    let profiler_path = output_dir.join("fake-nsys");
    let smi_path = output_dir.join("nvidia-smi-ready");
    let profile_dir = output_dir.join("profiles");
    let _ = std::fs::remove_dir_all(&output_dir);
    std::fs::create_dir_all(&output_dir).expect("fixture dir should be created");
    write_executable_script(
        &profiler_path,
        concat!(
            "#!/usr/bin/env python3\n",
            "import pathlib, subprocess, sys\n",
            "args = sys.argv[1:]\n",
            "if 'profile' in args and '--output' in args:\n",
            "    prefix = pathlib.Path(args[args.index('--output') + 1])\n",
            "    pathlib.Path(str(prefix) + '.nsys-rep').write_text('report\\n', encoding='utf-8')\n",
            "if '--' in args:\n",
            "    command = args[args.index('--') + 1:]\n",
            "    if command:\n",
            "        raise SystemExit(subprocess.run(command).returncode)\n",
        ),
    );
    write_executable_script(
        &smi_path,
        "#!/usr/bin/env python3\nprint('0, GPU-free, 24576, 4096, 20480')\n",
    );

    let output = Command::new(script_path())
        .arg("--tool")
        .arg("nsys")
        .arg("--nsys-command")
        .arg(&profiler_path)
        .arg("--skip-nsys-export")
        .arg("--check-gpu-memory")
        .arg("--nvidia-smi-command")
        .arg(&smi_path)
        .arg("--output-dir")
        .arg(&profile_dir)
        .arg("--name")
        .arg("json-gpu")
        .arg("--")
        .arg("python3")
        .arg("-c")
        .arg("print('timing_total_ms=1000')")
        .env_remove("CUDA_VISIBLE_DEVICES")
        .output()
        .expect("proof profile should run with GPU memory preflight");
    let success = output.status.success();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let json_path = profile_dir.join("json-gpu.profile.json");
    let json_text = std::fs::read_to_string(&json_path).unwrap_or_default();
    let _ = std::fs::remove_dir_all(&output_dir);

    assert!(
        success,
        "profile should pass: stdout={stdout} stderr={stderr}"
    );
    assert!(
        stdout.contains("gpu_memory_status=ready\n"),
        "profile stdout should report the GPU memory preflight: {stdout}"
    );
    assert!(
        json_text.contains("\"gpu_memory_check\"")
            && json_text.contains("\"status\": \"ready\"")
            && json_text.contains("\"free_mib\": 20480")
            && json_text.contains("\"selected_uuid\": \"GPU-free\""),
        "profile JSON should record GPU memory preflight details: {json_text}"
    );
}

#[cfg(unix)]
#[test]
fn proof_profile_records_low_gpu_memory_preflight_in_json() {
    let output_dir = workspace_root().join(format!(
        "temp/proof-profile-json-low-gpu-memory-{}",
        std::process::id()
    ));
    let profiler_path = output_dir.join("fake-nsys");
    let profiler_marker = output_dir.join("profiler-ran");
    let smi_path = output_dir.join("nvidia-smi-low");
    let profile_dir = output_dir.join("profiles");
    let _ = std::fs::remove_dir_all(&output_dir);
    std::fs::create_dir_all(&output_dir).expect("fixture dir should be created");
    write_executable_script(
        &profiler_path,
        &format!(
            "#!/usr/bin/env python3\nimport pathlib\npathlib.Path({:?}).write_text('ran\\n', encoding='utf-8')\n",
            profiler_marker.to_string_lossy()
        ),
    );
    write_executable_script(
        &smi_path,
        "#!/usr/bin/env python3\nprint('0, GPU-busy, 24576, 24288, 288')\n",
    );

    let output = Command::new(script_path())
        .arg("--tool")
        .arg("nsys")
        .arg("--nsys-command")
        .arg(&profiler_path)
        .arg("--skip-nsys-export")
        .arg("--check-gpu-memory")
        .arg("--min-gpu-free-mib")
        .arg("1024")
        .arg("--nvidia-smi-command")
        .arg(&smi_path)
        .arg("--output-dir")
        .arg(&profile_dir)
        .arg("--name")
        .arg("json-low-gpu")
        .arg("--")
        .arg("python3")
        .arg("-c")
        .arg("print('timing_total_ms=1000')")
        .env_remove("CUDA_VISIBLE_DEVICES")
        .output()
        .expect("proof profile should run GPU memory preflight");
    let success = output.status.success();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let json_path = profile_dir.join("json-low-gpu.profile.json");
    let json_text = std::fs::read_to_string(&json_path).unwrap_or_default();
    let profiler_ran = profiler_marker.exists();
    let profile_tmp_dir_created = profile_dir.join("json-low-gpu.tmp").exists();
    let target_tmp_dir_created = profile_dir.join("json-low-gpu.target.tmp").exists();
    let _ = std::fs::remove_dir_all(&output_dir);

    assert!(!success, "low GPU memory should fail the preflight");
    assert!(
        stdout.contains("gpu_memory_status=low\n"),
        "profile stdout should report low GPU memory: {stdout}"
    );
    assert!(
        !profiler_ran,
        "profile command should not run after a failed GPU memory preflight"
    );
    assert!(
        !profile_tmp_dir_created && !target_tmp_dir_created,
        "failed GPU memory preflight should not create profile runtime temp dirs"
    );
    assert!(
        json_text.contains("\"status\": \"gpu_memory_failed\"")
            && json_text.contains("\"profile_exit_code\": 1")
            && json_text.contains("\"gpu_memory_check\"")
            && json_text.contains("\"status\": \"low\"")
            && json_text.contains("\"free_mib\": 288"),
        "profile JSON should record failed GPU memory preflight details: {json_text}"
    );
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

#[cfg(unix)]
#[test]
fn proof_profile_check_tool_uses_env_profiler_command() {
    let output_dir = workspace_root().join(format!(
        "temp/proof-profile-check-tool-env-{}",
        std::process::id()
    ));
    let tool_path = output_dir.join("env-ncu");
    let _ = std::fs::remove_dir_all(&output_dir);
    std::fs::create_dir_all(&output_dir).expect("fixture dir should be created");
    write_executable(&tool_path);

    let output = Command::new(script_path())
        .arg("--tool")
        .arg("ncu")
        .arg("--output-dir")
        .arg(output_dir.join("profiles"))
        .arg("--check-tool")
        .env("LZVM_NCU_COMMAND", &tool_path)
        .output()
        .expect("proof profile env tool check should run");
    let success = output.status.success();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let profile_dir_created = output_dir.join("profiles").exists();
    let _ = std::fs::remove_dir_all(&output_dir);

    assert!(success, "env tool check should pass: stderr={stderr}");
    assert!(
        stdout.contains("tool=ncu\n")
            && stdout.contains("tool_source=env\n")
            && stdout.contains(&format!("tool_command={}\n", tool_path.display()))
            && stdout.contains("tool_status=ready\n")
            && stdout.contains("tool_resolved=temp/proof-profile-check-tool-env-"),
        "tool check should report the env-selected profiler: {stdout}"
    );
    assert!(
        !profile_dir_created,
        "env tool check should not create profile output directories"
    );
}

#[cfg(unix)]
#[test]
fn proof_profile_check_tool_resolves_bare_command_from_path() {
    let output_dir = workspace_root().join(format!(
        "temp/proof-profile-check-tool-path-{}",
        std::process::id()
    ));
    let bin_dir = output_dir.join("bin");
    let tool_path = bin_dir.join("profile-tool");
    let _ = std::fs::remove_dir_all(&output_dir);
    std::fs::create_dir_all(&bin_dir).expect("fixture bin dir should be created");
    write_executable(&tool_path);

    let mut command = Command::new(script_path());
    command
        .arg("--tool")
        .arg("nsys")
        .arg("--nsys-command")
        .arg("profile-tool")
        .arg("--output-dir")
        .arg(output_dir.join("profiles"))
        .arg("--check-tool");
    prepend_path(&mut command, &bin_dir);

    let output = command
        .output()
        .expect("proof profile PATH tool check should run");
    let success = output.status.success();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let profile_dir_created = output_dir.join("profiles").exists();
    let _ = std::fs::remove_dir_all(&output_dir);

    assert!(success, "PATH tool check should pass: stderr={stderr}");
    assert!(
        stdout.contains("tool=nsys\n")
            && stdout.contains("tool_source=arg\n")
            && stdout.contains("tool_command=profile-tool\n")
            && stdout.contains("tool_status=ready\n")
            && stdout.contains("tool_resolved=temp/proof-profile-check-tool-path-"),
        "tool check should resolve bare profiler commands from PATH: {stdout}"
    );
    assert!(
        !profile_dir_created,
        "PATH tool check should not create profile output directories"
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

#[cfg(unix)]
#[test]
fn proof_profile_rejects_symlinked_output_path() {
    use std::os::unix::fs::symlink;

    let output_dir = workspace_root().join(format!(
        "temp/proof-profile-symlink-output-{}",
        std::process::id()
    ));
    let profile_dir = output_dir.join("profiles");
    let _ = std::fs::remove_dir_all(&output_dir);
    std::fs::create_dir_all(&profile_dir).expect("profile fixture dir should be created");
    let redirected = output_dir.join("redirected.profile.json");
    std::fs::write(&redirected, "sentinel\n").expect("redirect target should write");
    symlink(
        &redirected,
        profile_dir.join("symlinked-profile.profile.json"),
    )
    .expect("profile JSON symlink fixture should be created");

    let output = Command::new(script_path())
        .arg("--output-dir")
        .arg(&profile_dir)
        .arg("--name")
        .arg("symlinked-profile")
        .arg("--")
        .arg("python3")
        .arg("-c")
        .arg("print('timing_total_ms=1000')")
        .output()
        .expect("proof profile should reject symlinked output path");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let redirected_text =
        std::fs::read_to_string(&redirected).expect("redirect target should remain readable");
    let _ = std::fs::remove_dir_all(&output_dir);

    assert!(
        !success,
        "proof profile should reject symlinked output paths"
    );
    assert!(
        stderr.contains("profile_json output path must not be a symlink"),
        "symlink rejection should explain the output path: stderr={stderr}"
    );
    assert_eq!(
        redirected_text, "sentinel\n",
        "rejected profile output should not overwrite a symlink target"
    );
}

#[cfg(unix)]
#[test]
fn proof_profile_rejects_output_path_replaced_with_symlink() {
    let output_dir = workspace_root().join(format!(
        "temp/proof-profile-symlink-race-{}",
        std::process::id()
    ));
    let profiler_path = output_dir.join("fake-nsys");
    let profile_dir = output_dir.join("profiles");
    let json_path = profile_dir.join("race-profile.profile.json");
    let redirected = output_dir.join("redirected.profile.json");
    let _ = std::fs::remove_dir_all(&output_dir);
    std::fs::create_dir_all(&output_dir).expect("fixture dir should be created");
    std::fs::write(&redirected, "sentinel\n").expect("redirect target should write");
    write_executable_script(
        &profiler_path,
        concat!(
            "#!/usr/bin/env python3\n",
            "import pathlib, subprocess, sys\n",
            "args = sys.argv[1:]\n",
            "if 'profile' in args and '--output' in args:\n",
            "    prefix = pathlib.Path(args[args.index('--output') + 1])\n",
            "    pathlib.Path(str(prefix) + '.nsys-rep').write_text('report\\n', encoding='utf-8')\n",
            "if '--' in args:\n",
            "    command = args[args.index('--') + 1:]\n",
            "    if command:\n",
            "        raise SystemExit(subprocess.run(command).returncode)\n",
        ),
    );

    let output = Command::new(script_path())
        .arg("--tool")
        .arg("nsys")
        .arg("--nsys-command")
        .arg(&profiler_path)
        .arg("--skip-nsys-export")
        .arg("--output-dir")
        .arg(&profile_dir)
        .arg("--name")
        .arg("race-profile")
        .arg("--")
        .arg("python3")
        .arg("-c")
        .arg(format!(
            "import os, pathlib\njson_path = pathlib.Path({:?})\nredirected = pathlib.Path({:?})\njson_path.unlink(missing_ok=True)\nos.symlink(redirected, json_path)\nprint('timing_total_ms=1000')\n",
            json_path.to_string_lossy(),
            redirected.to_string_lossy()
        ))
        .output()
        .expect("proof profile should reject replaced symlink output path");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let redirected_text =
        std::fs::read_to_string(&redirected).expect("redirect target should remain readable");
    let _ = std::fs::remove_dir_all(&output_dir);

    assert!(
        !success,
        "proof profile should reject output paths replaced with symlinks"
    );
    assert!(
        stderr.contains("output path must not be a symlink"),
        "no-follow write rejection should explain the path constraint: stderr={stderr}"
    );
    assert_eq!(
        redirected_text, "sentinel\n",
        "rejected no-follow write should not overwrite a symlink target"
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

#[test]
fn proof_profile_rejects_dot_profile_names() {
    for name in [".", ".."] {
        let output = Command::new(script_path())
            .arg("--name")
            .arg(name)
            .arg("--dry-run")
            .arg("--")
            .arg("python3")
            .arg("-c")
            .arg("print('timing_total_ms=1000')")
            .output()
            .expect("proof profile dry-run should reject dot names");

        assert!(
            !output.status.success(),
            "proof profile should reject profile name {name:?}"
        );
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("regular profile name"),
            "dot-name rejection should explain the profile name boundary: stderr={}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
