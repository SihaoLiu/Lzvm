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
