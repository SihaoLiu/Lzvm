#[test]
fn large_host_to_device_copies_use_page_locked_registration() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_path = crate_root.join("native/cuda_host.cpp");
    let source = std::fs::read_to_string(&source_path).expect("cuda host source should read");

    assert!(
        source.contains("cudaHostRegister"),
        "large H2D copies should register host pages before cudaMemcpy"
    );
    assert!(
        source.contains("cudaHostUnregister"),
        "large H2D copies should unregister temporary host page registration"
    );
    assert!(
        source.contains("LZVM_CUDA_H2D_PIN_THRESHOLD_BYTES")
            && source.contains("pinned_copy_threshold_bytes"),
        "large H2D page locking should have a tunable byte threshold"
    );
}

#[test]
fn stream_host_to_device_copies_defer_page_locked_registration_release() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let host_source = std::fs::read_to_string(crate_root.join("native/cuda_host.cpp"))
        .expect("cuda host source should read");
    let runtime_source = std::fs::read_to_string(crate_root.join("native/cuda_host_runtime.cpp"))
        .expect("cuda host runtime source should read");
    let header_source = std::fs::read_to_string(crate_root.join("native/cuda_host.hpp"))
        .expect("cuda host header should read");
    let stream_copy_body = function_body(
        &host_source,
        "extern \"C\" int lzvm_cuda_copy_h2d_bytes_on_stream",
        "extern \"C\" int lzvm_cuda_copy_d2h_bytes",
    );

    assert!(
        stream_copy_body.contains("register_large_host_copy")
            && stream_copy_body.contains("track_pending_host_copy_registration"),
        "stream H2D copies should keep large host ranges registered until stream completion"
    );
    assert!(
        host_source.contains("lzvm_cuda_reap_host_copy_registrations")
            && header_source.contains("lzvm_cuda_reap_host_copy_registrations")
            && runtime_source.contains("lzvm_cuda_reap_host_copy_registrations"),
        "CUDA stream/device synchronization should reap completed host copy registrations"
    );
}

#[test]
fn cuda_host_exposes_device_memory_info() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_path = crate_root.join("native/cuda_host_runtime.cpp");
    let source = std::fs::read_to_string(&source_path).expect("cuda host source should read");

    assert!(
        source.contains("cudaMemGetInfo"),
        "CUDA host layer should expose device memory capacity for retention budgeting"
    );
}

#[test]
fn cuda_host_exposes_async_device_to_pinned_host_copy() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let host_source = std::fs::read_to_string(crate_root.join("native/cuda_host.cpp"))
        .expect("cuda host source should read");
    let runtime_source = std::fs::read_to_string(crate_root.join("native/cuda_host_runtime.cpp"))
        .expect("cuda host runtime source should read");

    assert!(
        runtime_source.contains("cudaHostAlloc"),
        "D2H async copies need page-locked host allocation"
    );
    assert!(
        runtime_source.contains("cudaFreeHost"),
        "page-locked host allocation should have a matching release path"
    );
    assert!(
        host_source.contains("lzvm_cuda_copy_d2h_bytes_on_stream")
            && host_source.contains("cudaMemcpyAsync")
            && host_source.contains("cudaMemcpyDeviceToHost"),
        "D2H copies should be enqueueable on a CUDA stream"
    );
}

#[test]
fn cuda_host_exposes_graph_capture_replay_runtime() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let runtime_source = std::fs::read_to_string(crate_root.join("native/cuda_host_runtime.cpp"))
        .expect("cuda host runtime source should read");
    let header_source = std::fs::read_to_string(crate_root.join("native/cuda_host.hpp"))
        .expect("cuda host header should read");
    let rust_source = std::fs::read_to_string(crate_root.join("src/cuda_stream.rs"))
        .expect("CUDA stream source should read");

    for symbol in [
        "cudaStreamBeginCapture",
        "cudaStreamEndCapture",
        "cudaGraphInstantiate",
        "cudaGraphExecUpdate",
        "cudaGraphLaunch",
        "cudaGraphDestroy",
        "cudaGraphExecDestroy",
    ] {
        assert!(
            runtime_source.contains(symbol),
            "CUDA runtime layer should expose {symbol}"
        );
    }

    for symbol in [
        "lzvm_cuda_stream_begin_capture",
        "lzvm_cuda_stream_end_capture",
        "lzvm_cuda_graph_destroy",
        "lzvm_cuda_graph_instantiate",
        "lzvm_cuda_graph_exec_update",
        "lzvm_cuda_graph_exec_destroy",
        "lzvm_cuda_graph_launch",
    ] {
        assert!(
            header_source.contains(symbol) && rust_source.contains(symbol),
            "CUDA graph wrapper should bind {symbol}"
        );
    }

    assert!(
        rust_source.contains("CudaGraph") && rust_source.contains("CudaGraphExec"),
        "CUDA graph handles should have Rust RAII wrappers"
    );
}

#[test]
fn allocator_does_not_wait_to_reuse_pending_configured_blocks() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_path = crate_root.join("native/cuda_host.cpp");
    let source = std::fs::read_to_string(&source_path).expect("cuda host source should read");
    let body = function_body(&source, "int alloc_bytes_impl", "void free_bytes_impl");

    assert!(
        source.contains("kPendingCacheNoWaitBytes") && source.contains("std::size_t{512} << 20"),
        "allocator should define the pending-cache no-wait threshold"
    );
    assert!(
        body.contains("bytes <= pending_cache_no_wait_bytes(kPendingCacheNoWaitBytes)")
            && body.contains("pending_index = std::numeric_limits<std::size_t>::max()"),
        "configured pending cached blocks should fall through to a fresh allocation instead of synchronizing"
    );
}

fn function_body(source: &str, start_marker: &str, end_marker: &str) -> String {
    let start = source
        .find(start_marker)
        .unwrap_or_else(|| panic!("missing start marker {start_marker}"));
    let rest = &source[start..];
    let end = rest
        .find(end_marker)
        .unwrap_or_else(|| panic!("missing end marker {end_marker}"));
    rest[..end].to_owned()
}
