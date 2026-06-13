extern "C" int lzvm_cuda_goldilocks_validate_canonical_words_device(
    const uint64_t* values,
    size_t word_count,
    unsigned int* found) {
    if (found == nullptr) {
        return -1;
    }
    *found = 0;
    if (word_count == 0) {
        return 0;
    }
    if (values == nullptr) {
        return -1;
    }

    DeviceBuffer<unsigned int> device_found;
    LZVM_CUDA_RETURN_ON_ERROR(device_found.reset(1));
    const unsigned int initial_found = 0;
    LZVM_CUDA_RETURN_ON_ERROR(
        device_found.copy_from_bytes(&initial_found, sizeof(unsigned int)));
    const size_t blocks = (word_count + kThreads - 1) / kThreads;
    validate_canonical_words_kernel<<<blocks, kThreads>>>(
        values, word_count, device_found.data());
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_check_launch());
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_synchronize());
    LZVM_CUDA_RETURN_ON_ERROR(device_found.copy_to_bytes(found, sizeof(unsigned int)));
    return 0;
}

extern "C" int lzvm_cuda_goldilocks_begin_validate_canonical_words_device(
    const uint64_t* values,
    size_t word_count,
    unsigned int* device_found) {
    if (device_found == nullptr) {
        return -1;
    }
    if (word_count == 0) {
        return 0;
    }
    if (values == nullptr) {
        return -1;
    }

    const size_t blocks = (word_count + kThreads - 1) / kThreads;
    validate_canonical_words_kernel<<<blocks, kThreads>>>(values, word_count, device_found);
    return lzvm_cuda_check_launch();
}

extern "C" int lzvm_cuda_goldilocks_begin_validate_canonical_words_device_on_stream(
    const uint64_t* values,
    size_t word_count,
    unsigned int* device_found,
    void* stream_raw) {
    if (device_found == nullptr) {
        return -1;
    }
    if (word_count == 0) {
        return 0;
    }
    if (values == nullptr) {
        return -1;
    }

    cudaStream_t stream = static_cast<cudaStream_t>(stream_raw);
    const size_t blocks = (word_count + kThreads - 1) / kThreads;
    validate_canonical_words_kernel<<<blocks, kThreads, 0, stream>>>(
        values, word_count, device_found);
    return lzvm_cuda_check_launch();
}
