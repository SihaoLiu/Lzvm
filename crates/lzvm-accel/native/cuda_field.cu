#include <cuda_runtime.h>
#include <stdint.h>

#include "cuda_host.hpp"

#define LZVM_CUDA_RETURN_ON_ERROR(expr) \
    do { \
        const int status__ = (expr); \
        if (status__ != 0) { \
            return status__; \
        } \
    } while (0)

namespace {

constexpr uint64_t kModulus = 0xffffffff00000001ULL;
constexpr size_t kThreads = 256;
constexpr size_t kPoseidon2Width4 = 4;
constexpr size_t kPoseidon2Width8 = 8;
constexpr size_t kPoseidon2Width16 = 16;
constexpr size_t kPoseidon2HalfRounds = 4;
constexpr size_t kPoseidon2Width4PartialRounds = 21;
constexpr size_t kPoseidon2PartialRounds = 22;
constexpr size_t kKeccakRateBytes = 136;
constexpr size_t kKeccakStateLanes = 25;
constexpr size_t kKeccakRateLanes = 17;
constexpr size_t kKeccakOutputBytes = 32;
constexpr size_t kMaxRootBits = 32;

__device__ __constant__ uint64_t kNttStageRoots[kMaxRootBits + 1];
__device__ __constant__ uint64_t kNttStageRootInverses[kMaxRootBits + 1];
__device__ __constant__ unsigned int kNttStageRootLimit;

#include "cuda_field_constants.cuh"


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

__device__ void poseidon2_matmul_external_width4(uint64_t* state) {
    poseidon2_matmul_m4(state);
}

__device__ void poseidon2_pow7add_width4(uint64_t* state, size_t offset) {
    for (size_t index = 0; index < kPoseidon2Width4; ++index) {
        state[index] =
            poseidon2_pow7(add_mod(state[index], kPoseidon2Width4RoundConstants[offset + index]));
    }
}

__device__ void poseidon2_hash_width4(uint64_t* state) {
    poseidon2_matmul_external_width4(state);

    for (size_t round = 0; round < kPoseidon2HalfRounds; ++round) {
        poseidon2_pow7add_width4(state, round * kPoseidon2Width4);
        poseidon2_matmul_external_width4(state);
    }

    const size_t partial_offset = kPoseidon2HalfRounds * kPoseidon2Width4;
    for (size_t round = 0; round < kPoseidon2Width4PartialRounds; ++round) {
        state[0] =
            poseidon2_pow7(add_mod(state[0], kPoseidon2Width4RoundConstants[partial_offset + round]));
        uint64_t sum = 0;
        for (size_t index = 0; index < kPoseidon2Width4; ++index) {
            sum = add_mod(sum, state[index]);
        }
        for (size_t index = 0; index < kPoseidon2Width4; ++index) {
            state[index] = add_mod(mul_mod(state[index], kPoseidon2Width4Diag[index]), sum);
        }
    }

    const size_t final_offset =
        kPoseidon2HalfRounds * kPoseidon2Width4 + kPoseidon2Width4PartialRounds;
    for (size_t round = 0; round < kPoseidon2HalfRounds; ++round) {
        poseidon2_pow7add_width4(state, final_offset + round * kPoseidon2Width4);
        poseidon2_matmul_external_width4(state);
    }
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

__device__ void poseidon2_matmul_external_width16(uint64_t* state) {
    poseidon2_matmul_m4(&state[0]);
    poseidon2_matmul_m4(&state[4]);
    poseidon2_matmul_m4(&state[8]);
    poseidon2_matmul_m4(&state[12]);

    uint64_t stored[4] = {0, 0, 0, 0};
    for (size_t chunk = 0; chunk < kPoseidon2Width16; chunk += 4) {
        for (size_t index = 0; index < 4; ++index) {
            stored[index] = add_mod(stored[index], state[chunk + index]);
        }
    }
    for (size_t index = 0; index < kPoseidon2Width16; ++index) {
        state[index] = add_mod(state[index], stored[index % 4]);
    }
}

__device__ void poseidon2_pow7add_width16(uint64_t* state, size_t offset) {
    for (size_t index = 0; index < kPoseidon2Width16; ++index) {
        state[index] =
            poseidon2_pow7(add_mod(state[index], kPoseidon2Width16RoundConstants[offset + index]));
    }
}

__device__ void poseidon2_hash_width16(uint64_t* state) {
    poseidon2_matmul_external_width16(state);

    for (size_t round = 0; round < kPoseidon2HalfRounds; ++round) {
        poseidon2_pow7add_width16(state, round * kPoseidon2Width16);
        poseidon2_matmul_external_width16(state);
    }

    const size_t partial_offset = kPoseidon2HalfRounds * kPoseidon2Width16;
    for (size_t round = 0; round < kPoseidon2PartialRounds; ++round) {
        state[0] = poseidon2_pow7(
            add_mod(state[0], kPoseidon2Width16RoundConstants[partial_offset + round]));
        uint64_t sum = 0;
        for (size_t index = 0; index < kPoseidon2Width16; ++index) {
            sum = add_mod(sum, state[index]);
        }
        for (size_t index = 0; index < kPoseidon2Width16; ++index) {
            state[index] = add_mod(mul_mod(state[index], kPoseidon2Width16Diag[index]), sum);
        }
    }

    const size_t final_offset = kPoseidon2HalfRounds * kPoseidon2Width16 + kPoseidon2PartialRounds;
    for (size_t round = 0; round < kPoseidon2HalfRounds; ++round) {
        poseidon2_pow7add_width16(state, final_offset + round * kPoseidon2Width16);
        poseidon2_matmul_external_width16(state);
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
            stage_bits <= static_cast<size_t>(kNttStageRootLimit)
            ? (inverse_roots ? kNttStageRootInverses[stage_bits] : kNttStageRoots[stage_bits])
            : pow_mod(root, len / stage_len);
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

__global__ void pack_row_major_columns_kernel(const uint64_t* values, uint64_t* columns,
                                              size_t source_len, size_t target_len,
                                              size_t column_count) {
    const size_t index = blockIdx.x * blockDim.x + threadIdx.x;
    const size_t total = source_len * column_count;
    if (index < total) {
        const size_t row = index / column_count;
        const size_t column = index % column_count;
        columns[column * target_len + row] = values[index];
    }
}

__global__ void unpack_row_major_columns_kernel(const uint64_t* columns, uint64_t* out,
                                                size_t target_len, size_t column_count) {
    const size_t index = blockIdx.x * blockDim.x + threadIdx.x;
    const size_t total = target_len * column_count;
    if (index < total) {
        const size_t row = index / column_count;
        const size_t column = index % column_count;
        out[index] = columns[column * target_len + row];
    }
}

__global__ void scale_kernel(uint64_t* values, size_t len, uint64_t factor) {
    const size_t index = blockIdx.x * blockDim.x + threadIdx.x;
    if (index < len) {
        values[index] = mul_mod(values[index], factor);
    }
}

__global__ void poseidon2_width4_kernel(const uint64_t* values, uint64_t* out, size_t state_count) {
    const size_t state_index = blockIdx.x * blockDim.x + threadIdx.x;
    if (state_index < state_count) {
        uint64_t state[kPoseidon2Width4];
        const size_t offset = state_index * kPoseidon2Width4;
        for (size_t index = 0; index < kPoseidon2Width4; ++index) {
            state[index] = values[offset + index];
        }
        poseidon2_hash_width4(state);
        for (size_t index = 0; index < kPoseidon2Width4; ++index) {
            out[offset + index] = state[index];
        }
    }
}

__global__ void poseidon2_width4_find_nonce_kernel(
    const uint64_t* challenge,
    uint64_t start,
    size_t count,
    uint64_t target,
    uint64_t* out,
    unsigned int* found) {
    const size_t index = blockIdx.x * blockDim.x + threadIdx.x;
    if (index < count) {
        const uint64_t candidate = start + index;
        uint64_t state[kPoseidon2Width4] = {
            challenge[0],
            challenge[1],
            challenge[2],
            candidate,
        };
        poseidon2_hash_width4(state);
        if (state[0] < target) {
            atomicMin(reinterpret_cast<unsigned long long*>(out), static_cast<unsigned long long>(candidate));
            atomicExch(found, 1U);
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

__global__ void poseidon2_width16_kernel(const uint64_t* values, uint64_t* out, size_t state_count) {
    const size_t state_index = blockIdx.x * blockDim.x + threadIdx.x;
    if (state_index < state_count) {
        uint64_t state[kPoseidon2Width16];
        const size_t offset = state_index * kPoseidon2Width16;
        for (size_t index = 0; index < kPoseidon2Width16; ++index) {
            state[index] = values[offset + index];
        }
        poseidon2_hash_width16(state);
        for (size_t index = 0; index < kPoseidon2Width16; ++index) {
            out[offset + index] = state[index];
        }
    }
}

__global__ void pack_poseidon2_width8_merkle_parent_inputs_kernel(
    const uint64_t* current_states,
    uint64_t* packed,
    size_t child_state_count) {
    const size_t parent_index = blockIdx.x * blockDim.x + threadIdx.x;
    const size_t parent_state_count = (child_state_count + 1) / 2;
    if (parent_index < parent_state_count) {
        const size_t out_offset = parent_index * kPoseidon2Width8;
        const size_t first_child = parent_index * 2;
        for (size_t slot = 0; slot < 2; ++slot) {
            const size_t child_index = first_child + slot;
            if (child_index < child_state_count) {
                const size_t child_offset = child_index * kPoseidon2Width8;
                const size_t slot_offset = out_offset + slot * 4;
                for (size_t word = 0; word < 4; ++word) {
                    packed[slot_offset + word] = current_states[child_offset + word];
                }
            }
        }
    }
}

__global__ void pack_poseidon2_width16_merkle_parent_inputs_kernel(
    const uint64_t* current_states,
    uint64_t* packed,
    size_t child_state_count) {
    const size_t parent_index = blockIdx.x * blockDim.x + threadIdx.x;
    const size_t parent_state_count = (child_state_count + 3) / 4;
    if (parent_index < parent_state_count) {
        const size_t out_offset = parent_index * kPoseidon2Width16;
        const size_t first_child = parent_index * 4;
        for (size_t slot = 0; slot < 4; ++slot) {
            const size_t child_index = first_child + slot;
            if (child_index < child_state_count) {
                const size_t child_offset = child_index * kPoseidon2Width16;
                const size_t slot_offset = out_offset + slot * 4;
                for (size_t word = 0; word < 4; ++word) {
                    packed[slot_offset + word] = current_states[child_offset + word];
                }
            }
        }
    }
}

__global__ void pack_poseidon2_width8_linear_round_inputs_kernel(
    const uint64_t* current_states,
    const uint64_t* row_values,
    uint64_t* packed,
    size_t row_count,
    size_t chunk_len) {
    const size_t row_index = blockIdx.x * blockDim.x + threadIdx.x;
    if (row_index < row_count) {
        const size_t state_offset = row_index * kPoseidon2Width8;
        const size_t row_offset = row_index * chunk_len;
        for (size_t word = 0; word < chunk_len; ++word) {
            packed[state_offset + word] = row_values[row_offset + word];
        }
        for (size_t word = 0; word < kPoseidon2HalfRounds; ++word) {
            packed[state_offset + kPoseidon2HalfRounds + word] =
                current_states[state_offset + word];
        }
    }
}

__global__ void pack_poseidon2_width16_linear_round_inputs_kernel(
    const uint64_t* current_states,
    const uint64_t* row_values,
    uint64_t* packed,
    size_t row_count,
    size_t chunk_len) {
    const size_t row_index = blockIdx.x * blockDim.x + threadIdx.x;
    if (row_index < row_count) {
        const size_t state_offset = row_index * kPoseidon2Width16;
        const size_t row_offset = row_index * chunk_len;
        for (size_t word = 0; word < chunk_len; ++word) {
            packed[state_offset + word] = row_values[row_offset + word];
        }
        for (size_t word = 0; word < kPoseidon2HalfRounds; ++word) {
            packed[state_offset + kPoseidon2Width16 - kPoseidon2HalfRounds + word] =
                current_states[state_offset + word];
        }
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
        ntt_stage_kernel<<<stage_blocks, kThreads>>>(
            device_values, len, stage_len, stage_bits, root, inverse_roots);
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

int run_poseidon2_width4_on_device(
    const uint64_t* device_values,
    uint64_t* device_out,
    size_t state_count) {
    if (state_count == 0) {
        return 0;
    }
    if (device_values == nullptr || device_out == nullptr) {
        return -1;
    }

    const size_t blocks = (state_count + kThreads - 1) / kThreads;
    poseidon2_width4_kernel<<<blocks, kThreads>>>(device_values, device_out, state_count);
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_check_launch());
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_synchronize());
    return 0;
}

int run_poseidon2_width8_on_device(
    const uint64_t* device_values,
    uint64_t* device_out,
    size_t state_count) {
    if (state_count == 0) {
        return 0;
    }
    if (device_values == nullptr || device_out == nullptr) {
        return -1;
    }

    const size_t blocks = (state_count + kThreads - 1) / kThreads;
    poseidon2_width8_kernel<<<blocks, kThreads>>>(device_values, device_out, state_count);
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_check_launch());
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_synchronize());
    return 0;
}

int run_poseidon2_width16_on_device(
    const uint64_t* device_values,
    uint64_t* device_out,
    size_t state_count) {
    if (state_count == 0) {
        return 0;
    }
    if (device_values == nullptr || device_out == nullptr) {
        return -1;
    }

    const size_t blocks = (state_count + kThreads - 1) / kThreads;
    poseidon2_width16_kernel<<<blocks, kThreads>>>(device_values, device_out, state_count);
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_check_launch());
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_synchronize());
    return 0;
}

int run_poseidon2_width8_merkle_parent_on_device(
    const uint64_t* device_values,
    uint64_t* device_out,
    size_t child_state_count) {
    if (child_state_count == 0) {
        return 0;
    }
    if (device_values == nullptr || device_out == nullptr) {
        return -1;
    }

    const size_t parent_state_count = (child_state_count + 1) / 2;
    DeviceBuffer<uint64_t> device_packed;

    LZVM_CUDA_RETURN_ON_ERROR(device_packed.reset(parent_state_count * kPoseidon2Width8));
    LZVM_CUDA_RETURN_ON_ERROR(cudaMemset(
        device_packed.data(), 0, parent_state_count * kPoseidon2Width8 * sizeof(uint64_t)));

    const size_t blocks = (parent_state_count + kThreads - 1) / kThreads;
    pack_poseidon2_width8_merkle_parent_inputs_kernel<<<blocks, kThreads>>>(
        device_values, device_packed.data(), child_state_count);
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_check_launch());
    LZVM_CUDA_RETURN_ON_ERROR(
        run_poseidon2_width8_on_device(device_packed.data(), device_out, parent_state_count));
    return 0;
}

int run_poseidon2_width16_merkle_parent_on_device(
    const uint64_t* device_values,
    uint64_t* device_out,
    size_t child_state_count) {
    if (child_state_count == 0) {
        return 0;
    }
    if (device_values == nullptr || device_out == nullptr) {
        return -1;
    }

    const size_t parent_state_count = (child_state_count + 3) / 4;
    DeviceBuffer<uint64_t> device_packed;

    LZVM_CUDA_RETURN_ON_ERROR(device_packed.reset(parent_state_count * kPoseidon2Width16));
    LZVM_CUDA_RETURN_ON_ERROR(cudaMemset(
        device_packed.data(), 0, parent_state_count * kPoseidon2Width16 * sizeof(uint64_t)));

    const size_t blocks = (parent_state_count + kThreads - 1) / kThreads;
    pack_poseidon2_width16_merkle_parent_inputs_kernel<<<blocks, kThreads>>>(
        device_values, device_packed.data(), child_state_count);
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_check_launch());
    LZVM_CUDA_RETURN_ON_ERROR(
        run_poseidon2_width16_on_device(device_packed.data(), device_out, parent_state_count));
    return 0;
}

int run_poseidon2_width8_linear_round_on_device(
    const uint64_t* current_states,
    const uint64_t* row_values,
    uint64_t* device_out,
    size_t row_count,
    size_t chunk_len) {
    if (row_count == 0) {
        return 0;
    }
    if (current_states == nullptr || row_values == nullptr || device_out == nullptr) {
        return -1;
    }
    if (chunk_len == 0 || chunk_len > kPoseidon2HalfRounds) {
        return -2;
    }

    const size_t state_bytes = row_count * kPoseidon2Width8 * sizeof(uint64_t);
    DeviceBuffer<uint64_t> device_packed;

    LZVM_CUDA_RETURN_ON_ERROR(device_packed.reset(row_count * kPoseidon2Width8));
    LZVM_CUDA_RETURN_ON_ERROR(cudaMemset(device_packed.data(), 0, state_bytes));

    const size_t blocks = (row_count + kThreads - 1) / kThreads;
    pack_poseidon2_width8_linear_round_inputs_kernel<<<blocks, kThreads>>>(
        current_states, row_values, device_packed.data(), row_count, chunk_len);
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_check_launch());
    return run_poseidon2_width8_on_device(device_packed.data(), device_out, row_count);
}

int run_poseidon2_width16_linear_round_on_device(
    const uint64_t* current_states,
    const uint64_t* row_values,
    uint64_t* device_out,
    size_t row_count,
    size_t chunk_len) {
    if (row_count == 0) {
        return 0;
    }
    if (current_states == nullptr || row_values == nullptr || device_out == nullptr) {
        return -1;
    }
    if (chunk_len == 0 || chunk_len > kPoseidon2Width16 - kPoseidon2HalfRounds) {
        return -2;
    }

    const size_t state_bytes = row_count * kPoseidon2Width16 * sizeof(uint64_t);
    DeviceBuffer<uint64_t> device_packed;

    LZVM_CUDA_RETURN_ON_ERROR(device_packed.reset(row_count * kPoseidon2Width16));
    LZVM_CUDA_RETURN_ON_ERROR(cudaMemset(device_packed.data(), 0, state_bytes));

    const size_t blocks = (row_count + kThreads - 1) / kThreads;
    pack_poseidon2_width16_linear_round_inputs_kernel<<<blocks, kThreads>>>(
        current_states, row_values, device_packed.data(), row_count, chunk_len);
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_check_launch());
    return run_poseidon2_width16_on_device(device_packed.data(), device_out, row_count);
}

}  // namespace

extern "C" int lzvm_cuda_setup_init(
    const uint64_t* roots,
    size_t root_count,
    size_t max_bits_ext) {
    if (roots == nullptr) {
        return -1;
    }
    if (root_count == 0 || root_count > kMaxRootBits + 1 || max_bits_ext >= root_count) {
        return -2;
    }

    int device_id = 0;
    LZVM_CUDA_RETURN_ON_ERROR(cudaGetDevice(&device_id));
    LZVM_CUDA_RETURN_ON_ERROR(cudaSetDevice(device_id));
    LZVM_CUDA_RETURN_ON_ERROR(
        cudaMemcpyToSymbol(kNttStageRoots, roots, root_count * sizeof(uint64_t)));

    uint64_t inverse_roots[kMaxRootBits + 1];
    for (size_t index = 0; index < root_count; ++index) {
        inverse_roots[index] = host_pow_mod(roots[index], kModulus - 2);
    }
    LZVM_CUDA_RETURN_ON_ERROR(cudaMemcpyToSymbol(
        kNttStageRootInverses, inverse_roots, root_count * sizeof(uint64_t)));

    const unsigned int root_limit = static_cast<unsigned int>(max_bits_ext);
    LZVM_CUDA_RETURN_ON_ERROR(
        cudaMemcpyToSymbol(kNttStageRootLimit, &root_limit, sizeof(root_limit)));
    return 0;
}

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

    const size_t bytes = len * sizeof(uint64_t);
    DeviceBuffer<uint64_t> device_lhs;
    DeviceBuffer<uint64_t> device_rhs;
    DeviceBuffer<uint64_t> device_out;

    LZVM_CUDA_RETURN_ON_ERROR(device_lhs.reset(len));
    LZVM_CUDA_RETURN_ON_ERROR(device_rhs.reset(len));
    LZVM_CUDA_RETURN_ON_ERROR(device_out.reset(len));
    LZVM_CUDA_RETURN_ON_ERROR(device_lhs.copy_from_bytes(lhs, bytes));
    LZVM_CUDA_RETURN_ON_ERROR(device_rhs.copy_from_bytes(rhs, bytes));

    const size_t blocks = (len + kThreads - 1) / kThreads;
    add_kernel<<<blocks, kThreads>>>(device_lhs.data(), device_rhs.data(), device_out.data(), len);
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_check_launch());
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_synchronize());
    LZVM_CUDA_RETURN_ON_ERROR(device_out.copy_to_bytes(out, bytes));
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

    const size_t bytes = len * sizeof(uint64_t);
    DeviceBuffer<uint64_t> device_even;
    DeviceBuffer<uint64_t> device_odd;
    DeviceBuffer<uint64_t> device_twiddle;
    DeviceBuffer<uint64_t> device_out_even;
    DeviceBuffer<uint64_t> device_out_odd;

    LZVM_CUDA_RETURN_ON_ERROR(device_even.reset(len));
    LZVM_CUDA_RETURN_ON_ERROR(device_odd.reset(len));
    LZVM_CUDA_RETURN_ON_ERROR(device_twiddle.reset(len));
    LZVM_CUDA_RETURN_ON_ERROR(device_out_even.reset(len));
    LZVM_CUDA_RETURN_ON_ERROR(device_out_odd.reset(len));
    LZVM_CUDA_RETURN_ON_ERROR(device_even.copy_from_bytes(even, bytes));
    LZVM_CUDA_RETURN_ON_ERROR(device_odd.copy_from_bytes(odd, bytes));
    LZVM_CUDA_RETURN_ON_ERROR(device_twiddle.copy_from_bytes(twiddle, bytes));

    const size_t blocks = (len + kThreads - 1) / kThreads;
    butterfly_kernel<<<blocks, kThreads>>>(
        device_even.data(),
        device_odd.data(),
        device_twiddle.data(),
        device_out_even.data(),
        device_out_odd.data(),
        len);
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_check_launch());
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_synchronize());
    LZVM_CUDA_RETURN_ON_ERROR(device_out_even.copy_to_bytes(out_even, bytes));
    LZVM_CUDA_RETURN_ON_ERROR(device_out_odd.copy_to_bytes(out_odd, bytes));
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

    const size_t bytes = len * sizeof(uint64_t);
    DeviceBuffer<uint64_t> device_lhs;
    DeviceBuffer<uint64_t> device_rhs;
    DeviceBuffer<uint64_t> device_out;

    LZVM_CUDA_RETURN_ON_ERROR(device_lhs.reset(len));
    LZVM_CUDA_RETURN_ON_ERROR(device_rhs.reset(len));
    LZVM_CUDA_RETURN_ON_ERROR(device_out.reset(len));
    LZVM_CUDA_RETURN_ON_ERROR(device_lhs.copy_from_bytes(lhs, bytes));
    LZVM_CUDA_RETURN_ON_ERROR(device_rhs.copy_from_bytes(rhs, bytes));

    const size_t blocks = (len + kThreads - 1) / kThreads;
    mul_kernel<<<blocks, kThreads>>>(device_lhs.data(), device_rhs.data(), device_out.data(), len);
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_check_launch());
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_synchronize());
    LZVM_CUDA_RETURN_ON_ERROR(device_out.copy_to_bytes(out, bytes));
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

    const size_t bytes = len * sizeof(uint64_t);
    DeviceBuffer<uint64_t> device_values;

    LZVM_CUDA_RETURN_ON_ERROR(device_values.reset(len));
    LZVM_CUDA_RETURN_ON_ERROR(device_values.copy_from_bytes(values, bytes));

    cudaError_t status = run_ntt(device_values.data(), len, bits, root, false);
    if (status != cudaSuccess) {
        return static_cast<int>(status);
    }

    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_synchronize());
    LZVM_CUDA_RETURN_ON_ERROR(device_values.copy_to_bytes(out, bytes));
    return 0;
}

extern "C" int lzvm_cuda_goldilocks_intt(
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

    const size_t bytes = len * sizeof(uint64_t);
    DeviceBuffer<uint64_t> device_values;

    LZVM_CUDA_RETURN_ON_ERROR(device_values.reset(len));
    LZVM_CUDA_RETURN_ON_ERROR(device_values.copy_from_bytes(values, bytes));

    const uint64_t root_inverse = host_pow_mod(root, kModulus - 2);
    cudaError_t status = run_ntt(device_values.data(), len, bits, root_inverse, true);
    if (status != cudaSuccess) {
        return static_cast<int>(status);
    }

    const uint64_t inverse_len = host_pow_mod(static_cast<uint64_t>(len), kModulus - 2);
    const size_t blocks = (len + kThreads - 1) / kThreads;
    scale_kernel<<<blocks, kThreads>>>(device_values.data(), len, inverse_len);
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_check_launch());

    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_synchronize());
    LZVM_CUDA_RETURN_ON_ERROR(device_values.copy_to_bytes(out, bytes));
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

    const size_t source_bytes = source_len * sizeof(uint64_t);
    const size_t target_bytes = target_len * sizeof(uint64_t);
    DeviceBuffer<uint64_t> device_values;

    LZVM_CUDA_RETURN_ON_ERROR(device_values.reset(target_len));
    LZVM_CUDA_RETURN_ON_ERROR(device_values.copy_from_bytes(values, source_bytes));
    LZVM_CUDA_RETURN_ON_ERROR(run_coset_extend_on_device(
        device_values.data(),
        source_len,
        source_bits,
        target_len,
        target_bits,
        source_root_inverse,
        target_root,
        shift));
    LZVM_CUDA_RETURN_ON_ERROR(device_values.copy_to_bytes(out, target_bytes));
    return 0;
}

extern "C" int lzvm_cuda_goldilocks_coset_extend_row_major_columns(
    const uint64_t* values,
    uint64_t* out,
    size_t source_len,
    size_t source_bits,
    size_t target_len,
    size_t target_bits,
    size_t column_count,
    uint64_t source_root_inverse,
    uint64_t target_root,
    uint64_t shift) {
    if (values == nullptr || out == nullptr) {
        return -1;
    }
    if (source_len == 0 || target_len == 0 || source_len > target_len || column_count == 0) {
        return -2;
    }

    const size_t source_words = source_len * column_count;
    const size_t target_words = target_len * column_count;
    const size_t source_bytes = source_words * sizeof(uint64_t);
    const size_t target_bytes = target_words * sizeof(uint64_t);
    DeviceBuffer<uint64_t> device_values;
    DeviceBuffer<uint64_t> device_columns;
    DeviceBuffer<uint64_t> device_out;

    LZVM_CUDA_RETURN_ON_ERROR(device_values.reset(source_words));
    LZVM_CUDA_RETURN_ON_ERROR(device_columns.reset(target_words));
    LZVM_CUDA_RETURN_ON_ERROR(device_out.reset(target_words));
    LZVM_CUDA_RETURN_ON_ERROR(device_values.copy_from_bytes(values, source_bytes));

    const size_t source_blocks = (source_words + kThreads - 1) / kThreads;
    pack_row_major_columns_kernel<<<source_blocks, kThreads>>>(
        device_values.data(), device_columns.data(), source_len, target_len, column_count);
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_check_launch());

    for (size_t column = 0; column < column_count; ++column) {
        LZVM_CUDA_RETURN_ON_ERROR(run_coset_extend_on_device_unsynced(
            device_columns.data() + column * target_len, source_len, source_bits, target_len,
            target_bits, source_root_inverse, target_root, shift));
    }

    const size_t target_blocks = (target_words + kThreads - 1) / kThreads;
    unpack_row_major_columns_kernel<<<target_blocks, kThreads>>>(
        device_columns.data(), device_out.data(), target_len, column_count);
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_check_launch());
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_synchronize());
    LZVM_CUDA_RETURN_ON_ERROR(device_out.copy_to_bytes(out, target_bytes));
    return 0;
}

extern "C" int lzvm_cuda_goldilocks_coset_extend_device(
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

    const size_t source_bytes = source_len * sizeof(uint64_t);
    LZVM_CUDA_RETURN_ON_ERROR(
        cudaMemcpy(out, values, source_bytes, cudaMemcpyDeviceToDevice));
    return run_coset_extend_on_device(
        out,
        source_len,
        source_bits,
        target_len,
        target_bits,
        source_root_inverse,
        target_root,
        shift);
}

extern "C" int lzvm_cuda_poseidon2_width4(
    const uint64_t* values,
    uint64_t* out,
    size_t state_count) {
    if (state_count == 0) {
        return 0;
    }
    if (values == nullptr || out == nullptr) {
        return -1;
    }

    const size_t word_count = state_count * kPoseidon2Width4;
    const size_t bytes = word_count * sizeof(uint64_t);
    DeviceBuffer<uint64_t> device_values;
    DeviceBuffer<uint64_t> device_out;

    LZVM_CUDA_RETURN_ON_ERROR(device_values.reset(word_count));
    LZVM_CUDA_RETURN_ON_ERROR(device_out.reset(word_count));
    LZVM_CUDA_RETURN_ON_ERROR(device_values.copy_from_bytes(values, bytes));
    LZVM_CUDA_RETURN_ON_ERROR(
        run_poseidon2_width4_on_device(device_values.data(), device_out.data(), state_count));
    LZVM_CUDA_RETURN_ON_ERROR(device_out.copy_to_bytes(out, bytes));
    return 0;
}

extern "C" int lzvm_cuda_poseidon2_width4_device(
    const uint64_t* values,
    uint64_t* out,
    size_t state_count) {
    return run_poseidon2_width4_on_device(values, out, state_count);
}

extern "C" int lzvm_cuda_poseidon2_width4_find_nonce(
    const uint64_t* challenge,
    uint64_t start,
    size_t count,
    uint64_t target,
    uint64_t* out,
    unsigned int* found) {
    if (count == 0) {
        return 0;
    }
    if (challenge == nullptr || out == nullptr || found == nullptr) {
        return -1;
    }

    DeviceBuffer<uint64_t> device_challenge;
    DeviceBuffer<uint64_t> device_out;
    DeviceBuffer<unsigned int> device_found;

    LZVM_CUDA_RETURN_ON_ERROR(device_challenge.reset(3));
    LZVM_CUDA_RETURN_ON_ERROR(device_out.reset(1));
    LZVM_CUDA_RETURN_ON_ERROR(device_found.reset(1));

    const uint64_t initial_out = UINT64_MAX;
    const unsigned int initial_found = 0;
    LZVM_CUDA_RETURN_ON_ERROR(
        device_challenge.copy_from_bytes(challenge, 3 * sizeof(uint64_t)));
    LZVM_CUDA_RETURN_ON_ERROR(device_out.copy_from_bytes(&initial_out, sizeof(uint64_t)));
    LZVM_CUDA_RETURN_ON_ERROR(
        device_found.copy_from_bytes(&initial_found, sizeof(unsigned int)));

    const size_t blocks = (count + kThreads - 1) / kThreads;
    poseidon2_width4_find_nonce_kernel<<<blocks, kThreads>>>(
        device_challenge.data(),
        start,
        count,
        target,
        device_out.data(),
        device_found.data());
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_check_launch());
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_synchronize());
    LZVM_CUDA_RETURN_ON_ERROR(device_out.copy_to_bytes(out, sizeof(uint64_t)));
    LZVM_CUDA_RETURN_ON_ERROR(device_found.copy_to_bytes(found, sizeof(unsigned int)));
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

    const size_t word_count = state_count * kPoseidon2Width8;
    const size_t bytes = word_count * sizeof(uint64_t);
    DeviceBuffer<uint64_t> device_values;
    DeviceBuffer<uint64_t> device_out;

    LZVM_CUDA_RETURN_ON_ERROR(device_values.reset(word_count));
    LZVM_CUDA_RETURN_ON_ERROR(device_out.reset(word_count));
    LZVM_CUDA_RETURN_ON_ERROR(device_values.copy_from_bytes(values, bytes));
    LZVM_CUDA_RETURN_ON_ERROR(
        run_poseidon2_width8_on_device(device_values.data(), device_out.data(), state_count));
    LZVM_CUDA_RETURN_ON_ERROR(device_out.copy_to_bytes(out, bytes));
    return 0;
}

extern "C" int lzvm_cuda_poseidon2_width8_device(
    const uint64_t* values,
    uint64_t* out,
    size_t state_count) {
    return run_poseidon2_width8_on_device(values, out, state_count);
}

extern "C" int lzvm_cuda_poseidon2_width8_merkle_parent_device(
    const uint64_t* values,
    uint64_t* out,
    size_t child_state_count) {
    return run_poseidon2_width8_merkle_parent_on_device(values, out, child_state_count);
}

extern "C" int lzvm_cuda_poseidon2_width8_linear_round_device(
    const uint64_t* current_states,
    const uint64_t* row_values,
    uint64_t* out,
    size_t row_count,
    size_t chunk_len) {
    return run_poseidon2_width8_linear_round_on_device(
        current_states, row_values, out, row_count, chunk_len);
}

extern "C" int lzvm_cuda_poseidon2_width16(
    const uint64_t* values,
    uint64_t* out,
    size_t state_count) {
    if (state_count == 0) {
        return 0;
    }
    if (values == nullptr || out == nullptr) {
        return -1;
    }

    const size_t word_count = state_count * kPoseidon2Width16;
    const size_t bytes = word_count * sizeof(uint64_t);
    DeviceBuffer<uint64_t> device_values;
    DeviceBuffer<uint64_t> device_out;

    LZVM_CUDA_RETURN_ON_ERROR(device_values.reset(word_count));
    LZVM_CUDA_RETURN_ON_ERROR(device_out.reset(word_count));
    LZVM_CUDA_RETURN_ON_ERROR(device_values.copy_from_bytes(values, bytes));
    LZVM_CUDA_RETURN_ON_ERROR(
        run_poseidon2_width16_on_device(device_values.data(), device_out.data(), state_count));
    LZVM_CUDA_RETURN_ON_ERROR(device_out.copy_to_bytes(out, bytes));
    return 0;
}

extern "C" int lzvm_cuda_poseidon2_width16_device(
    const uint64_t* values,
    uint64_t* out,
    size_t state_count) {
    return run_poseidon2_width16_on_device(values, out, state_count);
}

extern "C" int lzvm_cuda_poseidon2_width16_merkle_parent_device(
    const uint64_t* values,
    uint64_t* out,
    size_t child_state_count) {
    return run_poseidon2_width16_merkle_parent_on_device(values, out, child_state_count);
}

extern "C" int lzvm_cuda_poseidon2_width16_linear_round_device(
    const uint64_t* current_states,
    const uint64_t* row_values,
    uint64_t* out,
    size_t row_count,
    size_t chunk_len) {
    return run_poseidon2_width16_linear_round_on_device(
        current_states, row_values, out, row_count, chunk_len);
}

extern "C" int lzvm_cuda_keccak256_fixed(
    const uint8_t* input,
    size_t message_len,
    uint8_t* out,
    size_t message_count) {
    if (message_count == 0) {
        return 0;
    }
    if (message_len == 0) {
        return -2;
    }
    if (input == nullptr || out == nullptr) {
        return -1;
    }

    const size_t input_bytes = message_count * message_len;
    const size_t output_bytes = message_count * kKeccakOutputBytes;
    DeviceBuffer<uint8_t> device_input;
    DeviceBuffer<uint8_t> device_out;

    LZVM_CUDA_RETURN_ON_ERROR(device_input.reset(input_bytes));
    LZVM_CUDA_RETURN_ON_ERROR(device_out.reset(output_bytes));

    LZVM_CUDA_RETURN_ON_ERROR(device_input.copy_from_bytes(input, input_bytes));

    const size_t blocks = (message_count + kThreads - 1) / kThreads;
    keccak256_fixed_kernel<<<blocks, kThreads>>>(
        device_input.data(), device_out.data(), message_len, message_count);
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_check_launch());
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_synchronize());
    LZVM_CUDA_RETURN_ON_ERROR(device_out.copy_to_bytes(out, output_bytes));
    return 0;
}
