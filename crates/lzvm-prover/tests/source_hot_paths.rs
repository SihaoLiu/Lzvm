use std::path::Path;

#[test]
fn cuda_row_major_hashing_copies_validated_bytes_without_host_word_repacking() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_path = crate_root.join("src/merkle_hash.rs");
    let source = std::fs::read_to_string(&source_path).expect("Merkle hash source should read");

    let arity2 = function_body(
        &source,
        "fn cuda_linear_hashes_row_major_arity2",
        "fn cuda_linear_hashes_row_major_arity4",
    );
    let arity4 = function_body(
        &source,
        "fn cuda_linear_hashes_row_major_arity4",
        "type CudaPoseidon2LinearRoundOp",
    );

    for body in [arity2, arity4] {
        assert!(
            body.contains("copy_row_major_bytes_to_device"),
            "row-major CUDA hashing should copy validated bytes directly"
        );
        assert!(
            !body.contains("row_major_words_from_bytes"),
            "row-major CUDA hashing should avoid host-side word repacking"
        );
    }
}

#[test]
fn cuda_witness_leaf_extension_serializes_device_words_without_extended_felt_vector() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_path = crate_root.join("src/witness_commitment/extend.rs");
    let source =
        std::fs::read_to_string(&source_path).expect("witness extension source should read");

    let cuda_body = function_body(
        &source,
        "fn extend_witness_stage_row_major_bytes",
        "#[cfg(not(feature = \"cuda\"))]",
    );
    let validation_index = cuda_body
        .find("cuda_goldilocks_coset_extend_row_major_columns_output_bytes")
        .expect("CUDA witness leaf extension should validate domain shape");
    let setup_index = cuda_body
        .find("prepare_gpu_setup")
        .expect("CUDA witness leaf extension should initialize CUDA setup");
    let allocation_index = cuda_body
        .find("CudaDeviceBuffer::new")
        .expect("CUDA witness leaf extension should allocate device buffers");

    assert!(
        validation_index < setup_index,
        "CUDA witness leaf extension should validate domain shape before CUDA setup"
    );
    assert!(
        validation_index < allocation_index,
        "CUDA witness leaf extension should validate domain shape before device allocation"
    );
    assert!(
        cuda_body.contains("cuda_goldilocks_coset_extend_row_major_columns_device"),
        "CUDA witness leaf extension should write extended rows through device buffers"
    );
    assert!(
        !cuda_body.contains("cuda_goldilocks_coset_extend_row_major_columns("),
        "CUDA witness leaf extension should avoid the host-returning extension API"
    );
    assert!(
        !cuda_body.contains("Result<Vec<Felt>"),
        "CUDA witness leaf extension should not materialize extended Felt values"
    );
    assert!(
        !cuda_body.contains(".collect::<Result<Vec<_>, _>>()"),
        "CUDA witness leaf extension should serialize validated words directly"
    );
}

#[test]
fn cuda_fri_fixed_extension_uses_device_output_without_extended_word_vector() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_path = crate_root.join("src/prove_fri_polynomial.rs");
    let source = std::fs::read_to_string(&source_path).expect("FRI polynomial source should read");

    let cuda_body = function_body(&source, "fn extend_row_major_columns", "fn fri_error");
    let validation_index = cuda_body
        .find("cuda_goldilocks_coset_extend_row_major_columns_output_bytes")
        .expect("CUDA FRI fixed extension should validate domain shape");
    let setup_index = cuda_body
        .find("prepare_gpu_setup")
        .expect("CUDA FRI fixed extension should initialize CUDA setup");
    let allocation_index = cuda_body
        .find("CudaDeviceBuffer::new")
        .expect("CUDA FRI fixed extension should allocate device buffers");

    assert!(
        validation_index < setup_index,
        "CUDA FRI fixed extension should validate domain shape before CUDA setup"
    );
    assert!(
        validation_index < allocation_index,
        "CUDA FRI fixed extension should validate domain shape before device allocation"
    );
    assert!(
        cuda_body.contains("cuda_goldilocks_coset_extend_row_major_columns_device"),
        "CUDA FRI fixed extension should write extended rows through device buffers"
    );
    assert!(
        !cuda_body.contains("cuda_goldilocks_coset_extend_row_major_columns("),
        "CUDA FRI fixed extension should avoid the host-returning extension API"
    );
    assert!(
        !cuda_body.contains("collect::<Result<Vec<_>, _>>()"),
        "CUDA FRI fixed extension should avoid a separate extended word vector"
    );
}

#[test]
fn cuda_merkle_root_folds_on_device_without_host_level_loop() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_path = crate_root.join("src/merkle_hash.rs");
    let source = std::fs::read_to_string(&source_path).expect("Merkle hash source should read");

    let cuda_body = function_body(
        &source,
        "fn root_from_digest_level_on_cuda",
        "fn digest_level_as_state_words",
    );

    assert!(
        cuda_body.contains("cuda_poseidon2_width8_merkle_root_device"),
        "arity-2 CUDA root folding should use the native root operation"
    );
    assert!(
        cuda_body.contains("cuda_poseidon2_width16_merkle_root_device"),
        "arity-4 CUDA root folding should use the native root operation"
    );
    assert!(
        !cuda_body.contains("while state_count > 1"),
        "CUDA root folding should not loop over Merkle levels on the host"
    );
    assert!(
        !cuda_body.contains("CudaDeviceBuffer::new"),
        "CUDA root folding should not allocate a device buffer per Merkle level"
    );
}

#[test]
fn witness_merkle_tree_uses_device_parent_level_pipeline() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_path = crate_root.join("src/witness_commitment/tree.rs");
    let source = std::fs::read_to_string(&source_path).expect("witness tree source should read");

    let commit_body = function_body(
        &source,
        "fn commit_witness_stage_leaves",
        "fn open_witness_stage_commitment",
    );

    assert!(
        commit_body.contains("parent_levels_from_digest_level"),
        "witness Merkle tree construction should reuse the CUDA parent-level pipeline"
    );
    assert!(
        !commit_body.contains("parent_hashes(&level"),
        "witness Merkle tree construction should avoid re-uploading each parent level"
    );

    let merkle_source_path = crate_root.join("src/merkle_hash.rs");
    let merkle_source =
        std::fs::read_to_string(&merkle_source_path).expect("Merkle hash source should read");
    let cuda_body = function_body(
        &merkle_source,
        "fn parent_levels_from_device_buffer",
        "pub(crate) fn root_from_digest_level",
    );
    assert!(
        !cuda_body.contains("from_u64_words"),
        "CUDA parent-level pipeline should not upload each Merkle level"
    );
}

fn function_body<'a>(source: &'a str, start: &str, end: &str) -> &'a str {
    let body = source
        .split_once(start)
        .unwrap_or_else(|| panic!("missing function start: {start}"))
        .1;
    body.split_once(end)
        .unwrap_or_else(|| panic!("missing function end: {end}"))
        .0
}
