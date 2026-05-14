use std::fmt;
use std::ops::{Add, Mul, Sub};

pub const MODULUS: u64 = 0xffff_ffff_0000_0001;

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

impl Mul for Felt {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self(((self.0 as u128 * rhs.0 as u128) % MODULUS as u128) as u64)
    }
}
