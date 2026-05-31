use std::sync::OnceLock;

use num_bigint::BigUint;
use num_traits::Zero;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Secp256k1Error {
    NonInvertibleScalar,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SecpPoint {
    x: BigUint,
    y: BigUint,
    infinity: bool,
}

impl SecpPoint {
    fn identity() -> Self {
        Self {
            x: BigUint::zero(),
            y: BigUint::zero(),
            infinity: true,
        }
    }

    pub(crate) fn from_limbs(limbs: &[u64; 8]) -> Self {
        if limbs.iter().all(|limb| *limb == 0) {
            return Self::identity();
        }
        Self {
            x: limbs_to_biguint(&limbs[..4]),
            y: limbs_to_biguint(&limbs[4..]),
            infinity: false,
        }
    }

    pub(crate) fn to_limbs(&self) -> [u64; 8] {
        if self.infinity {
            return [0; 8];
        }
        let mut limbs = [0_u64; 8];
        limbs[..4].copy_from_slice(&biguint_to_limbs::<4>(&self.x));
        limbs[4..].copy_from_slice(&biguint_to_limbs::<4>(&self.y));
        limbs
    }
}

pub(crate) fn secp256k1_point_add(
    first: &SecpPoint,
    second: &SecpPoint,
) -> Result<SecpPoint, Secp256k1Error> {
    if first.infinity {
        return Ok(second.clone());
    }
    if second.infinity {
        return Ok(first.clone());
    }
    let modulus = secp256k1_field_modulus();
    if first.x == second.x {
        if (&first.y + &second.y) % modulus == BigUint::zero() {
            return Ok(SecpPoint::identity());
        }
        return secp256k1_point_double(first);
    }
    let numerator = field_sub(&second.y, &first.y, modulus);
    let denominator = field_sub(&second.x, &first.x, modulus);
    let denominator_inv =
        mod_inv(&denominator, modulus).ok_or(Secp256k1Error::NonInvertibleScalar)?;
    let lambda = field_mul(&numerator, &denominator_inv, modulus);
    let x = field_sub(
        &field_sub(&field_square(&lambda, modulus), &first.x, modulus),
        &second.x,
        modulus,
    );
    let y = field_sub(
        &field_mul(&lambda, &field_sub(&first.x, &x, modulus), modulus),
        &first.y,
        modulus,
    );
    Ok(SecpPoint {
        x,
        y,
        infinity: false,
    })
}

pub(crate) fn secp256k1_point_double(point: &SecpPoint) -> Result<SecpPoint, Secp256k1Error> {
    if point.infinity || point.y.is_zero() {
        return Ok(SecpPoint::identity());
    }
    let modulus = secp256k1_field_modulus();
    let numerator = field_mul(
        &BigUint::from(3_u8),
        &field_square(&point.x, modulus),
        modulus,
    );
    let denominator = field_mul(&BigUint::from(2_u8), &point.y, modulus);
    let denominator_inv =
        mod_inv(&denominator, modulus).ok_or(Secp256k1Error::NonInvertibleScalar)?;
    let lambda = field_mul(&numerator, &denominator_inv, modulus);
    let x = field_sub(
        &field_square(&lambda, modulus),
        &field_mul(&BigUint::from(2_u8), &point.x, modulus),
        modulus,
    );
    let y = field_sub(
        &field_mul(&lambda, &field_sub(&point.x, &x, modulus), modulus),
        &point.y,
        modulus,
    );
    Ok(SecpPoint {
        x,
        y,
        infinity: false,
    })
}

pub(crate) fn secp256k1_double_scalar_mul(
    first_scalar: &[u64; 4],
    first_point: &SecpPoint,
    second_scalar: &[u64; 4],
    second_point: &SecpPoint,
) -> Result<SecpPoint, Secp256k1Error> {
    let mut result = SecpPoint::identity();
    for bit in (0..256).rev() {
        result = secp256k1_point_double(&result)?;
        if limb_bit(first_scalar, bit) {
            result = secp256k1_point_add(&result, first_point)?;
        }
        if limb_bit(second_scalar, bit) {
            result = secp256k1_point_add(&result, second_point)?;
        }
    }
    Ok(result)
}

pub(crate) fn limbs_to_biguint(limbs: &[u64]) -> BigUint {
    let mut bytes = Vec::with_capacity(limbs.len() * 8);
    for limb in limbs {
        bytes.extend_from_slice(&limb.to_le_bytes());
    }
    BigUint::from_bytes_le(&bytes)
}

pub(crate) fn biguint_to_limbs<const N: usize>(value: &BigUint) -> [u64; N] {
    let mut bytes = value.to_bytes_le();
    bytes.resize(N * 8, 0);
    let mut limbs = [0_u64; N];
    for (limb, chunk) in limbs.iter_mut().zip(bytes.chunks_exact(8)) {
        *limb = u64::from_le_bytes(chunk.try_into().expect("limb chunk is exactly 8 bytes"));
    }
    limbs
}

pub(crate) fn mod_inv(value: &BigUint, modulus: &BigUint) -> Option<BigUint> {
    let value = value % modulus;
    if value.is_zero() {
        return None;
    }
    Some(value.modpow(&(modulus - BigUint::from(2_u8)), modulus))
}

pub(crate) fn secp256k1_order() -> &'static BigUint {
    static ORDER: OnceLock<BigUint> = OnceLock::new();
    ORDER.get_or_init(|| {
        BigUint::parse_bytes(
            b"fffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141",
            16,
        )
        .expect("secp256k1 order should parse")
    })
}

fn limb_bit(limbs: &[u64; 4], bit: usize) -> bool {
    ((limbs[bit / 64] >> (bit % 64)) & 1) == 1
}

fn field_sub(first: &BigUint, second: &BigUint, modulus: &BigUint) -> BigUint {
    if first >= second {
        (first - second) % modulus
    } else {
        (first + modulus - second) % modulus
    }
}

fn field_mul(first: &BigUint, second: &BigUint, modulus: &BigUint) -> BigUint {
    (first * second) % modulus
}

fn field_square(value: &BigUint, modulus: &BigUint) -> BigUint {
    field_mul(value, value, modulus)
}

fn secp256k1_field_modulus() -> &'static BigUint {
    static MODULUS: OnceLock<BigUint> = OnceLock::new();
    MODULUS.get_or_init(|| {
        BigUint::parse_bytes(
            b"fffffffffffffffffffffffffffffffffffffffffffffffffffffffefffffc2f",
            16,
        )
        .expect("secp256k1 field modulus should parse")
    })
}
