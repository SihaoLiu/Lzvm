use std::fmt;
use std::ops::{Add, Mul, Neg, Sub};

pub const MODULUS: u64 = 0xffff_ffff_0000_0001;
pub const SHIFT: Felt = Felt(7);

const ROOTS_OF_UNITY: [u64; 33] = [
    1,
    18_446_744_069_414_584_320,
    281_474_976_710_656,
    16_777_216,
    4096,
    64,
    8,
    2_198_989_700_608,
    4_404_853_092_538_523_347,
    6_434_636_298_004_421_797,
    4_255_134_452_441_852_017,
    9_113_133_275_150_391_358,
    4_355_325_209_153_869_931,
    4_308_460_244_895_131_701,
    7_126_024_226_993_609_386,
    1_873_558_160_482_552_414,
    8_167_150_655_112_846_419,
    5_718_075_921_287_398_682,
    3_411_401_055_030_829_696,
    8_982_441_859_486_529_725,
    1_971_462_654_193_939_361,
    6_553_637_399_136_210_105,
    8_124_823_329_697_072_476,
    5_936_499_541_590_631_774,
    2_709_866_199_236_980_323,
    8_877_499_657_461_974_390,
    3_757_607_247_483_852_735,
    4_969_973_714_567_017_225,
    2_147_253_751_702_802_259,
    2_530_564_950_562_219_707,
    1_905_180_297_017_055_339,
    3_524_815_499_551_269_279,
    7_277_203_076_849_721_926,
];

const POSEIDON2_WIDTH_8_DIAG: [u64; 8] = [
    0xa988_11a1_fed4_e3a5,
    0x1cc4_8b54_f377_e2a0,
    0xe40c_d4f6_c560_9a26,
    0x11de_79eb_ca97_a4a3,
    0x9177_c73d_8b7e_929c,
    0x2a6f_e808_5797_e791,
    0x3de6_e933_29f8_d5ad,
    0x3f7a_f912_5da9_62fe,
];

const POSEIDON2_WIDTH_8_ROUND_CONSTANTS: [u64; 86] = [
    0xdd57_43e7_f2a5_a5d9,
    0xcb3a_864e_58ad_a44b,
    0xffa2_449e_d32f_8cdc,
    0x4202_5f65_d6bd_13ee,
    0x7889_175e_2550_6323,
    0x34b9_8bb0_3d24_b737,
    0xbdcc_535e_cc4f_aa2a,
    0x5b20_ad86_9fc0_d033,
    0xf1dd_a5b9_259d_fcb4,
    0x2751_5210_be11_2d59,
    0x4227_d171_8c76_6c3f,
    0x26d3_3316_1a5b_d794,
    0x49b9_3895_7bf4_b026,
    0x4a56_b593_8b21_3669,
    0x1120_426b_48c8_353d,
    0x6b32_3c3f_10a5_6cad,
    0xce57_d624_5ddc_a6b2,
    0xb1fc_8d40_2bba_1eb1,
    0xb5c5_096c_a959_bd04,
    0x6db5_5cd3_06d3_1f7f,
    0xc49d_293a_81cb_9641,
    0x1ce5_5a4f_e979_719f,
    0xa92e_60a9_d178_a4d1,
    0x002c_c649_73bc_fd8c,
    0xcea7_21cc_e82f_b11b,
    0xe5b5_5eb8_098e_ce81,
    0x4e30_525c_6f1d_dd66,
    0x43c6_7028_2707_0987,
    0xaca6_8430_a7b5_762a,
    0x3674_2386_34df_9c93,
    0x88ce_e1c8_25e3_3433,
    0xde99_ae8d_74b5_7176,
    0x4888_97d8_5ff5_1f56,
    0x1140_737c_cb16_2218,
    0xa7ee_b921_5866_ed35,
    0x9bd2_976f_ee49_fcc9,
    0xc0c8_f0de_580a_3fcc,
    0x4fb2_dae6_ee8f_c793,
    0x343a_89f3_5f37_395b,
    0x223b_525a_77ca_72c8,
    0x56cc_b625_74aa_a918,
    0xc4d5_07d8_027a_f9ed,
    0xa080_673c_f0b7_e95c,
    0xf018_4884_eb70_dcf8,
    0x044f_10b0_cb3d_5c69,
    0xe9e3_f799_3938_f186,
    0x1b76_1c80_e772_f459,
    0x606c_ec60_7a1b_5fac,
    0x14a0_c2e1_d45f_03cd,
    0x4eac_e885_5398_574f,
    0xf905_ca71_03ef_f3e6,
    0xf8c8_f8d2_0862_c059,
    0xb524_fe8b_dd67_8e5a,
    0xfbb7_8659_01a1_ec41,
    0x014e_f119_7d34_1346,
    0x9725_e208_25d0_7394,
    0xfdb2_5aef_2c5b_ae3b,
    0xbe54_02dc_598c_971e,
    0x93a5_711f_04cd_ca3d,
    0xc45a_9a5b_2f8f_b97b,
    0xfe89_46a9_2493_3545,
    0x2af9_97a2_7369_091c,
    0xaa62_c88e_0b29_4011,
    0x058e_b9d8_10ce_9f74,
    0xb3cb_23ec_ed34_9ae4,
    0xa364_8177_a77b_4a84,
    0x4315_3d90_5992_d95d,
    0xf4e2_a97c_da44_aa4b,
    0x5baa_2702_b908_682f,
    0x0829_23bd_f4f7_50d1,
    0x98ae_09a3_2589_3803,
    0xf8a6_4750_7796_8838,
    0xceb0_735b_f00b_2c5f,
    0x0a1a_5d95_3888_e072,
    0x2fcb_1904_89f9_4475,
    0xb5be_0627_0dec_69fc,
    0x739c_b934_b09a_cf8b,
    0x5377_50b7_5ec7_f25b,
    0xe9dd_318b_ae1f_3961,
    0xf746_2137_299e_fe1a,
    0xb1f6_b8ee_e9ad_b940,
    0xbdeb_cc8a_809d_fe6b,
    0x40fc_1f79_1b17_8113,
    0x3ac1_c336_2d01_4864,
    0x9a01_6184_bdb8_aeba,
    0x95f2_3944_59fb_c25e,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Felt(u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldError {
    NonCanonical { value: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainError {
    UnsupportedBits {
        bits: usize,
        max_bits: usize,
    },
    LengthMismatch {
        expected: usize,
        found: usize,
    },
    InvalidExtensionBits {
        source_bits: usize,
        target_bits: usize,
    },
    LengthOverflow,
}

impl fmt::Display for FieldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonCanonical { value } => write!(f, "non-canonical field element: {value}"),
        }
    }
}

impl std::error::Error for FieldError {}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedBits { bits, max_bits } => {
                write!(f, "unsupported field domain bits {bits}, max {max_bits}")
            }
            Self::LengthMismatch { expected, found } => {
                write!(
                    f,
                    "field domain length mismatch: expected {expected}, found {found}"
                )
            }
            Self::InvalidExtensionBits {
                source_bits,
                target_bits,
            } => write!(
                f,
                "invalid field extension bits: source {source_bits}, target {target_bits}"
            ),
            Self::LengthOverflow => write!(f, "field domain length overflow"),
        }
    }
}

impl std::error::Error for DomainError {}

impl Felt {
    pub const ZERO: Self = Self(0);
    pub const ONE: Self = Self(1);

    pub fn from_canonical(value: u64) -> Result<Self, FieldError> {
        if value < MODULUS {
            Ok(Self(value))
        } else {
            Err(FieldError::NonCanonical { value })
        }
    }

    pub fn from_u64(value: u64) -> Self {
        Self((value as u128 % MODULUS as u128) as u64)
    }

    pub fn to_u64(self) -> u64 {
        self.0
    }

    pub fn to_le_bytes(self) -> [u8; 8] {
        self.0.to_le_bytes()
    }

    pub fn from_le_bytes(bytes: [u8; 8]) -> Self {
        Self::from_u64(u64::from_le_bytes(bytes))
    }

    pub fn pow(self, mut exponent: u64) -> Self {
        let mut base = self;
        let mut result = Self::ONE;
        while exponent > 0 {
            if exponent & 1 == 1 {
                result = result * base;
            }
            base = base * base;
            exponent >>= 1;
        }
        result
    }

    pub fn inverse(self) -> Option<Self> {
        if self == Self::ZERO {
            None
        } else {
            Some(self.pow(MODULUS - 2))
        }
    }

    pub fn root_of_unity(bits: usize) -> Option<Self> {
        ROOTS_OF_UNITY.get(bits).copied().map(Self)
    }
}

pub fn ntt_in_place(values: &mut [Felt], bits: usize) -> Result<(), DomainError> {
    transform_in_place(values, bits, false)
}

pub fn intt_in_place(values: &mut [Felt], bits: usize) -> Result<(), DomainError> {
    transform_in_place(values, bits, true)
}

pub fn interpolate_subgroup_evaluations(
    values: &[Felt],
    bits: usize,
) -> Result<Vec<Felt>, DomainError> {
    let mut coefficients = values.to_vec();
    intt_in_place(&mut coefficients, bits)?;
    Ok(coefficients)
}

pub fn evaluate_polynomial(coefficients: &[Felt], point: Felt) -> Felt {
    coefficients
        .iter()
        .rev()
        .fold(Felt::ZERO, |acc, coefficient| acc * point + *coefficient)
}

pub fn coset_extend_evaluations(
    values: &[Felt],
    source_bits: usize,
    target_bits: usize,
) -> Result<Vec<Felt>, DomainError> {
    if target_bits < source_bits {
        return Err(DomainError::InvalidExtensionBits {
            source_bits,
            target_bits,
        });
    }
    validate_domain_len(values.len(), source_bits)?;
    let target_len = domain_len(target_bits)?;
    let target_root = domain_root(target_bits)?;
    let coefficients = interpolate_subgroup_evaluations(values, source_bits)?;

    let mut out = Vec::with_capacity(target_len);
    let mut power = Felt::ONE;
    for _ in 0..target_len {
        out.push(evaluate_polynomial(&coefficients, SHIFT * power));
        power = power * target_root;
    }
    Ok(out)
}

pub fn poseidon2_hash_8(input: [Felt; 8]) -> [Felt; 8] {
    const WIDTH: usize = 8;
    const HALF_ROUNDS: usize = 4;
    const PARTIAL_ROUNDS: usize = 22;

    let mut state = input;
    poseidon2_matmul_external_8(&mut state);

    for round in 0..HALF_ROUNDS {
        let offset = round * WIDTH;
        poseidon2_pow7add_8(
            &mut state,
            &POSEIDON2_WIDTH_8_ROUND_CONSTANTS[offset..offset + WIDTH],
        );
        poseidon2_matmul_external_8(&mut state);
    }

    let partial_offset = HALF_ROUNDS * WIDTH;
    for round in 0..PARTIAL_ROUNDS {
        state[0] = poseidon2_pow7(
            state[0] + Felt::from_u64(POSEIDON2_WIDTH_8_ROUND_CONSTANTS[partial_offset + round]),
        );
        let sum = state
            .iter()
            .copied()
            .fold(Felt::ZERO, |acc, value| acc + value);
        for (index, value) in state.iter_mut().enumerate() {
            *value = *value * Felt::from_u64(POSEIDON2_WIDTH_8_DIAG[index]) + sum;
        }
    }

    let final_offset = HALF_ROUNDS * WIDTH + PARTIAL_ROUNDS;
    for round in 0..HALF_ROUNDS {
        let offset = final_offset + round * WIDTH;
        poseidon2_pow7add_8(
            &mut state,
            &POSEIDON2_WIDTH_8_ROUND_CONSTANTS[offset..offset + WIDTH],
        );
        poseidon2_matmul_external_8(&mut state);
    }

    state
}

fn poseidon2_pow7(value: Felt) -> Felt {
    let square = value * value;
    let fourth = square * square;
    fourth * square * value
}

fn poseidon2_pow7add_8(state: &mut [Felt; 8], constants: &[u64]) {
    for (value, constant) in state.iter_mut().zip(constants) {
        *value = poseidon2_pow7(*value + Felt::from_u64(*constant));
    }
}

fn poseidon2_matmul_external_8(state: &mut [Felt; 8]) {
    poseidon2_matmul_m4(&mut state[0..4]);
    poseidon2_matmul_m4(&mut state[4..8]);
    let stored = [
        state[0] + state[4],
        state[1] + state[5],
        state[2] + state[6],
        state[3] + state[7],
    ];
    for (index, value) in state.iter_mut().enumerate() {
        *value = *value + stored[index % 4];
    }
}

fn poseidon2_matmul_m4(values: &mut [Felt]) {
    let t0 = values[0] + values[1];
    let t1 = values[2] + values[3];
    let t2 = values[1] + values[1] + t1;
    let t3 = values[3] + values[3] + t0;
    let t1_2 = t1 + t1;
    let t0_2 = t0 + t0;
    let t4 = t1_2 + t1_2 + t3;
    let t5 = t0_2 + t0_2 + t2;
    let t6 = t3 + t5;
    let t7 = t2 + t4;

    values[0] = t6;
    values[1] = t5;
    values[2] = t7;
    values[3] = t4;
}

fn transform_in_place(values: &mut [Felt], bits: usize, inverse: bool) -> Result<(), DomainError> {
    let n = validate_domain_len(values.len(), bits)?;
    let root = domain_root(bits)?;
    let twiddle = if inverse {
        root.inverse().expect("domain roots are nonzero")
    } else {
        root
    };
    let mut out = vec![Felt::ZERO; n];
    for (k, slot) in out.iter_mut().enumerate() {
        let mut acc = Felt::ZERO;
        for (j, value) in values.iter().enumerate() {
            let exponent = checked_index_product(j, k)?;
            acc = acc + *value * twiddle.pow(exponent);
        }
        *slot = acc;
    }
    if inverse {
        let n_u64 = u64::try_from(n).map_err(|_| DomainError::LengthOverflow)?;
        let inv_n = Felt::from_u64(n_u64)
            .inverse()
            .expect("domain length is nonzero");
        for value in &mut out {
            *value = *value * inv_n;
        }
    }
    values.copy_from_slice(&out);
    Ok(())
}

fn domain_root(bits: usize) -> Result<Felt, DomainError> {
    Felt::root_of_unity(bits).ok_or(DomainError::UnsupportedBits {
        bits,
        max_bits: ROOTS_OF_UNITY.len() - 1,
    })
}

fn validate_domain_len(found: usize, bits: usize) -> Result<usize, DomainError> {
    let expected = domain_len(bits)?;
    if found != expected {
        return Err(DomainError::LengthMismatch { expected, found });
    }
    Ok(expected)
}

fn domain_len(bits: usize) -> Result<usize, DomainError> {
    if bits >= ROOTS_OF_UNITY.len() {
        return Err(DomainError::UnsupportedBits {
            bits,
            max_bits: ROOTS_OF_UNITY.len() - 1,
        });
    }
    1_usize
        .checked_shl(u32::try_from(bits).map_err(|_| DomainError::LengthOverflow)?)
        .ok_or(DomainError::LengthOverflow)
}

fn checked_index_product(a: usize, b: usize) -> Result<u64, DomainError> {
    let product = a.checked_mul(b).ok_or(DomainError::LengthOverflow)?;
    u64::try_from(product).map_err(|_| DomainError::LengthOverflow)
}

impl Add for Felt {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(((self.0 as u128 + rhs.0 as u128) % MODULUS as u128) as u64)
    }
}

impl Sub for Felt {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        if self.0 >= rhs.0 {
            Self(self.0 - rhs.0)
        } else {
            Self(MODULUS - (rhs.0 - self.0))
        }
    }
}

impl Neg for Felt {
    type Output = Self;

    fn neg(self) -> Self::Output {
        if self == Self::ZERO {
            Self::ZERO
        } else {
            Self(MODULUS - self.0)
        }
    }
}

impl Mul for Felt {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self(((self.0 as u128 * rhs.0 as u128) % MODULUS as u128) as u64)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Ext3 {
    pub c0: Felt,
    pub c1: Felt,
    pub c2: Felt,
}

impl Ext3 {
    pub const ZERO: Self = Self {
        c0: Felt::ZERO,
        c1: Felt::ZERO,
        c2: Felt::ZERO,
    };
    pub const ONE: Self = Self {
        c0: Felt::ONE,
        c1: Felt::ZERO,
        c2: Felt::ZERO,
    };

    pub const fn new(c0: Felt, c1: Felt, c2: Felt) -> Self {
        Self { c0, c1, c2 }
    }

    pub fn from_u64s(values: [u64; 3]) -> Self {
        Self {
            c0: Felt::from_u64(values[0]),
            c1: Felt::from_u64(values[1]),
            c2: Felt::from_u64(values[2]),
        }
    }

    pub fn to_u64s(self) -> [u64; 3] {
        [self.c0.to_u64(), self.c1.to_u64(), self.c2.to_u64()]
    }

    pub fn pow(self, mut exponent: u64) -> Self {
        let mut base = self;
        let mut result = Self::ONE;
        while exponent > 0 {
            if exponent & 1 == 1 {
                result = result * base;
            }
            base = base * base;
            exponent >>= 1;
        }
        result
    }

    pub fn inverse(self) -> Option<Self> {
        if self == Self::ZERO {
            return None;
        }

        let aa = self.c0 * self.c0;
        let ac = self.c0 * self.c2;
        let ba = self.c1 * self.c0;
        let bb = self.c1 * self.c1;
        let bc = self.c1 * self.c2;
        let cc = self.c2 * self.c2;

        let aaa = aa * self.c0;
        let aac = aa * self.c2;
        let abc = ba * self.c2;
        let abb = ba * self.c1;
        let acc = ac * self.c2;
        let bbb = bb * self.c1;
        let bcc = bc * self.c2;
        let ccc = cc * self.c2;

        let determinant = abc + abc + abc + abb - aaa - aac - aac - acc - bbb + bcc - ccc;
        let determinant_inverse = determinant.inverse()?;

        let c0 = (bc + bb - aa - ac - ac - cc) * determinant_inverse;
        let c1 = (ba - cc) * determinant_inverse;
        let c2 = (ac + cc - bb) * determinant_inverse;

        Some(Self { c0, c1, c2 })
    }
}

impl Add for Ext3 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            c0: self.c0 + rhs.c0,
            c1: self.c1 + rhs.c1,
            c2: self.c2 + rhs.c2,
        }
    }
}

impl Sub for Ext3 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            c0: self.c0 - rhs.c0,
            c1: self.c1 - rhs.c1,
            c2: self.c2 - rhs.c2,
        }
    }
}

impl Neg for Ext3 {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self {
            c0: -self.c0,
            c1: -self.c1,
            c2: -self.c2,
        }
    }
}

impl Mul for Ext3 {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        let a = (self.c0 + self.c1) * (rhs.c0 + rhs.c1);
        let b = (self.c0 + self.c2) * (rhs.c0 + rhs.c2);
        let c = (self.c1 + self.c2) * (rhs.c1 + rhs.c2);
        let d = self.c0 * rhs.c0;
        let e = self.c1 * rhs.c1;
        let f = self.c2 * rhs.c2;
        let g = d - e;

        Self {
            c0: (c + g) - f,
            c1: (((a + c) - e) - e) - d,
            c2: b - g,
        }
    }
}
