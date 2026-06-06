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
fn cuda_row_major_hashing_downloads_digest_prefixes() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_path = crate_root.join("src/merkle_hash.rs");
    let source = std::fs::read_to_string(&source_path).expect("Merkle hash source should read");

    let body = function_body(
        &source,
        "fn cuda_linear_hashes_with_row_major_device_rounds",
        "fn push_felt_words",
    );

    assert!(
        body.contains("to_state_prefix_u64_words(row_count, width, HASH_WORDS)"),
        "row-major CUDA leaf hashing should download digest prefixes"
    );
    assert!(
        !body.contains("to_u64_words()"),
        "row-major CUDA leaf hashing should avoid full state downloads"
    );
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
    assert!(
        cuda_body.contains("Felt::as_u64_slice"),
        "CUDA witness leaf extension should upload Felt words without staging a byte vector"
    );
    assert!(
        !cuda_body.contains("row_major_felt_bytes(values"),
        "CUDA witness leaf extension should avoid per-call Felt-to-byte staging"
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
    assert!(
        cuda_body.contains("Felt::as_u64_slice"),
        "CUDA FRI fixed extension should upload Felt words without staging a byte vector"
    );
    assert!(
        !cuda_body.contains("row_major_felt_bytes(values"),
        "CUDA FRI fixed extension should avoid per-call Felt-to-byte staging"
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

#[test]
fn cuda_stage_source_cache_can_rebuild_sparse_trace_on_device() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_path = crate_root.join("src/witness_execution.rs");
    let source =
        std::fs::read_to_string(&source_path).expect("witness execution source should read");

    let upload_body = function_body(
        &source,
        "fn upload_from_trace_or_preloaded_if_empty",
        "fn upload_from_trace_if_empty",
    );

    let sparse_index = upload_body
        .find("upload_from_trace_sparse_if_profitable_if_empty")
        .expect("CUDA stage source upload should try sparse device reconstruction");
    let full_index = upload_body
        .find("upload_from_trace_if_empty")
        .expect("CUDA stage source upload should keep the full upload fallback");

    assert!(
        sparse_index < full_index,
        "sparse CUDA trace reconstruction should be attempted before full trace upload"
    );
    assert!(
        source.contains("CudaDeviceBuffer::from_sparse_u64_words"),
        "sparse CUDA trace reconstruction should avoid full host trace upload"
    );
}

#[test]
fn witness_commitment_segments_use_logical_tree_byte_count() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_path = crate_root.join("src/witness_commitment/segment.rs");
    let source = std::fs::read_to_string(&source_path).expect("witness segment source should read");

    assert!(
        source.contains("commitment.tree_byte_count()"),
        "witness commitment segments should not materialize tree bytes to read their length"
    );
    assert!(
        !source.contains("commitment.tree_bytes().len()"),
        "witness commitment segments should use logical tree byte counts"
    );
}

#[test]
fn cli_witness_summary_uses_logical_tree_byte_count() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_path = crate_root.join("../lzvm-cli/src/prove_witness.rs");
    let source = std::fs::read_to_string(&source_path).expect("prove witness source should read");

    let body = function_body(
        &source,
        "fn write_witness_output_summary_with_trace",
        "fn finish_all_units_witness_run",
    );

    assert!(
        body.contains("commitment.tree_byte_count()"),
        "witness output summary should use the logical tree byte count"
    );
    assert!(
        !body.contains("commitment.tree_bytes().len()"),
        "witness output summary should not force tree byte materialization"
    );
}

#[test]
fn witness_opening_reads_through_commitment_accessors() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_path = crate_root.join("src/witness_commitment/tree.rs");
    let source = std::fs::read_to_string(&source_path).expect("witness tree source should read");

    let body = function_body(
        &source,
        "pub fn open_witness_stage_commitment",
        "pub fn decode_witness_stage_leaf_values",
    );

    assert!(
        body.contains("commitment.read_opening_values"),
        "witness openings should let the commitment choose how leaf rows are read"
    );
    assert!(
        body.contains("commitment.read_digest_at"),
        "witness openings should let the commitment choose how sibling digests are read"
    );
    assert!(
        !body.contains("commitment.tree_bytes()"),
        "witness openings should not force full tree byte materialization"
    );
}

#[test]
fn witness_stage_commitments_use_internal_tree_storage() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_path = crate_root.join("src/witness_commitment/values.rs");
    let source = std::fs::read_to_string(&source_path).expect("witness values source should read");

    let commitment_body = function_body(
        &source,
        "pub struct WitnessStageCommitment",
        "impl WitnessStageCommitment",
    );

    assert!(
        source.contains("enum WitnessStageTreeStorage"),
        "witness stage commitments should have an internal tree storage abstraction"
    );
    assert!(
        commitment_body.contains("tree: WitnessStageTreeStorage"),
        "witness stage commitments should store tree data through the abstraction"
    );
    assert!(
        !commitment_body.contains("tree_bytes: Vec<u8>"),
        "witness stage commitments should not hard-code host tree bytes in the main struct"
    );
}

#[test]
fn cuda_compact_witness_opening_uses_direct_row_extension() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_path = crate_root.join("src/witness_commitment/values.rs");
    let source = std::fs::read_to_string(&source_path).expect("witness values source should read");

    let body = function_body(
        &source,
        "fn extended_row_values_cuda",
        "fn extended_leaf_bytes",
    );

    assert!(
        body.contains("cuda_goldilocks_coset_extend_row_major_columns_row_device"),
        "CUDA compact witness opening should extend only the requested row on device"
    );
    assert!(
        !body.contains("extended_rows_device()?"),
        "CUDA compact witness opening should not materialize full row-major extension for one row"
    );
}

#[test]
fn cuda_compact_witness_commit_reuses_device_leaf_hash_level() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let trace_path = crate_root.join("src/witness_commitment/trace.rs");
    let trace_source =
        std::fs::read_to_string(&trace_path).expect("witness trace source should read");

    let compact_body = function_body(
        &trace_source,
        "fn commit_extended_witness_stage",
        "fn record_optional_duration",
    );

    assert!(
        compact_body.contains("compact_witness_stage_leaf_hash_level_with_source_device_timing"),
        "compact witness commits should keep leaf hash states on device"
    );
    assert!(
        compact_body.contains("commit_witness_stage_leaves_compact_with_leaf_hash_level"),
        "compact witness commits should pass device leaf hash states into tree construction"
    );
    assert!(
        !compact_body.contains("let leaf_hashes"),
        "compact witness commits should not force host leaf hash vectors before tree construction"
    );

    let tree_path = crate_root.join("src/witness_commitment/tree.rs");
    let tree_source = std::fs::read_to_string(&tree_path).expect("witness tree source should read");
    let commit_body = function_body(
        &tree_source,
        "pub(crate) fn commit_witness_stage_leaves_compact_with_leaf_hash_level",
        "fn validate_witness_stage_leaves",
    );

    assert!(
        commit_body.contains("leaf_level.root()"),
        "device leaf hash commits should derive the Merkle root from the existing device level"
    );
    assert!(
        !commit_body.contains("state_buffer_from_digest_level"),
        "device leaf hash commits should not re-upload digest prefixes as padded states"
    );
}

#[test]
fn cuda_merkle_opening_uses_bounded_device_path_primitive() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let merkle_path = crate_root.join("src/merkle_hash.rs");
    let merkle_source =
        std::fs::read_to_string(&merkle_path).expect("Merkle hash source should read");

    let opening_body = function_body(
        &merkle_source,
        "pub(crate) fn opening_path",
        "pub(crate) fn linear_hash",
    );

    assert!(
        opening_body.contains("cuda_poseidon2_width16_merkle_opening_path_device")
            && opening_body.contains("cuda_poseidon2_width8_merkle_opening_path_device"),
        "CUDA Merkle openings should gather siblings and root with bounded device primitives"
    );
    assert!(
        !opening_body.contains("read_device_state_digest("),
        "CUDA Merkle openings should not perform per-level device-to-host digest reads"
    );
}

#[test]
fn cuda_compact_witness_commit_defers_canonical_check_synchronization_until_root_read() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let extend_path = crate_root.join("src/witness_commitment/extend.rs");
    let extend_source =
        std::fs::read_to_string(&extend_path).expect("witness extension source should read");
    let tree_path = crate_root.join("src/witness_commitment/tree.rs");
    let tree_source = std::fs::read_to_string(&tree_path).expect("witness tree source should read");

    assert!(
        extend_source.contains("struct PendingCanonicalCudaDigestLevel"),
        "compact CUDA leaf levels should carry a pending canonical check with the digest level"
    );

    let source_device_body = function_body(
        &extend_source,
        "fn compact_witness_stage_leaf_hash_level_from_source_device_timed",
        "fn validate_source_device_buffer",
    );
    assert!(
        source_device_body.contains("begin_validate_row_major_device_words"),
        "source-device compact leaf hashing should launch canonical validation without synchronizing"
    );
    assert!(
        !source_device_body.contains("        validate_row_major_device_words"),
        "source-device compact leaf hashing should not synchronize before hashing extended rows"
    );

    let device_commit_body = function_body(
        &tree_source,
        "pub(crate) fn commit_witness_stage_device_compact_with_leaf_hash_level",
        "fn validate_witness_stage_leaves",
    );
    assert!(
        device_commit_body.contains("leaf_level.root()")
            && device_commit_body.contains("leaf_level.finish_canonical_check()"),
        "device compact commitment should finish the pending canonical check after the root read synchronization"
    );
}

#[test]
fn cuda_compact_witness_commit_defers_digest_tree_until_opening() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let tree_path = crate_root.join("src/witness_commitment/tree.rs");
    let tree_source = std::fs::read_to_string(&tree_path).expect("witness tree source should read");
    let values_path = crate_root.join("src/witness_commitment/values.rs");
    let values_source =
        std::fs::read_to_string(&values_path).expect("witness values source should read");

    let commit_body = function_body(
        &tree_source,
        "pub(crate) fn commit_witness_stage_leaves_compact_with_leaf_hash_level",
        "fn validate_witness_stage_leaves",
    );
    assert!(
        commit_body.contains("leaf_level.root()"),
        "device leaf hash commits should derive the root without downloading the full digest tree"
    );
    assert!(
        commit_body.contains("digest_tree: None"),
        "device leaf hash commits should defer host digest tree materialization"
    );
    assert!(
        !commit_body.contains("append_digest_tree_bytes_from_device_level"),
        "device leaf hash commits should not eagerly download every Merkle parent level"
    );

    let opening_body = function_body(
        &tree_source,
        "pub fn open_witness_stage_commitment",
        "pub fn decode_witness_stage_leaf_values",
    );
    assert!(
        opening_body.contains("commitment.open_compact_on_demand"),
        "witness openings should let compact CUDA storage build only the queried path"
    );

    let storage_body = function_body(
        &values_source,
        "impl WitnessStageCompactTreeStorage",
        "fn extended_row_values",
    );
    assert!(
        storage_body.contains("open_on_demand_cuda"),
        "compact CUDA storage should have an on-demand opening path"
    );
    assert!(
        storage_body.contains("digest_tree.is_none()"),
        "on-demand opening should only apply when no host digest tree is stored"
    );
}

#[test]
fn cuda_narrow_witness_commit_uses_compact_device_leaf_level() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let trace_path = crate_root.join("src/witness_commitment/trace.rs");
    let trace_source =
        std::fs::read_to_string(&trace_path).expect("witness trace source should read");
    let extend_path = crate_root.join("src/witness_commitment/extend.rs");
    let extend_source =
        std::fs::read_to_string(&extend_path).expect("witness extension source should read");
    let merkle_path = crate_root.join("src/merkle_hash.rs");
    let merkle_source =
        std::fs::read_to_string(&merkle_path).expect("Merkle hash source should read");

    let commit_body = function_body(
        &trace_source,
        "fn commit_extended_witness_stage",
        "fn record_optional_duration",
    );
    assert!(
        !commit_body.contains("if stage.column_count() > super::HASH_WORDS"),
        "CUDA witness stages should not restrict compact device commits to wide rows"
    );
    assert!(
        !commit_body.contains("commit_witness_stage_leaves_owned_with_leaf_hashes"),
        "CUDA narrow witness stages should avoid host-owned tree construction"
    );

    let leaf_level_body = function_body(
        &extend_source,
        "fn compact_witness_stage_leaf_hash_level_timed",
        "fn extended_row_count_from_bytes",
    );
    assert!(
        leaf_level_body.contains("linear_hash_level_from_validated_row_major_device_buffer"),
        "compact CUDA leaf levels should support narrow and wide rows from device memory"
    );
    assert!(
        !leaf_level_body.contains("column_count <= HASH_WORDS"),
        "compact CUDA leaf levels should not reject narrow witness rows"
    );

    assert!(
        merkle_source.contains("from_device_state_prefix_u64_words"),
        "narrow CUDA row digests should expand row prefixes into padded device states"
    );
}

#[test]
fn cuda_compact_opening_reuses_retained_stage_source_device_buffer() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let values_path = crate_root.join("src/witness_commitment/values.rs");
    let values_source =
        std::fs::read_to_string(&values_path).expect("witness values source should read");
    let tree_path = crate_root.join("src/witness_commitment/tree.rs");
    let tree_source = std::fs::read_to_string(&tree_path).expect("witness tree source should read");
    let execution_path = crate_root.join("src/witness_execution.rs");
    let execution_source =
        std::fs::read_to_string(&execution_path).expect("witness execution source should read");

    let compact_storage_body = function_body(
        &values_source,
        "struct WitnessStageCompactTreeStorage",
        "impl Clone for WitnessStageCompactTreeStorage",
    );
    assert!(
        compact_storage_body.contains("retained_source_device"),
        "compact CUDA storage should be able to keep an already uploaded stage source buffer"
    );

    let open_on_demand_body = function_body(
        &values_source,
        "fn open_on_demand_cuda",
        "fn extended_rows_device",
    );
    assert!(
        open_on_demand_body.contains("self.source_device_buffer(source_device)"),
        "compact CUDA openings should accept retained trace source device buffers before uploading host values"
    );
    assert!(
        values_source.contains("source_device: Option<&WitnessStageSourceDeviceView>"),
        "compact CUDA openings should expose a source device view hook for trace-output retained buffers"
    );
    assert!(
        values_source.contains("cuda_goldilocks_coset_extend_row_major_columns_strided_row_device"),
        "compact CUDA openings should extend only the requested row from retained strided source buffers"
    );
    assert!(
        values_source.contains("cuda_memory_info")
            && values_source.contains("RETAINED_SOURCE_DEVICE_RESERVE_BYTES"),
        "retained source budgeting should size itself from CUDA device memory unless overridden"
    );
    assert!(
        values_source.contains("RETAINED_SOURCE_DEVICE_REGISTRY")
            && values_source.contains("fn reserve_retained_device_buffer")
            && values_source.contains("fn buffer_key(&self)"),
        "retained source budgeting should de-duplicate multiple stage views that share one CUDA buffer"
    );
    assert!(
        values_source.contains("fn retained_byte_len(&self) -> usize")
            && values_source.contains("self.buffer().len()"),
        "retained source budgeting should account for the full retained CUDA buffer"
    );

    let device_commit_body = function_body(
        &tree_source,
        "pub(crate) fn commit_witness_stage_leaves_compact_with_leaf_hash_level",
        "fn validate_witness_stage_leaves",
    );
    assert!(
        device_commit_body.contains("retained_source_device"),
        "device compact commitments should receive the retained source buffer from the trace source cache"
    );

    let source_cache_body = function_body(
        &execution_source,
        "struct WitnessStageSourceDeviceCache",
        "impl WitnessStageSourceDeviceCache",
    );
    assert!(
        source_cache_body.contains("Arc<CudaDeviceBuffer>"),
        "stage source cache should share uploaded buffers with compact commitments without copying"
    );
    assert!(
        execution_source.contains("fn retain_fri_stage_source_devices() -> bool")
            && execution_source.contains("Ok(\"0\") | Ok(\"false\") | Ok(\"no\")"),
        "CUDA stage source retention should default on and remain explicitly disableable"
    );
}

#[test]
fn cuda_fri_polynomial_reuses_fixed_source_device_cache() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let polynomial_path = crate_root.join("src/prove_fri_polynomial.rs");
    let polynomial_source =
        std::fs::read_to_string(&polynomial_path).expect("FRI polynomial source should read");
    let opening_path = crate_root.join("src/prove_fri_opening.rs");
    let opening_source =
        std::fs::read_to_string(&opening_path).expect("FRI opening source should read");

    assert!(
        polynomial_source.contains("struct PcsFriFixedColumnsCache"),
        "FRI polynomial construction should expose a fixed-column cache"
    );
    assert!(
        polynomial_source.contains("source_device: Option<CudaDeviceBuffer>"),
        "CUDA FRI fixed extension should cache the uploaded fixed source buffer"
    );
    assert!(
        polynomial_source
            .contains("build_pcs_fri_polynomial_values_with_slices_stage_sources_and_fixed_cache"),
        "FRI polynomial construction should accept a shared fixed-column cache"
    );
    assert!(
        opening_source.contains("build_pcs_fri_transcript_values_from_trace_refs_with_fixed_cache"),
        "FRI transcript building should share the fixed-column cache across trace values"
    );
    assert!(
        opening_source.contains("let mut fixed_columns_cache = PcsFriFixedColumnsCache::default()"),
        "FRI transcript segment building should keep one fixed-column cache for the whole segment batch"
    );
}

#[test]
fn cuda_retained_source_compact_commit_avoids_host_source_clone() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let tree_path = crate_root.join("src/witness_commitment/tree.rs");
    let tree_source = std::fs::read_to_string(&tree_path).expect("witness tree source should read");
    let values_path = crate_root.join("src/witness_commitment/values.rs");
    let values_source =
        std::fs::read_to_string(&values_path).expect("witness values source should read");

    let device_commit_body = function_body(
        &tree_source,
        "pub(crate) fn commit_witness_stage_leaves_compact_with_leaf_hash_level",
        "fn validate_witness_stage_leaves",
    );
    assert!(
        device_commit_body.contains("retained_source_device.as_ref().is_some"),
        "retained-source compact commits should branch on retained device availability"
    );
    assert!(
        device_commit_body.contains("Vec::new()"),
        "retained-source compact commits should avoid cloning full host source values"
    );

    let source_buffer_body = function_body(
        &values_source,
        "fn source_device_buffer",
        "fn extended_row_values_cuda",
    );
    assert!(
        source_buffer_body.contains("retained_source_device"),
        "source device lookup should accept retained buffers before requiring host source values"
    );
}

#[test]
fn cuda_guest_pc_trace_reuses_repeated_small_stage_commitments() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let trace_path = crate_root.join("src/witness_commitment/trace.rs");
    let trace_source =
        std::fs::read_to_string(&trace_path).expect("witness trace source should read");
    let execution_path = crate_root.join("src/witness_execution.rs");
    let execution_source =
        std::fs::read_to_string(&execution_path).expect("witness execution source should read");

    assert!(
        trace_source.contains("WitnessStageCommitmentReuseCache"),
        "CUDA stage commitment code should expose a content-checked reuse cache"
    );
    assert!(
        trace_source.contains("MAX_REUSE_SOURCE_BYTES"),
        "stage reuse should stay bounded and avoid comparing very large stage sources"
    );
    assert!(
        trace_source.contains("source_values == stage.values()"),
        "stage reuse should confirm source equality instead of trusting roots or fingerprints"
    );
    assert!(
        execution_source.contains("stage_commitment_reuse_cache"),
        "guest PC trace streaming should keep the reuse cache across trace segments"
    );
}

#[test]
fn cuda_guest_pc_device_material_marks_stage_commitments_external() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let execution_path = crate_root.join("src/witness_execution.rs");
    let execution_source =
        std::fs::read_to_string(&execution_path).expect("witness execution source should read");
    let body = function_body(
        &execution_source,
        "fn run_prove_witness_commitments_from_trace_inner",
        "stage_source_device_cache.upload_from_trace_or_preloaded_if_empty",
    );

    assert!(
        body.contains("guest_pc_device_segment_material.is_some()"),
        "guest PC device material should be enough to mark stage commitments external-source-backed"
    );
    assert!(
        !body.contains("trace_ref.is_none() && guest_pc_device_segment_material.is_some()"),
        "host trace availability should not force retained full-trace CUDA source buffers"
    );
}

#[test]
fn cuda_guest_pc_device_material_pipeline_is_default_enabled() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let backend_path = crate_root.join("src/guest_pc_trace_backend.rs");
    let backend_source =
        std::fs::read_to_string(&backend_path).expect("guest PC backend source should read");
    let execution_path = crate_root.join("src/witness_execution.rs");
    let execution_source =
        std::fs::read_to_string(&execution_path).expect("witness execution source should read");

    let device_source_body = function_body(
        &backend_source,
        "fn guest_pc_device_trace_source_enabled",
        "fn guest_pc_device_trace_source_deep_validation_enabled",
    );
    let segment_output_body = function_body(
        &backend_source,
        "fn guest_pc_trace_less_segment_output_enabled",
        "fn build_layout_zisk_main_trace_segment",
    );
    let commitment_input_body = function_body(
        &execution_source,
        "fn guest_pc_trace_less_commitment_input_enabled",
        "#[cfg(not(feature = \"cuda\"))]",
    );

    for body in [
        device_source_body,
        segment_output_body,
        commitment_input_body,
    ] {
        assert!(
            body.contains(".unwrap_or(true)"),
            "CUDA guest PC device-material path should be on by default"
        );
        assert!(
            body.contains("\"0\"") && body.contains("\"false\"") && body.contains("\"no\""),
            "CUDA guest PC device-material path should keep explicit off values"
        );
    }
}

#[test]
fn cuda_guest_pc_trace_uses_device_backed_stage_sources() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let execution_path = crate_root.join("src/witness_execution.rs");
    let execution_source =
        std::fs::read_to_string(&execution_path).expect("witness execution source should read");
    let trace_path = crate_root.join("src/witness_commitment/trace.rs");
    let trace_source =
        std::fs::read_to_string(&trace_path).expect("witness trace source should read");

    assert!(
        !execution_source.contains("from_device_row_major_u64_slice"),
        "guest PC trace should avoid device-to-device stage source slicing"
    );
    assert!(
        execution_source.contains("from_row_major_column_window"),
        "guest PC trace should describe stage sources as column windows over a full trace device upload"
    );
    assert!(
        execution_source.contains("try_evaluate_regular_constraints_cuda_base"),
        "regular constraints should be able to consume device-backed stage sources before host extraction"
    );
    assert!(
        execution_source.contains("commit_witness_stage_source_devices_and_indexed_timing"),
        "stage commitments should be able to consume device-backed stage sources"
    );
    assert!(
        trace_source
            .contains("compact_witness_stage_leaf_hash_level_from_source_device_view_timing"),
        "device-backed stage commitment should hash leaves from the retained source view"
    );
}

#[test]
fn guest_pc_trace_commitment_input_accepts_preloaded_stage_source_devices() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let execution_path = crate_root.join("src/witness_execution.rs");
    let execution_source =
        std::fs::read_to_string(&execution_path).expect("witness execution source should read");

    let input_body = function_body(
        &execution_source,
        "struct WitnessTraceCommitmentInput",
        "struct ProveWitnessTraceRunObservers",
    );
    assert!(
        input_body.contains("stage_source_devices"),
        "witness trace commitment input should allow guest trace streaming to pass preloaded CUDA stage sources"
    );

    let run_body = function_body(
        &execution_source,
        "fn run_prove_witness_commitments_from_trace_inner",
        "fn retain_fri_stage_source_devices",
    );
    assert!(
        run_body.contains("upload_from_trace_or_preloaded_if_empty"),
        "commitment execution should prefer preloaded stage source devices before uploading the full host trace"
    );
    assert!(
        !run_body.contains("stage_source_device_cache.upload_from_trace_if_empty(&layout, &trace)"),
        "guest PC trace commitment should not force a full-trace H2D upload when source devices are already available"
    );
}

#[test]
fn guest_pc_trace_segments_pass_terminal_prefix_rows_to_cuda_source_upload() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let backend_path = crate_root.join("src/guest_pc_trace_backend.rs");
    let backend_source =
        std::fs::read_to_string(&backend_path).expect("guest PC trace backend source should read");
    let execution_path = crate_root.join("src/witness_execution.rs");
    let execution_source =
        std::fs::read_to_string(&execution_path).expect("witness execution source should read");

    assert!(
        backend_source.contains("trace_source_prefix_rows"),
        "guest PC trace segment output should expose how many rows came from real reports before terminal padding"
    );
    assert!(
        execution_source.contains("terminal_trace_source_prefix_rows"),
        "witness execution should carry terminal prefix rows into CUDA source upload planning"
    );
    assert!(
        execution_source.contains("upload_from_trace_prefix_and_terminal_fill_if_empty"),
        "CUDA source upload should be able to avoid H2D for terminal padding rows"
    );
}

#[test]
fn guest_pc_trace_segments_have_device_trace_builder_ingress() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let backend_path = crate_root.join("src/guest_pc_trace_backend.rs");
    let backend_source =
        std::fs::read_to_string(&backend_path).expect("guest PC trace backend source should read");
    let execution_path = crate_root.join("src/witness_execution.rs");
    let execution_source =
        std::fs::read_to_string(&execution_path).expect("witness execution source should read");

    assert!(
        backend_source.contains("struct GuestPcTraceDeviceTraceBuilder"),
        "guest PC trace lowering should expose a device trace builder for CUDA source buffers"
    );
    assert!(
        backend_source.contains("build_guest_pc_trace_stage_source_devices"),
        "guest PC trace lowering should be able to build CUDA stage source devices before commitment"
    );
    assert!(
        backend_source.contains("validate_guest_pc_trace_device_source_matches_trace"),
        "device-built trace sources need an explicit host-trace equivalence check before use"
    );

    let commit_body = function_body(
        &execution_source,
        "fn run_prove_witness_commitments_with_guest_pc_trace_segment_commitments_inner",
        "fn merge_backend_unit_values",
    );
    assert!(
        commit_body.contains("build_preloaded_guest_pc_trace_stage_source_devices"),
        "segmented guest PC commitments should try the CUDA device trace builder through the shared preloaded source helper"
    );
    assert!(
        commit_body.contains("stage_source_devices: preloaded_stage_source_devices"),
        "segmented guest PC commitments should pass preloaded CUDA stage sources into commitment input"
    );
}

#[test]
fn guest_pc_trace_segments_build_device_trace_from_compact_descriptors() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let backend_path = crate_root.join("src/guest_pc_trace_backend.rs");
    let backend_source =
        std::fs::read_to_string(&backend_path).expect("guest PC trace backend source should read");
    let execution_path = crate_root.join("src/witness_execution.rs");
    let execution_source =
        std::fs::read_to_string(&execution_path).expect("witness execution source should read");

    assert!(
        backend_source.contains("struct ZiskMainDeviceTraceDescriptors"),
        "guest PC trace lowering should expose compact CUDA trace descriptors"
    );
    assert!(
        backend_source.contains("append_zisk_main_device_trace_descriptor"),
        "guest PC trace lowering should produce descriptors while applying each report"
    );
    assert!(
        backend_source.contains("CudaDeviceBuffer::from_zisk_main_trace_descriptors"),
        "guest PC CUDA source builder should expand compact descriptors directly on device"
    );
    assert!(
        !backend_source.contains("trace.values()).collect"),
        "descriptor source building should not scan the completed host trace"
    );
    assert!(
        execution_source.contains("device_trace_descriptors()"),
        "guest PC segmented commitments should pass compact descriptors into the CUDA source builder"
    );
}

#[test]
fn guest_pc_trace_device_descriptors_pack_kind_fields_into_control_word() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let backend_path = crate_root.join("src/guest_pc_trace_backend.rs");
    let backend_source =
        std::fs::read_to_string(&backend_path).expect("guest PC trace backend source should read");
    let cuda_path = crate_root.join("../lzvm-accel/native/cuda_zisk_main_trace.cuh");
    let cuda_source =
        std::fs::read_to_string(&cuda_path).expect("CUDA trace descriptor source should read");

    assert!(
        backend_source.contains("const ZISK_MAIN_DEVICE_TRACE_DESCRIPTOR_WORDS: usize = 11")
            && backend_source
                .contains("const ZISK_MAIN_DEVICE_TRACE_WIDE_DESCRIPTOR_WORDS: usize = 14"),
        "guest PC trace descriptors should prefer an 11-word packed format with a 14-word fallback"
    );
    let append_body = function_body(
        &backend_source,
        "fn append_zisk_main_device_trace_descriptor",
        "fn zisk_main_device_trace_source_descriptor",
    );
    assert!(
        append_body.contains("ZISK_MAIN_DEVICE_TRACE_A_KIND_SHIFT")
            && append_body.contains("ZISK_MAIN_DEVICE_TRACE_B_KIND_SHIFT")
            && append_body.contains("ZISK_MAIN_DEVICE_TRACE_STORE_KIND_SHIFT"),
        "descriptor kind fields should be packed into the control word"
    );
    assert!(
        !append_body.contains("a_kind,\n        a_payload")
            && !append_body.contains("b_kind,\n        b_payload")
            && !append_body.contains("store_kind,\n        store_payload"),
        "descriptor words should not carry standalone kind slots"
    );
    assert!(
        cuda_source.contains("constexpr size_t kZiskMainCompactDescriptorWords = 11")
            && cuda_source.contains("constexpr size_t kZiskMainWideDescriptorWords = 14"),
        "CUDA descriptor expansion must support both compact and fallback descriptor widths"
    );
}

#[test]
fn zisk_main_descriptor_expansion_writes_rows_without_full_zero_prefill() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let cuda_path = crate_root.join("../lzvm-accel/native/cuda_zisk_main_trace.cuh");
    let cuda_source =
        std::fs::read_to_string(&cuda_path).expect("CUDA trace descriptor source should read");

    let body = function_body(
        &cuda_source,
        "__global__ void expand_zisk_main_trace_descriptors_kernel",
        "extern \"C\" int lzvm_cuda_expand_zisk_main_trace_descriptors",
    );
    assert!(
        !body.contains("for (size_t column = 0; column < kZiskMainTraceColumns; ++column)"),
        "descriptor expansion should not prefill every row with zero before writing known columns"
    );
    assert!(
        body.contains("row[38] = 0"),
        "descriptor expansion should still explicitly bind the unused trace column to zero"
    );
}

#[test]
fn trace_less_guest_pc_opening_reuses_retained_device_descriptors() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let backend_path = crate_root.join("src/guest_pc_trace_backend.rs");
    let backend_source =
        std::fs::read_to_string(&backend_path).expect("guest PC trace backend source should read");
    let execution_path = crate_root.join("src/witness_execution.rs");
    let execution_source =
        std::fs::read_to_string(&execution_path).expect("witness execution source should read");
    let opening_path = crate_root.join("src/witness_opening.rs");
    let opening_source =
        std::fs::read_to_string(&opening_path).expect("witness opening source should read");
    let accel_path = crate_root.join("../lzvm-accel/src/cuda_buffer.rs");
    let accel_source =
        std::fs::read_to_string(&accel_path).expect("CUDA buffer source should read");

    assert!(
        accel_source.contains("from_zisk_main_trace_descriptors_device"),
        "CUDA descriptor expansion should accept an already-uploaded descriptor buffer"
    );
    assert!(
        backend_source.contains("device_trace_descriptor_buffer")
            && backend_source.contains("from_zisk_main_trace_descriptors_device"),
        "guest PC device trace builders should retain uploaded descriptor buffers for later source rebuilds"
    );
    let cache_body = function_body(
        &execution_source,
        "struct WitnessStageSourceDeviceCache",
        "fn upload_from_trace_sparse_if_profitable_if_empty",
    );
    assert!(
        cache_body.contains("guest_pc_device_descriptor_buffer"),
        "preloaded guest PC source caches should preserve descriptor device buffers"
    );
    assert!(
        execution_source.contains("guest_pc_device_descriptor_buffer"),
        "trace outputs should carry retained descriptor device buffers across commitment and opening"
    );
    assert!(
        opening_source.contains("guest_pc_device_descriptor_buffer")
            && opening_source.contains("build_guest_pc_trace_stage_source_devices_from_device_descriptors"),
        "guest PC openings should rebuild external sources from retained device descriptors before falling back to host descriptors"
    );
}

#[test]
fn lean_eth_block_public_input_binding_tracks_runtime_checks() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_path = crate_root.join("../../lean/Lzvm/EthBlockPublicInputBinding.lean");
    let lean_source =
        std::fs::read_to_string(&lean_path).expect("Lean ETH binding source should read");
    let lean_root_path = crate_root.join("../../lean/Lzvm.lean");
    let lean_root_source =
        std::fs::read_to_string(&lean_root_path).expect("Lean root source should read");
    let artifact_path = crate_root.join("src/proof_artifact.rs");
    let artifact_source =
        std::fs::read_to_string(&artifact_path).expect("proof artifact source should read");

    assert!(
        lean_root_source.contains("import Lzvm.EthBlockPublicInputBinding"),
        "top-level Lean module should include the ETH block public-input binding model"
    );
    assert!(
        lean_source.contains("structure RuntimeEthBlockPublicInputBindingValidation")
            && lean_source
                .contains("runtime_eth_block_public_input_binding_checked_acceptance_sound"),
        "Lean should expose a checked ETH public-input binding soundness theorem"
    );
    assert!(
        artifact_source.contains("fn validate_eth_block_binding")
            && artifact_source.contains("public_values_hash")
            && artifact_source.contains("validate_proof_bindings"),
        "Rust proof artifact construction should keep runtime public-input binding checks"
    );
}

#[test]
fn lean_challenge_segment_binding_tracks_runtime_transcript_checks() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_path = crate_root.join("../../lean/Lzvm/ChallengeSegmentBinding.lean");
    let lean_source =
        std::fs::read_to_string(&lean_path).expect("Lean challenge binding source should read");
    let lean_root_path = crate_root.join("../../lean/Lzvm.lean");
    let lean_root_source =
        std::fs::read_to_string(&lean_root_path).expect("Lean root source should read");
    let artifact_path = crate_root.join("src/proof_artifact.rs");
    let artifact_source =
        std::fs::read_to_string(&artifact_path).expect("proof artifact source should read");

    assert!(
        lean_root_source.contains("import Lzvm.ChallengeSegmentBinding"),
        "top-level Lean module should include the challenge segment binding model"
    );
    assert!(
        lean_source.contains("structure RuntimeChallengeSegmentBindingValidation")
            && lean_source.contains("runtime_challenge_segment_binding_checked_acceptance_sound"),
        "Lean should expose a checked challenge segment binding soundness theorem"
    );
    assert!(
        artifact_source.contains("fn validate_contribution_proof_challenge_values")
            && artifact_source.contains("derive_global_challenge_from_proof_segments")
            && artifact_source.contains("CHALLENGE_VALUES_SEGMENT_ID")
            && artifact_source.contains("parse_challenge_values_segment"),
        "Rust proof artifact validation should recompute and check challenge segment bindings"
    );
}

#[test]
fn external_source_device_commitments_do_not_retain_full_trace_sources() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let trace_path = crate_root.join("src/witness_commitment/trace.rs");
    let trace_source =
        std::fs::read_to_string(&trace_path).expect("witness commitment trace source should read");

    let body = function_body(
        &trace_source,
        "fn commit_extended_witness_stage_source_device",
        "fn commit_extended_witness_stage",
    );
    assert!(
        body.contains("let retained_source_device = if external_source_required")
            && body.contains("None")
            && body.contains("Some(source_view.clone())"),
        "external-source commitments should not consume retention budget with full trace source buffers"
    );
    assert!(
        body.contains("retained_source_device,"),
        "device compact commitments should receive the explicit retention decision"
    );
}

#[test]
fn compact_cuda_opening_uses_unsynced_source_extension_before_checked_gpu_work() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let accel_path = crate_root.join("../lzvm-accel/src/lib.rs");
    let accel_source = std::fs::read_to_string(&accel_path).expect("lzvm-accel source should read");
    let cuda_path = crate_root.join("../lzvm-accel/native/cuda_field.cu");
    let cuda_source = std::fs::read_to_string(&cuda_path).expect("CUDA field source should read");
    let values_path = crate_root.join("src/witness_commitment/values.rs");
    let values_source = std::fs::read_to_string(&values_path)
        .expect("witness commitment values source should read");

    assert!(
        accel_source.contains("cuda_goldilocks_coset_extend_row_major_columns_device_unsynced")
            && accel_source
                .contains("cuda_goldilocks_coset_extend_row_major_columns_strided_device_unsynced"),
        "lzvm-accel should expose unsynced device coset extension for internally chained GPU work"
    );
    assert!(
        cuda_source.contains("lzvm_cuda_goldilocks_coset_extend_row_major_columns_device_unsynced")
            && cuda_source.contains(
                "lzvm_cuda_goldilocks_coset_extend_row_major_columns_strided_device_unsynced"
            ),
        "native CUDA should provide no-final-sync device row-major extension entry points"
    );
    let extend_body = function_body(
        &values_source,
        "fn extend_source_device_buffer_cuda",
        "fn extended_row_values_cuda",
    );
    assert!(
        extend_body.contains("cuda_goldilocks_coset_extend_row_major_columns_device_unsynced")
            && extend_body
                .contains("cuda_goldilocks_coset_extend_row_major_columns_strided_device_unsynced"),
        "compact opening should defer row-major extension synchronization to the following checked CUDA copy or hash operation"
    );
}

#[test]
fn guest_pc_trace_device_material_builder_does_not_construct_host_trace() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let backend_path = crate_root.join("src/guest_pc_trace_backend.rs");
    let backend_source =
        std::fs::read_to_string(&backend_path).expect("guest PC trace backend source should read");

    assert!(
        backend_source.contains("struct GuestPcTraceDeviceSegmentMaterial"),
        "guest PC trace lowering should have an explicit device-backed segment material"
    );
    let body = function_body(
        &backend_source,
        "fn build_layout_zisk_main_trace_segment_device_material",
        "fn build_layout_zisk_main_trace_segment",
    );
    assert!(
        body.contains("validate_and_apply_zisk_main_report"),
        "device material should keep the same Zisk Main validation and state transition path"
    );
    assert!(
        body.contains("append_zisk_main_device_trace_descriptor"),
        "device material should build compact CUDA descriptors while validating reports"
    );
    assert!(
        body.contains("zisk_main_unit_values"),
        "device material should still produce unit values for public binding"
    );
    assert!(
        !body.contains("trace_builder()")
            && !body.contains("write_zisk_main_row_columns")
            && !body.contains("write_zisk_main_terminal_row")
            && !body.contains("builder.build()"),
        "device material should not allocate or fill a full host trace"
    );
}

#[test]
fn guest_pc_trace_device_material_builds_stage_sources_without_host_trace() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let backend_path = crate_root.join("src/guest_pc_trace_backend.rs");
    let backend_source =
        std::fs::read_to_string(&backend_path).expect("guest PC trace backend source should read");

    let body = function_body(
        &backend_source,
        "fn build_guest_pc_trace_stage_source_devices_from_device_material",
        "fn build_guest_pc_trace_stage_source_devices_from_device_descriptors",
    );
    assert!(
        body.contains("CudaDeviceBuffer::from_u64_words")
            && body.contains("build_guest_pc_trace_stage_source_devices_from_device_descriptors"),
        "device material should upload descriptors once and delegate expansion to the device-descriptor helper"
    );
    let device_body = function_body(
        &backend_source,
        "fn build_guest_pc_trace_stage_source_devices_from_device_descriptors_timing",
        "fn build_guest_pc_trace_stage_source_devices(\n",
    );
    assert!(
        device_body.contains("CudaDeviceBuffer::from_zisk_main_trace_descriptors_device"),
        "device descriptor material should expand descriptors into a CUDA trace buffer without host reupload"
    );
    assert!(
        device_body.contains("guest_pc_device_trace_builder_from_layout"),
        "device material should derive stage windows from layout metadata"
    );
    assert!(
        !device_body.contains("WitnessTraceBuffer")
            && !device_body.contains("validate_guest_pc_trace_device_source_matches_trace")
            && !device_body.contains("trace.values()"),
        "device material stage-source construction should not require host trace values"
    );
}

#[test]
fn witness_execution_prefers_guest_pc_device_material_before_host_trace_source() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let execution_path = crate_root.join("src/witness_execution.rs");
    let execution_source =
        std::fs::read_to_string(&execution_path).expect("witness execution source should read");

    let helper_body = function_body(
        &execution_source,
        "fn build_preloaded_guest_pc_trace_stage_source_devices",
        "fn run_prove_witness_commitments_with_guest_pc_trace_segments_inner",
    );
    let material_index = helper_body
        .find("segment_output.device_segment_material()")
        .expect("preloaded source helper should inspect device material");
    let host_trace_index = helper_body
        .find("segment_output.trace_if_available()")
        .expect("preloaded source helper should keep the host trace fallback");
    assert!(
        material_index < host_trace_index,
        "device material should be tried before host trace based source construction"
    );
    assert!(
        helper_body.contains("build_guest_pc_trace_stage_source_devices_from_device_material"),
        "preloaded source helper should build CUDA stage sources directly from device material"
    );

    let segment_body = function_body(
        &execution_source,
        "fn run_prove_witness_commitments_with_guest_pc_trace_segment_commitments_inner",
        "fn merge_backend_unit_values",
    );
    assert!(
        segment_body.contains("build_preloaded_guest_pc_trace_stage_source_devices"),
        "segmented guest PC commitment path should use the shared preloaded source helper"
    );
}

#[test]
fn guest_pc_segment_output_keeps_host_trace_optional_for_device_material_path() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let backend_path = crate_root.join("src/guest_pc_trace_backend.rs");
    let backend_source =
        std::fs::read_to_string(&backend_path).expect("guest PC trace backend source should read");
    let execution_path = crate_root.join("src/witness_execution.rs");
    let execution_source =
        std::fs::read_to_string(&execution_path).expect("witness execution source should read");

    let segment_output_fields = function_body(
        &backend_source,
        "pub struct GuestPcTraceSegmentRunOutput",
        "impl GuestPcTraceSegmentRunOutput",
    );
    assert!(
        segment_output_fields.contains("trace: Option<WitnessTraceBuffer>"),
        "guest PC segment output should be able to carry device material without a host trace"
    );
    assert!(
        segment_output_fields.contains("unit_values: Vec<WitnessTraceUnitValue>")
            && segment_output_fields.contains("proof_values: Vec<WitnessTraceProofValue>"),
        "guest PC segment output should carry backend values independently of host trace storage"
    );
    assert!(
        !segment_output_fields.contains("output: WitnessTraceRunOutput"),
        "guest PC segment output should not require a full WitnessTraceRunOutput"
    );

    let segment_output_impl = function_body(
        &backend_source,
        "impl GuestPcTraceSegmentRunOutput",
        "pub fn run_guest_pc_trace_segments_with_context",
    );
    assert!(
        segment_output_impl.contains("trace_if_available(&self)")
            && segment_output_impl.contains("unit_values(&self)")
            && segment_output_impl.contains("proof_values(&self)")
            && segment_output_impl.contains("into_trace(self) -> Option<WitnessTraceBuffer>"),
        "guest PC segment output should expose host trace and backend values through separate accessors"
    );

    let helper_body = function_body(
        &execution_source,
        "fn build_preloaded_guest_pc_trace_stage_source_devices",
        "fn run_prove_witness_commitments_with_guest_pc_trace_segments_inner",
    );
    assert!(
        helper_body.contains("segment_output.trace_if_available()"),
        "preloaded source fallback should request host trace explicitly only after device material is unavailable"
    );
    assert!(
        !helper_body.contains("segment_output.output().trace()"),
        "preloaded source helper should not force a mandatory WitnessTraceRunOutput just to reach the host trace fallback"
    );

    let segment_body = function_body(
        &execution_source,
        "fn run_prove_witness_commitments_with_guest_pc_trace_segment_commitments_inner",
        "fn merge_backend_unit_values",
    );
    assert!(
        !segment_body.contains("segment_output.into_output()"),
        "segmented commitment path should not force a host trace output wrapper before merging backend values"
    );
    assert!(
        !segment_body.contains("device_segment_material().cloned()"),
        "segmented commitment path should move compact device material instead of cloning descriptors"
    );
}

#[test]
fn guest_pc_segment_commitment_input_can_be_trace_less_with_preloaded_device_sources() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let execution_path = crate_root.join("src/witness_execution.rs");
    let execution_source =
        std::fs::read_to_string(&execution_path).expect("witness execution source should read");

    let input_fields = function_body(
        &execution_source,
        "struct WitnessTraceCommitmentInput",
        "struct ProveWitnessTraceRunObservers",
    );
    assert!(
        input_fields.contains("trace: Option<WitnessTraceBuffer>"),
        "witness commitment input should be able to carry preloaded CUDA stage sources without host trace values"
    );

    let trace_helper_body = function_body(
        &execution_source,
        "fn guest_pc_segment_commitment_trace",
        "fn run_prove_witness_commitments_with_guest_pc_trace_segments_inner",
    );
    assert!(
        trace_helper_body.contains("guest_pc_trace_less_commitment_input_enabled()")
            && trace_helper_body.contains("Ok(None)"),
        "guest PC segment commitment input should skip host trace when a gated preloaded CUDA source is available"
    );
    assert!(
        trace_helper_body.contains("require_guest_pc_segment_host_trace"),
        "guest PC segment commitment input should keep an explicit host trace fallback"
    );

    let inner_body = function_body(
        &execution_source,
        "fn run_prove_witness_commitments_from_trace_inner",
        "fn retain_fri_stage_source_devices",
    );
    assert!(
        inner_body.contains("trace.as_ref()"),
        "commitment execution should treat host trace as optional and borrow it only for fallback consumers"
    );
    assert!(
        inner_body.contains("trace_rows = layout.row_count()")
            && inner_body.contains("trace_columns = layout.column_count()"),
        "trace-less commitment metadata should come from layout rather than a host trace buffer"
    );
}

#[test]
fn trace_less_guest_pc_opening_uses_external_device_source_provider() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let execution_path = crate_root.join("src/witness_execution.rs");
    let execution_source =
        std::fs::read_to_string(&execution_path).expect("witness execution source should read");
    let opening_path = crate_root.join("src/witness_opening.rs");
    let opening_source =
        std::fs::read_to_string(&opening_path).expect("witness opening source should read");
    let tree_path = crate_root.join("src/witness_commitment/tree.rs");
    let tree_source =
        std::fs::read_to_string(&tree_path).expect("witness commitment tree source should read");
    let values_path = crate_root.join("src/witness_commitment/values.rs");
    let values_source = std::fs::read_to_string(&values_path)
        .expect("witness commitment values source should read");

    let input_fields = function_body(
        &execution_source,
        "struct WitnessTraceCommitmentInput",
        "struct ProveWitnessTraceRunObservers",
    );
    assert!(
        input_fields.contains("guest_pc_device_segment_material"),
        "trace-less guest PC outputs should carry compact device material for later openings"
    );

    let inner_body = function_body(
        &execution_source,
        "fn run_prove_witness_commitments_from_trace_inner",
        "fn retain_fri_stage_source_devices",
    );
    assert!(
        inner_body.contains("external_source_commitment_required")
            && inner_body
                .contains("commit_witness_stage_source_devices_and_indexed_timing_external_source"),
        "trace-less CUDA commitment should explicitly allow external-source compact commitments"
    );

    let opening_body = function_body(
        &opening_source,
        "fn build_witness_opening_unit_segment_from_trace_output",
        "fn field_digest_from_words",
    );
    assert!(
        opening_body.contains("guest_pc_external_stage_sources")
            && opening_body.contains("build_guest_pc_trace_stage_source_devices_from_device_material")
            && opening_body.contains("source_device.source_view()"),
        "trace output witness openings should rebuild guest PC stage sources from compact device material"
    );

    let device_commit_body = function_body(
        &tree_source,
        "pub(crate) fn commit_witness_stage_device_compact_with_leaf_hash_level",
        "fn validate_witness_stage_leaves",
    );
    assert!(
        device_commit_body.contains("external_source_required")
            && device_commit_body.contains("SourceDeviceRetentionUnavailable"),
        "device compact commitments should only skip retained source when external source is explicit"
    );

    let source_buffer_body = function_body(
        &values_source,
        "fn source_device_buffer",
        "fn extend_source_device_buffer_cuda",
    );
    assert!(
        source_buffer_body.contains("ExternalSourceUnavailable"),
        "external-source compact openings should fail explicitly when no provider is available"
    );
}

#[test]
fn trace_less_guest_pc_opening_does_not_rebuild_external_source_when_retained() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let opening_path = crate_root.join("src/witness_opening.rs");
    let opening_source =
        std::fs::read_to_string(&opening_path).expect("witness opening source should read");

    let opening_body = function_body(
        &opening_source,
        "fn build_witness_opening_unit_segment_from_trace_output",
        "fn guest_pc_external_stage_sources",
    );
    assert!(
        !opening_body
            .contains("let guest_pc_external_stage_sources = guest_pc_external_stage_sources(unit, output)?;"),
        "trace-output openings should not rebuild guest PC external sources before checking retained source views"
    );
    assert!(
        opening_body.contains("ensure_guest_pc_external_stage_sources")
            && opening_body.contains("retained_source_view.is_none()"),
        "trace-output openings should lazily build external sources only when retained source views are missing"
    );
}

#[test]
fn trace_output_opening_rebuilds_external_source_only_when_required() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let opening_path = crate_root.join("src/witness_opening.rs");
    let opening_source =
        std::fs::read_to_string(&opening_path).expect("witness opening source should read");

    let opening_body = function_body(
        &opening_source,
        "fn build_witness_opening_unit_segment_from_trace_output",
        "fn guest_pc_external_stage_sources",
    );
    assert!(
        opening_body
            .contains("retained_source_view.is_none() && commitment.requires_external_source()"),
        "trace-output openings should not build guest PC external providers for commitments that can open from retained or embedded material"
    );
}

#[test]
fn trace_less_guest_pc_segment_output_can_skip_host_trace_build() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let backend_path = crate_root.join("src/guest_pc_trace_backend.rs");
    let backend_source =
        std::fs::read_to_string(&backend_path).expect("guest PC trace backend source should read");

    let segment_trace_struct = function_body(
        &backend_source,
        "struct GuestPcTraceSegmentTrace",
        "struct GuestPcTraceSegmentSlice",
    );
    assert!(
        segment_trace_struct.contains("trace: Option<WitnessTraceBuffer>"),
        "guest PC segment internals should allow trace-less outputs"
    );

    let helper_body = function_body(
        &backend_source,
        "fn build_layout_zisk_main_trace_segment_for_segment_output",
        "fn build_layout_zisk_main_trace_segment",
    );
    assert!(
        helper_body.contains("guest_pc_trace_less_segment_output_enabled()")
            && helper_body.contains("build_layout_zisk_main_trace_segment_from_device_material"),
        "segmented guest PC output should prefer device material without building a host trace when gated"
    );
    assert!(
        helper_body.contains("build_layout_zisk_main_trace_segment("),
        "trace-less segmented output should keep the host-trace fallback"
    );

    let device_material_body = function_body(
        &backend_source,
        "fn build_layout_zisk_main_trace_segment_from_device_material",
        "fn build_layout_zisk_main_trace_segment_for_segment_output",
    );
    assert!(
        !device_material_body.contains("material.clone()"),
        "trace-less segmented output should not clone compact descriptor material"
    );
}

#[test]
fn guest_pc_device_segment_material_avoids_duplicate_trace_metadata() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let backend_path = crate_root.join("src/guest_pc_trace_backend.rs");
    let backend_source =
        std::fs::read_to_string(&backend_path).expect("guest PC trace backend source should read");

    let material_struct = function_body(
        &backend_source,
        "pub(crate) struct GuestPcTraceDeviceSegmentMaterial",
        "struct GuestPcTraceDeviceSegmentBuild",
    );
    for field in ["unit_values", "final_state", "continuation_state"] {
        assert!(
            !material_struct.contains(field),
            "device segment material should not duplicate {field}"
        );
    }

    let device_material_body = function_body(
        &backend_source,
        "fn build_layout_zisk_main_trace_segment_from_device_material",
        "fn build_layout_zisk_main_trace_segment_for_segment_output",
    );
    assert!(
        !device_material_body.contains("unit_values.clone()")
            && !device_material_body.contains("final_state.clone()")
            && !device_material_body.contains("continuation_state.clone()"),
        "trace-less segmented output should move trace metadata instead of cloning it into device material"
    );
}

#[test]
fn guest_pc_trace_device_descriptors_preallocate_rows_once() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let backend_path = crate_root.join("src/guest_pc_trace_backend.rs");
    let backend_source =
        std::fs::read_to_string(&backend_path).expect("guest PC trace backend source should read");

    let constructor_body = function_body(
        &backend_source,
        "impl ZiskMainDeviceTraceDescriptors",
        "const ZISK_MAIN_DEVICE_TRACE_COLUMNS",
    );
    assert!(
        constructor_body.contains("Vec::with_capacity"),
        "guest PC device descriptors should preallocate compact row storage once per segment"
    );

    let append_body = function_body(
        &backend_source,
        "fn append_zisk_main_device_trace_descriptor",
        "fn zisk_main_device_trace_source_descriptor",
    );
    assert!(
        !append_body.contains("try_reserve"),
        "guest PC descriptor append should not reserve on every trace row"
    );
}

#[test]
fn guest_pc_trace_segments_stream_through_bounded_pipeline() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let backend_path = crate_root.join("src/guest_pc_trace_backend.rs");
    let backend_source =
        std::fs::read_to_string(&backend_path).expect("guest PC trace backend source should read");
    let execution_path = crate_root.join("src/witness_execution.rs");
    let execution_source =
        std::fs::read_to_string(&execution_path).expect("witness execution source should read");

    let stream_body = function_body(
        &backend_source,
        "fn for_each_guest_pc_trace_segment<E>",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
    );
    assert!(
        stream_body.contains("thread::scope"),
        "guest PC trace streaming should build segments on a scoped producer thread"
    );
    assert!(
        stream_body.contains("mpsc::sync_channel"),
        "guest PC trace streaming should keep producer buffering bounded"
    );
    assert!(
        stream_body.contains("produce_guest_pc_trace_segments"),
        "guest PC trace streaming should keep segment production outside the commit consumer"
    );

    let commit_body = function_body(
        &execution_source,
        "fn run_prove_witness_commitments_with_guest_pc_trace_segment_commitments_inner",
        "fn merge_backend_unit_values",
    );
    assert!(
        commit_body.contains("for_each_guest_pc_trace_segment"),
        "guest PC trace commitments should continue consuming the streaming segment API"
    );
}

#[test]
fn guest_pc_trace_timing_reports_device_source_build_work() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let execution_path = crate_root.join("src/witness_execution.rs");
    let execution_source =
        std::fs::read_to_string(&execution_path).expect("witness execution source should read");
    let cli_path = crate_root.join("../lzvm-cli/src/prove_witness/guest_pc_trace.rs");
    let cli_source =
        std::fs::read_to_string(&cli_path).expect("guest PC CLI timing source should read");

    let timing_fields = function_body(
        &execution_source,
        "pub struct ProveWitnessGuestPcTraceTiming",
        "impl ProveWitnessGuestPcTraceTiming",
    );
    assert!(
        timing_fields.contains("guest_device_source_build_duration"),
        "guest PC timing should carry a device source build bucket"
    );

    let accumulator_fields = function_body(
        &execution_source,
        "struct ProveWitnessTraceTimingAccumulator",
        "impl ProveWitnessTraceTimingAccumulator",
    );
    assert!(
        accumulator_fields.contains("device_source_build_duration"),
        "trace timing accumulation should include preloaded CUDA source build work"
    );

    let commit_body = function_body(
        &execution_source,
        "fn run_prove_witness_commitments_with_guest_pc_trace_segment_commitments_inner",
        "fn merge_backend_unit_values",
    );
    assert!(
        commit_body.contains("device_source_build_duration")
            && commit_body.contains("build_preloaded_guest_pc_trace_stage_source_devices"),
        "guest PC segment timing should wrap preloaded CUDA source construction"
    );

    assert!(
        cli_source.contains("\"guest_device_source_build\"")
            && cli_source.contains("guest_device_source_build_duration()"),
        "CLI timing output should include device source build work"
    );
}

#[test]
fn guest_pc_trace_timing_splits_device_source_upload_and_expand_work() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let backend_path = crate_root.join("src/guest_pc_trace_backend.rs");
    let backend_source =
        std::fs::read_to_string(&backend_path).expect("guest PC trace backend source should read");
    let execution_path = crate_root.join("src/witness_execution.rs");
    let execution_source =
        std::fs::read_to_string(&execution_path).expect("witness execution source should read");
    let cli_path = crate_root.join("../lzvm-cli/src/prove_witness/guest_pc_trace.rs");
    let cli_source =
        std::fs::read_to_string(&cli_path).expect("guest PC CLI timing source should read");

    assert!(
        backend_source.contains("struct GuestPcDeviceSourceBuildTiming"),
        "guest PC backend should expose device source sub-timing"
    );

    let material_body = function_body(
        &backend_source,
        "fn build_guest_pc_trace_stage_source_devices_from_device_material",
        "#[cfg(feature = \"cuda\")]\npub(crate) fn build_guest_pc_trace_stage_source_devices_from_device_descriptors",
    );
    assert!(
        material_body.contains("descriptor_upload_duration")
            && material_body.contains("CudaDeviceBuffer::from_u64_words"),
        "guest PC device material source build should time descriptor uploads"
    );

    let descriptor_body = function_body(
        &backend_source,
        "fn build_guest_pc_trace_stage_source_devices_from_device_descriptors_timing",
        "#[cfg(feature = \"cuda\")]\npub(crate) fn build_guest_pc_trace_stage_source_devices(\n",
    );
    assert!(
        descriptor_body.contains("trace_expand_duration")
            && descriptor_body.contains("from_zisk_main_trace_descriptors_device"),
        "guest PC device descriptor source build should time trace expansion"
    );

    let accumulator_fields = function_body(
        &execution_source,
        "struct ProveWitnessTraceTimingAccumulator",
        "impl ProveWitnessTraceTimingAccumulator",
    );
    assert!(
        accumulator_fields.contains("device_source_descriptor_upload_duration")
            && accumulator_fields.contains("device_source_trace_expand_duration"),
        "trace timing accumulation should retain upload and expansion buckets"
    );

    for (line_name, accessor) in [
        (
            "\"guest_device_source_descriptor_upload\"",
            "guest_device_source_descriptor_upload_duration()",
        ),
        (
            "\"guest_device_source_trace_expand\"",
            "guest_device_source_trace_expand_duration()",
        ),
    ] {
        assert!(
            cli_source.contains(line_name) && cli_source.contains(accessor),
            "CLI timing output should include {line_name}"
        );
    }
}

#[test]
fn guest_pc_trace_timing_reports_descriptor_upload_shape() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let backend_path = crate_root.join("src/guest_pc_trace_backend.rs");
    let backend_source =
        std::fs::read_to_string(&backend_path).expect("guest PC trace backend source should read");
    let execution_path = crate_root.join("src/witness_execution.rs");
    let execution_source =
        std::fs::read_to_string(&execution_path).expect("witness execution source should read");
    let cli_path = crate_root.join("../lzvm-cli/src/prove_witness/guest_pc_trace.rs");
    let cli_source =
        std::fs::read_to_string(&cli_path).expect("guest PC CLI timing source should read");
    let proof_timing_path = crate_root.join("../lzvm-cli/src/prove_witness/proof_timing.rs");
    let proof_timing_source =
        std::fs::read_to_string(&proof_timing_path).expect("proof timing source should read");
    let leaf_extend_path = crate_root.join("src/witness_commitment/extend.rs");
    let leaf_extend_source =
        std::fs::read_to_string(&leaf_extend_path).expect("leaf extend source should read");
    let opening_values_path = crate_root.join("src/witness_commitment/values.rs");
    let opening_values_source =
        std::fs::read_to_string(&opening_values_path).expect("opening values source should read");
    let artifact_timing_path = crate_root.join("src/proof_artifact_timing.rs");
    let artifact_timing_source =
        std::fs::read_to_string(&artifact_timing_path).expect("artifact timing source should read");

    let timing_fields = function_body(
        &backend_source,
        "struct GuestPcDeviceSourceBuildTiming",
        "impl GuestPcDeviceSourceBuildTiming",
    );
    assert!(
        timing_fields.contains("descriptor_upload_byte_count")
            && timing_fields.contains("descriptor_upload_row_count"),
        "guest PC backend timing should carry descriptor upload bytes and rows"
    );

    let material_body = function_body(
        &backend_source,
        "fn build_guest_pc_trace_stage_source_devices_from_device_material",
        "#[cfg(feature = \"cuda\")]\npub(crate) fn build_guest_pc_trace_stage_source_devices_from_device_descriptors",
    );
    assert!(
        material_body.contains("descriptor_upload_byte_count")
            && material_body.contains("descriptor_upload_row_count")
            && material_body.contains("descriptors.words()")
            && material_body.contains(".saturating_mul(std::mem::size_of::<u64>())")
            && material_body.contains("descriptors.descriptor_rows()"),
        "guest PC device material source build should count uploaded descriptor bytes and rows"
    );

    let accumulator_fields = function_body(
        &execution_source,
        "struct ProveWitnessTraceTimingAccumulator",
        "impl ProveWitnessTraceTimingAccumulator",
    );
    assert!(
        accumulator_fields.contains("device_source_descriptor_upload_byte_count")
            && accumulator_fields.contains("device_source_descriptor_upload_row_count"),
        "trace timing accumulation should retain descriptor upload byte and row counts"
    );

    for (line_name, accessor) in [
        ("\"guest_segment_count\"", "segment_count()"),
        (
            "\"guest_device_source_descriptor_upload_bytes\"",
            "guest_device_source_descriptor_upload_byte_count()",
        ),
        (
            "\"guest_device_source_descriptor_upload_rows\"",
            "guest_device_source_descriptor_upload_row_count()",
        ),
        (
            "\"guest_stage_leaf_hash_rows\"",
            "guest_stage_leaf_hash_row_count()",
        ),
        (
            "\"guest_stage_leaf_hash_bytes\"",
            "guest_stage_leaf_hash_byte_count()",
        ),
        (
            "\"guest_stage_leaf_hash_arity2_rows\"",
            "guest_stage_leaf_hash_arity2_row_count()",
        ),
        (
            "\"guest_stage_leaf_hash_arity2_bytes\"",
            "guest_stage_leaf_hash_arity2_byte_count()",
        ),
        (
            "\"guest_stage_leaf_hash_arity4_rows\"",
            "guest_stage_leaf_hash_arity4_row_count()",
        ),
        (
            "\"guest_stage_leaf_hash_arity4_bytes\"",
            "guest_stage_leaf_hash_arity4_byte_count()",
        ),
        (
            "\"guest_stage_leaf_coset_extend_calls\"",
            "guest_stage_leaf_coset_extend_call_count()",
        ),
        (
            "\"guest_stage_leaf_coset_extend_output_bytes\"",
            "guest_stage_leaf_coset_extend_output_byte_count()",
        ),
        (
            "\"guest_stage_leaf_coset_extend_columns\"",
            "guest_stage_leaf_coset_extend_column_count()",
        ),
        (
            "\"guest_stage_leaf_coset_extend_max_columns\"",
            "guest_stage_leaf_coset_extend_max_column_count()",
        ),
        (
            "\"guest_stage_leaf_coset_extend_ntt_launches\"",
            "guest_stage_leaf_coset_extend_ntt_launch_count()",
        ),
        (
            "\"guest_stage_leaf_coset_extend_bit_reverse_launches\"",
            "guest_stage_leaf_coset_extend_bit_reverse_launch_count()",
        ),
        (
            "\"guest_stage_leaf_coset_extend_ntt_stage_launches\"",
            "guest_stage_leaf_coset_extend_ntt_stage_launch_count()",
        ),
        (
            "\"guest_stage_leaf_coset_extend_ntt_block_twiddle_launches\"",
            "guest_stage_leaf_coset_extend_ntt_block_twiddle_launch_count()",
        ),
        (
            "\"guest_stage_leaf_coset_extend_normalize_launches\"",
            "guest_stage_leaf_coset_extend_normalize_launch_count()",
        ),
        (
            "\"guest_stage_leaf_coset_extend_pack_launches\"",
            "guest_stage_leaf_coset_extend_pack_launch_count()",
        ),
        (
            "\"guest_stage_leaf_coset_extend_unpack_launches\"",
            "guest_stage_leaf_coset_extend_unpack_launch_count()",
        ),
    ] {
        assert!(
            cli_source.contains(line_name) && cli_source.contains(accessor),
            "CLI timing output should include {line_name}"
        );
    }

    for (line_name, accessor) in [
        (
            "guest_stage_{stage_index}_leaf_coset_extend_ntt_launches",
            "leaf_coset_extend_ntt_launch_count()",
        ),
        (
            "guest_stage_{stage_index}_leaf_coset_extend_bit_reverse_launches",
            "leaf_coset_extend_bit_reverse_launch_count()",
        ),
        (
            "guest_stage_{stage_index}_leaf_coset_extend_ntt_stage_launches",
            "leaf_coset_extend_ntt_stage_launch_count()",
        ),
        (
            "guest_stage_{stage_index}_leaf_coset_extend_ntt_block_twiddle_launches",
            "leaf_coset_extend_ntt_block_twiddle_launch_count()",
        ),
        (
            "guest_stage_{stage_index}_leaf_coset_extend_normalize_launches",
            "leaf_coset_extend_normalize_launch_count()",
        ),
        (
            "guest_stage_{stage_index}_leaf_coset_extend_pack_launches",
            "leaf_coset_extend_pack_launch_count()",
        ),
        (
            "guest_stage_{stage_index}_leaf_coset_extend_unpack_launches",
            "leaf_coset_extend_unpack_launch_count()",
        ),
    ] {
        assert!(
            cli_source.contains(line_name) && cli_source.contains(accessor),
            "CLI timing output should include dynamic {line_name}"
        );
    }

    for (line_name, field) in [
        (
            "\"finish_witness_opening_query_count\"",
            "witness_opening_query_count",
        ),
        (
            "\"finish_witness_opening_query_unit_count\"",
            "witness_opening_query_unit_count",
        ),
        (
            "\"finish_witness_opening_single_query_unit_count\"",
            "witness_opening_single_query_unit_count",
        ),
        (
            "\"finish_witness_opening_max_queries_per_unit\"",
            "witness_opening_max_queries_per_unit",
        ),
        (
            "\"finish_witness_opening_stage_count\"",
            "witness_opening_stage_count",
        ),
        (
            "\"finish_witness_opening_retained_source_count\"",
            "witness_opening_retained_source_count",
        ),
        (
            "\"finish_witness_opening_external_source_count\"",
            "witness_opening_external_source_count",
        ),
        (
            "\"finish_witness_opening_embedded_source_count\"",
            "witness_opening_embedded_source_count",
        ),
        (
            "\"finish_witness_opening_missing_source_count\"",
            "witness_opening_missing_source_count",
        ),
        (
            "\"finish_witness_opening_leaf_hash_rows\"",
            "witness_opening_leaf_hash_row_count",
        ),
        (
            "\"finish_witness_opening_leaf_hash_bytes\"",
            "witness_opening_leaf_hash_byte_count",
        ),
        (
            "\"finish_witness_opening_leaf_hash_arity2_rows\"",
            "witness_opening_leaf_hash_arity2_row_count",
        ),
        (
            "\"finish_witness_opening_leaf_hash_arity2_bytes\"",
            "witness_opening_leaf_hash_arity2_byte_count",
        ),
        (
            "\"finish_witness_opening_leaf_hash_arity4_rows\"",
            "witness_opening_leaf_hash_arity4_row_count",
        ),
        (
            "\"finish_witness_opening_leaf_hash_arity4_bytes\"",
            "witness_opening_leaf_hash_arity4_byte_count",
        ),
        (
            "\"finish_witness_opening_leaf_coset_extend_calls\"",
            "witness_opening_leaf_coset_extend_call_count",
        ),
        (
            "\"finish_witness_opening_leaf_coset_extend_output_bytes\"",
            "witness_opening_leaf_coset_extend_output_byte_count",
        ),
        (
            "\"finish_witness_opening_leaf_coset_extend_columns\"",
            "witness_opening_leaf_coset_extend_column_count",
        ),
        (
            "\"finish_witness_opening_leaf_coset_extend_max_columns\"",
            "witness_opening_leaf_coset_extend_max_column_count",
        ),
        (
            "\"finish_witness_opening_leaf_coset_extend_ntt_launches\"",
            "witness_opening_leaf_coset_extend_ntt_launch_count",
        ),
        (
            "\"finish_witness_opening_leaf_coset_extend_bit_reverse_launches\"",
            "witness_opening_leaf_coset_extend_bit_reverse_launch_count",
        ),
        (
            "\"finish_witness_opening_leaf_coset_extend_ntt_stage_launches\"",
            "witness_opening_leaf_coset_extend_ntt_stage_launch_count",
        ),
        (
            "\"finish_witness_opening_leaf_coset_extend_ntt_block_twiddle_launches\"",
            "witness_opening_leaf_coset_extend_ntt_block_twiddle_launch_count",
        ),
        (
            "\"finish_witness_opening_leaf_coset_extend_normalize_launches\"",
            "witness_opening_leaf_coset_extend_normalize_launch_count",
        ),
        (
            "\"finish_witness_opening_leaf_coset_extend_pack_launches\"",
            "witness_opening_leaf_coset_extend_pack_launch_count",
        ),
        (
            "\"finish_witness_opening_leaf_coset_extend_unpack_launches\"",
            "witness_opening_leaf_coset_extend_unpack_launch_count",
        ),
    ] {
        assert!(
            proof_timing_source.contains(line_name) && proof_timing_source.contains(field),
            "CLI timing output should include {line_name}"
        );
    }

    for (line_name, field) in [
        (
            "finish_witness_stage_{}_opening_leaf_hash_rows",
            "leaf_hash_row_count",
        ),
        (
            "finish_witness_stage_{}_opening_retained_source_count",
            "retained_source_count",
        ),
        (
            "finish_witness_stage_{}_opening_external_source_count",
            "external_source_count",
        ),
        (
            "finish_witness_stage_{}_opening_embedded_source_count",
            "embedded_source_count",
        ),
        (
            "finish_witness_stage_{}_opening_missing_source_count",
            "missing_source_count",
        ),
        (
            "finish_witness_stage_{}_opening_leaf_hash_bytes",
            "leaf_hash_byte_count",
        ),
        (
            "finish_witness_stage_{}_opening_leaf_coset_extend_calls",
            "leaf_coset_extend_call_count",
        ),
        (
            "finish_witness_stage_{}_opening_leaf_coset_extend_output_bytes",
            "leaf_coset_extend_output_byte_count",
        ),
        (
            "finish_witness_stage_{}_opening_leaf_coset_extend_columns",
            "leaf_coset_extend_column_count",
        ),
        (
            "finish_witness_stage_{}_opening_leaf_coset_extend_max_columns",
            "leaf_coset_extend_max_column_count",
        ),
        (
            "finish_witness_stage_{}_opening_leaf_coset_extend_ntt_launches",
            "leaf_coset_extend_ntt_launch_count",
        ),
        (
            "finish_witness_stage_{}_opening_leaf_coset_extend_bit_reverse_launches",
            "leaf_coset_extend_bit_reverse_launch_count",
        ),
        (
            "finish_witness_stage_{}_opening_leaf_coset_extend_ntt_stage_launches",
            "leaf_coset_extend_ntt_stage_launch_count",
        ),
        (
            "finish_witness_stage_{}_opening_leaf_coset_extend_ntt_block_twiddle_launches",
            "leaf_coset_extend_ntt_block_twiddle_launch_count",
        ),
        (
            "finish_witness_stage_{}_opening_leaf_coset_extend_normalize_launches",
            "leaf_coset_extend_normalize_launch_count",
        ),
        (
            "finish_witness_stage_{}_opening_leaf_coset_extend_pack_launches",
            "leaf_coset_extend_pack_launch_count",
        ),
        (
            "finish_witness_stage_{}_opening_leaf_coset_extend_unpack_launches",
            "leaf_coset_extend_unpack_launch_count",
        ),
    ] {
        assert!(
            proof_timing_source.contains(line_name) && proof_timing_source.contains(field),
            "CLI timing output should include dynamic {line_name}"
        );
    }

    for source in [
        leaf_extend_source.as_str(),
        opening_values_source.as_str(),
        execution_source.as_str(),
        artifact_timing_source.as_str(),
    ] {
        assert!(
            source.contains("leaf_hash_arity2_row_count")
                && source.contains("leaf_hash_arity2_byte_count")
                && source.contains("leaf_hash_arity4_row_count")
                && source.contains("leaf_hash_arity4_byte_count"),
            "leaf hash timing should split arity2 and arity4 row and byte counts"
        );
    }

    for source in [
        opening_values_source.as_str(),
        artifact_timing_source.as_str(),
        proof_timing_source.as_str(),
    ] {
        assert!(
            source.contains("leaf_coset_extend_call_count")
                && source.contains("leaf_coset_extend_output_byte_count")
                && source.contains("leaf_coset_extend_column_count")
                && source.contains("leaf_coset_extend_max_column_count")
                && source.contains("leaf_coset_extend_ntt_launch_count")
                && source.contains("leaf_coset_extend_bit_reverse_launch_count")
                && source.contains("leaf_coset_extend_ntt_stage_launch_count")
                && source.contains("leaf_coset_extend_ntt_block_twiddle_launch_count")
                && source.contains("leaf_coset_extend_normalize_launch_count")
                && source.contains("leaf_coset_extend_pack_launch_count")
                && source.contains("leaf_coset_extend_unpack_launch_count"),
            "opening timing should expose coset extension workload shape"
        );
    }

    assert!(
        artifact_timing_source.contains("witness_stage_opening_work")
            && proof_timing_source.contains("witness_stage_opening_work"),
        "opening timing aggregation should expose per-stage work shape"
    );

    for source in [
        leaf_extend_source.as_str(),
        execution_source.as_str(),
        cli_source.as_str(),
    ] {
        assert!(
            source.contains("leaf_coset_extend_call_count")
                && source.contains("leaf_coset_extend_output_byte_count")
                && source.contains("leaf_coset_extend_column_count")
                && source.contains("leaf_coset_extend_max_column_count")
                && source.contains("leaf_coset_extend_ntt_launch_count")
                && source.contains("leaf_coset_extend_bit_reverse_launch_count")
                && source.contains("leaf_coset_extend_ntt_stage_launch_count")
                && source.contains("leaf_coset_extend_ntt_block_twiddle_launch_count")
                && source.contains("leaf_coset_extend_normalize_launch_count")
                && source.contains("leaf_coset_extend_pack_launch_count")
                && source.contains("leaf_coset_extend_unpack_launch_count"),
            "leaf extension timing should expose coset extension workload shape"
        );
    }
}

#[test]
fn guest_pc_trace_segments_split_runner_and_lowerer_with_bounded_pending_queue() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let backend_path = crate_root.join("src/guest_pc_trace_backend.rs");
    let backend_source =
        std::fs::read_to_string(&backend_path).expect("guest PC trace backend source should read");

    assert!(
        backend_source.contains("struct GuestPcTracePendingSegmentSlice"),
        "guest PC trace streaming should represent guest-run slices separately from built traces"
    );
    assert!(
        backend_source.contains("fn produce_guest_pc_trace_pending_slices"),
        "guest PC trace streaming should advance the guest machine in a separate runner stage"
    );
    assert!(
        backend_source.contains("fn lower_guest_pc_trace_pending_segments"),
        "guest PC trace streaming should lower pending slices into trace buffers in a separate stage"
    );

    let produce_body = function_body(
        &backend_source,
        "fn produce_guest_pc_trace_segments",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]\nstruct LayoutTraceCapacity",
    );
    assert!(
        produce_body.contains("mpsc::sync_channel(guest_pc_trace_segment_queue_capacity())"),
        "pending guest slices should flow through the same bounded queue policy as built segments"
    );
    assert!(
        produce_body.contains("produce_guest_pc_trace_pending_slices")
            && produce_body.contains("lower_guest_pc_trace_pending_segments"),
        "guest PC trace streaming should overlap guest execution with trace lowering"
    );
}

#[test]
fn guest_pc_trace_segments_reuse_fixed_columns_across_segments() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_path = crate_root.join("src/witness_execution.rs");
    let source =
        std::fs::read_to_string(&source_path).expect("witness execution source should read");

    let observers_body = function_body(
        &source,
        "struct ProveWitnessTraceRunObservers",
        "type WitnessFixedColumnsLoadResult",
    );
    assert!(
        observers_body.contains("fixed_columns_cache"),
        "trace run observers should be able to borrow a shared fixed-column cache"
    );

    let segment_body = function_body(
        &source,
        "fn run_prove_witness_commitments_with_guest_pc_trace_segment_commitments_inner",
        "fn merge_backend_unit_values",
    );
    assert!(
        segment_body.contains("let mut fixed_columns_cache = WitnessFixedColumnsCache::new()"),
        "guest PC trace segment commitments should keep fixed columns cached across segments"
    );
    assert!(
        segment_body.contains("fixed_columns_cache: Some(&mut fixed_columns_cache)"),
        "each segment should borrow the shared fixed-column cache instead of reloading it"
    );

    let inner_body = function_body(
        &source,
        "fn run_prove_witness_commitments_from_trace_inner",
        "fn load_witness_shared_inputs",
    );
    assert!(
        inner_body.contains("fixed_columns_cache")
            && inner_body.contains("as_deref_mut()")
            && inner_body.contains("unwrap_or(&mut local_fixed_columns)"),
        "per-trace commitment should prefer an observer-provided fixed-column cache"
    );
}

#[test]
fn all_units_witness_reuses_shared_inputs_and_borrows_trace_bundle_bytes() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_path = crate_root.join("src/witness_execution.rs");
    let source =
        std::fs::read_to_string(&source_path).expect("witness execution source should read");

    let single_unit_inner = function_body(
        &source,
        "fn run_prove_witness_commitments_with_trace_backend_inner",
        "pub fn run_prove_witness_commitments_for_all_units",
    );
    assert!(
        !single_unit_inner.contains("read_witness_input"),
        "witness execution should preload input data before entering the per-unit body"
    );
    assert!(
        !single_unit_inner.contains("load_public_inputs"),
        "witness execution should preload public inputs before entering the per-unit body"
    );
    assert!(
        single_unit_inner.contains("layout.request(&shared_inputs.input[..])"),
        "witness execution should borrow shared input bytes for each unit"
    );
    assert!(
        !single_unit_inner.contains("request_borrowed"),
        "witness trace layout should use one request constructor for owned and borrowed inputs"
    );
    assert!(
        !single_unit_inner.contains("shared_inputs.input.clone()"),
        "witness execution should avoid cloning shared input bytes for each unit"
    );

    let all_units_body = function_body(
        &source,
        "pub fn run_prove_witness_commitments_for_all_units",
        "pub fn run_prove_witness_commitments_for_all_units_with_trace_bundle",
    );
    assert!(
        all_units_body.contains("load_witness_shared_inputs(plan)"),
        "all-units witness execution should load shared inputs once"
    );

    let trace_bundle_body = function_body(
        &source,
        "pub fn run_prove_witness_commitments_for_all_units_with_trace_bundle",
        "fn validate_trace_bundle_unit_set",
    );
    assert!(
        trace_bundle_body.contains("run_prove_witness_commitments_with_trace_bytes_inner"),
        "all-units trace bundle execution should parse precomputed trace bytes directly"
    );
    assert!(
        !trace_bundle_body.contains("TraceBytesBackend"),
        "all-units trace bundle execution should avoid copying through the witness backend buffer"
    );
}

#[test]
fn cli_single_unit_precomputed_trace_bytes_parse_without_backend_copy() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_path = crate_root.join("../lzvm-cli/src/prove_witness.rs");
    let source = std::fs::read_to_string(&source_path).expect("prove witness source should read");

    let single_unit_body = function_body(
        &source,
        "let output = if let Some(path) = &plan.inputs.witness_library",
        "let commitments = output.commitments();",
    );

    assert!(
        single_unit_body.contains("run_prove_witness_commitments_with_trace_bytes"),
        "single-unit precomputed trace execution should parse trace bytes directly"
    );
    assert!(
        !single_unit_body.contains("TraceBytesBackend"),
        "single-unit precomputed trace execution should avoid copying through the witness backend buffer"
    );
}

#[test]
fn cli_prove_witness_parses_trace_bundle_without_unit_trace_copies() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_path = crate_root.join("../lzvm-cli/src/prove_witness.rs");
    let source = std::fs::read_to_string(&source_path).expect("prove witness source should read");

    let trace_bundle_body = function_body(
        &source,
        "let trace_bundle = match",
        "let challenge_values_segment = match",
    );

    assert!(
        trace_bundle_body.contains("parse_trace_bundle_ref"),
        "prove witness should parse trace bundles as borrowed section views"
    );
    assert!(
        !trace_bundle_body.contains("read_trace_bundle_file"),
        "prove witness should not clone trace bundle unit bytes while reading"
    );
}

#[test]
fn cuda_parent_levels_upload_digest_prefixes_without_host_state_expansion() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_path = crate_root.join("src/merkle_hash.rs");
    let source = std::fs::read_to_string(&source_path).expect("Merkle hash source should read");

    let body = function_body(
        &source,
        "fn parent_levels_from_digest_level_on_cuda",
        "fn parent_levels_from_device_buffer",
    );

    assert!(
        body.contains("state_buffer_from_digest_level(level, width)"),
        "CUDA parent levels should upload compact digest prefixes before expanding padded states"
    );
    assert!(
        !body.contains("digest_level_as_state_words(level, width)"),
        "CUDA parent levels should avoid host-side padded state expansion"
    );
}

#[test]
fn cuda_state_prefix_expansion_avoids_temporary_prefix_device_buffer() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_path = crate_root.join("../lzvm-accel/src/cuda_buffer.rs");
    let source = std::fs::read_to_string(&source_path).expect("accel source should read");

    let body = function_body(
        &source,
        "pub fn from_state_prefix_u64_words",
        "pub fn to_u64_words",
    );

    assert!(
        body.contains("words.as_ptr().cast()"),
        "state prefix expansion should copy compact host prefixes directly into padded device states"
    );
    assert!(
        !body.contains("Self::from_u64_words(words)"),
        "state prefix expansion should avoid an intermediate compact device buffer"
    );
}

#[test]
fn secp256k1_double_scalar_mul_uses_projective_accumulator() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_path = crate_root.join("src/secp256k1_host.rs");
    let source = std::fs::read_to_string(&source_path).expect("secp256k1 source should read");

    let body = function_body(
        &source,
        "pub(crate) fn secp256k1_double_scalar_mul",
        "pub(crate) fn limbs_to_biguint",
    );

    assert!(
        body.contains("SecpProjectivePoint"),
        "double-scalar multiplication should use a projective accumulator"
    );
    assert!(
        !body.contains("secp256k1_point_add"),
        "double-scalar multiplication should avoid affine additions in the bit loop"
    );
    assert!(
        !body.contains("secp256k1_point_double"),
        "double-scalar multiplication should avoid affine doublings in the bit loop"
    );
}

#[test]
fn guest_machine_reports_inline_common_effect_storage() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_path = crate_root.join("src/guest_machine/mod.rs");
    let source = std::fs::read_to_string(&source_path).expect("guest machine source should read");

    let body = function_body(
        &source,
        "struct GuestInstructionEffects",
        "pub struct GuestMachineRunReport",
    );

    assert!(
        source.contains("pub type GuestRegisterWriteList = SmallVec<[GuestRegisterWrite; 1]>;"),
        "guest register writes should keep one inline slot"
    );
    assert!(
        source.contains("pub type GuestMemoryAccessList = SmallVec<[GuestMemoryAccess; 2]>;"),
        "guest memory accesses should keep two inline slots"
    );
    assert!(
        body.contains("register_writes: GuestRegisterWriteList"),
        "guest machine reports should inline common small effect lists"
    );
    assert!(
        body.contains("memory_accesses: GuestMemoryAccessList"),
        "guest machine reports should inline common memory effect lists"
    );
    assert!(
        !body.contains("\n    register_writes: Vec<GuestRegisterWrite>"),
        "guest register writes should avoid one allocation per writing instruction"
    );
    assert!(
        !body.contains("\n    memory_accesses: Vec<GuestMemoryAccess>"),
        "guest memory accesses should avoid one allocation per memory instruction"
    );
}

#[test]
fn fri_opening_from_transcript_values_borrows_large_vectors() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_path = crate_root.join("src/prove_fri_opening.rs");
    let source = std::fs::read_to_string(&source_path).expect("FRI opening source should read");

    let body = function_body(
        &source,
        "pub fn build_pcs_fri_opening_segment_from_transcript_values",
        "pub fn build_pcs_fri_opening_segment_from_trace",
    );

    assert!(
        !body.contains("challenges.clone()"),
        "FRI opening construction should borrow transcript challenges"
    );
    assert!(
        !body.contains("polynomial.clone()"),
        "FRI opening construction should borrow transcript polynomial values"
    );
    assert!(
        body.contains("build_pcs_fri_opening_segment_from_value_refs"),
        "FRI opening construction should use the borrowed opening builder"
    );

    let helper_body = function_body(
        &source,
        "fn build_pcs_fri_opening_segment_from_value_refs",
        "pub fn build_pcs_fri_transcript_values_from_trace",
    );
    for copy_operation in [
        "challenges.clone()",
        "polynomial.clone()",
        "challenges.to_vec()",
        "polynomial.to_vec()",
        "clone_from",
    ] {
        assert!(
            !helper_body.contains(copy_operation),
            "borrowed FRI opening builder should not copy transcript vectors with {copy_operation}"
        );
    }
}

#[test]
fn fri_opening_from_trace_borrows_challenges() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_path = crate_root.join("src/prove_fri_opening.rs");
    let source = std::fs::read_to_string(&source_path).expect("FRI opening source should read");

    let body = function_body(
        &source,
        "pub fn build_pcs_fri_opening_segment_from_trace",
        "pub fn build_pcs_fri_opening_segment_from_trace_segments",
    );

    assert!(
        body.contains("build_pcs_fri_opening_segment_from_value_refs"),
        "trace FRI opening construction should use the borrowed opening builder"
    );
    assert!(
        body.contains("PcsFriOpeningTraceValue"),
        "trace FRI opening construction should keep local values in a borrowed challenge holder"
    );
    assert!(
        body.contains("challenges: input.challenges"),
        "trace FRI opening construction should store challenge slices directly"
    );
    for copy_operation in [
        "challenges.clone()",
        "challenges.to_vec()",
        "challenges.to_owned()",
        "Vec::from(input.challenges",
        "Vec::from(value.challenges",
        "input.challenges.iter().copied().collect",
        "input.challenges.iter().cloned().collect",
        "clone_from",
    ] {
        assert!(
            !body.contains(copy_operation),
            "trace FRI opening construction should borrow challenges instead of copying with {copy_operation}"
        );
    }
}

#[test]
fn all_units_transcript_proof_borrows_auxiliary_vectors() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_path = crate_root.join("src/proof_artifact.rs");
    let source = std::fs::read_to_string(&source_path).expect("proof artifact source should read");
    let body = function_body(
        &source,
        "fn build_witness_transcript_proof_artifact_for_all_units",
        "struct AllUnitsTranscriptProofInputs",
    );

    for copy_operation in [
        "let mut auxiliary_inputs",
        "ProveWitnessAuxiliaryInputs {",
        "output.auxiliary_inputs().clone()",
        "values.packed_values.clone()",
        "values.packed_values.to_owned()",
        "Vec::from(values.packed_values",
        "proof_inputs.proof_values.to_vec()",
        "proof_inputs.proof_values.to_owned()",
        "Vec::from(proof_inputs.proof_values",
        "proof_inputs.group_values.to_vec()",
        "proof_inputs.group_values.to_owned()",
        "Vec::from(proof_inputs.group_values",
        "values.values.clone()",
        "values.values.to_owned()",
        "Vec::from(values.values",
        "extend_from_slice",
        "clone_from",
    ] {
        assert!(
            !body.contains(copy_operation),
            "all-units transcript proof should borrow auxiliary values instead of copying with {copy_operation}"
        );
    }
    assert!(
        body.contains("ProveWitnessAuxiliaryInputSlices"),
        "all-units transcript proof should assemble borrowed auxiliary slices"
    );
    assert!(
        body.contains("build_pcs_fri_transcript_values_from_trace_segment_refs"),
        "all-units transcript proof should use the borrowed transcript builder"
    );
    assert!(
        body.contains("build_witness_opening_segment_batch_from_trace_outputs"),
        "all-units transcript proof should build witness openings from trace outputs so CUDA source buffers can be reused"
    );

    let fri_source_path = crate_root.join("src/prove_fri_opening.rs");
    let fri_source =
        std::fs::read_to_string(&fri_source_path).expect("FRI opening source should read");
    let helper_body = function_body(
        &fri_source,
        "pub(crate) fn build_pcs_fri_transcript_values_from_trace_segment_refs",
        "pub fn build_pcs_fri_opening_segment_from_transcript_values",
    );
    assert!(
        helper_body.contains("build_pcs_fri_transcript_values_from_trace_refs"),
        "borrowed trace segment builder should keep borrowed auxiliary slices through FRI transcript construction"
    );
    assert!(
        !helper_body.contains("ProveWitnessAuxiliaryInputs"),
        "borrowed trace segment builder should not rebuild owned auxiliary inputs"
    );
}

#[test]
fn unit_transcript_proof_uses_trace_output_witness_openings() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_path = crate_root.join("src/proof_artifact.rs");
    let source = std::fs::read_to_string(&source_path).expect("proof artifact source should read");
    let body = function_body(
        &source,
        "pub fn build_witness_proof_artifact_for_unit",
        "pub struct WitnessAllUnitsProofRequest",
    );

    assert!(
        body.contains("build_witness_opening_segment_batch_from_trace_outputs"),
        "unit transcript proof should build witness openings from trace outputs so CUDA source buffers can be reused"
    );
    assert!(
        body.contains("&[request.output]"),
        "unit transcript proof should pass the trace output through to the witness opening builder"
    );
    assert!(
        !body.contains(
            "build_witness_opening_segment(request.schedule, &query_segment, commitments)"
        ),
        "unit transcript proof should not fall back to commitment-only witness openings"
    );
}

#[test]
fn all_units_non_transcript_proof_uses_trace_output_witness_openings() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_path = crate_root.join("src/proof_artifact.rs");
    let source = std::fs::read_to_string(&source_path).expect("proof artifact source should read");

    let helper_body = function_body(
        &source,
        "fn build_witness_proof_artifact_from_trace_outputs_with_bindings_and_material_summaries",
        "pub struct ProofArtifactInputs",
    );
    assert!(
        helper_body.contains("build_witness_opening_segment_batch_from_trace_outputs"),
        "non-transcript proof construction should preserve trace-output source providers for witness openings"
    );
    assert!(
        !helper_body.contains("build_witness_opening_segment_batch(schedule"),
        "non-transcript trace-output proof construction should not use commitment-only witness openings"
    );

    let all_units_body = function_body(
        &source,
        "pub fn build_witness_proof_artifact_for_all_units",
        "fn build_witness_transcript_proof_artifact_for_all_units",
    );
    assert!(
        all_units_body.contains(
            "build_witness_proof_artifact_from_trace_outputs_with_bindings_and_material_summaries"
        ),
        "all-units non-transcript proof path should call the trace-output proof builder"
    );
}

#[test]
fn public_proof_artifact_builders_require_trace_evidence_outputs() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_path = crate_root.join("src/proof_artifact.rs");
    let source = std::fs::read_to_string(&source_path).expect("proof artifact source should read");

    for (start, end, label) in [
        (
            "pub fn build_witness_proof_core_artifact",
            "pub(crate) fn build_witness_proof_core_artifact_with_bindings",
            "core proof artifact builder",
        ),
        (
            "pub fn build_witness_proof_artifact",
            "pub fn build_witness_proof_artifact_with_bindings",
            "full proof artifact builder",
        ),
        (
            "pub fn build_witness_proof_artifact_with_bindings",
            "fn build_witness_proof_artifact_with_bindings_and_material_summaries",
            "binding-aware proof artifact builder",
        ),
    ] {
        let body = function_body(&source, start, end);
        assert!(
            body.contains("witness_outputs: &[&ProveWitnessTraceCommitments]"),
            "{label} should require runtime trace evidence outputs"
        );
        assert!(
            !body.contains("witness_outputs: &[&ProveWitnessCommitments]"),
            "{label} should not accept commitment-only witness outputs"
        );
    }
}

#[test]
fn trace_constraint_segment_uses_runtime_evidence_flags() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_path = crate_root.join("src/proof_artifact.rs");
    let source = std::fs::read_to_string(&source_path).expect("proof artifact source should read");
    let body = function_body(
        &source,
        "fn build_trace_constraint_evidence_segment",
        "pub struct WitnessProofRequest",
    );

    assert!(
        body.contains("let evidence = output.trace_constraint_evidence();"),
        "trace constraint segment should derive each unit from runtime evidence"
    );
    for accessor in [
        "evidence.regular_constraint_count()",
        "evidence.trace_extracted()",
        "evidence.regular_constraints_evaluated()",
        "evidence.witness_values_committed()",
        "evidence.constraint_checker_conformant()",
    ] {
        assert!(
            body.contains(accessor),
            "trace constraint segment should encode runtime evidence through {accessor}"
        );
    }
    for hardcoded_flag in [
        "trace_extracted: true",
        "regular_constraints_evaluated: true",
        "witness_values_committed: true",
        "constraint_checker_conformant: true",
    ] {
        assert!(
            !body.contains(hardcoded_flag),
            "trace constraint segment should not synthesize runtime evidence with {hardcoded_flag}"
        );
    }
}

#[test]
fn lean_trace_constraint_artifact_binding_tracks_runtime_preflight_checks() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_path = crate_root.join("../../lean/Lzvm/TraceConstraintArtifactBinding.lean");
    let lean_source = std::fs::read_to_string(&lean_path)
        .expect("Lean trace artifact binding source should read");
    let lean_root_path = crate_root.join("../../lean/Lzvm.lean");
    let lean_root_source =
        std::fs::read_to_string(&lean_root_path).expect("top-level Lean source should read");
    let preflight_path = crate_root.join("src/proof_preflight.rs");
    let preflight_source =
        std::fs::read_to_string(&preflight_path).expect("proof preflight source should read");

    assert!(
        lean_root_source.contains("import Lzvm.TraceConstraintArtifactBinding"),
        "top-level Lean module should include the trace constraint artifact binding model"
    );
    assert!(
        lean_source.contains("structure RuntimeTraceConstraintArtifactBindingValidation")
            && lean_source
                .contains("runtime_trace_constraint_artifact_binding_checked_acceptance_sound"),
        "Lean should expose a checked trace constraint artifact binding soundness theorem"
    );
    assert!(
        lean_source.contains("RuntimeTraceConstraintValidation")
            && lean_source.contains("runtime_trace_constraint_checked_acceptance_sound"),
        "Lean trace artifact binding should compose with the trace constraint soundness model"
    );
    assert!(
        preflight_source.contains("validate_trace_constraint_witness_commitments")
            && preflight_source.contains("contains_witness_commitment_segments")
            && preflight_source
                .contains("validate_trace_constraint_witness_commitments_for_unit_count"),
        "Rust proof preflight should keep trace constraint witness commitment binding checks"
    );
}

#[test]
fn lean_opening_segment_binding_tracks_runtime_opening_checks() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_path = crate_root.join("../../lean/Lzvm/OpeningSegmentBinding.lean");
    let lean_source = std::fs::read_to_string(&lean_path)
        .expect("Lean opening segment binding source should read");
    let lean_root_path = crate_root.join("../../lean/Lzvm.lean");
    let lean_root_source =
        std::fs::read_to_string(&lean_root_path).expect("top-level Lean source should read");
    let setup_preflight_path = crate_root.join("src/setup_preflight.rs");
    let setup_preflight_source =
        std::fs::read_to_string(&setup_preflight_path).expect("setup preflight source should read");
    let constant_opening_path = crate_root.join("src/constant_opening.rs");
    let constant_opening_source = std::fs::read_to_string(&constant_opening_path)
        .expect("constant opening source should read");
    let witness_opening_path = crate_root.join("src/witness_opening.rs");
    let witness_opening_source =
        std::fs::read_to_string(&witness_opening_path).expect("witness opening source should read");
    let fri_opening_path = crate_root.join("src/pcs_fri/validation.rs");
    let fri_opening_source =
        std::fs::read_to_string(&fri_opening_path).expect("FRI opening source should read");

    assert!(
        lean_root_source.contains("import Lzvm.OpeningSegmentBinding"),
        "top-level Lean module should include the opening segment binding model"
    );
    assert!(
        lean_source.contains("structure RuntimeOpeningSegmentBindingValidation")
            && lean_source.contains("runtime_opening_segment_binding_checked_acceptance_sound"),
        "Lean should expose a checked opening segment binding soundness theorem"
    );
    assert!(
        lean_source.contains("RuntimeOpeningValidation")
            && lean_source.contains("runtime_opening_checked_acceptance_sound"),
        "Lean opening segment binding should compose with the opening soundness model"
    );
    assert!(
        setup_preflight_source.contains("validate_constant_opening_segments")
            && setup_preflight_source.contains("validate_witness_opening_segments")
            && setup_preflight_source.contains("validate_optional_pcs_fri_opening_proof_segments"),
        "setup preflight should keep runtime opening segment validation for all opening kinds"
    );
    assert!(
        constant_opening_source.contains("verify_constant_tree_opening_root")
            && witness_opening_source.contains("verify_witness_stage_opening_root")
            && fri_opening_source.contains("verify_fri_query_path")
            && fri_opening_source.contains("validate_pcs_fri_opening_folds_from_units"),
        "opening validators should keep Merkle path and FRI fold checks"
    );
}

#[test]
fn lean_query_plan_binding_tracks_runtime_transcript_opening_checks() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_path = crate_root.join("../../lean/Lzvm/QueryPlanBinding.lean");
    let lean_source =
        std::fs::read_to_string(&lean_path).expect("Lean query plan binding source should read");
    let lean_root_path = crate_root.join("../../lean/Lzvm.lean");
    let lean_root_source =
        std::fs::read_to_string(&lean_root_path).expect("top-level Lean source should read");
    let setup_preflight_path = crate_root.join("src/setup_preflight.rs");
    let setup_preflight_source =
        std::fs::read_to_string(&setup_preflight_path).expect("setup preflight source should read");
    let query_plan_path = crate_root.join("src/pcs_query_plan.rs");
    let query_plan_source =
        std::fs::read_to_string(&query_plan_path).expect("PCS query plan source should read");
    let transcript_segments_path = crate_root.join("src/pcs_transcript_segments.rs");
    let transcript_segments_source = std::fs::read_to_string(&transcript_segments_path)
        .expect("PCS transcript segments source should read");

    assert!(
        lean_root_source.contains("import Lzvm.QueryPlanBinding"),
        "top-level Lean module should include the query plan binding model"
    );
    assert!(
        lean_source.contains("structure RuntimeQueryPlanBindingValidation")
            && lean_source.contains("runtime_query_plan_binding_checked_acceptance_sound"),
        "Lean should expose a checked query plan binding soundness theorem"
    );
    assert!(
        lean_source.contains("RuntimeChallengeSegmentBindingValidation")
            && lean_source.contains("RuntimeOpeningSegmentBindingValidation")
            && lean_source.contains("runtime_challenge_segment_binding_checked_acceptance_sound")
            && lean_source.contains("runtime_opening_segment_binding_checked_acceptance_sound"),
        "Lean query plan binding should compose challenge and opening segment soundness"
    );
    assert!(
        setup_preflight_source.contains("validate_pcs_query_plan_segments")
            && setup_preflight_source.contains("validate_constant_opening_segments")
            && setup_preflight_source.contains("validate_witness_opening_segments")
            && setup_preflight_source.contains("validate_optional_pcs_fri_opening_proof_segments"),
        "setup preflight should validate query plan before opening-dependent checks"
    );
    assert!(
        query_plan_source.contains("validate_transcript_pcs_query_plan_segments")
            && query_plan_source.contains("validate_seeded_pcs_query_plan_segments")
            && query_plan_source.contains("derive_pcs_final_query_challenge_from_segments")
            && query_plan_source.contains("build_pcs_query_plan_segment_from_challenge")
            && query_plan_source.contains("build_pcs_query_plan_segment_with_bindings")
            && query_plan_source.contains("validate_pcs_evaluation_units_match_query_units")
            && query_plan_source.contains("validate_pcs_fri_opening_units_match_query_units")
            && query_plan_source.contains("validate_unit_values_units_match_query_units"),
        "query plan validation should bind transcript-derived challenges and all query-indexed artifacts"
    );
    assert!(
        transcript_segments_source.contains("derive_pcs_transcript_challenges_from_segments")
            && transcript_segments_source
                .contains("validate_pcs_evaluation_units_match_query_units")
            && transcript_segments_source
                .contains("validate_pcs_fri_opening_units_match_query_units")
            && transcript_segments_source.contains("validate_unit_values_units_match_query_units"),
        "transcript segment checks should retain query-unit matching for each opened artifact"
    );
}

#[test]
fn lean_pipeline_binding_tracks_runtime_preflight_and_artifact_checks() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lean_path = crate_root.join("../../lean/Lzvm/PipelineBinding.lean");
    let lean_source =
        std::fs::read_to_string(&lean_path).expect("Lean pipeline binding source should read");
    let lean_root_path = crate_root.join("../../lean/Lzvm.lean");
    let lean_root_source =
        std::fs::read_to_string(&lean_root_path).expect("top-level Lean source should read");
    let proof_artifact_path = crate_root.join("src/proof_artifact.rs");
    let proof_artifact_source =
        std::fs::read_to_string(&proof_artifact_path).expect("proof artifact source should read");
    let setup_preflight_path = crate_root.join("src/setup_preflight.rs");
    let setup_preflight_source =
        std::fs::read_to_string(&setup_preflight_path).expect("setup preflight source should read");
    let proof_preflight_path = crate_root.join("src/proof_preflight.rs");
    let proof_preflight_source =
        std::fs::read_to_string(&proof_preflight_path).expect("proof preflight source should read");

    assert!(
        lean_root_source.contains("import Lzvm.PipelineBinding"),
        "top-level Lean module should include the runtime pipeline binding model"
    );
    assert!(
        lean_source.contains("structure RuntimePipelineBindingValidation")
            && lean_source.contains("runtime_pipeline_binding_checked_acceptance_sound"),
        "Lean should expose a checked runtime pipeline binding soundness theorem"
    );
    assert!(
        lean_source.contains("RuntimeEthBlockPublicInputBindingValidation")
            && lean_source.contains("RuntimeTraceConstraintArtifactBindingValidation")
            && lean_source.contains("RuntimeQueryPlanBindingValidation")
            && lean_source
                .contains("runtime_eth_block_public_input_binding_checked_acceptance_sound")
            && lean_source
                .contains("runtime_trace_constraint_artifact_binding_checked_acceptance_sound")
            && lean_source.contains("runtime_query_plan_binding_checked_acceptance_sound"),
        "Lean runtime pipeline binding should compose public input, trace, and query plan soundness"
    );
    assert!(
        proof_artifact_source.contains("fn validate_proof_bindings")
            && proof_artifact_source.contains("validate_setup_preflight")
            && proof_artifact_source.contains("validate_setup_preflight_hashes")
            && proof_artifact_source.contains("public inputs setup hash mismatch"),
        "proof artifact verification should retain binding and setup preflight checks"
    );
    assert!(
        setup_preflight_source.contains("validate_setup_preflight_hashes")
            && setup_preflight_source.contains("validate_proof_public_values_for_setup_preflight")
            && setup_preflight_source.contains("validate_optional_trace_constraint_segment")
            && setup_preflight_source.contains("validate_pcs_query_plan_segments")
            && setup_preflight_source.contains("validate_constant_opening_segments")
            && setup_preflight_source.contains("validate_witness_opening_segments")
            && setup_preflight_source.contains("validate_optional_pcs_fri_opening_proof_segments"),
        "setup preflight should keep all runtime proof-artifact binding checks wired together"
    );
    assert!(
        proof_preflight_source.contains("validate_proof_public_values_for_setup_preflight")
            && proof_preflight_source.contains("validate_trace_constraint_witness_commitments")
            && proof_preflight_source.contains("parse_trace_constraint_segment")
            && proof_preflight_source.contains("TRACE_CONSTRAINT_SEGMENT_ID"),
        "proof preflight should keep public values and trace constraint artifact checks"
    );
}

#[test]
fn guest_pc_trace_writes_use_direct_trace_builder_helpers() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_path = crate_root.join("src/guest_pc_trace_backend.rs");
    let source = std::fs::read_to_string(&source_path).expect("guest PC trace source should read");

    let scalar_body = function_body(&source, "fn write_column", "fn write_optional_column");
    assert!(
        scalar_body.contains("write_trusted_resolved_scalar_value"),
        "guest PC scalar trace writes should avoid slice-based builder dispatch"
    );
    assert!(
        !scalar_body.contains("write_resolved_column_values(row, column.resolved(), &[value])"),
        "guest PC scalar trace writes should not construct a one-value slice"
    );

    let pair_body = function_body(
        &source,
        "fn write_wide_column",
        "fn write_optional_wide_column",
    );
    assert!(
        pair_body.contains("write_trusted_resolved_pair_values"),
        "guest PC pair trace writes should avoid slice-based builder dispatch"
    );
    assert!(
        !pair_body.contains("write_resolved_column_values(row, column.resolved(), &values)"),
        "guest PC pair trace writes should not route through the slice builder API"
    );
}

#[test]
fn zisk_main_report_writes_stream_trace_values_without_row_value_vector() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_path = crate_root.join("src/guest_pc_trace_backend.rs");
    let source = std::fs::read_to_string(&source_path).expect("guest PC trace source should read");

    let write_body = function_body(
        &source,
        "fn write_zisk_main_report_columns",
        "fn write_zisk_main_row_columns",
    );
    assert!(
        !write_body.contains("let values = validate_and_apply_zisk_main_report("),
        "Zisk Main report writes should stream rows instead of collecting trace values first"
    );

    let validate_body = function_body(
        &source,
        "fn validate_and_apply_zisk_main_report",
        "fn lower_stateful_zisk_main_report_rows",
    );
    for allocation in [
        "Result<Vec<ZiskMainReportTraceValues>",
        "let mut values = Vec::with_capacity",
        "values.push(",
        "Ok(values)",
    ] {
        assert!(
            !validate_body.contains(allocation),
            "Zisk Main report validation should not allocate a trace-value vector with {allocation}"
        );
    }
}

#[test]
fn zisk_main_memory_access_validation_avoids_temporary_vectors() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_path = crate_root.join("src/guest_pc_trace_backend.rs");
    let source = std::fs::read_to_string(&source_path).expect("guest PC trace source should read");

    let matching_body = function_body(
        &source,
        "fn matching_memory_access",
        "fn validate_zisk_main_memory_accesses",
    );
    assert!(
        !matching_body.contains("collect"),
        "Zisk Main memory access matching should scan without collecting matches"
    );
    assert!(
        !matching_body.contains("Vec<"),
        "Zisk Main memory access matching should avoid temporary vectors"
    );

    let validate_body = function_body(
        &source,
        "fn validate_zisk_main_memory_accesses",
        "fn validate_zisk_main_precompile_memory_accesses",
    );
    for allocation in [
        "Vec::",
        "Vec<",
        "collect",
        "vec![",
        "expected.push",
        "expected.extend",
    ] {
        assert!(
            !validate_body.contains(allocation),
            "Zisk Main memory access validation should avoid temporary vectors with {allocation}"
        );
    }
}

#[test]
fn raw_fixed_material_uses_raw_row_major_bytes() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_path = crate_root.join("src/fixed_material.rs");
    let source = std::fs::read_to_string(&source_path).expect("fixed material source should read");

    let load_body = function_body(
        &source,
        "fn load_fixed_columns_material_inner",
        "fn fixed_columns_to_row_major_values",
    );
    assert!(
        load_body.contains("raw_fixed_bytes_to_row_major_values(&path, &raw_bytes)?"),
        "raw fixed-column material should reuse row-major raw bytes for Felt material"
    );
    assert!(
        load_body.contains("raw_layout_columns_match_physical_order(&raw_layout)"),
        "raw fixed-column material should guard raw-byte reuse by column order"
    );
    assert!(
        source.contains("fn raw_fixed_bytes_to_row_major_values"),
        "fixed material should provide a raw-byte row-major conversion path"
    );
}

#[test]
fn cuda_regular_constraints_borrow_felt_words_without_value_vectors() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_path = crate_root.join("src/regular_constraints/cuda.rs");
    let source =
        std::fs::read_to_string(&source_path).expect("regular constraints CUDA source should read");

    let body = function_body(
        &source,
        "pub(crate) fn try_evaluate_regular_constraints_cuda_base",
        "fn cuda_base_source_supported",
    );

    assert!(
        body.contains("Felt::as_u64_slice"),
        "CUDA regular constraints should borrow Felt words for CUDA inputs"
    );
    assert!(
        !body.contains(".map(|value| value.to_u64())"),
        "CUDA regular constraints should avoid per-call Felt-to-u64 value vectors"
    );
    assert!(
        body.contains("fixed_values_device_buffer"),
        "CUDA regular constraints should accept a proven row-major fixed device buffer"
    );
}

#[test]
fn regular_constraint_fixed_device_buffer_stays_out_of_cpu_inputs() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_path = crate_root.join("src/regular_constraints.rs");
    let source =
        std::fs::read_to_string(&source_path).expect("regular constraints source should read");

    let input_body = function_body(
        &source,
        "pub struct RegularConstraintInputs",
        "#[derive(Debug, Clone, PartialEq, Eq)]",
    );

    assert!(
        !source.contains("RegularFixedValuesDeviceBuffer"),
        "regular constraint inputs should not use a Send/Sync wrapper for CUDA fixed values"
    );
    assert!(
        !input_body.contains("CudaDeviceBuffer"),
        "regular constraint inputs should not carry CUDA device buffers into CPU workers"
    );
}

#[test]
fn witness_regular_constraints_pass_only_row_major_fixed_device_buffer() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_path = crate_root.join("src/witness_execution.rs");
    let source =
        std::fs::read_to_string(&source_path).expect("witness execution source should read");

    let body = function_body(
        &source,
        "fn validate_witness_regular_constraints",
        "fn map_regular_constraint_eval_error",
    );

    assert!(
        body.contains("material.row_major_device_buffer()"),
        "witness regular constraints should use the fixed material row-major device-buffer guard"
    );
    assert!(
        body.contains("evaluate_regular_constraints_first_violations_with_cuda_fixed_values"),
        "witness regular constraints should pass the guarded buffer only to the CUDA acceleration path"
    );
}

#[test]
fn constant_opening_uses_file_backed_row_paths() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_path = crate_root.join("src/constant_opening.rs");
    let source =
        std::fs::read_to_string(&source_path).expect("constant opening source should read");

    let body = function_body(
        &source,
        "pub fn build_constant_opening_segment",
        "fn field_digest_from_words",
    );

    assert!(
        body.contains("open_constant_tree_row_from_file"),
        "constant openings should read only queried rows and sibling paths"
    );
    assert!(
        body.contains("verify_constant_tree_opening_root"),
        "file-backed constant openings should still be checked against the scheduled root"
    );
    assert!(
        !body.contains("read_constant_tree_file_with_digest"),
        "constant openings should not hash full constant-tree files during proof finish"
    );
    assert!(
        !body.contains("read_constant_tree_file("),
        "constant openings should not materialize full constant-tree files during proof finish"
    );
}

#[test]
fn constant_opening_uses_prevalidated_tree_material_summaries() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let opening_path = crate_root.join("src/constant_opening.rs");
    let opening_source =
        std::fs::read_to_string(&opening_path).expect("constant opening source should read");
    let proof_path = crate_root.join("src/proof_artifact.rs");
    let proof_source =
        std::fs::read_to_string(&proof_path).expect("proof artifact source should read");

    assert!(
        opening_source.contains("validate_constant_opening_materials"),
        "constant-tree material validation should be available before proof finish"
    );
    assert!(
        opening_source.contains("build_constant_opening_segment_with_material_summaries"),
        "constant opening should accept prevalidated material summaries"
    );
    assert!(
        proof_source.contains("constant_tree_material_summaries"),
        "proof artifact requests should carry prevalidated constant-tree material summaries"
    );
    assert!(
        proof_source.contains("build_witness_proof_artifact_with_bindings_and_material_summaries"),
        "all-units non-transcript proof construction should consume prevalidated summaries"
    );
}

#[test]
fn guest_machine_advance_avoids_full_state_clone() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_path = crate_root.join("src/guest_machine/mod.rs");
    let source = std::fs::read_to_string(&source_path).expect("guest machine source should read");
    let body = function_body(
        &source,
        "fn advance_guest_machine_inner",
        "pub(crate) fn decode_current_guest_instruction",
    );

    assert!(
        !body.contains("state.clone()"),
        "guest machine advance should avoid cloning the full state on every instruction"
    );
    assert!(
        body.contains("GuestMachineStateCheckpoint"),
        "guest machine advance should use an explicit checkpoint for error rollback"
    );
}

#[test]
fn guest_machine_fetch_uses_specialized_memory_path() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_path = crate_root.join("src/guest_machine/mod.rs");
    let source = std::fs::read_to_string(&source_path).expect("guest machine source should read");
    let body = function_body(
        &source,
        "fn fetch_decode_guest_instruction",
        "fn execute_guest_instruction",
    );

    assert!(
        body.contains("memory.fetch_instruction("),
        "guest machine fetch should use the specialized memory path"
    );
    assert!(
        !body.contains("fetch_guest_instruction(memory"),
        "guest machine fetch should avoid the generic reader path in the hot loop"
    );
}

#[test]
fn guest_pc_trace_slice_reuses_prepared_instruction_for_advance() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_path = crate_root.join("src/guest_pc_trace_backend.rs");
    let source = std::fs::read_to_string(&source_path).expect("guest PC trace source should read");
    let body = function_body(
        &source,
        "fn run_guest_pc_trace_segment_slice",
        "fn zisk_main_instruction_max_rows",
    );

    assert!(
        body.contains("prepare_current_guest_instruction(memory, pc)"),
        "guest PC trace slices should keep the fetched instruction for row planning"
    );
    assert!(
        body.contains("advance_guest_machine_with_prepared_fcalls"),
        "guest PC trace slices should advance with the prepared instruction instead of fetching it again"
    );
    assert!(
        !body.contains("advance_guest_machine_with_fcalls(memory, state, handler)"),
        "guest PC trace slices should not refetch and decode the same instruction in the hot loop"
    );
}

#[test]
fn fri_opening_timing_reports_unit_tree_query_and_fold_work() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let timing_path = crate_root.join("src/proof_artifact_timing.rs");
    let timing_source =
        std::fs::read_to_string(&timing_path).expect("proof artifact timing source should read");
    let cli_path = crate_root.join("../lzvm-cli/src/prove_witness/proof_timing.rs");
    let cli_source = std::fs::read_to_string(&cli_path).expect("proof timing source should read");
    let opening_path = crate_root.join("src/prove_fri_opening.rs");
    let opening_source =
        std::fs::read_to_string(&opening_path).expect("FRI opening source should read");
    let fri_build_path = crate_root.join("src/pcs_fri/build.rs");
    let fri_build_source =
        std::fs::read_to_string(&fri_build_path).expect("FRI build source should read");

    assert!(
        timing_source.contains("fri_opening_unit_build")
            && timing_source.contains("fri_opening_layer_tree")
            && timing_source.contains("fri_opening_query")
            && timing_source.contains("fri_opening_fold")
            && timing_source.contains("fri_opening_unit_count")
            && timing_source.contains("fri_opening_layer_count")
            && timing_source.contains("fri_opening_query_count"),
        "proof artifact timing should expose FRI opening work shape"
    );
    assert!(
        cli_source.contains("finish_fri_opening_unit_build")
            && cli_source.contains("finish_fri_opening_layer_tree")
            && cli_source.contains("finish_fri_opening_query")
            && cli_source.contains("finish_fri_opening_fold")
            && cli_source.contains("finish_fri_opening_unit_count")
            && cli_source.contains("finish_fri_opening_layer_count")
            && cli_source.contains("finish_fri_opening_query_count"),
        "CLI timing output should report FRI opening sub-buckets and counts"
    );
    assert!(
        opening_source.contains("build_pcs_fri_opening_segment_from_transcript_values_with_timing")
            && opening_source.contains("build_pcs_fri_opening_segment_from_value_refs_with_timing")
            && opening_source.contains("PcsFriOpeningBuildTiming"),
        "proof artifact finish should be able to pass a FRI opening timing accumulator"
    );
    assert!(
        fri_build_source.contains("build_pcs_fri_opening_unit_with_timing")
            && fri_build_source.contains("record_fri_opening_duration")
            && fri_build_source.contains("timing.add_layer_tree")
            && fri_build_source.contains("timing.add_query_work")
            && fri_build_source.contains("timing.add_fold_work"),
        "FRI opening unit build should time tree, query, and fold work separately"
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
