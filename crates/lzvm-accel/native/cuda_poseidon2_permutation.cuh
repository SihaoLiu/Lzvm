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
