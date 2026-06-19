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
