size_t merkle_opening_level_count(size_t state_count, size_t arity) {
    size_t level_count = 0;
    while (state_count > 1) {
        state_count = (state_count + arity - 1) / arity;
        ++level_count;
    }
    return level_count;
}

template <size_t Width, size_t Arity>
int run_poseidon2_merkle_opening_path_on_device(
    const uint64_t* device_values,
    uint64_t* root_out,
    uint64_t* siblings_out,
    size_t child_state_count,
    size_t query_index) {
    if (child_state_count == 0) {
        return -2;
    }
    if (query_index >= child_state_count) {
        return -2;
    }
    if (device_values == nullptr || root_out == nullptr) {
        return -1;
    }

    const size_t level_count = merkle_opening_level_count(child_state_count, Arity);
    const size_t sibling_word_count = level_count * (Arity - 1) * kPoseidon2DigestWords;
    if (sibling_word_count > 0 && siblings_out == nullptr) {
        return -1;
    }

    DeviceBuffer<uint64_t> device_siblings;
    LZVM_CUDA_RETURN_ON_ERROR(device_siblings.reset(sibling_word_count));

    const size_t first_parent_state_count = (child_state_count + Arity - 1) / Arity;
    const size_t second_parent_state_count =
        first_parent_state_count > 1 ? (first_parent_state_count + Arity - 1) / Arity : 0;
    DeviceBuffer<uint64_t> scratch_a;
    DeviceBuffer<uint64_t> scratch_b;
    LZVM_CUDA_RETURN_ON_ERROR(scratch_a.reset(first_parent_state_count * Width));
    LZVM_CUDA_RETURN_ON_ERROR(scratch_b.reset(second_parent_state_count * Width));

    const uint64_t* current = device_values;
    uint64_t* next = scratch_a.data();
    size_t state_count = child_state_count;
    size_t sibling_cursor = 0;
    size_t level_query = query_index;
    while (state_count > 1) {
        const size_t child_slot = level_query % Arity;
        const size_t group_start = (level_query / Arity) * Arity;
        for (size_t slot = 0; slot < Arity; ++slot) {
            if (slot == child_slot) {
                continue;
            }
            const size_t child_index = group_start + slot;
            uint64_t* sibling_out = device_siblings.data() + sibling_cursor;
            if (child_index < state_count) {
                LZVM_CUDA_RETURN_ON_ERROR(cudaMemcpyAsync(
                    sibling_out,
                    current + child_index * Width,
                    kPoseidon2DigestWords * sizeof(uint64_t),
                    cudaMemcpyDeviceToDevice));
            } else {
                LZVM_CUDA_RETURN_ON_ERROR(cudaMemsetAsync(
                    sibling_out,
                    0,
                    kPoseidon2DigestWords * sizeof(uint64_t)));
            }
            sibling_cursor += kPoseidon2DigestWords;
        }

        const size_t parent_state_count = (state_count + Arity - 1) / Arity;
        const size_t blocks = (parent_state_count + kThreads - 1) / kThreads;
        if constexpr (Width == kPoseidon2Width8) {
            poseidon2_width8_merkle_parent_kernel<<<blocks, kThreads>>>(
                current, next, state_count);
        } else {
            poseidon2_width16_merkle_parent_kernel<<<blocks, kThreads>>>(
                current, next, state_count);
        }
        LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_check_launch());

        current = next;
        state_count = parent_state_count;
        level_query /= Arity;
        next = next == scratch_a.data() ? scratch_b.data() : scratch_a.data();
    }

    LZVM_CUDA_RETURN_ON_ERROR(cudaMemcpy(
        root_out,
        current,
        kPoseidon2DigestWords * sizeof(uint64_t),
        cudaMemcpyDeviceToHost));
    if (sibling_word_count > 0) {
        LZVM_CUDA_RETURN_ON_ERROR(cudaMemcpy(
            siblings_out,
            device_siblings.data(),
            sibling_word_count * sizeof(uint64_t),
            cudaMemcpyDeviceToHost));
    }
    return 0;
}

int run_poseidon2_width8_merkle_opening_path_on_device(
    const uint64_t* device_values,
    uint64_t* root_out,
    uint64_t* siblings_out,
    size_t child_state_count,
    size_t query_index) {
    return run_poseidon2_merkle_opening_path_on_device<kPoseidon2Width8, 2>(
        device_values,
        root_out,
        siblings_out,
        child_state_count,
        query_index);
}

int run_poseidon2_width16_merkle_opening_path_on_device(
    const uint64_t* device_values,
    uint64_t* root_out,
    uint64_t* siblings_out,
    size_t child_state_count,
    size_t query_index) {
    return run_poseidon2_merkle_opening_path_on_device<kPoseidon2Width16, 4>(
        device_values,
        root_out,
        siblings_out,
        child_state_count,
        query_index);
}
