#include "cuda_host.hpp"

#include <cuda_runtime.h>

#include <chrono>
#include <cstring>
#include <limits>

namespace {

std::size_t saturated_nanoseconds_since(std::chrono::steady_clock::time_point started) {
    const auto elapsed = std::chrono::steady_clock::now() - started;
    const auto ns = std::chrono::duration_cast<std::chrono::nanoseconds>(elapsed).count();
    if (ns <= 0) {
        return 0;
    }
    const auto max = std::numeric_limits<std::size_t>::max();
    if (static_cast<unsigned long long>(ns) > max) {
        return max;
    }
    return static_cast<std::size_t>(ns);
}

}  // namespace

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

extern "C" int lzvm_cuda_stream_begin_capture(void* stream) {
    try {
        if (stream == nullptr) {
            return -1;
        }
        return static_cast<int>(cudaStreamBeginCapture(
            static_cast<cudaStream_t>(stream), cudaStreamCaptureModeThreadLocal));
    } catch (...) {
        return -1;
    }
}

extern "C" int lzvm_cuda_stream_end_capture(void* stream, void** graph_out) {
    try {
        if (stream == nullptr || graph_out == nullptr) {
            return -1;
        }
        *graph_out = nullptr;
        cudaGraph_t graph = nullptr;
        const int status = static_cast<int>(
            cudaStreamEndCapture(static_cast<cudaStream_t>(stream), &graph));
        if (status == 0) {
            *graph_out = graph;
        } else if (graph != nullptr) {
            (void)cudaGraphDestroy(graph);
        }
        return status;
    } catch (...) {
        return -1;
    }
}

extern "C" int lzvm_cuda_graph_destroy(void* graph) {
    try {
        if (graph == nullptr) {
            return 0;
        }
        return static_cast<int>(cudaGraphDestroy(static_cast<cudaGraph_t>(graph)));
    } catch (...) {
        return -1;
    }
}

extern "C" int lzvm_cuda_graph_instantiate(void* graph, void** exec_out) {
    try {
        if (graph == nullptr || exec_out == nullptr) {
            return -1;
        }
        *exec_out = nullptr;
        cudaGraphExec_t exec = nullptr;
        const int status = static_cast<int>(
            cudaGraphInstantiate(&exec, static_cast<cudaGraph_t>(graph), 0));
        if (status == 0) {
            *exec_out = exec;
        }
        return status;
    } catch (...) {
        return -1;
    }
}

extern "C" int lzvm_cuda_graph_exec_update(void* exec, void* graph) {
    try {
        if (exec == nullptr || graph == nullptr) {
            return -1;
        }
        cudaGraphExecUpdateResultInfo result_info{};
        return static_cast<int>(cudaGraphExecUpdate(
            static_cast<cudaGraphExec_t>(exec), static_cast<cudaGraph_t>(graph), &result_info));
    } catch (...) {
        return -1;
    }
}

extern "C" int lzvm_cuda_graph_exec_destroy(void* exec) {
    try {
        if (exec == nullptr) {
            return 0;
        }
        return static_cast<int>(cudaGraphExecDestroy(static_cast<cudaGraphExec_t>(exec)));
    } catch (...) {
        return -1;
    }
}

extern "C" int lzvm_cuda_graph_launch(void* exec, void* stream) {
    try {
        if (exec == nullptr || stream == nullptr) {
            return -1;
        }
        return static_cast<int>(
            cudaGraphLaunch(static_cast<cudaGraphExec_t>(exec), static_cast<cudaStream_t>(stream)));
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

extern "C" int lzvm_cuda_pinned_host_alloc_copy_from(
    void** out,
    const void* src,
    std::size_t bytes) {
    if (out == nullptr) {
        return -1;
    }
    *out = nullptr;
    if (bytes == 0) {
        return 0;
    }
    if (src == nullptr) {
        return -1;
    }
    const int status =
        static_cast<int>(cudaHostAlloc(out, bytes, cudaHostAllocDefault));
    if (status != 0) {
        *out = nullptr;
        return status;
    }
    std::memcpy(*out, src, bytes);
    return 0;
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
    const auto sync_started = std::chrono::steady_clock::now();
    const int status = static_cast<int>(cudaDeviceSynchronize());
    lzvm_cuda_record_device_synchronize_wait(saturated_nanoseconds_since(sync_started));
    return status;
}
