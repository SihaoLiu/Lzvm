#include <cuda_runtime.h>
#include <stdint.h>

namespace {

constexpr uint64_t kModulus = 0xffffffff00000001ULL;
constexpr size_t kThreads = 256;

__device__ uint64_t add_mod(uint64_t lhs, uint64_t rhs) {
    const uint64_t threshold = kModulus - rhs;
    return lhs >= threshold ? lhs - threshold : lhs + rhs;
}

uint64_t host_mul_mod(uint64_t lhs, uint64_t rhs) {
    const unsigned __int128 product =
        static_cast<unsigned __int128>(lhs) * static_cast<unsigned __int128>(rhs);
    return static_cast<uint64_t>(product % kModulus);
}

uint64_t host_pow_mod(uint64_t base, uint64_t exponent) {
    uint64_t result = 1;
    while (exponent > 0) {
        if ((exponent & 1) != 0) {
            result = host_mul_mod(result, base);
        }
        base = host_mul_mod(base, base);
        exponent >>= 1;
    }
    return result;
}

__device__ uint64_t mul_mod(uint64_t lhs, uint64_t rhs) {
    const unsigned __int128 product =
        static_cast<unsigned __int128>(lhs) * static_cast<unsigned __int128>(rhs);
    return static_cast<uint64_t>(product % kModulus);
}

__device__ uint64_t pow_mod(uint64_t base, size_t exponent) {
    uint64_t result = 1;
    while (exponent > 0) {
        if ((exponent & 1) != 0) {
            result = mul_mod(result, base);
        }
        base = mul_mod(base, base);
        exponent >>= 1;
    }
    return result;
}

__device__ uint64_t sub_mod(uint64_t lhs, uint64_t rhs) {
    return lhs >= rhs ? lhs - rhs : kModulus - (rhs - lhs);
}

__device__ size_t reverse_bits(size_t value, size_t bits) {
    size_t out = 0;
    for (size_t index = 0; index < bits; ++index) {
        out = (out << 1) | (value & 1);
        value >>= 1;
    }
    return out;
}

__global__ void add_kernel(const uint64_t* lhs, const uint64_t* rhs, uint64_t* out, size_t len) {
    const size_t index = blockIdx.x * blockDim.x + threadIdx.x;
    if (index < len) {
        out[index] = add_mod(lhs[index], rhs[index]);
    }
}

__global__ void mul_kernel(const uint64_t* lhs, const uint64_t* rhs, uint64_t* out, size_t len) {
    const size_t index = blockIdx.x * blockDim.x + threadIdx.x;
    if (index < len) {
        out[index] = mul_mod(lhs[index], rhs[index]);
    }
}

__global__ void butterfly_kernel(
    const uint64_t* even,
    const uint64_t* odd,
    const uint64_t* twiddle,
    uint64_t* out_even,
    uint64_t* out_odd,
    size_t len) {
    const size_t index = blockIdx.x * blockDim.x + threadIdx.x;
    if (index < len) {
        const uint64_t scaled = mul_mod(odd[index], twiddle[index]);
        out_even[index] = add_mod(even[index], scaled);
        out_odd[index] = sub_mod(even[index], scaled);
    }
}

__global__ void bit_reverse_kernel(uint64_t* values, size_t len, size_t bits) {
    const size_t index = blockIdx.x * blockDim.x + threadIdx.x;
    if (index < len) {
        const size_t reverse = reverse_bits(index, bits);
        if (index < reverse) {
            const uint64_t tmp = values[index];
            values[index] = values[reverse];
            values[reverse] = tmp;
        }
    }
}

__global__ void ntt_stage_kernel(uint64_t* values, size_t len, size_t stage_len, uint64_t root) {
    const size_t pair_count = len / 2;
    const size_t pair = blockIdx.x * blockDim.x + threadIdx.x;
    if (pair < pair_count) {
        const size_t half = stage_len / 2;
        const size_t group = pair / half;
        const size_t offset = pair % half;
        const size_t even_index = group * stage_len + offset;
        const size_t odd_index = even_index + half;
        const uint64_t stage_twiddle = pow_mod(root, len / stage_len);
        const uint64_t factor = pow_mod(stage_twiddle, offset);
        const uint64_t even = values[even_index];
        const uint64_t odd = mul_mod(values[odd_index], factor);
        values[even_index] = add_mod(even, odd);
        values[odd_index] = sub_mod(even, odd);
    }
}

__global__ void normalize_shift_and_pad_kernel(
    uint64_t* values,
    size_t source_len,
    size_t target_len,
    uint64_t inverse_len,
    uint64_t shift) {
    const size_t index = blockIdx.x * blockDim.x + threadIdx.x;
    if (index < target_len) {
        if (index < source_len) {
            values[index] = mul_mod(mul_mod(values[index], inverse_len), pow_mod(shift, index));
        } else {
            values[index] = 0;
        }
    }
}

cudaError_t run_ntt(uint64_t* device_values, size_t len, size_t bits, uint64_t root) {
    const size_t blocks = (len + kThreads - 1) / kThreads;
    bit_reverse_kernel<<<blocks, kThreads>>>(device_values, len, bits);
    cudaError_t status = cudaGetLastError();
    if (status != cudaSuccess) {
        return status;
    }

    for (size_t stage_len = 2; stage_len <= len; stage_len <<= 1) {
        const size_t pair_count = len / 2;
        const size_t stage_blocks = (pair_count + kThreads - 1) / kThreads;
        ntt_stage_kernel<<<stage_blocks, kThreads>>>(device_values, len, stage_len, root);
        status = cudaGetLastError();
        if (status != cudaSuccess) {
            return status;
        }
    }
    return cudaSuccess;
}

int free_after_error(cudaError_t status, uint64_t* lhs, uint64_t* rhs, uint64_t* out) {
    cudaFree(lhs);
    cudaFree(rhs);
    cudaFree(out);
    return static_cast<int>(status);
}

int free_after_butterfly_error(
    cudaError_t status,
    uint64_t* even,
    uint64_t* odd,
    uint64_t* twiddle,
    uint64_t* out_even,
    uint64_t* out_odd) {
    cudaFree(even);
    cudaFree(odd);
    cudaFree(twiddle);
    cudaFree(out_even);
    cudaFree(out_odd);
    return static_cast<int>(status);
}

int free_single_after_error(cudaError_t status, uint64_t* values) {
    cudaFree(values);
    return static_cast<int>(status);
}

}  // namespace

extern "C" int lzvm_cuda_goldilocks_add(
    const uint64_t* lhs,
    const uint64_t* rhs,
    uint64_t* out,
    size_t len) {
    if (len == 0) {
        return 0;
    }
    if (lhs == nullptr || rhs == nullptr || out == nullptr) {
        return -1;
    }

    uint64_t* device_lhs = nullptr;
    uint64_t* device_rhs = nullptr;
    uint64_t* device_out = nullptr;
    const size_t bytes = len * sizeof(uint64_t);

    cudaError_t status = cudaMalloc(&device_lhs, bytes);
    if (status != cudaSuccess) {
        return static_cast<int>(status);
    }
    status = cudaMalloc(&device_rhs, bytes);
    if (status != cudaSuccess) {
        return free_after_error(status, device_lhs, device_rhs, device_out);
    }
    status = cudaMalloc(&device_out, bytes);
    if (status != cudaSuccess) {
        return free_after_error(status, device_lhs, device_rhs, device_out);
    }

    status = cudaMemcpy(device_lhs, lhs, bytes, cudaMemcpyHostToDevice);
    if (status != cudaSuccess) {
        return free_after_error(status, device_lhs, device_rhs, device_out);
    }
    status = cudaMemcpy(device_rhs, rhs, bytes, cudaMemcpyHostToDevice);
    if (status != cudaSuccess) {
        return free_after_error(status, device_lhs, device_rhs, device_out);
    }

    const size_t blocks = (len + kThreads - 1) / kThreads;
    add_kernel<<<blocks, kThreads>>>(device_lhs, device_rhs, device_out, len);
    status = cudaGetLastError();
    if (status != cudaSuccess) {
        return free_after_error(status, device_lhs, device_rhs, device_out);
    }
    status = cudaDeviceSynchronize();
    if (status != cudaSuccess) {
        return free_after_error(status, device_lhs, device_rhs, device_out);
    }
    status = cudaMemcpy(out, device_out, bytes, cudaMemcpyDeviceToHost);
    if (status != cudaSuccess) {
        return free_after_error(status, device_lhs, device_rhs, device_out);
    }

    cudaFree(device_lhs);
    cudaFree(device_rhs);
    cudaFree(device_out);
    return 0;
}

extern "C" int lzvm_cuda_goldilocks_butterfly(
    const uint64_t* even,
    const uint64_t* odd,
    const uint64_t* twiddle,
    uint64_t* out_even,
    uint64_t* out_odd,
    size_t len) {
    if (len == 0) {
        return 0;
    }
    if (even == nullptr || odd == nullptr || twiddle == nullptr || out_even == nullptr ||
        out_odd == nullptr) {
        return -1;
    }

    uint64_t* device_even = nullptr;
    uint64_t* device_odd = nullptr;
    uint64_t* device_twiddle = nullptr;
    uint64_t* device_out_even = nullptr;
    uint64_t* device_out_odd = nullptr;
    const size_t bytes = len * sizeof(uint64_t);

    cudaError_t status = cudaMalloc(&device_even, bytes);
    if (status != cudaSuccess) {
        return static_cast<int>(status);
    }
    status = cudaMalloc(&device_odd, bytes);
    if (status != cudaSuccess) {
        return free_after_butterfly_error(
            status, device_even, device_odd, device_twiddle, device_out_even, device_out_odd);
    }
    status = cudaMalloc(&device_twiddle, bytes);
    if (status != cudaSuccess) {
        return free_after_butterfly_error(
            status, device_even, device_odd, device_twiddle, device_out_even, device_out_odd);
    }
    status = cudaMalloc(&device_out_even, bytes);
    if (status != cudaSuccess) {
        return free_after_butterfly_error(
            status, device_even, device_odd, device_twiddle, device_out_even, device_out_odd);
    }
    status = cudaMalloc(&device_out_odd, bytes);
    if (status != cudaSuccess) {
        return free_after_butterfly_error(
            status, device_even, device_odd, device_twiddle, device_out_even, device_out_odd);
    }

    status = cudaMemcpy(device_even, even, bytes, cudaMemcpyHostToDevice);
    if (status != cudaSuccess) {
        return free_after_butterfly_error(
            status, device_even, device_odd, device_twiddle, device_out_even, device_out_odd);
    }
    status = cudaMemcpy(device_odd, odd, bytes, cudaMemcpyHostToDevice);
    if (status != cudaSuccess) {
        return free_after_butterfly_error(
            status, device_even, device_odd, device_twiddle, device_out_even, device_out_odd);
    }
    status = cudaMemcpy(device_twiddle, twiddle, bytes, cudaMemcpyHostToDevice);
    if (status != cudaSuccess) {
        return free_after_butterfly_error(
            status, device_even, device_odd, device_twiddle, device_out_even, device_out_odd);
    }

    const size_t blocks = (len + kThreads - 1) / kThreads;
    butterfly_kernel<<<blocks, kThreads>>>(
        device_even, device_odd, device_twiddle, device_out_even, device_out_odd, len);
    status = cudaGetLastError();
    if (status != cudaSuccess) {
        return free_after_butterfly_error(
            status, device_even, device_odd, device_twiddle, device_out_even, device_out_odd);
    }
    status = cudaDeviceSynchronize();
    if (status != cudaSuccess) {
        return free_after_butterfly_error(
            status, device_even, device_odd, device_twiddle, device_out_even, device_out_odd);
    }
    status = cudaMemcpy(out_even, device_out_even, bytes, cudaMemcpyDeviceToHost);
    if (status != cudaSuccess) {
        return free_after_butterfly_error(
            status, device_even, device_odd, device_twiddle, device_out_even, device_out_odd);
    }
    status = cudaMemcpy(out_odd, device_out_odd, bytes, cudaMemcpyDeviceToHost);
    if (status != cudaSuccess) {
        return free_after_butterfly_error(
            status, device_even, device_odd, device_twiddle, device_out_even, device_out_odd);
    }

    cudaFree(device_even);
    cudaFree(device_odd);
    cudaFree(device_twiddle);
    cudaFree(device_out_even);
    cudaFree(device_out_odd);
    return 0;
}

extern "C" int lzvm_cuda_goldilocks_mul(
    const uint64_t* lhs,
    const uint64_t* rhs,
    uint64_t* out,
    size_t len) {
    if (len == 0) {
        return 0;
    }
    if (lhs == nullptr || rhs == nullptr || out == nullptr) {
        return -1;
    }

    uint64_t* device_lhs = nullptr;
    uint64_t* device_rhs = nullptr;
    uint64_t* device_out = nullptr;
    const size_t bytes = len * sizeof(uint64_t);

    cudaError_t status = cudaMalloc(&device_lhs, bytes);
    if (status != cudaSuccess) {
        return static_cast<int>(status);
    }
    status = cudaMalloc(&device_rhs, bytes);
    if (status != cudaSuccess) {
        return free_after_error(status, device_lhs, device_rhs, device_out);
    }
    status = cudaMalloc(&device_out, bytes);
    if (status != cudaSuccess) {
        return free_after_error(status, device_lhs, device_rhs, device_out);
    }

    status = cudaMemcpy(device_lhs, lhs, bytes, cudaMemcpyHostToDevice);
    if (status != cudaSuccess) {
        return free_after_error(status, device_lhs, device_rhs, device_out);
    }
    status = cudaMemcpy(device_rhs, rhs, bytes, cudaMemcpyHostToDevice);
    if (status != cudaSuccess) {
        return free_after_error(status, device_lhs, device_rhs, device_out);
    }

    const size_t blocks = (len + kThreads - 1) / kThreads;
    mul_kernel<<<blocks, kThreads>>>(device_lhs, device_rhs, device_out, len);
    status = cudaGetLastError();
    if (status != cudaSuccess) {
        return free_after_error(status, device_lhs, device_rhs, device_out);
    }
    status = cudaDeviceSynchronize();
    if (status != cudaSuccess) {
        return free_after_error(status, device_lhs, device_rhs, device_out);
    }
    status = cudaMemcpy(out, device_out, bytes, cudaMemcpyDeviceToHost);
    if (status != cudaSuccess) {
        return free_after_error(status, device_lhs, device_rhs, device_out);
    }

    cudaFree(device_lhs);
    cudaFree(device_rhs);
    cudaFree(device_out);
    return 0;
}

extern "C" int lzvm_cuda_goldilocks_ntt(
    const uint64_t* values,
    uint64_t* out,
    size_t len,
    size_t bits,
    uint64_t root) {
    if (values == nullptr || out == nullptr) {
        return -1;
    }
    if (len == 0) {
        return -2;
    }

    uint64_t* device_values = nullptr;
    const size_t bytes = len * sizeof(uint64_t);
    cudaError_t status = cudaMalloc(&device_values, bytes);
    if (status != cudaSuccess) {
        return static_cast<int>(status);
    }

    status = cudaMemcpy(device_values, values, bytes, cudaMemcpyHostToDevice);
    if (status != cudaSuccess) {
        return free_single_after_error(status, device_values);
    }

    status = run_ntt(device_values, len, bits, root);
    if (status != cudaSuccess) {
        return free_single_after_error(status, device_values);
    }

    status = cudaDeviceSynchronize();
    if (status != cudaSuccess) {
        return free_single_after_error(status, device_values);
    }
    status = cudaMemcpy(out, device_values, bytes, cudaMemcpyDeviceToHost);
    if (status != cudaSuccess) {
        return free_single_after_error(status, device_values);
    }

    cudaFree(device_values);
    return 0;
}

extern "C" int lzvm_cuda_goldilocks_coset_extend(
    const uint64_t* values,
    uint64_t* out,
    size_t source_len,
    size_t source_bits,
    size_t target_len,
    size_t target_bits,
    uint64_t source_root_inverse,
    uint64_t target_root,
    uint64_t shift) {
    if (values == nullptr || out == nullptr) {
        return -1;
    }
    if (source_len == 0 || target_len == 0 || source_len > target_len) {
        return -2;
    }

    uint64_t* device_values = nullptr;
    const size_t source_bytes = source_len * sizeof(uint64_t);
    const size_t target_bytes = target_len * sizeof(uint64_t);
    cudaError_t status = cudaMalloc(&device_values, target_bytes);
    if (status != cudaSuccess) {
        return static_cast<int>(status);
    }

    status = cudaMemcpy(device_values, values, source_bytes, cudaMemcpyHostToDevice);
    if (status != cudaSuccess) {
        return free_single_after_error(status, device_values);
    }

    status = run_ntt(device_values, source_len, source_bits, source_root_inverse);
    if (status != cudaSuccess) {
        return free_single_after_error(status, device_values);
    }

    const uint64_t inverse_len = host_pow_mod(static_cast<uint64_t>(source_len), kModulus - 2);
    const size_t blocks = (target_len + kThreads - 1) / kThreads;
    normalize_shift_and_pad_kernel<<<blocks, kThreads>>>(
        device_values, source_len, target_len, inverse_len, shift);
    status = cudaGetLastError();
    if (status != cudaSuccess) {
        return free_single_after_error(status, device_values);
    }

    status = run_ntt(device_values, target_len, target_bits, target_root);
    if (status != cudaSuccess) {
        return free_single_after_error(status, device_values);
    }

    status = cudaDeviceSynchronize();
    if (status != cudaSuccess) {
        return free_single_after_error(status, device_values);
    }
    status = cudaMemcpy(out, device_values, target_bytes, cudaMemcpyDeviceToHost);
    if (status != cudaSuccess) {
        return free_single_after_error(status, device_values);
    }

    cudaFree(device_values);
    return 0;
}
