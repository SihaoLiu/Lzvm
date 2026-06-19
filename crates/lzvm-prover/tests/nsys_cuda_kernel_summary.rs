use std::path::Path;
use std::process::Command;

#[test]
fn nsys_cuda_kernel_summary_reports_kernel_launch_and_stream_shape() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let script_path = crate_root.join("../../scripts/nsys-cuda-kernel-summary.py");
    let script_source =
        std::fs::read_to_string(&script_path).expect("nsys CUDA kernel summary source should read");

    for required in [
        "CUPTI_ACTIVITY_KIND_KERNEL",
        "CUPTI_ACTIVITY_KIND_RUNTIME",
        "CUPTI_ACTIVITY_KIND_MEMCPY",
        "ENUM_CUDA_MEMCPY_OPER",
        "StringIds",
        "cudaLaunchKernel",
        "cudaGraphInstantiate",
        "cudaGraphLaunch",
        "kernel_gpu_activity",
        "runtime_cuda_kernel_launch_api",
        "runtime_cuda_graph_api",
        "stream_kernel_activity",
        "stream_idle_gap_hotspots",
        "fusion_candidates",
        "graph_shape_candidates",
        "kernel_adjacency_candidates",
        "runtime_cuda_sync_api",
        "cuda_graph_fusion_separation_triage",
        "graph_fusion_priority_hint",
        "transfer_residency_hint",
        "top_d2h_wait",
    ] {
        assert!(
            script_source.contains(required),
            "nsys CUDA kernel summary should expose {required}"
        );
    }

    let output = Command::new("python3")
        .arg(&script_path)
        .arg("--self-test")
        .output()
        .expect("nsys CUDA kernel summary self-test should run");

    assert!(
        output.status.success(),
        "nsys CUDA kernel summary self-test should pass: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    for required in [
        "ntt_stage_kernel",
        "poseidon2_width16_merkle_parent_kernel",
        "cudaLaunchKernel_v7000",
        "cudaGraphInstantiate_v10000",
        "cudaGraphLaunch_v10000",
        "kernel_gpu_ms",
        "launch_api_ms",
        "graph_api_ms",
        "avg_kernel_us",
        "launch_to_kernel_ratio",
        "runtime_cuda_graph_api",
        "stream_kernel_activity",
        "stream_idle_gap_hotspots",
        "idle_gap_ms",
        "max_idle_gap_ms",
        "fusion_candidates",
        "graph_shape_candidates",
        "grid_x,block_x,streams",
        "ntt_stage_kernel,64,256,1,2",
        "kernel_adjacency_candidates",
        "previous_kernel,next_kernel",
        "ntt_stage_kernel,ntt_stage_kernel,2,0.210,0.170",
        "runtime_cuda_sync_api",
        "cudaDeviceSynchronize_v3020",
        "cuda_graph_fusion_separation_triage",
        "dominant_wait",
        "graph_or_fusion_upper_bound_ms",
        "graph_api_present",
        "graph_launch_calls",
        "sync_to_launch_ratio",
        "next_action_hint",
        "graph_fusion_priority_hint",
        "defer_graph_or_fusion_until_stream_idle_is_explained",
        "top_stream_occupancy_ratio",
        "top_stream_idle_ms",
        "launch_to_top_stream_idle_ratio",
        "top_graph_shape",
        "top_same_stream_pair",
        "transfer_residency_hint",
        "batch_or_keep_small_d2h_on_device",
        "top_d2h_wait,1152",
        "inspect_stream_idle_or_cpu_producer",
    ] {
        assert!(
            stdout.contains(required),
            "nsys CUDA kernel summary should print {required}"
        );
    }
}

#[test]
fn nsys_cuda_kernel_summary_defers_graph_when_stream_idle_exceeds_launch_upper_bound() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = crate_root.join("../..");
    let script_path = workspace_root.join("scripts/nsys-cuda-kernel-summary.py");
    let temp_dir = workspace_root.join("temp");
    std::fs::create_dir_all(&temp_dir).expect("workspace temp directory should exist");
    let profile_path = temp_dir.join(format!(
        "nsys-kernel-idle-priority-{}.sqlite",
        std::process::id()
    ));

    let fixture = r#"
import sqlite3
import sys

conn = sqlite3.connect(sys.argv[1])
conn.executescript("""
create table StringIds (id integer primary key, value text);
create table CUPTI_ACTIVITY_KIND_RUNTIME (
    start integer,
    end integer,
    correlationId integer,
    nameId integer,
    returnValue integer
);
create table CUPTI_ACTIVITY_KIND_KERNEL (
    start integer,
    end integer,
    streamId integer,
    correlationId integer,
    shortName integer,
    demangledName integer,
    gridX integer,
    blockX integer
);
""")
conn.executemany(
    "insert into StringIds (id, value) values (?, ?)",
    [
        (1, "cudaLaunchKernel_v7000"),
        (2, "cudaDeviceSynchronize_v3020"),
        (3, "ntt_stage_block_twiddle_kernel"),
    ],
)
runtime_rows = []
kernel_rows = []
start = 200_000_000
for index in range(8):
    correlation = 100 + index
    runtime_rows.append((start - 110_000_000, start, correlation, 1, 0))
    kernel_rows.append((start, start + 425_000_000, 7, correlation, 3, 3, 8192, 256))
    start += 825_000_000
runtime_rows.append((7_000_000_000, 7_100_000_000, 0, 2, 0))
conn.executemany(
    "insert into CUPTI_ACTIVITY_KIND_RUNTIME (start, end, correlationId, nameId, returnValue) values (?, ?, ?, ?, ?)",
    runtime_rows,
)
conn.executemany(
    "insert into CUPTI_ACTIVITY_KIND_KERNEL (start, end, streamId, correlationId, shortName, demangledName, gridX, blockX) values (?, ?, ?, ?, ?, ?, ?, ?)",
    kernel_rows,
)
conn.commit()
"#;

    let build_output = Command::new("python3")
        .arg("-c")
        .arg(fixture)
        .arg(&profile_path)
        .output()
        .expect("idle-priority nsys fixture should build");
    assert!(
        build_output.status.success(),
        "idle-priority nsys fixture should build: stderr={}",
        String::from_utf8_lossy(&build_output.stderr)
    );

    let output = Command::new("python3")
        .arg(&script_path)
        .arg(&profile_path)
        .output()
        .expect("nsys CUDA kernel summary should run on idle-priority fixture");
    let _ = std::fs::remove_file(&profile_path);

    assert!(
        output.status.success(),
        "idle-priority fixture should summarize: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(
            "next_action_hint,inspect_stream_idle_or_cpu_producer,top kernel stream is idle for more than a quarter of its active window"
        ),
        "idle-priority fixture should direct attention to stream idle: {stdout}"
    );
    assert!(
        stdout.contains(
            "graph_fusion_priority_hint,defer_graph_or_fusion_until_stream_idle_is_explained"
        ),
        "idle-priority fixture should not prioritize Graph or fusion when stream idle exceeds the launch upper bound: {stdout}"
    );
    assert!(
        !stdout.contains("graph_fusion_priority_hint,measure_cuda_graph_or_kernel_fusion"),
        "idle-priority fixture should not recommend Graph or fusion as the next priority: {stdout}"
    );
}
