#include <cuda_runtime.h>
#include <stdint.h>

namespace {

constexpr uint64_t kModulus = 0xffffffff00000001ULL;
constexpr size_t kThreads = 256;
constexpr size_t kPoseidon2Width8 = 8;
constexpr size_t kPoseidon2HalfRounds = 4;
constexpr size_t kPoseidon2PartialRounds = 22;

__device__ __constant__ uint64_t kPoseidon2Width8Diag[kPoseidon2Width8] = {
    0xa98811a1fed4e3a5ULL,
    0x1cc48b54f377e2a0ULL,
    0xe40cd4f6c5609a26ULL,
    0x11de79ebca97a4a3ULL,
    0x9177c73d8b7e929cULL,
    0x2a6fe8085797e791ULL,
    0x3de6e93329f8d5adULL,
    0x3f7af9125da962feULL,
};

__device__ __constant__ uint64_t kPoseidon2Width8RoundConstants[86] = {
    0xdd5743e7f2a5a5d9ULL,
    0xcb3a864e58ada44bULL,
    0xffa2449ed32f8cdcULL,
    0x42025f65d6bd13eeULL,
    0x7889175e25506323ULL,
    0x34b98bb03d24b737ULL,
    0xbdcc535ecc4faa2aULL,
    0x5b20ad869fc0d033ULL,
    0xf1dda5b9259dfcb4ULL,
    0x27515210be112d59ULL,
    0x4227d1718c766c3fULL,
    0x26d333161a5bd794ULL,
    0x49b938957bf4b026ULL,
    0x4a56b5938b213669ULL,
    0x1120426b48c8353dULL,
    0x6b323c3f10a56cadULL,
    0xce57d6245ddca6b2ULL,
    0xb1fc8d402bba1eb1ULL,
    0xb5c5096ca959bd04ULL,
    0x6db55cd306d31f7fULL,
    0xc49d293a81cb9641ULL,
    0x1ce55a4fe979719fULL,
    0xa92e60a9d178a4d1ULL,
    0x002cc64973bcfd8cULL,
    0xcea721cce82fb11bULL,
    0xe5b55eb8098ece81ULL,
    0x4e30525c6f1ddd66ULL,
    0x43c6702827070987ULL,
    0xaca68430a7b5762aULL,
    0x3674238634df9c93ULL,
    0x88cee1c825e33433ULL,
    0xde99ae8d74b57176ULL,
    0x488897d85ff51f56ULL,
    0x1140737ccb162218ULL,
    0xa7eeb9215866ed35ULL,
    0x9bd2976fee49fcc9ULL,
    0xc0c8f0de580a3fccULL,
    0x4fb2dae6ee8fc793ULL,
    0x343a89f35f37395bULL,
    0x223b525a77ca72c8ULL,
    0x56ccb62574aaa918ULL,
    0xc4d507d8027af9edULL,
    0xa080673cf0b7e95cULL,
    0xf0184884eb70dcf8ULL,
    0x044f10b0cb3d5c69ULL,
    0xe9e3f7993938f186ULL,
    0x1b761c80e772f459ULL,
    0x606cec607a1b5facULL,
    0x14a0c2e1d45f03cdULL,
    0x4eace8855398574fULL,
    0xf905ca7103eff3e6ULL,
    0xf8c8f8d20862c059ULL,
    0xb524fe8bdd678e5aULL,
    0xfbb7865901a1ec41ULL,
    0x014ef1197d341346ULL,
    0x9725e20825d07394ULL,
    0xfdb25aef2c5bae3bULL,
    0xbe5402dc598c971eULL,
    0x93a5711f04cdca3dULL,
    0xc45a9a5b2f8fb97bULL,
    0xfe8946a924933545ULL,
    0x2af997a27369091cULL,
    0xaa62c88e0b294011ULL,
    0x058eb9d810ce9f74ULL,
    0xb3cb23eced349ae4ULL,
    0xa3648177a77b4a84ULL,
    0x43153d905992d95dULL,
    0xf4e2a97cda44aa4bULL,
    0x5baa2702b908682fULL,
    0x082923bdf4f750d1ULL,
    0x98ae09a325893803ULL,
    0xf8a6475077968838ULL,
    0xceb0735bf00b2c5fULL,
    0x0a1a5d953888e072ULL,
    0x2fcb190489f94475ULL,
    0xb5be06270dec69fcULL,
    0x739cb934b09acf8bULL,
    0x537750b75ec7f25bULL,
    0xe9dd318bae1f3961ULL,
    0xf7462137299efe1aULL,
    0xb1f6b8eee9adb940ULL,
    0xbdebcc8a809dfe6bULL,
    0x40fc1f791b178113ULL,
    0x3ac1c3362d014864ULL,
    0x9a016184bdb8aebaULL,
    0x95f2394459fbc25eULL,
};

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

__device__ uint64_t poseidon2_pow7(uint64_t value) {
    const uint64_t square = mul_mod(value, value);
    const uint64_t fourth = mul_mod(square, square);
    return mul_mod(mul_mod(fourth, square), value);
}

__device__ void poseidon2_matmul_m4(uint64_t* values) {
    const uint64_t t0 = add_mod(values[0], values[1]);
    const uint64_t t1 = add_mod(values[2], values[3]);
    const uint64_t t2 = add_mod(add_mod(values[1], values[1]), t1);
    const uint64_t t3 = add_mod(add_mod(values[3], values[3]), t0);
    const uint64_t t1_2 = add_mod(t1, t1);
    const uint64_t t0_2 = add_mod(t0, t0);
    const uint64_t t4 = add_mod(add_mod(t1_2, t1_2), t3);
    const uint64_t t5 = add_mod(add_mod(t0_2, t0_2), t2);
    const uint64_t t6 = add_mod(t3, t5);
    const uint64_t t7 = add_mod(t2, t4);

    values[0] = t6;
    values[1] = t5;
    values[2] = t7;
    values[3] = t4;
}

__device__ void poseidon2_matmul_external_width8(uint64_t* state) {
    poseidon2_matmul_m4(&state[0]);
    poseidon2_matmul_m4(&state[4]);

    uint64_t stored[4];
    for (size_t index = 0; index < 4; ++index) {
        stored[index] = add_mod(state[index], state[index + 4]);
    }
    for (size_t index = 0; index < kPoseidon2Width8; ++index) {
        state[index] = add_mod(state[index], stored[index % 4]);
    }
}

__device__ void poseidon2_pow7add_width8(uint64_t* state, size_t offset) {
    for (size_t index = 0; index < kPoseidon2Width8; ++index) {
        state[index] = poseidon2_pow7(add_mod(state[index], kPoseidon2Width8RoundConstants[offset + index]));
    }
}

__device__ void poseidon2_hash_width8(uint64_t* state) {
    poseidon2_matmul_external_width8(state);

    for (size_t round = 0; round < kPoseidon2HalfRounds; ++round) {
        poseidon2_pow7add_width8(state, round * kPoseidon2Width8);
        poseidon2_matmul_external_width8(state);
    }

    const size_t partial_offset = kPoseidon2HalfRounds * kPoseidon2Width8;
    for (size_t round = 0; round < kPoseidon2PartialRounds; ++round) {
        state[0] =
            poseidon2_pow7(add_mod(state[0], kPoseidon2Width8RoundConstants[partial_offset + round]));
        uint64_t sum = 0;
        for (size_t index = 0; index < kPoseidon2Width8; ++index) {
            sum = add_mod(sum, state[index]);
        }
        for (size_t index = 0; index < kPoseidon2Width8; ++index) {
            state[index] = add_mod(mul_mod(state[index], kPoseidon2Width8Diag[index]), sum);
        }
    }

    const size_t final_offset = kPoseidon2HalfRounds * kPoseidon2Width8 + kPoseidon2PartialRounds;
    for (size_t round = 0; round < kPoseidon2HalfRounds; ++round) {
        poseidon2_pow7add_width8(state, final_offset + round * kPoseidon2Width8);
        poseidon2_matmul_external_width8(state);
    }
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

__global__ void poseidon2_width8_kernel(const uint64_t* values, uint64_t* out, size_t state_count) {
    const size_t state_index = blockIdx.x * blockDim.x + threadIdx.x;
    if (state_index < state_count) {
        uint64_t state[kPoseidon2Width8];
        const size_t offset = state_index * kPoseidon2Width8;
        for (size_t index = 0; index < kPoseidon2Width8; ++index) {
            state[index] = values[offset + index];
        }
        poseidon2_hash_width8(state);
        for (size_t index = 0; index < kPoseidon2Width8; ++index) {
            out[offset + index] = state[index];
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

extern "C" int lzvm_cuda_poseidon2_width8(
    const uint64_t* values,
    uint64_t* out,
    size_t state_count) {
    if (state_count == 0) {
        return 0;
    }
    if (values == nullptr || out == nullptr) {
        return -1;
    }

    uint64_t* device_values = nullptr;
    uint64_t* device_out = nullptr;
    const size_t word_count = state_count * kPoseidon2Width8;
    const size_t bytes = word_count * sizeof(uint64_t);
    cudaError_t status = cudaMalloc(&device_values, bytes);
    if (status != cudaSuccess) {
        return static_cast<int>(status);
    }
    status = cudaMalloc(&device_out, bytes);
    if (status != cudaSuccess) {
        return free_after_error(status, device_values, device_out, nullptr);
    }

    status = cudaMemcpy(device_values, values, bytes, cudaMemcpyHostToDevice);
    if (status != cudaSuccess) {
        return free_after_error(status, device_values, device_out, nullptr);
    }

    const size_t blocks = (state_count + kThreads - 1) / kThreads;
    poseidon2_width8_kernel<<<blocks, kThreads>>>(device_values, device_out, state_count);
    status = cudaGetLastError();
    if (status != cudaSuccess) {
        return free_after_error(status, device_values, device_out, nullptr);
    }
    status = cudaDeviceSynchronize();
    if (status != cudaSuccess) {
        return free_after_error(status, device_values, device_out, nullptr);
    }
    status = cudaMemcpy(out, device_out, bytes, cudaMemcpyDeviceToHost);
    if (status != cudaSuccess) {
        return free_after_error(status, device_values, device_out, nullptr);
    }

    cudaFree(device_values);
    cudaFree(device_out);
    return 0;
}
