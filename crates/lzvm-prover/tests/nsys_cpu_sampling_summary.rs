use std::path::Path;
use std::process::Command;

#[test]
fn nsys_cpu_sampling_summary_reports_application_hotspots() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = crate_root.join("../..");
    let script_path = workspace_root.join("scripts/nsys-cpu-sampling-summary.py");
    let script_source = std::fs::read_to_string(&script_path)
        .expect("nsys CPU sampling summary source should read");

    for required in [
        "SAMPLING_CALLCHAINS",
        "COMPOSITE_EVENTS",
        "ENUM_SAMPLING_THREAD_STATE",
        "ThreadNames",
        "top_cpu_self_samples",
        "application_cpu_hotspots",
        "cpu_sample_pct",
        "application_sample_pct",
    ] {
        assert!(
            script_source.contains(required),
            "nsys CPU sampling summary should expose {required}"
        );
    }

    let output = Command::new("python3")
        .arg(&script_path)
        .arg("--self-test")
        .output()
        .expect("nsys CPU sampling summary self-test should run");

    assert!(
        output.status.success(),
        "nsys CPU sampling summary self-test should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    for required in [
        "top_cpu_self_samples",
        "symbol,module,samples,cpu_sample_pct",
        "apply_zisk_main_lowered_report_row,lzvm,3,42.857",
        "advance_guest_machine_prepared_inner,lzvm,2,28.571",
        "application_cpu_hotspots",
        "symbol,module,samples,application_sample_pct",
        "apply_zisk_main_lowered_report_row,lzvm,3,60.000",
        "advance_guest_machine_prepared_inner,lzvm,2,40.000",
    ] {
        assert!(
            stdout.contains(required),
            "nsys CPU sampling summary should print {required}: {stdout}"
        );
    }
}
