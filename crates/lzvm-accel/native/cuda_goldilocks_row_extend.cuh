#pragma once

__global__ void extend_row_major_columns_row_partial_kernel(
    const uint64_t* values,
    const uint64_t* weights,
    uint64_t* partials,
    size_t source_len,
    size_t column_count,
    size_t chunk_count) {
    const size_t block_index = blockIdx.x;
    const size_t column = block_index / chunk_count;
    const size_t chunk = block_index - column * chunk_count;
    const size_t row = chunk * blockDim.x + threadIdx.x;
    __shared__ uint64_t sums[kThreads];
    uint64_t value = 0;
    if (column < column_count && row < source_len) {
        value = mul_mod(values[row * column_count + column], weights[row]);
    }
    sums[threadIdx.x] = value;
    __syncthreads();

    for (size_t stride = blockDim.x / 2; stride > 0; stride >>= 1) {
        if (threadIdx.x < stride) {
            sums[threadIdx.x] = add_mod(sums[threadIdx.x], sums[threadIdx.x + stride]);
        }
        __syncthreads();
    }
    if (threadIdx.x == 0 && column < column_count) {
        partials[column * chunk_count + chunk] = sums[0];
    }
}

__global__ void extend_row_major_columns_shifted_row_partial_kernel(
    const uint64_t* values,
    const uint64_t* weights,
    uint64_t* partials,
    size_t source_len,
    size_t column_count,
    size_t chunk_count,
    size_t weight_shift) {
    const size_t block_index = blockIdx.x;
    const size_t column = block_index / chunk_count;
    const size_t chunk = block_index - column * chunk_count;
    const size_t row = chunk * blockDim.x + threadIdx.x;
    __shared__ uint64_t sums[kThreads];
    uint64_t value = 0;
    if (column < column_count && row < source_len) {
        const size_t weight_row = row + weight_shift;
        const size_t weight_index = weight_row >= source_len ? weight_row - source_len : weight_row;
        value = mul_mod(values[row * column_count + column], weights[weight_index]);
    }
    sums[threadIdx.x] = value;
    __syncthreads();

    for (size_t stride = blockDim.x / 2; stride > 0; stride >>= 1) {
        if (threadIdx.x < stride) {
            sums[threadIdx.x] = add_mod(sums[threadIdx.x], sums[threadIdx.x + stride]);
        }
        __syncthreads();
    }
    if (threadIdx.x == 0 && column < column_count) {
        partials[column * chunk_count + chunk] = sums[0];
    }
}

__global__ void extend_row_major_columns_strided_row_partial_kernel(
    const uint64_t* values,
    const uint64_t* weights,
    uint64_t* partials,
    size_t source_len,
    size_t source_row_stride,
    size_t column_offset,
    size_t column_count,
    size_t chunk_count) {
    const size_t block_index = blockIdx.x;
    const size_t column = block_index / chunk_count;
    const size_t chunk = block_index - column * chunk_count;
    const size_t row = chunk * blockDim.x + threadIdx.x;
    __shared__ uint64_t sums[kThreads];
    uint64_t value = 0;
    if (column < column_count && row < source_len) {
        value =
            mul_mod(values[row * source_row_stride + column_offset + column], weights[row]);
    }
    sums[threadIdx.x] = value;
    __syncthreads();

    for (size_t stride = blockDim.x / 2; stride > 0; stride >>= 1) {
        if (threadIdx.x < stride) {
            sums[threadIdx.x] = add_mod(sums[threadIdx.x], sums[threadIdx.x + stride]);
        }
        __syncthreads();
    }
    if (threadIdx.x == 0 && column < column_count) {
        partials[column * chunk_count + chunk] = sums[0];
    }
}

__global__ void extend_row_major_columns_strided_shifted_row_partial_kernel(
    const uint64_t* values,
    const uint64_t* weights,
    uint64_t* partials,
    size_t source_len,
    size_t source_row_stride,
    size_t column_offset,
    size_t column_count,
    size_t chunk_count,
    size_t weight_shift) {
    const size_t block_index = blockIdx.x;
    const size_t column = block_index / chunk_count;
    const size_t chunk = block_index - column * chunk_count;
    const size_t row = chunk * blockDim.x + threadIdx.x;
    __shared__ uint64_t sums[kThreads];
    uint64_t value = 0;
    if (column < column_count && row < source_len) {
        const size_t weight_row = row + weight_shift;
        const size_t weight_index = weight_row >= source_len ? weight_row - source_len : weight_row;
        value = mul_mod(
            values[row * source_row_stride + column_offset + column], weights[weight_index]);
    }
    sums[threadIdx.x] = value;
    __syncthreads();

    for (size_t stride = blockDim.x / 2; stride > 0; stride >>= 1) {
        if (threadIdx.x < stride) {
            sums[threadIdx.x] = add_mod(sums[threadIdx.x], sums[threadIdx.x + stride]);
        }
        __syncthreads();
    }
    if (threadIdx.x == 0 && column < column_count) {
        partials[column * chunk_count + chunk] = sums[0];
    }
}

__global__ void extend_row_major_columns_rows_partial_kernel(
    const uint64_t* values,
    const uint64_t* weights,
    uint64_t* partials,
    size_t source_len,
    size_t column_count,
    size_t chunk_count) {
    const size_t block_index = blockIdx.x;
    const size_t column_chunk_count = column_count * chunk_count;
    const size_t target_row = block_index / column_chunk_count;
    const size_t offset = block_index - target_row * column_chunk_count;
    const size_t column = offset / chunk_count;
    const size_t chunk = offset - column * chunk_count;
    const size_t row = chunk * blockDim.x + threadIdx.x;
    __shared__ uint64_t sums[kThreads];
    uint64_t value = 0;
    if (column < column_count && row < source_len) {
        value =
            mul_mod(values[row * column_count + column], weights[target_row * source_len + row]);
    }
    sums[threadIdx.x] = value;
    __syncthreads();

    for (size_t stride = blockDim.x / 2; stride > 0; stride >>= 1) {
        if (threadIdx.x < stride) {
            sums[threadIdx.x] = add_mod(sums[threadIdx.x], sums[threadIdx.x + stride]);
        }
        __syncthreads();
    }
    if (threadIdx.x == 0 && column < column_count) {
        partials[(target_row * column_count + column) * chunk_count + chunk] = sums[0];
    }
}

__global__ void extend_row_major_columns_strided_rows_partial_kernel(
    const uint64_t* values,
    const uint64_t* weights,
    uint64_t* partials,
    size_t source_len,
    size_t source_row_stride,
    size_t column_offset,
    size_t column_count,
    size_t chunk_count) {
    const size_t block_index = blockIdx.x;
    const size_t column_chunk_count = column_count * chunk_count;
    const size_t target_row = block_index / column_chunk_count;
    const size_t offset = block_index - target_row * column_chunk_count;
    const size_t column = offset / chunk_count;
    const size_t chunk = offset - column * chunk_count;
    const size_t row = chunk * blockDim.x + threadIdx.x;
    __shared__ uint64_t sums[kThreads];
    uint64_t value = 0;
    if (column < column_count && row < source_len) {
        value = mul_mod(
            values[row * source_row_stride + column_offset + column],
            weights[target_row * source_len + row]);
    }
    sums[threadIdx.x] = value;
    __syncthreads();

    for (size_t stride = blockDim.x / 2; stride > 0; stride >>= 1) {
        if (threadIdx.x < stride) {
            sums[threadIdx.x] = add_mod(sums[threadIdx.x], sums[threadIdx.x + stride]);
        }
        __syncthreads();
    }
    if (threadIdx.x == 0 && column < column_count) {
        partials[(target_row * column_count + column) * chunk_count + chunk] = sums[0];
    }
}

__global__ void extend_row_major_columns_shifted_rows_partial_kernel(
    const uint64_t* values,
    const uint64_t* weights,
    const uint64_t* weight_shifts,
    uint64_t* partials,
    size_t source_len,
    size_t column_count,
    size_t chunk_count) {
    const size_t block_index = blockIdx.x;
    const size_t column_chunk_count = column_count * chunk_count;
    const size_t target_row = block_index / column_chunk_count;
    const size_t offset = block_index - target_row * column_chunk_count;
    const size_t column = offset / chunk_count;
    const size_t chunk = offset - column * chunk_count;
    const size_t row = chunk * blockDim.x + threadIdx.x;
    __shared__ uint64_t sums[kThreads];
    uint64_t value = 0;
    if (column < column_count && row < source_len) {
        const size_t weight_row = row + static_cast<size_t>(weight_shifts[target_row]);
        const size_t weight_index = weight_row >= source_len ? weight_row - source_len : weight_row;
        value = mul_mod(values[row * column_count + column], weights[weight_index]);
    }
    sums[threadIdx.x] = value;
    __syncthreads();

    for (size_t stride = blockDim.x / 2; stride > 0; stride >>= 1) {
        if (threadIdx.x < stride) {
            sums[threadIdx.x] = add_mod(sums[threadIdx.x], sums[threadIdx.x + stride]);
        }
        __syncthreads();
    }
    if (threadIdx.x == 0 && column < column_count) {
        partials[(target_row * column_count + column) * chunk_count + chunk] = sums[0];
    }
}

__global__ void extend_row_major_columns_shifted_rows_fused_partial_kernel(
    const uint64_t* values,
    const uint64_t* weights,
    const uint64_t* weight_shifts,
    uint64_t* partials,
    size_t source_len,
    size_t column_count,
    size_t chunk_count,
    size_t target_row_count) {
    const size_t block_index = blockIdx.x;
    const size_t column = block_index / chunk_count;
    const size_t chunk = block_index - column * chunk_count;
    const size_t row = chunk * blockDim.x + threadIdx.x;
    __shared__ uint64_t sums0[kThreads];
    __shared__ uint64_t sums1[kThreads];
    __shared__ uint64_t sums2[kThreads];
    __shared__ uint64_t sums3[kThreads];
    uint64_t value0 = 0;
    uint64_t value1 = 0;
    uint64_t value2 = 0;
    uint64_t value3 = 0;
    if (column < column_count && row < source_len) {
        const uint64_t source_value = values[row * column_count + column];
        const size_t weight_row0 = row + static_cast<size_t>(weight_shifts[0]);
        const size_t weight_index0 = weight_row0 >= source_len ? weight_row0 - source_len : weight_row0;
        value0 = mul_mod(source_value, weights[weight_index0]);
        if (target_row_count > 1) {
            const size_t weight_row1 = row + static_cast<size_t>(weight_shifts[1]);
            const size_t weight_index1 = weight_row1 >= source_len ? weight_row1 - source_len : weight_row1;
            value1 = mul_mod(source_value, weights[weight_index1]);
        }
        if (target_row_count > 2) {
            const size_t weight_row2 = row + static_cast<size_t>(weight_shifts[2]);
            const size_t weight_index2 = weight_row2 >= source_len ? weight_row2 - source_len : weight_row2;
            value2 = mul_mod(source_value, weights[weight_index2]);
        }
        if (target_row_count > 3) {
            const size_t weight_row3 = row + static_cast<size_t>(weight_shifts[3]);
            const size_t weight_index3 = weight_row3 >= source_len ? weight_row3 - source_len : weight_row3;
            value3 = mul_mod(source_value, weights[weight_index3]);
        }
    }
    sums0[threadIdx.x] = value0;
    sums1[threadIdx.x] = value1;
    sums2[threadIdx.x] = value2;
    sums3[threadIdx.x] = value3;
    __syncthreads();

    for (size_t stride = blockDim.x / 2; stride > 0; stride >>= 1) {
        if (threadIdx.x < stride) {
            sums0[threadIdx.x] = add_mod(sums0[threadIdx.x], sums0[threadIdx.x + stride]);
            if (target_row_count > 1) {
                sums1[threadIdx.x] = add_mod(sums1[threadIdx.x], sums1[threadIdx.x + stride]);
            }
            if (target_row_count > 2) {
                sums2[threadIdx.x] = add_mod(sums2[threadIdx.x], sums2[threadIdx.x + stride]);
            }
            if (target_row_count > 3) {
                sums3[threadIdx.x] = add_mod(sums3[threadIdx.x], sums3[threadIdx.x + stride]);
            }
        }
        __syncthreads();
    }
    if (threadIdx.x == 0 && column < column_count) {
        const size_t partial_base = column * chunk_count + chunk;
        partials[partial_base] = sums0[0];
        if (target_row_count > 1) {
            partials[column_count * chunk_count + partial_base] = sums1[0];
        }
        if (target_row_count > 2) {
            partials[2 * column_count * chunk_count + partial_base] = sums2[0];
        }
        if (target_row_count > 3) {
            partials[3 * column_count * chunk_count + partial_base] = sums3[0];
        }
    }
}

__global__ void extend_row_major_columns_strided_shifted_rows_partial_kernel(
    const uint64_t* values,
    const uint64_t* weights,
    const uint64_t* weight_shifts,
    uint64_t* partials,
    size_t source_len,
    size_t source_row_stride,
    size_t column_offset,
    size_t column_count,
    size_t chunk_count) {
    const size_t block_index = blockIdx.x;
    const size_t column_chunk_count = column_count * chunk_count;
    const size_t target_row = block_index / column_chunk_count;
    const size_t offset = block_index - target_row * column_chunk_count;
    const size_t column = offset / chunk_count;
    const size_t chunk = offset - column * chunk_count;
    const size_t row = chunk * blockDim.x + threadIdx.x;
    __shared__ uint64_t sums[kThreads];
    uint64_t value = 0;
    if (column < column_count && row < source_len) {
        const size_t weight_row = row + static_cast<size_t>(weight_shifts[target_row]);
        const size_t weight_index = weight_row >= source_len ? weight_row - source_len : weight_row;
        value = mul_mod(
            values[row * source_row_stride + column_offset + column], weights[weight_index]);
    }
    sums[threadIdx.x] = value;
    __syncthreads();

    for (size_t stride = blockDim.x / 2; stride > 0; stride >>= 1) {
        if (threadIdx.x < stride) {
            sums[threadIdx.x] = add_mod(sums[threadIdx.x], sums[threadIdx.x + stride]);
        }
        __syncthreads();
    }
    if (threadIdx.x == 0 && column < column_count) {
        partials[(target_row * column_count + column) * chunk_count + chunk] = sums[0];
    }
}

__global__ void extend_row_major_columns_row_final_kernel(
    const uint64_t* partials,
    uint64_t* out,
    size_t chunk_count,
    size_t column_count) {
    const size_t column = blockIdx.x;
    __shared__ uint64_t sums[kThreads];
    uint64_t value = 0;
    if (column < column_count) {
        for (size_t chunk = threadIdx.x; chunk < chunk_count; chunk += blockDim.x) {
            value = add_mod(value, partials[column * chunk_count + chunk]);
        }
    }
    sums[threadIdx.x] = value;
    __syncthreads();

    for (size_t stride = blockDim.x / 2; stride > 0; stride >>= 1) {
        if (threadIdx.x < stride) {
            sums[threadIdx.x] = add_mod(sums[threadIdx.x], sums[threadIdx.x + stride]);
        }
        __syncthreads();
    }
    if (threadIdx.x == 0 && column < column_count) {
        out[column] = sums[0];
    }
}

__global__ void extend_row_major_columns_rows_final_kernel(
    const uint64_t* partials,
    uint64_t* out,
    size_t chunk_count,
    size_t column_count) {
    const size_t block_index = blockIdx.x;
    const size_t target_row = block_index / column_count;
    const size_t column = block_index - target_row * column_count;
    __shared__ uint64_t sums[kThreads];
    uint64_t value = 0;
    if (column < column_count) {
        for (size_t chunk = threadIdx.x; chunk < chunk_count; chunk += blockDim.x) {
            value = add_mod(
                value, partials[(target_row * column_count + column) * chunk_count + chunk]);
        }
    }
    sums[threadIdx.x] = value;
    __syncthreads();

    for (size_t stride = blockDim.x / 2; stride > 0; stride >>= 1) {
        if (threadIdx.x < stride) {
            sums[threadIdx.x] = add_mod(sums[threadIdx.x], sums[threadIdx.x + stride]);
        }
        __syncthreads();
    }
    if (threadIdx.x == 0 && column < column_count) {
        out[target_row * column_count + column] = sums[0];
    }
}

__global__ void extend_row_major_columns_rows_scatter_final_kernel(
    const uint64_t* partials,
    const uint64_t* output_rows,
    uint64_t* out,
    size_t chunk_count,
    size_t column_count) {
    const size_t block_index = blockIdx.x;
    const size_t target_row = block_index / column_count;
    const size_t column = block_index - target_row * column_count;
    __shared__ uint64_t sums[kThreads];
    uint64_t value = 0;
    if (column < column_count) {
        for (size_t chunk = threadIdx.x; chunk < chunk_count; chunk += blockDim.x) {
            value = add_mod(
                value, partials[(target_row * column_count + column) * chunk_count + chunk]);
        }
    }
    sums[threadIdx.x] = value;
    __syncthreads();

    for (size_t stride = blockDim.x / 2; stride > 0; stride >>= 1) {
        if (threadIdx.x < stride) {
            sums[threadIdx.x] = add_mod(sums[threadIdx.x], sums[threadIdx.x + stride]);
        }
        __syncthreads();
    }
    if (threadIdx.x == 0 && column < column_count) {
        const size_t output_row = static_cast<size_t>(output_rows[target_row]);
        out[output_row * column_count + column] = sums[0];
    }
}

extern "C" int lzvm_cuda_goldilocks_coset_extend_row_major_columns_row_device(
    const uint64_t* values,
    const uint64_t* weights,
    uint64_t* out,
    size_t source_len,
    size_t column_count) {
    if (values == nullptr || weights == nullptr || out == nullptr) {
        return -1;
    }
    if (source_len == 0 || column_count == 0) {
        return -2;
    }

    const size_t chunk_count = (source_len + kThreads - 1) / kThreads;
    const size_t partial_count = chunk_count * column_count;
    if (chunk_count == 0 || partial_count / column_count != chunk_count) {
        return -2;
    }
    DeviceBuffer<uint64_t> partials;
    LZVM_CUDA_RETURN_ON_ERROR(partials.reset(partial_count));
    const size_t block_count = partial_count;
    extend_row_major_columns_row_partial_kernel<<<block_count, kThreads>>>(
        values, weights, partials.data(), source_len, column_count, chunk_count);
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_check_launch());
    extend_row_major_columns_row_final_kernel<<<column_count, kThreads>>>(
        partials.data(), out, chunk_count, column_count);
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_check_launch());
    return lzvm_cuda_synchronize();
}

extern "C" int lzvm_cuda_goldilocks_coset_extend_row_major_columns_shifted_row_device(
    const uint64_t* values,
    const uint64_t* weights,
    uint64_t* out,
    size_t source_len,
    size_t column_count,
    size_t weight_shift) {
    if (values == nullptr || weights == nullptr || out == nullptr) {
        return -1;
    }
    if (source_len == 0 || column_count == 0 || weight_shift >= source_len) {
        return -2;
    }

    const size_t chunk_count = (source_len + kThreads - 1) / kThreads;
    const size_t partial_count = chunk_count * column_count;
    if (chunk_count == 0 || partial_count / column_count != chunk_count) {
        return -2;
    }
    DeviceBuffer<uint64_t> partials;
    LZVM_CUDA_RETURN_ON_ERROR(partials.reset(partial_count));
    const size_t block_count = partial_count;
    extend_row_major_columns_shifted_row_partial_kernel<<<block_count, kThreads>>>(
        values, weights, partials.data(), source_len, column_count, chunk_count, weight_shift);
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_check_launch());
    extend_row_major_columns_row_final_kernel<<<column_count, kThreads>>>(
        partials.data(), out, chunk_count, column_count);
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_check_launch());
    return lzvm_cuda_synchronize();
}

extern "C" int lzvm_cuda_goldilocks_coset_extend_row_major_columns_shifted_rows_device(
    const uint64_t* values,
    const uint64_t* weights,
    const uint64_t* weight_shifts,
    const uint64_t* output_rows,
    uint64_t* out,
    size_t source_len,
    size_t column_count,
    size_t target_row_count) {
    if (target_row_count == 0) {
        return 0;
    }
    if (values == nullptr || weights == nullptr || weight_shifts == nullptr ||
        output_rows == nullptr || out == nullptr) {
        return -1;
    }
    if (source_len == 0 || column_count == 0) {
        return -2;
    }

    const size_t chunk_count = (source_len + kThreads - 1) / kThreads;
    const size_t row_column_count = target_row_count * column_count;
    if (chunk_count == 0 || row_column_count / column_count != target_row_count) {
        return -2;
    }
    const size_t partial_count = chunk_count * row_column_count;
    if (partial_count / row_column_count != chunk_count) {
        return -2;
    }
    DeviceBuffer<uint64_t> partials;
    LZVM_CUDA_RETURN_ON_ERROR(partials.reset(partial_count));
    if (target_row_count >= 2 && target_row_count <= 4) {
        extend_row_major_columns_shifted_rows_fused_partial_kernel<<<chunk_count * column_count, kThreads>>>(
            values, weights, weight_shifts, partials.data(), source_len, column_count,
            chunk_count, target_row_count);
    } else {
        extend_row_major_columns_shifted_rows_partial_kernel<<<partial_count, kThreads>>>(
            values, weights, weight_shifts, partials.data(), source_len, column_count, chunk_count);
    }
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_check_launch());
    extend_row_major_columns_rows_scatter_final_kernel<<<row_column_count, kThreads>>>(
        partials.data(), output_rows, out, chunk_count, column_count);
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_check_launch());
    return lzvm_cuda_synchronize();
}

extern "C" int lzvm_cuda_goldilocks_coset_extend_row_major_columns_rows_device(
    const uint64_t* values,
    const uint64_t* weights,
    uint64_t* out,
    size_t source_len,
    size_t column_count,
    size_t target_row_count) {
    if (target_row_count == 0) {
        return 0;
    }
    if (values == nullptr || weights == nullptr || out == nullptr) {
        return -1;
    }
    if (source_len == 0 || column_count == 0) {
        return -2;
    }

    const size_t chunk_count = (source_len + kThreads - 1) / kThreads;
    const size_t row_column_count = target_row_count * column_count;
    if (chunk_count == 0 || row_column_count / column_count != target_row_count) {
        return -2;
    }
    const size_t partial_count = chunk_count * row_column_count;
    if (partial_count / row_column_count != chunk_count) {
        return -2;
    }
    DeviceBuffer<uint64_t> partials;
    LZVM_CUDA_RETURN_ON_ERROR(partials.reset(partial_count));
    extend_row_major_columns_rows_partial_kernel<<<partial_count, kThreads>>>(
        values, weights, partials.data(), source_len, column_count, chunk_count);
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_check_launch());
    extend_row_major_columns_rows_final_kernel<<<row_column_count, kThreads>>>(
        partials.data(), out, chunk_count, column_count);
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_check_launch());
    return lzvm_cuda_synchronize();
}

extern "C" int lzvm_cuda_goldilocks_coset_extend_row_major_columns_selected_rows_device(
    const uint64_t* values,
    const uint64_t* weights,
    uint64_t* out,
    size_t source_len,
    size_t column_count,
    size_t target_row_count) {
    return lzvm_cuda_goldilocks_coset_extend_row_major_columns_rows_device(
        values, weights, out, source_len, column_count, target_row_count);
}

extern "C" int lzvm_cuda_goldilocks_coset_extend_row_major_columns_strided_row_device(
    const uint64_t* values,
    const uint64_t* weights,
    uint64_t* out,
    size_t source_len,
    size_t source_row_stride,
    size_t column_offset,
    size_t column_count) {
    if (values == nullptr || weights == nullptr || out == nullptr) {
        return -1;
    }
    if (source_len == 0 || source_row_stride == 0 || column_count == 0 ||
        column_offset > source_row_stride ||
        column_count > source_row_stride - column_offset) {
        return -2;
    }

    const size_t chunk_count = (source_len + kThreads - 1) / kThreads;
    const size_t partial_count = chunk_count * column_count;
    if (chunk_count == 0 || partial_count / column_count != chunk_count) {
        return -2;
    }
    DeviceBuffer<uint64_t> partials;
    LZVM_CUDA_RETURN_ON_ERROR(partials.reset(partial_count));
    const size_t block_count = partial_count;
    extend_row_major_columns_strided_row_partial_kernel<<<block_count, kThreads>>>(
        values, weights, partials.data(), source_len, source_row_stride, column_offset,
        column_count, chunk_count);
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_check_launch());
    extend_row_major_columns_row_final_kernel<<<column_count, kThreads>>>(
        partials.data(), out, chunk_count, column_count);
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_check_launch());
    return lzvm_cuda_synchronize();
}

extern "C" int lzvm_cuda_goldilocks_coset_extend_row_major_columns_strided_shifted_row_device(
    const uint64_t* values,
    const uint64_t* weights,
    uint64_t* out,
    size_t source_len,
    size_t source_row_stride,
    size_t column_offset,
    size_t column_count,
    size_t weight_shift) {
    if (values == nullptr || weights == nullptr || out == nullptr) {
        return -1;
    }
    if (source_len == 0 || source_row_stride == 0 || column_count == 0 ||
        weight_shift >= source_len || column_offset > source_row_stride ||
        column_count > source_row_stride - column_offset) {
        return -2;
    }

    const size_t chunk_count = (source_len + kThreads - 1) / kThreads;
    const size_t partial_count = chunk_count * column_count;
    if (chunk_count == 0 || partial_count / column_count != chunk_count) {
        return -2;
    }
    DeviceBuffer<uint64_t> partials;
    LZVM_CUDA_RETURN_ON_ERROR(partials.reset(partial_count));
    const size_t block_count = partial_count;
    extend_row_major_columns_strided_shifted_row_partial_kernel<<<block_count, kThreads>>>(
        values, weights, partials.data(), source_len, source_row_stride, column_offset,
        column_count, chunk_count, weight_shift);
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_check_launch());
    extend_row_major_columns_row_final_kernel<<<column_count, kThreads>>>(
        partials.data(), out, chunk_count, column_count);
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_check_launch());
    return lzvm_cuda_synchronize();
}

extern "C" int lzvm_cuda_goldilocks_coset_extend_row_major_columns_strided_shifted_rows_device(
    const uint64_t* values,
    const uint64_t* weights,
    const uint64_t* weight_shifts,
    const uint64_t* output_rows,
    uint64_t* out,
    size_t source_len,
    size_t source_row_stride,
    size_t column_offset,
    size_t column_count,
    size_t target_row_count) {
    if (target_row_count == 0) {
        return 0;
    }
    if (values == nullptr || weights == nullptr || weight_shifts == nullptr ||
        output_rows == nullptr || out == nullptr) {
        return -1;
    }
    if (source_len == 0 || source_row_stride == 0 || column_count == 0 ||
        column_offset > source_row_stride ||
        column_count > source_row_stride - column_offset) {
        return -2;
    }

    const size_t chunk_count = (source_len + kThreads - 1) / kThreads;
    const size_t row_column_count = target_row_count * column_count;
    if (chunk_count == 0 || row_column_count / column_count != target_row_count) {
        return -2;
    }
    const size_t partial_count = chunk_count * row_column_count;
    if (partial_count / row_column_count != chunk_count) {
        return -2;
    }
    DeviceBuffer<uint64_t> partials;
    LZVM_CUDA_RETURN_ON_ERROR(partials.reset(partial_count));
    extend_row_major_columns_strided_shifted_rows_partial_kernel<<<partial_count, kThreads>>>(
        values, weights, weight_shifts, partials.data(), source_len, source_row_stride,
        column_offset, column_count, chunk_count);
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_check_launch());
    extend_row_major_columns_rows_scatter_final_kernel<<<row_column_count, kThreads>>>(
        partials.data(), output_rows, out, chunk_count, column_count);
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_check_launch());
    return lzvm_cuda_synchronize();
}

extern "C" int lzvm_cuda_goldilocks_coset_extend_row_major_columns_strided_selected_rows_device(
    const uint64_t* values,
    const uint64_t* weights,
    uint64_t* out,
    size_t source_len,
    size_t source_row_stride,
    size_t column_offset,
    size_t column_count,
    size_t target_row_count) {
    return lzvm_cuda_goldilocks_coset_extend_row_major_columns_strided_rows_device(
        values, weights, out, source_len, source_row_stride, column_offset, column_count,
        target_row_count);
}

extern "C" int lzvm_cuda_goldilocks_coset_extend_row_major_columns_strided_rows_device(
    const uint64_t* values,
    const uint64_t* weights,
    uint64_t* out,
    size_t source_len,
    size_t source_row_stride,
    size_t column_offset,
    size_t column_count,
    size_t target_row_count) {
    if (target_row_count == 0) {
        return 0;
    }
    if (values == nullptr || weights == nullptr || out == nullptr) {
        return -1;
    }
    if (source_len == 0 || source_row_stride == 0 || column_count == 0 ||
        column_offset > source_row_stride ||
        column_count > source_row_stride - column_offset) {
        return -2;
    }

    const size_t chunk_count = (source_len + kThreads - 1) / kThreads;
    const size_t row_column_count = target_row_count * column_count;
    if (chunk_count == 0 || row_column_count / column_count != target_row_count) {
        return -2;
    }
    const size_t partial_count = chunk_count * row_column_count;
    if (partial_count / row_column_count != chunk_count) {
        return -2;
    }
    DeviceBuffer<uint64_t> partials;
    LZVM_CUDA_RETURN_ON_ERROR(partials.reset(partial_count));
    extend_row_major_columns_strided_rows_partial_kernel<<<partial_count, kThreads>>>(
        values, weights, partials.data(), source_len, source_row_stride, column_offset,
        column_count, chunk_count);
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_check_launch());
    extend_row_major_columns_rows_final_kernel<<<row_column_count, kThreads>>>(
        partials.data(), out, chunk_count, column_count);
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_check_launch());
    return lzvm_cuda_synchronize();
}
