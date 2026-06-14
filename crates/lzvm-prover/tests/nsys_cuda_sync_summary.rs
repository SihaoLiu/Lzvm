use std::path::Path;
use std::process::Command;

#[test]
fn nsys_cuda_sync_summary_reports_explicit_host_sync_waits() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/nsys-cuda-sync-summary.py");
    let script_source =
        std::fs::read_to_string(&script_path).expect("nsys CUDA sync summary source should read");

    for required in [
        "CUPTI_ACTIVITY_KIND_RUNTIME",
        "StringIds",
        "cudaDeviceSynchronize",
        "cudaStreamSynchronize",
        "cudaEventSynchronize",
        "runtime_cuda_sync_api",
        "sync_wait_candidates",
    ] {
        assert!(
            script_source.contains(required),
            "nsys CUDA sync summary should expose {required}"
        );
    }

    let output = Command::new("python3")
        .arg(&script_path)
        .arg("--self-test")
        .output()
        .expect("nsys CUDA sync summary self-test should run");

    assert!(
        output.status.success(),
        "nsys CUDA sync summary self-test should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    for required in [
        "cudaDeviceSynchronize_v3020",
        "cudaStreamSynchronize_v3020",
        "cudaEventSynchronize_v3020",
        "host_api_ms",
        "avg_host_api_us",
        "return_code",
        "sync_wait_candidates",
    ] {
        assert!(
            stdout.contains(required),
            "nsys CUDA sync summary should print {required}"
        );
    }
}
