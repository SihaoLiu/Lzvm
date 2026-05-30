__global__ void pack_poseidon2_width8_linear_round_row_major_inputs_kernel(
    const uint64_t* current_states,
    const uint64_t* row_values,
    uint64_t* packed,
    size_t row_count,
    size_t column_count,
    size_t offset,
    size_t chunk_len) {
    const size_t row_index = blockIdx.x * blockDim.x + threadIdx.x;
    if (row_index < row_count) {
        const size_t state_offset = row_index * kPoseidon2Width8;
        const size_t row_offset = row_index * column_count + offset;
        for (size_t word = 0; word < chunk_len; ++word) {
            packed[state_offset + word] = row_values[row_offset + word];
        }
        for (size_t word = 0; word < kPoseidon2HalfRounds; ++word) {
            packed[state_offset + kPoseidon2HalfRounds + word] =
                current_states[state_offset + word];
        }
    }
}

__global__ void pack_poseidon2_width16_linear_round_row_major_inputs_kernel(
    const uint64_t* current_states,
    const uint64_t* row_values,
    uint64_t* packed,
    size_t row_count,
    size_t column_count,
    size_t offset,
    size_t chunk_len) {
    const size_t row_index = blockIdx.x * blockDim.x + threadIdx.x;
    if (row_index < row_count) {
        const size_t state_offset = row_index * kPoseidon2Width16;
        const size_t row_offset = row_index * column_count + offset;
        for (size_t word = 0; word < chunk_len; ++word) {
            packed[state_offset + word] = row_values[row_offset + word];
        }
        for (size_t word = 0; word < kPoseidon2HalfRounds; ++word) {
            packed[state_offset + kPoseidon2Width16 - kPoseidon2HalfRounds + word] =
                current_states[state_offset + word];
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

    const size_t state_bytes = row_count * kPoseidon2Width8 * sizeof(uint64_t);
    DeviceBuffer<uint64_t> device_packed;

    LZVM_CUDA_RETURN_ON_ERROR(device_packed.reset(row_count * kPoseidon2Width8));
    LZVM_CUDA_RETURN_ON_ERROR(cudaMemset(device_packed.data(), 0, state_bytes));

    const size_t blocks = (row_count + kThreads - 1) / kThreads;
    pack_poseidon2_width8_linear_round_row_major_inputs_kernel<<<blocks, kThreads>>>(
        current_states, row_values, device_packed.data(), row_count, column_count, offset,
        chunk_len);
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_check_launch());
    return run_poseidon2_width8_on_device(device_packed.data(), device_out, row_count);
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

    const size_t state_bytes = row_count * kPoseidon2Width16 * sizeof(uint64_t);
    DeviceBuffer<uint64_t> device_packed;

    LZVM_CUDA_RETURN_ON_ERROR(device_packed.reset(row_count * kPoseidon2Width16));
    LZVM_CUDA_RETURN_ON_ERROR(cudaMemset(device_packed.data(), 0, state_bytes));

    const size_t blocks = (row_count + kThreads - 1) / kThreads;
    pack_poseidon2_width16_linear_round_row_major_inputs_kernel<<<blocks, kThreads>>>(
        current_states, row_values, device_packed.data(), row_count, column_count, offset,
        chunk_len);
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_check_launch());
    return run_poseidon2_width16_on_device(device_packed.data(), device_out, row_count);
}
