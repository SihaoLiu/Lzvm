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
    std::size_t row_stride;
    std::size_t column_offset;
    const std::uint64_t* values;
    const std::uint64_t* values_device;
    std::size_t value_count;
};

struct LzvmCudaRegularConstraintOutput {
    std::uint64_t row;
    std::uint64_t value;
    std::uint32_t found;
};

struct LzvmCudaAllocatorStats {
    std::size_t cuda_malloc_calls;
    std::size_t cuda_malloc_bytes;
    std::size_t cuda_malloc_wait_ns;
    std::size_t cuda_malloc_max_wait_ns;
    std::size_t cuda_host_register_calls;
    std::size_t cuda_host_register_bytes;
    std::size_t cuda_host_register_wait_ns;
    std::size_t cuda_host_register_max_wait_ns;
    std::size_t cuda_host_unregister_calls;
    std::size_t cuda_host_unregister_wait_ns;
    std::size_t cuda_host_unregister_max_wait_ns;
    std::size_t cuda_copy_h2d_calls;
    std::size_t cuda_copy_h2d_bytes;
    std::size_t cuda_copy_h2d_wait_ns;
    std::size_t cuda_copy_h2d_max_wait_ns;
    std::size_t cuda_copy_d2h_calls;
    std::size_t cuda_copy_d2h_bytes;
    std::size_t cuda_copy_d2h_wait_ns;
    std::size_t cuda_copy_d2h_max_wait_ns;
    std::size_t cuda_copy_d2h_hot_bytes;
    std::size_t cuda_copy_d2h_hot_count;
    std::size_t cuda_copy_d2h_hot_wait_ns;
    std::size_t cuda_copy_d2d_calls;
    std::size_t cuda_copy_d2d_bytes;
    std::size_t cuda_copy_d2d_wait_ns;
    std::size_t cuda_copy_d2d_max_wait_ns;
    std::size_t cuda_free_calls;
    std::size_t cuda_device_synchronize_calls;
    std::size_t cached_blocks;
    std::size_t cached_bytes;
    std::size_t cuda_event_query_calls;
    std::size_t cuda_event_query_ready_count;
    std::size_t cuda_event_query_not_ready_count;
    std::size_t cuda_event_synchronize_calls;
    std::size_t cuda_event_synchronize_bytes;
    std::size_t cuda_event_synchronize_max_bytes;
    std::size_t cuda_event_synchronize_wait_ns;
    std::size_t cuda_event_synchronize_max_wait_ns;
    std::size_t cuda_event_synchronize_hot_bytes;
    std::size_t cuda_event_synchronize_hot_count;
    std::size_t cuda_event_synchronize_hot_wait_ns;
    std::size_t cached_reuse_count;
    std::size_t pending_reuse_count;
    std::size_t no_wait_bypass_count;
    std::size_t no_wait_bypass_bytes;
};

struct LzvmCudaMemoryInfo {
    std::size_t free_bytes;
    std::size_t total_bytes;
};

extern "C" int lzvm_cuda_alloc_bytes(void** out, std::size_t bytes);
extern "C" void lzvm_cuda_free_bytes(void* ptr);
extern "C" int lzvm_cuda_stream_create(void** out);
extern "C" int lzvm_cuda_stream_destroy(void* stream);
extern "C" int lzvm_cuda_stream_synchronize(void* stream);
extern "C" int lzvm_cuda_event_create(void** out);
extern "C" int lzvm_cuda_event_destroy(void* event);
extern "C" int lzvm_cuda_event_record(void* event, void* stream);
extern "C" int lzvm_cuda_event_synchronize(void* event);
extern "C" int lzvm_cuda_stream_wait_event(void* stream, void* event);
extern "C" int lzvm_cuda_allocator_clear_cache(void);
extern "C" int lzvm_cuda_allocator_stats(LzvmCudaAllocatorStats* out);
extern "C" int lzvm_cuda_memory_info(LzvmCudaMemoryInfo* out);
extern "C" int lzvm_cuda_copy_h2d_bytes(void* dst, const void* src, std::size_t bytes);
extern "C" int lzvm_cuda_copy_h2d_bytes_on_stream(
    void* dst,
    const void* src,
    std::size_t bytes,
    void* stream);
extern "C" int lzvm_cuda_copy_d2h_bytes(void* dst, const void* src, std::size_t bytes);
extern "C" int lzvm_cuda_copy_h2d_row_slice_words(
    void* dst,
    const void* src,
    std::size_t row_count,
    std::size_t source_width_words,
    std::size_t start_word,
    std::size_t slice_width_words);
extern "C" int lzvm_cuda_copy_d2d_row_slice_words(
    void* dst,
    const void* src,
    std::size_t row_count,
    std::size_t source_width_words,
    std::size_t start_word,
    std::size_t slice_width_words);
extern "C" int lzvm_cuda_copy_d2d_row_slice_words_on_stream(
    void* dst,
    const void* src,
    std::size_t row_count,
    std::size_t source_width_words,
    std::size_t start_word,
    std::size_t slice_width_words,
    void* stream);
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
extern "C" int lzvm_cuda_expand_state_prefix_words_device_to_device(
    void* dst,
    const void* src,
    std::size_t state_count,
    std::size_t state_width_words,
    std::size_t prefix_words);
extern "C" int lzvm_cuda_expand_state_prefix_words_device_to_device_on_stream(
    void* dst,
    const void* src,
    std::size_t state_count,
    std::size_t state_width_words,
    std::size_t prefix_words,
    void* stream);
extern "C" int lzvm_cuda_memset_zero_bytes(void* dst, std::size_t bytes);
extern "C" int lzvm_cuda_memset_zero_bytes_on_stream(
    void* dst,
    std::size_t bytes,
    void* stream);
extern "C" int lzvm_cuda_fill_row_major_column_u64(
    std::uint64_t* dst,
    std::size_t row_count,
    std::size_t row_width_words,
    std::size_t start_row,
    std::size_t column,
    std::uint64_t value);
extern "C" int lzvm_cuda_fill_row_major_suffix_from_row_u64(
    std::uint64_t* dst,
    const std::uint64_t* row_values,
    std::size_t row_count,
    std::size_t row_width_words,
    std::size_t start_row);
extern "C" int lzvm_cuda_expand_zisk_main_trace_descriptors(
    std::uint64_t* dst,
    const std::uint64_t* descriptors,
    std::size_t descriptor_words,
    std::size_t descriptor_count,
    std::size_t row_count,
    std::size_t row_width_words,
    std::uint64_t terminal_pc);
extern "C" int lzvm_cuda_check_launch(void);
extern "C" int lzvm_cuda_synchronize(void);
extern "C" int lzvm_cuda_goldilocks_validate_canonical_words_device(
    const std::uint64_t* values,
    std::size_t word_count,
    std::uint32_t* found);
extern "C" int lzvm_cuda_goldilocks_begin_validate_canonical_words_device(
    const std::uint64_t* values,
    std::size_t word_count,
    std::uint32_t* device_found);
extern "C" int lzvm_cuda_goldilocks_coset_extend_row_major_columns_device_unsynced(
    const std::uint64_t* values,
    std::uint64_t* out,
    std::uint64_t* workspace,
    std::size_t source_len,
    std::size_t source_bits,
    std::size_t target_len,
    std::size_t target_bits,
    std::size_t column_count,
    std::uint64_t source_root_inverse,
    std::uint64_t target_root,
    std::uint64_t shift);
extern "C" int lzvm_cuda_goldilocks_coset_extend_row_major_columns_strided_device_unsynced(
    const std::uint64_t* values,
    std::uint64_t* out,
    std::uint64_t* workspace,
    std::size_t source_len,
    std::size_t source_bits,
    std::size_t target_len,
    std::size_t target_bits,
    std::size_t source_row_stride,
    std::size_t column_offset,
    std::size_t column_count,
    std::uint64_t source_root_inverse,
    std::uint64_t target_root,
    std::uint64_t shift);
extern "C" int lzvm_cuda_goldilocks_coset_extend_row_major_columns_row_device(
    const std::uint64_t* values,
    const std::uint64_t* weights,
    std::uint64_t* out,
    std::size_t source_len,
    std::size_t column_count);
extern "C" int lzvm_cuda_goldilocks_coset_extend_row_major_columns_rows_device(
    const std::uint64_t* values,
    const std::uint64_t* weights,
    std::uint64_t* out,
    std::size_t source_len,
    std::size_t column_count,
    std::size_t target_row_count);
extern "C" int lzvm_cuda_goldilocks_coset_extend_row_major_columns_selected_rows_device(
    const std::uint64_t* values,
    const std::uint64_t* weights,
    std::uint64_t* out,
    std::size_t source_len,
    std::size_t column_count,
    std::size_t target_row_count);
extern "C" int lzvm_cuda_goldilocks_coset_extend_row_major_columns_strided_row_device(
    const std::uint64_t* values,
    const std::uint64_t* weights,
    std::uint64_t* out,
    std::size_t source_len,
    std::size_t source_row_stride,
    std::size_t column_offset,
    std::size_t column_count);
extern "C" int lzvm_cuda_goldilocks_coset_extend_row_major_columns_strided_rows_device(
    const std::uint64_t* values,
    const std::uint64_t* weights,
    std::uint64_t* out,
    std::size_t source_len,
    std::size_t source_row_stride,
    std::size_t column_offset,
    std::size_t column_count,
    std::size_t target_row_count);
extern "C" int lzvm_cuda_goldilocks_coset_extend_row_major_columns_strided_selected_rows_device(
    const std::uint64_t* values,
    const std::uint64_t* weights,
    std::uint64_t* out,
    std::size_t source_len,
    std::size_t source_row_stride,
    std::size_t column_offset,
    std::size_t column_count,
    std::size_t target_row_count);
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
