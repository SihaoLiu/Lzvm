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
