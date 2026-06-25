use std::process::Command;

#[test]
fn improve_log_writer_accepts_summary_flag_and_keeps_csv_parseable() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = crate_root
        .parent()
        .and_then(std::path::Path::parent)
        .expect("workspace root should resolve");
    let script_path = workspace_root.join("scripts/append-improve-log.py");
    let log_path = workspace_root.join(format!(
        "temp/improve-log-summary-flag-{}.csv",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&log_path);

    let summary = "summary with, comma";
    let output = Command::new(&script_path)
        .arg("--path")
        .arg(&log_path)
        .arg("--commit")
        .arg("test")
        .arg("--small")
        .arg("1.0")
        .arg("--large")
        .arg("2.0")
        .arg("--summary")
        .arg(summary)
        .output()
        .expect("improve-log writer should run");
    assert!(
        output.status.success(),
        "improve-log writer should accept --summary: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let check_output = Command::new(&script_path)
        .arg("--path")
        .arg(&log_path)
        .arg("--check")
        .output()
        .expect("improve-log writer check should run");
    assert!(
        check_output.status.success(),
        "improve-log writer check should pass: stderr={}",
        String::from_utf8_lossy(&check_output.stderr)
    );

    let contents = std::fs::read_to_string(&log_path).expect("improve log should read");
    let rows = contents.lines().collect::<Vec<_>>();
    assert_eq!(
        rows.len(),
        2,
        "improve log should contain a header and one row"
    );
    assert!(
        rows[1].ends_with("\",\"summary with, comma\""),
        "summary field should remain double-quoted: {}",
        rows[1]
    );

    let csv_reader_check = Command::new("python3")
        .arg("-c")
        .arg(concat!(
            "import csv, sys\n",
            "with open(sys.argv[1], newline='') as f:\n",
            "    reader = csv.reader(f)\n",
            "    header = next(reader)\n",
            "    assert header == ['timestamp', 'commit', 'small_proof_time_s', 'large_proof_time_s', 'summary']\n",
            "    for index, row in enumerate(reader, start=2):\n",
            "        assert len(row) == 5, (index, len(row), row)\n",
        ))
        .arg(&log_path)
        .output()
        .expect("csv.reader verification should run");
    assert!(
        csv_reader_check.status.success(),
        "csv.reader should parse every improve-log row as five fields: stderr={}",
        String::from_utf8_lossy(&csv_reader_check.stderr)
    );

    let parsed = rows[1].split(',').collect::<Vec<_>>();
    assert_ne!(
        parsed.len(),
        5,
        "plain comma splitting should demonstrate why summary quoting matters"
    );
    let _ = std::fs::remove_file(&log_path);
}

#[test]
fn improve_log_check_accepts_quoted_timing_fields_with_commas() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = crate_root
        .parent()
        .and_then(std::path::Path::parent)
        .expect("workspace root should resolve");
    let script_path = workspace_root.join("scripts/append-improve-log.py");
    let log_path = workspace_root.join(format!(
        "temp/improve-log-quoted-timings-{}.csv",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&log_path);
    std::fs::write(
        &log_path,
        concat!(
            "timestamp,commit,small_proof_time_s,large_proof_time_s,summary\n",
            "\"2026-06-19T05:55:00-0700\",\"testcase\",\"8.55,8.54,8.49 avg=8.53\",",
            "\"52.29,51.61,51.21 avg=51.70\",\"Summary, with comma\"\n",
        ),
    )
    .expect("temporary improve log should write");

    let output = Command::new(&script_path)
        .arg("--path")
        .arg(&log_path)
        .arg("--check")
        .output()
        .expect("improve-log writer check should run");
    let _ = std::fs::remove_file(&log_path);

    assert!(
        output.status.success(),
        "improve-log check should treat quoted timing commas as field contents: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn improve_log_writer_averages_stable_run_samples() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = crate_root
        .parent()
        .and_then(std::path::Path::parent)
        .expect("workspace root should resolve");
    let script_path = workspace_root.join("scripts/append-improve-log.py");
    let log_path = workspace_root.join(format!(
        "temp/improve-log-stable-runs-{}.csv",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&log_path);

    let output = Command::new(&script_path)
        .arg("--path")
        .arg(&log_path)
        .arg("--commit")
        .arg("test")
        .arg("--small-runs")
        .arg("8.55,8.54,8.49")
        .arg("--large-runs")
        .arg("52.29,51.61,51.21,90.00")
        .arg("--summary")
        .arg("stable run samples")
        .output()
        .expect("improve-log writer should run");
    assert!(
        output.status.success(),
        "improve-log writer should average stable samples: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let contents = std::fs::read_to_string(&log_path).expect("improve log should read");
    let rows = contents.lines().collect::<Vec<_>>();
    assert_eq!(rows.len(), 2);
    assert!(
        rows[1].contains("\"avg=8.527 samples=8.490;8.540;8.550 used=3/3\""),
        "small run average should be recorded with all stable samples: {}",
        rows[1]
    );
    assert!(
        rows[1].contains("\"avg=51.703 samples=51.210;51.610;52.290 used=3/4\""),
        "large run average should exclude the noisy sample: {}",
        rows[1]
    );
    let _ = std::fs::remove_file(&log_path);
}

#[test]
fn improve_log_writer_rejects_average_above_max() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = crate_root
        .parent()
        .and_then(std::path::Path::parent)
        .expect("workspace root should resolve");
    let script_path = workspace_root.join("scripts/append-improve-log.py");
    let log_path = workspace_root.join(format!(
        "temp/improve-log-max-average-{}.csv",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&log_path);

    let output = Command::new(&script_path)
        .arg("--path")
        .arg(&log_path)
        .arg("--commit")
        .arg("test")
        .arg("--small-runs")
        .arg("8.55,8.54,8.49")
        .arg("--small-max-avg-s")
        .arg("8.0")
        .arg("--summary")
        .arg("threshold guard")
        .output()
        .expect("improve-log writer should run");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let log_created = log_path.exists();
    let _ = std::fs::remove_file(&log_path);

    assert!(
        !success,
        "improve-log writer should reject averages above the configured max"
    );
    assert!(
        stderr.contains("small proof time: average 8.527s exceeds --small-max-avg-s 8.000s"),
        "max average rejection should explain the threshold: stderr={stderr}"
    );
    assert!(
        !log_created,
        "rejected average should not create an improve log"
    );
}

#[test]
fn improve_log_writer_rejects_path_outside_temp() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = crate_root
        .parent()
        .and_then(std::path::Path::parent)
        .expect("workspace root should resolve");
    let script_path = workspace_root.join("scripts/append-improve-log.py");
    let log_path = workspace_root.join(format!(
        "target/improve-log-outside-{}.csv",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&log_path);

    let output = Command::new(&script_path)
        .arg("--path")
        .arg(&log_path)
        .arg("--commit")
        .arg("test")
        .arg("--small-runs")
        .arg("8.55,8.54,8.49")
        .arg("--large-runs")
        .arg("52.29,51.61,51.21")
        .arg("--summary")
        .arg("path guard")
        .output()
        .expect("improve-log writer should run");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let log_created = log_path.exists();
    let _ = std::fs::remove_file(&log_path);

    assert!(
        !success,
        "improve-log writer should reject paths outside temp"
    );
    assert!(
        stderr.contains("--path must be under"),
        "path rejection should explain the temp boundary: stderr={stderr}"
    );
    assert!(!log_created, "rejected path should not create a log");
}

#[cfg(unix)]
#[test]
fn improve_log_writer_rejects_symlinked_log_path() {
    use std::os::unix::fs::symlink;

    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = crate_root
        .parent()
        .and_then(std::path::Path::parent)
        .expect("workspace root should resolve");
    let script_path = workspace_root.join("scripts/append-improve-log.py");
    let dir = workspace_root.join(format!("temp/improve-log-symlink-{}", std::process::id()));
    let log_path = dir.join("improve-log.csv");
    let redirected = dir.join("redirected.csv");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture dir should be created");
    std::fs::write(&redirected, "sentinel\n").expect("redirect target should write");
    symlink(&redirected, &log_path).expect("improve log symlink fixture should be created");

    let output = Command::new(&script_path)
        .arg("--path")
        .arg(&log_path)
        .arg("--commit")
        .arg("test")
        .arg("--small")
        .arg("1.0")
        .arg("--large")
        .arg("2.0")
        .arg("--summary")
        .arg("link guard")
        .output()
        .expect("improve-log writer should run");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let redirected_text =
        std::fs::read_to_string(&redirected).expect("redirect target should remain readable");
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        !success,
        "improve-log writer should reject symlinked log paths"
    );
    assert!(
        stderr.contains("--path must not be a symlink"),
        "symlink rejection should explain the path constraint: stderr={stderr}"
    );
    assert_eq!(
        redirected_text, "sentinel\n",
        "rejected improve-log append should not overwrite a symlink target"
    );
}

#[test]
fn improve_log_check_rejects_missing_log() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = crate_root
        .parent()
        .and_then(std::path::Path::parent)
        .expect("workspace root should resolve");
    let script_path = workspace_root.join("scripts/append-improve-log.py");
    let log_path = workspace_root.join(format!(
        "temp/improve-log-missing-check-{}.csv",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&log_path);

    let output = Command::new(&script_path)
        .arg("--path")
        .arg(&log_path)
        .arg("--check")
        .output()
        .expect("improve-log writer check should run");

    assert!(
        !output.status.success(),
        "improve-log check should reject a missing target log"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("improve log path does not exist"),
        "missing log rejection should explain the absent path: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn improve_log_writer_rejects_timing_log_outside_temp() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = crate_root
        .parent()
        .and_then(std::path::Path::parent)
        .expect("workspace root should resolve");
    let script_path = workspace_root.join("scripts/append-improve-log.py");
    let log_path = workspace_root.join(format!(
        "temp/improve-log-input-path-{}.csv",
        std::process::id()
    ));
    let outside_log = workspace_root.join(format!(
        "target/improve-log-input-outside-{}.log",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&log_path);
    let _ = std::fs::remove_file(&outside_log);
    std::fs::write(&outside_log, "timing_total_ms=8550\n")
        .expect("outside timing log should write");

    let output = Command::new(&script_path)
        .arg("--path")
        .arg(&log_path)
        .arg("--commit")
        .arg("test")
        .arg("--small-log")
        .arg(&outside_log)
        .arg("--large-runs")
        .arg("52.29,51.61,51.21")
        .arg("--summary")
        .arg("input path guard")
        .output()
        .expect("improve-log writer should run");
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let log_created = log_path.exists();
    let _ = std::fs::remove_file(&log_path);
    let _ = std::fs::remove_file(&outside_log);

    assert!(
        !success,
        "improve-log writer should reject timing logs outside temp"
    );
    assert!(
        stderr.contains("--small-log must be under"),
        "timing log rejection should explain the temp boundary: stderr={stderr}"
    );
    assert!(
        !log_created,
        "rejected timing log should not create an improve log"
    );
}

#[test]
fn improve_log_writer_rejects_unstable_run_samples() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = crate_root
        .parent()
        .and_then(std::path::Path::parent)
        .expect("workspace root should resolve");
    let script_path = workspace_root.join("scripts/append-improve-log.py");
    let log_path = workspace_root.join(format!(
        "temp/improve-log-unstable-runs-{}.csv",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&log_path);

    let output = Command::new(&script_path)
        .arg("--path")
        .arg(&log_path)
        .arg("--commit")
        .arg("test")
        .arg("--small-runs")
        .arg("8.0,11.0,14.0")
        .arg("--summary")
        .arg("unstable run samples")
        .output()
        .expect("improve-log writer should run");
    let _ = std::fs::remove_file(&log_path);

    assert!(
        !output.status.success(),
        "improve-log writer should reject unstable samples"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("no group of at least three runs"),
        "unstable sample rejection should explain the missing stable group: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn improve_log_writer_averages_timing_total_logs() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = crate_root
        .parent()
        .and_then(std::path::Path::parent)
        .expect("workspace root should resolve");
    let script_path = workspace_root.join("scripts/append-improve-log.py");
    let dir = workspace_root.join(format!(
        "temp/improve-log-timing-logs-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("timing log fixture dir should be created");
    let log_path = dir.join("improve-log.csv");
    let small_logs = write_timing_logs(&dir, "small", &[8550, 8540, 8490]);
    let large_logs = write_timing_logs(&dir, "large", &[52290, 51610, 51210, 90000]);

    let mut command = Command::new(&script_path);
    command
        .arg("--path")
        .arg(&log_path)
        .arg("--commit")
        .arg("test")
        .arg("--summary")
        .arg("timing log samples");
    for path in &small_logs {
        command.arg("--small-log").arg(path);
    }
    for path in &large_logs {
        command.arg("--large-log").arg(path);
    }
    let output = command
        .output()
        .expect("improve-log writer should run with timing logs");
    assert!(
        output.status.success(),
        "improve-log writer should average timing logs: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let contents = std::fs::read_to_string(&log_path).expect("improve log should read");
    let rows = contents.lines().collect::<Vec<_>>();
    assert_eq!(rows.len(), 2);
    assert!(
        rows[1].contains("\"avg=8.527 samples=8.490;8.540;8.550 used=3/3\""),
        "small timing logs should be converted from milliseconds to seconds: {}",
        rows[1]
    );
    assert!(
        rows[1].contains("\"avg=51.703 samples=51.210;51.610;52.290 used=3/4\""),
        "large timing logs should exclude the noisy timing log: {}",
        rows[1]
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn improve_log_writer_rejects_ambiguous_timing_logs() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = crate_root
        .parent()
        .and_then(std::path::Path::parent)
        .expect("workspace root should resolve");
    let script_path = workspace_root.join("scripts/append-improve-log.py");
    let dir = workspace_root.join(format!(
        "temp/improve-log-ambiguous-timing-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("timing log fixture dir should be created");
    let log_path = dir.join("improve-log.csv");
    let ambiguous_log = dir.join("ambiguous.log");
    std::fs::write(
        &ambiguous_log,
        "timing_total_ms=8000\ntiming_total_ms=8010\n",
    )
    .expect("ambiguous timing log should write");

    let output = Command::new(&script_path)
        .arg("--path")
        .arg(&log_path)
        .arg("--commit")
        .arg("test")
        .arg("--small-log")
        .arg(&ambiguous_log)
        .arg("--summary")
        .arg("ambiguous timing log")
        .output()
        .expect("improve-log writer should run");
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        !output.status.success(),
        "improve-log writer should reject ambiguous timing logs"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("expected exactly one timing_total_ms"),
        "ambiguous timing log rejection should explain the timing_total_ms count: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn write_timing_logs(
    dir: &std::path::Path,
    label: &str,
    timing_total_ms: &[u64],
) -> Vec<std::path::PathBuf> {
    timing_total_ms
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let path = dir.join(format!("{label}-{index}.log"));
            std::fs::write(
                &path,
                format!("setup_hash=fixture\ntiming_total_ms={value}\nproof=ok\n"),
            )
            .expect("timing log should write");
            path
        })
        .collect()
}
