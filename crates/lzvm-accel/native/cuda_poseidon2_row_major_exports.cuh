extern "C" int lzvm_cuda_poseidon2_width8_linear_round_row_major_device(
    const uint64_t* current_states,
    const uint64_t* row_values,
    uint64_t* out,
    size_t row_count,
    size_t column_count,
    size_t offset,
    size_t chunk_len) {
    return run_poseidon2_width8_linear_round_row_major_on_device(
        current_states, row_values, out, row_count, column_count, offset, chunk_len);
}

extern "C" int lzvm_cuda_poseidon2_width16_linear_round_row_major_device(
    const uint64_t* current_states,
    const uint64_t* row_values,
    uint64_t* out,
    size_t row_count,
    size_t column_count,
    size_t offset,
    size_t chunk_len) {
    return run_poseidon2_width16_linear_round_row_major_on_device(
        current_states, row_values, out, row_count, column_count, offset, chunk_len);
}
