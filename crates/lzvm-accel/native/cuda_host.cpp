#include "cuda_host.hpp"

#include <cuda_runtime.h>

#include <cstdint>
#include <limits>
#include <mutex>
#include <unordered_map>
#include <vector>

namespace {

struct AllocationRecord {
    std::size_t bytes;
    int device;
};

struct CachedAllocation {
    void* ptr;
    std::size_t bytes;
    int device;
};

constexpr std::size_t kMaxCachedBytes = std::size_t{16} << 30;
constexpr std::size_t kMaxCachedBlocksPerSize = 2;

std::mutex g_allocator_mutex;
std::unordered_map<void*, AllocationRecord> g_active_allocations;
std::vector<CachedAllocation> g_cached_allocations;
std::size_t g_cuda_malloc_calls = 0;
std::size_t g_cuda_free_calls = 0;
std::size_t g_cuda_device_synchronize_calls = 0;

std::size_t cached_bytes_locked() {
    std::size_t total = 0;
    for (const CachedAllocation& allocation : g_cached_allocations) {
        total += allocation.bytes;
    }
    return total;
}

std::size_t cached_blocks_for_size_locked(int device, std::size_t bytes) {
    std::size_t count = 0;
    for (const CachedAllocation& allocation : g_cached_allocations) {
        if (allocation.device == device && allocation.bytes == bytes) {
            ++count;
        }
    }
    return count;
}

int first_status(int primary, int secondary) {
    return primary != 0 ? primary : secondary;
}

int set_allocation_device(int device, int* previous_device) {
    *previous_device = -1;
    const int current_status = static_cast<int>(cudaGetDevice(previous_device));
    if (current_status != 0) {
        return current_status;
    }
    if (*previous_device == device) {
        return 0;
    }
    return static_cast<int>(cudaSetDevice(device));
}

int restore_device(int previous_device) {
    if (previous_device < 0) {
        return 0;
    }
    return static_cast<int>(cudaSetDevice(previous_device));
}

int synchronize_allocation_device(int device) {
    int previous_device = -1;
    int status = set_allocation_device(device, &previous_device);
    if (status != 0) {
        return status;
    }
    {
        std::lock_guard<std::mutex> lock(g_allocator_mutex);
        ++g_cuda_device_synchronize_calls;
    }
    status = static_cast<int>(cudaDeviceSynchronize());
    const int restore_status = restore_device(previous_device);
    return first_status(status, restore_status);
}

int free_allocation_on_device(void* ptr, int device) {
    int previous_device = -1;
    int status = set_allocation_device(device, &previous_device);
    if (status == 0) {
        status = static_cast<int>(cudaFree(ptr));
    }
    const int restore_status = restore_device(previous_device);
    return first_status(status, restore_status);
}

int release_cached_blocks_locked() {
    int first = 0;
    for (std::size_t index = 0; index < g_cached_allocations.size();) {
        const CachedAllocation allocation = g_cached_allocations[index];
        const int status = free_allocation_on_device(allocation.ptr, allocation.device);
        ++g_cuda_free_calls;
        if (status == 0) {
            g_cached_allocations.erase(g_cached_allocations.begin() + index);
        } else {
            first = first_status(first, status);
            ++index;
        }
    }
    return first;
}

bool should_cache_allocation_locked(int device, std::size_t bytes) {
    if (bytes > kMaxCachedBytes) {
        return false;
    }
    if (cached_blocks_for_size_locked(device, bytes) >= kMaxCachedBlocksPerSize) {
        return false;
    }
    return cached_bytes_locked() <= kMaxCachedBytes - bytes;
}

int alloc_bytes_impl(void** out, std::size_t bytes) {
    if (out == nullptr) {
        return -1;
    }
    *out = nullptr;
    if (bytes == 0) {
        return 0;
    }
    int device = 0;
    const int device_status = static_cast<int>(cudaGetDevice(&device));
    if (device_status != 0) {
        return device_status;
    }

    {
        std::lock_guard<std::mutex> lock(g_allocator_mutex);
        for (auto it = g_cached_allocations.begin(); it != g_cached_allocations.end(); ++it) {
            if (it->device == device && it->bytes == bytes) {
                void* ptr = it->ptr;
                const auto inserted =
                    g_active_allocations.emplace(ptr, AllocationRecord{bytes, device});
                if (!inserted.second) {
                    return -1;
                }
                g_cached_allocations.erase(it);
                *out = ptr;
                return 0;
            }
        }
    }

    void* ptr = nullptr;
    int status = static_cast<int>(cudaMalloc(&ptr, bytes));
    if (status != 0) {
        int release_status = 0;
        {
            std::lock_guard<std::mutex> lock(g_allocator_mutex);
            release_status = release_cached_blocks_locked();
        }
        status = static_cast<int>(cudaMalloc(&ptr, bytes));
        if (status != 0) {
            status = first_status(release_status, status);
        }
    }
    if (status == 0) {
        bool active_recorded = false;
        try {
            std::lock_guard<std::mutex> lock(g_allocator_mutex);
            const auto inserted =
                g_active_allocations.emplace(ptr, AllocationRecord{bytes, device});
            active_recorded = inserted.second;
        } catch (...) {
            (void)free_allocation_on_device(ptr, device);
            std::lock_guard<std::mutex> lock(g_allocator_mutex);
            ++g_cuda_free_calls;
            throw;
        }
        if (!active_recorded) {
            (void)free_allocation_on_device(ptr, device);
            std::lock_guard<std::mutex> lock(g_allocator_mutex);
            ++g_cuda_free_calls;
            return -1;
        }
        *out = ptr;
        std::lock_guard<std::mutex> lock(g_allocator_mutex);
        ++g_cuda_malloc_calls;
    }
    return status;
}

void free_bytes_impl(void* ptr) {
    if (ptr == nullptr) {
        return;
    }

    AllocationRecord record{};
    bool found = false;
    bool cache_candidate = false;
    {
        std::lock_guard<std::mutex> lock(g_allocator_mutex);
        auto active = g_active_allocations.find(ptr);
        if (active != g_active_allocations.end()) {
            record = active->second;
            g_active_allocations.erase(active);
            found = true;
            cache_candidate = should_cache_allocation_locked(record.device, record.bytes);
        }
    }

    if (!found) {
        (void)cudaFree(ptr);
        std::lock_guard<std::mutex> lock(g_allocator_mutex);
        ++g_cuda_free_calls;
        return;
    }

    if (cache_candidate && synchronize_allocation_device(record.device) == 0) {
        try {
            std::lock_guard<std::mutex> lock(g_allocator_mutex);
            if (should_cache_allocation_locked(record.device, record.bytes)) {
                g_cached_allocations.push_back(CachedAllocation{ptr, record.bytes, record.device});
                return;
            }
        } catch (...) {
            (void)free_allocation_on_device(ptr, record.device);
            std::lock_guard<std::mutex> lock(g_allocator_mutex);
            ++g_cuda_free_calls;
            return;
        }
    }

    (void)free_allocation_on_device(ptr, record.device);
    {
        std::lock_guard<std::mutex> lock(g_allocator_mutex);
        ++g_cuda_free_calls;
    }
}

}  // namespace

extern "C" int lzvm_cuda_alloc_bytes(void** out, std::size_t bytes) {
    try {
        return alloc_bytes_impl(out, bytes);
    } catch (...) {
        if (out != nullptr) {
            *out = nullptr;
        }
        return -1;
    }
}

extern "C" void lzvm_cuda_free_bytes(void* ptr) {
    try {
        free_bytes_impl(ptr);
    } catch (...) {
        if (ptr != nullptr) {
            (void)cudaFree(ptr);
            std::lock_guard<std::mutex> lock(g_allocator_mutex);
            ++g_cuda_free_calls;
            return;
        }
    }
}

extern "C" int lzvm_cuda_allocator_clear_cache(void) {
    try {
        std::lock_guard<std::mutex> lock(g_allocator_mutex);
        const int status = release_cached_blocks_locked();
        if (status == 0) {
            g_cuda_malloc_calls = 0;
            g_cuda_free_calls = 0;
            g_cuda_device_synchronize_calls = 0;
        }
        return status;
    } catch (...) {
        return -1;
    }
}

extern "C" int lzvm_cuda_allocator_stats(LzvmCudaAllocatorStats* out) {
    try {
        if (out == nullptr) {
            return -1;
        }
        std::lock_guard<std::mutex> lock(g_allocator_mutex);
        out->cuda_malloc_calls = g_cuda_malloc_calls;
        out->cuda_free_calls = g_cuda_free_calls;
        out->cuda_device_synchronize_calls = g_cuda_device_synchronize_calls;
        out->cached_blocks = g_cached_allocations.size();
        out->cached_bytes = cached_bytes_locked();
        return 0;
    } catch (...) {
        return -1;
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
