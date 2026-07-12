extern "C" int lzvm_cuda_poseidon2_width8_linear_round_column_major_digest_device(
    const uint64_t* current_states,
    const uint64_t* column_values,
    uint64_t* out,
    size_t row_count,
    size_t column_count,
    size_t offset,
    size_t chunk_len) {
    return run_poseidon2_width8_linear_round_column_major_digest_on_device(
        current_states, column_values, out, row_count, column_count, offset, chunk_len);
}

extern "C" int lzvm_cuda_poseidon2_width8_linear_round_column_major_digest_device_on_stream(
    const uint64_t* current_states,
    const uint64_t* column_values,
    uint64_t* out,
    size_t row_count,
    size_t column_count,
    size_t offset,
    size_t chunk_len,
    void* stream_raw) {
    return run_poseidon2_width8_linear_round_column_major_digest_on_device_on_stream(
        current_states, column_values, out, row_count, column_count, offset, chunk_len,
        static_cast<cudaStream_t>(stream_raw));
}

extern "C" int lzvm_cuda_poseidon2_width16_linear_round_column_major_digest_device(
    const uint64_t* current_states,
    const uint64_t* column_values,
    uint64_t* out,
    size_t row_count,
    size_t column_count,
    size_t offset,
    size_t chunk_len) {
    return run_poseidon2_width16_linear_round_column_major_digest_on_device(
        current_states, column_values, out, row_count, column_count, offset, chunk_len);
}

extern "C" int lzvm_cuda_poseidon2_width16_linear_round_column_major_digest_device_on_stream(
    const uint64_t* current_states,
    const uint64_t* column_values,
    uint64_t* out,
    size_t row_count,
    size_t column_count,
    size_t offset,
    size_t chunk_len,
    void* stream_raw) {
    return run_poseidon2_width16_linear_round_column_major_digest_on_device_on_stream(
        current_states, column_values, out, row_count, column_count, offset, chunk_len,
        static_cast<cudaStream_t>(stream_raw));
}
