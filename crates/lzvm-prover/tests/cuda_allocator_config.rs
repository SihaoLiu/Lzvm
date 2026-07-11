use std::path::Path;

fn function_body(source: &str, start: &str, end: &str) -> String {
    let start_index = source.find(start).expect("body start should exist");
    let rest = &source[start_index..];
    let end_index = rest.find(end).expect("body end should exist");
    rest[..end_index].to_owned()
}

#[test]
fn cuda_allocator_pending_no_wait_limit_is_runtime_configurable() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let native_source_path = crate_root.join("../lzvm-accel/native/cuda_host.cpp");
    let native_source =
        std::fs::read_to_string(&native_source_path).expect("CUDA host source should read");

    let alloc_body = function_body(
        &native_source,
        "int alloc_bytes_impl(void** out, std::size_t bytes)",
        "void free_bytes_impl(void* ptr)",
    );

    assert!(
        native_source.contains("kPendingCacheNoWaitBytes")
            && native_source.contains("std::size_t{512} << 20"),
        "CUDA allocator should keep the default pending no-wait limit at 512 MiB"
    );
    assert!(
        native_source.contains("LZVM_CUDA_PENDING_CACHE_NO_WAIT_BYTES")
            && native_source.contains("std::getenv(kPendingCacheNoWaitBytesEnv)")
            && native_source.contains("pending_cache_no_wait_bytes(std::size_t fallback)"),
        "CUDA allocator should expose a runtime pending no-wait limit"
    );
    assert!(
        alloc_body.contains("pending_cache_no_wait_bytes(kPendingCacheNoWaitBytes)")
            && !alloc_body.contains("bytes <= kPendingCacheNoWaitBytes"),
        "allocation should compare pending cache bytes against the runtime limit"
    );
}
