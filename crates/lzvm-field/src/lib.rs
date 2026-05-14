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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Felt(u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldError {
    NonCanonical { value: u64 },
}

impl fmt::Display for FieldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonCanonical { value } => write!(f, "non-canonical field element: {value}"),
        }
    }
}

impl std::error::Error for FieldError {}

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
