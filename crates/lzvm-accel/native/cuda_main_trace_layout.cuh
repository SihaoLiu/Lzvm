#pragma once

constexpr size_t kMainTraceColumns = 39;
constexpr size_t kMainTraceCompactWords = 11;
constexpr size_t kMainTraceSparseWords = 9;
constexpr size_t kMainTraceWideWords = 14;
constexpr uint64_t kMainTraceSourceMemory = 1;
constexpr uint64_t kMainTraceSourceImmediate = 2;
constexpr uint64_t kMainTraceSourceRegister = 3;
constexpr uint64_t kMainTraceSourceIndirect = 4;
constexpr uint64_t kMainTraceStoreMemory = 1;
constexpr uint64_t kMainTraceStoreRegister = 2;
constexpr uint64_t kMainTraceStoreIndirect = 3;
constexpr uint64_t kMainTraceKindMask = 0x7ULL;
constexpr unsigned kMainTraceAKindShift = 32;
constexpr unsigned kMainTraceBKindShift = 35;
constexpr unsigned kMainTraceStoreKindShift = 38;
constexpr unsigned kMainTraceLayoutLegacy = 0;
constexpr unsigned kMainTraceLayoutWithStoreAddress = 1;

__device__ uint64_t main_trace_low32(uint64_t value) {
    return value & 0xffffffffULL;
}

__device__ uint64_t main_trace_high32(uint64_t value) {
    return value >> 32;
}

__device__ uint64_t main_trace_signed_field(uint64_t value) {
    if ((value & (1ULL << 63)) == 0) {
        return value;
    }
    const uint64_t magnitude = (~value) + 1;
    return magnitude == 0 ? 0 : kModulus - magnitude;
}

__device__ uint64_t main_trace_i32_bits_to_i64_bits(uint32_t value) {
    return static_cast<uint64_t>(static_cast<int64_t>(static_cast<int32_t>(value)));
}

__device__ int64_t main_trace_signed_i64(uint64_t value) {
    if ((value & (1ULL << 63)) == 0) {
        return static_cast<int64_t>(value);
    }
    const uint64_t magnitude = (~value) + 1;
    return -static_cast<int64_t>(magnitude);
}

__device__ uint64_t main_trace_signed_address_field(uint64_t offset, uint64_t base) {
    const int64_t address =
        main_trace_signed_i64(offset) + static_cast<int64_t>(main_trace_low32(base));
    return main_trace_signed_field(static_cast<uint64_t>(address));
}

__device__ uint64_t main_trace_source_offset_field(uint64_t kind, uint64_t payload) {
    if (kind == kMainTraceSourceImmediate || kind == kMainTraceSourceMemory) {
        return main_trace_low32(payload);
    }
    if (kind == kMainTraceSourceRegister) {
        return payload;
    }
    if (kind == kMainTraceSourceIndirect) {
        return main_trace_signed_field(payload);
    }
    return 0;
}

__device__ uint64_t main_trace_store_offset_field(uint64_t kind, uint64_t payload) {
    if (kind == kMainTraceStoreRegister) {
        return payload;
    }
    if (kind == kMainTraceStoreMemory || kind == kMainTraceStoreIndirect) {
        return main_trace_signed_field(payload);
    }
    return 0;
}

__device__ uint64_t main_trace_store_address_field(
    uint64_t kind,
    uint64_t payload,
    uint64_t base) {
    if (kind == kMainTraceStoreIndirect) {
        return main_trace_signed_address_field(payload, base);
    }
    return main_trace_store_offset_field(kind, payload);
}

__device__ void main_trace_write_expanded_row(
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
    uint64_t store_prev_value,
    unsigned layout_kind) {
    const uint64_t a_kind = (control >> kMainTraceAKindShift) & kMainTraceKindMask;
    const uint64_t b_kind = (control >> kMainTraceBKindShift) & kMainTraceKindMask;
    const uint64_t store_kind = (control >> kMainTraceStoreKindShift) & kMainTraceKindMask;

    row[0] = main_trace_low32(a);
    row[1] = main_trace_high32(a);
    row[2] = main_trace_low32(b);
    row[3] = main_trace_high32(b);
    row[4] = main_trace_low32(c);
    row[5] = main_trace_high32(c);
    row[6] = (control >> 8) & 1;
    row[7] = pc;
    row[8] = a_kind == kMainTraceSourceImmediate ? 1 : 0;
    row[9] = a_kind == kMainTraceSourceMemory ? 1 : 0;
    row[10] = main_trace_source_offset_field(a_kind, a_payload);
    row[11] =
        (a_kind == kMainTraceSourceImmediate || a_kind == kMainTraceSourceMemory)
            ? main_trace_high32(a_payload)
            : 0;
    row[12] = (control >> 13) & 1;
    row[13] = b_kind == kMainTraceSourceImmediate ? 1 : 0;
    row[14] = b_kind == kMainTraceSourceMemory ? 1 : 0;
    row[15] = main_trace_source_offset_field(b_kind, b_payload);
    row[16] =
        (b_kind == kMainTraceSourceImmediate || b_kind == kMainTraceSourceMemory)
            ? main_trace_high32(b_payload)
            : 0;
    row[17] = b_kind == kMainTraceSourceIndirect ? 1 : 0;
    row[18] = (control >> 16) & 0xffffULL;
    row[19] = (control >> 12) & 1;
    row[20] = control & 0xffULL;
    row[21] = (control >> 9) & 1;
    row[22] = store_kind == kMainTraceStoreMemory ? 1 : 0;
    row[23] = store_kind == kMainTraceStoreIndirect ? 1 : 0;
    row[24] = main_trace_store_offset_field(store_kind, store_payload);
    row[25] = (control >> 10) & 1;
    row[26] = main_trace_signed_field(jmp_offset1);
    row[27] = main_trace_signed_field(jmp_offset2);
    row[28] = (control >> 11) & 1;
    row[29] =
        b_kind == kMainTraceSourceIndirect
            ? main_trace_signed_address_field(b_payload, a)
            : main_trace_source_offset_field(b_kind, b_payload);

    if (layout_kind == kMainTraceLayoutWithStoreAddress) {
        row[30] = main_trace_store_address_field(store_kind, store_payload, a);
        row[31] = a_prev_mem_step;
        row[32] = b_prev_mem_step;
        row[33] = store_prev_mem_step;
        row[34] = main_trace_low32(store_prev_value);
        row[35] = main_trace_high32(store_prev_value);
        row[36] = a_kind == kMainTraceSourceRegister ? 1 : 0;
        row[37] = b_kind == kMainTraceSourceRegister ? 1 : 0;
        row[38] = store_kind == kMainTraceStoreRegister ? 1 : 0;
    } else {
        row[30] = a_prev_mem_step;
        row[31] = b_prev_mem_step;
        row[32] = store_prev_mem_step;
        row[33] = main_trace_low32(store_prev_value);
        row[34] = main_trace_high32(store_prev_value);
        row[35] = a_kind == kMainTraceSourceRegister ? 1 : 0;
        row[36] = b_kind == kMainTraceSourceRegister ? 1 : 0;
        row[37] = store_kind == kMainTraceStoreRegister ? 1 : 0;
        row[38] = 0;
    }
}

__device__ uint64_t main_trace_sparse_high32(
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
    return (high_position & 1U) == 0 ? main_trace_low32(packed) : main_trace_high32(packed);
}

__device__ uint64_t main_trace_sparse_value(
    uint64_t low32,
    uint64_t mask,
    const uint64_t* high_words,
    size_t high_word_count,
    size_t high_word_offset,
    unsigned field_index) {
    return main_trace_low32(low32) |
           (main_trace_sparse_high32(
                mask, high_words, high_word_count, high_word_offset, field_index)
            << 32);
}

__device__ void main_trace_write_terminal_row(uint64_t* row, uint64_t terminal_pc) {
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
}

__device__ void main_trace_write_descriptor_row(
    uint64_t* row,
    const uint64_t* descriptor,
    size_t descriptor_words,
    unsigned layout_kind) {
    const uint64_t a = descriptor[0];
    const uint64_t b = descriptor[1];
    const uint64_t c = descriptor[2];
    uint64_t pc;
    uint64_t a_payload;
    uint64_t b_payload;
    uint64_t store_payload;
    uint64_t control;
    if (descriptor_words == kMainTraceCompactWords) {
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
    if (descriptor_words == kMainTraceCompactWords) {
        const uint64_t packed_pc_and_store_step = descriptor[7];
        const uint64_t packed_jumps = descriptor[8];
        const uint64_t packed_reg_steps = descriptor[9];
        pc = packed_pc_and_store_step & 0xffffffffULL;
        jmp_offset1 = main_trace_i32_bits_to_i64_bits(static_cast<uint32_t>(packed_jumps));
        jmp_offset2 = main_trace_i32_bits_to_i64_bits(static_cast<uint32_t>(packed_jumps >> 32));
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

    main_trace_write_expanded_row(
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
        store_prev_value,
        layout_kind);
}

__global__ void expand_main_trace_descriptors_layout_kernel(
    uint64_t* dst,
    const uint64_t* descriptors,
    size_t descriptor_words,
    size_t descriptor_count,
    size_t row_count,
    uint64_t terminal_pc,
    unsigned layout_kind) {
    const size_t row_index = blockIdx.x * blockDim.x + threadIdx.x;
    if (row_index >= row_count) {
        return;
    }
    uint64_t* row = dst + row_index * kMainTraceColumns;
    if (row_index >= descriptor_count) {
        main_trace_write_terminal_row(row, terminal_pc);
        return;
    }
    main_trace_write_descriptor_row(
        row, descriptors + row_index * descriptor_words, descriptor_words, layout_kind);
}

__global__ void expand_selected_main_trace_descriptor_rows_layout_kernel(
    uint64_t* dst,
    const uint64_t* descriptors,
    size_t descriptor_words,
    size_t descriptor_count,
    uint64_t terminal_pc,
    const uint64_t* rows,
    size_t selected_row_count,
    size_t start_word,
    size_t slice_width_words,
    unsigned layout_kind) {
    const size_t selected_row_index = blockIdx.x * blockDim.x + threadIdx.x;
    if (selected_row_index >= selected_row_count) {
        return;
    }
    uint64_t expanded[kMainTraceColumns];
    const uint64_t source_row = rows[selected_row_index];
    if (source_row >= descriptor_count) {
        main_trace_write_terminal_row(expanded, terminal_pc);
    } else {
        main_trace_write_descriptor_row(
            expanded,
            descriptors + static_cast<size_t>(source_row) * descriptor_words,
            descriptor_words,
            layout_kind);
    }
    uint64_t* output_row = dst + selected_row_index * slice_width_words;
    for (size_t column = 0; column < slice_width_words; ++column) {
        output_row[column] = expanded[start_word + column];
    }
}

__global__ void expand_sparse_main_trace_descriptors_layout_kernel(
    uint64_t* dst,
    const uint64_t* descriptors,
    const uint64_t* high_words,
    size_t high_word_count,
    size_t descriptor_count,
    size_t row_count,
    uint64_t terminal_pc,
    unsigned layout_kind) {
    const size_t row_index = blockIdx.x * blockDim.x + threadIdx.x;
    if (row_index >= row_count) {
        return;
    }
    uint64_t* row = dst + row_index * kMainTraceColumns;
    if (row_index >= descriptor_count) {
        main_trace_write_terminal_row(row, terminal_pc);
        return;
    }

    const uint64_t* descriptor = descriptors + row_index * kMainTraceSparseWords;
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

    const uint64_t a =
        main_trace_sparse_value(ab, mask, high_words, high_word_count, high_word_offset, 0);
    const uint64_t b =
        main_trace_sparse_value(ab >> 32, mask, high_words, high_word_count, high_word_offset, 1);
    const uint64_t c = main_trace_sparse_value(
        c_and_a_payload, mask, high_words, high_word_count, high_word_offset, 2);
    const uint64_t a_payload = main_trace_sparse_value(
        c_and_a_payload >> 32, mask, high_words, high_word_count, high_word_offset, 3);
    const uint64_t b_payload = main_trace_sparse_value(
        b_and_store_payload, mask, high_words, high_word_count, high_word_offset, 4);
    const uint64_t store_payload = main_trace_sparse_value(
        b_and_store_payload >> 32, mask, high_words, high_word_count, high_word_offset, 5);
    const uint64_t store_prev_value = main_trace_sparse_value(
        store_prev_and_mask, mask, high_words, high_word_count, high_word_offset, 6);
    const uint64_t pc = packed_pc_and_store_step & 0xffffffffULL;
    const uint64_t jmp_offset1 =
        main_trace_i32_bits_to_i64_bits(static_cast<uint32_t>(packed_jumps));
    const uint64_t jmp_offset2 =
        main_trace_i32_bits_to_i64_bits(static_cast<uint32_t>(packed_jumps >> 32));
    const uint64_t a_prev_mem_step = packed_reg_steps & 0xffffffffULL;
    const uint64_t b_prev_mem_step = packed_reg_steps >> 32;
    const uint64_t store_prev_mem_step = packed_pc_and_store_step >> 32;

    main_trace_write_expanded_row(
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
        store_prev_value,
        layout_kind);
}

int launch_expand_main_trace_descriptors_layout(
    uint64_t* dst,
    const uint64_t* descriptors,
    size_t descriptor_words,
    size_t descriptor_count,
    size_t row_count,
    size_t row_width_words,
    uint64_t terminal_pc,
    unsigned layout_kind,
    cudaStream_t stream) {
    if (row_count == 0) {
        return 0;
    }
    if (dst == nullptr || (descriptor_count > 0 && descriptors == nullptr)) {
        return -1;
    }
    if (descriptor_words != kMainTraceCompactWords && descriptor_words != kMainTraceWideWords) {
        return -2;
    }
    if (row_width_words != kMainTraceColumns || descriptor_count > row_count ||
        layout_kind > kMainTraceLayoutWithStoreAddress) {
        return -2;
    }

    const size_t blocks = (row_count + kThreads - 1) / kThreads;
    if (blocks > static_cast<size_t>(std::numeric_limits<int>::max())) {
        return -2;
    }
    expand_main_trace_descriptors_layout_kernel<<<
        static_cast<int>(blocks),
        kThreads,
        0,
        stream>>>(dst, descriptors, descriptor_words, descriptor_count, row_count, terminal_pc, layout_kind);
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_check_launch());
    return 0;
}

int launch_expand_sparse_main_trace_descriptors_layout(
    uint64_t* dst,
    const uint64_t* descriptors,
    const uint64_t* high_words,
    size_t high_word_count,
    size_t descriptor_count,
    size_t row_count,
    size_t row_width_words,
    uint64_t terminal_pc,
    unsigned layout_kind,
    cudaStream_t stream) {
    if (row_count == 0) {
        return 0;
    }
    if (dst == nullptr || (descriptor_count > 0 && descriptors == nullptr) ||
        (high_word_count > 0 && high_words == nullptr)) {
        return -1;
    }
    if (row_width_words != kMainTraceColumns || descriptor_count > row_count ||
        layout_kind > kMainTraceLayoutWithStoreAddress) {
        return -2;
    }

    const size_t blocks = (row_count + kThreads - 1) / kThreads;
    if (blocks > static_cast<size_t>(std::numeric_limits<int>::max())) {
        return -2;
    }
    expand_sparse_main_trace_descriptors_layout_kernel<<<
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
        terminal_pc,
        layout_kind);
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_check_launch());
    return 0;
}

int launch_expand_selected_main_trace_descriptor_rows_layout(
    uint64_t* dst,
    const uint64_t* descriptors,
    size_t descriptor_words,
    size_t descriptor_count,
    size_t row_count,
    size_t row_width_words,
    uint64_t terminal_pc,
    const uint64_t* rows,
    size_t selected_row_count,
    size_t start_word,
    size_t slice_width_words,
    unsigned layout_kind,
    cudaStream_t stream) {
    if (selected_row_count == 0 || slice_width_words == 0) {
        return 0;
    }
    if (dst == nullptr || rows == nullptr || (descriptor_count > 0 && descriptors == nullptr)) {
        return -1;
    }
    if (descriptor_words != kMainTraceCompactWords && descriptor_words != kMainTraceWideWords) {
        return -2;
    }
    if (row_width_words != kMainTraceColumns || descriptor_count > row_count ||
        start_word > row_width_words || slice_width_words > row_width_words - start_word ||
        layout_kind > kMainTraceLayoutWithStoreAddress) {
        return -2;
    }

    const size_t blocks = (selected_row_count + kThreads - 1) / kThreads;
    if (blocks > static_cast<size_t>(std::numeric_limits<int>::max())) {
        return -2;
    }
    expand_selected_main_trace_descriptor_rows_layout_kernel<<<
        static_cast<int>(blocks),
        kThreads,
        0,
        stream>>>(
        dst,
        descriptors,
        descriptor_words,
        descriptor_count,
        terminal_pc,
        rows,
        selected_row_count,
        start_word,
        slice_width_words,
        layout_kind);
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_check_launch());
    return 0;
}

extern "C" int lzvm_cuda_expand_main_trace_descriptors_layout(
    uint64_t* dst,
    const uint64_t* descriptors,
    size_t descriptor_words,
    size_t descriptor_count,
    size_t row_count,
    size_t row_width_words,
    uint64_t terminal_pc,
    unsigned layout_kind) {
    return launch_expand_main_trace_descriptors_layout(
        dst,
        descriptors,
        descriptor_words,
        descriptor_count,
        row_count,
        row_width_words,
        terminal_pc,
        layout_kind,
        nullptr);
}

extern "C" int lzvm_cuda_expand_main_trace_descriptors_layout_on_stream(
    uint64_t* dst,
    const uint64_t* descriptors,
    size_t descriptor_words,
    size_t descriptor_count,
    size_t row_count,
    size_t row_width_words,
    uint64_t terminal_pc,
    unsigned layout_kind,
    void* stream_raw) {
    return launch_expand_main_trace_descriptors_layout(
        dst,
        descriptors,
        descriptor_words,
        descriptor_count,
        row_count,
        row_width_words,
        terminal_pc,
        layout_kind,
        static_cast<cudaStream_t>(stream_raw));
}

extern "C" int lzvm_cuda_expand_main_trace_descriptor_selected_row_major_u64_slice_layout(
    uint64_t* dst,
    const uint64_t* descriptors,
    size_t descriptor_words,
    size_t descriptor_count,
    size_t row_count,
    size_t row_width_words,
    uint64_t terminal_pc,
    const uint64_t* rows,
    size_t selected_row_count,
    size_t start_word,
    size_t slice_width_words,
    unsigned layout_kind) {
    return launch_expand_selected_main_trace_descriptor_rows_layout(
        dst,
        descriptors,
        descriptor_words,
        descriptor_count,
        row_count,
        row_width_words,
        terminal_pc,
        rows,
        selected_row_count,
        start_word,
        slice_width_words,
        layout_kind,
        nullptr);
}

extern "C" int lzvm_cuda_expand_sparse_main_trace_descriptors_layout(
    uint64_t* dst,
    const uint64_t* descriptors,
    const uint64_t* high_words,
    size_t high_word_count,
    size_t descriptor_count,
    size_t row_count,
    size_t row_width_words,
    uint64_t terminal_pc,
    unsigned layout_kind) {
    return launch_expand_sparse_main_trace_descriptors_layout(
        dst,
        descriptors,
        high_words,
        high_word_count,
        descriptor_count,
        row_count,
        row_width_words,
        terminal_pc,
        layout_kind,
        nullptr);
}
