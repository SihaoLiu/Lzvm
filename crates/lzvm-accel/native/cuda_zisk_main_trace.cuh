#pragma once

constexpr size_t kZiskMainTraceColumns = 39;
constexpr size_t kZiskMainCompactDescriptorWords = 11;
constexpr size_t kZiskMainSparseDescriptorWords = 9;
constexpr size_t kZiskMainWideDescriptorWords = 14;
constexpr uint64_t kZiskMainSourceMemory = 1;
constexpr uint64_t kZiskMainSourceImmediate = 2;
constexpr uint64_t kZiskMainSourceRegister = 3;
constexpr uint64_t kZiskMainSourceIndirect = 4;
constexpr uint64_t kZiskMainStoreMemory = 1;
constexpr uint64_t kZiskMainStoreRegister = 2;
constexpr uint64_t kZiskMainStoreIndirect = 3;
constexpr uint64_t kZiskMainKindMask = 0x7ULL;
constexpr unsigned kZiskMainAKindShift = 32;
constexpr unsigned kZiskMainBKindShift = 35;
constexpr unsigned kZiskMainStoreKindShift = 38;

__device__ uint64_t zisk_main_low32(uint64_t value) {
    return value & 0xffffffffULL;
}

__device__ uint64_t zisk_main_high32(uint64_t value) {
    return value >> 32;
}

__device__ uint64_t zisk_main_signed_field(uint64_t value) {
    if ((value & (1ULL << 63)) == 0) {
        return value;
    }
    const uint64_t magnitude = (~value) + 1;
    return magnitude == 0 ? 0 : kModulus - magnitude;
}

__device__ uint64_t zisk_main_i32_bits_to_i64_bits(uint32_t value) {
    return static_cast<uint64_t>(static_cast<int64_t>(static_cast<int32_t>(value)));
}

__device__ int64_t zisk_main_signed_i64(uint64_t value) {
    if ((value & (1ULL << 63)) == 0) {
        return static_cast<int64_t>(value);
    }
    const uint64_t magnitude = (~value) + 1;
    return -static_cast<int64_t>(magnitude);
}

__device__ uint64_t zisk_main_signed_address_field(uint64_t offset, uint64_t base) {
    const int64_t address =
        zisk_main_signed_i64(offset) + static_cast<int64_t>(zisk_main_low32(base));
    return zisk_main_signed_field(static_cast<uint64_t>(address));
}

__device__ uint64_t zisk_main_source_offset_field(uint64_t kind, uint64_t payload) {
    if (kind == kZiskMainSourceImmediate || kind == kZiskMainSourceMemory) {
        return zisk_main_low32(payload);
    }
    if (kind == kZiskMainSourceRegister) {
        return payload;
    }
    if (kind == kZiskMainSourceIndirect) {
        return zisk_main_signed_field(payload);
    }
    return 0;
}

__device__ void zisk_main_write_source(
    uint64_t* row,
    uint64_t kind,
    uint64_t payload,
    size_t src_imm_column,
    size_t src_mem_column,
    size_t offset_column,
    size_t high_column,
    size_t src_reg_column) {
    if (kind == kZiskMainSourceImmediate) {
        row[src_imm_column] = 1;
        row[offset_column] = zisk_main_low32(payload);
        row[high_column] = zisk_main_high32(payload);
    } else if (kind == kZiskMainSourceMemory) {
        row[src_mem_column] = 1;
        row[offset_column] = zisk_main_low32(payload);
        row[high_column] = zisk_main_high32(payload);
    } else if (kind == kZiskMainSourceRegister) {
        row[offset_column] = payload;
        row[src_reg_column] = 1;
    } else if (kind == kZiskMainSourceIndirect) {
        row[offset_column] = zisk_main_signed_field(payload);
    }
}

__device__ void zisk_main_write_expanded_row(
    uint64_t* row,
    uint64_t a,
    uint64_t b,
    uint64_t c,
    uint64_t pc,
    uint64_t a_payload,
    uint64_t b_payload,
    uint64_t store_payload,
    uint64_t control,
    uint64_t jmp_offset1,
    uint64_t jmp_offset2,
    uint64_t a_prev_mem_step,
    uint64_t b_prev_mem_step,
    uint64_t store_prev_mem_step,
    uint64_t store_prev_value) {
    const uint64_t a_kind = (control >> kZiskMainAKindShift) & kZiskMainKindMask;
    const uint64_t b_kind = (control >> kZiskMainBKindShift) & kZiskMainKindMask;
    const uint64_t store_kind = (control >> kZiskMainStoreKindShift) & kZiskMainKindMask;

    row[0] = zisk_main_low32(a);
    row[1] = zisk_main_high32(a);
    row[2] = zisk_main_low32(b);
    row[3] = zisk_main_high32(b);
    row[4] = zisk_main_low32(c);
    row[5] = zisk_main_high32(c);
    row[6] = (control >> 8) & 1;
    row[7] = pc;

    row[8] = a_kind == kZiskMainSourceImmediate ? 1 : 0;
    row[9] = a_kind == kZiskMainSourceMemory ? 1 : 0;
    row[10] = zisk_main_source_offset_field(a_kind, a_payload);
    row[11] =
        (a_kind == kZiskMainSourceImmediate || a_kind == kZiskMainSourceMemory)
            ? zisk_main_high32(a_payload)
            : 0;
    row[12] = (control >> 13) & 1;
    row[13] = b_kind == kZiskMainSourceImmediate ? 1 : 0;
    row[14] = b_kind == kZiskMainSourceMemory ? 1 : 0;
    row[15] = zisk_main_source_offset_field(b_kind, b_payload);
    row[16] =
        (b_kind == kZiskMainSourceImmediate || b_kind == kZiskMainSourceMemory)
            ? zisk_main_high32(b_payload)
            : 0;
    row[17] = b_kind == kZiskMainSourceIndirect ? 1 : 0;
    row[18] = (control >> 16) & 0xffffULL;
    row[19] = (control >> 12) & 1;
    row[20] = control & 0xffULL;
    row[21] = (control >> 9) & 1;

    row[22] = store_kind == kZiskMainStoreMemory ? 1 : 0;
    row[23] = store_kind == kZiskMainStoreIndirect ? 1 : 0;
    row[24] = store_kind == kZiskMainStoreRegister ? store_payload
              : (store_kind == kZiskMainStoreMemory || store_kind == kZiskMainStoreIndirect)
                  ? zisk_main_signed_field(store_payload)
                  : 0;

    row[25] = (control >> 10) & 1;
    row[26] = zisk_main_signed_field(jmp_offset1);
    row[27] = zisk_main_signed_field(jmp_offset2);
    row[28] = (control >> 11) & 1;
    row[29] = b_kind == kZiskMainSourceIndirect ? zisk_main_signed_address_field(b_payload, a)
                                                 : zisk_main_source_offset_field(b_kind, b_payload);
    row[30] = a_prev_mem_step;
    row[31] = b_prev_mem_step;
    row[32] = store_prev_mem_step;
    row[33] = zisk_main_low32(store_prev_value);
    row[34] = zisk_main_high32(store_prev_value);
    row[35] = a_kind == kZiskMainSourceRegister ? 1 : 0;
    row[36] = b_kind == kZiskMainSourceRegister ? 1 : 0;
    row[37] = store_kind == kZiskMainStoreRegister ? 1 : 0;
    row[38] = 0;
}

__device__ uint64_t zisk_main_sparse_high32(
    uint64_t mask,
    const uint64_t* high_words,
    size_t high_word_count,
    size_t high_word_offset,
    unsigned field_index) {
    if (((mask >> field_index) & 1ULL) == 0) {
        return 0;
    }
    const uint64_t prior_mask = mask & ((1ULL << field_index) - 1ULL);
    const unsigned high_position = __popcll(prior_mask);
    const size_t word_index = high_word_offset + high_position / 2;
    if (word_index >= high_word_count) {
        return 0;
    }
    const uint64_t packed = high_words[word_index];
    return (high_position & 1U) == 0 ? zisk_main_low32(packed) : zisk_main_high32(packed);
}

__device__ uint64_t zisk_main_sparse_value(
    uint64_t low32,
    uint64_t mask,
    const uint64_t* high_words,
    size_t high_word_count,
    size_t high_word_offset,
    unsigned field_index) {
    return zisk_main_low32(low32) |
           (zisk_main_sparse_high32(
                mask, high_words, high_word_count, high_word_offset, field_index)
            << 32);
}

__global__ void expand_zisk_main_trace_descriptors_kernel(
    uint64_t* dst,
    const uint64_t* descriptors,
    size_t descriptor_words,
    size_t descriptor_count,
    size_t row_count,
    uint64_t terminal_pc) {
    const size_t row_index = blockIdx.x * blockDim.x + threadIdx.x;
    if (row_index >= row_count) {
        return;
    }

    uint64_t* row = dst + row_index * kZiskMainTraceColumns;

    if (row_index >= descriptor_count) {
        row[0] = 0;
        row[1] = 0;
        row[2] = 0;
        row[3] = 0;
        row[4] = 0;
        row[5] = 0;
        row[6] = 0;
        row[7] = terminal_pc;
        row[8] = 1;
        row[9] = 0;
        row[10] = 0;
        row[11] = 0;
        row[12] = 0;
        row[13] = 1;
        row[14] = 0;
        row[15] = 0;
        row[16] = 0;
        row[17] = 0;
        row[18] = 0;
        row[19] = 0;
        row[20] = 1;
        row[21] = 0;
        row[22] = 0;
        row[23] = 0;
        row[24] = 0;
        row[25] = 0;
        row[26] = 0;
        row[27] = 0;
        row[28] = 0;
        row[29] = 0;
        row[30] = 0;
        row[31] = 0;
        row[32] = 0;
        row[33] = 0;
        row[34] = 0;
        row[35] = 0;
        row[36] = 0;
        row[37] = 0;
        row[38] = 0;
        return;
    }

    const uint64_t* descriptor = descriptors + row_index * descriptor_words;
    const uint64_t a = descriptor[0];
    const uint64_t b = descriptor[1];
    const uint64_t c = descriptor[2];
    uint64_t pc;
    uint64_t a_payload;
    uint64_t b_payload;
    uint64_t store_payload;
    uint64_t control;
    if (descriptor_words == kZiskMainCompactDescriptorWords) {
        a_payload = descriptor[3];
        b_payload = descriptor[4];
        store_payload = descriptor[5];
        control = descriptor[6];
    } else {
        pc = descriptor[3];
        a_payload = descriptor[4];
        b_payload = descriptor[5];
        store_payload = descriptor[6];
        control = descriptor[7];
    }
    uint64_t jmp_offset1;
    uint64_t jmp_offset2;
    uint64_t a_prev_mem_step;
    uint64_t b_prev_mem_step;
    uint64_t store_prev_mem_step;
    uint64_t store_prev_value;
    if (descriptor_words == kZiskMainCompactDescriptorWords) {
        const uint64_t packed_pc_and_store_step = descriptor[7];
        const uint64_t packed_jumps = descriptor[8];
        const uint64_t packed_reg_steps = descriptor[9];
        pc = packed_pc_and_store_step & 0xffffffffULL;
        jmp_offset1 = zisk_main_i32_bits_to_i64_bits(static_cast<uint32_t>(packed_jumps));
        jmp_offset2 = zisk_main_i32_bits_to_i64_bits(static_cast<uint32_t>(packed_jumps >> 32));
        a_prev_mem_step = packed_reg_steps & 0xffffffffULL;
        b_prev_mem_step = packed_reg_steps >> 32;
        store_prev_mem_step = packed_pc_and_store_step >> 32;
        store_prev_value = descriptor[10];
    } else {
        jmp_offset1 = descriptor[8];
        jmp_offset2 = descriptor[9];
        a_prev_mem_step = descriptor[10];
        b_prev_mem_step = descriptor[11];
        store_prev_mem_step = descriptor[12];
        store_prev_value = descriptor[13];
    }

    zisk_main_write_expanded_row(
        row,
        a,
        b,
        c,
        pc,
        a_payload,
        b_payload,
        store_payload,
        control,
        jmp_offset1,
        jmp_offset2,
        a_prev_mem_step,
        b_prev_mem_step,
        store_prev_mem_step,
        store_prev_value);
}

__global__ void expand_sparse_zisk_main_trace_descriptors_kernel(
    uint64_t* dst,
    const uint64_t* descriptors,
    const uint64_t* high_words,
    size_t high_word_count,
    size_t descriptor_count,
    size_t row_count,
    uint64_t terminal_pc) {
    const size_t row_index = blockIdx.x * blockDim.x + threadIdx.x;
    if (row_index >= row_count) {
        return;
    }

    uint64_t* row = dst + row_index * kZiskMainTraceColumns;

    if (row_index >= descriptor_count) {
        row[0] = 0;
        row[1] = 0;
        row[2] = 0;
        row[3] = 0;
        row[4] = 0;
        row[5] = 0;
        row[6] = 0;
        row[7] = terminal_pc;
        row[8] = 1;
        row[9] = 0;
        row[10] = 0;
        row[11] = 0;
        row[12] = 0;
        row[13] = 1;
        row[14] = 0;
        row[15] = 0;
        row[16] = 0;
        row[17] = 0;
        row[18] = 0;
        row[19] = 0;
        row[20] = 1;
        row[21] = 0;
        row[22] = 0;
        row[23] = 0;
        row[24] = 0;
        row[25] = 0;
        row[26] = 0;
        row[27] = 0;
        row[28] = 0;
        row[29] = 0;
        row[30] = 0;
        row[31] = 0;
        row[32] = 0;
        row[33] = 0;
        row[34] = 0;
        row[35] = 0;
        row[36] = 0;
        row[37] = 0;
        row[38] = 0;
        return;
    }

    const uint64_t* descriptor = descriptors + row_index * kZiskMainSparseDescriptorWords;
    const uint64_t ab = descriptor[0];
    const uint64_t c_and_a_payload = descriptor[1];
    const uint64_t b_and_store_payload = descriptor[2];
    const uint64_t control = descriptor[3];
    const uint64_t packed_pc_and_store_step = descriptor[4];
    const uint64_t packed_jumps = descriptor[5];
    const uint64_t packed_reg_steps = descriptor[6];
    const uint64_t store_prev_and_mask = descriptor[7];
    const uint64_t mask = store_prev_and_mask >> 32;
    const size_t high_word_offset = static_cast<size_t>(descriptor[8]);

    const uint64_t a = zisk_main_sparse_value(
        ab, mask, high_words, high_word_count, high_word_offset, 0);
    const uint64_t b = zisk_main_sparse_value(
        ab >> 32, mask, high_words, high_word_count, high_word_offset, 1);
    const uint64_t c = zisk_main_sparse_value(
        c_and_a_payload, mask, high_words, high_word_count, high_word_offset, 2);
    const uint64_t a_payload = zisk_main_sparse_value(
        c_and_a_payload >> 32, mask, high_words, high_word_count, high_word_offset, 3);
    const uint64_t b_payload = zisk_main_sparse_value(
        b_and_store_payload, mask, high_words, high_word_count, high_word_offset, 4);
    const uint64_t store_payload = zisk_main_sparse_value(
        b_and_store_payload >> 32, mask, high_words, high_word_count, high_word_offset, 5);
    const uint64_t store_prev_value = zisk_main_sparse_value(
        store_prev_and_mask, mask, high_words, high_word_count, high_word_offset, 6);
    const uint64_t pc = packed_pc_and_store_step & 0xffffffffULL;
    const uint64_t jmp_offset1 =
        zisk_main_i32_bits_to_i64_bits(static_cast<uint32_t>(packed_jumps));
    const uint64_t jmp_offset2 =
        zisk_main_i32_bits_to_i64_bits(static_cast<uint32_t>(packed_jumps >> 32));
    const uint64_t a_prev_mem_step = packed_reg_steps & 0xffffffffULL;
    const uint64_t b_prev_mem_step = packed_reg_steps >> 32;
    const uint64_t store_prev_mem_step = packed_pc_and_store_step >> 32;

    zisk_main_write_expanded_row(
        row,
        a,
        b,
        c,
        pc,
        a_payload,
        b_payload,
        store_payload,
        control,
        jmp_offset1,
        jmp_offset2,
        a_prev_mem_step,
        b_prev_mem_step,
        store_prev_mem_step,
        store_prev_value);
}

int launch_expand_zisk_main_trace_descriptors(
    uint64_t* dst,
    const uint64_t* descriptors,
    size_t descriptor_words,
    size_t descriptor_count,
    size_t row_count,
    size_t row_width_words,
    uint64_t terminal_pc,
    cudaStream_t stream) {
    if (row_count == 0) {
        return 0;
    }
    if (dst == nullptr || (descriptor_count > 0 && descriptors == nullptr)) {
        return -1;
    }
    if (descriptor_words != kZiskMainCompactDescriptorWords &&
        descriptor_words != kZiskMainWideDescriptorWords) {
        return -2;
    }
    if (row_width_words != kZiskMainTraceColumns || descriptor_count > row_count) {
        return -2;
    }

    const size_t blocks = (row_count + kThreads - 1) / kThreads;
    if (blocks > static_cast<size_t>(std::numeric_limits<int>::max())) {
        return -2;
    }
    expand_zisk_main_trace_descriptors_kernel<<<static_cast<int>(blocks), kThreads, 0, stream>>>(
        dst, descriptors, descriptor_words, descriptor_count, row_count, terminal_pc);
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_check_launch());
    return 0;
}

int launch_expand_sparse_zisk_main_trace_descriptors(
    uint64_t* dst,
    const uint64_t* descriptors,
    const uint64_t* high_words,
    size_t high_word_count,
    size_t descriptor_count,
    size_t row_count,
    size_t row_width_words,
    uint64_t terminal_pc,
    cudaStream_t stream) {
    if (row_count == 0) {
        return 0;
    }
    if (dst == nullptr || (descriptor_count > 0 && descriptors == nullptr) ||
        (high_word_count > 0 && high_words == nullptr)) {
        return -1;
    }
    if (row_width_words != kZiskMainTraceColumns || descriptor_count > row_count) {
        return -2;
    }

    const size_t blocks = (row_count + kThreads - 1) / kThreads;
    if (blocks > static_cast<size_t>(std::numeric_limits<int>::max())) {
        return -2;
    }
    expand_sparse_zisk_main_trace_descriptors_kernel<<<
        static_cast<int>(blocks),
        kThreads,
        0,
        stream>>>(
        dst,
        descriptors,
        high_words,
        high_word_count,
        descriptor_count,
        row_count,
        terminal_pc);
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_check_launch());
    return 0;
}

extern "C" int lzvm_cuda_expand_zisk_main_trace_descriptors(
    uint64_t* dst,
    const uint64_t* descriptors,
    size_t descriptor_words,
    size_t descriptor_count,
    size_t row_count,
    size_t row_width_words,
    uint64_t terminal_pc) {
    return launch_expand_zisk_main_trace_descriptors(
        dst,
        descriptors,
        descriptor_words,
        descriptor_count,
        row_count,
        row_width_words,
        terminal_pc,
        nullptr);
}

extern "C" int lzvm_cuda_expand_zisk_main_trace_descriptors_on_stream(
    uint64_t* dst,
    const uint64_t* descriptors,
    size_t descriptor_words,
    size_t descriptor_count,
    size_t row_count,
    size_t row_width_words,
    uint64_t terminal_pc,
    void* stream_raw) {
    return launch_expand_zisk_main_trace_descriptors(
        dst,
        descriptors,
        descriptor_words,
        descriptor_count,
        row_count,
        row_width_words,
        terminal_pc,
        static_cast<cudaStream_t>(stream_raw));
}

extern "C" int lzvm_cuda_expand_sparse_zisk_main_trace_descriptors(
    uint64_t* dst,
    const uint64_t* descriptors,
    const uint64_t* high_words,
    size_t high_word_count,
    size_t descriptor_count,
    size_t row_count,
    size_t row_width_words,
    uint64_t terminal_pc) {
    return launch_expand_sparse_zisk_main_trace_descriptors(
        dst,
        descriptors,
        high_words,
        high_word_count,
        descriptor_count,
        row_count,
        row_width_words,
        terminal_pc,
        nullptr);
}
