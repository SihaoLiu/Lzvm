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
        "kernel_separation_candidates",
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
        "kernel_separation_candidates",
        "separation_hint",
        "split_or_reduce_register_pressure",
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
fn ncu_cuda_kernel_summary_allows_missing_auxiliary_occupancy_metrics() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = crate_root.join("../..");
    let script_path = workspace_root.join("scripts/ncu-cuda-kernel-summary.py");
    let temp_dir = workspace_root.join("temp");
    std::fs::create_dir_all(&temp_dir).expect("workspace temp directory should exist");

    let partial_csv = temp_file(&temp_dir, "ncu-prefix-partial-metrics.csv");
    std::fs::write(
        &partial_csv,
        concat!(
            "==PROF== Connected to process 1\n",
            "\"Kernel Name\",\"gpu__time_duration.sum\",",
            "\"sm__throughput.avg.pct_of_peak_sustained_elapsed\",",
            "\"gpu__dram_throughput.avg.pct_of_peak_sustained_elapsed\",",
            "\"gpu__compute_memory_throughput.avg.pct_of_peak_sustained_elapsed\",",
            "\"sm__issue_active.avg.pct_of_peak_sustained_elapsed\",",
            "\"launch__occupancy_limit_registers\",",
            "\"launch__occupancy_limit_shared_mem\",",
            "\"launch__occupancy_limit_blocks\",",
            "\"launch__registers_per_thread\",",
            "\"launch__shared_mem_per_block\"\n",
            "\"\",\"us\",\"%\",\"%\",\"%\",\"%\",\"block\",\"block\",\"block\",\"register/thread\",\"Kbyte/block\"\n",
            "\"ntt_stage_block_twiddle_kernel\",\"4500.0\",\"60.0\",\"58.0\",\"58.0\",\"53.0\",\"6\",\"14\",\"24\",\"38\",\"1.104\"\n",
        ),
    )
    .expect("partial NCU sample should write");
    let output = Command::new("python3")
        .arg(&script_path)
        .arg(&partial_csv)
        .output()
        .expect("ncu CUDA kernel summary should run on partial sample");
    let _ = std::fs::remove_file(&partial_csv);

    assert!(
        output.status.success(),
        "partial NCU CSV should parse with missing auxiliary metrics: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("ntt_stage_block_twiddle_kernel,1,4.500,4500.000"),
        "partial NCU CSV should still summarize duration: {stdout}"
    );
    assert!(
        stdout.contains("53.000,na,38.000"),
        "missing active warps should be printed as na: {stdout}"
    );
    assert!(
        stdout.contains("split_or_reduce_register_pressure"),
        "available occupancy limits should still drive separation hints: {stdout}"
    );
}

#[test]
fn ncu_cuda_kernel_summary_accepts_speed_of_light_launch_raw_without_duration() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = crate_root.join("../..");
    let script_path = workspace_root.join("scripts/ncu-cuda-kernel-summary.py");
    let temp_dir = workspace_root.join("temp");
    std::fs::create_dir_all(&temp_dir).expect("workspace temp directory should exist");

    let speed_of_light_csv = temp_file(&temp_dir, "ncu-speed-of-light-launch-raw.csv");
    std::fs::write(
        &speed_of_light_csv,
        concat!(
            "\"ID\",\"Kernel Name\",",
            "\"launch__occupancy_limit_blocks\",",
            "\"launch__occupancy_limit_registers\",",
            "\"launch__occupancy_limit_shared_mem\",",
            "\"launch__occupancy_limit_warps\",",
            "\"launch__registers_per_thread\",",
            "\"launch__shared_mem_per_block\"\n",
            "\"\",\"\",\"block\",\"block\",\"block\",\"block\",\"register/thread\",\"Kbyte/block\"\n",
            "\"0\",\"<unnamed>::ntt_stage_block_twiddle_kernel(unsigned long *, unsigned long, unsigned long, unsigned long, unsigned long, bool)\",\"24.0\",\"6.0\",\"14.0\",\"6.0\",\"38\",\"1.104\"\n",
        ),
    )
    .expect("speed-of-light NCU raw sample should write");
    let output = Command::new("python3")
        .arg(&script_path)
        .arg(&speed_of_light_csv)
        .output()
        .expect("ncu CUDA kernel summary should run on speed-of-light sample");
    let _ = std::fs::remove_file(&speed_of_light_csv);

    assert!(
        output.status.success(),
        "speed-of-light launch-only NCU CSV should parse: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("ntt_stage_block_twiddle_kernel,1,0.000,0.000"),
        "launch-only NCU CSV should still summarize the kernel with zero duration: {stdout}"
    );
    assert!(
        stdout.contains("6.000,14.000,6.000,24.000"),
        "launch-only NCU CSV should preserve occupancy limits: {stdout}"
    );
    assert!(
        stdout.contains("split_or_reduce_register_pressure"),
        "launch-only occupancy limits should still drive separation hints: {stdout}"
    );
    assert!(
        stdout.contains("metric_collection_quality"),
        "launch-only NCU CSV should expose metric collection quality: {stdout}"
    );
    assert!(
        stdout.contains("occupancy_only_missing_duration"),
        "launch-only NCU CSV should flag missing duration metrics before it is used as a throughput profile: {stdout}"
    );
    assert!(
        !stdout.contains("unsigned long"),
        "launch-only NCU CSV should print normalized short kernel names: {stdout}"
    );
}

#[test]
fn ncu_cuda_kernel_summary_accepts_command_line_metric_rows() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = crate_root.join("../..");
    let script_path = workspace_root.join("scripts/ncu-cuda-kernel-summary.py");
    let temp_dir = workspace_root.join("temp");
    std::fs::create_dir_all(&temp_dir).expect("workspace temp directory should exist");

    let metric_rows_csv = temp_file(&temp_dir, "ncu-command-line-metric-rows.csv");
    std::fs::write(
        &metric_rows_csv,
        concat!(
            "==PROF== Connected to process 1\n",
            "\"ID\",\"Process ID\",\"Process Name\",\"Host Name\",\"Kernel Name\",",
            "\"Context\",\"Stream\",\"Block Size\",\"Grid Size\",\"Device\",\"CC\",",
            "\"Metric Name\",\"Metric Unit\",\"Metric Value\"\n",
            "\"0\",\"1\",\"lzvm\",\"host\",",
            "\"<unnamed>::ntt_stage_kernel(unsigned long *, unsigned long, unsigned long, unsigned long, unsigned long, bool)\",",
            "\"1\",\"7\",\"(256, 1, 1)\",\"(16384, 1, 1)\",\"0\",\"12.0\",",
            "\"gpu__time_duration.sum\",\"ns\",\"4734624\"\n",
            "\"0\",\"1\",\"lzvm\",\"host\",",
            "\"<unnamed>::ntt_stage_kernel(unsigned long *, unsigned long, unsigned long, unsigned long, unsigned long, bool)\",",
            "\"1\",\"7\",\"(256, 1, 1)\",\"(16384, 1, 1)\",\"0\",\"12.0\",",
            "\"sm__throughput.avg.pct_of_peak_sustained_elapsed\",\"%\",\"3.96\"\n",
            "\"0\",\"1\",\"lzvm\",\"host\",",
            "\"<unnamed>::ntt_stage_kernel(unsigned long *, unsigned long, unsigned long, unsigned long, unsigned long, bool)\",",
            "\"1\",\"7\",\"(256, 1, 1)\",\"(16384, 1, 1)\",\"0\",\"12.0\",",
            "\"gpu__dram_throughput.avg.pct_of_peak_sustained_elapsed\",\"%\",\"34.82\"\n",
            "\"0\",\"1\",\"lzvm\",\"host\",",
            "\"<unnamed>::ntt_stage_kernel(unsigned long *, unsigned long, unsigned long, unsigned long, unsigned long, bool)\",",
            "\"1\",\"7\",\"(256, 1, 1)\",\"(16384, 1, 1)\",\"0\",\"12.0\",",
            "\"gpu__compute_memory_throughput.avg.pct_of_peak_sustained_elapsed\",\"%\",\"42.55\"\n",
            "\"0\",\"1\",\"lzvm\",\"host\",",
            "\"<unnamed>::ntt_stage_kernel(unsigned long *, unsigned long, unsigned long, unsigned long, unsigned long, bool)\",",
            "\"1\",\"7\",\"(256, 1, 1)\",\"(16384, 1, 1)\",\"0\",\"12.0\",",
            "\"sm__issue_active.avg.pct_of_peak_sustained_elapsed\",\"%\",\"0.39\"\n",
            "\"0\",\"1\",\"lzvm\",\"host\",",
            "\"<unnamed>::ntt_stage_kernel(unsigned long *, unsigned long, unsigned long, unsigned long, unsigned long, bool)\",",
            "\"1\",\"7\",\"(256, 1, 1)\",\"(16384, 1, 1)\",\"0\",\"12.0\",",
            "\"sm__warps_active.avg.pct_of_peak_sustained_active\",\"%\",\"88.44\"\n",
            "\"0\",\"1\",\"lzvm\",\"host\",",
            "\"<unnamed>::ntt_stage_kernel(unsigned long *, unsigned long, unsigned long, unsigned long, unsigned long, bool)\",",
            "\"1\",\"7\",\"(256, 1, 1)\",\"(16384, 1, 1)\",\"0\",\"12.0\",",
            "\"launch__occupancy_limit_registers\",\"block\",\"6\"\n",
            "\"0\",\"1\",\"lzvm\",\"host\",",
            "\"<unnamed>::ntt_stage_kernel(unsigned long *, unsigned long, unsigned long, unsigned long, unsigned long, bool)\",",
            "\"1\",\"7\",\"(256, 1, 1)\",\"(16384, 1, 1)\",\"0\",\"12.0\",",
            "\"launch__occupancy_limit_shared_mem\",\"block\",\"16\"\n",
            "\"0\",\"1\",\"lzvm\",\"host\",",
            "\"<unnamed>::ntt_stage_kernel(unsigned long *, unsigned long, unsigned long, unsigned long, unsigned long, bool)\",",
            "\"1\",\"7\",\"(256, 1, 1)\",\"(16384, 1, 1)\",\"0\",\"12.0\",",
            "\"launch__occupancy_limit_warps\",\"block\",\"6\"\n",
            "\"0\",\"1\",\"lzvm\",\"host\",",
            "\"<unnamed>::ntt_stage_kernel(unsigned long *, unsigned long, unsigned long, unsigned long, unsigned long, bool)\",",
            "\"1\",\"7\",\"(256, 1, 1)\",\"(16384, 1, 1)\",\"0\",\"12.0\",",
            "\"launch__occupancy_limit_blocks\",\"block\",\"24\"\n",
            "\"0\",\"1\",\"lzvm\",\"host\",",
            "\"<unnamed>::ntt_stage_kernel(unsigned long *, unsigned long, unsigned long, unsigned long, unsigned long, bool)\",",
            "\"1\",\"7\",\"(256, 1, 1)\",\"(16384, 1, 1)\",\"0\",\"12.0\",",
            "\"launch__registers_per_thread\",\"register/thread\",\"40\"\n",
            "\"0\",\"1\",\"lzvm\",\"host\",",
            "\"<unnamed>::ntt_stage_kernel(unsigned long *, unsigned long, unsigned long, unsigned long, unsigned long, bool)\",",
            "\"1\",\"7\",\"(256, 1, 1)\",\"(16384, 1, 1)\",\"0\",\"12.0\",",
            "\"launch__shared_mem_per_block\",\"byte/block\",\"1024\"\n",
        ),
    )
    .expect("command-line NCU metric-row sample should write");
    let output = Command::new("python3")
        .arg(&script_path)
        .arg(&metric_rows_csv)
        .output()
        .expect("ncu CUDA kernel summary should run on metric-row sample");
    let _ = std::fs::remove_file(&metric_rows_csv);

    assert!(
        output.status.success(),
        "command-line metric-row NCU CSV should parse: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("ntt_stage_kernel,1,4.735,4734.624"),
        "metric-row NCU CSV should summarize duration with ns conversion: {stdout}"
    );
    assert!(
        stdout.contains("40.000,1.000"),
        "metric-row NCU CSV should preserve registers and byte shared memory: {stdout}"
    );
}

#[test]
fn ncu_cuda_kernel_summary_flags_descriptor_expansion_shape_candidates() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = crate_root.join("../..");
    let script_path = workspace_root.join("scripts/ncu-cuda-kernel-summary.py");
    let temp_dir = workspace_root.join("temp");
    std::fs::create_dir_all(&temp_dir).expect("workspace temp directory should exist");

    let descriptor_csv = temp_file(&temp_dir, "ncu-descriptor-expansion-shape.csv");
    std::fs::write(
        &descriptor_csv,
        concat!(
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
            "\"expand_main_trace_descriptors_kernel\",\"4734.624\",\"3.90\",\"34.45\",\"42.55\",\"0.386\",\"88.44\",\"6\",\"16\",\"6\",\"24\",\"40\",\"1.000\"\n",
            "\"poseidon2_merkle_digest_parent_kernel\",\"2262.080\",\"35.0\",\"15.0\",\"18.0\",\"20.0\",\"42.0\",\"8\",\"12\",\"8\",\"24\",\"56\",\"2.000\"\n",
        ),
    )
    .expect("descriptor NCU sample should write");
    let output = Command::new("python3")
        .arg(&script_path)
        .arg(&descriptor_csv)
        .output()
        .expect("ncu CUDA kernel summary should run on descriptor sample");
    let _ = std::fs::remove_file(&descriptor_csv);

    assert!(
        output.status.success(),
        "descriptor NCU CSV should parse: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("descriptor_expansion_shape_candidates"),
        "descriptor expansion candidates should have a dedicated table: {stdout}"
    );
    assert!(
        stdout.contains("expand_main_trace_descriptors_kernel,1,4.735,34.450,3.900,0.386,40.000,redesign_descriptor_fields_before_kernel_split"),
        "low-issue descriptor expansion should be flagged with a representation-level hint: {stdout}"
    );
}

#[test]
fn ncu_cuda_kernel_summary_downgrades_tiny_register_limited_kernels() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = crate_root.join("../..");
    let script_path = workspace_root.join("scripts/ncu-cuda-kernel-summary.py");
    let temp_dir = workspace_root.join("temp");
    std::fs::create_dir_all(&temp_dir).expect("workspace temp directory should exist");

    let tiny_csv = temp_file(&temp_dir, "ncu-tiny-register-limited.csv");
    std::fs::write(
        &tiny_csv,
        concat!(
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
            "\"ntt_stage_block_twiddle_kernel\",\"34.432\",\"60.967\",\"55.769\",\"55.769\",\"53.829\",\"89.209\",\"6\",\"14\",\"6\",\"24\",\"38\",\"1.104\"\n",
        ),
    )
    .expect("tiny NCU sample should write");
    let output = Command::new("python3")
        .arg(&script_path)
        .arg(&tiny_csv)
        .output()
        .expect("ncu CUDA kernel summary should run on tiny kernel sample");
    let _ = std::fs::remove_file(&tiny_csv);

    assert!(
        output.status.success(),
        "tiny NCU CSV should parse: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("ntt_stage_block_twiddle_kernel,1,0.034,38.000,6.000,6.000,14.000,60.967,53.829,kernel_time_secondary"),
        "tiny register-limited kernels should not drive kernel splitting: {stdout}"
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
            "\"<unnamed>::ntt_stage_block_twiddle_kernel(unsigned long *, unsigned long, unsigned long, unsigned long, unsigned long, bool)\",\"45.0\",\"63.0\",\"55.0\",\"58.0\",\"40.0\",\"90.0\",\"6\",\"14\",\"6\",\"24\",\"38\",\"1.104\"\n",
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
        stdout.contains("profile=") && stdout.contains("ntt_stage_block_twiddle_kernel,1,"),
        "imported report should produce a kernel summary: {stdout}"
    );
    assert!(
        !stdout.contains("unsigned long"),
        "imported report should print normalized short kernel names: {stdout}"
    );
}

#[test]
fn ncu_cuda_kernel_summary_discovers_cuda_home_ncu_for_binary_reports() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = crate_root.join("../..");
    let script_path = workspace_root.join("scripts/ncu-cuda-kernel-summary.py");
    let temp_dir = workspace_root.join("temp");
    std::fs::create_dir_all(&temp_dir).expect("workspace temp directory should exist");

    let report_path = temp_file(&temp_dir, "ncu-cuda-home-import-sample.ncu-rep");
    std::fs::write(&report_path, [0xfe, 0x4e, 0x43, 0x55]).expect("fake NCU report should write");
    let fake_cuda_home = temp_dir.join(format!(
        "{}-{}-fake-cuda-home",
        std::process::id(),
        std::thread::current().name().unwrap_or("test")
    ));
    let fake_cuda_bin = fake_cuda_home.join("bin");
    std::fs::create_dir_all(&fake_cuda_bin).expect("fake CUDA bin should exist");
    let fake_ncu_path = fake_cuda_bin.join("ncu");
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
            "\"Kernel Name\",\"gpu__time_duration.sum\"\n",
            "\"\",\"us\"\n",
            "\"ntt_stage_kernel\",\"25.0\"\n",
            "CSV\n",
        ),
    )
    .expect("fake CUDA_HOME ncu should write");
    #[cfg(unix)]
    {
        let mut permissions = std::fs::metadata(&fake_ncu_path)
            .expect("fake CUDA_HOME ncu metadata should read")
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&fake_ncu_path, permissions)
            .expect("fake CUDA_HOME ncu should be executable");
    }

    let output = Command::new("python3")
        .arg(&script_path)
        .arg(&report_path)
        .env("CUDA_HOME", &fake_cuda_home)
        .env_remove("LZVM_NCU_COMMAND")
        .output()
        .expect("ncu CUDA kernel summary should run with CUDA_HOME discovery");
    let _ = std::fs::remove_file(&report_path);
    let _ = std::fs::remove_dir_all(&fake_cuda_home);

    assert!(
        output.status.success(),
        "CUDA_HOME ncu should be discovered for binary report imports: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("ntt_stage_kernel,1,0.025,25.000"),
        "CUDA_HOME-imported report should produce a kernel summary: {stdout}"
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
