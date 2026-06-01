#[cfg(test)]
use std::cell::Cell;

use lzvm_artifacts::setup_info::Boundary;
use lzvm_field::{Ext3, Felt, SHIFT};

use super::{FriPolynomialError, FriPolynomialZerofierTable};

impl FriPolynomialZerofierTable {
    pub fn build(
        base_domain_bits: u32,
        extended_domain_bits: u32,
        boundaries: &[Boundary],
    ) -> Result<Self, FriPolynomialError> {
        let domain_points = build_fri_domain_points(extended_domain_bits)?;
        let base_size = domain_size_u64(base_domain_bits)?;
        let base_root = Felt::root_of_unity(base_domain_bits as usize).ok_or(
            FriPolynomialError::UnsupportedDomainBits {
                bits: base_domain_bits,
            },
        )?;
        let every_row = build_every_row_zerofier(base_size, &domain_points)?;
        let column_count = boundaries.len();
        let mut values = vec![Felt::ZERO; domain_points.len().saturating_mul(column_count)];
        for (boundary_index, boundary) in boundaries.iter().enumerate() {
            for (row, x) in domain_points.iter().copied().enumerate() {
                values[row * column_count + boundary_index] = match boundary.name.as_deref() {
                    Some("everyRow") => every_row[row],
                    Some("firstRow") => {
                        build_one_row_zerofier(boundary_index, row, x, Felt::ONE, every_row[row])?
                    }
                    Some("lastRow") => build_one_row_zerofier(
                        boundary_index,
                        row,
                        x,
                        base_root.pow(base_size),
                        every_row[row],
                    )?,
                    Some("everyFrame") => {
                        build_frame_zerofier(boundary_index, boundary, base_size, base_root, x)?
                    }
                    _ => {
                        return Err(FriPolynomialError::UnsupportedBoundary {
                            boundary_index,
                            name: boundary.name.clone(),
                        });
                    }
                };
            }
        }
        Ok(Self {
            column_count,
            values,
        })
    }
}

pub fn build_fri_domain_points(bits: u32) -> Result<Vec<Felt>, FriPolynomialError> {
    let root = Felt::root_of_unity(bits as usize)
        .ok_or(FriPolynomialError::UnsupportedDomainBits { bits })?;
    let size = domain_size_usize(bits)?;
    let mut points = Vec::with_capacity(size);
    let mut point = SHIFT;
    for _ in 0..size {
        points.push(point);
        point = point * root;
    }
    Ok(points)
}

pub fn derive_opening_xis(
    base_domain_bits: u32,
    opening_points: &[i64],
    xi_challenge: Ext3,
) -> Result<Vec<Ext3>, FriPolynomialError> {
    let root = Felt::root_of_unity(base_domain_bits as usize).ok_or(
        FriPolynomialError::UnsupportedDomainBits {
            bits: base_domain_bits,
        },
    )?;
    opening_points
        .iter()
        .map(|opening_point| {
            let mut scalar = root.pow(opening_point.unsigned_abs());
            if *opening_point < 0 {
                scalar = scalar
                    .inverse()
                    .ok_or(FriPolynomialError::ZeroDenominator { opening_index: 0 })?;
            }
            Ok(xi_challenge * scalar_ext(scalar))
        })
        .collect()
}

fn build_every_row_zerofier(
    base_size: u64,
    domain_points: &[Felt],
) -> Result<Vec<Felt>, FriPolynomialError> {
    let mut denominators: Vec<_> = domain_points
        .iter()
        .map(|x| x.pow(base_size) - Felt::ONE)
        .collect();
    batch_inverse_zerofier_values(0, &mut denominators)?;
    Ok(denominators)
}

fn batch_inverse_zerofier_values(
    boundary_index: usize,
    values: &mut [Felt],
) -> Result<(), FriPolynomialError> {
    if values.is_empty() {
        return Ok(());
    }

    let mut prefixes = Vec::with_capacity(values.len());
    let mut product = Felt::ONE;
    for (row, value) in values.iter().copied().enumerate() {
        if value == Felt::ZERO {
            return Err(FriPolynomialError::ZeroZerofierDenominator {
                boundary_index,
                row,
            });
        }
        prefixes.push(product);
        product = product * value;
    }

    #[cfg(test)]
    EVERY_ROW_ZEROFIER_INVERSE_COUNT.with(|count| count.set(count.get() + 1));

    let mut suffix_inverse =
        product
            .inverse()
            .ok_or(FriPolynomialError::ZeroZerofierDenominator {
                boundary_index,
                row: values.len() - 1,
            })?;
    for (value, prefix) in values.iter_mut().zip(prefixes).rev() {
        let denominator = *value;
        *value = suffix_inverse * prefix;
        suffix_inverse = suffix_inverse * denominator;
    }
    Ok(())
}

fn build_one_row_zerofier(
    boundary_index: usize,
    row: usize,
    x: Felt,
    root: Felt,
    every_row: Felt,
) -> Result<Felt, FriPolynomialError> {
    ((x - root) * every_row)
        .inverse()
        .ok_or(FriPolynomialError::ZeroZerofierDenominator {
            boundary_index,
            row,
        })
}

fn build_frame_zerofier(
    boundary_index: usize,
    boundary: &Boundary,
    base_size: u64,
    base_root: Felt,
    x: Felt,
) -> Result<Felt, FriPolynomialError> {
    let offset_min = boundary_offset(boundary_index, boundary.offset_min, "offset_min")?;
    let offset_max = boundary_offset(boundary_index, boundary.offset_max, "offset_max")?;
    let mut value = Felt::ONE;
    for offset in 0..offset_min {
        value = value * (x - base_root.pow(offset));
    }
    for offset in 0..offset_max {
        let Some(exponent) = base_size.checked_sub(offset + 1) else {
            return Err(FriPolynomialError::InvalidBoundaryOffset {
                boundary_index,
                field: "offset_max",
                value: i64::try_from(offset).unwrap_or(i64::MAX),
            });
        };
        value = value * (x - base_root.pow(exponent));
    }
    Ok(value)
}

fn boundary_offset(
    boundary_index: usize,
    value: Option<i64>,
    field: &'static str,
) -> Result<u64, FriPolynomialError> {
    let value = value.ok_or(FriPolynomialError::MissingBoundaryOffset {
        boundary_index,
        field,
    })?;
    if value < 0 {
        return Err(FriPolynomialError::InvalidBoundaryOffset {
            boundary_index,
            field,
            value,
        });
    }
    u64::try_from(value).map_err(|_| FriPolynomialError::InvalidBoundaryOffset {
        boundary_index,
        field,
        value,
    })
}

fn domain_size_usize(bits: u32) -> Result<usize, FriPolynomialError> {
    1usize
        .checked_shl(bits)
        .ok_or(FriPolynomialError::LengthOverflow)
}

fn domain_size_u64(bits: u32) -> Result<u64, FriPolynomialError> {
    1u64.checked_shl(bits)
        .ok_or(FriPolynomialError::LengthOverflow)
}

fn scalar_ext(value: Felt) -> Ext3 {
    Ext3::new(value, Felt::ZERO, Felt::ZERO)
}

#[cfg(test)]
thread_local! {
    static EVERY_ROW_ZEROFIER_INVERSE_COUNT: Cell<usize> = const { Cell::new(0) };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_row_zerofier_uses_one_inverse() {
        let domain_points = build_fri_domain_points(3).expect("domain points should build");
        let expected: Vec<_> = domain_points
            .iter()
            .map(|x| {
                (x.pow(4) - Felt::ONE)
                    .inverse()
                    .expect("coset denominator should be nonzero")
            })
            .collect();

        EVERY_ROW_ZEROFIER_INVERSE_COUNT.with(|count| count.set(0));
        let values =
            build_every_row_zerofier(4, &domain_points).expect("every row zerofier should build");

        assert_eq!(values, expected);
        assert_eq!(EVERY_ROW_ZEROFIER_INVERSE_COUNT.with(Cell::get), 1);
    }

    #[test]
    fn batched_zerofier_inverse_reports_zero_row() {
        let mut values = [Felt::from_u64(3), Felt::ZERO, Felt::from_u64(5)];

        let err = batch_inverse_zerofier_values(7, &mut values)
            .expect_err("zero denominator should fail");

        assert_eq!(
            err,
            FriPolynomialError::ZeroZerofierDenominator {
                boundary_index: 7,
                row: 1,
            }
        );
    }

    #[test]
    fn batched_zerofier_inverse_accepts_empty_input() {
        let mut values = [];

        batch_inverse_zerofier_values(4, &mut values).expect("empty input should not fail");
        assert!(values.is_empty());
    }
}
