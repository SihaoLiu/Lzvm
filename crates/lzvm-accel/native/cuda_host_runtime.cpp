#include "cuda_host.hpp"

#include <cuda_runtime.h>

extern "C" int lzvm_cuda_stream_create(void** out) {
    try {
        if (out == nullptr) {
            return -1;
        }
        *out = nullptr;
        cudaStream_t stream = nullptr;
        const int status = static_cast<int>(
            cudaStreamCreateWithFlags(&stream, cudaStreamNonBlocking));
        if (status == 0) {
            *out = stream;
        }
        return status;
    } catch (...) {
        return -1;
    }
}

extern "C" int lzvm_cuda_stream_destroy(void* stream) {
    try {
        if (stream == nullptr) {
            return 0;
        }
        return static_cast<int>(cudaStreamDestroy(static_cast<cudaStream_t>(stream)));
    } catch (...) {
        return -1;
    }
}

extern "C" int lzvm_cuda_stream_synchronize(void* stream) {
    try {
        return static_cast<int>(cudaStreamSynchronize(static_cast<cudaStream_t>(stream)));
    } catch (...) {
        return -1;
    }
}

extern "C" int lzvm_cuda_event_create(void** out) {
    try {
        if (out == nullptr) {
            return -1;
        }
        *out = nullptr;
        cudaEvent_t event = nullptr;
        const int status =
            static_cast<int>(cudaEventCreateWithFlags(&event, cudaEventDisableTiming));
        if (status == 0) {
            *out = event;
        }
        return status;
    } catch (...) {
        return -1;
    }
}

extern "C" int lzvm_cuda_event_destroy(void* event) {
    try {
        if (event == nullptr) {
            return 0;
        }
        return static_cast<int>(cudaEventDestroy(static_cast<cudaEvent_t>(event)));
    } catch (...) {
        return -1;
    }
}

extern "C" int lzvm_cuda_event_record(void* event, void* stream) {
    try {
        if (event == nullptr) {
            return -1;
        }
        return static_cast<int>(
            cudaEventRecord(static_cast<cudaEvent_t>(event), static_cast<cudaStream_t>(stream)));
    } catch (...) {
        return -1;
    }
}

extern "C" int lzvm_cuda_event_synchronize(void* event) {
    try {
        if (event == nullptr) {
            return -1;
        }
        return static_cast<int>(cudaEventSynchronize(static_cast<cudaEvent_t>(event)));
    } catch (...) {
        return -1;
    }
}

extern "C" int lzvm_cuda_stream_wait_event(void* stream, void* event) {
    try {
        if (event == nullptr) {
            return -1;
        }
        return static_cast<int>(cudaStreamWaitEvent(
            static_cast<cudaStream_t>(stream), static_cast<cudaEvent_t>(event), 0));
    } catch (...) {
        return -1;
    }
}

extern "C" int lzvm_cuda_memory_info(LzvmCudaMemoryInfo* out) {
    try {
        if (out == nullptr) {
            return -1;
        }
        std::size_t free_bytes = 0;
        std::size_t total_bytes = 0;
        const int status = static_cast<int>(cudaMemGetInfo(&free_bytes, &total_bytes));
        if (status != 0) {
            return status;
        }
        out->free_bytes = free_bytes;
        out->total_bytes = total_bytes;
        return 0;
    } catch (...) {
        return -1;
    }
}

extern "C" int lzvm_cuda_pinned_host_alloc(void** out, std::size_t bytes) {
    if (out == nullptr) {
        return -1;
    }
    *out = nullptr;
    if (bytes == 0) {
        return 0;
    }
    return static_cast<int>(cudaHostAlloc(out, bytes, cudaHostAllocDefault));
}

extern "C" void lzvm_cuda_pinned_host_free(void* ptr) {
    if (ptr != nullptr) {
        (void)cudaFreeHost(ptr);
    }
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

extern "C" int lzvm_cuda_memset_zero_bytes_on_stream(
    void* dst,
    std::size_t bytes,
    void* stream_raw) {
    if (bytes == 0) {
        return 0;
    }
    if (dst == nullptr || stream_raw == nullptr) {
        return -1;
    }
    return static_cast<int>(
        cudaMemsetAsync(dst, 0, bytes, static_cast<cudaStream_t>(stream_raw)));
}

extern "C" int lzvm_cuda_check_launch(void) {
    return static_cast<int>(cudaGetLastError());
}

extern "C" int lzvm_cuda_synchronize(void) {
    return static_cast<int>(cudaDeviceSynchronize());
}
