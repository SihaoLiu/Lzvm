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

    assert!(
        !cuda_body.contains("Result<Vec<Felt>"),
        "CUDA witness leaf extension should not materialize extended Felt values"
    );
    assert!(
        !cuda_body.contains(".collect::<Result<Vec<_>, _>>()"),
        "CUDA witness leaf extension should serialize validated words directly"
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
