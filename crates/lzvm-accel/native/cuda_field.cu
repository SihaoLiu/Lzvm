#include <cuda_runtime.h>
#include <stdint.h>

namespace {

constexpr uint64_t kModulus = 0xffffffff00000001ULL;

__device__ uint64_t add_mod(uint64_t lhs, uint64_t rhs) {
    const uint64_t threshold = kModulus - rhs;
    return lhs >= threshold ? lhs - threshold : lhs + rhs;
}

__device__ uint64_t mul_mod(uint64_t lhs, uint64_t rhs) {
    const unsigned __int128 product =
        static_cast<unsigned __int128>(lhs) * static_cast<unsigned __int128>(rhs);
    return static_cast<uint64_t>(product % kModulus);
}

__device__ uint64_t sub_mod(uint64_t lhs, uint64_t rhs) {
    return lhs >= rhs ? lhs - rhs : kModulus - (rhs - lhs);
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

    const size_t threads = 256;
    const size_t blocks = (len + threads - 1) / threads;
    add_kernel<<<blocks, threads>>>(device_lhs, device_rhs, device_out, len);
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

    const size_t threads = 256;
    const size_t blocks = (len + threads - 1) / threads;
    butterfly_kernel<<<blocks, threads>>>(
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

    const size_t threads = 256;
    const size_t blocks = (len + threads - 1) / threads;
    mul_kernel<<<blocks, threads>>>(device_lhs, device_rhs, device_out, len);
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
