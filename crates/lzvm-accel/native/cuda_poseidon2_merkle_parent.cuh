__global__ void poseidon2_width8_merkle_parent_kernel(
    const uint64_t* current_states,
    uint64_t* out,
    size_t child_state_count) {
    const size_t parent_index = blockIdx.x * blockDim.x + threadIdx.x;
    const size_t parent_state_count = (child_state_count + 1) / 2;
    if (parent_index < parent_state_count) {
        uint64_t state[kPoseidon2Width8] = {0, 0, 0, 0, 0, 0, 0, 0};
        const size_t first_child = parent_index * 2;
        for (size_t slot = 0; slot < 2; ++slot) {
            const size_t child_index = first_child + slot;
            if (child_index < child_state_count) {
                const size_t child_offset = child_index * kPoseidon2Width8;
                const size_t slot_offset = slot * kPoseidon2DigestWords;
                for (size_t word = 0; word < kPoseidon2DigestWords; ++word) {
                    state[slot_offset + word] = current_states[child_offset + word];
                }
            }
        }
        poseidon2_hash_width8(state);

        const size_t out_offset = parent_index * kPoseidon2Width8;
        for (size_t word = 0; word < kPoseidon2Width8; ++word) {
            out[out_offset + word] = state[word];
        }
    }
}

__global__ void poseidon2_width16_merkle_parent_kernel(
    const uint64_t* current_states,
    uint64_t* out,
    size_t child_state_count) {
    const size_t parent_index = blockIdx.x * blockDim.x + threadIdx.x;
    const size_t parent_state_count = (child_state_count + 3) / 4;
    if (parent_index < parent_state_count) {
        uint64_t state[kPoseidon2Width16] = {
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        };
        const size_t first_child = parent_index * 4;
        for (size_t slot = 0; slot < 4; ++slot) {
            const size_t child_index = first_child + slot;
            if (child_index < child_state_count) {
                const size_t child_offset = child_index * kPoseidon2Width16;
                const size_t slot_offset = slot * kPoseidon2DigestWords;
                for (size_t word = 0; word < kPoseidon2DigestWords; ++word) {
                    state[slot_offset + word] = current_states[child_offset + word];
                }
            }
        }
        poseidon2_hash_width16(state);

        const size_t out_offset = parent_index * kPoseidon2Width16;
        for (size_t word = 0; word < kPoseidon2Width16; ++word) {
            out[out_offset + word] = state[word];
        }
    }
}

int run_poseidon2_width8_merkle_parent_on_device(
    const uint64_t* device_values,
    uint64_t* device_out,
    size_t child_state_count) {
    if (child_state_count == 0) {
        return 0;
    }
    if (device_values == nullptr || device_out == nullptr) {
        return -1;
    }

    const size_t parent_state_count = (child_state_count + 1) / 2;
    const size_t blocks = (parent_state_count + kThreads - 1) / kThreads;
    poseidon2_width8_merkle_parent_kernel<<<blocks, kThreads>>>(
        device_values, device_out, child_state_count);
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_check_launch());
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_synchronize());
    return 0;
}

int run_poseidon2_width16_merkle_parent_on_device(
    const uint64_t* device_values,
    uint64_t* device_out,
    size_t child_state_count) {
    if (child_state_count == 0) {
        return 0;
    }
    if (device_values == nullptr || device_out == nullptr) {
        return -1;
    }

    const size_t parent_state_count = (child_state_count + 3) / 4;
    const size_t blocks = (parent_state_count + kThreads - 1) / kThreads;
    poseidon2_width16_merkle_parent_kernel<<<blocks, kThreads>>>(
        device_values, device_out, child_state_count);
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_check_launch());
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_synchronize());
    return 0;
}
