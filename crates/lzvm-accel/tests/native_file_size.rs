use std::path::Path;

const MAX_NATIVE_SOURCE_LINES: usize = 1_300;

#[test]
fn native_sources_stay_under_size_guideline() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let native_root = crate_root.join("native");
    let mut oversized = Vec::new();

    collect_oversized_sources(&native_root, &native_root, &mut oversized);

    assert!(
        oversized.is_empty(),
        "oversized native source files: {}",
        oversized.join(", ")
    );
}

#[test]
fn row_major_poseidon_helpers_do_not_allocate_packed_states() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_path = crate_root.join("native/cuda_poseidon2_row_major.cuh");
    let source =
        std::fs::read_to_string(&source_path).expect("row-major native source should read");

    assert!(
        !source.contains("DeviceBuffer<uint64_t>"),
        "row-major Poseidon helpers should not allocate a packed device buffer"
    );
    assert!(
        !source.contains("cudaMemset"),
        "row-major Poseidon helpers should not clear packed state buffers"
    );
}

#[test]
fn row_major_poseidon_helpers_defer_synchronization_to_callers() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_path = crate_root.join("native/cuda_poseidon2_row_major.cuh");
    let source =
        std::fs::read_to_string(&source_path).expect("row-major native source should read");

    for body in [
        function_body(
            &source,
            "int run_poseidon2_width8_linear_round_row_major_on_device",
            "int run_poseidon2_width16_linear_round_row_major_on_device",
        ),
        function_body(
            &source,
            "int run_poseidon2_width16_linear_round_row_major_on_device",
            "*** end of row-major source ***",
        ),
    ] {
        assert!(
            !body.contains("lzvm_cuda_synchronize"),
            "row-major Poseidon helpers should let the next same-stream operation synchronize"
        );
    }
}

#[test]
fn row_major_poseidon_has_digest_prefix_round_helpers() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_path = crate_root.join("native/cuda_poseidon2_row_major.cuh");
    let source =
        std::fs::read_to_string(&source_path).expect("row-major native source should read");

    assert!(
        source.contains("run_poseidon2_width8_linear_round_row_major_digest_on_device")
            && source.contains("run_poseidon2_width16_linear_round_row_major_digest_on_device"),
        "row-major linear rounds should expose digest-prefix helpers for Merkle levels"
    );
}

#[test]
fn h2d_copy_wrapper_uses_registered_host_memory_for_large_copies() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source = std::fs::read_to_string(crate_root.join("native/cuda_host.cpp"))
        .expect("CUDA host source should read");
    let body = function_body(
        &source,
        "extern \"C\" int lzvm_cuda_copy_h2d_bytes",
        "extern \"C\" int lzvm_cuda_copy_d2h_bytes",
    );

    assert!(
        source.contains("kPinnedCopyThreshold")
            && source.contains("cudaHostRegister")
            && body.contains("register_large_host_copy")
            && body.contains("unregister_host_copy"),
        "large H2D copies should use registered host memory until their source is GPU-resident"
    );
}

#[test]
fn normalize_shift_uses_block_powers() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source = std::fs::read_to_string(crate_root.join("native/cuda_field.cu"))
        .expect("CUDA field source should read");
    let body = function_body(
        &source,
        "__global__ void normalize_shift_and_pad_kernel",
        "#include \"cuda_goldilocks_ntt.cuh\"",
    );

    assert!(
        body.contains("block_shift") && !body.contains("pow_mod(shift, index)"),
        "normalize shift should not exponentiate the coset shift for every element"
    );
}

#[test]
fn merkle_parent_poseidon_helpers_do_not_allocate_packed_states() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source =
        std::fs::read_to_string(crate_root.join("native/cuda_poseidon2_merkle_parent.cuh"))
            .expect("Merkle parent native source should read");

    for body in [
        function_body(
            &source,
            "int run_poseidon2_width8_merkle_parent_on_device",
            "int run_poseidon2_width16_merkle_parent_on_device",
        ),
        function_body(
            &source,
            "int run_poseidon2_width16_merkle_parent_on_device",
            "*** end of Merkle parent source ***",
        ),
    ] {
        assert!(
            !body.contains("DeviceBuffer<uint64_t>"),
            "Merkle parent helpers should not allocate a packed device buffer"
        );
        assert!(
            !body.contains("cudaMemset"),
            "Merkle parent helpers should not clear packed state buffers"
        );
    }
}

#[test]
fn merkle_root_poseidon_helpers_defer_level_synchronization_to_root_read() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source = std::fs::read_to_string(crate_root.join("native/cuda_poseidon2_merkle_root.cuh"))
        .expect("Merkle root native source should read");

    for body in [
        function_body(
            &source,
            "int run_poseidon2_width8_merkle_root_on_device",
            "int run_poseidon2_width16_merkle_root_on_device",
        ),
        function_body(
            &source,
            "int run_poseidon2_width16_merkle_root_on_device",
            "*** end of Merkle root source ***",
        ),
    ] {
        assert!(
            !body.contains("lzvm_cuda_synchronize"),
            "Merkle root helpers should rely on same-stream ordering and the final root read"
        );
    }
}

#[test]
fn ntt_uses_block_twiddle_kernel_for_large_stages() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source = std::fs::read_to_string(crate_root.join("native/cuda_goldilocks_ntt.cuh"))
        .expect("CUDA NTT native source should read");
    let run_ntt_body = function_body(
        &source,
        "cudaError_t run_ntt",
        "int run_coset_extend_on_device_unsynced",
    );

    assert!(
        run_ntt_body.contains("ntt_stage_block_twiddle_kernel"),
        "large NTT stages should avoid per-butterfly full twiddle exponentiation"
    );
    assert!(
        run_ntt_body.contains("const size_t half = stage_len / 2;"),
        "NTT stage dispatch should derive the half-stage length once"
    );
    assert!(
        run_ntt_body.contains("else if (half > kThreads)"),
        "block twiddle path should be limited to stages with block-aligned offsets"
    );
    assert!(
        run_ntt_body.contains("if (!use_precomputed_factors)")
            && run_ntt_body.contains("ntt_stage_root_kernel"),
        "noncanonical roots should retain an explicit per-stage fallback"
    );
}

#[test]
fn ntt_reuses_precomputed_factors_in_non_block_stages() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source = std::fs::read_to_string(crate_root.join("native/cuda_goldilocks_ntt.cuh"))
        .expect("CUDA NTT native source should read");
    let stage_bodies = [
        function_body(
            &source,
            "__global__ void ntt_stage_kernel",
            "__global__ void ntt_stage_block_twiddle_kernel",
        ),
        function_body(
            &source,
            "__global__ void ntt_stage_column_group_kernel",
            "__global__ void ntt_stage_block_twiddle_column_group_kernel",
        ),
    ];

    assert!(
        source.contains("constexpr size_t kNttThreadFactorMinBits = 1;"),
        "setup-time factors should cover every NTT stage"
    );
    for body in stage_bodies {
        assert!(
            body.contains("ntt_stage_thread_factor_index(stage_bits, inverse_roots)")
                && body.contains("__ldg(&kNttThreadFactors["),
            "non-block NTT stages should load setup-time twiddle factors"
        );
    }
    assert!(
        !source.contains("ntt_stage_thread_twiddle_kernel")
            && !source.contains("ntt_stage_thread_twiddle_column_group_kernel"),
        "identical non-block NTT kernels should stay merged"
    );
}

#[test]
fn ntt_block_twiddle_kernel_reuses_precomputed_thread_factors() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source = std::fs::read_to_string(crate_root.join("native/cuda_goldilocks_ntt.cuh"))
        .expect("CUDA NTT native source should read");
    let block_body = function_body(
        &source,
        "__global__ void ntt_stage_block_twiddle_kernel",
        "cudaError_t run_ntt",
    );

    assert!(
        source.contains("kNttThreadFactors")
            && source.contains("kNttBlockTwiddles")
            && block_body.contains("__ldg(&kNttThreadFactors[")
            && block_body.contains("ntt_stage_block_base(kNttBlockTwiddles[factor_stage]"),
        "block-twiddle NTT should reuse setup-time thread factors and block twiddles"
    );
}

#[test]
fn row_major_ntt_groups_four_columns_per_launch_sequence() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let ntt_source = std::fs::read_to_string(crate_root.join("native/cuda_goldilocks_ntt.cuh"))
        .expect("CUDA NTT native source should read");
    let field_source = std::fs::read_to_string(crate_root.join("native/cuda_field.cu"))
        .expect("CUDA field source should read");
    let grouped_ntt_body = function_body(
        &ntt_source,
        "cudaError_t run_ntt_column_group",
        "int run_coset_extend_on_device_unsynced",
    );
    let grouped_normalize_body = function_body(
        &ntt_source,
        "__global__ void normalize_shift_and_pad_column_group_kernel",
        "cudaError_t run_ntt",
    );
    let grouped_bit_reverse_body = function_body(
        &ntt_source,
        "__global__ void bit_reverse_column_group_kernel",
        "__global__ void ntt_stage_column_group_kernel",
    );
    let grouped_block_stage_body = function_body(
        &ntt_source,
        "__global__ void ntt_stage_block_twiddle_column_group_kernel",
        "__global__ void normalize_shift_and_pad_column_group_kernel",
    );

    assert!(
        ntt_source.contains("constexpr size_t kNttColumnGroupSize = 4;")
            && grouped_ntt_body.contains("ntt_stage_column_group_kernel")
            && grouped_ntt_body.contains("ntt_stage_block_twiddle_column_group_kernel"),
        "grouped NTT should preserve the tuned four-column stage dispatch"
    );
    assert!(
        grouped_ntt_body.contains("column_count == 0 || column_count > kNttColumnGroupSize"),
        "grouped NTT should reject column counts outside its fixed kernel bound"
    );
    assert!(
        grouped_normalize_body
            .contains("for (size_t column = 0; column < kNttColumnGroupSize; ++column)")
            && !grouped_normalize_body.contains("blockIdx.y"),
        "grouped NTT normalization should compute each row factor once for the column group"
    );
    assert!(
        grouped_bit_reverse_body
            .contains("for (size_t column = 0; column < kNttColumnGroupSize; ++column)")
            && !grouped_bit_reverse_body.contains("blockIdx.y"),
        "grouped bit reversal should compute each reversed index once for the column group"
    );
    assert!(
        ntt_source.contains("constexpr size_t kNttBlockStageColumnsPerThread = 2;")
            && grouped_block_stage_body.contains("kNttBlockStageColumnsPerThread")
            && grouped_ntt_body.contains("block_stage_grid"),
        "large grouped NTT stages should reuse each block factor across tuned two-column groups"
    );

    for body in [
        function_body(
            &field_source,
            "int run_row_major_columns_device",
            "extern \"C\" int lzvm_cuda_goldilocks_coset_extend_row_major_columns_device",
        ),
        function_body(
            &field_source,
            "int run_row_major_columns_strided_device",
            "extern \"C\" int lzvm_cuda_goldilocks_coset_extend_row_major_columns_strided_device",
        ),
    ] {
        assert!(
            body.contains("run_coset_extend_column_groups_on_device_unsynced"),
            "row-major extension should launch NTTs in bounded column groups"
        );
    }
}

fn collect_oversized_sources(root: &Path, path: &Path, oversized: &mut Vec<String>) {
    let entries = std::fs::read_dir(path).expect("native source directory should read");
    for entry in entries {
        let entry = entry.expect("native source entry should read");
        let path = entry.path();
        if path.is_dir() {
            collect_oversized_sources(root, &path, oversized);
            continue;
        }
        if !matches!(
            path.extension().and_then(|value| value.to_str()),
            Some("cu" | "cuh" | "cpp" | "hpp")
        ) {
            continue;
        }

        let contents = std::fs::read_to_string(&path).unwrap_or_else(|error| {
            panic!(
                "native source file should read: {}: {error}",
                path.display()
            )
        });
        let line_count = contents.lines().count();
        if line_count > MAX_NATIVE_SOURCE_LINES {
            let relative = path.strip_prefix(root).unwrap_or(&path);
            oversized.push(format!("{} has {line_count} lines", relative.display()));
        }
    }
}

fn function_body<'a>(source: &'a str, start: &str, end: &str) -> &'a str {
    let body = source
        .split_once(start)
        .unwrap_or_else(|| panic!("missing function start: {start}"))
        .1;
    body.split_once(end).map_or(body, |(body, _)| body)
}
