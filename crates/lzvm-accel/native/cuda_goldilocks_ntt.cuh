#pragma once

__device__ uint64_t ntt_stage_twiddle(
    size_t len,
    size_t stage_len,
    size_t stage_bits,
    uint64_t root,
    bool inverse_roots) {
    return stage_bits <= static_cast<size_t>(kNttStageRootLimit)
        ? (inverse_roots ? kNttStageRootInverses[stage_bits] : kNttStageRoots[stage_bits])
        : pow_mod(root, len / stage_len);
}

__global__ void ntt_stage_kernel(
    uint64_t* values,
    size_t len,
    size_t stage_len,
    size_t stage_bits,
    uint64_t root,
    bool inverse_roots) {
    const size_t pair_count = len / 2;
    const size_t pair = blockIdx.x * blockDim.x + threadIdx.x;
    if (pair < pair_count) {
        const size_t half = stage_len / 2;
        const size_t group = pair / half;
        const size_t offset = pair % half;
        const size_t even_index = group * stage_len + offset;
        const size_t odd_index = even_index + half;
        const uint64_t stage_twiddle =
            ntt_stage_twiddle(len, stage_len, stage_bits, root, inverse_roots);
        const uint64_t factor = pow_mod(stage_twiddle, offset);
        const uint64_t even = values[even_index];
        const uint64_t odd = mul_mod(values[odd_index], factor);
        values[even_index] = add_mod(even, odd);
        values[odd_index] = sub_mod(even, odd);
    }
}

__global__ void ntt_stage_block_twiddle_kernel(
    uint64_t* values,
    size_t len,
    size_t stage_len,
    size_t stage_bits,
    uint64_t root,
    bool inverse_roots) {
    __shared__ uint64_t block_base;
    __shared__ uint64_t stage_twiddle;

    const size_t pair_count = len / 2;
    const size_t pair = blockIdx.x * blockDim.x + threadIdx.x;
    const size_t half = stage_len / 2;
    const size_t block_offset = (blockIdx.x * blockDim.x) % half;
    if (threadIdx.x == 0) {
        stage_twiddle = ntt_stage_twiddle(len, stage_len, stage_bits, root, inverse_roots);
        block_base = pow_mod(stage_twiddle, block_offset);
    }
    __syncthreads();

    if (pair < pair_count) {
        const size_t group = pair / half;
        const size_t offset = block_offset + threadIdx.x;
        const size_t even_index = group * stage_len + offset;
        const size_t odd_index = even_index + half;
        const uint64_t factor =
            mul_mod(block_base, pow_mod(stage_twiddle, static_cast<size_t>(threadIdx.x)));
        const uint64_t even = values[even_index];
        const uint64_t odd = mul_mod(values[odd_index], factor);
        values[even_index] = add_mod(even, odd);
        values[odd_index] = sub_mod(even, odd);
    }
}

cudaError_t run_ntt(
    uint64_t* device_values,
    size_t len,
    size_t bits,
    uint64_t root,
    bool inverse_roots) {
    const size_t blocks = (len + kThreads - 1) / kThreads;
    bit_reverse_kernel<<<blocks, kThreads>>>(device_values, len, bits);
    cudaError_t status = static_cast<cudaError_t>(lzvm_cuda_check_launch());
    if (status != cudaSuccess) {
        return status;
    }

    for (size_t stage_len = 2, stage_bits = 1; stage_len <= len; stage_len <<= 1, ++stage_bits) {
        const size_t pair_count = len / 2;
        const size_t stage_blocks = (pair_count + kThreads - 1) / kThreads;
        if (stage_len / 2 > kThreads) {
            ntt_stage_block_twiddle_kernel<<<stage_blocks, kThreads>>>(
                device_values, len, stage_len, stage_bits, root, inverse_roots);
        } else {
            ntt_stage_kernel<<<stage_blocks, kThreads>>>(
                device_values, len, stage_len, stage_bits, root, inverse_roots);
        }
        status = static_cast<cudaError_t>(lzvm_cuda_check_launch());
        if (status != cudaSuccess) {
            return status;
        }
    }
    return cudaSuccess;
}

int run_coset_extend_on_device_unsynced(
    uint64_t* device_values,
    size_t source_len,
    size_t source_bits,
    size_t target_len,
    size_t target_bits,
    uint64_t source_root_inverse,
    uint64_t target_root,
    uint64_t shift) {
    cudaError_t status =
        run_ntt(device_values, source_len, source_bits, source_root_inverse, true);
    if (status != cudaSuccess) {
        return static_cast<int>(status);
    }

    const uint64_t inverse_len = host_pow_mod(static_cast<uint64_t>(source_len), kModulus - 2);
    const size_t blocks = (target_len + kThreads - 1) / kThreads;
    normalize_shift_and_pad_kernel<<<blocks, kThreads>>>(
        device_values, source_len, target_len, inverse_len, shift);
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_check_launch());

    status = run_ntt(device_values, target_len, target_bits, target_root, false);
    if (status != cudaSuccess) {
        return static_cast<int>(status);
    }

    return 0;
}

int run_coset_extend_on_device(
    uint64_t* device_values,
    size_t source_len,
    size_t source_bits,
    size_t target_len,
    size_t target_bits,
    uint64_t source_root_inverse,
    uint64_t target_root,
    uint64_t shift) {
    LZVM_CUDA_RETURN_ON_ERROR(run_coset_extend_on_device_unsynced(
        device_values, source_len, source_bits, target_len, target_bits, source_root_inverse,
        target_root, shift));
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_synchronize());
    return 0;
}
