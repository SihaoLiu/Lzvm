use std::path::Path;
use std::process::Command;

#[test]
fn ncu_cuda_kernel_summary_reports_duration_throughput_and_occupancy_limits() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/ncu-cuda-kernel-summary.py");
    let script_source =
        std::fs::read_to_string(&script_path).expect("ncu CUDA kernel summary source should read");

    for required in [
        "gpu__time_duration.sum",
        "sm__throughput.avg.pct_of_peak_sustained_elapsed",
        "gpu__dram_throughput.avg.pct_of_peak_sustained_elapsed",
        "launch__occupancy_limit_registers",
        "launch__registers_per_thread",
        "kernel_metric_summary",
        "occupancy_limits",
        "memory_bound_candidates",
    ] {
        assert!(
            script_source.contains(required),
            "ncu CUDA kernel summary should expose {required}"
        );
    }

    let output = Command::new("python3")
        .arg(&script_path)
        .arg("--self-test")
        .output()
        .expect("ncu CUDA kernel summary self-test should run");

    assert!(
        output.status.success(),
        "ncu CUDA kernel summary self-test should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    for required in [
        "ntt_stage_kernel",
        "poseidon2_width16_merkle_parent_kernel",
        "duration_ms",
        "sm_throughput_pct",
        "dram_throughput_pct",
        "registers_per_thread",
        "occupancy_limits",
        "memory_bound_candidates",
        "register_limited",
    ] {
        assert!(
            stdout.contains(required),
            "ncu CUDA kernel summary should print {required}"
        );
    }
}
