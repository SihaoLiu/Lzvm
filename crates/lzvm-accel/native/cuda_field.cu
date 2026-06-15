#include <cuda_runtime.h>
#include <stdint.h>
#include <algorithm>
#include <chrono>
#include <limits>
#include <vector>

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
constexpr size_t kPoseidon2DigestWords = 4;
constexpr size_t kPoseidon2Width4PartialRounds = 21;
constexpr size_t kPoseidon2PartialRounds = 22;
constexpr size_t kKeccakRateBytes = 136;
constexpr size_t kKeccakStateLanes = 25;
constexpr size_t kKeccakRateLanes = 17;
constexpr size_t kKeccakOutputBytes = 32;
constexpr size_t kMaxRootBits = 32;
constexpr uint64_t kGoldilocksEpsilon = 0xffffffffULL;

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

int record_direct_d2h_copy(void* dst, const void* src, size_t bytes) {
    if (bytes == 0) {
        return 0;
    }
    const auto copy_started = std::chrono::steady_clock::now();
    const int status = static_cast<int>(cudaMemcpy(dst, src, bytes, cudaMemcpyDeviceToHost));
    const auto elapsed = std::chrono::steady_clock::now() - copy_started;
    const auto elapsed_ns =
        std::chrono::duration_cast<std::chrono::nanoseconds>(elapsed).count();
    lzvm_cuda_record_direct_copy_d2h_wait(
        bytes,
        elapsed_ns > 0 ? static_cast<size_t>(elapsed_ns) : size_t{0});
    return status;
}

__device__ uint64_t add_wrapping_modulus(uint64_t lhs, uint64_t rhs) {
    const uint64_t sum = lhs + rhs;
    return sum < lhs ? sum + kGoldilocksEpsilon : sum;
}

__device__ uint64_t reduce_goldilocks_product(unsigned __int128 value) {
    const uint64_t lo = static_cast<uint64_t>(value);
    const uint64_t hi = static_cast<uint64_t>(value >> 64);
    const uint64_t hi_hi = hi >> 32;
    const uint64_t hi_lo = hi & kGoldilocksEpsilon;

    uint64_t reduced = lo - hi_hi;
    if (lo < hi_hi) {
        reduced -= kGoldilocksEpsilon;
    }
    reduced = add_wrapping_modulus(reduced, hi_lo * kGoldilocksEpsilon);
    return reduced >= kModulus ? reduced - kModulus : reduced;
}

__device__ uint64_t mul_mod(uint64_t lhs, uint64_t rhs) {
    const unsigned __int128 product =
        static_cast<unsigned __int128>(lhs) * static_cast<unsigned __int128>(rhs);
    return reduce_goldilocks_product(product);
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

#include "cuda_regular_constraints.cuh"
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

__global__ void validate_canonical_words_kernel(
    const uint64_t* values,
    size_t word_count,
    unsigned int* found) {
    const size_t index = blockIdx.x * blockDim.x + threadIdx.x;
    if (index < word_count && values[index] >= kModulus) {
        atomicExch(found, 1U);
    }
}

__global__ void normalize_shift_and_pad_kernel(
    uint64_t* values,
    size_t source_len,
    size_t target_len,
    uint64_t inverse_len,
    uint64_t shift) {
    __shared__ uint64_t block_shift;
    __shared__ uint64_t thread_powers[8];
    if (threadIdx.x == 0) {
        block_shift = pow_mod(shift, static_cast<size_t>(blockIdx.x) * blockDim.x);
        thread_powers[0] = shift;
        for (size_t bit = 1; bit < 8; ++bit) {
            thread_powers[bit] = mul_mod(thread_powers[bit - 1], thread_powers[bit - 1]);
        }
    }
    __syncthreads();

    const size_t index = blockIdx.x * blockDim.x + threadIdx.x;
    if (index < target_len) {
        if (index < source_len) {
            uint64_t factor = block_shift;
            size_t exponent = static_cast<size_t>(threadIdx.x);
            for (size_t bit = 0; exponent != 0 && bit < 8; ++bit, exponent >>= 1) {
                if ((exponent & 1) != 0) { factor = mul_mod(factor, thread_powers[bit]); }
            }
            values[index] = mul_mod(mul_mod(values[index], inverse_len), factor);
        } else {
            values[index] = 0;
        }
    }
}

#include "cuda_goldilocks_ntt.cuh"

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

__global__ void pack_row_major_columns_strided_kernel(
    const uint64_t* values, uint64_t* columns, size_t source_len, size_t target_len,
    size_t source_row_stride, size_t column_offset, size_t column_count) {
    const size_t index = blockIdx.x * blockDim.x + threadIdx.x;
    const size_t total = source_len * column_count;
    if (index < total) {
        const size_t row = index / column_count;
        const size_t column = index % column_count;
        columns[column * target_len + row] = values[row * source_row_stride + column_offset + column];
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

__global__ void copy_d2d_selected_row_major_rows_kernel(
    uint64_t* dst,
    const uint64_t* src,
    const uint64_t* rows,
    size_t selected_row_count,
    size_t source_row_count,
    size_t row_width_words) {
    const size_t word_index = blockIdx.x * blockDim.x + threadIdx.x;
    const size_t total_words = selected_row_count * row_width_words;
    if (word_index >= total_words) {
        return;
    }
    const size_t selected_row_index = word_index / row_width_words;
    const size_t column = word_index - selected_row_index * row_width_words;
    const uint64_t source_row = rows[selected_row_index];
    if (source_row >= source_row_count) {
        return;
    }
    dst[word_index] = src[static_cast<size_t>(source_row) * row_width_words + column];
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

#include "cuda_poseidon2_merkle_parent.cuh"
#include "cuda_poseidon2_merkle_opening.cuh"
#include "cuda_poseidon2_merkle_root.cuh"
#include "cuda_poseidon2_merkle_digest.cuh"
#include "cuda_poseidon2_row_major.cuh"

}  // namespace

extern "C" int lzvm_cuda_current_device(int* out) {
    if (out == nullptr) {
        return -1;
    }
    LZVM_CUDA_RETURN_ON_ERROR(cudaGetDevice(out));
    return 0;
}

extern "C" int lzvm_cuda_setup_root_limit(unsigned int* out) {
    if (out == nullptr) {
        return -1;
    }
    LZVM_CUDA_RETURN_ON_ERROR(
        cudaMemcpyFromSymbol(out, kNttStageRootLimit, sizeof(*out)));
    return 0;
}

extern "C" int lzvm_cuda_copy_d2d_selected_row_major_rows(
    void* dst,
    const void* src,
    const uint64_t* rows,
    size_t selected_row_count,
    size_t source_row_count,
    size_t row_width_words) {
    if (selected_row_count == 0) {
        return 0;
    }
    if (dst == nullptr || src == nullptr || rows == nullptr) {
        return -1;
    }
    if (row_width_words == 0) {
        return -2;
    }
    if (selected_row_count > std::numeric_limits<size_t>::max() / row_width_words ||
        source_row_count > std::numeric_limits<size_t>::max() / row_width_words) {
        return -2;
    }
    const size_t total_words = selected_row_count * row_width_words;
    const size_t blocks = (total_words + kThreads - 1) / kThreads;
    copy_d2d_selected_row_major_rows_kernel<<<blocks, kThreads>>>(
        static_cast<uint64_t*>(dst),
        static_cast<const uint64_t*>(src),
        rows,
        selected_row_count,
        source_row_count,
        row_width_words);
    return lzvm_cuda_check_launch();
}

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

    cudaError_t status = run_ntt(device_values.data(), len, bits, root, false, 0);
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
    cudaError_t status = run_ntt(device_values.data(), len, bits, root_inverse, true, 0);
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

int run_row_major_columns_device(
    const uint64_t* values, uint64_t* out, uint64_t* workspace, size_t source_len,
    size_t source_bits, size_t target_len, size_t target_bits, size_t column_count,
    uint64_t source_root_inverse, uint64_t target_root, uint64_t shift, bool synchronize,
    cudaStream_t stream) {
    if (values == nullptr || out == nullptr) { return -1; }
    if (source_len == 0 || target_len == 0 || source_len > target_len || column_count == 0) {
        return -2;
    }

    const size_t source_words = source_len * column_count;
    const size_t target_words = target_len * column_count;
    DeviceBuffer<uint64_t> device_columns;
    uint64_t* columns = workspace;

    if (columns == nullptr) {
        if (!synchronize) {
            return -1;
        }
        LZVM_CUDA_RETURN_ON_ERROR(device_columns.reset(target_words));
        columns = device_columns.data();
    }
    const size_t source_blocks = (source_words + kThreads - 1) / kThreads;
    pack_row_major_columns_kernel<<<source_blocks, kThreads, 0, stream>>>(
        values, columns, source_len, target_len, column_count);
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_check_launch());
    for (size_t column = 0; column < column_count; ++column) {
        LZVM_CUDA_RETURN_ON_ERROR(run_coset_extend_on_device_unsynced(
            columns + column * target_len, source_len, source_bits, target_len, target_bits,
            source_root_inverse, target_root, shift, stream));
    }
    const size_t target_blocks = (target_words + kThreads - 1) / kThreads;
    unpack_row_major_columns_kernel<<<target_blocks, kThreads, 0, stream>>>(
        columns, out, target_len, column_count);
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_check_launch());
    return synchronize ? lzvm_cuda_synchronize() : 0;
}

extern "C" int lzvm_cuda_goldilocks_coset_extend_row_major_columns_device(
    const uint64_t* values, uint64_t* out, size_t source_len, size_t source_bits,
    size_t target_len, size_t target_bits, size_t column_count, uint64_t source_root_inverse,
    uint64_t target_root, uint64_t shift) {
    return run_row_major_columns_device(
        values, out, nullptr, source_len, source_bits, target_len, target_bits, column_count,
        source_root_inverse, target_root, shift, true, 0);
}

extern "C" int lzvm_cuda_goldilocks_coset_extend_row_major_columns_device_unsynced(
    const uint64_t* values, uint64_t* out, uint64_t* workspace, size_t source_len,
    size_t source_bits, size_t target_len, size_t target_bits, size_t column_count,
    uint64_t source_root_inverse, uint64_t target_root, uint64_t shift) {
    return run_row_major_columns_device(
        values, out, workspace, source_len, source_bits, target_len, target_bits, column_count,
        source_root_inverse, target_root, shift, false, 0);
}

extern "C" int lzvm_cuda_goldilocks_coset_extend_row_major_columns_device_on_stream(
    const uint64_t* values, uint64_t* out, uint64_t* workspace, size_t source_len,
    size_t source_bits, size_t target_len, size_t target_bits, size_t column_count,
    uint64_t source_root_inverse, uint64_t target_root, uint64_t shift, void* stream_raw) {
    cudaStream_t stream = static_cast<cudaStream_t>(stream_raw);
    return run_row_major_columns_device(
        values, out, workspace, source_len, source_bits, target_len, target_bits, column_count,
        source_root_inverse, target_root, shift, false, stream);
}

int run_row_major_columns_strided_device(
    const uint64_t* values, uint64_t* out, uint64_t* workspace, size_t source_len,
    size_t source_bits, size_t target_len, size_t target_bits, size_t source_row_stride,
    size_t column_offset, size_t column_count, uint64_t source_root_inverse, uint64_t target_root,
    uint64_t shift, bool synchronize, cudaStream_t stream) {
    if (values == nullptr || out == nullptr) { return -1; }
    if (source_len == 0 || target_len == 0 || source_len > target_len || column_count == 0 ||
        source_row_stride == 0 || column_offset > source_row_stride ||
        column_count > source_row_stride - column_offset) {
        return -2;
    }

    const size_t source_words = source_len * column_count;
    const size_t target_words = target_len * column_count;
    DeviceBuffer<uint64_t> device_columns;
    uint64_t* columns = workspace;

    if (columns == nullptr) {
        if (!synchronize) {
            return -1;
        }
        LZVM_CUDA_RETURN_ON_ERROR(device_columns.reset(target_words));
        columns = device_columns.data();
    }
    const size_t source_blocks = (source_words + kThreads - 1) / kThreads;
    pack_row_major_columns_strided_kernel<<<source_blocks, kThreads, 0, stream>>>(
        values, columns, source_len, target_len, source_row_stride, column_offset, column_count);
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_check_launch());
    for (size_t column = 0; column < column_count; ++column) {
        LZVM_CUDA_RETURN_ON_ERROR(run_coset_extend_on_device_unsynced(
            columns + column * target_len, source_len, source_bits, target_len, target_bits,
            source_root_inverse, target_root, shift, stream));
    }
    const size_t target_blocks = (target_words + kThreads - 1) / kThreads;
    unpack_row_major_columns_kernel<<<target_blocks, kThreads, 0, stream>>>(
        columns, out, target_len, column_count);
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_check_launch());
    return synchronize ? lzvm_cuda_synchronize() : 0;
}

extern "C" int lzvm_cuda_goldilocks_coset_extend_row_major_columns_strided_device(
    const uint64_t* values, uint64_t* out, size_t source_len, size_t source_bits,
    size_t target_len, size_t target_bits, size_t source_row_stride, size_t column_offset,
    size_t column_count, uint64_t source_root_inverse, uint64_t target_root, uint64_t shift) {
    return run_row_major_columns_strided_device(
        values, out, nullptr, source_len, source_bits, target_len, target_bits, source_row_stride,
        column_offset, column_count, source_root_inverse, target_root, shift, true, 0);
}

extern "C" int lzvm_cuda_goldilocks_coset_extend_row_major_columns_strided_device_unsynced(
    const uint64_t* values, uint64_t* out, uint64_t* workspace, size_t source_len,
    size_t source_bits, size_t target_len, size_t target_bits, size_t source_row_stride,
    size_t column_offset, size_t column_count, uint64_t source_root_inverse, uint64_t target_root,
    uint64_t shift) {
    return run_row_major_columns_strided_device(
        values, out, workspace, source_len, source_bits, target_len, target_bits, source_row_stride,
        column_offset, column_count, source_root_inverse, target_root, shift, false, 0);
}

extern "C" int lzvm_cuda_goldilocks_coset_extend_row_major_columns_strided_device_on_stream(
    const uint64_t* values, uint64_t* out, uint64_t* workspace, size_t source_len,
    size_t source_bits, size_t target_len, size_t target_bits, size_t source_row_stride,
    size_t column_offset, size_t column_count, uint64_t source_root_inverse, uint64_t target_root,
    uint64_t shift, void* stream_raw) {
    cudaStream_t stream = static_cast<cudaStream_t>(stream_raw);
    return run_row_major_columns_strided_device(
        values, out, workspace, source_len, source_bits, target_len, target_bits, source_row_stride,
        column_offset, column_count, source_root_inverse, target_root, shift, false, stream);
}

#include "cuda_goldilocks_row_extend.cuh"

#include "cuda_goldilocks_canonical.cuh"

#include "cuda_row_major_fill.cuh"
#include "cuda_zisk_main_trace.cuh"

extern "C" int lzvm_cuda_goldilocks_coset_extend_row_major_columns(
    const uint64_t* values, uint64_t* out, size_t source_len, size_t source_bits,
    size_t target_len, size_t target_bits, size_t column_count, uint64_t source_root_inverse,
    uint64_t target_root, uint64_t shift) {
    if (values == nullptr || out == nullptr) { return -1; }
    if (source_len == 0 || target_len == 0 || source_len > target_len || column_count == 0) {
        return -2;
    }

    const size_t source_words = source_len * column_count;
    const size_t target_words = target_len * column_count;
    const size_t source_bytes = source_words * sizeof(uint64_t);
    const size_t target_bytes = target_words * sizeof(uint64_t);
    DeviceBuffer<uint64_t> device_values;
    DeviceBuffer<uint64_t> device_out;

    LZVM_CUDA_RETURN_ON_ERROR(device_values.reset(source_words));
    LZVM_CUDA_RETURN_ON_ERROR(device_out.reset(target_words));
    LZVM_CUDA_RETURN_ON_ERROR(device_values.copy_from_bytes(values, source_bytes));
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_goldilocks_coset_extend_row_major_columns_device(
        device_values.data(), device_out.data(), source_len, source_bits, target_len, target_bits,
        column_count, source_root_inverse, target_root, shift));
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

extern "C" int lzvm_cuda_poseidon2_width16_linear_round_device(
    const uint64_t* current_states,
    const uint64_t* row_values,
    uint64_t* out,
    size_t row_count,
    size_t chunk_len) {
    return run_poseidon2_width16_linear_round_on_device(
        current_states, row_values, out, row_count, chunk_len);
}

#include "cuda_poseidon2_merkle_exports.cuh"
#include "cuda_poseidon2_row_major_exports.cuh"
#include "cuda_keccak_exports.cuh"
