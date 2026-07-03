extern "C" int lzvm_cuda_expand_state_prefix_words(
    void* dst,
    const void* src,
    std::size_t state_count,
    std::size_t state_width_words,
    std::size_t prefix_words) {
    if (state_count == 0) {
        return 0;
    }
    if (state_width_words == 0 || prefix_words > state_width_words) {
        return -2;
    }
    if (dst == nullptr || (prefix_words != 0 && src == nullptr)) {
        return -1;
    }

    constexpr std::size_t word_bytes = sizeof(std::uint64_t);
    if (state_width_words > std::numeric_limits<std::size_t>::max() / word_bytes ||
        prefix_words > std::numeric_limits<std::size_t>::max() / word_bytes) {
        return -2;
    }

    const std::size_t dst_pitch = state_width_words * word_bytes;
    const std::size_t src_pitch = prefix_words * word_bytes;
    const std::size_t width_bytes = prefix_words * word_bytes;
    if (state_count > std::numeric_limits<std::size_t>::max() / dst_pitch) {
        return -2;
    }
    const std::size_t dst_bytes = state_count * dst_pitch;
    const int clear_status = static_cast<int>(cudaMemset(dst, 0, dst_bytes));
    if (clear_status != 0 || prefix_words == 0) {
        return clear_status;
    }

    const auto copy_started = std::chrono::steady_clock::now();
    const int status = static_cast<int>(
        cudaMemcpy2D(dst, dst_pitch, src, src_pitch, width_bytes, state_count,
                     cudaMemcpyHostToDevice));
    {
        std::lock_guard<std::mutex> lock(g_allocator_mutex);
        record_cuda_copy_h2d_wait(
            saturated_multiply(width_bytes, state_count),
            saturated_nanoseconds_since(copy_started));
    }
    return status;
}

extern "C" int lzvm_cuda_expand_state_prefix_words_device_to_device(
    void* dst,
    const void* src,
    std::size_t state_count,
    std::size_t state_width_words,
    std::size_t prefix_words) {
    if (state_count == 0) {
        return 0;
    }
    if (state_width_words == 0 || prefix_words > state_width_words) {
        return -2;
    }
    if (dst == nullptr || (prefix_words != 0 && src == nullptr)) {
        return -1;
    }

    constexpr std::size_t word_bytes = sizeof(std::uint64_t);
    if (state_width_words > std::numeric_limits<std::size_t>::max() / word_bytes ||
        prefix_words > std::numeric_limits<std::size_t>::max() / word_bytes) {
        return -2;
    }

    const std::size_t dst_pitch = state_width_words * word_bytes;
    const std::size_t src_pitch = prefix_words * word_bytes;
    const std::size_t width_bytes = prefix_words * word_bytes;
    if (state_count > std::numeric_limits<std::size_t>::max() / dst_pitch) {
        return -2;
    }
    const std::size_t dst_bytes = state_count * dst_pitch;
    const int clear_status = static_cast<int>(cudaMemset(dst, 0, dst_bytes));
    if (clear_status != 0 || prefix_words == 0) {
        return clear_status;
    }

    const auto copy_started = std::chrono::steady_clock::now();
    const int status = static_cast<int>(
        cudaMemcpy2D(dst, dst_pitch, src, src_pitch, width_bytes, state_count,
                     cudaMemcpyDeviceToDevice));
    {
        std::lock_guard<std::mutex> lock(g_allocator_mutex);
        record_cuda_copy_d2d_wait(
            saturated_multiply(width_bytes, state_count),
            saturated_nanoseconds_since(copy_started));
    }
    return status;
}

extern "C" int lzvm_cuda_expand_state_prefix_words_device_to_device_on_stream(
    void* dst,
    const void* src,
    std::size_t state_count,
    std::size_t state_width_words,
    std::size_t prefix_words,
    void* stream_raw) {
    if (state_count == 0) {
        return 0;
    }
    if (state_width_words == 0 || prefix_words > state_width_words) {
        return -2;
    }
    if (dst == nullptr || stream_raw == nullptr || (prefix_words != 0 && src == nullptr)) {
        return -1;
    }

    constexpr std::size_t word_bytes = sizeof(std::uint64_t);
    if (state_width_words > std::numeric_limits<std::size_t>::max() / word_bytes ||
        prefix_words > std::numeric_limits<std::size_t>::max() / word_bytes) {
        return -2;
    }

    const std::size_t dst_pitch = state_width_words * word_bytes;
    const std::size_t src_pitch = prefix_words * word_bytes;
    const std::size_t width_bytes = prefix_words * word_bytes;
    if (state_count > std::numeric_limits<std::size_t>::max() / dst_pitch) {
        return -2;
    }
    cudaStream_t stream = static_cast<cudaStream_t>(stream_raw);
    const std::size_t dst_bytes = state_count * dst_pitch;
    int status = static_cast<int>(cudaMemsetAsync(dst, 0, dst_bytes, stream));
    if (status != 0 || prefix_words == 0) {
        return status;
    }

    const auto copy_started = std::chrono::steady_clock::now();
    status = static_cast<int>(
        cudaMemcpy2DAsync(dst, dst_pitch, src, src_pitch, width_bytes, state_count,
                          cudaMemcpyDeviceToDevice, stream));
    {
        std::lock_guard<std::mutex> lock(g_allocator_mutex);
        record_cuda_copy_d2d_wait(
            saturated_multiply(width_bytes, state_count),
            saturated_nanoseconds_since(copy_started));
    }
    return status;
}
