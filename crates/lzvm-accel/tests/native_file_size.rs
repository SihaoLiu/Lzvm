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
        run_ntt_body.contains("stage_len / 2 > kThreads"),
        "block twiddle path should be limited to stages with block-aligned offsets"
    );
}

#[test]
fn ntt_reuses_shared_twiddle_powers_inside_stage_kernels() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source = std::fs::read_to_string(crate_root.join("native/cuda_goldilocks_ntt.cuh"))
        .expect("CUDA NTT native source should read");

    assert!(
        source.contains("ntt_stage_thread_twiddle"),
        "NTT kernels should use a shared helper for thread-local twiddle factors"
    );
    assert!(
        source.contains("ntt_stage_prepare_thread_powers"),
        "NTT kernels should prepare per-block twiddle powers once in shared memory"
    );

    let block_body = function_body(
        &source,
        "__global__ void ntt_stage_block_twiddle_kernel",
        "cudaError_t run_ntt",
    );
    assert!(
        !block_body.contains("pow_mod(stage_twiddle, static_cast<size_t>(threadIdx.x))"),
        "block-twiddle NTT should not repeat twiddle exponentiation in every thread"
    );
}

#[test]
fn ntt_block_twiddle_base_reuses_thread_powers() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source = std::fs::read_to_string(crate_root.join("native/cuda_goldilocks_ntt.cuh"))
        .expect("CUDA NTT native source should read");
    let block_body = function_body(
        &source,
        "__global__ void ntt_stage_block_twiddle_kernel",
        "cudaError_t run_ntt",
    );

    assert!(
        source.contains("ntt_stage_block_base")
            && block_body.contains("ntt_stage_block_base(thread_powers, block_offset)"),
        "block-twiddle NTT should derive block bases from shared thread powers"
    );
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
