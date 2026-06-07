extern "C" int lzvm_cuda_poseidon2_width8_merkle_parent_device(
    const uint64_t* values,
    uint64_t* out,
    size_t child_state_count) {
    return run_poseidon2_width8_merkle_parent_on_device(values, out, child_state_count);
}

extern "C" int lzvm_cuda_poseidon2_width8_merkle_root_device(
    const uint64_t* values,
    uint64_t* out,
    size_t child_state_count) {
    return run_poseidon2_width8_merkle_root_on_device(values, out, child_state_count);
}

extern "C" int lzvm_cuda_poseidon2_width8_merkle_opening_path_device(
    const uint64_t* values,
    uint64_t* root_out,
    uint64_t* siblings_out,
    size_t child_state_count,
    size_t query_index) {
    return run_poseidon2_width8_merkle_opening_path_on_device(
        values, root_out, siblings_out, child_state_count, query_index);
}

extern "C" int lzvm_cuda_poseidon2_width8_merkle_digest_root_device(
    const uint64_t* values,
    uint64_t* out,
    size_t child_state_count) {
    return run_poseidon2_width8_merkle_digest_root_on_device(values, out, child_state_count);
}

extern "C" int lzvm_cuda_poseidon2_width8_merkle_digest_parent_device(
    const uint64_t* values,
    uint64_t* out,
    size_t child_state_count) {
    return run_poseidon2_width8_merkle_digest_parent_on_device(values, out, child_state_count);
}

extern "C" int lzvm_cuda_poseidon2_width8_merkle_digest_selected_parent_device(
    const uint64_t* values,
    uint64_t* out,
    size_t child_state_count,
    size_t parent_index) {
    return run_poseidon2_width8_merkle_digest_selected_parent_on_device(
        values, out, child_state_count, parent_index);
}

extern "C" int lzvm_cuda_poseidon2_width8_merkle_digest_opening_path_device(
    const uint64_t* values,
    uint64_t* root_out,
    uint64_t* siblings_out,
    size_t child_state_count,
    size_t query_index) {
    return run_poseidon2_width8_merkle_digest_opening_path_on_device(
        values, root_out, siblings_out, child_state_count, query_index);
}

extern "C" int lzvm_cuda_poseidon2_width8_merkle_digest_opening_prefix_device(
    const uint64_t* values,
    uint64_t* siblings_out,
    size_t child_state_count,
    size_t query_index,
    size_t prefix_level_count) {
    return run_poseidon2_width8_merkle_digest_opening_prefix_on_device(
        values, siblings_out, child_state_count, query_index, prefix_level_count);
}

extern "C" int lzvm_cuda_poseidon2_width8_merkle_digest_opening_prefix_batch_device(
    const uint64_t* values,
    const size_t* query_indices,
    uint64_t* siblings_out,
    size_t child_state_count,
    size_t query_count,
    size_t prefix_level_count) {
    return run_poseidon2_width8_merkle_digest_opening_prefix_batch_on_device(
        values, query_indices, siblings_out, child_state_count, query_count, prefix_level_count);
}

extern "C" int lzvm_cuda_poseidon2_width16_merkle_parent_device(
    const uint64_t* values,
    uint64_t* out,
    size_t child_state_count) {
    return run_poseidon2_width16_merkle_parent_on_device(values, out, child_state_count);
}

extern "C" int lzvm_cuda_poseidon2_width16_merkle_root_device(
    const uint64_t* values,
    uint64_t* out,
    size_t child_state_count) {
    return run_poseidon2_width16_merkle_root_on_device(values, out, child_state_count);
}

extern "C" int lzvm_cuda_poseidon2_width16_merkle_opening_path_device(
    const uint64_t* values,
    uint64_t* root_out,
    uint64_t* siblings_out,
    size_t child_state_count,
    size_t query_index) {
    return run_poseidon2_width16_merkle_opening_path_on_device(
        values, root_out, siblings_out, child_state_count, query_index);
}

extern "C" int lzvm_cuda_poseidon2_width16_merkle_digest_root_device(
    const uint64_t* values,
    uint64_t* out,
    size_t child_state_count) {
    return run_poseidon2_width16_merkle_digest_root_on_device(values, out, child_state_count);
}

extern "C" int lzvm_cuda_poseidon2_width16_merkle_digest_parent_device(
    const uint64_t* values,
    uint64_t* out,
    size_t child_state_count) {
    return run_poseidon2_width16_merkle_digest_parent_on_device(values, out, child_state_count);
}

extern "C" int lzvm_cuda_poseidon2_width16_merkle_digest_selected_parent_device(
    const uint64_t* values,
    uint64_t* out,
    size_t child_state_count,
    size_t parent_index) {
    return run_poseidon2_width16_merkle_digest_selected_parent_on_device(
        values, out, child_state_count, parent_index);
}

extern "C" int lzvm_cuda_poseidon2_width16_merkle_digest_opening_path_device(
    const uint64_t* values,
    uint64_t* root_out,
    uint64_t* siblings_out,
    size_t child_state_count,
    size_t query_index) {
    return run_poseidon2_width16_merkle_digest_opening_path_on_device(
        values, root_out, siblings_out, child_state_count, query_index);
}

extern "C" int lzvm_cuda_poseidon2_width16_merkle_digest_opening_prefix_device(
    const uint64_t* values,
    uint64_t* siblings_out,
    size_t child_state_count,
    size_t query_index,
    size_t prefix_level_count) {
    return run_poseidon2_width16_merkle_digest_opening_prefix_on_device(
        values, siblings_out, child_state_count, query_index, prefix_level_count);
}

extern "C" int lzvm_cuda_poseidon2_width16_merkle_digest_opening_prefix_batch_device(
    const uint64_t* values,
    const size_t* query_indices,
    uint64_t* siblings_out,
    size_t child_state_count,
    size_t query_count,
    size_t prefix_level_count) {
    return run_poseidon2_width16_merkle_digest_opening_prefix_batch_on_device(
        values, query_indices, siblings_out, child_state_count, query_count, prefix_level_count);
}
