use std::process::Command;

fn workspace_root() -> &'static std::path::Path {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("workspace root should resolve")
}

fn batch_script_path() -> std::path::PathBuf {
    workspace_root().join("scripts/run-proof-timing-batch.py")
}

fn scripts_pycache_path() -> std::path::PathBuf {
    workspace_root().join("scripts/__pycache__")
}

fn current_commit() -> String {
    let output = Command::new("git")
        .args(["rev-parse", "--short=8", "HEAD"])
        .current_dir(workspace_root())
        .output()
        .expect("git should resolve current commit");
    assert!(
        output.status.success(),
        "git should resolve current commit: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("git output should be utf-8")
        .trim()
        .to_owned()
}

fn test_dir(name: &str) -> std::path::PathBuf {
    workspace_root().join(format!("temp/{name}-{}", std::process::id()))
}

fn single_batch_dir(dir: &std::path::Path) -> std::path::PathBuf {
    let mut batch_dirs = std::fs::read_dir(dir)
        .expect("fixture dir should read")
        .map(|entry| entry.expect("batch entry should read").path())
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    batch_dirs.sort();
    assert_eq!(
        batch_dirs.len(),
        1,
        "run should leave exactly one batch dir"
    );
    batch_dirs.remove(0)
}

#[test]
fn proof_timing_batch_rejects_nonfinite_numeric_values() {
    for (flag, value) in [
        ("--small-timeout", "nan"),
        ("--large-max-avg-s", "inf"),
        ("--max-relative-spread", "nan"),
    ] {
        let output = Command::new(batch_script_path())
            .arg(flag)
            .arg(value)
            .output()
            .expect("proof timing batch nonfinite validation should run");
        let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

        assert!(
            !output.status.success(),
            "nonfinite value should be rejected for {flag}"
        );
        assert!(
            stdout.is_empty(),
            "nonfinite validation should fail before printing status: {stdout}"
        );
        assert!(
            stderr.contains("must be finite"),
            "nonfinite validation should explain finite floats: stderr={stderr}"
        );
    }
}

#[test]
fn proof_timing_batch_discovers_wide_run_status_paths() {
    let script_path = batch_script_path();
    let pycache_path = scripts_pycache_path();
    let pycache_existed_before = pycache_path.exists();
    let dir = test_dir("proof-timing-batch-wide-status");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture dir should be created");
    std::fs::write(dir.join("small-999.status"), b"status").expect("status should write");
    std::fs::write(dir.join("small-1000.status"), b"status").expect("status should write");
    std::fs::write(dir.join("small-1000.log"), b"log").expect("log should write");
    std::fs::write(dir.join("small-extra.status"), b"status").expect("status should write");
    std::fs::write(dir.join("large-1000.status"), b"status").expect("status should write");
    let python = format!(
        concat!(
            "import importlib.util, pathlib\n",
            "script = pathlib.Path({:?})\n",
            "fixture = pathlib.Path({:?})\n",
            "spec = importlib.util.spec_from_file_location('timing_batch', script)\n",
            "module = importlib.util.module_from_spec(spec)\n",
            "spec.loader.exec_module(module)\n",
            "for path in module.discovered_run_paths(fixture, 'small', '.status'):\n",
            "    print(path.name)\n"
        ),
        script_path.display().to_string(),
        dir.display().to_string()
    );

    let output = Command::new("python3")
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .arg("-c")
        .arg(python)
        .output()
        .expect("python helper check should run");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let discovered = stdout.lines().collect::<Vec<_>>();
    let _ = std::fs::remove_dir_all(&dir);

    assert!(success, "python helper check should pass: stderr={stderr}");
    assert_eq!(
        discovered,
        vec!["small-999.status", "small-1000.status"],
        "only matching numeric status paths should be discovered in run order"
    );
    if !pycache_existed_before {
        assert!(
            !pycache_path.exists(),
            "python helper import should not leave scripts bytecode cache"
        );
    }
}

#[test]
fn proof_timing_batch_runs_commands_and_appends_stable_log() {
    let script_path = batch_script_path();
    let dir = test_dir("proof-timing-batch-test");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture dir should be created");
    let log_path = dir.join("improve-log.csv");

    let output = Command::new(&script_path)
        .arg("--work-dir")
        .arg(&dir)
        .arg("--path")
        .arg(&log_path)
        .arg("--commit")
        .arg("test")
        .arg("--runs")
        .arg("3")
        .arg("--require-proof-output")
        .arg("--small-command")
        .arg(concat!(
            "printf 'status=ok\\nverify_outputs=true\\ntiming_total_ms=100{run}\\n",
            "timing_guest_stage_tree_commit_root_count=1\\n",
            "timing_guest_stage_tree_commit_root_materialization_groups=1\\n",
            "timing_guest_stage_tree_commit_root_materialization_max_group_size=1\\n",
            "timing_finish_witness_opening_row_dedup_input_rows=0\\n",
            "timing_finish_witness_opening_row_dedup_unique_rows=0\\n",
            "timing_finish_witness_opening_row_dedup_elided_rows=0\\n",
            "timing_finish_fri_opening_ms=10\\n",
            "timing_finish_fri_opening_unit_build_ms=8\\n",
            "timing_finish_fri_opening_layer_tree_ms=2\\n",
            "timing_finish_fri_opening_query_ms=3\\n",
            "timing_finish_fri_opening_fold_ms=1\\n",
            "timing_finish_fri_opening_unit_count=1\\n",
            "timing_finish_fri_opening_layer_count=2\\n",
            "timing_finish_fri_opening_query_count=3\\n",
            "timing_finish_fri_transcript_unit_build_ms=4\\n",
            "timing_finish_fri_transcript_layer_tree_ms=2\\n",
            "timing_finish_fri_transcript_fold_ms=1\\n",
            "timing_finish_fri_transcript_unit_count=1\\n",
            "timing_finish_fri_transcript_layer_count=2\\n",
            "timing_finish_contribution_segment_ms=5\\n",
            "timing_finish_contribution_verify_ms=6\\n",
            "timing_finish_contribution_challenge_ms=7\\n'"
        ))
        .arg("--large-command")
        .arg(concat!(
            "printf 'status=ok\\nverify_outputs=true\\ntiming_total_ms=200{run}\\n",
            "timing_guest_stage_tree_commit_root_count=1\\n",
            "timing_guest_stage_tree_commit_root_materialization_groups=1\\n",
            "timing_guest_stage_tree_commit_root_materialization_max_group_size=1\\n",
            "timing_finish_witness_opening_row_dedup_input_rows=0\\n",
            "timing_finish_witness_opening_row_dedup_unique_rows=0\\n",
            "timing_finish_witness_opening_row_dedup_elided_rows=0\\n",
            "timing_finish_fri_opening_ms=10\\n",
            "timing_finish_fri_opening_unit_build_ms=8\\n",
            "timing_finish_fri_opening_layer_tree_ms=2\\n",
            "timing_finish_fri_opening_query_ms=3\\n",
            "timing_finish_fri_opening_fold_ms=1\\n",
            "timing_finish_fri_opening_unit_count=1\\n",
            "timing_finish_fri_opening_layer_count=2\\n",
            "timing_finish_fri_opening_query_count=3\\n",
            "timing_finish_fri_transcript_unit_build_ms=4\\n",
            "timing_finish_fri_transcript_layer_tree_ms=2\\n",
            "timing_finish_fri_transcript_fold_ms=1\\n",
            "timing_finish_fri_transcript_unit_count=1\\n",
            "timing_finish_fri_transcript_layer_count=2\\n",
            "timing_finish_contribution_segment_ms=5\\n",
            "timing_finish_contribution_verify_ms=6\\n",
            "timing_finish_contribution_challenge_ms=7\\n'"
        ))
        .arg("--summary")
        .arg("batch timing")
        .env("LZVM_CUDA_RETAINED_SOURCE_BYTES", "123456")
        .env("LZVM_GUEST_PC_TRACE_DESCRIPTOR_HIGH32_STATS", "1")
        .output()
        .expect("proof timing batch should run");
    assert!(
        output.status.success(),
        "proof timing batch should append stable timing fields: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(
        stdout.contains("small_runs=3"),
        "batch output should report small run count: {stdout}"
    );
    assert!(
        stdout.contains("small_stable_runs=3"),
        "batch output should report stable small run count: {stdout}"
    );
    assert!(
        stdout.contains("small_stable_avg_s=1.002"),
        "batch output should report the stable small average: {stdout}"
    );
    assert!(
        stdout.contains("small_stable_spread_s=0.002")
            && stdout.contains("small_stable_relative_spread=0.001996"),
        "batch output should report stable small timing spread: {stdout}"
    );
    assert!(
        stdout.contains("small_excluded_runs=0"),
        "batch output should report that no small runs were excluded: {stdout}"
    );
    assert!(
        stdout.contains("small_timing_summaries=3"),
        "batch output should report small timing summary count: {stdout}"
    );
    assert!(
        stdout.contains("small_stable_timing_summary="),
        "batch output should report the stable small timing summary: {stdout}"
    );
    assert!(
        stdout.contains("large_runs=3"),
        "batch output should report large run count: {stdout}"
    );
    assert!(
        stdout.contains("large_stable_runs=3"),
        "batch output should report stable large run count: {stdout}"
    );
    assert!(
        stdout.contains("large_stable_avg_s=2.002"),
        "batch output should report the stable large average: {stdout}"
    );
    assert!(
        stdout.contains("large_stable_spread_s=0.002")
            && stdout.contains("large_stable_relative_spread=0.000999"),
        "batch output should report stable large timing spread: {stdout}"
    );
    assert!(
        stdout.contains("large_excluded_runs=0"),
        "batch output should report that no large runs were excluded: {stdout}"
    );
    assert!(
        stdout.contains("large_timing_summaries=3"),
        "batch output should report large timing summary count: {stdout}"
    );
    assert!(
        stdout.contains("large_stable_timing_summary="),
        "batch output should report the stable large timing summary: {stdout}"
    );
    let batch_dir = stdout
        .lines()
        .find_map(|line| line.strip_prefix("batch_dir="))
        .map(std::path::PathBuf::from)
        .expect("batch output should include a batch dir");
    assert!(
        stdout.contains("batch_json="),
        "batch output should report the batch json path: {stdout}"
    );
    let batch_json =
        std::fs::read_to_string(batch_dir.join("batch.json")).expect("batch json should read");
    assert!(
        batch_json.contains("\"appended\": true"),
        "batch json should record successful append: {batch_json}"
    );
    assert!(
        batch_json.contains("\"runs\": 3") && batch_json.contains("\"max_runs\": 3"),
        "batch json should record the effective stable-run cap: {batch_json}"
    );
    assert!(
        batch_json.contains("\"inherited_runtime_env\": {")
            && batch_json.contains("\"LZVM_CUDA_RETAINED_SOURCE_BYTES\": \"123456\"")
            && batch_json.contains("\"LZVM_GUEST_PC_TRACE_DESCRIPTOR_HIGH32_STATS\": \"1\""),
        "batch json should record inherited runtime env: {batch_json}"
    );
    assert!(
        batch_json.contains("\"small_run_count\": 3")
            && batch_json.contains("\"large_run_count\": 3")
            && batch_json.contains("\"small_stable_run_count\": 3")
            && batch_json.contains("\"large_stable_run_count\": 3"),
        "batch json should record explicit run counts: {batch_json}"
    );
    assert!(
        batch_json.contains("\"small_excluded_log_count\": 0")
            && batch_json.contains("\"large_excluded_log_count\": 0")
            && batch_json.contains("\"small_excluded_run_count\": 0")
            && batch_json.contains("\"large_excluded_run_count\": 0"),
        "batch json should record empty excluded run diagnostics: {batch_json}"
    );
    assert!(
        batch_json.contains("\"small_command\": \"printf 'status=ok"),
        "batch json should record command templates: {batch_json}"
    );
    assert!(
        batch_json.contains("\"small_logs\": ["),
        "batch json should record input logs: {batch_json}"
    );
    assert!(
        batch_json.contains("\"small_stable_logs\": [")
            && batch_json.contains("small-001.log")
            && batch_json.contains("\"large_stable_logs\": [")
            && batch_json.contains("large-001.log"),
        "batch json should record stable timing log paths: {batch_json}"
    );
    assert!(
        batch_json.contains("\"small_stable_avg_s\": 1.002")
            && batch_json.contains("\"large_stable_avg_s\": 2.002")
            && batch_json.contains("\"small_stable_avg_ms\": 1002")
            && batch_json.contains("\"large_stable_avg_ms\": 2002"),
        "batch json should record stable average proof times: {batch_json}"
    );
    assert!(
        batch_json.contains("\"small_timing_s\": [")
            && batch_json.contains("1.001")
            && batch_json.contains("1.003")
            && batch_json.contains("\"large_timing_s\": [")
            && batch_json.contains("2.001")
            && batch_json.contains("2.003"),
        "batch json should record raw proof timing samples: {batch_json}"
    );
    assert!(
        batch_json.contains("\"small_stable_spread_s\": 0.002")
            && batch_json.contains("\"large_stable_spread_s\": 0.002")
            && batch_json.contains("\"small_stable_spread_ms\": 2")
            && batch_json.contains("\"large_stable_spread_ms\": 2")
            && batch_json.contains("\"small_stable_relative_spread\": 0.001996")
            && batch_json.contains("\"large_stable_relative_spread\": 0.000999")
            && batch_json.contains("\"small_timing_parse_failed_count\": 0")
            && batch_json.contains("\"large_timing_parse_failed_count\": 0"),
        "batch json should record stable timing spread and parse status: {batch_json}"
    );
    assert!(
        batch_json.contains("\"small_statuses\": [") && batch_json.contains("small-001.status"),
        "batch json should record per-run status paths: {batch_json}"
    );
    assert!(
        batch_json.contains("\"small_timing_summaries\": [")
            && batch_json.contains("small-001.proof-timing-summary.csv")
            && batch_json.contains("\"large_timing_summaries\": [")
            && batch_json.contains("large-001.proof-timing-summary.csv"),
        "batch json should record per-run timing summary paths: {batch_json}"
    );
    assert!(
        batch_json.contains("\"small_stable_timing_summary\":")
            && batch_json.contains("small-stable.proof-timing-summary.csv")
            && batch_json.contains("\"large_stable_timing_summary\":")
            && batch_json.contains("large-stable.proof-timing-summary.csv"),
        "batch json should record stable timing summary paths: {batch_json}"
    );
    assert!(
        batch_json.contains("\"append_script\": \"scripts/append-improve-log.py\"")
            && batch_json.contains("\"append_status\":")
            && batch_json.contains("append.status")
            && batch_json.contains("\"append_stdout\":")
            && batch_json.contains("append.stdout")
            && batch_json.contains("\"append_stderr\":")
            && batch_json.contains("append.stderr"),
        "batch json should record append helper artifacts: {batch_json}"
    );
    let append_status =
        std::fs::read_to_string(batch_dir.join("append.status")).expect("append status read");
    assert!(
        append_status.contains("exit_code=0")
            && append_status.contains("append_stdout=")
            && append_status.contains("append_stderr="),
        "append status should record the successful append outcome: {append_status}"
    );
    let small_status =
        std::fs::read_to_string(batch_dir.join("small-001.status")).expect("status should read");
    assert!(
        small_status.contains("proof_timing_summary=")
            && small_status.contains("small-001.proof-timing-summary.csv"),
        "status should record the per-run timing summary: {small_status}"
    );
    let small_summary =
        std::fs::read_to_string(batch_dir.join("small-001.proof-timing-summary.csv"))
            .expect("small timing summary should read");
    assert!(
        small_summary.starts_with("profile,") && small_summary.contains(",0,1001,"),
        "per-run timing summary should contain CSV timing data: {small_summary}"
    );
    let small_stable_summary =
        std::fs::read_to_string(batch_dir.join("small-stable.proof-timing-summary.csv"))
            .expect("stable small timing summary should read");
    assert!(
        small_stable_summary.contains("aggregate,total_count,valid_total_count")
            && small_stable_summary.contains(
                "dominant_segment_commit_memory_pressure_hint"
            )
            && small_stable_summary.contains("aggregate,3,3,1001"),
        "stable timing summary should contain aggregate timing and memory columns: {small_stable_summary}"
    );

    let contents = std::fs::read_to_string(&log_path).expect("improve log should read");
    assert!(
        contents.contains("\"avg=1.002 samples=1.001;1.002;1.003 used=3/3\""),
        "small timing logs should be averaged from milliseconds: {contents}"
    );
    assert!(
        contents.contains("\"avg=2.002 samples=2.001;2.002;2.003 used=3/3\""),
        "large timing logs should be averaged from milliseconds: {contents}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn proof_timing_batch_summarizes_runs_without_guest_root_shape_counts() {
    let script_path = batch_script_path();
    let dir = test_dir("proof-timing-batch-no-root-shape");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture dir should be created");
    let log_path = dir.join("improve-log.csv");

    let output = Command::new(&script_path)
        .arg("--work-dir")
        .arg(&dir)
        .arg("--path")
        .arg(&log_path)
        .arg("--commit")
        .arg("test")
        .arg("--runs")
        .arg("3")
        .arg("--small-command")
        .arg(concat!(
            "printf 'timing_total_ms=100{run}\\n",
            "timing_finish_witness_opening_row_dedup_input_rows=0\\n",
            "timing_finish_witness_opening_row_dedup_unique_rows=0\\n",
            "timing_finish_witness_opening_row_dedup_elided_rows=0\\n",
            "timing_finish_fri_opening_ms=10\\n",
            "timing_finish_fri_opening_unit_build_ms=8\\n",
            "timing_finish_fri_opening_layer_tree_ms=2\\n",
            "timing_finish_fri_opening_query_ms=3\\n",
            "timing_finish_fri_opening_fold_ms=1\\n",
            "timing_finish_fri_opening_unit_count=1\\n",
            "timing_finish_fri_opening_layer_count=2\\n",
            "timing_finish_fri_opening_query_count=3\\n",
            "timing_finish_fri_transcript_unit_build_ms=4\\n",
            "timing_finish_fri_transcript_layer_tree_ms=2\\n",
            "timing_finish_fri_transcript_fold_ms=1\\n",
            "timing_finish_fri_transcript_unit_count=1\\n",
            "timing_finish_fri_transcript_layer_count=2\\n",
            "timing_finish_contribution_segment_ms=5\\n",
            "timing_finish_contribution_verify_ms=6\\n",
            "timing_finish_contribution_challenge_ms=7\\n'"
        ))
        .arg("--summary")
        .arg("root shape absent")
        .output()
        .expect("proof timing batch should run");

    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let batch_dir = single_batch_dir(&dir);
    let small_status =
        std::fs::read_to_string(batch_dir.join("small-001.status")).expect("status should read");
    let small_summary =
        std::fs::read_to_string(batch_dir.join("small-001.proof-timing-summary.csv"))
            .expect("small timing summary should read");
    let stable_summary =
        std::fs::read_to_string(batch_dir.join("small-stable.proof-timing-summary.csv"))
            .expect("stable timing summary should read");
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        success,
        "proof timing batch should summarize logs without optional root-shape counters: stderr={stderr}"
    );
    assert!(
        stdout.contains("small_timing_summaries=3")
            && stdout.contains("small_stable_timing_summary="),
        "batch output should report generated timing summaries: {stdout}"
    );
    assert!(
        small_status.contains("proof_timing_summary=")
            && small_status.contains("small-001.proof-timing-summary.csv"),
        "status should record the per-run summary: {small_status}"
    );
    assert!(
        small_summary.contains(",0,0,0,0.000,no,none,"),
        "missing root-shape counters should default to zero in per-run CSV: {small_summary}"
    );
    assert!(
        stable_summary.contains("aggregate,total_count,valid_total_count")
            && stable_summary.contains("aggregate,3,3,1001"),
        "stable summary should still aggregate runs without root-shape counters: {stable_summary}"
    );
}

#[test]
fn proof_timing_batch_defaults_commit_to_head() {
    let script_path = batch_script_path();
    let dir = test_dir("proof-timing-batch-default-commit");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture dir should be created");
    let log_path = dir.join("improve-log.csv");
    let commit = current_commit();

    let output = Command::new(&script_path)
        .arg("--work-dir")
        .arg(&dir)
        .arg("--path")
        .arg(&log_path)
        .arg("--runs")
        .arg("3")
        .arg("--small-command")
        .arg(concat!(
            "printf 'timing_total_ms=1000\\n",
            "timing_guest_stage_tree_commit_root_count=1\\n",
            "timing_guest_stage_tree_commit_root_materialization_groups=1\\n",
            "timing_guest_stage_tree_commit_root_materialization_max_group_size=1\\n",
            "timing_finish_witness_opening_row_dedup_input_rows=0\\n",
            "timing_finish_witness_opening_row_dedup_unique_rows=0\\n",
            "timing_finish_witness_opening_row_dedup_elided_rows=0\\n",
            "timing_finish_fri_opening_ms=10\\n",
            "timing_finish_fri_opening_unit_build_ms=8\\n",
            "timing_finish_fri_opening_layer_tree_ms=2\\n",
            "timing_finish_fri_opening_query_ms=3\\n",
            "timing_finish_fri_opening_fold_ms=1\\n",
            "timing_finish_fri_opening_unit_count=1\\n",
            "timing_finish_fri_opening_layer_count=2\\n",
            "timing_finish_fri_opening_query_count=3\\n",
            "timing_finish_fri_transcript_unit_build_ms=4\\n",
            "timing_finish_fri_transcript_layer_tree_ms=2\\n",
            "timing_finish_fri_transcript_fold_ms=1\\n",
            "timing_finish_fri_transcript_unit_count=1\\n",
            "timing_finish_fri_transcript_layer_count=2\\n",
            "timing_finish_contribution_segment_ms=5\\n",
            "timing_finish_contribution_verify_ms=6\\n",
            "timing_finish_contribution_challenge_ms=7\\n'"
        ))
        .arg("--summary")
        .arg("default commit")
        .output()
        .expect("proof timing batch should run");
    assert!(
        output.status.success(),
        "proof timing batch should default commit: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let batch_json = std::fs::read_to_string(single_batch_dir(&dir).join("batch.json"))
        .expect("batch json should read");
    assert!(
        batch_json.contains(&format!("\"commit\": \"{commit}\"")),
        "batch json should record the effective commit: {batch_json}"
    );
    let contents = std::fs::read_to_string(&log_path).expect("improve log should read");
    assert!(
        contents.contains(&format!("\"{commit}\"")),
        "improve log should record the effective commit: {contents}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn proof_timing_batch_reruns_until_stable_sample_group() {
    let script_path = batch_script_path();
    let dir = test_dir("proof-timing-batch-rerun-stable");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture dir should be created");
    let log_path = dir.join("improve-log.csv");

    let output = Command::new(&script_path)
        .arg("--work-dir")
        .arg(&dir)
        .arg("--path")
        .arg(&log_path)
        .arg("--commit")
        .arg("test")
        .arg("--runs")
        .arg("3")
        .arg("--max-runs")
        .arg("4")
        .arg("--small-command")
        .arg(concat!(
            "printf 'runs={runs}\\nmax_runs={max_runs}\\nenv_runs=%s\\nenv_max_runs=%s\\n' ",
            "\"$LZVM_TIMING_BATCH_RUNS\" \"$LZVM_TIMING_BATCH_MAX_RUNS\"; ",
            "if [ \"{run}\" = \"1\" ]; then printf 'timing_total_ms=9000\\n'; ",
            "else printf 'timing_total_ms=100{run}\\n'; fi; ",
            "printf 'timing_guest_stage_tree_commit_root_count=1\\n",
            "timing_guest_stage_tree_commit_root_materialization_groups=1\\n",
            "timing_guest_stage_tree_commit_root_materialization_max_group_size=1\\n",
            "timing_finish_witness_opening_row_dedup_input_rows=0\\n",
            "timing_finish_witness_opening_row_dedup_unique_rows=0\\n",
            "timing_finish_witness_opening_row_dedup_elided_rows=0\\n",
            "timing_finish_fri_opening_ms=10\\n",
            "timing_finish_fri_opening_unit_build_ms=8\\n",
            "timing_finish_fri_opening_layer_tree_ms=2\\n",
            "timing_finish_fri_opening_query_ms=3\\n",
            "timing_finish_fri_opening_fold_ms=1\\n",
            "timing_finish_fri_opening_unit_count=1\\n",
            "timing_finish_fri_opening_layer_count=2\\n",
            "timing_finish_fri_opening_query_count=3\\n",
            "timing_finish_fri_transcript_unit_build_ms=4\\n",
            "timing_finish_fri_transcript_layer_tree_ms=2\\n",
            "timing_finish_fri_transcript_fold_ms=1\\n",
            "timing_finish_fri_transcript_unit_count=1\\n",
            "timing_finish_fri_transcript_layer_count=2\\n",
            "timing_finish_contribution_segment_ms=5\\n",
            "timing_finish_contribution_verify_ms=6\\n",
            "timing_finish_contribution_challenge_ms=7\\n'"
        ))
        .arg("--summary")
        .arg("rerun stable")
        .output()
        .expect("proof timing batch should run");
    assert!(
        output.status.success(),
        "proof timing batch should rerun through an unstable sample: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(
        stdout.contains("small_runs=4"),
        "batch output should report the extra run: {stdout}"
    );
    assert!(
        stdout.contains("small_stable_runs=3"),
        "batch output should report the stable subset size: {stdout}"
    );
    assert!(
        stdout.contains("small_stable_avg_s=1.003"),
        "batch output should report the stable subset average: {stdout}"
    );
    assert!(
        stdout.contains("small_stable_spread_s=0.002")
            && stdout.contains("small_stable_relative_spread=0.001994"),
        "batch output should report the stable subset spread: {stdout}"
    );
    assert!(
        stdout.contains("small_excluded_runs=1")
            && stdout.contains("small_excluded_timing_s=9.000"),
        "batch output should report the excluded outlier timing: {stdout}"
    );
    let batch_dir = stdout
        .lines()
        .find_map(|line| line.strip_prefix("batch_dir="))
        .map(std::path::PathBuf::from)
        .expect("batch output should include a batch dir");
    assert!(
        batch_dir.join("small-004.status").exists(),
        "extra run status should be recorded"
    );
    let extra_log =
        std::fs::read_to_string(batch_dir.join("small-004.log")).expect("extra log should read");
    assert!(
        extra_log.contains("runs=3\n")
            && extra_log.contains("max_runs=4\n")
            && extra_log.contains("env_runs=3\n")
            && extra_log.contains("env_max_runs=4\n"),
        "command template and environment should keep target runs distinct from max runs: {extra_log}"
    );
    let batch_json =
        std::fs::read_to_string(batch_dir.join("batch.json")).expect("batch json should read");
    assert!(
        batch_json.contains("\"runs\": 3") && batch_json.contains("\"max_runs\": 4"),
        "batch json should record the stable-run target and cap: {batch_json}"
    );
    assert!(
        batch_json.contains("\"small_run_count\": 4")
            && batch_json.contains("\"small_stable_run_count\": 3"),
        "batch json should distinguish attempted and stable run counts: {batch_json}"
    );
    let stable_logs = batch_json
        .split("\"small_stable_logs\": [")
        .nth(1)
        .and_then(|tail| tail.split(']').next())
        .expect("batch json should contain small stable logs");
    assert!(
        !stable_logs.contains("small-001.log")
            && stable_logs.contains("small-002.log")
            && stable_logs.contains("small-003.log")
            && stable_logs.contains("small-004.log"),
        "batch json should identify the stable timing subset: {batch_json}"
    );
    let excluded_logs = batch_json
        .split("\"small_excluded_logs\": [")
        .nth(1)
        .and_then(|tail| tail.split(']').next())
        .expect("batch json should contain small excluded logs");
    assert!(
        excluded_logs.contains("small-001.log")
            && !excluded_logs.contains("small-002.log")
            && !excluded_logs.contains("small-003.log")
            && !excluded_logs.contains("small-004.log"),
        "batch json should identify the excluded timing log: {batch_json}"
    );
    assert!(
        batch_json.contains("\"small_excluded_log_count\": 1")
            && batch_json.contains("\"small_excluded_run_count\": 1"),
        "batch json should count the excluded timing log and parseable sample: {batch_json}"
    );
    let raw_timings = batch_json
        .split("\"small_timing_s\": [")
        .nth(1)
        .and_then(|tail| tail.split(']').next())
        .expect("batch json should contain small raw timings");
    assert!(
        raw_timings.contains("9.0")
            && raw_timings.contains("1.002")
            && raw_timings.contains("1.003")
            && raw_timings.contains("1.004"),
        "batch json should keep every parseable timing sample: {batch_json}"
    );
    let stable_timings = batch_json
        .split("\"small_stable_timing_s\": [")
        .nth(1)
        .and_then(|tail| tail.split(']').next())
        .expect("batch json should contain stable small timings");
    assert!(
        !stable_timings.contains("9.0")
            && stable_timings.contains("1.002")
            && stable_timings.contains("1.003")
            && stable_timings.contains("1.004"),
        "batch json should keep the stable timing values separate from the outlier: {batch_json}"
    );
    let excluded_timings = batch_json
        .split("\"small_excluded_timing_s\": [")
        .nth(1)
        .and_then(|tail| tail.split(']').next())
        .expect("batch json should contain excluded small timings");
    assert!(
        excluded_timings.contains("9.0")
            && !excluded_timings.contains("1.002")
            && !excluded_timings.contains("1.003")
            && !excluded_timings.contains("1.004"),
        "batch json should keep excluded timing values separate from the stable subset: {batch_json}"
    );
    assert!(
        batch_json.contains("\"small_stable_spread_s\": 0.002")
            && batch_json.contains("\"small_stable_spread_ms\": 2")
            && batch_json.contains("\"small_stable_relative_spread\": 0.001994"),
        "batch json should report stable subset spread after dropping the outlier: {batch_json}"
    );
    let stable_summary =
        std::fs::read_to_string(batch_dir.join("small-stable.proof-timing-summary.csv"))
            .expect("stable timing summary should read");
    assert!(
        stable_summary.contains("aggregate,3,3,1002,1003.000,1003.000,1004")
            && !stable_summary.contains("9000"),
        "stable timing summary should exclude the outlier run: {stable_summary}"
    );
    let contents = std::fs::read_to_string(&log_path).expect("improve log should read");
    assert!(
        contents.contains("\"avg=1.003 samples=1.002;1.003;1.004 used=3/4\""),
        "improve log should drop the outlier after the extra run: {contents}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn proof_timing_batch_rejects_average_above_max() {
    let script_path = batch_script_path();
    let dir = test_dir("proof-timing-batch-max-average");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture dir should be created");
    let log_path = dir.join("improve-log.csv");

    let output = Command::new(&script_path)
        .arg("--work-dir")
        .arg(&dir)
        .arg("--path")
        .arg(&log_path)
        .arg("--commit")
        .arg("test")
        .arg("--runs")
        .arg("3")
        .arg("--small-max-avg-s")
        .arg("1.0")
        .arg("--small-command")
        .arg("printf 'timing_total_ms=1500\n'")
        .arg("--summary")
        .arg("max average")
        .output()
        .expect("proof timing batch should run");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let batch_dir = single_batch_dir(&dir);
    let batch_json =
        std::fs::read_to_string(batch_dir.join("batch.json")).expect("batch json should read");
    let log_created = log_path.exists();
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        !success,
        "proof timing batch should reject averages above the configured max"
    );
    assert!(
        stderr.contains("small proof time: average 1.500s exceeds --small-max-avg-s 1.000s"),
        "max average rejection should come from append-improve-log: stderr={stderr}"
    );
    assert!(
        batch_json.contains("\"small_max_avg_s\": 1.0")
            && batch_json.contains("\"small_stable_avg_s\": 1.5")
            && batch_json.contains("\"small_stable_avg_ms\": 1500")
            && batch_json.contains("\"appended\": false"),
        "batch json should record the configured max and failed append state: {batch_json}"
    );
    assert!(!log_created, "rejected batch should not append improve log");
}

#[test]
fn proof_timing_batch_can_append_average_rejection() {
    let script_path = batch_script_path();
    let dir = test_dir("proof-timing-batch-append-max-average");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture dir should be created");
    let log_path = dir.join("improve-log.csv");

    let output = Command::new(&script_path)
        .arg("--work-dir")
        .arg(&dir)
        .arg("--path")
        .arg(&log_path)
        .arg("--commit")
        .arg("test")
        .arg("--runs")
        .arg("3")
        .arg("--small-max-avg-s")
        .arg("1.0")
        .arg("--append-max-average-rejections")
        .arg("--small-command")
        .arg("printf 'timing_total_ms=1500\n'")
        .arg("--summary")
        .arg("max average")
        .output()
        .expect("proof timing batch should run");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let batch_dir = single_batch_dir(&dir);
    let batch_json =
        std::fs::read_to_string(batch_dir.join("batch.json")).expect("batch json should read");
    let contents = std::fs::read_to_string(&log_path).expect("improve log should read");
    let append_status = std::fs::read_to_string(batch_dir.join("append.status"))
        .expect("append status should read");
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        !success,
        "proof timing batch should still reject averages above the configured max"
    );
    assert!(
        stderr.contains("small proof time: average 1.500s exceeds --small-max-avg-s 1.000s"),
        "max average rejection should remain visible to callers: stderr={stderr}"
    );
    assert!(
        batch_json.contains("\"small_max_avg_s\": 1.0")
            && batch_json.contains("\"append_max_average_rejections\": true")
            && batch_json.contains("\"small_stable_avg_s\": 1.5")
            && batch_json.contains("\"appended\": true"),
        "batch json should record that the rejected average was appended: {batch_json}"
    );
    assert!(
        contents
            .contains("\"avg=1.500 samples=1.500;1.500;1.500 used=3/3 rejected baseline=1.000\""),
        "improve log should label the stable rejected average: {contents}"
    );
    assert!(
        contents.contains("\"max average; rejected small baseline\""),
        "improve log summary should label the rejected target: {contents}"
    );
    assert!(
        append_status.contains("exit_code=0"),
        "append helper should succeed before the batch reports rejection: {append_status}"
    );
}

#[test]
fn proof_timing_batch_materializes_requested_summary_on_average_rejection() {
    let script_path = batch_script_path();
    let dir = test_dir("proof-timing-batch-rejected-summary");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture dir should be created");
    let log_path = dir.join("improve-log.csv");
    let requested_summary = dir.join("strict-large-summary.csv");

    let output = Command::new(&script_path)
        .arg("--work-dir")
        .arg(&dir)
        .arg("--path")
        .arg(&log_path)
        .arg("--commit")
        .arg("test")
        .arg("--runs")
        .arg("3")
        .arg("--large-max-avg-s")
        .arg("1.0")
        .arg("--append-max-average-rejections")
        .arg("--large-command")
        .arg(concat!(
            "printf 'timing_total_ms=1500\\n",
            "timing_guest_stage_tree_commit_root_count=1\\n",
            "timing_guest_stage_tree_commit_root_materialization_groups=1\\n",
            "timing_guest_stage_tree_commit_root_materialization_max_group_size=1\\n",
            "timing_finish_witness_opening_row_dedup_input_rows=0\\n",
            "timing_finish_witness_opening_row_dedup_unique_rows=0\\n",
            "timing_finish_witness_opening_row_dedup_elided_rows=0\\n",
            "timing_finish_fri_opening_ms=10\\n",
            "timing_finish_fri_opening_unit_build_ms=8\\n",
            "timing_finish_fri_opening_layer_tree_ms=2\\n",
            "timing_finish_fri_opening_query_ms=3\\n",
            "timing_finish_fri_opening_fold_ms=1\\n",
            "timing_finish_fri_opening_unit_count=1\\n",
            "timing_finish_fri_opening_layer_count=2\\n",
            "timing_finish_fri_opening_query_count=3\\n",
            "timing_finish_fri_transcript_unit_build_ms=4\\n",
            "timing_finish_fri_transcript_layer_tree_ms=2\\n",
            "timing_finish_fri_transcript_fold_ms=1\\n",
            "timing_finish_fri_transcript_unit_count=1\\n",
            "timing_finish_fri_transcript_layer_count=2\\n",
            "timing_finish_contribution_segment_ms=5\\n",
            "timing_finish_contribution_verify_ms=6\\n",
            "timing_finish_contribution_challenge_ms=7\\n'"
        ))
        .arg("--summary")
        .arg(&requested_summary)
        .output()
        .expect("proof timing batch should run");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let batch_dir = single_batch_dir(&dir);
    let requested_summary_text =
        std::fs::read_to_string(&requested_summary).expect("requested summary should read");
    let stable_summary_text =
        std::fs::read_to_string(batch_dir.join("large-stable.proof-timing-summary.csv"))
            .expect("stable summary should read");
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        !success,
        "proof timing batch should still reject averages above the configured max"
    );
    assert!(
        stderr.contains("large proof time: average 1.500s exceeds --large-max-avg-s 1.000s"),
        "max average rejection should remain visible to callers: stderr={stderr}"
    );
    assert_eq!(
        requested_summary_text, stable_summary_text,
        "rejected run should materialize the requested summary from the stable large summary"
    );
    assert!(
        requested_summary_text.contains("aggregate,total_count,valid_total_count")
            && requested_summary_text.contains("aggregate,3,3,1500"),
        "requested summary should contain aggregate timing rows: {requested_summary_text}"
    );
}

#[test]
fn proof_timing_batch_removes_failed_per_run_summary_outputs() {
    let script_path = batch_script_path();
    let dir = test_dir("proof-timing-batch-run-summary-fails");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture dir should be created");
    let log_path = dir.join("improve-log.csv");
    let summary_script = dir.join("summary-helper.py");
    std::fs::write(
        &summary_script,
        "import sys\nsys.stderr.write('run summary failed\\n')\nsys.exit(9)\n",
    )
    .expect("summary helper should write");

    let output = Command::new(&script_path)
        .arg("--work-dir")
        .arg(&dir)
        .arg("--path")
        .arg(&log_path)
        .arg("--commit")
        .arg("test")
        .arg("--runs")
        .arg("3")
        .arg("--timing-summary-script")
        .arg(&summary_script)
        .arg("--small-command")
        .arg(concat!(
            "printf 'timing_total_ms=100{run}\\n",
            "timing_guest_stage_tree_commit_root_count=1\\n",
            "timing_guest_stage_tree_commit_root_materialization_groups=1\\n",
            "timing_guest_stage_tree_commit_root_materialization_max_group_size=1\\n",
            "timing_finish_witness_opening_row_dedup_input_rows=0\\n",
            "timing_finish_witness_opening_row_dedup_unique_rows=0\\n",
            "timing_finish_witness_opening_row_dedup_elided_rows=0\\n",
            "timing_finish_fri_opening_ms=10\\n",
            "timing_finish_fri_opening_unit_build_ms=8\\n",
            "timing_finish_fri_opening_layer_tree_ms=2\\n",
            "timing_finish_fri_opening_query_ms=3\\n",
            "timing_finish_fri_opening_fold_ms=1\\n",
            "timing_finish_fri_opening_unit_count=1\\n",
            "timing_finish_fri_opening_layer_count=2\\n",
            "timing_finish_fri_opening_query_count=3\\n",
            "timing_finish_fri_transcript_unit_build_ms=4\\n",
            "timing_finish_fri_transcript_layer_tree_ms=2\\n",
            "timing_finish_fri_transcript_fold_ms=1\\n",
            "timing_finish_fri_transcript_unit_count=1\\n",
            "timing_finish_fri_transcript_layer_count=2\\n",
            "timing_finish_contribution_segment_ms=5\\n",
            "timing_finish_contribution_verify_ms=6\\n",
            "timing_finish_contribution_challenge_ms=7\\n'"
        ))
        .arg("--summary")
        .arg("run summary failure")
        .output()
        .expect("proof timing batch should run");

    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let batch_dir = single_batch_dir(&dir);
    let batch_json =
        std::fs::read_to_string(batch_dir.join("batch.json")).expect("batch json should read");
    let status =
        std::fs::read_to_string(batch_dir.join("small-001.status")).expect("status should read");
    let summary = batch_dir.join("small-001.proof-timing-summary.csv");
    let summary_stderr = batch_dir.join("small-001.proof-timing-summary.csv.stderr");
    let summary_exists = summary.exists();
    let summary_stderr_exists = summary_stderr.exists();
    let log_created = log_path.exists();
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        !success,
        "proof timing batch should fail on per-run summary errors"
    );
    assert!(
        stderr.contains("timing summary failed with status 9")
            && stderr.contains("first stderr line: run summary failed"),
        "per-run summary failure should include status and diagnostic: stderr={stderr}"
    );
    assert!(
        status.contains("validation_error=timing summary failed with status 9"),
        "run status should record the summary validation error: {status}"
    );
    assert!(
        batch_json.contains("\"appended\": false") && batch_json.contains("small-001.log"),
        "batch json should retain the failed run log and append state: {batch_json}"
    );
    assert!(
        !summary_exists && !summary_stderr_exists,
        "failed per-run summary outputs should be removed"
    );
    assert!(
        !log_created,
        "per-run summary failure should not append improve log"
    );
}

#[test]
fn proof_timing_batch_records_batch_json_when_stable_summary_fails() {
    let script_path = batch_script_path();
    let dir = test_dir("proof-timing-batch-stable-summary-fails");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture dir should be created");
    let log_path = dir.join("improve-log.csv");
    let summary_script = dir.join("summary-helper.py");
    std::fs::write(
        &summary_script,
        concat!(
            "import sys\n",
            "if len(sys.argv) > 2:\n",
            "    sys.stderr.write('group summary failed\\n')\n",
            "    sys.exit(7)\n",
            "print('profile,total_count')\n",
            "print('run,1')\n",
        ),
    )
    .expect("summary helper should write");

    let output = Command::new(&script_path)
        .arg("--work-dir")
        .arg(&dir)
        .arg("--path")
        .arg(&log_path)
        .arg("--commit")
        .arg("test")
        .arg("--runs")
        .arg("3")
        .arg("--timing-summary-script")
        .arg(&summary_script)
        .arg("--small-command")
        .arg(concat!(
            "printf 'timing_total_ms=100{run}\\n",
            "timing_guest_stage_tree_commit_root_count=1\\n",
            "timing_guest_stage_tree_commit_root_materialization_groups=1\\n",
            "timing_guest_stage_tree_commit_root_materialization_max_group_size=1\\n",
            "timing_finish_witness_opening_row_dedup_input_rows=0\\n",
            "timing_finish_witness_opening_row_dedup_unique_rows=0\\n",
            "timing_finish_witness_opening_row_dedup_elided_rows=0\\n",
            "timing_finish_fri_opening_ms=10\\n",
            "timing_finish_fri_opening_unit_build_ms=8\\n",
            "timing_finish_fri_opening_layer_tree_ms=2\\n",
            "timing_finish_fri_opening_query_ms=3\\n",
            "timing_finish_fri_opening_fold_ms=1\\n",
            "timing_finish_fri_opening_unit_count=1\\n",
            "timing_finish_fri_opening_layer_count=2\\n",
            "timing_finish_fri_opening_query_count=3\\n",
            "timing_finish_fri_transcript_unit_build_ms=4\\n",
            "timing_finish_fri_transcript_layer_tree_ms=2\\n",
            "timing_finish_fri_transcript_fold_ms=1\\n",
            "timing_finish_fri_transcript_unit_count=1\\n",
            "timing_finish_fri_transcript_layer_count=2\\n",
            "timing_finish_contribution_segment_ms=5\\n",
            "timing_finish_contribution_verify_ms=6\\n",
            "timing_finish_contribution_challenge_ms=7\\n'"
        ))
        .arg("--summary")
        .arg("stable summary failure")
        .output()
        .expect("proof timing batch should run");

    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let batch_dir = single_batch_dir(&dir);
    let batch_json =
        std::fs::read_to_string(batch_dir.join("batch.json")).expect("batch json should read");
    let stable_summary = batch_dir.join("small-stable.proof-timing-summary.csv");
    let stable_summary_stderr = batch_dir.join("small-stable.proof-timing-summary.csv.stderr");
    let stable_summary_exists = stable_summary.exists();
    let stable_summary_stderr_exists = stable_summary_stderr.exists();
    let log_created = log_path.exists();
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        !success,
        "proof timing batch should fail when stable summary generation fails"
    );
    assert!(
        stderr.contains("group timing summary failed with status 7"),
        "stable summary failure should report the helper status: stderr={stderr}"
    );
    assert!(
        stderr.contains("first stderr line: group summary failed"),
        "stable summary failure should include the helper diagnostic: stderr={stderr}"
    );
    assert!(
        batch_json.contains("\"appended\": false")
            && batch_json.contains("small-001.log")
            && batch_json.contains("small-003.log")
            && batch_json.contains("small-001.status")
            && batch_json.contains("small-003.status")
            && batch_json.contains("small-001.proof-timing-summary.csv")
            && batch_json.contains("small-003.proof-timing-summary.csv")
            && batch_json.contains("\"small_stable_timing_summary\": null"),
        "batch json should retain completed run artifacts after stable summary failure: {batch_json}"
    );
    assert!(
        !log_created,
        "stable summary failure should not append improve log"
    );
    assert!(
        !stable_summary_exists && !stable_summary_stderr_exists,
        "failed stable summary output should be removed"
    );
}

#[test]
fn proof_timing_batch_sets_run_tmpdir_under_batch_dir() {
    let script_path = batch_script_path();
    let dir = test_dir("proof timing batch tmpdir");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture dir should be created");
    let log_path = dir.join("improve-log.csv");

    let output = Command::new(&script_path)
        .arg("--work-dir")
        .arg(&dir)
        .arg("--path")
        .arg(&log_path)
        .arg("--commit")
        .arg("test")
        .arg("--runs")
        .arg("3")
        .arg("--small-command")
        .arg(concat!(
            "printf marker > {tmp_dir}/marker && ",
            "python3 -c \"import os; ",
            "tmp=os.environ.get('TMPDIR',''); ",
            "print('tmpdir=' + tmp); ",
            "print('tmpdir_ok=' + str(os.path.isdir(tmp)).lower()); ",
            "print('marker_ok=' + str(os.path.exists(os.path.join(tmp, 'marker'))).lower()); ",
            "print('timing_total_ms=1000')\""
        ))
        .arg("--summary")
        .arg("tmpdir guard")
        .output()
        .expect("proof timing batch should run");
    assert!(
        output.status.success(),
        "proof timing batch should run with a managed TMPDIR: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let batch_dir = stdout
        .lines()
        .find_map(|line| line.strip_prefix("batch_dir="))
        .map(std::path::PathBuf::from)
        .expect("batch output should include a batch dir");
    let expected_tmp = batch_dir.join("small-001.tmp");
    let log =
        std::fs::read_to_string(batch_dir.join("small-001.log")).expect("run log should read");
    assert!(
        log.contains(&format!("tmpdir={}", expected_tmp.display())),
        "run command should receive the managed TMPDIR: {log}"
    );
    assert!(
        log.contains("tmpdir_ok=true"),
        "managed TMPDIR should exist before the command runs: {log}"
    );
    assert!(
        log.contains("marker_ok=true"),
        "command template should expose the managed TMPDIR: {log}"
    );
    assert!(
        expected_tmp.join("marker").exists(),
        "template-created marker should stay inside the run TMPDIR"
    );
    let status = std::fs::read_to_string(batch_dir.join("small-001.status"))
        .expect("status file should read");
    assert!(
        status.contains(&format!("tmp_dir={}", expected_tmp.display())),
        "status should record the managed TMPDIR: {status}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn proof_timing_batch_records_status_when_command_exits_nonzero() {
    let script_path = batch_script_path();
    let dir = test_dir("proof-timing-batch-nonzero");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture dir should be created");
    let log_path = dir.join("improve-log.csv");

    let output = Command::new(&script_path)
        .arg("--work-dir")
        .arg(&dir)
        .arg("--path")
        .arg(&log_path)
        .arg("--commit")
        .arg("test")
        .arg("--runs")
        .arg("3")
        .arg("--small-command")
        .arg("if [ -d \"$TMPDIR\" ]; then printf 'tmpdir_ok=true\\n'; fi; exit 5")
        .arg("--summary")
        .arg("nonzero status")
        .output()
        .expect("proof timing batch should run");

    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let batch_dir = single_batch_dir(&dir);
    let expected_tmp = batch_dir.join("small-001.tmp");
    let tmp_dir_exists = expected_tmp.exists();
    let log =
        std::fs::read_to_string(batch_dir.join("small-001.log")).expect("run log should read");
    let status = std::fs::read_to_string(batch_dir.join("small-001.status"))
        .expect("status file should read");
    let batch_json =
        std::fs::read_to_string(batch_dir.join("batch.json")).expect("batch json should read");
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        !success,
        "proof timing batch should fail when a command exits nonzero"
    );
    assert!(
        stderr.contains("exited with status 5"),
        "nonzero exit should explain the failing status: stderr={stderr}"
    );
    assert!(
        tmp_dir_exists,
        "managed TMPDIR should be created before a nonzero command exits"
    );
    assert!(
        log.contains("tmpdir_ok=true"),
        "nonzero command should observe the managed TMPDIR: {log}"
    );
    assert!(
        status.contains("exit_code=5"),
        "status should record the failing exit code: {status}"
    );
    assert!(
        status.contains("timed_out=false"),
        "status should record that the run did not time out: {status}"
    );
    assert!(
        status.contains(&format!("tmp_dir={}", expected_tmp.display())),
        "status should record the managed TMPDIR: {status}"
    );
    assert!(
        batch_json.contains("\"appended\": false")
            && batch_json.contains("small-001.log")
            && batch_json.contains("small-001.status"),
        "failed run should leave log and status paths in batch json: {batch_json}"
    );
}

#[test]
fn proof_timing_batch_records_status_when_command_times_out() {
    let script_path = batch_script_path();
    let dir = test_dir("proof-timing-batch-timeout");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture dir should be created");
    let log_path = dir.join("improve-log.csv");

    let output = Command::new(&script_path)
        .arg("--work-dir")
        .arg(&dir)
        .arg("--path")
        .arg(&log_path)
        .arg("--commit")
        .arg("test")
        .arg("--runs")
        .arg("3")
        .arg("--small-timeout")
        .arg("0.2")
        .arg("--small-command")
        .arg("if [ -d \"$TMPDIR\" ]; then printf 'tmpdir_ok=true\\n'; fi; sleep 5")
        .arg("--summary")
        .arg("timeout status")
        .output()
        .expect("proof timing batch should run");

    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let batch_dir = single_batch_dir(&dir);
    let expected_tmp = batch_dir.join("small-001.tmp");
    let tmp_dir_exists = expected_tmp.exists();
    let log =
        std::fs::read_to_string(batch_dir.join("small-001.log")).expect("run log should read");
    let status = std::fs::read_to_string(batch_dir.join("small-001.status"))
        .expect("status file should read");
    let batch_json =
        std::fs::read_to_string(batch_dir.join("batch.json")).expect("batch json should read");
    let _ = std::fs::remove_dir_all(&dir);

    assert!(!success, "proof timing batch should fail on timeout");
    assert!(
        stderr.contains("timed out after 0.200s"),
        "timeout should explain the timeout duration: stderr={stderr}"
    );
    assert!(
        tmp_dir_exists,
        "managed TMPDIR should be created before a timed-out command runs"
    );
    assert!(
        log.contains("tmpdir_ok=true"),
        "timed-out command should observe the managed TMPDIR: {log}"
    );
    assert!(
        status.contains("timed_out=true"),
        "status should record the timeout: {status}"
    );
    assert!(
        status.contains(&format!("tmp_dir={}", expected_tmp.display())),
        "status should record the managed TMPDIR: {status}"
    );
    assert!(
        batch_json.contains("\"appended\": false")
            && batch_json.contains("small-001.log")
            && batch_json.contains("small-001.status"),
        "timed-out run should leave log and status paths in batch json: {batch_json}"
    );
}

#[test]
fn proof_timing_batch_rejects_work_dir_outside_temp() {
    let script_path = batch_script_path();
    let outside_dir = workspace_root().join(format!(
        "target/proof-timing-batch-outside-{}",
        std::process::id()
    ));
    let log_path = workspace_root().join(format!(
        "temp/proof-timing-batch-outside-log-{}.csv",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&outside_dir);
    let _ = std::fs::remove_file(&log_path);

    let output = Command::new(&script_path)
        .arg("--work-dir")
        .arg(&outside_dir)
        .arg("--path")
        .arg(&log_path)
        .arg("--commit")
        .arg("test")
        .arg("--runs")
        .arg("3")
        .arg("--small-command")
        .arg("printf 'timing_total_ms=1000\n'")
        .arg("--summary")
        .arg("path guard")
        .output()
        .expect("proof timing batch should run");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let outside_created = outside_dir.exists();
    let log_created = log_path.exists();
    let _ = std::fs::remove_dir_all(&outside_dir);
    let _ = std::fs::remove_file(&log_path);

    assert!(
        !success,
        "proof timing batch should reject work dirs outside temp"
    );
    assert!(
        stderr.contains("--work-dir must be under"),
        "work dir rejection should explain the temp boundary: stderr={stderr}"
    );
    assert!(
        !outside_created,
        "rejected work dir should not be created outside temp"
    );
    assert!(
        !log_created,
        "rejected run should not create an improve log"
    );
}

#[test]
fn proof_timing_batch_rejects_log_path_outside_temp() {
    let script_path = batch_script_path();
    let dir = test_dir("proof-timing-batch-log-path");
    let outside_log = workspace_root().join(format!(
        "target/proof-timing-batch-log-{}.csv",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_file(&outside_log);

    let output = Command::new(&script_path)
        .arg("--work-dir")
        .arg(&dir)
        .arg("--path")
        .arg(&outside_log)
        .arg("--commit")
        .arg("test")
        .arg("--runs")
        .arg("3")
        .arg("--small-command")
        .arg("printf 'timing_total_ms=1000\n'")
        .arg("--summary")
        .arg("path guard")
        .output()
        .expect("proof timing batch should run");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let work_dir_created = dir.exists();
    let log_created = outside_log.exists();
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_file(&outside_log);

    assert!(
        !success,
        "proof timing batch should reject log paths outside temp"
    );
    assert!(
        stderr.contains("--path must be under"),
        "log path rejection should explain the temp boundary: stderr={stderr}"
    );
    assert!(
        !work_dir_created,
        "rejected log path should not create a batch dir"
    );
    assert!(!log_created, "rejected run should not create a log");
}

#[test]
fn proof_timing_batch_rejects_file_cwd_before_creating_batch_dir() {
    let script_path = batch_script_path();
    let dir = test_dir("proof-timing-batch-file-cwd");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture dir should be created");
    let cwd_file = dir.join("cwd-file");
    std::fs::write(&cwd_file, b"not a directory").expect("cwd fixture should write");
    let work_dir = dir.join("runs");
    let log_path = dir.join("improve-log.csv");

    let output = Command::new(&script_path)
        .arg("--work-dir")
        .arg(&work_dir)
        .arg("--cwd")
        .arg(&cwd_file)
        .arg("--path")
        .arg(&log_path)
        .arg("--commit")
        .arg("test")
        .arg("--runs")
        .arg("3")
        .arg("--small-command")
        .arg("printf 'timing_total_ms=1000\n'")
        .arg("--summary")
        .arg("cwd guard")
        .output()
        .expect("proof timing batch should run");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let work_dir_created = work_dir.exists();
    let log_created = log_path.exists();
    let _ = std::fs::remove_dir_all(&dir);

    assert!(!success, "proof timing batch should reject file cwd");
    assert!(
        stderr.contains("command working directory is not a directory"),
        "cwd rejection should explain the path type: stderr={stderr}"
    );
    assert!(
        !work_dir_created,
        "rejected cwd should not create a batch dir"
    );
    assert!(!log_created, "rejected run should not create a log");
}

#[cfg(unix)]
#[test]
fn proof_timing_batch_rejects_status_path_replaced_with_symlink() {
    let script_path = batch_script_path();
    let dir = test_dir("proof-timing-batch-status-symlink");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture dir should be created");
    let log_path = dir.join("improve-log.csv");
    let redirected = dir.join("redirected.status");
    std::fs::write(&redirected, "sentinel\n").expect("redirect target should write");
    let command = format!(
        "ln -s '{}' {{batch_dir}}/small-001.status; printf 'timing_total_ms=1000\\n'",
        redirected.display()
    );

    let output = Command::new(&script_path)
        .arg("--work-dir")
        .arg(&dir)
        .arg("--path")
        .arg(&log_path)
        .arg("--commit")
        .arg("test")
        .arg("--runs")
        .arg("3")
        .arg("--small-command")
        .arg(command)
        .arg("--summary")
        .arg("status link guard")
        .output()
        .expect("proof timing batch should run");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let redirected_text =
        std::fs::read_to_string(&redirected).expect("redirect target should remain readable");
    let batch_dir = single_batch_dir(&dir);
    let batch_json =
        std::fs::read_to_string(batch_dir.join("batch.json")).expect("batch json should read");
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        !success,
        "proof timing batch should reject status symlink output"
    );
    assert!(
        stderr.contains("output path must not be a symlink"),
        "status symlink rejection should explain the path constraint: stderr={stderr}"
    );
    assert_eq!(
        redirected_text, "sentinel\n",
        "rejected status write should not overwrite a symlink target"
    );
    assert!(
        batch_json.contains("\"appended\": false"),
        "failed symlink run should still record batch json: {batch_json}"
    );
}

#[cfg(unix)]
#[test]
fn proof_timing_batch_records_status_when_combined_log_is_symlink() {
    let script_path = batch_script_path();
    let dir = test_dir("proof-timing-batch-log-symlink");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture dir should be created");
    let log_path = dir.join("improve-log.csv");
    let redirected = dir.join("redirected.log");
    std::fs::write(&redirected, "timing_total_ms=9999\n").expect("redirect target should write");
    let command = format!(
        "ln -s '{}' {{batch_dir}}/small-001.log; printf 'timing_total_ms=1000\\n'",
        redirected.display()
    );

    let output = Command::new(&script_path)
        .arg("--work-dir")
        .arg(&dir)
        .arg("--path")
        .arg(&log_path)
        .arg("--commit")
        .arg("test")
        .arg("--runs")
        .arg("3")
        .arg("--small-command")
        .arg(command)
        .arg("--summary")
        .arg("log link guard")
        .output()
        .expect("proof timing batch should run");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let redirected_text =
        std::fs::read_to_string(&redirected).expect("redirect target should remain readable");
    let batch_dir = single_batch_dir(&dir);
    let status =
        std::fs::read_to_string(batch_dir.join("small-001.status")).expect("status should read");
    let batch_json =
        std::fs::read_to_string(batch_dir.join("batch.json")).expect("batch json should read");
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        !success,
        "proof timing batch should reject combined log symlink output"
    );
    assert!(
        stderr.contains("output path must not be a symlink"),
        "combined log symlink rejection should explain the path constraint: stderr={stderr}"
    );
    assert_eq!(
        redirected_text, "timing_total_ms=9999\n",
        "rejected combined log write should not overwrite a symlink target"
    );
    assert!(
        status.contains("validation_error=output path must not be a symlink")
            && status.contains("combined_log="),
        "combined log validation failure should be recorded in status: {status}"
    );
    assert!(
        batch_json.contains("\"appended\": false") && batch_json.contains("small-001.status"),
        "failed combined log write should still record batch json: {batch_json}"
    );
    assert!(
        batch_json.contains("\"small_timing_parse_failed_count\": 1")
            && !batch_json.contains("9.999"),
        "batch json timing discovery should not follow the rejected log symlink: {batch_json}"
    );
}

#[cfg(unix)]
#[test]
fn proof_timing_batch_rejects_preexisting_run_tmpdir_symlink() {
    let script_path = batch_script_path();
    let dir = test_dir("proof-timing-batch-tmpdir-symlink");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture dir should be created");
    let work_dir = dir.join("runs");
    let log_path = dir.join("improve-log.csv");
    let redirected = dir.join("redirected-tmp");
    std::fs::create_dir_all(&redirected).expect("redirect tmp fixture should be created");
    let command = format!(
        concat!(
            "if [ \"{{run}}\" = \"1\" ]; then ln -s '{}' {{batch_dir}}/small-002.tmp; fi; ",
            "if [ \"{{run}}\" = \"2\" ]; then printf marker > \"$TMPDIR/marker\"; fi; ",
            "printf 'timing_total_ms=1000\\n'"
        ),
        redirected.display()
    );

    let output = Command::new(&script_path)
        .arg("--work-dir")
        .arg(&work_dir)
        .arg("--path")
        .arg(&log_path)
        .arg("--commit")
        .arg("test")
        .arg("--runs")
        .arg("3")
        .arg("--small-command")
        .arg(command)
        .arg("--summary")
        .arg("tmpdir link guard")
        .output()
        .expect("proof timing batch should run");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let redirected_marker = redirected.join("marker").exists();
    let batch_dir = single_batch_dir(&work_dir);
    let status =
        std::fs::read_to_string(batch_dir.join("small-002.status")).expect("status should read");
    let batch_json =
        std::fs::read_to_string(batch_dir.join("batch.json")).expect("batch json should read");
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        !success,
        "proof timing batch should reject preexisting managed TMPDIR paths"
    );
    assert!(
        stderr.contains("run tmp dir must not already exist"),
        "tmpdir symlink rejection should explain the path constraint: stderr={stderr}"
    );
    assert!(
        !redirected_marker,
        "rejected run should not write through the tmpdir symlink"
    );
    assert!(
        status.contains("validation_error=run tmp dir must not already exist"),
        "tmpdir validation failure should be recorded in status: {status}"
    );
    assert!(
        batch_json.contains("\"appended\": false") && batch_json.contains("small-002.status"),
        "tmpdir validation failure should leave status in batch json: {batch_json}"
    );
}

#[cfg(unix)]
#[test]
fn proof_timing_batch_validates_open_stdout_capture() {
    let script_path = batch_script_path();
    let dir = test_dir("proof-timing-batch-stdout-replaced");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture dir should be created");
    let log_path = dir.join("improve-log.csv");
    let redirected = dir.join("redirected.stdout");
    std::fs::write(&redirected, "timing_total_ms=1000\n").expect("redirect target should write");
    let command = format!(
        concat!(
            "rm -f {{batch_dir}}/small-{{run_padded}}.stdout; ",
            "ln -s '{}' {{batch_dir}}/small-{{run_padded}}.stdout; ",
            "printf 'captured stdout without timing\\n'"
        ),
        redirected.display()
    );

    let output = Command::new(&script_path)
        .arg("--work-dir")
        .arg(&dir)
        .arg("--path")
        .arg(&log_path)
        .arg("--commit")
        .arg("test")
        .arg("--runs")
        .arg("3")
        .arg("--small-command")
        .arg(command)
        .arg("--summary")
        .arg("stdout capture guard")
        .output()
        .expect("proof timing batch should run");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let batch_dir = single_batch_dir(&dir);
    let combined_log =
        std::fs::read_to_string(batch_dir.join("small-001.log")).expect("log should read");
    let status =
        std::fs::read_to_string(batch_dir.join("small-001.status")).expect("status should read");
    let stdout_path = batch_dir.join("small-001.stdout");
    let stdout_is_symlink = std::fs::symlink_metadata(&stdout_path)
        .expect("stdout path should remain inspectable")
        .file_type()
        .is_symlink();
    let improve_log_created = log_path.exists();
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        !success,
        "proof timing batch should validate the opened stdout capture"
    );
    assert!(
        stderr.contains("expected exactly one timing_total_ms"),
        "stdout replacement should not spoof timing validation: stderr={stderr}"
    );
    assert!(
        stdout_is_symlink,
        "fixture should replace the stdout path with a symlink"
    );
    assert!(
        combined_log.contains("captured stdout without timing")
            && !combined_log.contains("timing_total_ms=1000"),
        "combined log should come from the opened capture file: {combined_log}"
    );
    assert!(
        status.contains("validation_error=") && status.contains("expected exactly one"),
        "status should record validation from the opened capture: {status}"
    );
    assert!(
        !improve_log_created,
        "rejected stdout replacement should not append improve log"
    );
}

#[test]
fn proof_timing_batch_replaces_invalid_capture_bytes() {
    let script_path = batch_script_path();
    let dir = test_dir("proof-timing-batch-invalid-capture-bytes");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture dir should be created");
    let log_path = dir.join("improve-log.csv");

    let output = Command::new(&script_path)
        .arg("--work-dir")
        .arg(&dir)
        .arg("--path")
        .arg(&log_path)
        .arg("--commit")
        .arg("test")
        .arg("--runs")
        .arg("3")
        .arg("--small-command")
        .arg(concat!(
            "python3 -c \"import sys; ",
            "sys.stdout.buffer.write(b'invalid=' + bytes([255]) + ",
            "b'\\ntiming_total_ms=1000\\n')\""
        ))
        .arg("--summary")
        .arg("invalid capture bytes")
        .output()
        .expect("proof timing batch should run");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let batch_dir = single_batch_dir(&dir);
    let combined_log =
        std::fs::read_to_string(batch_dir.join("small-001.log")).expect("log should read");
    let improve_log_created = log_path.exists();
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        success,
        "proof timing batch should tolerate invalid UTF-8 in captures: stderr={stderr}"
    );
    assert!(
        combined_log.contains("invalid=") && combined_log.contains("timing_total_ms=1000"),
        "combined log should include decoded capture output: {combined_log}"
    );
    assert!(
        improve_log_created,
        "valid timing captures with invalid bytes should still append improve log"
    );
}

#[test]
fn proof_timing_batch_rejects_missing_required_output_text() {
    let script_path = batch_script_path();
    let dir = test_dir("proof-timing-batch-required-text");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture dir should be created");
    let log_path = dir.join("improve-log.csv");

    let output = Command::new(&script_path)
        .arg("--work-dir")
        .arg(&dir)
        .arg("--path")
        .arg(&log_path)
        .arg("--commit")
        .arg("test")
        .arg("--runs")
        .arg("3")
        .arg("--require-text")
        .arg("status=ok")
        .arg("--small-command")
        .arg("python3 -c \"print('timing_total_ms=1000')\"")
        .arg("--summary")
        .arg("missing marker")
        .output()
        .expect("proof timing batch should run");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let batch_dir = single_batch_dir(&dir);
    let expected_tmp = batch_dir.join("small-001.tmp");
    let status = std::fs::read_to_string(batch_dir.join("small-001.status"))
        .expect("status file should read");
    let batch_json =
        std::fs::read_to_string(batch_dir.join("batch.json")).expect("batch json should read");
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        !success,
        "proof timing batch should reject output without the required marker"
    );
    assert!(
        stderr.contains("missing required text"),
        "required text rejection should explain the missing marker: stderr={}",
        stderr
    );
    assert!(
        status.contains("validation_error=") && status.contains("missing required text"),
        "validation failure should be recorded in status: {status}"
    );
    assert!(
        status.contains("exit_code=0"),
        "status should preserve the command exit code for validation failures: {status}"
    );
    assert!(
        status.contains(&format!("tmp_dir={}", expected_tmp.display())),
        "status should record the managed TMPDIR for validation failures: {status}"
    );
    assert!(
        batch_json.contains("\"appended\": false")
            && batch_json.contains("small-001.log")
            && batch_json.contains("small-001.status"),
        "validation failure should leave log and status paths in batch json: {batch_json}"
    );
}

#[test]
fn proof_timing_batch_rejects_logs_without_unique_total() {
    let script_path = batch_script_path();
    let dir = test_dir("proof-timing-batch-reject");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture dir should be created");
    let log_path = dir.join("improve-log.csv");

    let output = Command::new(&script_path)
        .arg("--work-dir")
        .arg(&dir)
        .arg("--path")
        .arg(&log_path)
        .arg("--commit")
        .arg("test")
        .arg("--runs")
        .arg("3")
        .arg("--small-command")
        .arg("python3 -c \"print('timing_total_ms=1000'); print('timing_total_ms=1001')\"")
        .arg("--summary")
        .arg("ambiguous batch timing")
        .output()
        .expect("proof timing batch should run");
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        !output.status.success(),
        "proof timing batch should reject ambiguous timing output"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("expected exactly one timing_total_ms"),
        "ambiguous timing rejection should explain the timing line count: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn proof_timing_batch_records_logs_when_append_fails() {
    let script_path = batch_script_path();
    let dir = test_dir("proof-timing-batch-append-fails");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture dir should be created");
    let log_path = dir.join("improve-log.csv");
    let failing_append = dir.join("failing-append.py");
    std::fs::write(
        &failing_append,
        "import sys\nsys.stderr.write('append failed\\n')\nsys.exit(7)\n",
    )
    .expect("failing append script should write");

    let output = Command::new(&script_path)
        .arg("--work-dir")
        .arg(&dir)
        .arg("--path")
        .arg(&log_path)
        .arg("--append-script")
        .arg(&failing_append)
        .arg("--commit")
        .arg("test")
        .arg("--runs")
        .arg("3")
        .arg("--small-command")
        .arg("printf 'timing_total_ms=100{run}\n'")
        .arg("--summary")
        .arg("append failure")
        .output()
        .expect("proof timing batch should run");

    assert!(
        !output.status.success(),
        "proof timing batch should reject append failures"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("append-improve-log failed with status 7"),
        "append failure should explain the failing status: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let mut batch_dirs = std::fs::read_dir(&dir)
        .expect("fixture dir should read")
        .map(|entry| entry.expect("batch entry should read").path())
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    batch_dirs.sort();
    assert_eq!(
        batch_dirs.len(),
        1,
        "failed append should leave exactly one batch dir"
    );
    let batch_json =
        std::fs::read_to_string(batch_dirs[0].join("batch.json")).expect("batch json should read");
    assert!(
        batch_json.contains("\"appended\": false"),
        "batch json should record that append did not succeed: {batch_json}"
    );
    assert!(
        batch_json.contains("\"append_script\":")
            && batch_json.contains("failing-append.py")
            && batch_json.contains("\"append_status\":")
            && batch_json.contains("append.status")
            && batch_json.contains("\"append_stderr\":")
            && batch_json.contains("append.stderr"),
        "batch json should record failed append artifacts: {batch_json}"
    );
    assert!(
        batch_json.contains("small-001.log"),
        "batch json should retain completed small logs: {batch_json}"
    );
    assert!(
        batch_json.contains("small-001.status"),
        "batch json should retain completed small statuses: {batch_json}"
    );
    assert!(
        batch_json.contains("\"large_logs\": []"),
        "batch json should record no large logs when no large command ran: {batch_json}"
    );
    assert!(
        batch_json.contains("\"large_run_count\": 0")
            && batch_json.contains("\"large_stable_run_count\": 0"),
        "batch json should record zero large counts when no large command ran: {batch_json}"
    );
    let append_stderr =
        std::fs::read_to_string(batch_dirs[0].join("append.stderr")).expect("append stderr read");
    let append_status =
        std::fs::read_to_string(batch_dirs[0].join("append.status")).expect("append status read");
    assert!(
        append_stderr.contains("append failed"),
        "append stderr should be captured: {append_stderr}"
    );
    assert!(
        append_status.contains("exit_code=7")
            && append_status.contains("append_stdout=")
            && append_status.contains("append_stderr="),
        "append status should record the failed append outcome: {append_status}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[cfg(unix)]
#[test]
fn proof_timing_batch_rejects_append_status_symlink_before_appending() {
    let script_path = batch_script_path();
    let dir = test_dir("proof-timing-batch-append-status-symlink");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture dir should be created");
    let log_path = dir.join("improve-log.csv");
    let redirected = dir.join("redirected-append.status");
    std::fs::write(&redirected, "sentinel\n").expect("redirect target should write");
    let command = format!(
        concat!(
            "if [ \"{{run}}\" = \"1\" ]; then ",
            "ln -s '{}' {{batch_dir}}/append.status; ",
            "fi; printf 'timing_total_ms=100{{run}}\\n'"
        ),
        redirected.display()
    );

    let output = Command::new(&script_path)
        .arg("--work-dir")
        .arg(&dir)
        .arg("--path")
        .arg(&log_path)
        .arg("--commit")
        .arg("test")
        .arg("--runs")
        .arg("3")
        .arg("--small-command")
        .arg(command)
        .arg("--summary")
        .arg("append status link guard")
        .output()
        .expect("proof timing batch should run");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let batch_dir = single_batch_dir(&dir);
    let batch_json =
        std::fs::read_to_string(batch_dir.join("batch.json")).expect("batch json should read");
    let redirected_text =
        std::fs::read_to_string(&redirected).expect("redirect target should remain readable");
    let improve_log_created = log_path.exists();
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        !success,
        "proof timing batch should reject append status symlinks before appending"
    );
    assert!(
        stderr.contains("append artifact path must not already exist"),
        "append artifact rejection should explain the preexisting path: stderr={stderr}"
    );
    assert_eq!(
        redirected_text, "sentinel\n",
        "rejected append status write should not overwrite a symlink target"
    );
    assert!(
        !improve_log_created,
        "append artifact rejection should happen before writing the improve log"
    );
    assert!(
        batch_json.contains("\"appended\": false")
            && batch_json.contains("\"append_status\": null")
            && batch_json.contains("\"append_stdout\": null")
            && batch_json.contains("\"append_stderr\": null"),
        "batch json should not report rejected append symlink artifacts: {batch_json}"
    );
}

#[cfg(unix)]
#[test]
fn proof_timing_batch_ignores_symlinked_append_artifacts_after_early_failure() {
    let script_path = batch_script_path();
    let dir = test_dir("proof-timing-batch-append-artifact-symlinks");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture dir should be created");
    let log_path = dir.join("improve-log.csv");
    let stdout_target = dir.join("redirected-append.stdout");
    let stderr_target = dir.join("redirected-append.stderr");
    let status_target = dir.join("redirected-append.status");
    std::fs::write(&stdout_target, "stdout sentinel\n").expect("stdout target should write");
    std::fs::write(&stderr_target, "stderr sentinel\n").expect("stderr target should write");
    std::fs::write(&status_target, "status sentinel\n").expect("status target should write");
    let command = format!(
        concat!(
            "ln -s '{}' {{batch_dir}}/append.stdout; ",
            "ln -s '{}' {{batch_dir}}/append.stderr; ",
            "ln -s '{}' {{batch_dir}}/append.status; ",
            "exit 5"
        ),
        stdout_target.display(),
        stderr_target.display(),
        status_target.display()
    );

    let output = Command::new(&script_path)
        .arg("--work-dir")
        .arg(&dir)
        .arg("--path")
        .arg(&log_path)
        .arg("--commit")
        .arg("test")
        .arg("--runs")
        .arg("3")
        .arg("--small-command")
        .arg(command)
        .arg("--summary")
        .arg("early append artifact link guard")
        .output()
        .expect("proof timing batch should run");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let batch_dir = single_batch_dir(&dir);
    let batch_json =
        std::fs::read_to_string(batch_dir.join("batch.json")).expect("batch json should read");
    let improve_log_created = log_path.exists();
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        !success,
        "proof timing batch should fail on the command's nonzero exit"
    );
    assert!(
        stderr.contains("exited with status 5"),
        "nonzero run should report the failing status: stderr={stderr}"
    );
    assert!(
        !improve_log_created,
        "early command failure should not create the improve log"
    );
    assert!(
        batch_json.contains("\"appended\": false")
            && batch_json.contains("\"append_status\": null")
            && batch_json.contains("\"append_stdout\": null")
            && batch_json.contains("\"append_stderr\": null"),
        "batch json should not report symlinked append artifacts after early failure: {batch_json}"
    );
}
