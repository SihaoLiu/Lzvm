const RATE_BYTES: usize = 136;

const ROUND_CONSTANTS: [u64; 24] = [
    0x0000_0000_0000_0001,
    0x0000_0000_0000_8082,
    0x8000_0000_0000_808a,
    0x8000_0000_8000_8000,
    0x0000_0000_0000_808b,
    0x0000_0000_8000_0001,
    0x8000_0000_8000_8081,
    0x8000_0000_0000_8009,
    0x0000_0000_0000_008a,
    0x0000_0000_0000_0088,
    0x0000_0000_8000_8009,
    0x0000_0000_8000_000a,
    0x0000_0000_8000_808b,
    0x8000_0000_0000_008b,
    0x8000_0000_0000_8089,
    0x8000_0000_0000_8003,
    0x8000_0000_0000_8002,
    0x8000_0000_0000_0080,
    0x0000_0000_0000_800a,
    0x8000_0000_8000_000a,
    0x8000_0000_8000_8081,
    0x8000_0000_0000_8080,
    0x0000_0000_8000_0001,
    0x8000_0000_8000_8008,
];

const ROTATION_OFFSETS: [u32; 25] = [
    0, 1, 62, 28, 27, 36, 44, 6, 55, 20, 3, 10, 43, 25, 39, 41, 45, 15, 21, 8, 18, 2, 61, 56, 14,
];

#[inline]
const fn lane_index(x: usize, y: usize) -> usize {
    x + 5 * y
}

#[inline]
fn xor_rate_block(state: &mut [u64; 25], block: &[u8]) {
    for (lane, chunk) in state
        .iter_mut()
        .take(RATE_BYTES / 8)
        .zip(block.chunks_exact(8))
    {
        let mut bytes = [0_u8; 8];
        bytes.copy_from_slice(chunk);
        *lane ^= u64::from_le_bytes(bytes);
    }
}

pub fn keccak_f1600(state: &mut [u64; 25]) {
    let mut c = [0_u64; 5];
    let mut d = [0_u64; 5];
    let mut b = [0_u64; 25];

    for &round_constant in &ROUND_CONSTANTS {
        for x in 0..5 {
            c[x] = state[lane_index(x, 0)]
                ^ state[lane_index(x, 1)]
                ^ state[lane_index(x, 2)]
                ^ state[lane_index(x, 3)]
                ^ state[lane_index(x, 4)];
        }

        for x in 0..5 {
            d[x] = c[(x + 4) % 5] ^ c[(x + 1) % 5].rotate_left(1);
        }

        for x in 0..5 {
            for y in 0..5 {
                state[lane_index(x, y)] ^= d[x];
            }
        }

        for x in 0..5 {
            for y in 0..5 {
                let source = lane_index(x, y);
                let dest = lane_index(y, (2 * x + 3 * y) % 5);
                b[dest] = state[source].rotate_left(ROTATION_OFFSETS[source]);
            }
        }

        for x in 0..5 {
            for y in 0..5 {
                let index = lane_index(x, y);
                let next = lane_index((x + 1) % 5, y);
                let next_next = lane_index((x + 2) % 5, y);
                state[index] = b[index] ^ ((!b[next]) & b[next_next]);
            }
        }

        state[0] ^= round_constant;
    }
}

pub fn keccak256(input: &[u8]) -> [u8; 32] {
    let mut state = [0_u64; 25];
    let mut offset = 0;

    while offset + RATE_BYTES <= input.len() {
        xor_rate_block(&mut state, &input[offset..offset + RATE_BYTES]);
        keccak_f1600(&mut state);
        offset += RATE_BYTES;
    }

    let mut block = [0_u8; RATE_BYTES];
    let remaining = input.len() - offset;
    block[..remaining].copy_from_slice(&input[offset..]);
    block[remaining] = 0x01;
    block[RATE_BYTES - 1] |= 0x80;

    xor_rate_block(&mut state, &block);
    keccak_f1600(&mut state);

    let mut out = [0_u8; 32];
    for (index, lane) in state.iter().take(4).enumerate() {
        out[index * 8..(index + 1) * 8].copy_from_slice(&lane.to_le_bytes());
    }

    out
}
