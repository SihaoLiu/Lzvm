#include <cuda_runtime.h>
#include <stdint.h>

namespace {

constexpr uint64_t kModulus = 0xffffffff00000001ULL;
constexpr size_t kThreads = 256;
constexpr size_t kPoseidon2Width4 = 4;
constexpr size_t kPoseidon2Width8 = 8;
constexpr size_t kPoseidon2Width16 = 16;
constexpr size_t kPoseidon2HalfRounds = 4;
constexpr size_t kPoseidon2Width4PartialRounds = 21;
constexpr size_t kPoseidon2PartialRounds = 22;

__device__ __constant__ uint64_t kPoseidon2Width4Diag[kPoseidon2Width4] = {
    0xf0ce126fe8a83094ULL,
    0x60f87e0b59fb4ee6ULL,
    0xa8106c221cd6d882ULL,
    0x5529eddc46e372e7ULL,
};

__device__ __constant__ uint64_t kPoseidon2Width4RoundConstants[53] = {
    0x5098165ee28e503eULL,
    0x41b84edfee6c0590ULL,
    0xdda6bc081661f7b8ULL,
    0xb56f892b5fc6d76cULL,
    0xb2b7e92b1f70399fULL,
    0x7075cc44042536e9ULL,
    0xd5aae31b4968adb1ULL,
    0x0713f06eb5e40337ULL,
    0x80dccd8a419cc2d5ULL,
    0x89ae3f75c9b53e2cULL,
    0x8aac5449eff27e1dULL,
    0xef29b2b24bf503f9ULL,
    0xa1d4f9eaaa62e9fcULL,
    0x2f215d5c5a0aa622ULL,
    0x7b3447f34ae22dd9ULL,
    0x4b614218a8e81eefULL,
    0xe063343114e0f434ULL,
    0x2cdedf7f0717ad4eULL,
    0x4662c297f2537cf5ULL,
    0x8fe48eee51761f3dULL,
    0x616aead4ae0ebf00ULL,
    0x9b40b73022b3089bULL,
    0xa051e1646094b036ULL,
    0xf69b2c13f377ff8eULL,
    0x96f7dec4549af9beULL,
    0x858371686234c707ULL,
    0x8483ec4d5e3e8114ULL,
    0x21aea04a4066e649ULL,
    0xbed21bd95c72ec7eULL,
    0x948655aafad4b757ULL,
    0xd4b2ed65735823e2ULL,
    0x1930ef5f54c40462ULL,
    0xb3cc1696b1d3811eULL,
    0xafe0336077202599ULL,
    0x11da6a906ef66e3eULL,
    0xd7abdf7d347fb43fULL,
    0x65e7d3c9f0e8da86ULL,
    0x0b73bdafed7f79f4ULL,
    0x619b24eb14c29f0fULL,
    0x85904bd8db9e3cd9ULL,
    0x4c9c28e673abb589ULL,
    0x73b20f643717949fULL,
    0x832ab3faa2c0639aULL,
    0xfa1d702bafb65207ULL,
    0x03f5f17b0409003cULL,
    0x2c3ff110b39f84d5ULL,
    0x4cdfd3ff34ce6f4fULL,
    0xd3acf5807f208db4ULL,
    0x13d28634ce48e600ULL,
    0xb065f0e667d7caf9ULL,
    0x44f6f3d6b12825caULL,
    0x243a64c03f36ea35ULL,
    0x470a3b7c2f6a6a7aULL,
};

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

__device__ __constant__ uint64_t kPoseidon2Width16Diag[kPoseidon2Width16] = {
    0xde9b91a467d6afc0ULL,
    0xc5f16b9c76a9be17ULL,
    0x0ab0fef2d540ac55ULL,
    0x3001d27009d05773ULL,
    0xed23b1f906d3d9ebULL,
    0x5ce73743cba97054ULL,
    0x1c3bab944af4ba24ULL,
    0x2faa105854dbafaeULL,
    0x53ffb3ae6d421a10ULL,
    0xbcda9df8884ba396ULL,
    0xfc1273e4a31807bbULL,
    0xc77952573d5142c0ULL,
    0x56683339a819b85eULL,
    0x328fcbd8f0ddc8ebULL,
    0xb5101e303fce9cb7ULL,
    0x774487b8c40089bbULL,
};

__device__ __constant__ uint64_t kPoseidon2Width16RoundConstants[150] = {
    0x15ebea3fc73397c3ULL,
    0xd73cd9fbfe8e275cULL,
    0x8c096bfce77f6c26ULL,
    0x4e128f68b53d8feaULL,
    0x29b779a36b2763f6ULL,
    0xfe2adc6fb65acd08ULL,
    0x8d2520e725ad0955ULL,
    0x1c2392b214624d2aULL,
    0x37482118206dcc6eULL,
    0x2f829bed19be019aULL,
    0x2fe298cb6f8159b0ULL,
    0x2bbad982deccdbbfULL,
    0xbad568b8cc60a81eULL,
    0xb86a814265baad10ULL,
    0xbec2005513b3acb3ULL,
    0x6bf89b59a07c2a94ULL,
    0xa25deeb835e230f5ULL,
    0x3c5bad8512b8b12aULL,
    0x7230f73c3cb7a4f2ULL,
    0xa70c87f095c74d0fULL,
    0x6b7606b830bb2e80ULL,
    0x6cd467cfc4f24274ULL,
    0xfeed794df42a9b0aULL,
    0x8cf7cf6163b7dbd3ULL,
    0x9a6e9dda597175a0ULL,
    0xaa52295a684faf7bULL,
    0x017b811cc3589d8dULL,
    0x55bfb699b6181648ULL,
    0xc2ccaf71501c2421ULL,
    0x1707950327596402ULL,
    0xdd2fcdcd42a8229fULL,
    0x8b9d7d5b27778a21ULL,
    0xac9a05525f9cf512ULL,
    0x2ba125c58627b5e8ULL,
    0xc74e91250a8147a5ULL,
    0xa3e64b640d5bb384ULL,
    0xf53047d18d1f9292ULL,
    0xbaaeddacae3a6374ULL,
    0xf2d0914a808b3db1ULL,
    0x18af1a3742bfa3b0ULL,
    0x9a621ef50c55bdb8ULL,
    0xc615f4d1cc5466f3ULL,
    0xb7fbac19a35cf793ULL,
    0xd2b1a15ba517e46dULL,
    0x4a290c4d7fd26f6fULL,
    0x4f0cf1bb1770c4c4ULL,
    0x548345386cd377f5ULL,
    0x33978d2789fddd42ULL,
    0xab78c59deb77e211ULL,
    0xc485b2a933d2be7fULL,
    0xbde3792c00c03c53ULL,
    0xab4cefe8f893d247ULL,
    0xc5c0e752eab7f85fULL,
    0xdbf5a76f893bafeaULL,
    0xa91f6003e3d984deULL,
    0x099539077f311e87ULL,
    0x097ec52232f9559eULL,
    0x53641bdf8991e48cULL,
    0x2afe9711d5ed9d7cULL,
    0xa7b13d3661b5d117ULL,
    0x5a0e243fe7af6556ULL,
    0x1076fae8932d5f00ULL,
    0x9b53a83d434934e3ULL,
    0xed3fd595a3c0344aULL,
    0x28eff4b01103d100ULL,
    0x60400ca3e2685a45ULL,
    0x1c8636beb3389b84ULL,
    0xac1332b60e13eff0ULL,
    0x2adafcc364e20f87ULL,
    0x79ffc2b14054ea0bULL,
    0x3f98e4c0908f0a05ULL,
    0xcdb230bc4e8a06c4ULL,
    0x1bcaf7705b152a74ULL,
    0xd9bca249a82a7470ULL,
    0x91e24af19bf82551ULL,
    0xa62b43ba5cb78858ULL,
    0xb4898117472e797fULL,
    0xb3228bca606cdaa0ULL,
    0x844461051bca39c9ULL,
    0xf3411581f6617d68ULL,
    0xf7fd50646782b533ULL,
    0x6ca664253c18fb48ULL,
    0x2d2fcdec0886a08fULL,
    0x29da00dd799b575eULL,
    0x47d966cc3b6e1e93ULL,
    0xde884e9a17ced59eULL,
    0xdacf46dc1c31a045ULL,
    0x5d2e3c121eb387f2ULL,
    0x51f8b0658b124499ULL,
    0x1e7dbd1daa72167dULL,
    0x8275015a25c55b88ULL,
    0xe8521c24ac7a70b3ULL,
    0x6521d121c40b3f67ULL,
    0xac12de797de135b0ULL,
    0xafa28ead79f6ed6aULL,
    0x685174a7a8d26f0bULL,
    0xeff92a08d35d9874ULL,
    0x3058734b76dd123aULL,
    0xfa55dcfba429f79cULL,
    0x559294d4324c7728ULL,
    0x7a770f53012dc178ULL,
    0xedd8f7c408f3883bULL,
    0x39b533cf8d795fa5ULL,
    0x160ef9de243a8c0aULL,
    0x431d52da6215fe3fULL,
    0x54c51a2a2ef6d528ULL,
    0x9b13892b46ff9d16ULL,
    0x263c46fcee210289ULL,
    0xb738c96d25aabdc4ULL,
    0x5c33a5203996d38fULL,
    0x2626496e7c98d8ddULL,
    0xc669e0a52785903aULL,
    0xaecde726c8ae1f47ULL,
    0x039343ef3a81e999ULL,
    0x2615ceaf044a54f9ULL,
    0x7e41e834662b66e1ULL,
    0x4ca5fd4895335783ULL,
    0x64b334d02916f2b0ULL,
    0x87268837389a6981ULL,
    0x034b75bcb20a6274ULL,
    0x58e658296cc2cd6eULL,
    0xe2d0f759acc31df4ULL,
    0x81a652e435093e20ULL,
    0x0b72b6e0172eaf47ULL,
    0x4aec43cec577d66dULL,
    0xde78365b028a84e6ULL,
    0x444e19569adc0ee4ULL,
    0x942b2451fa40d1daULL,
    0xe24506623ea5bd6cULL,
    0x082854bf2ef7c743ULL,
    0x69dbbc566f59d62eULL,
    0x248c38d02a7b5cb2ULL,
    0x4f4e8f8c09d15edbULL,
    0xd96682f188d310cfULL,
    0x6f9a25d56818b54cULL,
    0xb6cefed606546cd9ULL,
    0x5bc07523da38a67bULL,
    0x7df5a3c35b8111cfULL,
    0xaaa2cc5d4db34bb0ULL,
    0x9e673ff22a4653f8ULL,
    0xbd8b278d60739c62ULL,
    0xe10d20f6925b8815ULL,
    0xf6c87b91dd4da2bfULL,
    0xfed623e2f71b6f1aULL,
    0xa0f02fa52a94d0d3ULL,
    0xbb5794711b39fa16ULL,
    0xd3b94fba9d005c7fULL,
    0x15a26e89fad946c9ULL,
    0xf3cb87db8a67cf49ULL,
    0x400d2bf56aa2a577ULL,
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

int free_nonce_after_error(
    cudaError_t status,
    uint64_t* challenge,
    uint64_t* out,
    unsigned int* found) {
    cudaFree(challenge);
    cudaFree(out);
    cudaFree(found);
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

    const uint64_t root_inverse = host_pow_mod(root, kModulus - 2);
    status = run_ntt(device_values, len, bits, root_inverse);
    if (status != cudaSuccess) {
        return free_single_after_error(status, device_values);
    }

    const uint64_t inverse_len = host_pow_mod(static_cast<uint64_t>(len), kModulus - 2);
    const size_t blocks = (len + kThreads - 1) / kThreads;
    scale_kernel<<<blocks, kThreads>>>(device_values, len, inverse_len);
    status = cudaGetLastError();
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

    uint64_t* device_values = nullptr;
    uint64_t* device_out = nullptr;
    const size_t word_count = state_count * kPoseidon2Width4;
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
    poseidon2_width4_kernel<<<blocks, kThreads>>>(device_values, device_out, state_count);
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

    uint64_t* device_challenge = nullptr;
    uint64_t* device_out = nullptr;
    unsigned int* device_found = nullptr;
    cudaError_t status = cudaMalloc(&device_challenge, 3 * sizeof(uint64_t));
    if (status != cudaSuccess) {
        return static_cast<int>(status);
    }
    status = cudaMalloc(&device_out, sizeof(uint64_t));
    if (status != cudaSuccess) {
        return free_nonce_after_error(status, device_challenge, device_out, device_found);
    }
    status = cudaMalloc(&device_found, sizeof(unsigned int));
    if (status != cudaSuccess) {
        return free_nonce_after_error(status, device_challenge, device_out, device_found);
    }

    const uint64_t initial_out = UINT64_MAX;
    const unsigned int initial_found = 0;
    status = cudaMemcpy(device_challenge, challenge, 3 * sizeof(uint64_t), cudaMemcpyHostToDevice);
    if (status != cudaSuccess) {
        return free_nonce_after_error(status, device_challenge, device_out, device_found);
    }
    status = cudaMemcpy(device_out, &initial_out, sizeof(uint64_t), cudaMemcpyHostToDevice);
    if (status != cudaSuccess) {
        return free_nonce_after_error(status, device_challenge, device_out, device_found);
    }
    status = cudaMemcpy(device_found, &initial_found, sizeof(unsigned int), cudaMemcpyHostToDevice);
    if (status != cudaSuccess) {
        return free_nonce_after_error(status, device_challenge, device_out, device_found);
    }

    const size_t blocks = (count + kThreads - 1) / kThreads;
    poseidon2_width4_find_nonce_kernel<<<blocks, kThreads>>>(
        device_challenge, start, count, target, device_out, device_found);
    status = cudaGetLastError();
    if (status != cudaSuccess) {
        return free_nonce_after_error(status, device_challenge, device_out, device_found);
    }
    status = cudaDeviceSynchronize();
    if (status != cudaSuccess) {
        return free_nonce_after_error(status, device_challenge, device_out, device_found);
    }
    status = cudaMemcpy(out, device_out, sizeof(uint64_t), cudaMemcpyDeviceToHost);
    if (status != cudaSuccess) {
        return free_nonce_after_error(status, device_challenge, device_out, device_found);
    }
    status = cudaMemcpy(found, device_found, sizeof(unsigned int), cudaMemcpyDeviceToHost);
    if (status != cudaSuccess) {
        return free_nonce_after_error(status, device_challenge, device_out, device_found);
    }

    cudaFree(device_challenge);
    cudaFree(device_out);
    cudaFree(device_found);
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

    uint64_t* device_values = nullptr;
    uint64_t* device_out = nullptr;
    const size_t word_count = state_count * kPoseidon2Width16;
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
    poseidon2_width16_kernel<<<blocks, kThreads>>>(device_values, device_out, state_count);
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
