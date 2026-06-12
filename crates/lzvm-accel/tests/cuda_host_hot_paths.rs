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
}

#[test]
fn cuda_host_exposes_device_memory_info() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_path = crate_root.join("native/cuda_host.cpp");
    let source = std::fs::read_to_string(&source_path).expect("cuda host source should read");

    assert!(
        source.contains("cudaMemGetInfo"),
        "CUDA host layer should expose device memory capacity for retention budgeting"
    );
}

#[test]
fn allocator_does_not_wait_to_reuse_pending_small_blocks() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_path = crate_root.join("native/cuda_host.cpp");
    let source = std::fs::read_to_string(&source_path).expect("cuda host source should read");
    let body = function_body(&source, "int alloc_bytes_impl", "void free_bytes_impl");

    assert!(
        source.contains("kPendingCacheNoWaitBytes"),
        "allocator should define a small pending-cache no-wait threshold"
    );
    assert!(
        body.contains("bytes <= pending_cache_no_wait_bytes(kPendingCacheNoWaitBytes)")
            && body.contains("pending_index = std::numeric_limits<std::size_t>::max()"),
        "small pending cached blocks should fall through to a fresh allocation instead of synchronizing"
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
