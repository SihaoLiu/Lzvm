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

__device__ __constant__ uint64_t kKeccakRoundConstants[24] = {
    0x0000000000000001ULL,
    0x0000000000008082ULL,
    0x800000000000808aULL,
    0x8000000080008000ULL,
    0x000000000000808bULL,
    0x0000000080000001ULL,
    0x8000000080008081ULL,
    0x8000000000008009ULL,
    0x000000000000008aULL,
    0x0000000000000088ULL,
    0x0000000080008009ULL,
    0x000000008000000aULL,
    0x000000008000808bULL,
    0x800000000000008bULL,
    0x8000000000008089ULL,
    0x8000000000008003ULL,
    0x8000000000008002ULL,
    0x8000000000000080ULL,
    0x000000000000800aULL,
    0x800000008000000aULL,
    0x8000000080008081ULL,
    0x8000000000008080ULL,
    0x0000000080000001ULL,
    0x8000000080008008ULL,
};

__device__ __constant__ unsigned int kKeccakRotationOffsets[25] = {
    0U, 1U, 62U, 28U, 27U, 36U, 44U, 6U, 55U, 20U, 3U, 10U, 43U, 25U, 39U, 41U, 45U, 15U, 21U,
    8U, 18U, 2U, 61U, 56U, 14U,
};
__device__ __forceinline__ size_t keccak_lane_index(size_t x, size_t y) {
    return x + 5 * y;
}
__device__ __forceinline__ uint64_t keccak_rotate_left(uint64_t value, unsigned int shift) {
    return shift == 0U ? value : ((value << shift) | (value >> (64U - shift)));
}

__device__ __forceinline__ uint64_t keccak_load64_le(const uint8_t* bytes) {
    uint64_t value = 0;
    for (size_t index = 0; index < 8; ++index) {
        value |= static_cast<uint64_t>(bytes[index]) << (index * 8);
    }
    return value;
}

__device__ __forceinline__ void keccak_store64_le(uint8_t* bytes, uint64_t value) {
    for (size_t index = 0; index < 8; ++index) {
        bytes[index] = static_cast<uint8_t>((value >> (index * 8)) & 0xffU);
    }
}

__device__ __forceinline__ void keccak_absorb_rate_block(uint64_t* state, const uint8_t* block) {
    for (size_t lane = 0; lane < kKeccakRateLanes; ++lane) {
        state[lane] ^= keccak_load64_le(block + lane * 8);
    }
}

__device__ void keccak_f1600(uint64_t* state) {
    uint64_t c[5];
    uint64_t d[5];
    uint64_t b[kKeccakStateLanes];

    for (size_t round = 0; round < 24; ++round) {
        for (size_t x = 0; x < 5; ++x) {
            c[x] = state[keccak_lane_index(x, 0)] ^ state[keccak_lane_index(x, 1)] ^
                state[keccak_lane_index(x, 2)] ^ state[keccak_lane_index(x, 3)] ^
                state[keccak_lane_index(x, 4)];
        }

        for (size_t x = 0; x < 5; ++x) {
            d[x] = c[(x + 4) % 5] ^ keccak_rotate_left(c[(x + 1) % 5], 1U);
        }

        for (size_t x = 0; x < 5; ++x) {
            for (size_t y = 0; y < 5; ++y) {
                state[keccak_lane_index(x, y)] ^= d[x];
            }
        }

        for (size_t x = 0; x < 5; ++x) {
            for (size_t y = 0; y < 5; ++y) {
                const size_t source = keccak_lane_index(x, y);
                const size_t dest = keccak_lane_index(y, (2 * x + 3 * y) % 5);
                b[dest] = keccak_rotate_left(state[source], kKeccakRotationOffsets[source]);
            }
        }

        for (size_t x = 0; x < 5; ++x) {
            for (size_t y = 0; y < 5; ++y) {
                const size_t index = keccak_lane_index(x, y);
                const size_t next = keccak_lane_index((x + 1) % 5, y);
                const size_t next_next = keccak_lane_index((x + 2) % 5, y);
                state[index] = b[index] ^ ((~b[next]) & b[next_next]);
            }
        }

        state[0] ^= kKeccakRoundConstants[round];
    }
}

__global__ void keccak256_fixed_kernel(
    const uint8_t* input,
    uint8_t* out,
    size_t message_len,
    size_t message_count) {
    const size_t message_index = blockIdx.x * blockDim.x + threadIdx.x;
    if (message_index < message_count) {
        const uint8_t* message = input + message_index * message_len;
        uint64_t state[kKeccakStateLanes];
        for (size_t lane = 0; lane < kKeccakStateLanes; ++lane) {
            state[lane] = 0;
        }

        size_t offset = 0;
        while (offset + kKeccakRateBytes <= message_len) {
            keccak_absorb_rate_block(state, message + offset);
            keccak_f1600(state);
            offset += kKeccakRateBytes;
        }

        uint8_t block[kKeccakRateBytes];
        for (size_t index = 0; index < kKeccakRateBytes; ++index) {
            block[index] = 0;
        }
        const size_t remaining = message_len - offset;
        for (size_t index = 0; index < remaining; ++index) {
            block[index] = message[offset + index];
        }
        block[remaining] = 0x01;
        block[kKeccakRateBytes - 1] |= 0x80;

        keccak_absorb_rate_block(state, block);
        keccak_f1600(state);

        uint8_t* digest = out + message_index * kKeccakOutputBytes;
        for (size_t lane = 0; lane < 4; ++lane) {
            keccak_store64_le(digest + lane * 8, state[lane]);
        }
    }
}
