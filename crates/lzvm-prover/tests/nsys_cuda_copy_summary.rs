use std::path::Path;
use std::process::Command;

#[test]
fn nsys_cuda_copy_summary_reports_host_and_gpu_memcpy_waits() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/nsys-cuda-copy-summary.py");
    let script_source =
        std::fs::read_to_string(&script_path).expect("nsys CUDA copy summary source should read");

    for required in [
        "CUPTI_ACTIVITY_KIND_RUNTIME",
        "CUPTI_ACTIVITY_KIND_MEMCPY",
        "ENUM_CUDA_MEMCPY_OPER",
        "StringIds",
        "cudaMemcpy",
        "host_api_ms",
        "gpu_memcpy_ms",
        "wait_ratio",
        "OSRT_CALLCHAINS",
        "cuda_memcpy_callchain_hotspots",
        "CUPTI_ACTIVITY_KIND_KERNEL",
        "d2h_wait_preceding_kernel_hotspots",
        "previous_kernel",
        "--cudabacktrace=memory:80000",
        "app_frame",
    ] {
        assert!(
            script_source.contains(required),
            "nsys CUDA copy summary should expose {required}"
        );
    }

    let output = Command::new("python3")
        .arg(&script_path)
        .arg("--self-test")
        .output()
        .expect("nsys CUDA copy summary self-test should run");

    assert!(
        output.status.success(),
        "nsys CUDA copy summary self-test should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Device-to-Host")
            && stdout.contains("host_api_ms")
            && stdout.contains("gpu_memcpy_ms")
            && stdout.contains("wait_ratio")
            && stdout.contains("cuda_memcpy_callchain_hotspots")
            && stdout.contains("api,direction,bytes")
            && stdout.contains("cudaMemcpy_v3020,Device-to-Host")
            && stdout.contains("copy_root_to_host")
            && stdout.contains("extract_opening_rows")
            && stdout.contains("app_frame")
            && stdout.contains("d2h_wait_preceding_kernel_hotspots")
            && stdout.contains("previous_kernel")
            && stdout.contains("poseidon2_merkle_digest_parent_kernel")
            && stdout.contains("pack_row_major_columns_strided_kernel")
            && stdout.contains("cuda_api_backtrace_hint"),
        "nsys CUDA copy summary should print D2H host/GPU wait correlation"
    );
}
