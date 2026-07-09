constexpr size_t kPoseidon2MerkleDigestParentThreads = 128;

template <size_t Width, size_t Arity>
__global__ void poseidon2_merkle_digest_parent_kernel(
    const uint64_t* current_digests,
    uint64_t* out,
    size_t child_state_count) {
    const size_t parent_index = blockIdx.x * blockDim.x + threadIdx.x;
    const size_t parent_state_count = (child_state_count + Arity - 1) / Arity;
    if (parent_index < parent_state_count) {
        uint64_t state[Width] = {};
        const size_t first_child = parent_index * Arity;
        for (size_t slot = 0; slot < Arity; ++slot) {
            const size_t child_index = first_child + slot;
            if (child_index < child_state_count) {
                const size_t child_offset = child_index * kPoseidon2DigestWords;
                const size_t slot_offset = slot * kPoseidon2DigestWords;
                for (size_t word = 0; word < kPoseidon2DigestWords; ++word) {
                    state[slot_offset + word] = current_digests[child_offset + word];
                }
            }
        }
        if constexpr (Width == kPoseidon2Width8) {
            poseidon2_hash_width8(state);
        } else {
            poseidon2_hash_width16(state);
        }

        const size_t out_offset = parent_index * kPoseidon2DigestWords;
        for (size_t word = 0; word < kPoseidon2DigestWords; ++word) {
            out[out_offset + word] = state[word];
        }
    }
}

template <size_t Width, size_t Arity>
__global__ void poseidon2_merkle_digest_selected_parent_kernel(
    const uint64_t* current_digests,
    uint64_t* out,
    size_t child_state_count,
    size_t parent_index) {
    if (blockIdx.x != 0 || threadIdx.x != 0) {
        return;
    }

    uint64_t state[Width] = {};
    const size_t first_child = parent_index * Arity;
    for (size_t slot = 0; slot < Arity; ++slot) {
        const size_t child_index = first_child + slot;
        if (child_index < child_state_count) {
            const size_t child_offset = child_index * kPoseidon2DigestWords;
            const size_t slot_offset = slot * kPoseidon2DigestWords;
            for (size_t word = 0; word < kPoseidon2DigestWords; ++word) {
                state[slot_offset + word] = current_digests[child_offset + word];
            }
        }
    }
    if constexpr (Width == kPoseidon2Width8) {
        poseidon2_hash_width8(state);
    } else {
        poseidon2_hash_width16(state);
    }

    for (size_t word = 0; word < kPoseidon2DigestWords; ++word) {
        out[word] = state[word];
    }
}

template <size_t Arity>
__global__ void poseidon2_merkle_digest_prefix_sibling_gather_kernel(
    const uint64_t* current_digests,
    const size_t* query_indices,
    uint64_t* siblings_out,
    size_t state_count,
    size_t query_count,
    size_t level,
    size_t level_span,
    size_t row_sibling_words,
    size_t level_sibling_words) {
    const size_t word_index = blockIdx.x * blockDim.x + threadIdx.x;
    const size_t sibling_count = query_count * (Arity - 1);
    const size_t word_count = sibling_count * kPoseidon2DigestWords;
    if (word_index >= word_count) {
        return;
    }

    const size_t word = word_index % kPoseidon2DigestWords;
    const size_t sibling_index = word_index / kPoseidon2DigestWords;
    const size_t sibling_slot = sibling_index % (Arity - 1);
    const size_t query = sibling_index / (Arity - 1);
    const size_t level_query = query_indices[query] / level_span;
    const size_t child_slot = level_query % Arity;
    const size_t slot = sibling_slot + (sibling_slot >= child_slot ? 1 : 0);
    const size_t child_index = (level_query / Arity) * Arity + slot;
    const size_t out_index = query * row_sibling_words
        + level * level_sibling_words
        + sibling_slot * kPoseidon2DigestWords
        + word;
    siblings_out[out_index] = child_index < state_count
        ? current_digests[child_index * kPoseidon2DigestWords + word]
        : 0;
}

template <size_t Arity>
__global__ void poseidon2_merkle_digest_prefix_single_sibling_gather_kernel(
    const uint64_t* current_digests,
    uint64_t* siblings_out,
    size_t state_count,
    size_t level,
    size_t level_span,
    size_t query_index,
    size_t level_sibling_words) {
    const size_t word_index = blockIdx.x * blockDim.x + threadIdx.x;
    if (word_index >= level_sibling_words) {
        return;
    }

    const size_t word = word_index % kPoseidon2DigestWords;
    const size_t sibling_slot = word_index / kPoseidon2DigestWords;
    const size_t level_query = query_index / level_span;
    const size_t child_slot = level_query % Arity;
    const size_t slot = sibling_slot + (sibling_slot >= child_slot ? 1 : 0);
    const size_t child_index = (level_query / Arity) * Arity + slot;
    const size_t out_index =
        level * level_sibling_words + sibling_slot * kPoseidon2DigestWords + word;
    siblings_out[out_index] = child_index < state_count
        ? current_digests[child_index * kPoseidon2DigestWords + word]
        : 0;
}

template <size_t Width, size_t Arity>
int run_poseidon2_merkle_digest_root_on_device(
    const uint64_t* device_values,
    uint64_t* device_out,
    size_t child_state_count) {
    if (child_state_count == 0) {
        return 0;
    }
    if (device_values == nullptr || device_out == nullptr) {
        return -1;
    }
    if (child_state_count == 1) {
        return cudaMemcpy(
            device_out,
            device_values,
            kPoseidon2DigestWords * sizeof(uint64_t),
            cudaMemcpyDeviceToDevice);
    }

    const size_t first_parent_state_count = (child_state_count + Arity - 1) / Arity;
    const size_t second_parent_state_count =
        first_parent_state_count > 1 ? (first_parent_state_count + Arity - 1) / Arity : 0;
    DeviceBuffer<uint64_t> scratch_a;
    DeviceBuffer<uint64_t> scratch_b;
    LZVM_CUDA_RETURN_ON_ERROR(scratch_a.reset(
        first_parent_state_count * kPoseidon2DigestWords));
    LZVM_CUDA_RETURN_ON_ERROR(scratch_b.reset(
        second_parent_state_count * kPoseidon2DigestWords));

    const uint64_t* current = device_values;
    uint64_t* next = scratch_a.data();
    size_t state_count = child_state_count;
    while (state_count > 1) {
        const size_t parent_state_count = (state_count + Arity - 1) / Arity;
        const size_t blocks = (parent_state_count + kPoseidon2MerkleDigestParentThreads - 1)
            / kPoseidon2MerkleDigestParentThreads;
        poseidon2_merkle_digest_parent_kernel<Width, Arity>
            <<<blocks, kPoseidon2MerkleDigestParentThreads>>>(
            current, next, state_count);
        LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_check_launch());

        current = next;
        state_count = parent_state_count;
        next = next == scratch_a.data() ? scratch_b.data() : scratch_a.data();
    }

    return cudaMemcpy(
        device_out,
        current,
        kPoseidon2DigestWords * sizeof(uint64_t),
        cudaMemcpyDeviceToDevice);
}

template <size_t Width, size_t Arity>
int run_poseidon2_merkle_digest_parent_on_device(
    const uint64_t* device_values,
    uint64_t* device_out,
    size_t child_state_count) {
    if (child_state_count == 0) {
        return 0;
    }
    if (device_values == nullptr || device_out == nullptr) {
        return -1;
    }

    const size_t parent_state_count = (child_state_count + Arity - 1) / Arity;
    const size_t blocks = (parent_state_count + kPoseidon2MerkleDigestParentThreads - 1)
        / kPoseidon2MerkleDigestParentThreads;
    poseidon2_merkle_digest_parent_kernel<Width, Arity>
        <<<blocks, kPoseidon2MerkleDigestParentThreads>>>(
        device_values,
        device_out,
        child_state_count);
    return lzvm_cuda_check_launch();
}

template <size_t Width, size_t Arity>
int run_poseidon2_merkle_digest_selected_parent_on_device(
    const uint64_t* device_values,
    uint64_t* device_out,
    size_t child_state_count,
    size_t parent_index) {
    if (child_state_count == 0) {
        return -2;
    }
    const size_t parent_state_count = (child_state_count + Arity - 1) / Arity;
    if (parent_index >= parent_state_count) {
        return -2;
    }
    if (device_values == nullptr || device_out == nullptr) {
        return -1;
    }

    poseidon2_merkle_digest_selected_parent_kernel<Width, Arity><<<1, 1>>>(
        device_values,
        device_out,
        child_state_count,
        parent_index);
    return lzvm_cuda_check_launch();
}

template <size_t Width, size_t Arity>
int run_poseidon2_merkle_digest_opening_path_on_device(
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
    LZVM_CUDA_RETURN_ON_ERROR(scratch_a.reset(
        first_parent_state_count * kPoseidon2DigestWords));
    LZVM_CUDA_RETURN_ON_ERROR(scratch_b.reset(
        second_parent_state_count * kPoseidon2DigestWords));

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
                    current + child_index * kPoseidon2DigestWords,
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
        const size_t blocks = (parent_state_count + kPoseidon2MerkleDigestParentThreads - 1)
            / kPoseidon2MerkleDigestParentThreads;
        poseidon2_merkle_digest_parent_kernel<Width, Arity>
            <<<blocks, kPoseidon2MerkleDigestParentThreads>>>(
            current, next, state_count);
        LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_check_launch());

        current = next;
        state_count = parent_state_count;
        level_query /= Arity;
        next = next == scratch_a.data() ? scratch_b.data() : scratch_a.data();
    }

    LZVM_CUDA_RETURN_ON_ERROR(record_direct_d2h_copy(
        root_out,
        current,
        kPoseidon2DigestWords * sizeof(uint64_t)));
    if (sibling_word_count > 0) {
        LZVM_CUDA_RETURN_ON_ERROR(record_direct_d2h_copy(
            siblings_out,
            device_siblings.data(),
            sibling_word_count * sizeof(uint64_t)));
    }
    return 0;
}

template <size_t Width, size_t Arity>
int run_poseidon2_merkle_digest_opening_prefix_on_device(
    const uint64_t* device_values,
    uint64_t* siblings_out,
    size_t child_state_count,
    size_t query_index,
    size_t prefix_level_count) {
    if (child_state_count == 0) {
        return -2;
    }
    if (query_index >= child_state_count) {
        return -2;
    }
    const size_t full_level_count = merkle_opening_level_count(child_state_count, Arity);
    if (prefix_level_count > full_level_count) {
        return -2;
    }
    if (device_values == nullptr) {
        return -1;
    }

    const size_t sibling_word_count = prefix_level_count * (Arity - 1) * kPoseidon2DigestWords;
    if (sibling_word_count > 0 && siblings_out == nullptr) {
        return -1;
    }
    if (prefix_level_count == 0) {
        return 0;
    }

    DeviceBuffer<uint64_t> device_siblings;
    LZVM_CUDA_RETURN_ON_ERROR(device_siblings.reset(sibling_word_count));

    const size_t first_parent_state_count = (child_state_count + Arity - 1) / Arity;
    const size_t second_parent_state_count =
        first_parent_state_count > 1 ? (first_parent_state_count + Arity - 1) / Arity : 0;
    DeviceBuffer<uint64_t> scratch_a;
    DeviceBuffer<uint64_t> scratch_b;
    LZVM_CUDA_RETURN_ON_ERROR(scratch_a.reset(
        first_parent_state_count * kPoseidon2DigestWords));
    LZVM_CUDA_RETURN_ON_ERROR(scratch_b.reset(
        second_parent_state_count * kPoseidon2DigestWords));

    const uint64_t* current = device_values;
    uint64_t* next = scratch_a.data();
    size_t state_count = child_state_count;
    size_t sibling_cursor = 0;
    size_t level_query = query_index;
    for (size_t level = 0; level < prefix_level_count; ++level) {
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
                    current + child_index * kPoseidon2DigestWords,
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

        if (level + 1 < prefix_level_count) {
            const size_t parent_state_count = (state_count + Arity - 1) / Arity;
            const size_t blocks = (parent_state_count + kPoseidon2MerkleDigestParentThreads - 1)
                / kPoseidon2MerkleDigestParentThreads;
            poseidon2_merkle_digest_parent_kernel<Width, Arity>
                <<<blocks, kPoseidon2MerkleDigestParentThreads>>>(
                current, next, state_count);
            LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_check_launch());

            current = next;
            state_count = parent_state_count;
            level_query /= Arity;
            next = next == scratch_a.data() ? scratch_b.data() : scratch_a.data();
        }
    }

    LZVM_CUDA_RETURN_ON_ERROR(record_direct_d2h_copy(
        siblings_out,
        device_siblings.data(),
        sibling_word_count * sizeof(uint64_t)));
    return 0;
}

template <size_t Width, size_t Arity>
int run_poseidon2_merkle_digest_opening_prefix_batch_to_device(
    const uint64_t* device_values,
    const size_t* query_indices,
    uint64_t* siblings_out,
    size_t child_state_count,
    size_t query_count,
    size_t prefix_level_count) {
    if (child_state_count == 0) {
        return -2;
    }
    const size_t full_level_count = merkle_opening_level_count(child_state_count, Arity);
    if (prefix_level_count > full_level_count) {
        return -2;
    }
    if (device_values == nullptr) {
        return -1;
    }
    if (query_count > 0 && query_indices == nullptr) {
        return -1;
    }

    const size_t row_sibling_words = prefix_level_count * (Arity - 1) * kPoseidon2DigestWords;
    const size_t sibling_word_count = query_count * row_sibling_words;
    if (sibling_word_count > 0 && siblings_out == nullptr) {
        return -1;
    }
    if (query_count == 0 || prefix_level_count == 0) {
        return 0;
    }

    for (size_t query = 0; query < query_count; ++query) {
        if (query_indices[query] >= child_state_count) {
            return -2;
        }
    }
    DeviceBuffer<size_t> device_query_indices;
    if (query_count > 1) {
        LZVM_CUDA_RETURN_ON_ERROR(device_query_indices.reset(query_count));
        LZVM_CUDA_RETURN_ON_ERROR(device_query_indices.copy_from_bytes(
            query_indices, query_count * sizeof(size_t)));
    }

    const size_t first_parent_state_count = (child_state_count + Arity - 1) / Arity;
    const size_t second_parent_state_count =
        first_parent_state_count > 1 ? (first_parent_state_count + Arity - 1) / Arity : 0;
    DeviceBuffer<uint64_t> scratch_a;
    DeviceBuffer<uint64_t> scratch_b;
    LZVM_CUDA_RETURN_ON_ERROR(scratch_a.reset(
        first_parent_state_count * kPoseidon2DigestWords));
    LZVM_CUDA_RETURN_ON_ERROR(scratch_b.reset(
        second_parent_state_count * kPoseidon2DigestWords));

    const uint64_t* current = device_values;
    uint64_t* next = scratch_a.data();
    size_t state_count = child_state_count;
    const size_t level_sibling_words = (Arity - 1) * kPoseidon2DigestWords;
    size_t level_span = 1;
    for (size_t level = 0; level < prefix_level_count; ++level) {
        const size_t gather_words = query_count * level_sibling_words;
        const size_t gather_blocks = (gather_words + kThreads - 1) / kThreads;
        if (query_count == 1) {
            poseidon2_merkle_digest_prefix_single_sibling_gather_kernel<Arity>
                <<<gather_blocks, kThreads>>>(
                current,
                siblings_out,
                state_count,
                level,
                level_span,
                query_indices[0],
                level_sibling_words);
        } else {
            poseidon2_merkle_digest_prefix_sibling_gather_kernel<Arity>
                <<<gather_blocks, kThreads>>>(
                current,
                device_query_indices.data(),
                siblings_out,
                state_count,
                query_count,
                level,
                level_span,
                row_sibling_words,
                level_sibling_words);
        }
        LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_check_launch());

        if (level + 1 < prefix_level_count) {
            const size_t parent_state_count = (state_count + Arity - 1) / Arity;
            const size_t blocks = (parent_state_count + kPoseidon2MerkleDigestParentThreads - 1)
                / kPoseidon2MerkleDigestParentThreads;
            poseidon2_merkle_digest_parent_kernel<Width, Arity>
                <<<blocks, kPoseidon2MerkleDigestParentThreads>>>(
                current, next, state_count);
            LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_check_launch());

            current = next;
            state_count = parent_state_count;
            level_span *= Arity;
            next = next == scratch_a.data() ? scratch_b.data() : scratch_a.data();
        }
    }

    return 0;
}

template <size_t Width, size_t Arity>
int run_poseidon2_merkle_digest_opening_prefix_batch_on_device(
    const uint64_t* device_values,
    const size_t* query_indices,
    uint64_t* siblings_out,
    size_t child_state_count,
    size_t query_count,
    size_t prefix_level_count) {
    if (child_state_count == 0) {
        return -2;
    }
    const size_t full_level_count = merkle_opening_level_count(child_state_count, Arity);
    if (prefix_level_count > full_level_count) {
        return -2;
    }
    if (device_values == nullptr) {
        return -1;
    }
    if (query_count > 0 && query_indices == nullptr) {
        return -1;
    }

    const size_t row_sibling_words = prefix_level_count * (Arity - 1) * kPoseidon2DigestWords;
    const size_t sibling_word_count = query_count * row_sibling_words;
    if (sibling_word_count > 0 && siblings_out == nullptr) {
        return -1;
    }
    if (query_count == 0 || prefix_level_count == 0) {
        return 0;
    }

    DeviceBuffer<uint64_t> device_siblings;
    LZVM_CUDA_RETURN_ON_ERROR(device_siblings.reset(sibling_word_count));
    const int to_device_status = run_poseidon2_merkle_digest_opening_prefix_batch_to_device<Width, Arity>(
        device_values,
        query_indices,
        device_siblings.data(),
        child_state_count,
        query_count,
        prefix_level_count);
    LZVM_CUDA_RETURN_ON_ERROR(to_device_status);
    LZVM_CUDA_RETURN_ON_ERROR(record_direct_d2h_copy(
        siblings_out,
        device_siblings.data(),
        sibling_word_count * sizeof(uint64_t)));
    return 0;
}

int run_poseidon2_width8_merkle_digest_root_on_device(
    const uint64_t* device_values,
    uint64_t* device_out,
    size_t child_state_count) {
    return run_poseidon2_merkle_digest_root_on_device<kPoseidon2Width8, 2>(
        device_values, device_out, child_state_count);
}

int run_poseidon2_width16_merkle_digest_root_on_device(
    const uint64_t* device_values,
    uint64_t* device_out,
    size_t child_state_count) {
    return run_poseidon2_merkle_digest_root_on_device<kPoseidon2Width16, 4>(
        device_values, device_out, child_state_count);
}

int run_poseidon2_width8_merkle_digest_parent_on_device(
    const uint64_t* device_values,
    uint64_t* device_out,
    size_t child_state_count) {
    return run_poseidon2_merkle_digest_parent_on_device<kPoseidon2Width8, 2>(
        device_values, device_out, child_state_count);
}

int run_poseidon2_width16_merkle_digest_parent_on_device(
    const uint64_t* device_values,
    uint64_t* device_out,
    size_t child_state_count) {
    return run_poseidon2_merkle_digest_parent_on_device<kPoseidon2Width16, 4>(
        device_values, device_out, child_state_count);
}

int run_poseidon2_width8_merkle_digest_selected_parent_on_device(
    const uint64_t* device_values,
    uint64_t* device_out,
    size_t child_state_count,
    size_t parent_index) {
    return run_poseidon2_merkle_digest_selected_parent_on_device<kPoseidon2Width8, 2>(
        device_values, device_out, child_state_count, parent_index);
}

int run_poseidon2_width16_merkle_digest_selected_parent_on_device(
    const uint64_t* device_values,
    uint64_t* device_out,
    size_t child_state_count,
    size_t parent_index) {
    return run_poseidon2_merkle_digest_selected_parent_on_device<kPoseidon2Width16, 4>(
        device_values, device_out, child_state_count, parent_index);
}

int run_poseidon2_width8_merkle_digest_opening_path_on_device(
    const uint64_t* device_values,
    uint64_t* root_out,
    uint64_t* siblings_out,
    size_t child_state_count,
    size_t query_index) {
    return run_poseidon2_merkle_digest_opening_path_on_device<kPoseidon2Width8, 2>(
        device_values,
        root_out,
        siblings_out,
        child_state_count,
        query_index);
}

int run_poseidon2_width16_merkle_digest_opening_path_on_device(
    const uint64_t* device_values,
    uint64_t* root_out,
    uint64_t* siblings_out,
    size_t child_state_count,
    size_t query_index) {
    return run_poseidon2_merkle_digest_opening_path_on_device<kPoseidon2Width16, 4>(
        device_values,
        root_out,
        siblings_out,
        child_state_count,
        query_index);
}

int run_poseidon2_width8_merkle_digest_opening_prefix_on_device(
    const uint64_t* device_values,
    uint64_t* siblings_out,
    size_t child_state_count,
    size_t query_index,
    size_t prefix_level_count) {
    return run_poseidon2_merkle_digest_opening_prefix_on_device<kPoseidon2Width8, 2>(
        device_values,
        siblings_out,
        child_state_count,
        query_index,
        prefix_level_count);
}

int run_poseidon2_width16_merkle_digest_opening_prefix_on_device(
    const uint64_t* device_values,
    uint64_t* siblings_out,
    size_t child_state_count,
    size_t query_index,
    size_t prefix_level_count) {
    return run_poseidon2_merkle_digest_opening_prefix_on_device<kPoseidon2Width16, 4>(
        device_values,
        siblings_out,
        child_state_count,
        query_index,
        prefix_level_count);
}

int run_poseidon2_width8_merkle_digest_opening_prefix_batch_on_device(
    const uint64_t* device_values,
    const size_t* query_indices,
    uint64_t* siblings_out,
    size_t child_state_count,
    size_t query_count,
    size_t prefix_level_count) {
    return run_poseidon2_merkle_digest_opening_prefix_batch_on_device<kPoseidon2Width8, 2>(
        device_values,
        query_indices,
        siblings_out,
        child_state_count,
        query_count,
        prefix_level_count);
}

int run_poseidon2_width8_merkle_digest_opening_prefix_batch_to_device(
    const uint64_t* device_values,
    const size_t* query_indices,
    uint64_t* siblings_out,
    size_t child_state_count,
    size_t query_count,
    size_t prefix_level_count) {
    return run_poseidon2_merkle_digest_opening_prefix_batch_to_device<kPoseidon2Width8, 2>(
        device_values,
        query_indices,
        siblings_out,
        child_state_count,
        query_count,
        prefix_level_count);
}

int run_poseidon2_width16_merkle_digest_opening_prefix_batch_on_device(
    const uint64_t* device_values,
    const size_t* query_indices,
    uint64_t* siblings_out,
    size_t child_state_count,
    size_t query_count,
    size_t prefix_level_count) {
    return run_poseidon2_merkle_digest_opening_prefix_batch_on_device<kPoseidon2Width16, 4>(
        device_values,
        query_indices,
        siblings_out,
        child_state_count,
        query_count,
        prefix_level_count);
}

int run_poseidon2_width16_merkle_digest_opening_prefix_batch_to_device(
    const uint64_t* device_values,
    const size_t* query_indices,
    uint64_t* siblings_out,
    size_t child_state_count,
    size_t query_count,
    size_t prefix_level_count) {
    return run_poseidon2_merkle_digest_opening_prefix_batch_to_device<kPoseidon2Width16, 4>(
        device_values,
        query_indices,
        siblings_out,
        child_state_count,
        query_count,
        prefix_level_count);
}
