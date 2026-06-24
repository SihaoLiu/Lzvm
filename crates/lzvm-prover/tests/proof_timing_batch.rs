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

fn test_dir(name: &str) -> std::path::PathBuf {
    workspace_root().join(format!("temp/{name}-{}", std::process::id()))
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
        .arg("printf 'status=ok\nverify_outputs=true\ntiming_total_ms=100{run}\n'")
        .arg("--large-command")
        .arg("printf 'status=ok\nverify_outputs=true\ntiming_total_ms=200{run}\n'")
        .arg("--summary")
        .arg("batch timing")
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
        stdout.contains("large_runs=3"),
        "batch output should report large run count: {stdout}"
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
        batch_json.contains("\"small_command\": \"printf 'status=ok"),
        "batch json should record command templates: {batch_json}"
    );
    assert!(
        batch_json.contains("\"small_logs\": ["),
        "batch json should record input logs: {batch_json}"
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
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        !output.status.success(),
        "proof timing batch should reject output without the required marker"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("missing required text"),
        "required text rejection should explain the missing marker: stderr={}",
        String::from_utf8_lossy(&output.stderr)
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
        batch_json.contains("small-001.log"),
        "batch json should retain completed small logs: {batch_json}"
    );
    assert!(
        batch_json.contains("\"large_logs\": []"),
        "batch json should record no large logs when no large command ran: {batch_json}"
    );
    let append_stderr =
        std::fs::read_to_string(batch_dirs[0].join("append.stderr")).expect("append stderr read");
    assert!(
        append_stderr.contains("append failed"),
        "append stderr should be captured: {append_stderr}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
