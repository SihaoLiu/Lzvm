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
    domain_points
        .iter()
        .enumerate()
        .map(|(row, x)| {
            (x.pow(base_size) - Felt::ONE).inverse().ok_or(
                FriPolynomialError::ZeroZerofierDenominator {
                    boundary_index: 0,
                    row,
                },
            )
        })
        .collect()
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
