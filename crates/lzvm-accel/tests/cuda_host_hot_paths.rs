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
