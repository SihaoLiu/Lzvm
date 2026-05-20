use std::path::Path;
use std::thread;

#[cfg(all(test, feature = "cuda"))]
use lzvm_accel::{cuda_poseidon2_width16_device, cuda_poseidon2_width8_device};
#[cfg(feature = "cuda")]
use lzvm_accel::{
    cuda_poseidon2_width16_linear_round_device, cuda_poseidon2_width16_merkle_parent_device,
    cuda_poseidon2_width8_linear_round_device, cuda_poseidon2_width8_merkle_parent_device,
    cuda_setup_init, CudaDeviceBuffer,
};
use lzvm_artifacts::constant_tree::{parse_constant_tree_bytes, read_constant_tree_file};
use lzvm_artifacts::fixed::{
    encode_raw_fixed_columns, read_raw_fixed_column_layout_file, write_raw_fixed_columns_file,
    FixedColumns,
};
use lzvm_artifacts::setup_info::UnitSetupInfo;
use lzvm_artifacts::verification_key::{
    encode_verification_key_binary, read_verification_key_binary_file, VerificationKeyRoot,
};
use lzvm_field::{coset_extend_evaluations, poseidon2_hash_16, poseidon2_hash_8, Felt};

use crate::{
    publish_staging_bytes, staging_path_for, write_staging_bytes, ConstantTreeLeavesWriteReport,
    ConstantTreeWriteReport, FixedColumnWriteReport, FixedExtensionBackend, SetupError,
    VerificationKeyWriteReport,
};

const WORD_BYTES: usize = 8;
const HASH_WORDS: usize = 4;
const MAX_CPU_TREE_WORKERS: usize = 8;
const MIN_PARALLEL_TREE_ROWS: usize = 1 << 14;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ConstantTreeShape {
    arity: usize,
    row_count: usize,
    column_count: usize,
    expected_tree_len: usize,
}

pub fn extend_fixed_columns_for_constant_tree(
    value: &FixedColumns,
    setup: &UnitSetupInfo,
) -> Result<Vec<u8>, SetupError> {
    extend_fixed_columns_for_constant_tree_with_backend(value, setup, FixedExtensionBackend::Cpu)
}

pub fn extend_fixed_columns_for_constant_tree_with_backend(
    value: &FixedColumns,
    setup: &UnitSetupInfo,
    backend: FixedExtensionBackend,
) -> Result<Vec<u8>, SetupError> {
    #[cfg(feature = "cuda")]
    if matches!(backend, FixedExtensionBackend::Cuda) {
        cuda_setup_init(setup.stark.n_bits_ext as usize)
            .map_err(|error| SetupError::CudaBackend(error.to_string()))?;
    }

    let extended_row_count = checked_domain_len(setup.stark.n_bits_ext)?;
    let columns = fixed_columns_for_extension(value, setup)?;
    let extended_columns = match backend {
        FixedExtensionBackend::Cpu => extend_columns_on_cpu(&columns, setup)?,
        FixedExtensionBackend::Cuda => extend_columns_on_cuda(&columns, setup)?,
    };

    encode_extended_columns(&extended_columns, extended_row_count)
}

fn fixed_columns_for_extension(
    value: &FixedColumns,
    setup: &UnitSetupInfo,
) -> Result<Vec<Vec<Felt>>, SetupError> {
    let raw = encode_raw_fixed_columns(value, setup)?;
    let row_count = checked_domain_len(setup.stark.n_bits)?;
    let column_count =
        usize::try_from(setup.n_constants).map_err(|_| SetupError::LengthOverflow)?;
    let word_count = row_count
        .checked_mul(column_count)
        .ok_or(SetupError::LengthOverflow)?;
    if raw.len()
        != word_count
            .checked_mul(WORD_BYTES)
            .ok_or(SetupError::LengthOverflow)?
    {
        return Err(SetupError::LengthOverflow);
    }
    let mut extended_columns = Vec::with_capacity(column_count);
    for column in 0..column_count {
        let mut values = Vec::with_capacity(row_count);
        for row in 0..row_count {
            let word_index = row
                .checked_mul(column_count)
                .and_then(|offset| offset.checked_add(column))
                .ok_or(SetupError::LengthOverflow)?;
            let byte_index = word_index
                .checked_mul(WORD_BYTES)
                .ok_or(SetupError::LengthOverflow)?;
            let value = u64::from_le_bytes(
                raw[byte_index..byte_index + WORD_BYTES]
                    .try_into()
                    .expect("slice length checked"),
            );
            values.push(Felt::from_canonical(value)?);
        }
        extended_columns.push(values);
    }
    Ok(extended_columns)
}

fn extend_columns_on_cpu(
    columns: &[Vec<Felt>],
    setup: &UnitSetupInfo,
) -> Result<Vec<Vec<Felt>>, SetupError> {
    columns
        .iter()
        .map(|values| {
            coset_extend_evaluations(
                values,
                setup.stark.n_bits as usize,
                setup.stark.n_bits_ext as usize,
            )
            .map_err(Into::into)
        })
        .collect()
}

#[cfg(feature = "cuda")]
fn extend_columns_on_cuda(
    columns: &[Vec<Felt>],
    setup: &UnitSetupInfo,
) -> Result<Vec<Vec<Felt>>, SetupError> {
    columns
        .iter()
        .map(|values| {
            let source = values
                .iter()
                .map(|value| value.to_u64())
                .collect::<Vec<_>>();
            let extended = lzvm_accel::cuda_goldilocks_coset_extend(
                &source,
                setup.stark.n_bits as usize,
                setup.stark.n_bits_ext as usize,
            )
            .map_err(|error| SetupError::CudaBackend(error.to_string()))?;
            extended
                .into_iter()
                .map(|value| Felt::from_canonical(value).map_err(Into::into))
                .collect()
        })
        .collect()
}

#[cfg(not(feature = "cuda"))]
fn extend_columns_on_cuda(
    _columns: &[Vec<Felt>],
    _setup: &UnitSetupInfo,
) -> Result<Vec<Vec<Felt>>, SetupError> {
    Err(SetupError::CudaUnavailable)
}

fn encode_extended_columns(
    extended_columns: &[Vec<Felt>],
    extended_row_count: usize,
) -> Result<Vec<u8>, SetupError> {
    let column_count = extended_columns.len();
    let byte_count = extended_row_count
        .checked_mul(column_count)
        .and_then(|count| count.checked_mul(WORD_BYTES))
        .ok_or(SetupError::LengthOverflow)?;
    for column_values in extended_columns {
        if column_values.len() != extended_row_count {
            return Err(SetupError::LengthOverflow);
        }
    }

    let mut out = Vec::with_capacity(byte_count);
    for row in 0..extended_row_count {
        for column_values in extended_columns {
            out.extend_from_slice(&column_values[row].to_le_bytes());
        }
    }
    Ok(out)
}

pub fn write_base_fixed_columns(
    path: impl AsRef<Path>,
    value: &FixedColumns,
    setup: &UnitSetupInfo,
) -> Result<FixedColumnWriteReport, SetupError> {
    let path = path.as_ref().to_path_buf();
    let parent = path
        .parent()
        .ok_or_else(|| SetupError::MissingParent { path: path.clone() })?;
    std::fs::create_dir_all(parent).map_err(|error| SetupError::Io {
        role: "create output directory",
        path: parent.to_path_buf(),
        message: error.to_string(),
    })?;

    let staging_path = staging_path_for(&path);
    write_raw_fixed_columns_file(&staging_path, value, setup)?;
    read_raw_fixed_column_layout_file(&staging_path, setup, &value.group_name, &value.unit_name)?;
    let bytes_written = std::fs::metadata(&staging_path)
        .map_err(|error| SetupError::Io {
            role: "read staging metadata",
            path: staging_path.clone(),
            message: error.to_string(),
        })?
        .len();
    std::fs::rename(&staging_path, &path).map_err(|error| SetupError::Io {
        role: "publish fixed columns",
        path: path.clone(),
        message: error.to_string(),
    })?;

    Ok(FixedColumnWriteReport {
        path,
        bytes_written,
    })
}

pub fn write_constant_tree_leaves(
    path: impl AsRef<Path>,
    value: &FixedColumns,
    setup: &UnitSetupInfo,
) -> Result<ConstantTreeLeavesWriteReport, SetupError> {
    write_constant_tree_leaves_with_backend(path, value, setup, FixedExtensionBackend::Cpu)
}

pub fn write_constant_tree_leaves_with_backend(
    path: impl AsRef<Path>,
    value: &FixedColumns,
    setup: &UnitSetupInfo,
    backend: FixedExtensionBackend,
) -> Result<ConstantTreeLeavesWriteReport, SetupError> {
    let path = path.as_ref().to_path_buf();
    let parent = path
        .parent()
        .ok_or_else(|| SetupError::MissingParent { path: path.clone() })?;
    std::fs::create_dir_all(parent).map_err(|error| SetupError::Io {
        role: "create output directory",
        path: parent.to_path_buf(),
        message: error.to_string(),
    })?;

    let leaves = extend_fixed_columns_for_constant_tree_with_backend(value, setup, backend)?;
    let expected_len = checked_domain_len(setup.stark.n_bits_ext)?
        .checked_mul(usize::try_from(setup.n_constants).map_err(|_| SetupError::LengthOverflow)?)
        .and_then(|words| words.checked_mul(8))
        .ok_or(SetupError::LengthOverflow)?;
    if leaves.len() != expected_len {
        return Err(SetupError::LengthOverflow);
    }

    let staging_path = staging_path_for(&path);
    std::fs::write(&staging_path, &leaves).map_err(|error| SetupError::Io {
        role: "write constant-tree leaves staging file",
        path: staging_path.clone(),
        message: error.to_string(),
    })?;
    let bytes_written = std::fs::metadata(&staging_path)
        .map_err(|error| SetupError::Io {
            role: "read staging metadata",
            path: staging_path.clone(),
            message: error.to_string(),
        })?
        .len();
    if bytes_written != u64::try_from(expected_len).map_err(|_| SetupError::LengthOverflow)? {
        return Err(SetupError::LengthOverflow);
    }
    std::fs::rename(&staging_path, &path).map_err(|error| SetupError::Io {
        role: "publish constant-tree leaves",
        path: path.clone(),
        message: error.to_string(),
    })?;

    Ok(ConstantTreeLeavesWriteReport {
        path,
        bytes_written,
        row_count: 1_u64
            .checked_shl(setup.stark.n_bits_ext)
            .ok_or(SetupError::LengthOverflow)?,
        column_count: setup.n_constants,
    })
}

pub fn build_constant_tree_from_fixed_columns(
    value: &FixedColumns,
    setup: &UnitSetupInfo,
) -> Result<Vec<u8>, SetupError> {
    build_constant_tree_from_fixed_columns_with_backend(value, setup, FixedExtensionBackend::Cpu)
}

pub fn build_constant_tree_from_fixed_columns_with_backend(
    value: &FixedColumns,
    setup: &UnitSetupInfo,
    backend: FixedExtensionBackend,
) -> Result<Vec<u8>, SetupError> {
    build_constant_tree_from_fixed_columns_with_cpu_parallelism(
        value,
        setup,
        backend,
        default_cpu_tree_parallelism(),
    )
}

pub(crate) fn build_constant_tree_from_fixed_columns_with_cpu_parallelism(
    value: &FixedColumns,
    setup: &UnitSetupInfo,
    backend: FixedExtensionBackend,
    cpu_parallelism: usize,
) -> Result<Vec<u8>, SetupError> {
    let leaves = extend_fixed_columns_for_constant_tree_with_backend(value, setup, backend)?;
    build_constant_tree_from_leaves_with_cpu_parallelism(&leaves, setup, backend, cpu_parallelism)
}

pub fn build_constant_tree_from_leaves(
    leaves: &[u8],
    setup: &UnitSetupInfo,
) -> Result<Vec<u8>, SetupError> {
    build_constant_tree_from_leaves_with_backend(leaves, setup, FixedExtensionBackend::Cpu)
}

pub fn build_constant_tree_from_leaves_with_backend(
    leaves: &[u8],
    setup: &UnitSetupInfo,
    backend: FixedExtensionBackend,
) -> Result<Vec<u8>, SetupError> {
    build_constant_tree_from_leaves_with_cpu_parallelism(
        leaves,
        setup,
        backend,
        default_cpu_tree_parallelism(),
    )
}

fn build_constant_tree_from_leaves_with_cpu_parallelism(
    leaves: &[u8],
    setup: &UnitSetupInfo,
    backend: FixedExtensionBackend,
    cpu_parallelism: usize,
) -> Result<Vec<u8>, SetupError> {
    match backend {
        FixedExtensionBackend::Cpu => {
            build_constant_tree_from_leaves_on_cpu(leaves, setup, cpu_parallelism)
        }
        FixedExtensionBackend::Cuda => build_constant_tree_from_leaves_on_cuda(leaves, setup),
    }
}

fn build_constant_tree_from_leaves_on_cpu(
    leaves: &[u8],
    setup: &UnitSetupInfo,
    cpu_parallelism: usize,
) -> Result<Vec<u8>, SetupError> {
    let shape = constant_tree_shape(leaves, setup)?;
    let worker_count = effective_cpu_tree_parallelism(cpu_parallelism, shape.row_count);

    let mut out = Vec::with_capacity(shape.expected_tree_len);
    out.extend_from_slice(leaves);

    let mut level = vec![[Felt::ZERO; HASH_WORDS]; shape.row_count];
    fill_leaf_level(leaves, shape, &mut level, worker_count)?;
    for digest in &level {
        append_digest(&mut out, *digest);
    }

    while level.len() > 1 {
        let extra_zeros = (shape.arity - (level.len() % shape.arity)) % shape.arity;
        for _ in 0..extra_zeros {
            let zero = [Felt::ZERO; HASH_WORDS];
            append_digest(&mut out, zero);
            level.push(zero);
        }

        let next = build_parent_level(&level, shape.arity, worker_count)?;
        for digest in &next {
            append_digest(&mut out, *digest);
        }
        level = next;
    }

    parse_constant_tree_bytes(out.clone(), setup)?;
    Ok(out)
}

#[cfg(feature = "cuda")]
fn build_constant_tree_from_leaves_on_cuda(
    leaves: &[u8],
    setup: &UnitSetupInfo,
) -> Result<Vec<u8>, SetupError> {
    let shape = constant_tree_shape(leaves, setup)?;
    let rows = read_constant_tree_leaf_rows(leaves, shape)?;

    let mut out = Vec::with_capacity(shape.expected_tree_len);
    out.extend_from_slice(leaves);

    let mut level = cuda_linear_hashes_with_states(&rows, shape.arity)?;
    for digest in &level.digests {
        append_digest(&mut out, *digest);
    }

    while level.state_count > 1 {
        let extra_zeros = (shape.arity - (level.state_count % shape.arity)) % shape.arity;
        for _ in 0..extra_zeros {
            let zero = [Felt::ZERO; HASH_WORDS];
            append_digest(&mut out, zero);
        }

        let next = cuda_parent_level_with_states(&level, shape.arity)?;
        for digest in &next.digests {
            append_digest(&mut out, *digest);
        }
        level = next;
    }

    parse_constant_tree_bytes(out.clone(), setup)?;
    Ok(out)
}

#[cfg(not(feature = "cuda"))]
fn build_constant_tree_from_leaves_on_cuda(
    _leaves: &[u8],
    _setup: &UnitSetupInfo,
) -> Result<Vec<u8>, SetupError> {
    Err(SetupError::CudaUnavailable)
}

pub fn write_constant_tree_from_fixed_columns(
    path: impl AsRef<Path>,
    value: &FixedColumns,
    setup: &UnitSetupInfo,
) -> Result<ConstantTreeWriteReport, SetupError> {
    let tree = build_constant_tree_from_fixed_columns(value, setup)?;
    write_base_constant_tree(path, &tree, setup, None)
}

pub fn write_base_constant_tree(
    path: impl AsRef<Path>,
    value: &[u8],
    setup: &UnitSetupInfo,
    expected_root: Option<&VerificationKeyRoot>,
) -> Result<ConstantTreeWriteReport, SetupError> {
    let path = path.as_ref().to_path_buf();
    let parent = path
        .parent()
        .ok_or_else(|| SetupError::MissingParent { path: path.clone() })?;
    std::fs::create_dir_all(parent).map_err(|error| SetupError::Io {
        role: "create output directory",
        path: parent.to_path_buf(),
        message: error.to_string(),
    })?;

    let tree = parse_constant_tree_bytes(value.to_vec(), setup)?;
    let root = tree.root()?;
    if let Some(expected) = expected_root {
        if expected != &root {
            return Err(SetupError::ConstantTreeRootMismatch {
                expected: expected.clone(),
                found: root,
            });
        }
    }

    let staging_path = staging_path_for(&path);
    std::fs::write(&staging_path, value).map_err(|error| SetupError::Io {
        role: "write constant-tree staging file",
        path: staging_path.clone(),
        message: error.to_string(),
    })?;
    let staged_tree = read_constant_tree_file(&staging_path, setup)?;
    let staged_root = staged_tree.root()?;
    if staged_root != root {
        return Err(SetupError::ConstantTreeRootMismatch {
            expected: root,
            found: staged_root,
        });
    }
    let bytes_written = std::fs::metadata(&staging_path)
        .map_err(|error| SetupError::Io {
            role: "read staging metadata",
            path: staging_path.clone(),
            message: error.to_string(),
        })?
        .len();
    std::fs::rename(&staging_path, &path).map_err(|error| SetupError::Io {
        role: "publish constant tree",
        path: path.clone(),
        message: error.to_string(),
    })?;

    Ok(ConstantTreeWriteReport {
        path,
        bytes_written,
        root,
    })
}

pub fn write_verification_key_from_constant_tree(
    binary_path: impl AsRef<Path>,
    tree_bytes: &[u8],
    setup: &UnitSetupInfo,
) -> Result<VerificationKeyWriteReport, SetupError> {
    let tree = parse_constant_tree_bytes(tree_bytes.to_vec(), setup)?;
    let root = tree.root()?;
    let binary_bytes = encode_verification_key_binary(&root)?;

    let binary_path = binary_path.as_ref().to_path_buf();
    let binary_staging =
        write_staging_bytes(&binary_path, &binary_bytes, "verification-key binary")?;

    let binary_root = read_verification_key_binary_file(&binary_staging)?;
    if binary_root != root {
        return Err(SetupError::ConstantTreeRootMismatch {
            expected: root.clone(),
            found: binary_root,
        });
    }

    let binary_size =
        publish_staging_bytes(&binary_staging, &binary_path, "verification-key binary")?;

    Ok(VerificationKeyWriteReport {
        binary_path,
        binary_bytes: binary_size,
        root,
    })
}

fn checked_domain_len(bits: u32) -> Result<usize, SetupError> {
    1_usize.checked_shl(bits).ok_or(SetupError::LengthOverflow)
}

fn validate_native_constant_tree_setup(setup: &UnitSetupInfo) -> Result<(), SetupError> {
    if !matches!(setup.stark.merkle_tree_arity, 2 | 4) {
        return Err(SetupError::UnsupportedConstantTreeArity {
            arity: setup.stark.merkle_tree_arity,
        });
    }
    match setup.stark.verification_hash_type.as_deref() {
        None | Some("GL") => Ok(()),
        _ => Err(SetupError::UnsupportedConstantTreeHash {
            hash_type: setup.stark.verification_hash_type.clone(),
        }),
    }
}

fn constant_tree_leaf_byte_count(
    row_count: usize,
    column_count: usize,
) -> Result<usize, SetupError> {
    row_count
        .checked_mul(column_count)
        .and_then(|words| words.checked_mul(WORD_BYTES))
        .ok_or(SetupError::LengthOverflow)
}

fn expected_constant_tree_byte_count_for_setup(setup: &UnitSetupInfo) -> Result<usize, SetupError> {
    lzvm_artifacts::constant_tree::expected_constant_tree_byte_count(setup).map_err(Into::into)
}

fn constant_tree_shape(
    leaves: &[u8],
    setup: &UnitSetupInfo,
) -> Result<ConstantTreeShape, SetupError> {
    validate_native_constant_tree_setup(setup)?;
    let arity =
        usize::try_from(setup.stark.merkle_tree_arity).map_err(|_| SetupError::LengthOverflow)?;
    let row_count = checked_domain_len(setup.stark.n_bits_ext)?;
    let column_count =
        usize::try_from(setup.n_constants).map_err(|_| SetupError::LengthOverflow)?;
    let expected_leaf_len = constant_tree_leaf_byte_count(row_count, column_count)?;
    if leaves.len() != expected_leaf_len {
        return Err(SetupError::InvalidConstantTreeLeafByteLength {
            expected: expected_leaf_len,
            found: leaves.len(),
        });
    }
    Ok(ConstantTreeShape {
        arity,
        row_count,
        column_count,
        expected_tree_len: expected_constant_tree_byte_count_for_setup(setup)?,
    })
}

#[cfg(feature = "cuda")]
fn read_constant_tree_leaf_rows(
    leaves: &[u8],
    shape: ConstantTreeShape,
) -> Result<Vec<Vec<Felt>>, SetupError> {
    let mut rows = Vec::with_capacity(shape.row_count);
    for row in 0..shape.row_count {
        let mut values = Vec::with_capacity(shape.column_count);
        for column in 0..shape.column_count {
            let word_index = row
                .checked_mul(shape.column_count)
                .and_then(|offset| offset.checked_add(column))
                .ok_or(SetupError::LengthOverflow)?;
            let byte_index = word_index
                .checked_mul(WORD_BYTES)
                .ok_or(SetupError::LengthOverflow)?;
            let value = u64::from_le_bytes(
                leaves[byte_index..byte_index + WORD_BYTES]
                    .try_into()
                    .expect("slice length checked"),
            );
            values.push(Felt::from_canonical(value)?);
        }
        rows.push(values);
    }
    Ok(rows)
}

fn linear_hash_leaf_row(
    leaves: &[u8],
    shape: ConstantTreeShape,
    row: usize,
) -> Result<[Felt; HASH_WORDS], SetupError> {
    match shape.arity {
        2 => linear_hash_leaf_row_arity2(leaves, shape, row),
        4 => linear_hash_leaf_row_arity4(leaves, shape, row),
        _ => Err(SetupError::UnsupportedConstantTreeArity {
            arity: u32::try_from(shape.arity).unwrap_or(u32::MAX),
        }),
    }
}

fn linear_hash_leaf_row_arity2(
    leaves: &[u8],
    shape: ConstantTreeShape,
    row: usize,
) -> Result<[Felt; HASH_WORDS], SetupError> {
    if shape.column_count <= HASH_WORDS {
        return padded_leaf_row_digest(leaves, shape, row);
    }

    let mut state = [Felt::ZERO; 8];
    let mut offset = 0;
    while offset < shape.column_count {
        let capacity = [state[0], state[1], state[2], state[3]];
        state[4..].copy_from_slice(&capacity);
        state[..HASH_WORDS].fill(Felt::ZERO);

        let chunk_len = (shape.column_count - offset).min(HASH_WORDS);
        for (index, slot) in state.iter_mut().enumerate().take(chunk_len) {
            *slot = read_leaf_value(leaves, shape, row, offset + index)?;
        }
        state = poseidon2_hash_8(state);
        offset += chunk_len;
    }

    Ok([state[0], state[1], state[2], state[3]])
}

fn linear_hash_leaf_row_arity4(
    leaves: &[u8],
    shape: ConstantTreeShape,
    row: usize,
) -> Result<[Felt; HASH_WORDS], SetupError> {
    const RATE: usize = 12;

    if shape.column_count <= HASH_WORDS {
        return padded_leaf_row_digest(leaves, shape, row);
    }

    let mut state = [Felt::ZERO; 16];
    let mut offset = 0;
    while offset < shape.column_count {
        let capacity = [state[0], state[1], state[2], state[3]];
        state[RATE..].copy_from_slice(&capacity);
        state[..RATE].fill(Felt::ZERO);

        let chunk_len = (shape.column_count - offset).min(RATE);
        for (index, slot) in state.iter_mut().enumerate().take(chunk_len) {
            *slot = read_leaf_value(leaves, shape, row, offset + index)?;
        }
        state = poseidon2_hash_16(state);
        offset += chunk_len;
    }

    Ok([state[0], state[1], state[2], state[3]])
}

fn padded_leaf_row_digest(
    leaves: &[u8],
    shape: ConstantTreeShape,
    row: usize,
) -> Result<[Felt; HASH_WORDS], SetupError> {
    let mut digest = [Felt::ZERO; HASH_WORDS];
    for (column, slot) in digest.iter_mut().enumerate().take(shape.column_count) {
        *slot = read_leaf_value(leaves, shape, row, column)?;
    }
    Ok(digest)
}

fn read_leaf_value(
    leaves: &[u8],
    shape: ConstantTreeShape,
    row: usize,
    column: usize,
) -> Result<Felt, SetupError> {
    let word_index = row
        .checked_mul(shape.column_count)
        .and_then(|offset| offset.checked_add(column))
        .ok_or(SetupError::LengthOverflow)?;
    let byte_index = word_index
        .checked_mul(WORD_BYTES)
        .ok_or(SetupError::LengthOverflow)?;
    let value = u64::from_le_bytes(
        leaves[byte_index..byte_index + WORD_BYTES]
            .try_into()
            .expect("slice length checked"),
    );
    Felt::from_canonical(value).map_err(Into::into)
}

fn parent_hash(
    children: &[[Felt; HASH_WORDS]],
    arity: usize,
) -> Result<[Felt; HASH_WORDS], SetupError> {
    match arity {
        2 => Ok(parent_hash_arity2(children[0], children[1])),
        4 => Ok(parent_hash_arity4(children)),
        _ => Err(SetupError::UnsupportedConstantTreeArity {
            arity: u32::try_from(arity).unwrap_or(u32::MAX),
        }),
    }
}

fn parent_hash_arity2(left: [Felt; HASH_WORDS], right: [Felt; HASH_WORDS]) -> [Felt; HASH_WORDS] {
    let state = poseidon2_hash_8([
        left[0], left[1], left[2], left[3], right[0], right[1], right[2], right[3],
    ]);
    [state[0], state[1], state[2], state[3]]
}

fn parent_hash_arity4(children: &[[Felt; HASH_WORDS]]) -> [Felt; HASH_WORDS] {
    let state = poseidon2_hash_16([
        children[0][0],
        children[0][1],
        children[0][2],
        children[0][3],
        children[1][0],
        children[1][1],
        children[1][2],
        children[1][3],
        children[2][0],
        children[2][1],
        children[2][2],
        children[2][3],
        children[3][0],
        children[3][1],
        children[3][2],
        children[3][3],
    ]);
    [state[0], state[1], state[2], state[3]]
}

pub(crate) fn cpu_tree_parallelism_for_base_units(base_unit_parallelism: usize) -> usize {
    let available = thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1);
    let base_unit_parallelism = base_unit_parallelism.max(1);
    available
        .checked_div(base_unit_parallelism)
        .unwrap_or(1)
        .clamp(1, MAX_CPU_TREE_WORKERS)
}

fn default_cpu_tree_parallelism() -> usize {
    cpu_tree_parallelism_for_base_units(1)
}

fn effective_cpu_tree_parallelism(requested: usize, row_count: usize) -> usize {
    if row_count < MIN_PARALLEL_TREE_ROWS {
        return 1;
    }
    requested.clamp(1, MAX_CPU_TREE_WORKERS).min(row_count)
}

fn fill_leaf_level(
    leaves: &[u8],
    shape: ConstantTreeShape,
    level: &mut [[Felt; HASH_WORDS]],
    worker_count: usize,
) -> Result<(), SetupError> {
    if worker_count <= 1 {
        for (row, digest) in level.iter_mut().enumerate() {
            *digest = linear_hash_leaf_row(leaves, shape, row)?;
        }
        return Ok(());
    }

    let rows_per_worker = level.len().div_ceil(worker_count);
    thread::scope(|scope| {
        let handles = level
            .chunks_mut(rows_per_worker)
            .enumerate()
            .map(|(chunk_index, chunk)| {
                let start = chunk_index * rows_per_worker;
                scope.spawn(move || {
                    for (offset, digest) in chunk.iter_mut().enumerate() {
                        *digest = linear_hash_leaf_row(leaves, shape, start + offset)?;
                    }
                    Ok::<(), SetupError>(())
                })
            })
            .collect::<Vec<_>>();

        for handle in handles {
            match handle.join() {
                Ok(result) => result?,
                Err(error) => std::panic::resume_unwind(error),
            }
        }
        Ok::<(), SetupError>(())
    })
}

fn build_parent_level(
    level: &[[Felt; HASH_WORDS]],
    arity: usize,
    worker_count: usize,
) -> Result<Vec<[Felt; HASH_WORDS]>, SetupError> {
    let parent_count = level.len() / arity;
    let mut next = vec![[Felt::ZERO; HASH_WORDS]; parent_count];
    let worker_count = effective_cpu_tree_parallelism(worker_count, parent_count);
    if worker_count <= 1 {
        for (index, digest) in next.iter_mut().enumerate() {
            let child_start = index * arity;
            *digest = parent_hash(&level[child_start..child_start + arity], arity)?;
        }
        return Ok(next);
    }

    let parents_per_worker = parent_count.div_ceil(worker_count);
    thread::scope(|scope| {
        let handles = next
            .chunks_mut(parents_per_worker)
            .enumerate()
            .map(|(chunk_index, chunk)| {
                let start = chunk_index * parents_per_worker;
                scope.spawn(move || {
                    for (offset, digest) in chunk.iter_mut().enumerate() {
                        let child_start = (start + offset) * arity;
                        *digest = parent_hash(&level[child_start..child_start + arity], arity)?;
                    }
                    Ok::<(), SetupError>(())
                })
            })
            .collect::<Vec<_>>();

        for handle in handles {
            match handle.join() {
                Ok(result) => result?,
                Err(error) => std::panic::resume_unwind(error),
            }
        }
        Ok::<(), SetupError>(())
    })?;
    Ok(next)
}

#[cfg(feature = "cuda")]
struct CudaTreeLevel {
    digests: Vec<[Felt; HASH_WORDS]>,
    states: CudaDeviceBuffer,
    state_count: usize,
}

#[cfg(feature = "cuda")]
fn cuda_linear_hashes_with_states(
    rows: &[Vec<Felt>],
    arity: usize,
) -> Result<CudaTreeLevel, SetupError> {
    match arity {
        2 => cuda_linear_hashes_arity2_with_states(rows),
        4 => cuda_linear_hashes_arity4_with_states(rows),
        _ => Err(SetupError::UnsupportedConstantTreeArity {
            arity: u32::try_from(arity).unwrap_or(u32::MAX),
        }),
    }
}

#[cfg(feature = "cuda")]
fn cuda_linear_hashes_arity2_with_states(rows: &[Vec<Felt>]) -> Result<CudaTreeLevel, SetupError> {
    const WIDTH: usize = 8;

    let value_count = rows.first().map_or(0, Vec::len);
    let digests = if value_count <= HASH_WORDS {
        rows.iter()
            .map(|row| padded_digest(row))
            .collect::<Vec<_>>()
    } else {
        let mut current_states = zero_state_buffer(rows.len(), WIDTH)?;
        let mut offset = 0;
        while offset < value_count {
            let chunk_len = (value_count - offset).min(HASH_WORDS);
            let row_values = pack_linear_round_values(rows, offset, chunk_len)?;
            let row_values_buffer =
                CudaDeviceBuffer::from_u64_words(&row_values).map_err(cuda_backend_error)?;
            let mut next_states = CudaDeviceBuffer::new(
                rows.len()
                    .checked_mul(WIDTH)
                    .and_then(|words| words.checked_mul(WORD_BYTES))
                    .ok_or(SetupError::LengthOverflow)?,
            )
            .map_err(cuda_backend_error)?;
            cuda_poseidon2_width8_linear_round_device(
                &current_states,
                &row_values_buffer,
                &mut next_states,
                chunk_len,
            )
            .map_err(cuda_backend_error)?;
            current_states = next_states;
            offset += chunk_len;
        }

        let output = current_states.to_u64_words().map_err(cuda_backend_error)?;
        cuda_digests_from_state_words(&output, WIDTH)?
    };

    let state_words = cuda_state_words_from_digests(&digests, WIDTH)?;
    let states = cuda_device_buffer_from_words(&state_words)?;
    Ok(CudaTreeLevel {
        state_count: digests.len(),
        digests,
        states,
    })
}

#[cfg(feature = "cuda")]
fn cuda_linear_hashes_arity4_with_states(rows: &[Vec<Felt>]) -> Result<CudaTreeLevel, SetupError> {
    const RATE: usize = 12;
    const WIDTH: usize = 16;

    let value_count = rows.first().map_or(0, Vec::len);
    let digests = if value_count <= HASH_WORDS {
        rows.iter()
            .map(|row| padded_digest(row))
            .collect::<Vec<_>>()
    } else {
        let mut current_states = zero_state_buffer(rows.len(), WIDTH)?;
        let mut offset = 0;
        while offset < value_count {
            let chunk_len = (value_count - offset).min(RATE);
            let row_values = pack_linear_round_values(rows, offset, chunk_len)?;
            let row_values_buffer =
                CudaDeviceBuffer::from_u64_words(&row_values).map_err(cuda_backend_error)?;
            let mut next_states = CudaDeviceBuffer::new(
                rows.len()
                    .checked_mul(WIDTH)
                    .and_then(|words| words.checked_mul(WORD_BYTES))
                    .ok_or(SetupError::LengthOverflow)?,
            )
            .map_err(cuda_backend_error)?;
            cuda_poseidon2_width16_linear_round_device(
                &current_states,
                &row_values_buffer,
                &mut next_states,
                chunk_len,
            )
            .map_err(cuda_backend_error)?;
            current_states = next_states;
            offset += chunk_len;
        }

        let output = current_states.to_u64_words().map_err(cuda_backend_error)?;
        cuda_digests_from_state_words(&output, WIDTH)?
    };

    let state_words = cuda_state_words_from_digests(&digests, WIDTH)?;
    let states = cuda_device_buffer_from_words(&state_words)?;
    Ok(CudaTreeLevel {
        state_count: digests.len(),
        digests,
        states,
    })
}

#[cfg(feature = "cuda")]
fn cuda_parent_level_with_states(
    level: &CudaTreeLevel,
    arity: usize,
) -> Result<CudaTreeLevel, SetupError> {
    match arity {
        2 => cuda_parent_level_arity2_with_states(level),
        4 => cuda_parent_level_arity4_with_states(level),
        _ => Err(SetupError::UnsupportedConstantTreeArity {
            arity: u32::try_from(arity).unwrap_or(u32::MAX),
        }),
    }
}

#[cfg(feature = "cuda")]
fn cuda_parent_level_arity2_with_states(
    level: &CudaTreeLevel,
) -> Result<CudaTreeLevel, SetupError> {
    const WIDTH: usize = 8;

    let parent_state_count = level.state_count.div_ceil(2);
    let mut states = CudaDeviceBuffer::new(
        parent_state_count
            .checked_mul(WIDTH)
            .and_then(|words| words.checked_mul(WORD_BYTES))
            .ok_or(SetupError::LengthOverflow)?,
    )
    .map_err(cuda_backend_error)?;
    cuda_poseidon2_width8_merkle_parent_device(&level.states, &mut states)
        .map_err(cuda_backend_error)?;
    let words = states.to_u64_words().map_err(cuda_backend_error)?;
    let digests = cuda_digests_from_state_words(&words, WIDTH)?;
    Ok(CudaTreeLevel {
        state_count: parent_state_count,
        digests,
        states,
    })
}

#[cfg(feature = "cuda")]
fn cuda_parent_level_arity4_with_states(
    level: &CudaTreeLevel,
) -> Result<CudaTreeLevel, SetupError> {
    const WIDTH: usize = 16;

    let parent_state_count = level.state_count.div_ceil(4);
    let mut states = CudaDeviceBuffer::new(
        parent_state_count
            .checked_mul(WIDTH)
            .and_then(|words| words.checked_mul(WORD_BYTES))
            .ok_or(SetupError::LengthOverflow)?,
    )
    .map_err(cuda_backend_error)?;
    cuda_poseidon2_width16_merkle_parent_device(&level.states, &mut states)
        .map_err(cuda_backend_error)?;
    let words = states.to_u64_words().map_err(cuda_backend_error)?;
    let digests = cuda_digests_from_state_words(&words, WIDTH)?;
    Ok(CudaTreeLevel {
        state_count: parent_state_count,
        digests,
        states,
    })
}

#[cfg(feature = "cuda")]
fn cuda_device_buffer_from_words(words: &[u64]) -> Result<CudaDeviceBuffer, SetupError> {
    if words.is_empty() {
        CudaDeviceBuffer::new(0).map_err(cuda_backend_error)
    } else {
        CudaDeviceBuffer::from_u64_words(words).map_err(cuda_backend_error)
    }
}

#[cfg(feature = "cuda")]
fn cuda_state_words_from_digests(
    digests: &[[Felt; HASH_WORDS]],
    width: usize,
) -> Result<Vec<u64>, SetupError> {
    let mut words = vec![
        0_u64;
        digests
            .len()
            .checked_mul(width)
            .ok_or(SetupError::LengthOverflow)?
    ];
    for (index, digest) in digests.iter().enumerate() {
        let offset = index.checked_mul(width).ok_or(SetupError::LengthOverflow)?;
        for (word_index, value) in digest.iter().enumerate() {
            words[offset + word_index] = value.to_u64();
        }
    }
    Ok(words)
}

#[cfg(feature = "cuda")]
fn cuda_digests_from_state_words(
    words: &[u64],
    width: usize,
) -> Result<Vec<[Felt; HASH_WORDS]>, SetupError> {
    let mut digests = Vec::with_capacity(words.len() / width);
    for state in words.chunks_exact(width) {
        digests.push(digest_from_state_words(state)?);
    }
    Ok(digests)
}

#[cfg(feature = "cuda")]
fn cuda_backend_error(error: lzvm_accel::AccelError) -> SetupError {
    SetupError::CudaBackend(error.to_string())
}

#[cfg(all(test, feature = "cuda"))]
type CudaPoseidon2DeviceOp =
    fn(&CudaDeviceBuffer, &mut CudaDeviceBuffer) -> Result<(), lzvm_accel::AccelError>;

#[cfg(all(test, feature = "cuda"))]
fn cuda_poseidon2_width8_device_words(words: &[u64]) -> Result<Vec<u64>, SetupError> {
    cuda_poseidon2_words_device(words, cuda_poseidon2_width8_device)
}

#[cfg(all(test, feature = "cuda"))]
fn cuda_poseidon2_width16_device_words(words: &[u64]) -> Result<Vec<u64>, SetupError> {
    cuda_poseidon2_words_device(words, cuda_poseidon2_width16_device)
}

#[cfg(all(test, feature = "cuda"))]
fn cuda_poseidon2_words_device(
    words: &[u64],
    operation: CudaPoseidon2DeviceOp,
) -> Result<Vec<u64>, SetupError> {
    if words.is_empty() {
        return Ok(Vec::new());
    }

    let input_buffer = CudaDeviceBuffer::from_u64_words(words).map_err(cuda_backend_error)?;
    let mut output_buffer = CudaDeviceBuffer::new(
        words
            .len()
            .checked_mul(WORD_BYTES)
            .ok_or(SetupError::LengthOverflow)?,
    )
    .map_err(cuda_backend_error)?;
    operation(&input_buffer, &mut output_buffer).map_err(cuda_backend_error)?;
    output_buffer.to_u64_words().map_err(cuda_backend_error)
}

#[cfg(feature = "cuda")]
fn padded_digest(values: &[Felt]) -> [Felt; HASH_WORDS] {
    let mut digest = [Felt::ZERO; HASH_WORDS];
    digest[..values.len()].copy_from_slice(values);
    digest
}

#[cfg(feature = "cuda")]
fn pack_linear_round_values(
    rows: &[Vec<Felt>],
    offset: usize,
    chunk_len: usize,
) -> Result<Vec<u64>, SetupError> {
    let mut input = Vec::with_capacity(
        rows.len()
            .checked_mul(chunk_len)
            .ok_or(SetupError::LengthOverflow)?,
    );
    for row in rows {
        input.extend(
            row[offset..offset + chunk_len]
                .iter()
                .map(|value| value.to_u64()),
        );
    }
    Ok(input)
}

#[cfg(feature = "cuda")]
fn zero_state_buffer(row_count: usize, width: usize) -> Result<CudaDeviceBuffer, SetupError> {
    let words = row_count
        .checked_mul(width)
        .ok_or(SetupError::LengthOverflow)?;
    if words == 0 {
        return CudaDeviceBuffer::new(0).map_err(cuda_backend_error);
    }
    let zeros = vec![0_u64; words];
    CudaDeviceBuffer::from_u64_words(&zeros).map_err(cuda_backend_error)
}

#[cfg(feature = "cuda")]
fn digest_from_state_words(words: &[u64]) -> Result<[Felt; HASH_WORDS], SetupError> {
    debug_assert!(words.len() >= HASH_WORDS);
    let mut digest = [Felt::ZERO; HASH_WORDS];
    for (value, word) in digest.iter_mut().zip(words) {
        *value = Felt::from_canonical(*word)?;
    }
    Ok(digest)
}

fn append_digest(out: &mut Vec<u8>, digest: [Felt; HASH_WORDS]) {
    for value in digest {
        out.extend_from_slice(&value.to_le_bytes());
    }
}

#[cfg(all(test, feature = "cuda"))]
mod tests {
    use super::{
        cuda_parent_level_with_states, cuda_poseidon2_width16_device_words,
        cuda_poseidon2_width8_device_words, CudaTreeLevel, HASH_WORDS,
    };
    use lzvm_field::{poseidon2_hash_16, poseidon2_hash_8, Felt};

    #[test]
    fn device_poseidon2_state_hashes_match_cpu_reference() {
        let width8_input = (1_u64..=16).collect::<Vec<_>>();
        let width8_expected = width8_input
            .chunks_exact(8)
            .flat_map(|chunk| poseidon2_hash_8(felt_array::<8>(chunk)).map(Felt::to_u64))
            .collect::<Vec<_>>();
        let width8_actual = cuda_poseidon2_width8_device_words(&width8_input)
            .expect("width-8 device hash should run");
        assert_eq!(width8_actual, width8_expected);

        let width16_input = (17_u64..=48).collect::<Vec<_>>();
        let width16_expected = width16_input
            .chunks_exact(16)
            .flat_map(|chunk| poseidon2_hash_16(felt_array::<16>(chunk)).map(Felt::to_u64))
            .collect::<Vec<_>>();
        let width16_actual = cuda_poseidon2_width16_device_words(&width16_input)
            .expect("width-16 device hash should run");
        assert_eq!(width16_actual, width16_expected);
    }

    #[test]
    fn cuda_parent_level_with_states_matches_cpu_reference() {
        let width8_level = CudaTreeLevel {
            digests: vec![
                digest([1, 2, 3, 4]),
                digest([5, 6, 7, 8]),
                digest([9, 10, 11, 12]),
            ],
            states: super::CudaDeviceBuffer::from_u64_words(&[
                1, 2, 3, 4, 101, 102, 103, 104, 5, 6, 7, 8, 201, 202, 203, 204, 9, 10, 11, 12, 301,
                302, 303, 304,
            ])
            .expect("device buffer should allocate"),
            state_count: 3,
        };
        let width8_next = cuda_parent_level_with_states(&width8_level, 2)
            .expect("width-8 parent level should hash");
        assert_eq!(width8_next.state_count, 2);
        assert_eq!(
            width8_next.digests,
            vec![
                super::parent_hash_arity2(width8_level.digests[0], width8_level.digests[1]),
                super::parent_hash_arity2(width8_level.digests[2], [Felt::ZERO; HASH_WORDS]),
            ]
        );

        let width16_level = CudaTreeLevel {
            digests: vec![
                digest([1, 2, 3, 4]),
                digest([5, 6, 7, 8]),
                digest([9, 10, 11, 12]),
                digest([13, 14, 15, 16]),
                digest([17, 18, 19, 20]),
            ],
            states: super::CudaDeviceBuffer::from_u64_words(&[
                1, 2, 3, 4, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 5, 6, 7, 8,
                201, 202, 203, 204, 205, 206, 207, 208, 209, 210, 211, 212, 9, 10, 11, 12, 301,
                302, 303, 304, 305, 306, 307, 308, 309, 310, 311, 312, 13, 14, 15, 16, 401, 402,
                403, 404, 405, 406, 407, 408, 409, 410, 411, 412, 17, 18, 19, 20, 501, 502, 503,
                504, 505, 506, 507, 508, 509, 510, 511, 512,
            ])
            .expect("device buffer should allocate"),
            state_count: 5,
        };
        let width16_next = cuda_parent_level_with_states(&width16_level, 4)
            .expect("width-16 parent level should hash");
        assert_eq!(width16_next.state_count, 2);
        assert_eq!(
            width16_next.digests,
            vec![
                super::parent_hash_arity4(&width16_level.digests[0..4]),
                super::parent_hash_arity4(&[
                    width16_level.digests[4],
                    [Felt::ZERO; HASH_WORDS],
                    [Felt::ZERO; HASH_WORDS],
                    [Felt::ZERO; HASH_WORDS],
                ]),
            ]
        );
    }

    fn felt_array<const WIDTH: usize>(words: &[u64]) -> [Felt; WIDTH] {
        let mut values = [Felt::ZERO; WIDTH];
        for (value, word) in values.iter_mut().zip(words) {
            *value = Felt::from_u64(*word);
        }
        values
    }

    fn digest(values: [u64; HASH_WORDS]) -> [Felt; HASH_WORDS] {
        values.map(Felt::from_u64)
    }
}
