use std::fmt;
use std::ops::{Add, Mul, Neg, Sub};

pub const MODULUS: u64 = 0xffff_ffff_0000_0001;
pub const SHIFT: Felt = Felt(7);

const MODULUS_U128: u128 = MODULUS as u128;
const GOLDILOCKS_EPSILON: u64 = 0xffff_ffff;

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

const POSEIDON2_WIDTH_4_DIAG: [u64; 4] = [
    0xf0ce_126f_e8a8_3094,
    0x60f8_7e0b_59fb_4ee6,
    0xa810_6c22_1cd6_d882,
    0x5529_eddc_46e3_72e7,
];

const POSEIDON2_WIDTH_4_ROUND_CONSTANTS: [u64; 53] = [
    0x5098_165e_e28e_503e,
    0x41b8_4edf_ee6c_0590,
    0xdda6_bc08_1661_f7b8,
    0xb56f_892b_5fc6_d76c,
    0xb2b7_e92b_1f70_399f,
    0x7075_cc44_0425_36e9,
    0xd5aa_e31b_4968_adb1,
    0x0713_f06e_b5e4_0337,
    0x80dc_cd8a_419c_c2d5,
    0x89ae_3f75_c9b5_3e2c,
    0x8aac_5449_eff2_7e1d,
    0xef29_b2b2_4bf5_03f9,
    0xa1d4_f9ea_aa62_e9fc,
    0x2f21_5d5c_5a0a_a622,
    0x7b34_47f3_4ae2_2dd9,
    0x4b61_4218_a8e8_1eef,
    0xe063_3431_14e0_f434,
    0x2cde_df7f_0717_ad4e,
    0x4662_c297_f253_7cf5,
    0x8fe4_8eee_5176_1f3d,
    0x616a_ead4_ae0e_bf00,
    0x9b40_b730_22b3_089b,
    0xa051_e164_6094_b036,
    0xf69b_2c13_f377_ff8e,
    0x96f7_dec4_549a_f9be,
    0x8583_7168_6234_c707,
    0x8483_ec4d_5e3e_8114,
    0x21ae_a04a_4066_e649,
    0xbed2_1bd9_5c72_ec7e,
    0x9486_55aa_fad4_b757,
    0xd4b2_ed65_7358_23e2,
    0x1930_ef5f_54c4_0462,
    0xb3cc_1696_b1d3_811e,
    0xafe0_3360_7720_2599,
    0x11da_6a90_6ef6_6e3e,
    0xd7ab_df7d_347f_b43f,
    0x65e7_d3c9_f0e8_da86,
    0x0b73_bdaf_ed7f_79f4,
    0x619b_24eb_14c2_9f0f,
    0x8590_4bd8_db9e_3cd9,
    0x4c9c_28e6_73ab_b589,
    0x73b2_0f64_3717_949f,
    0x832a_b3fa_a2c0_639a,
    0xfa1d_702b_afb6_5207,
    0x03f5_f17b_0409_003c,
    0x2c3f_f110_b39f_84d5,
    0x4cdf_d3ff_34ce_6f4f,
    0xd3ac_f580_7f20_8db4,
    0x13d2_8634_ce48_e600,
    0xb065_f0e6_67d7_caf9,
    0x44f6_f3d6_b128_25ca,
    0x243a_64c0_3f36_ea35,
    0x470a_3b7c_2f6a_6a7a,
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

const POSEIDON2_WIDTH_16_DIAG: [u64; 16] = [
    0xde9b_91a4_67d6_afc0,
    0xc5f1_6b9c_76a9_be17,
    0x0ab0_fef2_d540_ac55,
    0x3001_d270_09d0_5773,
    0xed23_b1f9_06d3_d9eb,
    0x5ce7_3743_cba9_7054,
    0x1c3b_ab94_4af4_ba24,
    0x2faa_1058_54db_afae,
    0x53ff_b3ae_6d42_1a10,
    0xbcda_9df8_884b_a396,
    0xfc12_73e4_a318_07bb,
    0xc779_5257_3d51_42c0,
    0x5668_3339_a819_b85e,
    0x328f_cbd8_f0dd_c8eb,
    0xb510_1e30_3fce_9cb7,
    0x7744_87b8_c400_89bb,
];

const POSEIDON2_WIDTH_16_ROUND_CONSTANTS: [u64; 150] = [
    0x15eb_ea3f_c733_97c3,
    0xd73c_d9fb_fe8e_275c,
    0x8c09_6bfc_e77f_6c26,
    0x4e12_8f68_b53d_8fea,
    0x29b7_79a3_6b27_63f6,
    0xfe2a_dc6f_b65a_cd08,
    0x8d25_20e7_25ad_0955,
    0x1c23_92b2_1462_4d2a,
    0x3748_2118_206d_cc6e,
    0x2f82_9bed_19be_019a,
    0x2fe2_98cb_6f81_59b0,
    0x2bba_d982_decc_dbbf,
    0xbad5_68b8_cc60_a81e,
    0xb86a_8142_65ba_ad10,
    0xbec2_0055_13b3_acb3,
    0x6bf8_9b59_a07c_2a94,
    0xa25d_eeb8_35e2_30f5,
    0x3c5b_ad85_12b8_b12a,
    0x7230_f73c_3cb7_a4f2,
    0xa70c_87f0_95c7_4d0f,
    0x6b76_06b8_30bb_2e80,
    0x6cd4_67cf_c4f2_4274,
    0xfeed_794d_f42a_9b0a,
    0x8cf7_cf61_63b7_dbd3,
    0x9a6e_9dda_5971_75a0,
    0xaa52_295a_684f_af7b,
    0x017b_811c_c358_9d8d,
    0x55bf_b699_b618_1648,
    0xc2cc_af71_501c_2421,
    0x1707_9503_2759_6402,
    0xdd2f_cdcd_42a8_229f,
    0x8b9d_7d5b_2777_8a21,
    0xac9a_0552_5f9c_f512,
    0x2ba1_25c5_8627_b5e8,
    0xc74e_9125_0a81_47a5,
    0xa3e6_4b64_0d5b_b384,
    0xf530_47d1_8d1f_9292,
    0xbaae_ddac_ae3a_6374,
    0xf2d0_914a_808b_3db1,
    0x18af_1a37_42bf_a3b0,
    0x9a62_1ef5_0c55_bdb8,
    0xc615_f4d1_cc54_66f3,
    0xb7fb_ac19_a35c_f793,
    0xd2b1_a15b_a517_e46d,
    0x4a29_0c4d_7fd2_6f6f,
    0x4f0c_f1bb_1770_c4c4,
    0x5483_4538_6cd3_77f5,
    0x3397_8d27_89fd_dd42,
    0xab78_c59d_eb77_e211,
    0xc485_b2a9_33d2_be7f,
    0xbde3_792c_00c0_3c53,
    0xab4c_efe8_f893_d247,
    0xc5c0_e752_eab7_f85f,
    0xdbf5_a76f_893b_afea,
    0xa91f_6003_e3d9_84de,
    0x0995_3907_7f31_1e87,
    0x097e_c522_32f9_559e,
    0x5364_1bdf_8991_e48c,
    0x2afe_9711_d5ed_9d7c,
    0xa7b1_3d36_61b5_d117,
    0x5a0e_243f_e7af_6556,
    0x1076_fae8_932d_5f00,
    0x9b53_a83d_4349_34e3,
    0xed3f_d595_a3c0_344a,
    0x28ef_f4b0_1103_d100,
    0x6040_0ca3_e268_5a45,
    0x1c86_36be_b338_9b84,
    0xac13_32b6_0e13_eff0,
    0x2ada_fcc3_64e2_0f87,
    0x79ff_c2b1_4054_ea0b,
    0x3f98_e4c0_908f_0a05,
    0xcdb2_30bc_4e8a_06c4,
    0x1bca_f770_5b15_2a74,
    0xd9bc_a249_a82a_7470,
    0x91e2_4af1_9bf8_2551,
    0xa62b_43ba_5cb7_8858,
    0xb489_8117_472e_797f,
    0xb322_8bca_606c_daa0,
    0x8444_6105_1bca_39c9,
    0xf341_1581_f661_7d68,
    0xf7fd_5064_6782_b533,
    0x6ca6_6425_3c18_fb48,
    0x2d2f_cdec_0886_a08f,
    0x29da_00dd_799b_575e,
    0x47d9_66cc_3b6e_1e93,
    0xde88_4e9a_17ce_d59e,
    0xdacf_46dc_1c31_a045,
    0x5d2e_3c12_1eb3_87f2,
    0x51f8_b065_8b12_4499,
    0x1e7d_bd1d_aa72_167d,
    0x8275_015a_25c5_5b88,
    0xe852_1c24_ac7a_70b3,
    0x6521_d121_c40b_3f67,
    0xac12_de79_7de1_35b0,
    0xafa2_8ead_79f6_ed6a,
    0x6851_74a7_a8d2_6f0b,
    0xeff9_2a08_d35d_9874,
    0x3058_734b_76dd_123a,
    0xfa55_dcfb_a429_f79c,
    0x5592_94d4_324c_7728,
    0x7a77_0f53_012d_c178,
    0xedd8_f7c4_08f3_883b,
    0x39b5_33cf_8d79_5fa5,
    0x160e_f9de_243a_8c0a,
    0x431d_52da_6215_fe3f,
    0x54c5_1a2a_2ef6_d528,
    0x9b13_892b_46ff_9d16,
    0x263c_46fc_ee21_0289,
    0xb738_c96d_25aa_bdc4,
    0x5c33_a520_3996_d38f,
    0x2626_496e_7c98_d8dd,
    0xc669_e0a5_2785_903a,
    0xaecd_e726_c8ae_1f47,
    0x0393_43ef_3a81_e999,
    0x2615_ceaf_044a_54f9,
    0x7e41_e834_662b_66e1,
    0x4ca5_fd48_9533_5783,
    0x64b3_34d0_2916_f2b0,
    0x8726_8837_389a_6981,
    0x034b_75bc_b20a_6274,
    0x58e6_5829_6cc2_cd6e,
    0xe2d0_f759_acc3_1df4,
    0x81a6_52e4_3509_3e20,
    0x0b72_b6e0_172e_af47,
    0x4aec_43ce_c577_d66d,
    0xde78_365b_028a_84e6,
    0x444e_1956_9adc_0ee4,
    0x942b_2451_fa40_d1da,
    0xe245_0662_3ea5_bd6c,
    0x0828_54bf_2ef7_c743,
    0x69db_bc56_6f59_d62e,
    0x248c_38d0_2a7b_5cb2,
    0x4f4e_8f8c_09d1_5edb,
    0xd966_82f1_88d3_10cf,
    0x6f9a_25d5_6818_b54c,
    0xb6ce_fed6_0654_6cd9,
    0x5bc0_7523_da38_a67b,
    0x7df5_a3c3_5b81_11cf,
    0xaaa2_cc5d_4db3_4bb0,
    0x9e67_3ff2_2a46_53f8,
    0xbd8b_278d_6073_9c62,
    0xe10d_20f6_925b_8815,
    0xf6c8_7b91_dd4d_a2bf,
    0xfed6_23e2_f71b_6f1a,
    0xa0f0_2fa5_2a94_d0d3,
    0xbb57_9471_1b39_fa16,
    0xd3b9_4fba_9d00_5c7f,
    0x15a2_6e89_fad9_46c9,
    0xf3cb_87db_8a67_cf49,
    0x400d_2bf5_6aa2_a577,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TranscriptError {
    UnsupportedArity { arity: usize },
    InvalidPermutationBits { bits: u32 },
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

impl fmt::Display for TranscriptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedArity { arity } => {
                write!(f, "unsupported transcript arity: {arity}")
            }
            Self::InvalidPermutationBits { bits } => {
                write!(f, "invalid transcript permutation bits: {bits}")
            }
            Self::LengthOverflow => write!(f, "transcript length overflow"),
        }
    }
}

impl std::error::Error for TranscriptError {}

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

    #[inline(always)]
    pub fn from_u64(value: u64) -> Self {
        if value >= MODULUS {
            Self(value - MODULUS)
        } else {
            Self(value)
        }
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
    let mut coefficients = interpolate_subgroup_evaluations(values, source_bits)?;
    coefficients.resize(target_len, Felt::ZERO);

    let mut shift_power = Felt::ONE;
    for coefficient in &mut coefficients {
        *coefficient = *coefficient * shift_power;
        shift_power = shift_power * SHIFT;
    }
    ntt_in_place(&mut coefficients, target_bits)?;
    Ok(coefficients)
}

pub fn poseidon2_hash_8(input: [Felt; 8]) -> [Felt; 8] {
    poseidon2_hash(
        input,
        &POSEIDON2_WIDTH_8_DIAG,
        &POSEIDON2_WIDTH_8_ROUND_CONSTANTS,
    )
}

pub fn poseidon2_hash_4(input: [Felt; 4]) -> [Felt; 4] {
    poseidon2_hash(
        input,
        &POSEIDON2_WIDTH_4_DIAG,
        &POSEIDON2_WIDTH_4_ROUND_CONSTANTS,
    )
}

pub fn poseidon2_hash_16(input: [Felt; 16]) -> [Felt; 16] {
    poseidon2_hash(
        input,
        &POSEIDON2_WIDTH_16_DIAG,
        &POSEIDON2_WIDTH_16_ROUND_CONSTANTS,
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoseidonTranscript {
    arity: usize,
    width: usize,
    pending_size: usize,
    state: [Felt; 4],
    pending: Vec<Felt>,
    out: Vec<Felt>,
    pending_cursor: usize,
    out_cursor: usize,
}

impl PoseidonTranscript {
    pub fn new(arity: usize) -> Result<Self, TranscriptError> {
        match arity {
            2 | 4 => {
                let width = arity
                    .checked_mul(4)
                    .ok_or(TranscriptError::LengthOverflow)?;
                let pending_size = arity
                    .checked_sub(1)
                    .and_then(|value| value.checked_mul(4))
                    .ok_or(TranscriptError::LengthOverflow)?;
                Ok(Self {
                    arity,
                    width,
                    pending_size,
                    state: [Felt::ZERO; 4],
                    pending: vec![Felt::ZERO; pending_size],
                    out: vec![Felt::ZERO; width],
                    pending_cursor: 0,
                    out_cursor: 0,
                })
            }
            _ => Err(TranscriptError::UnsupportedArity { arity }),
        }
    }

    pub fn put(&mut self, values: &[Felt]) {
        for value in values {
            self.add_scalar(*value);
        }
    }

    pub fn get_state(&mut self) -> [Felt; 4] {
        if self.pending_cursor > 0 {
            self.update_state();
        }
        self.state
    }

    pub fn get_state_words(&mut self) -> Vec<Felt> {
        if self.pending_cursor > 0 {
            self.update_state();
        }
        self.out.clone()
    }

    pub fn get_field(&mut self) -> Ext3 {
        Ext3::new(self.get_scalar(), self.get_scalar(), self.get_scalar())
    }

    pub fn get_permutations(
        &mut self,
        count: usize,
        bits: u32,
    ) -> Result<Vec<u64>, TranscriptError> {
        if bits == 0 || bits >= 64 {
            return Err(TranscriptError::InvalidPermutationBits { bits });
        }
        if count == 0 {
            return Ok(Vec::new());
        }
        let total_bits = count
            .checked_mul(usize::try_from(bits).map_err(|_| TranscriptError::LengthOverflow)?)
            .ok_or(TranscriptError::LengthOverflow)?;
        let field_count = total_bits
            .checked_add(62)
            .ok_or(TranscriptError::LengthOverflow)?
            / 63;
        let mut fields = Vec::with_capacity(field_count);
        for _ in 0..field_count {
            fields.push(self.get_scalar().to_u64());
        }

        let mut field_index = 0_usize;
        let mut bit_index = 0_u32;
        let mut out = Vec::with_capacity(count);
        for _ in 0..count {
            let mut sample = 0_u64;
            for output_bit in 0..bits {
                let bit = (fields[field_index] >> bit_index) & 1;
                if bit == 1 {
                    sample |= 1_u64 << output_bit;
                }
                bit_index += 1;
                if bit_index == 63 {
                    bit_index = 0;
                    field_index += 1;
                }
            }
            out.push(sample);
        }
        Ok(out)
    }

    fn add_scalar(&mut self, value: Felt) {
        self.pending[self.pending_cursor] = value;
        self.pending_cursor += 1;
        self.out_cursor = 0;
        if self.pending_cursor == self.pending_size {
            self.update_state();
        }
    }

    fn get_scalar(&mut self) -> Felt {
        if self.out_cursor == 0 {
            self.update_state();
        }
        let index = (self.width - self.out_cursor) % self.width;
        let value = self.out[index];
        self.out_cursor -= 1;
        value
    }

    fn update_state(&mut self) {
        while self.pending_cursor < self.pending_size {
            self.pending[self.pending_cursor] = Felt::ZERO;
            self.pending_cursor += 1;
        }
        let hash = match self.arity {
            2 => {
                let mut input = [Felt::ZERO; 8];
                input[..self.pending_size].copy_from_slice(&self.pending);
                input[self.pending_size..self.width].copy_from_slice(&self.state);
                poseidon2_hash_8(input).to_vec()
            }
            4 => {
                let mut input = [Felt::ZERO; 16];
                input[..self.pending_size].copy_from_slice(&self.pending);
                input[self.pending_size..self.width].copy_from_slice(&self.state);
                poseidon2_hash_16(input).to_vec()
            }
            _ => unreachable!("unsupported arities are rejected at construction"),
        };
        self.out = hash;
        self.out_cursor = self.width;
        self.pending.fill(Felt::ZERO);
        self.pending_cursor = 0;
        self.state.copy_from_slice(&self.out[..4]);
    }
}

fn poseidon2_hash<const WIDTH: usize>(
    input: [Felt; WIDTH],
    diagonal: &[u64; WIDTH],
    round_constants: &[u64],
) -> [Felt; WIDTH] {
    const HALF_ROUNDS: usize = 4;
    let partial_rounds = round_constants.len() - (2 * HALF_ROUNDS * WIDTH);

    let mut state = input;
    poseidon2_matmul_external(&mut state);

    for round in 0..HALF_ROUNDS {
        let offset = round * WIDTH;
        poseidon2_pow7add(&mut state, &round_constants[offset..offset + WIDTH]);
        poseidon2_matmul_external(&mut state);
    }

    let partial_offset = HALF_ROUNDS * WIDTH;
    for round in 0..partial_rounds {
        state[0] =
            poseidon2_pow7(state[0] + Felt::from_u64(round_constants[partial_offset + round]));
        let sum = state
            .iter()
            .copied()
            .fold(Felt::ZERO, |acc, value| acc + value);
        for (index, value) in state.iter_mut().enumerate() {
            *value = *value * Felt::from_u64(diagonal[index]) + sum;
        }
    }

    let final_offset = HALF_ROUNDS * WIDTH + partial_rounds;
    for round in 0..HALF_ROUNDS {
        let offset = final_offset + round * WIDTH;
        poseidon2_pow7add(&mut state, &round_constants[offset..offset + WIDTH]);
        poseidon2_matmul_external(&mut state);
    }

    state
}

fn poseidon2_pow7(value: Felt) -> Felt {
    let square = value * value;
    let fourth = square * square;
    fourth * square * value
}

fn poseidon2_pow7add<const WIDTH: usize>(state: &mut [Felt; WIDTH], constants: &[u64]) {
    for (value, constant) in state.iter_mut().zip(constants) {
        *value = poseidon2_pow7(*value + Felt::from_u64(*constant));
    }
}

fn poseidon2_matmul_external<const WIDTH: usize>(state: &mut [Felt; WIDTH]) {
    for chunk in state.chunks_exact_mut(4) {
        poseidon2_matmul_m4(chunk);
    }

    if WIDTH == 4 {
        return;
    }

    let mut stored = [Felt::ZERO; 4];
    for chunk in state.chunks_exact(4) {
        for (index, value) in chunk.iter().enumerate() {
            stored[index] = stored[index] + *value;
        }
    }

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

    bit_reverse_permute(values);
    let mut len = 2;
    while len <= n {
        let stage_exponent = u64::try_from(n / len).map_err(|_| DomainError::LengthOverflow)?;
        let stage_twiddle = twiddle.pow(stage_exponent);
        for chunk in values.chunks_exact_mut(len) {
            let mut factor = Felt::ONE;
            for index in 0..(len / 2) {
                let even = chunk[index];
                let odd = chunk[index + len / 2] * factor;
                chunk[index] = even + odd;
                chunk[index + len / 2] = even - odd;
                factor = factor * stage_twiddle;
            }
        }
        len = len.checked_mul(2).ok_or(DomainError::LengthOverflow)?;
    }

    if inverse {
        let n_u64 = u64::try_from(n).map_err(|_| DomainError::LengthOverflow)?;
        let inv_n = Felt::from_u64(n_u64)
            .inverse()
            .expect("domain length is nonzero");
        for value in values {
            *value = *value * inv_n;
        }
    }
    Ok(())
}

fn bit_reverse_permute(values: &mut [Felt]) {
    let bits = values.len().trailing_zeros();
    for index in 0..values.len() {
        let reverse = index.reverse_bits() >> (usize::BITS - bits);
        if index < reverse {
            values.swap(index, reverse);
        }
    }
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

impl Add for Felt {
    type Output = Self;

    #[inline(always)]
    fn add(self, rhs: Self) -> Self::Output {
        let sum = self.0 as u128 + rhs.0 as u128;
        if sum >= MODULUS_U128 {
            Self((sum - MODULUS_U128) as u64)
        } else {
            Self(sum as u64)
        }
    }
}

impl Sub for Felt {
    type Output = Self;

    #[inline(always)]
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

    #[inline(always)]
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

    #[inline(always)]
    fn mul(self, rhs: Self) -> Self::Output {
        Self(reduce_goldilocks_product(self.0 as u128 * rhs.0 as u128))
    }
}

#[inline(always)]
fn reduce_goldilocks_product(value: u128) -> u64 {
    let lo = value as u64;
    let hi = (value >> 64) as u64;
    let hi_hi = hi >> 32;
    let hi_lo = hi & GOLDILOCKS_EPSILON;

    let (mut reduced, borrow) = lo.overflowing_sub(hi_hi);
    if borrow {
        reduced = reduced.wrapping_sub(GOLDILOCKS_EPSILON);
    }

    reduced = add_wrapping_modulus(reduced, hi_lo * GOLDILOCKS_EPSILON);
    if reduced >= MODULUS {
        reduced -= MODULUS;
    }
    reduced
}

#[inline(always)]
fn add_wrapping_modulus(lhs: u64, rhs: u64) -> u64 {
    let (sum, carry) = lhs.overflowing_add(rhs);
    if carry {
        sum.wrapping_add(GOLDILOCKS_EPSILON)
    } else {
        sum
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
