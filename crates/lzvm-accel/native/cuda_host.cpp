#include "cuda_host.hpp"

#include <cuda_runtime.h>

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
