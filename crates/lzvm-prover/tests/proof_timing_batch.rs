use std::process::Command;

#[test]
fn proof_timing_batch_runs_commands_and_appends_stable_log() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = crate_root
        .parent()
        .and_then(std::path::Path::parent)
        .expect("workspace root should resolve");
    let script_path = workspace_root.join("scripts/run-proof-timing-batch.py");
    let dir = workspace_root.join(format!(
        "temp/proof-timing-batch-test-{}",
        std::process::id()
    ));
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
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = crate_root
        .parent()
        .and_then(std::path::Path::parent)
        .expect("workspace root should resolve");
    let script_path = workspace_root.join("scripts/run-proof-timing-batch.py");
    let dir = workspace_root.join(format!(
        "temp/proof-timing-batch-required-text-{}",
        std::process::id()
    ));
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
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = crate_root
        .parent()
        .and_then(std::path::Path::parent)
        .expect("workspace root should resolve");
    let script_path = workspace_root.join("scripts/run-proof-timing-batch.py");
    let dir = workspace_root.join(format!(
        "temp/proof-timing-batch-reject-{}",
        std::process::id()
    ));
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
