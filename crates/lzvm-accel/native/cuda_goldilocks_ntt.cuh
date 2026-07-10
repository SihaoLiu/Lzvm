#pragma once

constexpr size_t kNttThreadPowerBits = 8;
constexpr size_t kNttThreadPowerKernelMinHalf = 16;
constexpr size_t kNttThreadFactorMinBits = 10;
constexpr size_t kNttThreadFactorStageCount = kMaxRootBits - kNttThreadFactorMinBits + 1;
constexpr size_t kNttThreadFactorDirectionCount = 2;
constexpr size_t kNttThreadFactorCount =
    kNttThreadFactorDirectionCount * kNttThreadFactorStageCount * kThreads;

__device__ uint64_t kNttThreadFactors[kNttThreadFactorCount];
__device__ uint64_t
    kNttBlockTwiddles[kNttThreadFactorDirectionCount * kNttThreadFactorStageCount];

int setup_ntt_thread_factors(
    const uint64_t* roots,
    const uint64_t* inverse_roots,
    size_t root_count) {
    std::vector<uint64_t> thread_factors(kNttThreadFactorCount);
    std::vector<uint64_t> block_twiddles(
        kNttThreadFactorDirectionCount * kNttThreadFactorStageCount);
    for (size_t direction = 0; direction < kNttThreadFactorDirectionCount; ++direction) {
        for (size_t stage_bits = kNttThreadFactorMinBits; stage_bits < root_count; ++stage_bits) {
            const size_t stage_index =
                direction * kNttThreadFactorStageCount + stage_bits - kNttThreadFactorMinBits;
            const uint64_t stage_twiddle =
                direction == 0 ? roots[stage_bits] : inverse_roots[stage_bits];
            uint64_t factor = 1;
            const size_t factor_offset = stage_index * kThreads;
            for (size_t thread = 0; thread < kThreads; ++thread) {
                thread_factors[factor_offset + thread] = factor;
                factor = host_mul_mod(factor, stage_twiddle);
            }
            block_twiddles[stage_index] = factor;
        }
    }
    cudaError_t status = cudaMemcpyToSymbol(
        kNttThreadFactors,
        thread_factors.data(),
        thread_factors.size() * sizeof(uint64_t));
    if (status != cudaSuccess) {
        return static_cast<int>(status);
    }
    return static_cast<int>(
        cudaMemcpyToSymbol(
            kNttBlockTwiddles,
            block_twiddles.data(),
            block_twiddles.size() * sizeof(uint64_t)));
}

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

__device__ void ntt_stage_prepare_thread_powers(
    size_t len,
    size_t stage_len,
    size_t stage_bits,
    uint64_t root,
    bool inverse_roots,
    uint64_t* thread_powers) {
    if (threadIdx.x == 0) {
        const uint64_t stage_twiddle =
            ntt_stage_twiddle(len, stage_len, stage_bits, root, inverse_roots);
        thread_powers[0] = stage_twiddle;
        for (size_t bit = 1; bit < kNttThreadPowerBits; ++bit) {
            thread_powers[bit] = mul_mod(thread_powers[bit - 1], thread_powers[bit - 1]);
        }
    }
    __syncthreads();
}

__device__ size_t ntt_stage_thread_factor_index(size_t stage_bits, bool inverse_roots) {
    const size_t direction_offset =
        inverse_roots ? kNttThreadFactorStageCount : static_cast<size_t>(0);
    return direction_offset + stage_bits - kNttThreadFactorMinBits;
}

__device__ uint64_t ntt_stage_block_base(uint64_t block_twiddle, size_t block_offset) {
    if (block_offset == 0) {
        return 1;
    }
    return pow_mod(block_twiddle, block_offset >> kNttThreadPowerBits);
}

__device__ uint64_t ntt_stage_thread_twiddle(
    uint64_t base,
    const uint64_t* thread_powers,
    size_t exponent) {
    uint64_t factor = base;
    size_t bit = 0;
    while (exponent != 0 && bit < kNttThreadPowerBits) {
        if ((exponent & 1) != 0) {
            factor = mul_mod(factor, thread_powers[bit]);
        }
        exponent >>= 1;
        ++bit;
    }
    return factor;
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

__global__ void ntt_stage_thread_twiddle_kernel(
    uint64_t* values,
    size_t len,
    size_t stage_len,
    size_t stage_bits,
    uint64_t root,
    bool inverse_roots) {
    __shared__ uint64_t thread_powers[kNttThreadPowerBits];

    ntt_stage_prepare_thread_powers(
        len, stage_len, stage_bits, root, inverse_roots, thread_powers);

    const size_t pair_count = len / 2;
    const size_t pair = blockIdx.x * blockDim.x + threadIdx.x;
    if (pair < pair_count) {
        const size_t half = stage_len / 2;
        const size_t group = pair / half;
        const size_t offset = pair % half;
        const size_t even_index = group * stage_len + offset;
        const size_t odd_index = even_index + half;
        const uint64_t factor = ntt_stage_thread_twiddle(1, thread_powers, offset);
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
    bool inverse_roots) {
    __shared__ uint64_t block_base;

    const size_t pair_count = len / 2;
    const size_t pair = blockIdx.x * blockDim.x + threadIdx.x;
    const size_t half = stage_len / 2;
    const size_t block_offset = (blockIdx.x * blockDim.x) % half;
    const size_t factor_stage = ntt_stage_thread_factor_index(stage_bits, inverse_roots);
    if (threadIdx.x == 0) {
        block_base = ntt_stage_block_base(kNttBlockTwiddles[factor_stage], block_offset);
    }
    __syncthreads();

    if (pair < pair_count) {
        const size_t group = pair / half;
        const size_t offset = block_offset + threadIdx.x;
        const size_t even_index = group * stage_len + offset;
        const size_t odd_index = even_index + half;
        const uint64_t thread_factor =
            __ldg(&kNttThreadFactors[factor_stage * kThreads + threadIdx.x]);
        const uint64_t factor = mul_mod(block_base, thread_factor);
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
    bool inverse_roots,
    cudaStream_t stream) {
    const size_t blocks = (len + kThreads - 1) / kThreads;
    bit_reverse_kernel<<<blocks, kThreads, 0, stream>>>(device_values, len, bits);
    cudaError_t status = static_cast<cudaError_t>(lzvm_cuda_check_launch());
    if (status != cudaSuccess) {
        return status;
    }

    for (size_t stage_len = 2, stage_bits = 1; stage_len <= len; stage_len <<= 1, ++stage_bits) {
        const size_t pair_count = len / 2;
        const size_t stage_blocks = (pair_count + kThreads - 1) / kThreads;
        const size_t half = stage_len / 2;
        if (half > kThreads) {
            ntt_stage_block_twiddle_kernel<<<stage_blocks, kThreads, 0, stream>>>(
                device_values, len, stage_len, stage_bits, inverse_roots);
        } else if (half >= kNttThreadPowerKernelMinHalf) {
            ntt_stage_thread_twiddle_kernel<<<stage_blocks, kThreads, 0, stream>>>(
                device_values, len, stage_len, stage_bits, root, inverse_roots);
        } else {
            ntt_stage_kernel<<<stage_blocks, kThreads, 0, stream>>>(
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
    uint64_t shift,
    cudaStream_t stream) {
    cudaError_t status =
        run_ntt(device_values, source_len, source_bits, source_root_inverse, true, stream);
    if (status != cudaSuccess) {
        return static_cast<int>(status);
    }

    const uint64_t inverse_len = host_pow_mod(static_cast<uint64_t>(source_len), kModulus - 2);
    const size_t blocks = (target_len + kThreads - 1) / kThreads;
    normalize_shift_and_pad_kernel<<<blocks, kThreads, 0, stream>>>(
        device_values, source_len, target_len, inverse_len, shift);
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_check_launch());

    status = run_ntt(device_values, target_len, target_bits, target_root, false, stream);
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
        target_root, shift, 0));
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_synchronize());
    return 0;
}
