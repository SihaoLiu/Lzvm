#pragma once

constexpr size_t kMainTraceSelectedRowsPerBlock = 176;
constexpr size_t kMainTraceSelectedTargetBatch = 4;
constexpr uint64_t kMainTraceSelectedRegisterRowKindMask = 0x187ULL;
constexpr uint64_t kMainTraceSelectedRegisterRowKindValue = 0x83ULL;

__device__ uint64_t main_trace_register_row_column(
    size_t column,
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
    uint64_t b_kind,
    uint64_t store_kind) {
    const bool b_is_immediate = b_kind == kMainTraceSourceImmediate;
    const bool b_is_register = b_kind == kMainTraceSourceRegister;
    const bool b_is_indirect = b_kind == kMainTraceSourceIndirect;
    const bool store_is_register = store_kind == kMainTraceStoreRegister;
    const bool store_is_indirect = store_kind == kMainTraceStoreIndirect;
    const uint64_t b_offset = b_is_register
                                  ? b_payload
                                  : (b_is_indirect
                                         ? main_trace_signed_field(b_payload)
                                         : main_trace_low32(b_payload));
    switch (column) {
        case 0:
            return main_trace_low32(a);
        case 1:
            return main_trace_high32(a);
        case 2:
            return main_trace_low32(b);
        case 3:
            return main_trace_high32(b);
        case 4:
            return main_trace_low32(c);
        case 5:
            return main_trace_high32(c);
        case 6:
            return (control >> 8) & 1;
        case 7:
            return pc;
        case 10:
            return a_payload;
        case 12:
            return (control >> 13) & 1;
        case 13:
            return b_is_immediate ? 1 : 0;
        case 15:
            return b_offset;
        case 16:
            return b_is_immediate ? main_trace_high32(b_payload) : 0;
        case 17:
            return b_is_indirect ? 1 : 0;
        case 18:
            return (control >> 16) & 0xffffULL;
        case 19:
            return (control >> 12) & 1;
        case 20:
            return control & 0xffULL;
        case 21:
            return (control >> 9) & 1;
        case 23:
            return store_is_indirect ? 1 : 0;
        case 24:
            return store_is_register ? store_payload
                                     : main_trace_signed_field(store_payload);
        case 25:
            return (control >> 10) & 1;
        case 26:
            return main_trace_signed_field(jmp_offset1);
        case 27:
            return main_trace_signed_field(jmp_offset2);
        case 28:
            return (control >> 11) & 1;
        case 29:
            return b_is_indirect
                       ? main_trace_signed_address_field(b_payload, a)
                       : b_offset;
        case 30:
            return store_is_indirect
                       ? main_trace_signed_address_field(store_payload, a)
                       : store_payload;
        case 31:
            return a_prev_mem_step;
        case 32:
            return b_prev_mem_step;
        case 33:
            return store_prev_mem_step;
        case 34:
            return main_trace_low32(store_prev_value);
        case 35:
            return main_trace_high32(store_prev_value);
        case 36:
            return 1;
        case 37:
            return b_is_register ? 1 : 0;
        case 38:
            return store_is_register ? 1 : 0;
        default:
            return 0;
    }
}

__global__ __launch_bounds__(kThreads, 4)
void extend_main_trace_compact_descriptors_shifted_rows_partial_kernel(
    const uint64_t* descriptors,
    const uint64_t* weights0,
    const uint64_t* weights1,
    const uint64_t* weights2,
    const uint64_t* weights3,
    const uint64_t* weight_shifts,
    uint64_t* partials,
    size_t descriptor_count,
    size_t source_len,
    uint64_t terminal_pc,
    size_t column_offset,
    size_t column_count,
    size_t chunk_count,
    size_t target_row_count,
    unsigned layout_kind) {
    const unsigned lane = threadIdx.x & (kMainTraceWarpLanes - 1);
    const unsigned warp = threadIdx.x / kMainTraceWarpLanes;
    const size_t chunk = blockIdx.x;
    const size_t row_start = chunk * kMainTraceSelectedRowsPerBlock;
    const size_t row_end = min(row_start + kMainTraceSelectedRowsPerBlock, source_len);
    __shared__ uint64_t warp_sums
        [kMainTraceSelectedTargetBatch][kMainTraceColumns][kMainTraceWarpRowsPerBlock];
    uint64_t sums[kMainTraceSelectedTargetBatch] = {0, 0, 0, 0};
    uint64_t tail_sums[kMainTraceSelectedTargetBatch] = {0, 0, 0, 0};

    for (size_t row_index = row_start + warp; row_index < row_end;
         row_index += kMainTraceWarpRowsPerBlock) {
        uint64_t value = 0;
        uint64_t tail_value = 0;
        if (row_index < descriptor_count) {
            const uint64_t* descriptor =
                descriptors + row_index * kMainTraceCompactWords;
            const uint64_t local = lane < kMainTraceCompactWords ? descriptor[lane] : 0;
            const uint64_t d0 = main_trace_shuffle_u64(local, 0);
            const uint64_t d1 = main_trace_shuffle_u64(local, 1);
            const uint64_t d2 = main_trace_shuffle_u64(local, 2);
            const uint64_t d5 = main_trace_shuffle_u64(local, 5);
            const uint64_t d6 = main_trace_shuffle_u64(local, 6);
            const uint64_t d7 = main_trace_shuffle_u64(local, 7);
            const uint64_t d8 = main_trace_shuffle_u64(local, 8);
            const uint64_t d9 = main_trace_shuffle_u64(local, 9);
            const uint64_t source_kinds = d5 >> kMainTraceAKindShift;
            const uint64_t a_kind = source_kinds & kMainTraceKindMask;
            const uint64_t b_kind = (source_kinds >> 3) & kMainTraceKindMask;
            const uint64_t store_kind = (source_kinds >> 6) & kMainTraceKindMask;
            const bool register_row =
                layout_kind == kMainTraceLayoutWithStoreAddress &&
                (source_kinds & kMainTraceSelectedRegisterRowKindMask) ==
                    kMainTraceSelectedRegisterRowKindValue &&
                b_kind - kMainTraceSourceImmediate <=
                    kMainTraceSourceIndirect - kMainTraceSourceImmediate;
            uint64_t a_payload;
            uint64_t b_payload;
            uint64_t store_payload;
            if (register_row) {
                a_payload =
                    (d5 >> kMainTraceARegisterShift) & kMainTraceRegisterMask;
                b_payload = b_kind == kMainTraceSourceRegister
                                ? (d5 >> kMainTraceBRegisterShift) &
                                      kMainTraceRegisterMask
                                : (b_kind == kMainTraceSourceImmediate
                                       ? d1
                                       : main_trace_shuffle_u64(local, 3));
                store_payload = store_kind == kMainTraceStoreRegister
                                    ? (d5 >> kMainTraceStoreRegisterShift) &
                                          kMainTraceRegisterMask
                                    : main_trace_shuffle_u64(
                                          local,
                                          b_kind == kMainTraceSourceIndirect ? 4 : 3);
            } else {
                const uint64_t d3 = main_trace_shuffle_u64(local, 3);
                const uint64_t d4 = main_trace_shuffle_u64(local, 4);
                main_trace_decode_compact_payloads(
                    d0, d1, d3, d4, d5, &a_payload, &b_payload, &store_payload);
            }
            const uint64_t pc = d6 & 0xffffffffULL;
            const uint64_t jmp_offset1 =
                main_trace_i32_bits_to_i64_bits(static_cast<uint32_t>(d7));
            const uint64_t jmp_offset2 =
                main_trace_i32_bits_to_i64_bits(static_cast<uint32_t>(d7 >> 32));
            const uint64_t a_prev_mem_step = d8 & 0xffffffffULL;
            const uint64_t b_prev_mem_step = d8 >> 32;
            const uint64_t store_prev_mem_step = d6 >> 32;
            if (lane < column_count) {
                value = register_row
                            ? main_trace_register_row_column(
                                  column_offset + lane,
                                  d0,
                                  d1,
                                  d2,
                                  pc,
                                  a_payload,
                                  b_payload,
                                  store_payload,
                                  d5,
                                  jmp_offset1,
                                  jmp_offset2,
                                  a_prev_mem_step,
                                  b_prev_mem_step,
                                  store_prev_mem_step,
                                  d9,
                                  b_kind,
                                  store_kind)
                            : main_trace_expanded_column_with_kinds(
                                  column_offset + lane,
                                  d0,
                                  d1,
                                  d2,
                                  pc,
                                  a_payload,
                                  b_payload,
                                  store_payload,
                                  d5,
                                  jmp_offset1,
                                  jmp_offset2,
                                  a_prev_mem_step,
                                  b_prev_mem_step,
                                  store_prev_mem_step,
                                  d9,
                                  a_kind,
                                  b_kind,
                                  store_kind,
                                  layout_kind);
            }
            if (lane + kMainTraceWarpLanes < column_count) {
                tail_value = register_row
                                 ? main_trace_register_row_column(
                                       column_offset + lane + kMainTraceWarpLanes,
                                       d0,
                                       d1,
                                       d2,
                                       pc,
                                       a_payload,
                                       b_payload,
                                       store_payload,
                                       d5,
                                       jmp_offset1,
                                       jmp_offset2,
                                       a_prev_mem_step,
                                       b_prev_mem_step,
                                       store_prev_mem_step,
                                       d9,
                                       b_kind,
                                       store_kind)
                                 : main_trace_expanded_column_with_kinds(
                                       column_offset + lane + kMainTraceWarpLanes,
                                       d0,
                                       d1,
                                       d2,
                                       pc,
                                       a_payload,
                                       b_payload,
                                       store_payload,
                                       d5,
                                       jmp_offset1,
                                       jmp_offset2,
                                       a_prev_mem_step,
                                       b_prev_mem_step,
                                       store_prev_mem_step,
                                       d9,
                                       a_kind,
                                       b_kind,
                                       store_kind,
                                       layout_kind);
            }
        } else {
            if (lane < column_count) {
                value = main_trace_terminal_column(column_offset + lane, terminal_pc);
            }
            if (lane + kMainTraceWarpLanes < column_count) {
                tail_value = main_trace_terminal_column(
                    column_offset + lane + kMainTraceWarpLanes, terminal_pc);
            }
        }

#pragma unroll
        for (unsigned target = 0; target < kMainTraceSelectedTargetBatch; ++target) {
            uint64_t weight = 0;
            if (lane == target && target < target_row_count) {
                const uint64_t* target_weights =
                    target == 0 ? weights0
                                : (target == 1 ? weights1
                                               : (target == 2 ? weights2 : weights3));
                const size_t weight_row =
                    row_index + static_cast<size_t>(weight_shifts[target]);
                const size_t weight_index =
                    weight_row >= source_len ? weight_row - source_len : weight_row;
                weight = target_weights[weight_index];
            }
            weight = main_trace_shuffle_u64(weight, target);
            if (target < target_row_count) {
                sums[target] = add_mod(sums[target], mul_mod(value, weight));
                tail_sums[target] =
                    add_mod(tail_sums[target], mul_mod(tail_value, weight));
            }
        }
    }

#pragma unroll
    for (unsigned target = 0; target < kMainTraceSelectedTargetBatch; ++target) {
        if (target >= target_row_count) {
            continue;
        }
        if (lane < column_count) {
            warp_sums[target][lane][warp] = sums[target];
        }
        if (lane + kMainTraceWarpLanes < column_count) {
            warp_sums[target][lane + kMainTraceWarpLanes][warp] = tail_sums[target];
        }
    }
    __syncthreads();

    if (warp != 0) {
        return;
    }
#pragma unroll
    for (unsigned target = 0; target < kMainTraceSelectedTargetBatch; ++target) {
        if (target >= target_row_count) {
            continue;
        }
        if (lane < column_count) {
            uint64_t sum = 0;
#pragma unroll
            for (unsigned source_warp = 0; source_warp < kMainTraceWarpRowsPerBlock;
                 ++source_warp) {
                sum = add_mod(sum, warp_sums[target][lane][source_warp]);
            }
            partials[(target * column_count + lane) * chunk_count + chunk] = sum;
        }
        if (lane + kMainTraceWarpLanes < column_count) {
            const size_t column = lane + kMainTraceWarpLanes;
            uint64_t sum = 0;
#pragma unroll
            for (unsigned source_warp = 0; source_warp < kMainTraceWarpRowsPerBlock;
                 ++source_warp) {
                sum = add_mod(sum, warp_sums[target][column][source_warp]);
            }
            partials[(target * column_count + column) * chunk_count + chunk] = sum;
        }
    }
}

int launch_extend_main_trace_compact_descriptors_shifted_rows(
    const uint64_t* descriptors,
    const uint64_t* weights0,
    const uint64_t* weights1,
    const uint64_t* weights2,
    const uint64_t* weights3,
    const uint64_t* weight_shifts,
    const uint64_t* output_rows,
    uint64_t* out,
    size_t descriptor_count,
    size_t source_len,
    uint64_t terminal_pc,
    size_t column_offset,
    size_t column_count,
    size_t target_row_count,
    unsigned layout_kind) {
    if (target_row_count == 0) {
        return 0;
    }
    if (weights0 == nullptr || (target_row_count > 1 && weights1 == nullptr) ||
        (target_row_count > 2 && weights2 == nullptr) ||
        (target_row_count > 3 && weights3 == nullptr) || weight_shifts == nullptr ||
        output_rows == nullptr || out == nullptr ||
        (descriptor_count > 0 && descriptors == nullptr)) {
        return -1;
    }
    if (source_len == 0 || descriptor_count > source_len || column_count == 0 ||
        target_row_count > kMainTraceSelectedTargetBatch ||
        column_offset > kMainTraceColumns ||
        column_count > kMainTraceColumns - column_offset ||
        layout_kind > kMainTraceLayoutWithStoreAddress) {
        return -2;
    }

    const size_t chunk_count =
        (source_len + kMainTraceSelectedRowsPerBlock - 1) / kMainTraceSelectedRowsPerBlock;
    const size_t row_column_count = target_row_count * column_count;
    if (chunk_count == 0 || row_column_count / column_count != target_row_count) {
        return -2;
    }
    const size_t partial_count = chunk_count * row_column_count;
    if (partial_count / row_column_count != chunk_count ||
        chunk_count > static_cast<size_t>(std::numeric_limits<unsigned>::max()) ||
        row_column_count > static_cast<size_t>(std::numeric_limits<unsigned>::max())) {
        return -2;
    }
    DeviceBuffer<uint64_t> partials;
    LZVM_CUDA_RETURN_ON_ERROR(partials.reset(partial_count));
    const dim3 blocks(static_cast<unsigned>(chunk_count));
    extend_main_trace_compact_descriptors_shifted_rows_partial_kernel<<<blocks, kThreads>>>(
        descriptors,
        weights0,
        weights1,
        weights2,
        weights3,
        weight_shifts,
        partials.data(),
        descriptor_count,
        source_len,
        terminal_pc,
        column_offset,
        column_count,
        chunk_count,
        target_row_count,
        layout_kind);
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_check_launch());
    extend_row_major_columns_rows_scatter_final_kernel<<<row_column_count, kThreads>>>(
        partials.data(), output_rows, out, chunk_count, column_count);
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_check_launch());
    return lzvm_cuda_synchronize();
}

extern "C" int
lzvm_cuda_goldilocks_coset_extend_main_trace_compact_descriptors_shifted_rows_device(
    const uint64_t* descriptors,
    const uint64_t* weights0,
    const uint64_t* weights1,
    const uint64_t* weights2,
    const uint64_t* weights3,
    const uint64_t* weight_shifts,
    const uint64_t* output_rows,
    uint64_t* out,
    size_t descriptor_count,
    size_t source_len,
    uint64_t terminal_pc,
    size_t column_offset,
    size_t column_count,
    size_t target_row_count,
    unsigned layout_kind) {
    return launch_extend_main_trace_compact_descriptors_shifted_rows(
        descriptors,
        weights0,
        weights1,
        weights2,
        weights3,
        weight_shifts,
        output_rows,
        out,
        descriptor_count,
        source_len,
        terminal_pc,
        column_offset,
        column_count,
        target_row_count,
        layout_kind);
}
