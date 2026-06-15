#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
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

#[test]
fn ncu_cuda_kernel_summary_skips_profiler_preamble_and_rejects_metricless_exports() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = crate_root.join("../..");
    let script_path = workspace_root.join("scripts/ncu-cuda-kernel-summary.py");
    let temp_dir = workspace_root.join("temp");
    std::fs::create_dir_all(&temp_dir).expect("workspace temp directory should exist");

    let valid_csv = temp_file(&temp_dir, "ncu-prefix-valid.csv");
    std::fs::write(
        &valid_csv,
        concat!(
            "==WARNING== profiler preamble\n",
            "==PROF== Connected to process 1\n",
            "\"Kernel Name\",\"gpu__time_duration.sum\",",
            "\"sm__throughput.avg.pct_of_peak_sustained_elapsed\",",
            "\"gpu__dram_throughput.avg.pct_of_peak_sustained_elapsed\",",
            "\"gpu__compute_memory_throughput.avg.pct_of_peak_sustained_elapsed\",",
            "\"sm__issue_active.avg.pct_of_peak_sustained_elapsed\",",
            "\"sm__warps_active.avg.pct_of_peak_sustained_active\",",
            "\"launch__occupancy_limit_registers\",",
            "\"launch__occupancy_limit_shared_mem\",",
            "\"launch__occupancy_limit_warps\",",
            "\"launch__occupancy_limit_blocks\",",
            "\"launch__registers_per_thread\",",
            "\"launch__shared_mem_per_block\"\n",
            "\"\",\"ns\",\"%\",\"%\",\"%\",\"%\",\"%\",\"block\",\"block\",\"block\",\"block\",\"register/thread\",\"byte/block\"\n",
            "\"poseidon2_merkle_digest_parent_kernel\",\"25000.0\",\"40.0\",\"18.0\",\"22.0\",\"31.0\",\"70.0\",\"8\",\"12\",\"8\",\"24\",\"56\",\"1024\"\n",
        ),
    )
    .expect("valid NCU sample should write");
    let output = Command::new("python3")
        .arg(&script_path)
        .arg(&valid_csv)
        .output()
        .expect("ncu CUDA kernel summary should run on prefixed sample");
    let _ = std::fs::remove_file(&valid_csv);
    assert!(
        output.status.success(),
        "prefixed NCU CSV should parse: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("poseidon2_merkle_digest_parent_kernel"),
        "prefixed NCU CSV should produce a kernel summary"
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("0.025,25.000"),
        "nanosecond duration should be normalized to milliseconds and microseconds"
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("56.000,1.000"),
        "byte shared memory should be normalized to KB"
    );

    let metricless_csv = temp_file(&temp_dir, "ncu-prefix-metricless.csv");
    std::fs::write(
        &metricless_csv,
        concat!(
            "==WARNING== No metrics to collect found in sections.\n",
            "==PROF== Connected to process 1\n",
            "\"ID\",\"Kernel Name\",\"launch__registers_per_thread\"\n",
            "\"0\",\"poseidon2_merkle_digest_parent_kernel\",\"56\"\n",
        ),
    )
    .expect("metricless NCU sample should write");
    let output = Command::new("python3")
        .arg(&script_path)
        .arg(&metricless_csv)
        .output()
        .expect("ncu CUDA kernel summary should run on metricless sample");
    let _ = std::fs::remove_file(&metricless_csv);
    assert!(
        !output.status.success(),
        "metricless NCU CSV should be rejected"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("missing required metric columns"),
        "metricless NCU CSV should report missing metrics: {stderr}"
    );
    assert!(
        stderr.contains("No metrics to collect"),
        "metricless NCU CSV should preserve the profiler warning: {stderr}"
    );
}

#[test]
fn ncu_cuda_kernel_summary_imports_binary_reports_through_ncu() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = crate_root.join("../..");
    let script_path = workspace_root.join("scripts/ncu-cuda-kernel-summary.py");
    let temp_dir = workspace_root.join("temp");
    std::fs::create_dir_all(&temp_dir).expect("workspace temp directory should exist");

    let report_path = temp_file(&temp_dir, "ncu-import-sample.ncu-rep");
    std::fs::write(&report_path, [0xfe, 0x4e, 0x43, 0x55]).expect("fake NCU report should write");
    let fake_ncu_path = temp_file(&temp_dir, "fake-ncu.sh");
    std::fs::write(
        &fake_ncu_path,
        concat!(
            "#!/usr/bin/env bash\n",
            "set -euo pipefail\n",
            "if [[ \"$1\" != \"--import\" ]]; then echo \"missing import\" >&2; exit 9; fi\n",
            "if [[ \"$2\" != *\".ncu-rep\" ]]; then echo \"missing report\" >&2; exit 9; fi\n",
            "if [[ \"$3\" != \"--csv\" ]]; then echo \"missing csv\" >&2; exit 9; fi\n",
            "if [[ \"$4\" != \"--page\" || \"$5\" != \"raw\" ]]; then echo \"missing raw page\" >&2; exit 9; fi\n",
            "cat <<'CSV'\n",
            "\"Kernel Name\",\"gpu__time_duration.sum\",",
            "\"sm__throughput.avg.pct_of_peak_sustained_elapsed\",",
            "\"gpu__dram_throughput.avg.pct_of_peak_sustained_elapsed\",",
            "\"gpu__compute_memory_throughput.avg.pct_of_peak_sustained_elapsed\",",
            "\"sm__issue_active.avg.pct_of_peak_sustained_elapsed\",",
            "\"sm__warps_active.avg.pct_of_peak_sustained_active\",",
            "\"launch__occupancy_limit_registers\",",
            "\"launch__occupancy_limit_shared_mem\",",
            "\"launch__occupancy_limit_warps\",",
            "\"launch__occupancy_limit_blocks\",",
            "\"launch__registers_per_thread\",",
            "\"launch__shared_mem_per_block\"\n",
            "\"\",\"us\",\"%\",\"%\",\"%\",\"%\",\"%\",\"block\",\"block\",\"block\",\"block\",\"register/thread\",\"Kbyte/block\"\n",
            "\"ntt_stage_block_twiddle_kernel\",\"45.0\",\"63.0\",\"55.0\",\"58.0\",\"40.0\",\"90.0\",\"6\",\"14\",\"6\",\"24\",\"38\",\"1.104\"\n",
            "CSV\n",
        ),
    )
    .expect("fake ncu should write");
    #[cfg(unix)]
    {
        let mut permissions = std::fs::metadata(&fake_ncu_path)
            .expect("fake ncu metadata should read")
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&fake_ncu_path, permissions)
            .expect("fake ncu should be executable");
    }

    let output = Command::new("python3")
        .arg(&script_path)
        .arg("--ncu-command")
        .arg(&fake_ncu_path)
        .arg(&report_path)
        .output()
        .expect("ncu CUDA kernel summary should run on fake report");
    let _ = std::fs::remove_file(&report_path);
    let _ = std::fs::remove_file(&fake_ncu_path);

    assert!(
        output.status.success(),
        "fake NCU report should import through ncu: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("profile=") && stdout.contains("ntt_stage_block_twiddle_kernel"),
        "imported report should produce a kernel summary: {stdout}"
    );
}

fn temp_file(temp_dir: &Path, name: &str) -> PathBuf {
    temp_dir.join(format!(
        "{}-{}-{}",
        std::process::id(),
        std::thread::current().name().unwrap_or("test"),
        name
    ))
}
