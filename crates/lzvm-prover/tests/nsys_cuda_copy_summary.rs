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
            && stdout.contains("wait_ratio"),
        "nsys CUDA copy summary should print D2H host/GPU wait correlation"
    );
}
