#include "cuda_host.hpp"

#include <cuda_runtime.h>

#include <cstdint>
#include <limits>

extern "C" int lzvm_cuda_alloc_bytes(void** out, std::size_t bytes) {
    if (out == nullptr) {
        return -1;
    }
    if (bytes == 0) {
        *out = nullptr;
        return 0;
    }
    return static_cast<int>(cudaMalloc(out, bytes));
}

extern "C" void lzvm_cuda_free_bytes(void* ptr) {
    if (ptr != nullptr) {
        (void)cudaFree(ptr);
    }
}

extern "C" int lzvm_cuda_copy_h2d_bytes(void* dst, const void* src, std::size_t bytes) {
    if (bytes == 0) {
        return 0;
    }
    if (dst == nullptr || src == nullptr) {
        return -1;
    }
    return static_cast<int>(cudaMemcpy(dst, src, bytes, cudaMemcpyHostToDevice));
}

extern "C" int lzvm_cuda_copy_d2h_bytes(void* dst, const void* src, std::size_t bytes) {
    if (bytes == 0) {
        return 0;
    }
    if (dst == nullptr || src == nullptr) {
        return -1;
    }
    return static_cast<int>(cudaMemcpy(dst, src, bytes, cudaMemcpyDeviceToHost));
}

extern "C" int lzvm_cuda_copy_d2h_state_prefix_words(
    void* dst,
    const void* src,
    std::size_t state_count,
    std::size_t state_width_words,
    std::size_t prefix_words) {
    if (state_count == 0 || prefix_words == 0) {
        return 0;
    }
    if (dst == nullptr || src == nullptr) {
        return -1;
    }
    if (state_width_words == 0 || prefix_words > state_width_words) {
        return -2;
    }

    constexpr std::size_t word_bytes = sizeof(std::uint64_t);
    if (state_width_words > std::numeric_limits<std::size_t>::max() / word_bytes ||
        prefix_words > std::numeric_limits<std::size_t>::max() / word_bytes) {
        return -2;
    }

    const std::size_t dst_pitch = prefix_words * word_bytes;
    const std::size_t src_pitch = state_width_words * word_bytes;
    const std::size_t width_bytes = prefix_words * word_bytes;
    return static_cast<int>(
        cudaMemcpy2D(dst, dst_pitch, src, src_pitch, width_bytes, state_count,
                     cudaMemcpyDeviceToHost));
}

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

    return static_cast<int>(
        cudaMemcpy2D(dst, dst_pitch, src, src_pitch, width_bytes, state_count,
                     cudaMemcpyHostToDevice));
}

extern "C" int lzvm_cuda_memset_zero_bytes(void* dst, std::size_t bytes) {
    if (bytes == 0) {
        return 0;
    }
    if (dst == nullptr) {
        return -1;
    }
    return static_cast<int>(cudaMemset(dst, 0, bytes));
}

extern "C" int lzvm_cuda_check_launch(void) {
    return static_cast<int>(cudaGetLastError());
}

extern "C" int lzvm_cuda_synchronize(void) {
    return static_cast<int>(cudaDeviceSynchronize());
}
