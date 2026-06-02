#pragma once

#include <cstddef>
#include <cstdint>

struct LzvmCudaRegularConstraintEntry {
    std::uint32_t destination_id;
    std::uint32_t first_row;
    std::uint32_t last_row;
    std::uint32_t temp1_count;
    std::uint32_t ops_count;
    std::uint32_t ops_offset;
    std::uint32_t args_count;
    std::uint32_t args_offset;
};

struct LzvmCudaRegularStage {
    std::uint32_t stage_index;
    std::size_t column_count;
    const std::uint64_t* values;
    std::size_t value_count;
};

struct LzvmCudaRegularConstraintOutput {
    std::uint64_t row;
    std::uint64_t value;
    std::uint32_t found;
};

extern "C" int lzvm_cuda_alloc_bytes(void** out, std::size_t bytes);
extern "C" void lzvm_cuda_free_bytes(void* ptr);
extern "C" int lzvm_cuda_copy_h2d_bytes(void* dst, const void* src, std::size_t bytes);
extern "C" int lzvm_cuda_copy_d2h_bytes(void* dst, const void* src, std::size_t bytes);
extern "C" int lzvm_cuda_copy_d2h_state_prefix_words(
    void* dst,
    const void* src,
    std::size_t state_count,
    std::size_t state_width_words,
    std::size_t prefix_words);
extern "C" int lzvm_cuda_expand_state_prefix_words(
    void* dst,
    const void* src,
    std::size_t state_count,
    std::size_t state_width_words,
    std::size_t prefix_words);
extern "C" int lzvm_cuda_memset_zero_bytes(void* dst, std::size_t bytes);
extern "C" int lzvm_cuda_check_launch(void);
extern "C" int lzvm_cuda_synchronize(void);
extern "C" int lzvm_cuda_regular_constraints_base(
    const LzvmCudaRegularConstraintEntry* entries,
    std::size_t entry_count,
    const std::uint8_t* ops,
    std::size_t ops_count,
    const std::uint16_t* args,
    std::size_t args_count,
    const std::uint64_t* numbers,
    std::size_t number_count,
    const std::uint64_t* fixed_values,
    std::size_t fixed_value_count,
    const std::uint64_t* fixed_values_device,
    std::size_t fixed_column_count,
    const LzvmCudaRegularStage* stages,
    std::size_t stage_input_count,
    std::size_t stage_count,
    const std::int64_t* opening_point_offsets,
    std::size_t opening_point_offset_count,
    const std::uint64_t* unit_values,
    std::size_t unit_value_count,
    std::size_t domain_size,
    LzvmCudaRegularConstraintOutput* out);

template <typename T>
class DeviceBuffer {
public:
    DeviceBuffer() = default;
    DeviceBuffer(const DeviceBuffer&) = delete;
    DeviceBuffer& operator=(const DeviceBuffer&) = delete;

    ~DeviceBuffer() {
        release();
    }

    int reset(std::size_t count) {
        release();
        if (count == 0) {
            return 0;
        }
        const std::size_t bytes = count * sizeof(T);
        const int status = lzvm_cuda_alloc_bytes(reinterpret_cast<void**>(&ptr_), bytes);
        if (status == 0) {
            count_ = count;
        } else {
            ptr_ = nullptr;
            count_ = 0;
        }
        return status;
    }

    void release() {
        if (ptr_ != nullptr) {
            lzvm_cuda_free_bytes(ptr_);
            ptr_ = nullptr;
            count_ = 0;
        }
    }

    T* data() {
        return ptr_;
    }

    const T* data() const {
        return ptr_;
    }

    int copy_from_bytes(const void* src, std::size_t bytes) const {
        if (bytes == 0) {
            return 0;
        }
        return lzvm_cuda_copy_h2d_bytes(ptr_, src, bytes);
    }

    int copy_to_bytes(void* dst, std::size_t bytes) const {
        if (bytes == 0) {
            return 0;
        }
        return lzvm_cuda_copy_d2h_bytes(dst, ptr_, bytes);
    }

    std::size_t count() const {
        return count_;
    }

    std::size_t bytes() const {
        return count_ * sizeof(T);
    }

private:
    T* ptr_ = nullptr;
    std::size_t count_ = 0;
};
