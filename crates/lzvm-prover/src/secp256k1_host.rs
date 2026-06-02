use std::sync::OnceLock;

use num_bigint::BigUint;
use num_traits::{One, Zero};

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

#[derive(Debug, Clone, PartialEq, Eq)]
struct SecpProjectivePoint {
    x: BigUint,
    y: BigUint,
    z: BigUint,
}

impl SecpProjectivePoint {
    fn identity() -> Self {
        Self {
            x: BigUint::zero(),
            y: BigUint::one(),
            z: BigUint::zero(),
        }
    }

    fn from_affine(point: &SecpPoint) -> Self {
        if point.infinity {
            return Self::identity();
        }
        Self {
            x: point.x.clone(),
            y: point.y.clone(),
            z: BigUint::one(),
        }
    }

    fn is_identity(&self) -> bool {
        self.z.is_zero()
    }

    fn double(&self) -> Self {
        if self.is_identity() || self.y.is_zero() {
            return Self::identity();
        }
        let modulus = secp256k1_field_modulus();
        let xx = field_square(&self.x, modulus);
        let yy = field_square(&self.y, modulus);
        let yyyy = field_square(&yy, modulus);
        let x_plus_yy = field_add(&self.x, &yy, modulus);
        let s = field_mul_u64(
            &field_sub(
                &field_square(&x_plus_yy, modulus),
                &field_add(&xx, &yyyy, modulus),
                modulus,
            ),
            2,
            modulus,
        );
        let m = field_mul_u64(&xx, 3, modulus);
        let x = field_sub(
            &field_square(&m, modulus),
            &field_mul_u64(&s, 2, modulus),
            modulus,
        );
        let y = field_sub(
            &field_mul(&m, &field_sub(&s, &x, modulus), modulus),
            &field_mul_u64(&yyyy, 8, modulus),
            modulus,
        );
        let z = field_mul_u64(&field_mul(&self.y, &self.z, modulus), 2, modulus);
        Self { x, y, z }
    }

    fn add_affine(&self, point: &SecpPoint) -> Self {
        if point.infinity {
            return self.clone();
        }
        if self.is_identity() {
            return Self::from_affine(point);
        }
        let modulus = secp256k1_field_modulus();
        let zz = field_square(&self.z, modulus);
        let u2 = field_mul(&point.x, &zz, modulus);
        let s2 = field_mul(&point.y, &field_mul(&self.z, &zz, modulus), modulus);
        if u2 == self.x {
            if field_add(&self.y, &s2, modulus).is_zero() {
                return Self::identity();
            }
            return self.double();
        }

        let h = field_sub(&u2, &self.x, modulus);
        let hh = field_square(&h, modulus);
        let i = field_mul_u64(&hh, 4, modulus);
        let j = field_mul(&h, &i, modulus);
        let r = field_mul_u64(&field_sub(&s2, &self.y, modulus), 2, modulus);
        let v = field_mul(&self.x, &i, modulus);
        let x = field_sub(
            &field_sub(&field_square(&r, modulus), &j, modulus),
            &field_mul_u64(&v, 2, modulus),
            modulus,
        );
        let y = field_sub(
            &field_mul(&r, &field_sub(&v, &x, modulus), modulus),
            &field_mul_u64(&field_mul(&self.y, &j, modulus), 2, modulus),
            modulus,
        );
        let z = field_sub(
            &field_sub(
                &field_square(&field_add(&self.z, &h, modulus), modulus),
                &zz,
                modulus,
            ),
            &hh,
            modulus,
        );
        Self { x, y, z }
    }

    fn to_affine(&self) -> Result<SecpPoint, Secp256k1Error> {
        if self.is_identity() {
            return Ok(SecpPoint::identity());
        }
        let modulus = secp256k1_field_modulus();
        let z_inv = mod_inv(&self.z, modulus).ok_or(Secp256k1Error::NonInvertibleScalar)?;
        let z_inv_squared = field_square(&z_inv, modulus);
        let z_inv_cubed = field_mul(&z_inv_squared, &z_inv, modulus);
        Ok(SecpPoint {
            x: field_mul(&self.x, &z_inv_squared, modulus),
            y: field_mul(&self.y, &z_inv_cubed, modulus),
            infinity: false,
        })
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
    if !secp256k1_point_has_canonical_coordinates(first_point)
        || !secp256k1_point_has_canonical_coordinates(second_point)
    {
        return secp256k1_double_scalar_mul_affine(
            first_scalar,
            first_point,
            second_scalar,
            second_point,
        );
    }
    secp256k1_double_scalar_mul_projective(first_scalar, first_point, second_scalar, second_point)
}

fn secp256k1_double_scalar_mul_projective(
    first_scalar: &[u64; 4],
    first_point: &SecpPoint,
    second_scalar: &[u64; 4],
    second_point: &SecpPoint,
) -> Result<SecpPoint, Secp256k1Error> {
    let mut result = SecpProjectivePoint::identity();
    for bit in (0..256).rev() {
        result = result.double();
        if limb_bit(first_scalar, bit) {
            result = result.add_affine(first_point);
        }
        if limb_bit(second_scalar, bit) {
            result = result.add_affine(second_point);
        }
    }
    result.to_affine()
}

fn secp256k1_point_has_canonical_coordinates(point: &SecpPoint) -> bool {
    point.infinity || (point.x < *secp256k1_field_modulus() && point.y < *secp256k1_field_modulus())
}

pub(crate) fn limbs_to_biguint(limbs: &[u64]) -> BigUint {
    let mut bytes = Vec::with_capacity(limbs.len() * 8);
    for limb in limbs {
        bytes.extend_from_slice(&limb.to_le_bytes());
    }
    BigUint::from_bytes_le(&bytes)
}

fn secp256k1_double_scalar_mul_affine(
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

fn field_add(first: &BigUint, second: &BigUint, modulus: &BigUint) -> BigUint {
    (first + second) % modulus
}

fn field_mul(first: &BigUint, second: &BigUint, modulus: &BigUint) -> BigUint {
    (first * second) % modulus
}

fn field_mul_u64(value: &BigUint, multiplier: u64, modulus: &BigUint) -> BigUint {
    (value * BigUint::from(multiplier)) % modulus
}

fn field_square(value: &BigUint, modulus: &BigUint) -> BigUint {
    field_mul(value, value, modulus)
}

pub(crate) fn secp256k1_field_modulus() -> &'static BigUint {
    static MODULUS: OnceLock<BigUint> = OnceLock::new();
    MODULUS.get_or_init(|| {
        BigUint::parse_bytes(
            b"fffffffffffffffffffffffffffffffffffffffffffffffffffffffefffffc2f",
            16,
        )
        .expect("secp256k1 field modulus should parse")
    })
}

#[cfg(test)]
mod tests {
    use num_bigint::BigUint;

    use super::{
        limb_bit, secp256k1_double_scalar_mul, secp256k1_field_modulus, secp256k1_point_add,
        secp256k1_point_double, Secp256k1Error, SecpPoint, SecpProjectivePoint,
    };

    const SECP256K1_G: [u64; 8] = [
        0x59f2_815b_16f8_1798,
        0x029b_fcdb_2dce_28d9,
        0x55a0_6295_ce87_0b07,
        0x79be_667e_f9dc_bbac,
        0x9c47_d08f_fb10_d4b8,
        0xfd17_b448_a685_5419,
        0x5da4_fbfc_0e11_08a8,
        0x483a_da77_26a3_c465,
    ];

    #[test]
    fn projective_double_scalar_mul_matches_affine_reference_for_small_scalars() {
        let g = SecpPoint::from_limbs(&SECP256K1_G);
        let two_g = secp256k1_point_double(&g).expect("base point should double");
        for (first, second) in [(0, 0), (1, 0), (0, 1), (2, 3), (7, 11)] {
            let expected = secp256k1_point_add(
                &affine_scalar_mul(first, &g),
                &affine_scalar_mul(second, &two_g),
            )
            .expect("affine reference should add");
            let actual = secp256k1_double_scalar_mul(
                &scalar_limbs(first),
                &g,
                &scalar_limbs(second),
                &two_g,
            )
            .expect("projective double-scalar multiplication should run");

            assert_eq!(actual.to_limbs(), expected.to_limbs());
        }
    }

    #[test]
    fn double_scalar_mul_preserves_affine_semantics_for_noncanonical_inputs() {
        let g = SecpPoint::from_limbs(&SECP256K1_G);
        let modulus = secp256k1_field_modulus().clone();
        let raw_point = SecpPoint {
            x: modulus.clone(),
            y: BigUint::from(1_u8),
            infinity: false,
        };

        let expected =
            affine_double_scalar_mul_reference(&scalar_limbs(1), &raw_point, &[0; 4], &g)
                .expect("affine reference should preserve raw point");
        let actual = secp256k1_double_scalar_mul(&scalar_limbs(1), &raw_point, &[0; 4], &g)
            .expect("double-scalar multiplication should preserve raw point");

        assert_eq!(actual, expected);
    }

    #[test]
    fn double_scalar_mul_preserves_affine_semantics_for_noncanonical_second_input() {
        let g = SecpPoint::from_limbs(&SECP256K1_G);
        let raw_point = SecpPoint {
            x: secp256k1_field_modulus().clone(),
            y: BigUint::from(1_u8),
            infinity: false,
        };

        let expected =
            affine_double_scalar_mul_reference(&[0; 4], &g, &scalar_limbs(1), &raw_point)
                .expect("affine reference should preserve raw point");
        let actual = secp256k1_double_scalar_mul(&[0; 4], &g, &scalar_limbs(1), &raw_point)
            .expect("double-scalar multiplication should preserve raw point");

        assert_eq!(actual, expected);
    }

    #[test]
    fn double_scalar_mul_preserves_affine_error_for_noncanonical_zero_y() {
        let g = SecpPoint::from_limbs(&SECP256K1_G);
        let raw_point = SecpPoint {
            x: BigUint::from(1_u8),
            y: secp256k1_field_modulus().clone(),
            infinity: false,
        };

        let expected =
            affine_double_scalar_mul_reference(&scalar_limbs(2), &raw_point, &[0; 4], &g)
                .expect_err("affine reference should reject non-invertible denominator");
        let actual = secp256k1_double_scalar_mul(&scalar_limbs(2), &raw_point, &[0; 4], &g)
            .expect_err("double-scalar multiplication should preserve affine error");

        assert_eq!(actual, expected);
    }

    #[test]
    fn projective_accumulator_matches_affine_exceptional_branches() {
        let g = SecpPoint::from_limbs(&SECP256K1_G);
        let zero_y = SecpPoint {
            x: BigUint::from(1_u8),
            y: BigUint::from(0_u8),
            infinity: false,
        };
        let zero_y_actual = SecpProjectivePoint::from_affine(&zero_y)
            .double()
            .to_affine()
            .expect("zero-y projective double should convert");
        let zero_y_expected = secp256k1_point_double(&zero_y).expect("zero-y affine double");
        assert_eq!(zero_y_actual, zero_y_expected);

        let doubled_actual = SecpProjectivePoint::from_affine(&g)
            .add_affine(&g)
            .to_affine()
            .expect("same point projective add should convert");
        let doubled_expected = secp256k1_point_double(&g).expect("base point should double");
        assert_eq!(doubled_actual.to_limbs(), doubled_expected.to_limbs());

        let modulus = secp256k1_field_modulus();
        let neg_g = SecpPoint {
            x: g.x.clone(),
            y: (modulus - &g.y) % modulus,
            infinity: false,
        };
        let inverse_sum = SecpProjectivePoint::from_affine(&g)
            .add_affine(&neg_g)
            .to_affine()
            .expect("inverse projective add should convert");
        assert_eq!(inverse_sum, SecpPoint::identity());
    }

    fn affine_double_scalar_mul_reference(
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

    fn affine_scalar_mul(scalar: u64, point: &SecpPoint) -> SecpPoint {
        let mut out = SecpPoint::identity();
        for _ in 0..scalar {
            out = secp256k1_point_add(&out, point).expect("affine scalar reference should add");
        }
        out
    }

    fn scalar_limbs(value: u64) -> [u64; 4] {
        [value, 0, 0, 0]
    }
}
