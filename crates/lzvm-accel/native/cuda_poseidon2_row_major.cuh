__global__ void poseidon2_width8_linear_round_row_major_kernel(
    const uint64_t* current_states,
    const uint64_t* row_values,
    uint64_t* out,
    size_t row_count,
    size_t column_count,
    size_t offset,
    size_t chunk_len,
    size_t output_words) {
    const size_t row_index = blockIdx.x * blockDim.x + threadIdx.x;
    if (row_index < row_count) {
        uint64_t state[kPoseidon2Width8] = {0, 0, 0, 0, 0, 0, 0, 0};
        const size_t state_offset = row_index * kPoseidon2Width8;
        const size_t row_offset = row_index * column_count + offset;
        for (size_t word = 0; word < chunk_len; ++word) {
            state[word] = row_values[row_offset + word];
        }
        for (size_t word = 0; word < kPoseidon2HalfRounds; ++word) {
            state[kPoseidon2HalfRounds + word] = current_states[state_offset + word];
        }
        poseidon2_hash_width8(state);
        for (size_t word = 0; word < output_words; ++word) {
            out[state_offset + word] = state[word];
        }
    }
}

__global__ void poseidon2_width16_linear_round_row_major_kernel(
    const uint64_t* current_states,
    const uint64_t* row_values,
    uint64_t* out,
    size_t row_count,
    size_t column_count,
    size_t offset,
    size_t chunk_len,
    size_t output_words) {
    const size_t row_index = blockIdx.x * blockDim.x + threadIdx.x;
    if (row_index < row_count) {
        uint64_t state[kPoseidon2Width16] = {
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        };
        const size_t state_offset = row_index * kPoseidon2Width16;
        const size_t row_offset = row_index * column_count + offset;
        for (size_t word = 0; word < chunk_len; ++word) {
            state[word] = row_values[row_offset + word];
        }
        for (size_t word = 0; word < kPoseidon2HalfRounds; ++word) {
            state[kPoseidon2Width16 - kPoseidon2HalfRounds + word] =
                current_states[state_offset + word];
        }
        poseidon2_hash_width16(state);
        for (size_t word = 0; word < output_words; ++word) {
            out[state_offset + word] = state[word];
        }
    }
}

int run_poseidon2_width8_linear_round_row_major_on_device(
    const uint64_t* current_states,
    const uint64_t* row_values,
    uint64_t* device_out,
    size_t row_count,
    size_t column_count,
    size_t offset,
    size_t chunk_len) {
    if (row_count == 0) {
        return 0;
    }
    if (current_states == nullptr || row_values == nullptr || device_out == nullptr) {
        return -1;
    }
    if (chunk_len == 0 || chunk_len > kPoseidon2HalfRounds || offset > column_count ||
        chunk_len > column_count - offset) {
        return -2;
    }

    const size_t blocks = (row_count + kThreads - 1) / kThreads;
    poseidon2_width8_linear_round_row_major_kernel<<<blocks, kThreads>>>(
        current_states, row_values, device_out, row_count, column_count, offset, chunk_len,
        kPoseidon2Width8);
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_check_launch());
    return 0;
}

int run_poseidon2_width8_linear_round_row_major_digest_on_device_on_stream(
    const uint64_t* current_states,
    const uint64_t* row_values,
    uint64_t* device_out,
    size_t row_count,
    size_t column_count,
    size_t offset,
    size_t chunk_len,
    cudaStream_t stream) {
    if (row_count == 0) {
        return 0;
    }
    if (current_states == nullptr || row_values == nullptr || device_out == nullptr) {
        return -1;
    }
    if (chunk_len == 0 || chunk_len > kPoseidon2HalfRounds || offset > column_count ||
        chunk_len > column_count - offset) {
        return -2;
    }

    const size_t blocks = (row_count + kThreads - 1) / kThreads;
    poseidon2_width8_linear_round_row_major_kernel<<<blocks, kThreads, 0, stream>>>(
        current_states, row_values, device_out, row_count, column_count, offset, chunk_len,
        kPoseidon2DigestWords);
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_check_launch());
    return 0;
}

int run_poseidon2_width8_linear_round_row_major_digest_on_device(
    const uint64_t* current_states,
    const uint64_t* row_values,
    uint64_t* device_out,
    size_t row_count,
    size_t column_count,
    size_t offset,
    size_t chunk_len) {
    return run_poseidon2_width8_linear_round_row_major_digest_on_device_on_stream(
        current_states, row_values, device_out, row_count, column_count, offset, chunk_len, 0);
}

int run_poseidon2_width16_linear_round_row_major_on_device(
    const uint64_t* current_states,
    const uint64_t* row_values,
    uint64_t* device_out,
    size_t row_count,
    size_t column_count,
    size_t offset,
    size_t chunk_len) {
    if (row_count == 0) {
        return 0;
    }
    if (current_states == nullptr || row_values == nullptr || device_out == nullptr) {
        return -1;
    }
    if (chunk_len == 0 || chunk_len > kPoseidon2Width16 - kPoseidon2HalfRounds ||
        offset > column_count || chunk_len > column_count - offset) {
        return -2;
    }

    const size_t blocks = (row_count + kThreads - 1) / kThreads;
    poseidon2_width16_linear_round_row_major_kernel<<<blocks, kThreads>>>(
        current_states, row_values, device_out, row_count, column_count, offset, chunk_len,
        kPoseidon2Width16);
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_check_launch());
    return 0;
}

int run_poseidon2_width16_linear_round_row_major_digest_on_device_on_stream(
    const uint64_t* current_states,
    const uint64_t* row_values,
    uint64_t* device_out,
    size_t row_count,
    size_t column_count,
    size_t offset,
    size_t chunk_len,
    cudaStream_t stream) {
    if (row_count == 0) {
        return 0;
    }
    if (current_states == nullptr || row_values == nullptr || device_out == nullptr) {
        return -1;
    }
    if (chunk_len == 0 || chunk_len > kPoseidon2Width16 - kPoseidon2HalfRounds ||
        offset > column_count || chunk_len > column_count - offset) {
        return -2;
    }

    const size_t blocks = (row_count + kThreads - 1) / kThreads;
    poseidon2_width16_linear_round_row_major_kernel<<<blocks, kThreads, 0, stream>>>(
        current_states, row_values, device_out, row_count, column_count, offset, chunk_len,
        kPoseidon2DigestWords);
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_check_launch());
    return 0;
}

int run_poseidon2_width16_linear_round_row_major_digest_on_device(
    const uint64_t* current_states,
    const uint64_t* row_values,
    uint64_t* device_out,
    size_t row_count,
    size_t column_count,
    size_t offset,
    size_t chunk_len) {
    return run_poseidon2_width16_linear_round_row_major_digest_on_device_on_stream(
        current_states, row_values, device_out, row_count, column_count, offset, chunk_len, 0);
}
