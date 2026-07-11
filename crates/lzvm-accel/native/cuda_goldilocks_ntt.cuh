#pragma once

constexpr size_t kNttThreadPowerBits = 8;
constexpr size_t kNttThreadFactorMinBits = 1;
constexpr size_t kNttThreadFactorStageCount = kMaxRootBits - kNttThreadFactorMinBits + 1;
constexpr size_t kNttThreadFactorDirectionCount = 2;
constexpr size_t kNttThreadFactorCount =
    kNttThreadFactorDirectionCount * kNttThreadFactorStageCount * kThreads;
constexpr size_t kNttInitialStageBits = 9;
constexpr size_t kNttInitialStageLen = static_cast<size_t>(1) << kNttInitialStageBits;
constexpr size_t kNttColumnGroupSize = 4;
constexpr size_t kNttBlockStageColumnsPerThread = 2;

__device__ uint64_t kNttThreadFactors[kNttThreadFactorCount];
__device__ uint64_t
    kNttBlockTwiddles[kNttThreadFactorDirectionCount * kNttThreadFactorStageCount];
uint64_t gNttStageRoots[kMaxRootBits + 1];
uint64_t gNttStageRootInverses[kMaxRootBits + 1];
size_t gNttStageRootCount = 0;

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
    gNttStageRootCount = 0;
    cudaError_t status = cudaMemcpyToSymbol(
        kNttThreadFactors,
        thread_factors.data(),
        thread_factors.size() * sizeof(uint64_t));
    if (status != cudaSuccess) {
        return static_cast<int>(status);
    }
    status = cudaMemcpyToSymbol(
        kNttBlockTwiddles,
        block_twiddles.data(),
        block_twiddles.size() * sizeof(uint64_t));
    if (status != cudaSuccess) {
        return static_cast<int>(status);
    }
    std::copy_n(roots, root_count, gNttStageRoots);
    std::copy_n(inverse_roots, root_count, gNttStageRootInverses);
    gNttStageRootCount = root_count;
    return 0;
}

bool ntt_uses_precomputed_factors(size_t bits, uint64_t root, bool inverse_roots) {
    if (bits >= gNttStageRootCount) {
        return false;
    }
    const uint64_t expected_root =
        inverse_roots ? gNttStageRootInverses[bits] : gNttStageRoots[bits];
    return root == expected_root;
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

__global__ void ntt_stage_kernel(
    uint64_t* values,
    size_t len,
    size_t stage_len,
    size_t stage_bits,
    bool inverse_roots) {
    const size_t pair_count = len / 2;
    const size_t pair = blockIdx.x * blockDim.x + threadIdx.x;
    if (pair < pair_count) {
        const size_t half = stage_len / 2;
        const size_t group = pair / half;
        const size_t offset = pair % half;
        const size_t even_index = group * stage_len + offset;
        const size_t odd_index = even_index + half;
        const size_t factor_stage = ntt_stage_thread_factor_index(stage_bits, inverse_roots);
        const uint64_t factor =
            __ldg(&kNttThreadFactors[factor_stage * kThreads + offset]);
        const uint64_t even = values[even_index];
        const uint64_t odd = mul_mod(values[odd_index], factor);
        values[even_index] = add_mod(even, odd);
        values[odd_index] = sub_mod(even, odd);
    }
}

// After bit reversal, stages 1 through 9 are independent within 512-value chunks.
__global__ void ntt_initial_stages_kernel(
    uint64_t* values,
    size_t len,
    size_t value_stride,
    size_t fused_stage_bits,
    bool inverse_roots) {
    __shared__ uint64_t stage_values[kNttInitialStageLen];

    const size_t chunk_len = static_cast<size_t>(1) << fused_stage_bits;
    const size_t chunk_start = static_cast<size_t>(blockIdx.x) * chunk_len;
    uint64_t* column_values = values + static_cast<size_t>(blockIdx.y) * value_stride;
    for (size_t index = threadIdx.x; index < chunk_len; index += blockDim.x) {
        const size_t value_index = chunk_start + index;
        stage_values[index] = value_index < len ? column_values[value_index] : 0;
    }
    __syncthreads();

#pragma unroll
    for (size_t stage_bits = 1; stage_bits <= kNttInitialStageBits; ++stage_bits) {
        if (stage_bits <= fused_stage_bits) {
            const size_t half = static_cast<size_t>(1) << (stage_bits - 1);
            const size_t pair = threadIdx.x;
            if (pair < chunk_len / 2) {
                const size_t group = pair / half;
                const size_t offset = pair % half;
                const size_t even_index = group * (half * 2) + offset;
                const size_t odd_index = even_index + half;
                const size_t factor_stage =
                    ntt_stage_thread_factor_index(stage_bits, inverse_roots);
                const uint64_t factor =
                    __ldg(&kNttThreadFactors[factor_stage * kThreads + offset]);
                const uint64_t even = stage_values[even_index];
                const uint64_t odd = mul_mod(stage_values[odd_index], factor);
                stage_values[even_index] = add_mod(even, odd);
                stage_values[odd_index] = sub_mod(even, odd);
            }
        }
        __syncthreads();
    }

    for (size_t index = threadIdx.x; index < chunk_len; index += blockDim.x) {
        if (chunk_start + index < len) {
            column_values[chunk_start + index] = stage_values[index];
        }
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

__global__ void ntt_stage_root_kernel(
    uint64_t* values,
    size_t len,
    size_t stage_len,
    uint64_t stage_twiddle) {
    const size_t pair_count = len / 2;
    const size_t pair = blockIdx.x * blockDim.x + threadIdx.x;
    if (pair < pair_count) {
        const size_t half = stage_len / 2;
        const size_t group = pair / half;
        const size_t offset = pair % half;
        const size_t even_index = group * stage_len + offset;
        const size_t odd_index = even_index + half;
        const uint64_t factor = pow_mod(stage_twiddle, offset);
        const uint64_t even = values[even_index];
        const uint64_t odd = mul_mod(values[odd_index], factor);
        values[even_index] = add_mod(even, odd);
        values[odd_index] = sub_mod(even, odd);
    }
}

__global__ void bit_reverse_column_group_kernel(
    uint64_t* values,
    size_t len,
    size_t value_stride,
    size_t bits,
    size_t column_count) {
    const size_t index = blockIdx.x * blockDim.x + threadIdx.x;
    if (index < len) {
        const size_t reverse = reverse_bits(index, bits);
        if (index < reverse) {
#pragma unroll
            for (size_t column = 0; column < kNttColumnGroupSize; ++column) {
                if (column < column_count) {
                    uint64_t* column_values = values + column * value_stride;
                    const uint64_t tmp = column_values[index];
                    column_values[index] = column_values[reverse];
                    column_values[reverse] = tmp;
                }
            }
        }
    }
}

__global__ void ntt_stage_column_group_kernel(
    uint64_t* values,
    size_t len,
    size_t value_stride,
    size_t stage_len,
    size_t stage_bits,
    bool inverse_roots) {
    const size_t pair_count = len / 2;
    const size_t pair = blockIdx.x * blockDim.x + threadIdx.x;
    if (pair < pair_count) {
        uint64_t* column_values =
            values + static_cast<size_t>(blockIdx.y) * value_stride;
        const size_t half = stage_len / 2;
        const size_t group = pair / half;
        const size_t offset = pair % half;
        const size_t even_index = group * stage_len + offset;
        const size_t odd_index = even_index + half;
        const size_t factor_stage = ntt_stage_thread_factor_index(stage_bits, inverse_roots);
        const uint64_t factor =
            __ldg(&kNttThreadFactors[factor_stage * kThreads + offset]);
        const uint64_t even = column_values[even_index];
        const uint64_t odd = mul_mod(column_values[odd_index], factor);
        column_values[even_index] = add_mod(even, odd);
        column_values[odd_index] = sub_mod(even, odd);
    }
}

__global__ void ntt_stage_block_twiddle_column_group_kernel(
    uint64_t* values,
    size_t len,
    size_t value_stride,
    size_t column_count,
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
        const size_t first_column =
            static_cast<size_t>(blockIdx.y) * kNttBlockStageColumnsPerThread;
        const size_t group = pair / half;
        const size_t offset = block_offset + threadIdx.x;
        const size_t even_index = group * stage_len + offset;
        const size_t odd_index = even_index + half;
        const uint64_t thread_factor =
            __ldg(&kNttThreadFactors[factor_stage * kThreads + threadIdx.x]);
        const uint64_t factor = mul_mod(block_base, thread_factor);
#pragma unroll
        for (size_t local_column = 0; local_column < kNttBlockStageColumnsPerThread;
             ++local_column) {
            const size_t column = first_column + local_column;
            if (column < column_count) {
                uint64_t* column_values = values + column * value_stride;
                const uint64_t even = column_values[even_index];
                const uint64_t odd = mul_mod(column_values[odd_index], factor);
                column_values[even_index] = add_mod(even, odd);
                column_values[odd_index] = sub_mod(even, odd);
            }
        }
    }
}

__global__ void ntt_stage_root_column_group_kernel(
    uint64_t* values,
    size_t len,
    size_t value_stride,
    size_t stage_len,
    uint64_t stage_twiddle) {
    const size_t pair_count = len / 2;
    const size_t pair = blockIdx.x * blockDim.x + threadIdx.x;
    if (pair < pair_count) {
        uint64_t* column_values =
            values + static_cast<size_t>(blockIdx.y) * value_stride;
        const size_t half = stage_len / 2;
        const size_t group = pair / half;
        const size_t offset = pair % half;
        const size_t even_index = group * stage_len + offset;
        const size_t odd_index = even_index + half;
        const uint64_t factor = pow_mod(stage_twiddle, offset);
        const uint64_t even = column_values[even_index];
        const uint64_t odd = mul_mod(column_values[odd_index], factor);
        column_values[even_index] = add_mod(even, odd);
        column_values[odd_index] = sub_mod(even, odd);
    }
}

__global__ void normalize_shift_and_pad_column_group_kernel(
    uint64_t* values,
    size_t source_len,
    size_t target_len,
    size_t value_stride,
    size_t column_count,
    uint64_t inverse_len,
    uint64_t shift) {
    __shared__ uint64_t block_shift;
    __shared__ uint64_t thread_powers[kNttThreadPowerBits];
    if (threadIdx.x == 0) {
        block_shift = pow_mod(shift, static_cast<size_t>(blockIdx.x) * blockDim.x);
        thread_powers[0] = shift;
        for (size_t bit = 1; bit < kNttThreadPowerBits; ++bit) {
            thread_powers[bit] = mul_mod(thread_powers[bit - 1], thread_powers[bit - 1]);
        }
    }
    __syncthreads();

    const size_t index = blockIdx.x * blockDim.x + threadIdx.x;
    if (index < target_len) {
        uint64_t factor = 0;
        if (index < source_len) {
            factor = block_shift;
            size_t exponent = static_cast<size_t>(threadIdx.x);
            for (size_t bit = 0; exponent != 0 && bit < kNttThreadPowerBits;
                 ++bit, exponent >>= 1) {
                if ((exponent & 1) != 0) {
                    factor = mul_mod(factor, thread_powers[bit]);
                }
            }
        }
#pragma unroll
        for (size_t column = 0; column < kNttColumnGroupSize; ++column) {
            if (column < column_count) {
                uint64_t* column_values = values + column * value_stride;
                column_values[index] = index < source_len
                    ? mul_mod(mul_mod(column_values[index], inverse_len), factor)
                    : 0;
            }
        }
    }
}

cudaError_t run_ntt(
    uint64_t* device_values,
    size_t len,
    size_t bits,
    uint64_t root,
    bool inverse_roots,
    cudaStream_t stream) {
    const bool use_precomputed_factors =
        ntt_uses_precomputed_factors(bits, root, inverse_roots);
    const size_t blocks = (len + kThreads - 1) / kThreads;
    bit_reverse_kernel<<<blocks, kThreads, 0, stream>>>(device_values, len, bits);
    cudaError_t status = static_cast<cudaError_t>(lzvm_cuda_check_launch());
    if (status != cudaSuccess) {
        return status;
    }

    size_t first_stage_bits = 1;
    if (use_precomputed_factors && bits != 0) {
        const size_t fused_stage_bits = std::min(bits, kNttInitialStageBits);
        const size_t fused_stage_len = static_cast<size_t>(1) << fused_stage_bits;
        const size_t fused_blocks = (len + fused_stage_len - 1) / fused_stage_len;
        ntt_initial_stages_kernel<<<fused_blocks, kThreads, 0, stream>>>(
            device_values, len, 0, fused_stage_bits, inverse_roots);
        status = static_cast<cudaError_t>(lzvm_cuda_check_launch());
        if (status != cudaSuccess) {
            return status;
        }
        first_stage_bits = fused_stage_bits + 1;
    }

    size_t stage_len = static_cast<size_t>(1) << first_stage_bits;
    for (size_t stage_bits = first_stage_bits; stage_len <= len;
         stage_len <<= 1, ++stage_bits) {
        const size_t pair_count = len / 2;
        const size_t stage_blocks = (pair_count + kThreads - 1) / kThreads;
        const size_t half = stage_len / 2;
        if (!use_precomputed_factors) {
            const uint64_t stage_twiddle = host_pow_mod(root, len / stage_len);
            ntt_stage_root_kernel<<<stage_blocks, kThreads, 0, stream>>>(
                device_values, len, stage_len, stage_twiddle);
        } else if (half > kThreads) {
            ntt_stage_block_twiddle_kernel<<<stage_blocks, kThreads, 0, stream>>>(
                device_values, len, stage_len, stage_bits, inverse_roots);
        } else {
            ntt_stage_kernel<<<stage_blocks, kThreads, 0, stream>>>(
                device_values, len, stage_len, stage_bits, inverse_roots);
        }
        status = static_cast<cudaError_t>(lzvm_cuda_check_launch());
        if (status != cudaSuccess) {
            return status;
        }
    }
    return cudaSuccess;
}

cudaError_t run_ntt_column_group(
    uint64_t* device_values,
    size_t len,
    size_t value_stride,
    size_t bits,
    size_t column_count,
    uint64_t root,
    bool inverse_roots,
    cudaStream_t stream) {
    if (column_count == 0 || column_count > kNttColumnGroupSize) {
        return cudaErrorInvalidValue;
    }
    const bool use_precomputed_factors =
        ntt_uses_precomputed_factors(bits, root, inverse_roots);
    const size_t blocks = (len + kThreads - 1) / kThreads;
    bit_reverse_column_group_kernel<<<blocks, kThreads, 0, stream>>>(
        device_values, len, value_stride, bits, column_count);
    cudaError_t status = static_cast<cudaError_t>(lzvm_cuda_check_launch());
    if (status != cudaSuccess) {
        return status;
    }

    size_t first_stage_bits = 1;
    if (use_precomputed_factors && bits != 0) {
        const size_t fused_stage_bits = std::min(bits, kNttInitialStageBits);
        const size_t fused_stage_len = static_cast<size_t>(1) << fused_stage_bits;
        const size_t fused_blocks = (len + fused_stage_len - 1) / fused_stage_len;
        const dim3 fused_grid(fused_blocks, column_count);
        ntt_initial_stages_kernel<<<fused_grid, kThreads, 0, stream>>>(
            device_values, len, value_stride, fused_stage_bits, inverse_roots);
        status = static_cast<cudaError_t>(lzvm_cuda_check_launch());
        if (status != cudaSuccess) {
            return status;
        }
        first_stage_bits = fused_stage_bits + 1;
    }

    size_t stage_len = static_cast<size_t>(1) << first_stage_bits;
    for (size_t stage_bits = first_stage_bits; stage_len <= len;
         stage_len <<= 1, ++stage_bits) {
        const size_t pair_count = len / 2;
        const size_t stage_blocks = (pair_count + kThreads - 1) / kThreads;
        const size_t half = stage_len / 2;
        const dim3 grid(stage_blocks, column_count);
        if (!use_precomputed_factors) {
            const uint64_t stage_twiddle = host_pow_mod(root, len / stage_len);
            ntt_stage_root_column_group_kernel<<<grid, kThreads, 0, stream>>>(
                device_values, len, value_stride, stage_len, stage_twiddle);
        } else if (half > kThreads) {
            const dim3 block_stage_grid(
                stage_blocks,
                (column_count + kNttBlockStageColumnsPerThread - 1) /
                    kNttBlockStageColumnsPerThread);
            ntt_stage_block_twiddle_column_group_kernel
                <<<block_stage_grid, kThreads, 0, stream>>>(
                device_values,
                len,
                value_stride,
                column_count,
                stage_len,
                stage_bits,
                inverse_roots);
        } else {
            ntt_stage_column_group_kernel<<<grid, kThreads, 0, stream>>>(
                device_values,
                len,
                value_stride,
                stage_len,
                stage_bits,
                inverse_roots);
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

int run_coset_extend_column_group_on_device_unsynced(
    uint64_t* device_values,
    size_t source_len,
    size_t source_bits,
    size_t target_len,
    size_t target_bits,
    size_t column_count,
    uint64_t source_root_inverse,
    uint64_t target_root,
    uint64_t shift,
    cudaStream_t stream) {
    cudaError_t status = run_ntt_column_group(
        device_values,
        source_len,
        target_len,
        source_bits,
        column_count,
        source_root_inverse,
        true,
        stream);
    if (status != cudaSuccess) {
        return static_cast<int>(status);
    }

    const uint64_t inverse_len = host_pow_mod(static_cast<uint64_t>(source_len), kModulus - 2);
    const size_t blocks = (target_len + kThreads - 1) / kThreads;
    normalize_shift_and_pad_column_group_kernel<<<blocks, kThreads, 0, stream>>>(
        device_values,
        source_len,
        target_len,
        target_len,
        column_count,
        inverse_len,
        shift);
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_check_launch());

    status = run_ntt_column_group(
        device_values,
        target_len,
        target_len,
        target_bits,
        column_count,
        target_root,
        false,
        stream);
    if (status != cudaSuccess) {
        return static_cast<int>(status);
    }

    return 0;
}

int run_coset_extend_column_groups_on_device_unsynced(
    uint64_t* device_values,
    size_t source_len,
    size_t source_bits,
    size_t target_len,
    size_t target_bits,
    size_t column_count,
    uint64_t source_root_inverse,
    uint64_t target_root,
    uint64_t shift,
    cudaStream_t stream) {
    for (size_t column = 0; column < column_count; column += kNttColumnGroupSize) {
        const size_t group_count = std::min(kNttColumnGroupSize, column_count - column);
        LZVM_CUDA_RETURN_ON_ERROR(run_coset_extend_column_group_on_device_unsynced(
            device_values + column * target_len,
            source_len,
            source_bits,
            target_len,
            target_bits,
            group_count,
            source_root_inverse,
            target_root,
            shift,
            stream));
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
