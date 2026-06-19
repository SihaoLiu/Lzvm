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
        "hot_libc_nearest_application_callers",
        "nearest_app_symbol",
        "hot_libc_application_callchains",
        "application_callchain",
        "cpu_trace_memcpy_action_hints",
        "trace_report_storage_structural_candidate",
        "--top",
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
        "hot_libc_nearest_application_callers",
        "libc_symbol,nearest_app_symbol,nearest_app_module,samples,libc_sample_pct",
        "__memcpy_avx512_unaligned_erms,run_guest_pc_trace_segment_slice,lzvm,2,100.000",
        "hot_libc_application_callchains",
        "libc_symbol,application_callchain,samples,libc_sample_pct",
        "__memcpy_avx512_unaligned_erms,run_guest_pc_trace_segment_slice <= produce_guest_pc_trace_pending_slices,2,100.000",
        "cpu_trace_memcpy_action_hints",
        "nearest_app_symbol,samples,libc_sample_pct,action_hint",
        "run_guest_pc_trace_segment_slice,2,100.000,trace_report_storage_structural_candidate",
    ] {
        assert!(
            stdout.contains(required),
            "nsys CPU sampling summary should print {required}: {stdout}"
        );
    }

    let top_output = Command::new("python3")
        .arg(&script_path)
        .arg("--self-test")
        .arg("--top")
        .arg("3")
        .output()
        .expect("nsys CPU sampling summary --top self-test should run");
    assert!(
        top_output.status.success(),
        "nsys CPU sampling summary --top self-test should pass: stderr={}",
        String::from_utf8_lossy(&top_output.stderr)
    );
}
