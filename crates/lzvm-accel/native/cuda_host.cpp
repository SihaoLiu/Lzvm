#include "cuda_host.hpp"

#include <cuda_runtime.h>

#include <chrono>
#include <cstdlib>
#include <cstdint>
#include <limits>
#include <mutex>
#include <unordered_map>
#include <vector>
#include <unistd.h>

namespace {

struct AllocationRecord {
    std::size_t bytes;
    int device;
};

struct CachedAllocation {
    void* ptr;
    std::size_t bytes;
    int device;
    cudaEvent_t ready_event;
};

struct SizeWaitStats {
    std::size_t bytes = 0;
    std::size_t count = 0;
    std::size_t wait_ns = 0;
};

constexpr std::size_t kMaxCachedBytes = std::size_t{16} << 30;
constexpr std::size_t kMaxCachedBlocksPerSize = 2;
constexpr std::size_t kPinnedCopyThreshold = std::size_t{1} << 20;
constexpr std::size_t kPendingCacheNoWaitBytes = std::size_t{1} << 20;
constexpr const char* kPendingCacheNoWaitBytesEnv =
    "LZVM_CUDA_PENDING_CACHE_NO_WAIT_BYTES";
constexpr std::size_t kCopyD2hSizeStatsSlots = 64;
constexpr std::size_t kEventSynchronizeSizeStatsSlots = 64;

std::mutex g_allocator_mutex;
std::unordered_map<void*, AllocationRecord> g_active_allocations;
std::vector<CachedAllocation> g_cached_allocations;
std::size_t g_cuda_malloc_calls = 0;
std::size_t g_cuda_malloc_bytes = 0;
std::size_t g_cuda_malloc_wait_ns = 0;
std::size_t g_cuda_malloc_max_wait_ns = 0;
std::size_t g_cuda_host_register_calls = 0;
std::size_t g_cuda_host_register_bytes = 0;
std::size_t g_cuda_host_register_wait_ns = 0;
std::size_t g_cuda_host_register_max_wait_ns = 0;
std::size_t g_cuda_host_unregister_calls = 0;
std::size_t g_cuda_host_unregister_wait_ns = 0;
std::size_t g_cuda_host_unregister_max_wait_ns = 0;
std::size_t g_cuda_copy_h2d_calls = 0;
std::size_t g_cuda_copy_h2d_bytes = 0;
std::size_t g_cuda_copy_h2d_wait_ns = 0;
std::size_t g_cuda_copy_h2d_max_wait_ns = 0;
std::size_t g_cuda_copy_d2h_calls = 0;
std::size_t g_cuda_copy_d2h_bytes = 0;
std::size_t g_cuda_copy_d2h_wait_ns = 0;
std::size_t g_cuda_copy_d2h_max_wait_ns = 0;
SizeWaitStats g_cuda_copy_d2h_by_size[kCopyD2hSizeStatsSlots] = {};
std::size_t g_cuda_direct_copy_d2h_calls = 0;
std::size_t g_cuda_direct_copy_d2h_bytes = 0;
std::size_t g_cuda_direct_copy_d2h_wait_ns = 0;
std::size_t g_cuda_direct_copy_d2h_max_wait_ns = 0;
SizeWaitStats g_cuda_direct_copy_d2h_by_size[kCopyD2hSizeStatsSlots] = {};
std::size_t g_cuda_copy_d2d_calls = 0;
std::size_t g_cuda_copy_d2d_bytes = 0;
std::size_t g_cuda_copy_d2d_wait_ns = 0;
std::size_t g_cuda_copy_d2d_max_wait_ns = 0;
std::size_t g_cuda_free_calls = 0;
std::size_t g_cuda_device_synchronize_calls = 0;
std::size_t g_cuda_event_query_calls = 0;
std::size_t g_cuda_event_query_ready_count = 0;
std::size_t g_cuda_event_query_not_ready_count = 0;
std::size_t g_cuda_event_synchronize_calls = 0;
std::size_t g_cuda_event_synchronize_bytes = 0;
std::size_t g_cuda_event_synchronize_max_bytes = 0;
std::size_t g_cuda_event_synchronize_wait_ns = 0;
std::size_t g_cuda_event_synchronize_max_wait_ns = 0;
SizeWaitStats
    g_cuda_event_synchronize_by_size[kEventSynchronizeSizeStatsSlots] = {};
std::size_t g_cuda_cached_reuse_count = 0;
std::size_t g_cuda_pending_reuse_count = 0;
std::size_t g_cuda_no_wait_bypass_count = 0;
std::size_t g_cuda_no_wait_bypass_bytes = 0;

struct RegisteredHostRange {
    void* base = nullptr;
    std::size_t bytes = 0;
    bool registered = false;
};

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

std::size_t saturated_add(std::size_t left, std::size_t right) {
    const auto max = std::numeric_limits<std::size_t>::max();
    if (max - left < right) {
        return max;
    }
    return left + right;
}

std::size_t saturated_multiply(std::size_t left, std::size_t right) {
    const auto max = std::numeric_limits<std::size_t>::max();
    if (left != 0 && right > max / left) {
        return max;
    }
    return left * right;
}

std::size_t parse_byte_limit_or_default(const char* value, std::size_t fallback) {
    if (value == nullptr || *value == '\0') {
        return fallback;
    }
    std::size_t parsed = 0;
    for (const char* cursor = value; *cursor != '\0'; ++cursor) {
        if (*cursor < '0' || *cursor > '9') {
            return fallback;
        }
        const std::size_t digit = static_cast<std::size_t>(*cursor - '0');
        const std::size_t max = std::numeric_limits<std::size_t>::max();
        if (parsed > (max - digit) / 10) {
            return fallback;
        }
        parsed = parsed * 10 + digit;
    }
    return parsed;
}

std::size_t pending_cache_no_wait_bytes(std::size_t fallback) {
    return parse_byte_limit_or_default(
        std::getenv(kPendingCacheNoWaitBytesEnv), fallback);
}

void record_wait_by_size(
    SizeWaitStats* stats, std::size_t slots, std::size_t bytes,
    std::size_t elapsed_ns) {
    for (std::size_t index = 0; index < slots; ++index) {
        SizeWaitStats& size_stats = stats[index];
        if (size_stats.count != 0 && size_stats.bytes != bytes) {
            continue;
        }
        if (size_stats.count == 0) {
            size_stats.bytes = bytes;
        }
        size_stats.count = saturated_add(size_stats.count, 1);
        size_stats.wait_ns = saturated_add(size_stats.wait_ns, elapsed_ns);
        return;
    }
}

void pick_hot_size_wait(
    const SizeWaitStats* stats, std::size_t slots, std::size_t* hot_bytes,
    std::size_t* hot_count, std::size_t* hot_wait_ns) {
    *hot_bytes = *hot_count = *hot_wait_ns = 0;
    for (std::size_t index = 0; index < slots; ++index) {
        const SizeWaitStats& size_stats = stats[index];
        const std::size_t bytes = size_stats.bytes;
        if (size_stats.wait_ns > *hot_wait_ns
            || (size_stats.wait_ns == *hot_wait_ns && bytes > *hot_bytes)) {
            *hot_bytes = bytes;
            *hot_count = size_stats.count;
            *hot_wait_ns = size_stats.wait_ns;
        }
    }
}

bool size_wait_precedes(const SizeWaitStats& candidate, std::size_t bytes,
                        std::size_t wait_ns) {
    return candidate.wait_ns > wait_ns ||
           (candidate.wait_ns == wait_ns && candidate.bytes > bytes);
}

void pick_two_hot_size_waits(
    const SizeWaitStats* stats,
    std::size_t slots,
    std::size_t* hot_bytes,
    std::size_t* hot_count,
    std::size_t* hot_wait_ns,
    std::size_t* second_hot_bytes,
    std::size_t* second_hot_count,
    std::size_t* second_hot_wait_ns) {
    *hot_bytes = *hot_count = *hot_wait_ns = 0;
    *second_hot_bytes = *second_hot_count = *second_hot_wait_ns = 0;
    for (std::size_t index = 0; index < slots; ++index) {
        const SizeWaitStats& size_stats = stats[index];
        if (size_stats.count == 0) {
            continue;
        }
        if (size_wait_precedes(size_stats, *hot_bytes, *hot_wait_ns)) {
            *second_hot_bytes = *hot_bytes;
            *second_hot_count = *hot_count;
            *second_hot_wait_ns = *hot_wait_ns;
            *hot_bytes = size_stats.bytes;
            *hot_count = size_stats.count;
            *hot_wait_ns = size_stats.wait_ns;
            continue;
        }
        if (size_stats.bytes != *hot_bytes &&
            size_wait_precedes(size_stats, *second_hot_bytes,
                               *second_hot_wait_ns)) {
            *second_hot_bytes = size_stats.bytes;
            *second_hot_count = size_stats.count;
            *second_hot_wait_ns = size_stats.wait_ns;
        }
    }
}

void record_event_synchronize_wait(std::size_t bytes, std::size_t elapsed_ns) {
    g_cuda_event_synchronize_wait_ns =
        saturated_add(g_cuda_event_synchronize_wait_ns, elapsed_ns);
    if (elapsed_ns > g_cuda_event_synchronize_max_wait_ns) {
        g_cuda_event_synchronize_max_wait_ns = elapsed_ns;
    }
    record_wait_by_size(g_cuda_event_synchronize_by_size,
                        kEventSynchronizeSizeStatsSlots, bytes, elapsed_ns);
}

void record_cuda_malloc_wait(std::size_t elapsed_ns) {
    g_cuda_malloc_wait_ns = saturated_add(g_cuda_malloc_wait_ns, elapsed_ns);
    if (elapsed_ns > g_cuda_malloc_max_wait_ns) {
        g_cuda_malloc_max_wait_ns = elapsed_ns;
    }
}

void record_cuda_host_register_wait(std::size_t elapsed_ns) {
    g_cuda_host_register_wait_ns =
        saturated_add(g_cuda_host_register_wait_ns, elapsed_ns);
    if (elapsed_ns > g_cuda_host_register_max_wait_ns) {
        g_cuda_host_register_max_wait_ns = elapsed_ns;
    }
}

void record_cuda_host_unregister_wait(std::size_t elapsed_ns) {
    g_cuda_host_unregister_wait_ns =
        saturated_add(g_cuda_host_unregister_wait_ns, elapsed_ns);
    if (elapsed_ns > g_cuda_host_unregister_max_wait_ns) {
        g_cuda_host_unregister_max_wait_ns = elapsed_ns;
    }
}

void record_cuda_copy_wait(
    std::size_t bytes,
    std::size_t elapsed_ns,
    std::size_t* calls,
    std::size_t* byte_count,
    std::size_t* wait_ns,
    std::size_t* max_wait_ns) {
    if (calls == nullptr || byte_count == nullptr || wait_ns == nullptr ||
        max_wait_ns == nullptr) {
        return;
    }
    *calls = saturated_add(*calls, 1);
    *byte_count = saturated_add(*byte_count, bytes);
    *wait_ns = saturated_add(*wait_ns, elapsed_ns);
    if (elapsed_ns > *max_wait_ns) {
        *max_wait_ns = elapsed_ns;
    }
}

void record_cuda_copy_h2d_wait(std::size_t bytes, std::size_t elapsed_ns) {
    record_cuda_copy_wait(
        bytes, elapsed_ns, &g_cuda_copy_h2d_calls, &g_cuda_copy_h2d_bytes,
        &g_cuda_copy_h2d_wait_ns, &g_cuda_copy_h2d_max_wait_ns);
}

void record_cuda_copy_d2h_wait(std::size_t bytes, std::size_t elapsed_ns) {
    record_cuda_copy_wait(
        bytes, elapsed_ns, &g_cuda_copy_d2h_calls, &g_cuda_copy_d2h_bytes,
        &g_cuda_copy_d2h_wait_ns, &g_cuda_copy_d2h_max_wait_ns);
    record_wait_by_size(
        g_cuda_copy_d2h_by_size, kCopyD2hSizeStatsSlots, bytes, elapsed_ns);
}

void record_cuda_direct_copy_d2h_wait(std::size_t bytes, std::size_t elapsed_ns) {
    record_cuda_copy_wait(
        bytes,
        elapsed_ns,
        &g_cuda_direct_copy_d2h_calls,
        &g_cuda_direct_copy_d2h_bytes,
        &g_cuda_direct_copy_d2h_wait_ns,
        &g_cuda_direct_copy_d2h_max_wait_ns);
    record_wait_by_size(
        g_cuda_direct_copy_d2h_by_size,
        kCopyD2hSizeStatsSlots,
        bytes,
        elapsed_ns);
}

void record_cuda_copy_d2d_wait(std::size_t bytes, std::size_t elapsed_ns) {
    record_cuda_copy_wait(
        bytes, elapsed_ns, &g_cuda_copy_d2d_calls, &g_cuda_copy_d2d_bytes,
        &g_cuda_copy_d2d_wait_ns, &g_cuda_copy_d2d_max_wait_ns);
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

std::size_t host_page_size() {
    static const std::size_t page_size = []() {
        const long value = sysconf(_SC_PAGESIZE);
        return value > 0 ? static_cast<std::size_t>(value) : std::size_t{4096};
    }();
    return page_size;
}

RegisteredHostRange register_large_host_copy(const void* src, std::size_t bytes) {
    RegisteredHostRange range;
    if (src == nullptr || bytes < kPinnedCopyThreshold) {
        return range;
    }

    const auto start = reinterpret_cast<std::uintptr_t>(src);
    if (bytes > std::numeric_limits<std::uintptr_t>::max() - start) {
        return range;
    }
    const std::uintptr_t end = start + bytes;
    const std::size_t page_size = host_page_size();
    if (page_size == 0) {
        return range;
    }

    const std::uintptr_t aligned_start = (start / page_size) * page_size;
    if (end > std::numeric_limits<std::uintptr_t>::max() - (page_size - 1)) {
        return range;
    }
    const std::uintptr_t aligned_end = ((end + page_size - 1) / page_size) * page_size;
    if (aligned_end < aligned_start) {
        return range;
    }

    range.base = reinterpret_cast<void*>(aligned_start);
    range.bytes = static_cast<std::size_t>(aligned_end - aligned_start);
    const auto register_started = std::chrono::steady_clock::now();
    const int status =
        static_cast<int>(cudaHostRegister(range.base, range.bytes, cudaHostRegisterDefault));
    {
        std::lock_guard<std::mutex> lock(g_allocator_mutex);
        ++g_cuda_host_register_calls;
        g_cuda_host_register_bytes =
            saturated_add(g_cuda_host_register_bytes, range.bytes);
        record_cuda_host_register_wait(saturated_nanoseconds_since(register_started));
    }
    range.registered = status == 0;
    return range;
}

int unregister_host_copy(const RegisteredHostRange& range) {
    if (!range.registered) {
        return 0;
    }
    const auto unregister_started = std::chrono::steady_clock::now();
    const int status = static_cast<int>(cudaHostUnregister(range.base));
    {
        std::lock_guard<std::mutex> lock(g_allocator_mutex);
        ++g_cuda_host_unregister_calls;
        record_cuda_host_unregister_wait(saturated_nanoseconds_since(unregister_started));
    }
    return status;
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

int record_allocation_ready_event(int device, cudaEvent_t* out) {
    if (out == nullptr) {
        return -1;
    }
    *out = nullptr;
    int previous_device = -1;
    int status = set_allocation_device(device, &previous_device);
    cudaEvent_t event = nullptr;
    if (status == 0) {
        status = static_cast<int>(cudaEventCreateWithFlags(&event, cudaEventDisableTiming));
    }
    if (status == 0) {
        status = static_cast<int>(cudaEventRecord(event, 0));
    }
    if (status != 0 && event != nullptr) {
        (void)cudaEventDestroy(event);
        event = nullptr;
    }
    const int restore_status = restore_device(previous_device);
    status = first_status(status, restore_status);
    if (status == 0) {
        *out = event;
    } else if (event != nullptr) {
        (void)cudaEventDestroy(event);
    }
    return status;
}

int free_cached_allocation_on_device(const CachedAllocation& allocation) {
    int previous_device = -1;
    int status = set_allocation_device(allocation.device, &previous_device);
    if (status == 0 && allocation.ready_event != nullptr) {
        ++g_cuda_event_synchronize_calls;
        g_cuda_event_synchronize_bytes += allocation.bytes;
        if (allocation.bytes > g_cuda_event_synchronize_max_bytes) {
            g_cuda_event_synchronize_max_bytes = allocation.bytes;
        }
        const auto wait_started = std::chrono::steady_clock::now();
        status = static_cast<int>(cudaEventSynchronize(allocation.ready_event));
        record_event_synchronize_wait(
            allocation.bytes, saturated_nanoseconds_since(wait_started));
        const int destroy_status = static_cast<int>(cudaEventDestroy(allocation.ready_event));
        status = first_status(status, destroy_status);
    }
    if (status == 0) {
        status = static_cast<int>(cudaFree(allocation.ptr));
    }
    const int restore_status = restore_device(previous_device);
    return first_status(status, restore_status);
}

int reuse_cached_allocation_locked(std::size_t index, void** out) {
    CachedAllocation allocation = g_cached_allocations[index];
    const auto inserted = g_active_allocations.emplace(
        allocation.ptr, AllocationRecord{allocation.bytes, allocation.device});
    if (!inserted.second) {
        return -1;
    }
    if (allocation.ready_event != nullptr) {
        const int destroy_status = static_cast<int>(cudaEventDestroy(allocation.ready_event));
        if (destroy_status != 0) {
            g_active_allocations.erase(allocation.ptr);
            return destroy_status;
        }
    }
    g_cached_allocations.erase(g_cached_allocations.begin() + index);
    *out = allocation.ptr;
    return 0;
}

int release_cached_blocks_locked() {
    int first = 0;
    for (std::size_t index = 0; index < g_cached_allocations.size();) {
        const CachedAllocation allocation = g_cached_allocations[index];
        const int status = free_cached_allocation_on_device(allocation);
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
        std::size_t pending_index = std::numeric_limits<std::size_t>::max();
        for (std::size_t index = 0; index < g_cached_allocations.size(); ++index) {
            const CachedAllocation& allocation = g_cached_allocations[index];
            if (allocation.device != device || allocation.bytes != bytes) {
                continue;
            }
            int status = cudaSuccess;
            if (allocation.ready_event != nullptr) {
                ++g_cuda_event_query_calls;
                status = static_cast<int>(cudaEventQuery(allocation.ready_event));
                if (status == cudaSuccess) {
                    ++g_cuda_event_query_ready_count;
                } else if (status == cudaErrorNotReady) {
                    ++g_cuda_event_query_not_ready_count;
                }
            }
            if (status == cudaSuccess) {
                const int reuse_status = reuse_cached_allocation_locked(index, out);
                if (reuse_status == 0) {
                    ++g_cuda_cached_reuse_count;
                }
                return reuse_status;
            }
            if (status == cudaErrorNotReady) {
                if (pending_index == std::numeric_limits<std::size_t>::max()) {
                    pending_index = index;
                }
                continue;
            }
            return status;
        }
        if (pending_index != std::numeric_limits<std::size_t>::max()) {
            if (bytes <= pending_cache_no_wait_bytes(kPendingCacheNoWaitBytes)) {
                ++g_cuda_no_wait_bypass_count;
                g_cuda_no_wait_bypass_bytes += bytes;
                pending_index = std::numeric_limits<std::size_t>::max();
            }
        }
        if (pending_index != std::numeric_limits<std::size_t>::max()) {
            CachedAllocation& allocation = g_cached_allocations[pending_index];
            if (allocation.ready_event != nullptr) {
                ++g_cuda_event_synchronize_calls;
                g_cuda_event_synchronize_bytes += allocation.bytes;
                if (allocation.bytes > g_cuda_event_synchronize_max_bytes) {
                    g_cuda_event_synchronize_max_bytes = allocation.bytes;
                }
                const auto wait_started = std::chrono::steady_clock::now();
                const int status = static_cast<int>(cudaEventSynchronize(allocation.ready_event));
                record_event_synchronize_wait(
                    allocation.bytes, saturated_nanoseconds_since(wait_started));
                if (status != 0) {
                    return status;
                }
            }
            const int reuse_status = reuse_cached_allocation_locked(pending_index, out);
            if (reuse_status == 0) {
                ++g_cuda_pending_reuse_count;
            }
            return reuse_status;
        }
    }

    void* ptr = nullptr;
    auto malloc_started = std::chrono::steady_clock::now();
    int status = static_cast<int>(cudaMalloc(&ptr, bytes));
    {
        std::lock_guard<std::mutex> lock(g_allocator_mutex);
        record_cuda_malloc_wait(saturated_nanoseconds_since(malloc_started));
    }
    if (status != 0) {
        int release_status = 0;
        {
            std::lock_guard<std::mutex> lock(g_allocator_mutex);
            release_status = release_cached_blocks_locked();
        }
        malloc_started = std::chrono::steady_clock::now();
        status = static_cast<int>(cudaMalloc(&ptr, bytes));
        {
            std::lock_guard<std::mutex> lock(g_allocator_mutex);
            record_cuda_malloc_wait(saturated_nanoseconds_since(malloc_started));
        }
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
        g_cuda_malloc_bytes += bytes;
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

    cudaEvent_t ready_event = nullptr;
    if (cache_candidate && record_allocation_ready_event(record.device, &ready_event) == 0) {
        try {
            std::lock_guard<std::mutex> lock(g_allocator_mutex);
            if (should_cache_allocation_locked(record.device, record.bytes)) {
                g_cached_allocations.push_back(
                    CachedAllocation{ptr, record.bytes, record.device, ready_event});
                return;
            }
        } catch (...) {
            if (ready_event != nullptr) {
                (void)cudaEventDestroy(ready_event);
            }
            (void)free_allocation_on_device(ptr, record.device);
            std::lock_guard<std::mutex> lock(g_allocator_mutex);
            ++g_cuda_free_calls;
            return;
        }
        if (ready_event != nullptr) {
            (void)cudaEventDestroy(ready_event);
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
            g_cuda_malloc_bytes = 0;
            g_cuda_malloc_wait_ns = 0;
            g_cuda_malloc_max_wait_ns = 0;
            g_cuda_host_register_calls = 0;
            g_cuda_host_register_bytes = 0;
            g_cuda_host_register_wait_ns = 0;
            g_cuda_host_register_max_wait_ns = 0;
            g_cuda_host_unregister_calls = 0;
            g_cuda_host_unregister_wait_ns = 0;
            g_cuda_host_unregister_max_wait_ns = 0;
            g_cuda_copy_h2d_calls = 0;
            g_cuda_copy_h2d_bytes = 0;
            g_cuda_copy_h2d_wait_ns = 0;
            g_cuda_copy_h2d_max_wait_ns = 0;
            g_cuda_copy_d2h_calls = 0;
            g_cuda_copy_d2h_bytes = 0;
            g_cuda_copy_d2h_wait_ns = 0;
            g_cuda_copy_d2h_max_wait_ns = 0;
            g_cuda_direct_copy_d2h_calls = 0;
            g_cuda_direct_copy_d2h_bytes = 0;
            g_cuda_direct_copy_d2h_wait_ns = 0;
            g_cuda_direct_copy_d2h_max_wait_ns = 0;
            g_cuda_copy_d2d_calls = 0;
            g_cuda_copy_d2d_bytes = 0;
            g_cuda_copy_d2d_wait_ns = 0;
            g_cuda_copy_d2d_max_wait_ns = 0;
            g_cuda_free_calls = 0;
            g_cuda_device_synchronize_calls = 0;
            g_cuda_event_query_calls = 0;
            g_cuda_event_query_ready_count = 0;
            g_cuda_event_query_not_ready_count = 0;
            g_cuda_event_synchronize_calls = 0;
            g_cuda_event_synchronize_bytes = 0;
            g_cuda_event_synchronize_max_bytes = 0;
            g_cuda_event_synchronize_wait_ns = 0;
            g_cuda_event_synchronize_max_wait_ns = 0;
            for (SizeWaitStats& size_stats : g_cuda_copy_d2h_by_size) {
                size_stats = SizeWaitStats{};
            }
            for (SizeWaitStats& size_stats : g_cuda_direct_copy_d2h_by_size) {
                size_stats = SizeWaitStats{};
            }
            for (SizeWaitStats& size_stats : g_cuda_event_synchronize_by_size) {
                size_stats = SizeWaitStats{};
            }
            g_cuda_cached_reuse_count = 0;
            g_cuda_pending_reuse_count = 0;
            g_cuda_no_wait_bypass_count = 0;
            g_cuda_no_wait_bypass_bytes = 0;
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
        out->cuda_malloc_bytes = g_cuda_malloc_bytes;
        out->cuda_malloc_wait_ns = g_cuda_malloc_wait_ns;
        out->cuda_malloc_max_wait_ns = g_cuda_malloc_max_wait_ns;
        out->cuda_host_register_calls = g_cuda_host_register_calls;
        out->cuda_host_register_bytes = g_cuda_host_register_bytes;
        out->cuda_host_register_wait_ns = g_cuda_host_register_wait_ns;
        out->cuda_host_register_max_wait_ns = g_cuda_host_register_max_wait_ns;
        out->cuda_host_unregister_calls = g_cuda_host_unregister_calls;
        out->cuda_host_unregister_wait_ns = g_cuda_host_unregister_wait_ns;
        out->cuda_host_unregister_max_wait_ns = g_cuda_host_unregister_max_wait_ns;
        out->cuda_copy_h2d_calls = g_cuda_copy_h2d_calls;
        out->cuda_copy_h2d_bytes = g_cuda_copy_h2d_bytes;
        out->cuda_copy_h2d_wait_ns = g_cuda_copy_h2d_wait_ns;
        out->cuda_copy_h2d_max_wait_ns = g_cuda_copy_h2d_max_wait_ns;
        out->cuda_copy_d2h_calls = g_cuda_copy_d2h_calls;
        out->cuda_copy_d2h_bytes = g_cuda_copy_d2h_bytes;
        out->cuda_copy_d2h_wait_ns = g_cuda_copy_d2h_wait_ns;
        out->cuda_copy_d2h_max_wait_ns = g_cuda_copy_d2h_max_wait_ns;
        pick_two_hot_size_waits(
            g_cuda_copy_d2h_by_size, kCopyD2hSizeStatsSlots,
            &out->cuda_copy_d2h_hot_bytes, &out->cuda_copy_d2h_hot_count,
            &out->cuda_copy_d2h_hot_wait_ns,
            &out->cuda_copy_d2h_second_hot_bytes,
            &out->cuda_copy_d2h_second_hot_count,
            &out->cuda_copy_d2h_second_hot_wait_ns);
        out->cuda_direct_copy_d2h_calls = g_cuda_direct_copy_d2h_calls;
        out->cuda_direct_copy_d2h_bytes = g_cuda_direct_copy_d2h_bytes;
        out->cuda_direct_copy_d2h_wait_ns = g_cuda_direct_copy_d2h_wait_ns;
        out->cuda_direct_copy_d2h_max_wait_ns = g_cuda_direct_copy_d2h_max_wait_ns;
        pick_hot_size_wait(
            g_cuda_direct_copy_d2h_by_size,
            kCopyD2hSizeStatsSlots,
            &out->cuda_direct_copy_d2h_hot_bytes,
            &out->cuda_direct_copy_d2h_hot_count,
            &out->cuda_direct_copy_d2h_hot_wait_ns);
        out->cuda_copy_d2d_calls = g_cuda_copy_d2d_calls;
        out->cuda_copy_d2d_bytes = g_cuda_copy_d2d_bytes;
        out->cuda_copy_d2d_wait_ns = g_cuda_copy_d2d_wait_ns;
        out->cuda_copy_d2d_max_wait_ns = g_cuda_copy_d2d_max_wait_ns;
        out->cuda_free_calls = g_cuda_free_calls;
        out->cuda_device_synchronize_calls = g_cuda_device_synchronize_calls;
        out->cached_blocks = g_cached_allocations.size();
        out->cached_bytes = cached_bytes_locked();
        out->cuda_event_query_calls = g_cuda_event_query_calls;
        out->cuda_event_query_ready_count = g_cuda_event_query_ready_count;
        out->cuda_event_query_not_ready_count = g_cuda_event_query_not_ready_count;
        out->cuda_event_synchronize_calls = g_cuda_event_synchronize_calls;
        out->cuda_event_synchronize_bytes = g_cuda_event_synchronize_bytes;
        out->cuda_event_synchronize_max_bytes = g_cuda_event_synchronize_max_bytes;
        out->cuda_event_synchronize_wait_ns = g_cuda_event_synchronize_wait_ns;
        out->cuda_event_synchronize_max_wait_ns = g_cuda_event_synchronize_max_wait_ns;
        pick_hot_size_wait(g_cuda_event_synchronize_by_size,
                           kEventSynchronizeSizeStatsSlots,
                           &out->cuda_event_synchronize_hot_bytes,
                           &out->cuda_event_synchronize_hot_count,
                           &out->cuda_event_synchronize_hot_wait_ns);
        out->cached_reuse_count = g_cuda_cached_reuse_count;
        out->pending_reuse_count = g_cuda_pending_reuse_count;
        out->no_wait_bypass_count = g_cuda_no_wait_bypass_count;
        out->no_wait_bypass_bytes = g_cuda_no_wait_bypass_bytes;
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
    const RegisteredHostRange registered = register_large_host_copy(src, bytes);
    const auto copy_started = std::chrono::steady_clock::now();
    const int status = static_cast<int>(cudaMemcpy(dst, src, bytes, cudaMemcpyHostToDevice));
    {
        std::lock_guard<std::mutex> lock(g_allocator_mutex);
        record_cuda_copy_h2d_wait(bytes, saturated_nanoseconds_since(copy_started));
    }
    const int unregister_status = unregister_host_copy(registered);
    return first_status(status, unregister_status);
}

extern "C" int lzvm_cuda_copy_h2d_bytes_on_stream(
    void* dst,
    const void* src,
    std::size_t bytes,
    void* stream) {
    if (bytes == 0) {
        return 0;
    }
    if (dst == nullptr || src == nullptr || stream == nullptr) {
        return -1;
    }
    const auto copy_started = std::chrono::steady_clock::now();
    const int status = static_cast<int>(cudaMemcpyAsync(
        dst, src, bytes, cudaMemcpyHostToDevice, static_cast<cudaStream_t>(stream)));
    {
        std::lock_guard<std::mutex> lock(g_allocator_mutex);
        record_cuda_copy_h2d_wait(bytes, saturated_nanoseconds_since(copy_started));
    }
    return status;
}

extern "C" int lzvm_cuda_copy_d2h_bytes(void* dst, const void* src, std::size_t bytes) {
    if (bytes == 0) {
        return 0;
    }
    if (dst == nullptr || src == nullptr) {
        return -1;
    }
    const auto copy_started = std::chrono::steady_clock::now();
    const int status = static_cast<int>(cudaMemcpy(dst, src, bytes, cudaMemcpyDeviceToHost));
    {
        std::lock_guard<std::mutex> lock(g_allocator_mutex);
        record_cuda_copy_d2h_wait(bytes, saturated_nanoseconds_since(copy_started));
    }
    return status;
}

extern "C" int lzvm_cuda_copy_d2h_bytes_on_stream(
    void* dst,
    const void* src,
    std::size_t bytes,
    void* stream) {
    if (bytes == 0) {
        return 0;
    }
    if (dst == nullptr || src == nullptr || stream == nullptr) {
        return -1;
    }
    const auto copy_started = std::chrono::steady_clock::now();
    const int status = static_cast<int>(cudaMemcpyAsync(
        dst, src, bytes, cudaMemcpyDeviceToHost, static_cast<cudaStream_t>(stream)));
    {
        std::lock_guard<std::mutex> lock(g_allocator_mutex);
        record_cuda_copy_d2h_wait(bytes, saturated_nanoseconds_since(copy_started));
    }
    return status;
}

extern "C" void lzvm_cuda_record_direct_copy_d2h_wait(
    std::size_t bytes,
    std::size_t elapsed_ns) {
    std::lock_guard<std::mutex> lock(g_allocator_mutex);
    record_cuda_direct_copy_d2h_wait(bytes, elapsed_ns);
}

extern "C" int lzvm_cuda_copy_h2d_row_slice_words(
    void* dst,
    const void* src,
    std::size_t row_count,
    std::size_t source_width_words,
    std::size_t start_word,
    std::size_t slice_width_words) {
    if (row_count == 0 || slice_width_words == 0) {
        return 0;
    }
    if (dst == nullptr || src == nullptr) {
        return -1;
    }
    if (source_width_words == 0 || start_word > source_width_words ||
        slice_width_words > source_width_words - start_word) {
        return -2;
    }

    constexpr std::size_t word_bytes = sizeof(std::uint64_t);
    if (source_width_words > std::numeric_limits<std::size_t>::max() / word_bytes ||
        slice_width_words > std::numeric_limits<std::size_t>::max() / word_bytes ||
        start_word > std::numeric_limits<std::size_t>::max() / word_bytes) {
        return -2;
    }

    const std::size_t dst_pitch = slice_width_words * word_bytes;
    const std::size_t src_pitch = source_width_words * word_bytes;
    const std::size_t width_bytes = slice_width_words * word_bytes;
    const auto* source = static_cast<const std::uint8_t*>(src) + start_word * word_bytes;
    const auto copy_started = std::chrono::steady_clock::now();
    const int status = static_cast<int>(
        cudaMemcpy2D(dst, dst_pitch, source, src_pitch, width_bytes, row_count,
                     cudaMemcpyHostToDevice));
    {
        std::lock_guard<std::mutex> lock(g_allocator_mutex);
        record_cuda_copy_h2d_wait(
            saturated_multiply(width_bytes, row_count),
            saturated_nanoseconds_since(copy_started));
    }
    return status;
}

extern "C" int lzvm_cuda_copy_d2d_row_slice_words(
    void* dst,
    const void* src,
    std::size_t row_count,
    std::size_t source_width_words,
    std::size_t start_word,
    std::size_t slice_width_words) {
    if (row_count == 0 || slice_width_words == 0) {
        return 0;
    }
    if (dst == nullptr || src == nullptr) {
        return -1;
    }
    if (source_width_words == 0 || start_word > source_width_words ||
        slice_width_words > source_width_words - start_word) {
        return -2;
    }

    constexpr std::size_t word_bytes = sizeof(std::uint64_t);
    if (source_width_words > std::numeric_limits<std::size_t>::max() / word_bytes ||
        slice_width_words > std::numeric_limits<std::size_t>::max() / word_bytes ||
        start_word > std::numeric_limits<std::size_t>::max() / word_bytes) {
        return -2;
    }

    const std::size_t dst_pitch = slice_width_words * word_bytes;
    const std::size_t src_pitch = source_width_words * word_bytes;
    const std::size_t width_bytes = slice_width_words * word_bytes;
    const auto* source = static_cast<const std::uint8_t*>(src) + start_word * word_bytes;
    const auto copy_started = std::chrono::steady_clock::now();
    const int status = static_cast<int>(
        cudaMemcpy2D(dst, dst_pitch, source, src_pitch, width_bytes, row_count,
                     cudaMemcpyDeviceToDevice));
    {
        std::lock_guard<std::mutex> lock(g_allocator_mutex);
        record_cuda_copy_d2d_wait(
            saturated_multiply(width_bytes, row_count),
            saturated_nanoseconds_since(copy_started));
    }
    return status;
}

extern "C" int lzvm_cuda_copy_d2d_row_slice_words_on_stream(
    void* dst,
    const void* src,
    std::size_t row_count,
    std::size_t source_width_words,
    std::size_t start_word,
    std::size_t slice_width_words,
    void* stream_raw) {
    if (row_count == 0 || slice_width_words == 0) {
        return 0;
    }
    if (dst == nullptr || src == nullptr || stream_raw == nullptr) {
        return -1;
    }
    if (source_width_words == 0 || start_word > source_width_words ||
        slice_width_words > source_width_words - start_word) {
        return -2;
    }

    constexpr std::size_t word_bytes = sizeof(std::uint64_t);
    if (source_width_words > std::numeric_limits<std::size_t>::max() / word_bytes ||
        slice_width_words > std::numeric_limits<std::size_t>::max() / word_bytes ||
        start_word > std::numeric_limits<std::size_t>::max() / word_bytes) {
        return -2;
    }

    const std::size_t dst_pitch = slice_width_words * word_bytes;
    const std::size_t src_pitch = source_width_words * word_bytes;
    const std::size_t width_bytes = slice_width_words * word_bytes;
    const auto* source = static_cast<const std::uint8_t*>(src) + start_word * word_bytes;
    const auto copy_started = std::chrono::steady_clock::now();
    const int status = static_cast<int>(
        cudaMemcpy2DAsync(dst, dst_pitch, source, src_pitch, width_bytes, row_count,
                          cudaMemcpyDeviceToDevice, static_cast<cudaStream_t>(stream_raw)));
    {
        std::lock_guard<std::mutex> lock(g_allocator_mutex);
        record_cuda_copy_d2d_wait(
            saturated_multiply(width_bytes, row_count),
            saturated_nanoseconds_since(copy_started));
    }
    return status;
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
    const auto copy_started = std::chrono::steady_clock::now();
    const int status = static_cast<int>(
        cudaMemcpy2D(dst, dst_pitch, src, src_pitch, width_bytes, state_count,
                     cudaMemcpyDeviceToHost));
    {
        std::lock_guard<std::mutex> lock(g_allocator_mutex);
        record_cuda_copy_d2h_wait(
            saturated_multiply(width_bytes, state_count),
            saturated_nanoseconds_since(copy_started));
    }
    return status;
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
