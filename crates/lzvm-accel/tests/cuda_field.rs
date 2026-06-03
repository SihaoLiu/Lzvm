#[cfg(feature = "cuda")]
use lzvm_accel::{cuda_goldilocks_add, cuda_goldilocks_mul};
#[cfg(feature = "cuda")]
use lzvm_accel::{
    cuda_goldilocks_butterfly, cuda_goldilocks_coset_extend, cuda_goldilocks_coset_extend_device,
    cuda_goldilocks_coset_extend_row_major_columns,
    cuda_goldilocks_coset_extend_row_major_columns_device,
    cuda_goldilocks_coset_extend_row_major_columns_output_bytes, cuda_goldilocks_intt,
    cuda_goldilocks_ntt, cuda_keccak256_fixed, cuda_poseidon2_width16,
    cuda_poseidon2_width16_device, cuda_poseidon2_width16_linear_round_device,
    cuda_poseidon2_width16_linear_round_row_major_device,
    cuda_poseidon2_width16_merkle_parent_device, cuda_poseidon2_width16_merkle_root_device,
    cuda_poseidon2_width4, cuda_poseidon2_width4_device, cuda_poseidon2_width4_find_nonce,
    cuda_poseidon2_width8, cuda_poseidon2_width8_device, cuda_poseidon2_width8_linear_round_device,
    cuda_poseidon2_width8_linear_round_row_major_device,
    cuda_poseidon2_width8_merkle_parent_device, cuda_poseidon2_width8_merkle_root_device,
    cuda_setup_init, AccelError, CudaDeviceBuffer,
};
#[cfg(feature = "cuda")]
use lzvm_crypto::keccak256;
#[cfg(feature = "cuda")]
use lzvm_field::{
    coset_extend_evaluations, intt_in_place, ntt_in_place, poseidon2_hash_16, poseidon2_hash_4,
    poseidon2_hash_8, Felt,
};

#[cfg(feature = "cuda")]
const MODULUS: u64 = 0xffff_ffff_0000_0001;

#[cfg(feature = "cuda")]
fn add_mod(lhs: u64, rhs: u64) -> u64 {
    ((lhs as u128 + rhs as u128) % MODULUS as u128) as u64
}

#[cfg(feature = "cuda")]
fn mul_mod(lhs: u64, rhs: u64) -> u64 {
    ((lhs as u128 * rhs as u128) % MODULUS as u128) as u64
}

#[cfg(feature = "cuda")]
fn sub_mod(lhs: u64, rhs: u64) -> u64 {
    if lhs >= rhs {
        lhs - rhs
    } else {
        MODULUS - (rhs - lhs)
    }
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_adds_goldilocks_vectors() {
    let lhs = vec![
        0,
        1,
        MODULUS - 1,
        0xffff_ffff,
        MODULUS - 9,
        123_456_789,
        9_876_543_210,
        MODULUS / 2,
    ];
    let rhs = vec![
        7,
        MODULUS - 1,
        5,
        0xffff_ffff,
        27,
        987_654_321,
        MODULUS - 5,
        MODULUS / 2 + 3,
    ];
    let expected = lhs
        .iter()
        .zip(&rhs)
        .map(|(lhs, rhs)| add_mod(*lhs, *rhs))
        .collect::<Vec<_>>();

    let actual = cuda_goldilocks_add(&lhs, &rhs).expect("cuda addition should run");

    assert_eq!(actual, expected);
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_rejects_mismatched_vector_lengths() {
    let error = cuda_goldilocks_add(&[1, 2, 3], &[4, 5]).expect_err("lengths should be checked");

    assert!(error.to_string().contains("length mismatch"));
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_multiplies_goldilocks_vectors() {
    let lhs = vec![
        0,
        1,
        MODULUS - 1,
        0xffff_ffff,
        MODULUS - 9,
        123_456_789,
        9_876_543_210,
        MODULUS / 2,
    ];
    let rhs = vec![
        7,
        MODULUS - 1,
        5,
        0xffff_ffff,
        27,
        987_654_321,
        MODULUS - 5,
        MODULUS / 2 + 3,
    ];
    let expected = lhs
        .iter()
        .zip(&rhs)
        .map(|(lhs, rhs)| mul_mod(*lhs, *rhs))
        .collect::<Vec<_>>();

    let actual = cuda_goldilocks_mul(&lhs, &rhs).expect("cuda multiplication should run");

    assert_eq!(actual, expected);
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_computes_goldilocks_butterflies() {
    let even = vec![
        0,
        1,
        MODULUS - 1,
        0xffff_ffff,
        MODULUS - 9,
        123_456_789,
        9_876_543_210,
        MODULUS / 2,
    ];
    let odd = vec![
        7,
        MODULUS - 1,
        5,
        0xffff_ffff,
        27,
        987_654_321,
        MODULUS - 5,
        MODULUS / 2 + 3,
    ];
    let twiddle = vec![1, 3, 5, 7, 11, 13, 17, 19];
    let expected = even
        .iter()
        .zip(&odd)
        .zip(&twiddle)
        .map(|((even, odd), twiddle)| {
            let scaled = mul_mod(*odd, *twiddle);
            (add_mod(*even, scaled), sub_mod(*even, scaled))
        })
        .collect::<Vec<_>>();
    let expected_even = expected.iter().map(|(value, _)| *value).collect::<Vec<_>>();
    let expected_odd = expected.iter().map(|(_, value)| *value).collect::<Vec<_>>();

    let (actual_even, actual_odd) =
        cuda_goldilocks_butterfly(&even, &odd, &twiddle).expect("cuda butterfly should run");

    assert_eq!(actual_even, expected_even);
    assert_eq!(actual_odd, expected_odd);
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_device_buffer_round_trips_bytes() {
    let input = (0_u8..64)
        .map(|value| value.wrapping_mul(3))
        .collect::<Vec<_>>();
    let mut buffer = CudaDeviceBuffer::new(input.len()).expect("device buffer should allocate");

    assert_eq!(buffer.len(), input.len());
    assert!(!buffer.is_empty());

    buffer
        .copy_from(&input)
        .expect("host bytes should copy to device");
    let output = buffer.to_vec().expect("device bytes should copy to host");

    assert_eq!(output, input);
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_device_buffer_copies_byte_ranges_to_host() {
    let input = (0_u8..96)
        .map(|value| value.wrapping_mul(5).wrapping_add(7))
        .collect::<Vec<_>>();
    let mut buffer = CudaDeviceBuffer::new(input.len()).expect("device buffer should allocate");
    buffer
        .copy_from(&input)
        .expect("host bytes should copy to device");

    let mut output = vec![0_u8; 17];
    buffer
        .copy_range_to(29, &mut output)
        .expect("device range should copy to host");

    assert_eq!(output, input[29..46]);
    assert!(buffer.copy_range_to(90, &mut output).is_err());
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_device_buffer_zeroed_initializes_bytes() {
    let buffer = CudaDeviceBuffer::zeroed(96).expect("device buffer should allocate");

    assert_eq!(buffer.len(), 96);
    assert_eq!(
        buffer.to_vec().expect("device bytes should copy to host"),
        vec![0_u8; 96]
    );

    let empty = CudaDeviceBuffer::zeroed(0).expect("empty device buffer should allocate");
    assert!(empty.is_empty());
    assert_eq!(
        empty.to_vec().expect("empty bytes should copy to host"),
        Vec::<u8>::new()
    );
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_device_buffer_round_trips_u64_words() {
    let input = vec![0, 1, MODULUS - 1, 0xffff_ffff, MODULUS / 2, 123_456_789];
    let buffer = CudaDeviceBuffer::from_u64_words(&input).expect("word buffer should allocate");

    assert_eq!(buffer.len(), input.len() * 8);

    let output = buffer
        .to_u64_words()
        .expect("device words should copy back to host");

    assert_eq!(output, input);
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_device_buffer_copies_u64_words_directly() {
    let input = vec![
        0,
        1,
        MODULUS - 1,
        0x0102_0304_0506_0708,
        0x8877_6655_4433_2211,
        MODULUS / 2,
    ];
    let expected_bytes = input
        .iter()
        .flat_map(|word| word.to_le_bytes())
        .collect::<Vec<_>>();
    let mut buffer = CudaDeviceBuffer::new(input.len() * 8).expect("device buffer should allocate");

    buffer
        .copy_from_u64_words(&input)
        .expect("device words should copy from host");

    assert_eq!(
        buffer.to_vec().expect("device bytes should copy to host"),
        expected_bytes
    );
    assert_eq!(
        buffer
            .to_u64_words()
            .expect("device words should copy back to host"),
        input
    );

    let mut byte_buffer =
        CudaDeviceBuffer::new(expected_bytes.len()).expect("device byte buffer should allocate");
    byte_buffer
        .copy_from(&expected_bytes)
        .expect("device bytes should copy from host");
    assert_eq!(
        byte_buffer
            .to_u64_words()
            .expect("device bytes should copy back as words"),
        input
    );
    assert_eq!(
        buffer
            .copy_from_u64_words(&input[..input.len() - 1])
            .expect_err("short word input should fail"),
        AccelError::LengthMismatch {
            lhs: input.len() * 8,
            rhs: (input.len() - 1) * 8,
        }
    );

    let mut empty = CudaDeviceBuffer::new(0).expect("empty device buffer should allocate");
    empty
        .copy_from_u64_words(&[])
        .expect("empty word input should copy");
    assert_eq!(
        empty
            .to_u64_words()
            .expect("empty word buffer should copy back"),
        Vec::<u64>::new()
    );

    let odd = CudaDeviceBuffer::new(7).expect("odd device buffer should allocate");
    assert_eq!(
        odd.to_u64_words().expect_err("odd byte count should fail"),
        AccelError::LengthMismatch { lhs: 7, rhs: 0 }
    );
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_device_buffer_copies_state_prefix_words() {
    let input = vec![
        1, 2, 3, 4, 101, 102, 103, 104, 5, 6, 7, 8, 201, 202, 203, 204,
    ];
    let buffer = CudaDeviceBuffer::from_u64_words(&input).expect("word buffer should allocate");

    let output = buffer
        .to_state_prefix_u64_words(2, 8, 4)
        .expect("state prefixes should copy back to host");

    assert_eq!(output, vec![1, 2, 3, 4, 5, 6, 7, 8]);
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_device_buffer_expands_state_prefix_words() {
    let input = vec![1, 2, 3, 4, 5, 6, 7, 8];

    let buffer = CudaDeviceBuffer::from_state_prefix_u64_words(&input, 2, 8, 4)
        .expect("state prefixes should expand into padded states");

    assert_eq!(
        buffer
            .to_u64_words()
            .expect("expanded state words should copy back"),
        vec![1, 2, 3, 4, 0, 0, 0, 0, 5, 6, 7, 8, 0, 0, 0, 0]
    );

    let zero_prefix = CudaDeviceBuffer::from_state_prefix_u64_words(&[], 2, 8, 0)
        .expect("zero-width prefixes should expand into zeroed states");
    assert_eq!(
        zero_prefix
            .to_u64_words()
            .expect("zero-prefix states should copy back"),
        vec![0; 16]
    );

    let empty = CudaDeviceBuffer::from_state_prefix_u64_words(&[], 0, 8, 4)
        .expect("empty state prefix input should allocate an empty buffer");
    assert_eq!(
        empty.to_u64_words().expect("empty states should copy back"),
        Vec::<u64>::new()
    );
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_device_buffer_state_prefix_checks_shapes() {
    let input = vec![
        1, 2, 3, 4, 101, 102, 103, 104, 5, 6, 7, 8, 201, 202, 203, 204,
    ];
    let buffer = CudaDeviceBuffer::from_u64_words(&input).expect("word buffer should allocate");
    let empty = CudaDeviceBuffer::new(0).expect("empty device buffer should allocate");

    assert_eq!(
        empty
            .to_state_prefix_u64_words(0, 8, 4)
            .expect("empty state prefix copy should return no words"),
        Vec::<u64>::new()
    );
    assert_eq!(
        buffer
            .to_state_prefix_u64_words(2, 8, 0)
            .expect("zero-width prefix copy should return no words"),
        Vec::<u64>::new()
    );

    let prefix_error = buffer
        .to_state_prefix_u64_words(2, 4, 5)
        .expect_err("prefix wider than state should be rejected");
    assert!(prefix_error.to_string().contains("invalid field domain"));

    let length_error = buffer
        .to_state_prefix_u64_words(1, 8, 4)
        .expect_err("buffer byte length should match state shape");
    assert!(length_error.to_string().contains("length mismatch"));

    let expand_prefix_error = CudaDeviceBuffer::from_state_prefix_u64_words(&input, 2, 4, 5)
        .expect_err("prefix wider than state should be rejected");
    assert!(expand_prefix_error
        .to_string()
        .contains("invalid field domain"));

    let expand_length_error = CudaDeviceBuffer::from_state_prefix_u64_words(&input, 1, 8, 4)
        .expect_err("prefix input length should match state shape");
    assert!(expand_length_error.to_string().contains("length mismatch"));

    assert_eq!(
        CudaDeviceBuffer::from_state_prefix_u64_words(&[], 1, 0, 0)
            .expect_err("zero-width states should be rejected"),
        AccelError::InvalidDomain { bits: 0, len: 0 }
    );
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_computes_forward_ntt() {
    let input = vec![3, 5, 7, 11, 13, 17, 19, 23];
    let mut expected = input
        .iter()
        .map(|value| Felt::from_u64(*value))
        .collect::<Vec<_>>();
    ntt_in_place(&mut expected, 3).expect("cpu ntt should run");
    let expected = expected.into_iter().map(Felt::to_u64).collect::<Vec<_>>();

    let actual = cuda_goldilocks_ntt(&input, 3).expect("cuda ntt should run");

    assert_eq!(actual, expected);
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_initializes_setup_constants_before_ntt() {
    cuda_setup_init(4).expect("cuda setup should initialize");

    let input = vec![3, 5, 7, 11, 13, 17, 19, 23];
    let mut expected = input
        .iter()
        .map(|value| Felt::from_u64(*value))
        .collect::<Vec<_>>();
    ntt_in_place(&mut expected, 3).expect("cpu ntt should run");
    let expected = expected.into_iter().map(Felt::to_u64).collect::<Vec<_>>();

    let actual = cuda_goldilocks_ntt(&input, 3).expect("cuda ntt should run");

    assert_eq!(actual, expected);
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_computes_inverse_ntt() {
    let input = vec![3, 5, 7, 11];
    let mut expected = input
        .iter()
        .map(|value| Felt::from_u64(*value))
        .collect::<Vec<_>>();
    intt_in_place(&mut expected, 2).expect("cpu intt should run");
    let expected = expected.into_iter().map(Felt::to_u64).collect::<Vec<_>>();

    let actual = cuda_goldilocks_intt(&input, 2).expect("cuda intt should run");

    assert_eq!(actual, expected);
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_extends_evaluations_over_shifted_cosets() {
    let input = vec![5, 1, 9, 9];
    let source = input
        .iter()
        .map(|value| Felt::from_u64(*value))
        .collect::<Vec<_>>();
    let expected = coset_extend_evaluations(&source, 2, 4)
        .expect("cpu coset extension should run")
        .into_iter()
        .map(Felt::to_u64)
        .collect::<Vec<_>>();

    let actual =
        cuda_goldilocks_coset_extend(&input, 2, 4).expect("cuda coset extension should run");

    assert_eq!(actual, expected);
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_extends_evaluations_from_device_memory() {
    let input = vec![5, 1, 9, 9];
    let source = input
        .iter()
        .map(|value| Felt::from_u64(*value))
        .collect::<Vec<_>>();
    let expected = coset_extend_evaluations(&source, 2, 4)
        .expect("cpu coset extension should run")
        .into_iter()
        .map(Felt::to_u64)
        .collect::<Vec<_>>();

    let input_buffer =
        CudaDeviceBuffer::from_u64_words(&input).expect("input device buffer should allocate");
    let mut output_buffer =
        CudaDeviceBuffer::new(expected.len() * 8).expect("output device buffer should allocate");

    cuda_goldilocks_coset_extend_device(&input_buffer, &mut output_buffer, 2, 4)
        .expect("cuda coset extension should run from device memory");

    let actual = output_buffer
        .to_u64_words()
        .expect("device words should copy back to host");

    assert_eq!(actual, expected);
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_extends_row_major_columns_over_shifted_cosets() {
    let input = vec![5, 9, 2, 1, 9, 4, 3, 7, 6, 8, 11, 10];
    let source_bits = 2;
    let target_bits = 4;
    let column_count = 3;
    let source_rows = 1_usize << source_bits;
    let target_rows = 1_usize << target_bits;

    let extended_columns = (0..column_count)
        .map(|column| {
            let source = (0..source_rows)
                .map(|row| Felt::from_u64(input[row * column_count + column]))
                .collect::<Vec<_>>();
            coset_extend_evaluations(&source, source_bits, target_bits)
                .expect("cpu coset extension should run")
        })
        .collect::<Vec<_>>();
    let expected = (0..target_rows)
        .flat_map(|row| {
            extended_columns
                .iter()
                .map(move |column| column[row].to_u64())
        })
        .collect::<Vec<_>>();

    let actual = cuda_goldilocks_coset_extend_row_major_columns(
        &input,
        column_count,
        source_bits,
        target_bits,
    )
    .expect("cuda row-major column extension should run");

    assert_eq!(actual, expected);
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_extends_row_major_columns_from_device_memory() {
    let input = vec![5, 9, 2, 1, 9, 4, 3, 7, 6, 8, 11, 10];
    let source_bits = 2;
    let target_bits = 4;
    let column_count = 3;
    let expected = cuda_goldilocks_coset_extend_row_major_columns(
        &input,
        column_count,
        source_bits,
        target_bits,
    )
    .expect("host row-major extension should run");
    let input_buffer =
        CudaDeviceBuffer::from_u64_words(&input).expect("input device buffer should allocate");
    let mut output_buffer =
        CudaDeviceBuffer::new(expected.len() * 8).expect("output device buffer should allocate");

    cuda_goldilocks_coset_extend_row_major_columns_device(
        &input_buffer,
        &mut output_buffer,
        column_count,
        source_bits,
        target_bits,
    )
    .expect("device row-major extension should run");
    let actual = output_buffer
        .to_u64_words()
        .expect("device output should copy back");

    assert_eq!(actual, expected);
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_reports_row_major_extension_output_bytes_before_allocation() {
    let bytes = cuda_goldilocks_coset_extend_row_major_columns_output_bytes(12, 3, 2, 4)
        .expect("output byte count should compute");

    assert_eq!(bytes, (1_usize << 4) * 3 * 8);

    let error = cuda_goldilocks_coset_extend_row_major_columns_output_bytes(2, 1, 2, 3)
        .expect_err("source row mismatch should be rejected");

    assert_eq!(
        error,
        lzvm_accel::AccelError::InvalidDomain { bits: 2, len: 2 }
    );
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_rejects_row_major_device_output_length_mismatch() {
    let input = vec![5, 9, 2, 1, 9, 4, 3, 7, 6, 8, 11, 10];
    let source_bits = 2;
    let target_bits = 4;
    let column_count = 3;
    let input_buffer =
        CudaDeviceBuffer::from_u64_words(&input).expect("input device buffer should allocate");
    let mut output_buffer =
        CudaDeviceBuffer::new(((1_usize << target_bits) * column_count - 1) * 8)
            .expect("output device buffer should allocate");

    let error = cuda_goldilocks_coset_extend_row_major_columns_device(
        &input_buffer,
        &mut output_buffer,
        column_count,
        source_bits,
        target_bits,
    )
    .expect_err("short output should be rejected");

    assert!(error.to_string().contains("length mismatch"));
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_hashes_poseidon2_width_4_states() {
    let input = (0_u64..12).collect::<Vec<_>>();
    let expected = input
        .chunks_exact(4)
        .flat_map(|chunk| {
            let state = std::array::from_fn(|index| Felt::from_u64(chunk[index]));
            poseidon2_hash_4(state).map(Felt::to_u64)
        })
        .collect::<Vec<_>>();

    let actual = cuda_poseidon2_width4(&input).expect("cuda hash should run");

    assert_eq!(actual, expected);
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_hashes_poseidon2_width_4_states_from_device_memory() {
    let input = (0_u64..12).collect::<Vec<_>>();
    let expected = input
        .chunks_exact(4)
        .flat_map(|chunk| {
            let state = std::array::from_fn(|index| Felt::from_u64(chunk[index]));
            poseidon2_hash_4(state).map(Felt::to_u64)
        })
        .collect::<Vec<_>>();

    let input_buffer =
        CudaDeviceBuffer::from_u64_words(&input).expect("input device buffer should allocate");
    let mut output_buffer =
        CudaDeviceBuffer::new(input.len() * 8).expect("output device buffer should allocate");
    cuda_poseidon2_width4_device(&input_buffer, &mut output_buffer)
        .expect("cuda device hash should run");
    let actual = output_buffer
        .to_u64_words()
        .expect("device words should copy back to host");

    assert_eq!(actual, expected);
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_finds_poseidon2_width_4_nonce_ranges() {
    let challenge = [0_u64, 1, 2];
    let target = 1_u64 << 62;
    let expected = (0_u64..128)
        .find(|candidate| {
            let state = [
                Felt::from_u64(challenge[0]),
                Felt::from_u64(challenge[1]),
                Felt::from_u64(challenge[2]),
                Felt::from_u64(*candidate),
            ];
            poseidon2_hash_4(state)[0].to_u64() < target
        })
        .expect("fixture should contain a matching nonce");

    let actual = cuda_poseidon2_width4_find_nonce(challenge, 0, 128, target)
        .expect("cuda nonce search should run");

    assert_eq!(actual, Some(expected));
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_reports_empty_poseidon2_width_4_nonce_ranges() {
    let actual = cuda_poseidon2_width4_find_nonce([0, 1, 2], 0, 16, 0)
        .expect("cuda nonce search should run");

    assert_eq!(actual, None);
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_hashes_poseidon2_width_8_states() {
    let input = (0_u64..16).collect::<Vec<_>>();
    let expected = input
        .chunks_exact(8)
        .flat_map(|chunk| {
            let state = std::array::from_fn(|index| Felt::from_u64(chunk[index]));
            poseidon2_hash_8(state).map(Felt::to_u64)
        })
        .collect::<Vec<_>>();

    let actual = cuda_poseidon2_width8(&input).expect("cuda hash should run");

    assert_eq!(actual, expected);
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_hashes_poseidon2_width_8_states_from_device_memory() {
    let input = (0_u64..16).collect::<Vec<_>>();
    let expected = input
        .chunks_exact(8)
        .flat_map(|chunk| {
            let state = std::array::from_fn(|index| Felt::from_u64(chunk[index]));
            poseidon2_hash_8(state).map(Felt::to_u64)
        })
        .collect::<Vec<_>>();

    let input_buffer =
        CudaDeviceBuffer::from_u64_words(&input).expect("input device buffer should allocate");
    let mut output_buffer =
        CudaDeviceBuffer::new(input.len() * 8).expect("output device buffer should allocate");
    cuda_poseidon2_width8_device(&input_buffer, &mut output_buffer)
        .expect("cuda device hash should run");
    let actual = output_buffer
        .to_u64_words()
        .expect("device words should copy back to host");

    assert_eq!(actual, expected);
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_linear_round_poseidon2_width_8_states_from_device_memory() {
    let current_states = CudaDeviceBuffer::from_u64_words(&[0_u64; 24])
        .expect("current state buffer should allocate");
    let row_values = CudaDeviceBuffer::from_u64_words(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12])
        .expect("row buffer should allocate");
    let mut output_states =
        CudaDeviceBuffer::new(24 * 8).expect("output state buffer should allocate");

    cuda_poseidon2_width8_linear_round_device(&current_states, &row_values, &mut output_states, 4)
        .expect("cuda linear round should run");

    let expected = [
        poseidon2_hash_8(std::array::from_fn(|index| {
            Felt::from_u64([1, 2, 3, 4, 0, 0, 0, 0][index])
        }))
        .map(Felt::to_u64),
        poseidon2_hash_8(std::array::from_fn(|index| {
            Felt::from_u64([5, 6, 7, 8, 0, 0, 0, 0][index])
        }))
        .map(Felt::to_u64),
        poseidon2_hash_8(std::array::from_fn(|index| {
            Felt::from_u64([9, 10, 11, 12, 0, 0, 0, 0][index])
        }))
        .map(Felt::to_u64),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();
    let actual = output_states
        .to_u64_words()
        .expect("device words should copy back to host");

    assert_eq!(actual, expected);
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_linear_round_poseidon2_width_8_gathers_row_major_device_memory() {
    let current_states = CudaDeviceBuffer::from_u64_words(&[
        101, 102, 103, 104, 0, 0, 0, 0, 201, 202, 203, 204, 0, 0, 0, 0, 301, 302, 303, 304, 0, 0,
        0, 0,
    ])
    .expect("current state buffer should allocate");
    let row_values = CudaDeviceBuffer::from_u64_words(&[
        1, 2, 3, 4, 5, 6, 11, 12, 13, 14, 15, 16, 21, 22, 23, 24, 25, 26,
    ])
    .expect("row-major value buffer should allocate");
    let mut output_states =
        CudaDeviceBuffer::new(24 * 8).expect("output state buffer should allocate");

    cuda_poseidon2_width8_linear_round_row_major_device(
        &current_states,
        &row_values,
        &mut output_states,
        6,
        2,
        3,
    )
    .expect("cuda row-major linear round should run");

    let expected = [
        poseidon2_hash_8(std::array::from_fn(|index| {
            Felt::from_u64([3, 4, 5, 0, 101, 102, 103, 104][index])
        }))
        .map(Felt::to_u64),
        poseidon2_hash_8(std::array::from_fn(|index| {
            Felt::from_u64([13, 14, 15, 0, 201, 202, 203, 204][index])
        }))
        .map(Felt::to_u64),
        poseidon2_hash_8(std::array::from_fn(|index| {
            Felt::from_u64([23, 24, 25, 0, 301, 302, 303, 304][index])
        }))
        .map(Felt::to_u64),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();
    let actual = output_states
        .to_u64_words()
        .expect("device words should copy back to host");

    assert_eq!(actual, expected);
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_linear_round_row_major_device_rejects_invalid_shapes() {
    let current_states =
        CudaDeviceBuffer::zeroed(16 * 8).expect("current state buffer should allocate");
    let row_values = CudaDeviceBuffer::from_u64_words(&(1_u64..=12).collect::<Vec<_>>())
        .expect("row-major value buffer should allocate");
    let short_row_values = CudaDeviceBuffer::from_u64_words(&[1, 2, 3, 4, 5])
        .expect("short row-major value buffer should allocate");
    let mut output_states =
        CudaDeviceBuffer::new(16 * 8).expect("output state buffer should allocate");

    let error = cuda_poseidon2_width8_linear_round_row_major_device(
        &current_states,
        &row_values,
        &mut output_states,
        6,
        5,
        2,
    )
    .expect_err("window outside row should be rejected");
    assert!(error.to_string().contains("invalid field domain"));

    let error = cuda_poseidon2_width8_linear_round_row_major_device(
        &current_states,
        &short_row_values,
        &mut output_states,
        6,
        1,
        3,
    )
    .expect_err("row-major byte count should be checked");
    assert!(error.to_string().contains("length mismatch"));
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_hashes_poseidon2_width_8_parent_states_from_device_memory() {
    let input = vec![
        1, 2, 3, 4, 101, 102, 103, 104, 5, 6, 7, 8, 201, 202, 203, 204, 9, 10, 11, 12, 301, 302,
        303, 304,
    ];
    let expected_inputs = [[1, 2, 3, 4, 5, 6, 7, 8], [9, 10, 11, 12, 0, 0, 0, 0]];
    let expected = expected_inputs
        .into_iter()
        .flat_map(|state| {
            poseidon2_hash_8(state.map(Felt::from_u64))
                .map(Felt::to_u64)
                .into_iter()
        })
        .collect::<Vec<_>>();

    let input_buffer =
        CudaDeviceBuffer::from_u64_words(&input).expect("input device buffer should allocate");
    let mut output_buffer =
        CudaDeviceBuffer::new(expected.len() * 8).expect("output device buffer should allocate");
    cuda_poseidon2_width8_merkle_parent_device(&input_buffer, &mut output_buffer)
        .expect("cuda device parent hash should run");
    let actual = output_buffer
        .to_u64_words()
        .expect("device words should copy back to host");

    assert_eq!(actual, expected);
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_poseidon2_width8_merkle_root_device_matches_cpu_reference() {
    let input = vec![
        1, 2, 3, 4, 101, 102, 103, 104, 5, 6, 7, 8, 201, 202, 203, 204, 9, 10, 11, 12, 301, 302,
        303, 304, 13, 14, 15, 16, 401, 402, 403, 404, 17, 18, 19, 20, 501, 502, 503, 504,
    ];
    let expected = cpu_merkle_root_width8(&input);
    let input_buffer =
        CudaDeviceBuffer::from_u64_words(&input).expect("input device buffer should allocate");

    let actual = cuda_poseidon2_width8_merkle_root_device(&input_buffer)
        .expect("cuda device root hash should run");

    assert_eq!(actual, expected);
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_hashes_poseidon2_width_16_states() {
    let input = (0_u64..32).collect::<Vec<_>>();
    let expected = input
        .chunks_exact(16)
        .flat_map(|chunk| {
            let state = std::array::from_fn(|index| Felt::from_u64(chunk[index]));
            poseidon2_hash_16(state).map(Felt::to_u64)
        })
        .collect::<Vec<_>>();

    let actual = cuda_poseidon2_width16(&input).expect("cuda hash should run");

    assert_eq!(actual, expected);
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_linear_round_poseidon2_width_16_states_from_device_memory() {
    let current_states = CudaDeviceBuffer::from_u64_words(&[0_u64; 48])
        .expect("current state buffer should allocate");
    let row_values = CudaDeviceBuffer::from_u64_words(&[
        1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
        26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36,
    ])
    .expect("row buffer should allocate");
    let mut output_states =
        CudaDeviceBuffer::new(48 * 8).expect("output state buffer should allocate");

    cuda_poseidon2_width16_linear_round_device(
        &current_states,
        &row_values,
        &mut output_states,
        12,
    )
    .expect("cuda linear round should run");

    let expected = [
        poseidon2_hash_16(std::array::from_fn(|index| {
            Felt::from_u64([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 0, 0, 0, 0][index])
        }))
        .map(Felt::to_u64),
        poseidon2_hash_16(std::array::from_fn(|index| {
            Felt::from_u64([13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 0, 0, 0, 0][index])
        }))
        .map(Felt::to_u64),
        poseidon2_hash_16(std::array::from_fn(|index| {
            Felt::from_u64([25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 0, 0, 0, 0][index])
        }))
        .map(Felt::to_u64),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();
    let actual = output_states
        .to_u64_words()
        .expect("device words should copy back to host");

    assert_eq!(actual, expected);
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_linear_round_poseidon2_width_16_gathers_row_major_device_memory() {
    let current_states = CudaDeviceBuffer::from_u64_words(&[
        101, 102, 103, 104, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 201, 202, 203, 204, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0,
    ])
    .expect("current state buffer should allocate");
    let row_values = CudaDeviceBuffer::from_u64_words(&[
        1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 1011, 1012, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30,
        31, 32, 2011, 2012,
    ])
    .expect("row-major value buffer should allocate");
    let mut output_states =
        CudaDeviceBuffer::new(32 * 8).expect("output state buffer should allocate");

    cuda_poseidon2_width16_linear_round_row_major_device(
        &current_states,
        &row_values,
        &mut output_states,
        14,
        7,
        5,
    )
    .expect("cuda row-major linear round should run");

    let expected = [
        poseidon2_hash_16(std::array::from_fn(|index| {
            Felt::from_u64([8, 9, 10, 11, 12, 0, 0, 0, 0, 0, 0, 0, 101, 102, 103, 104][index])
        }))
        .map(Felt::to_u64),
        poseidon2_hash_16(std::array::from_fn(|index| {
            Felt::from_u64([28, 29, 30, 31, 32, 0, 0, 0, 0, 0, 0, 0, 201, 202, 203, 204][index])
        }))
        .map(Felt::to_u64),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();
    let actual = output_states
        .to_u64_words()
        .expect("device words should copy back to host");

    assert_eq!(actual, expected);
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_hashes_poseidon2_width_16_states_from_device_memory() {
    let input = (0_u64..32).collect::<Vec<_>>();
    let expected = input
        .chunks_exact(16)
        .flat_map(|chunk| {
            let state = std::array::from_fn(|index| Felt::from_u64(chunk[index]));
            poseidon2_hash_16(state).map(Felt::to_u64)
        })
        .collect::<Vec<_>>();

    let input_buffer =
        CudaDeviceBuffer::from_u64_words(&input).expect("input device buffer should allocate");
    let mut output_buffer =
        CudaDeviceBuffer::new(input.len() * 8).expect("output device buffer should allocate");
    cuda_poseidon2_width16_device(&input_buffer, &mut output_buffer)
        .expect("cuda device hash should run");
    let actual = output_buffer
        .to_u64_words()
        .expect("device words should copy back to host");

    assert_eq!(actual, expected);
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_hashes_poseidon2_width_16_parent_states_from_device_memory() {
    let mut input = Vec::with_capacity(5 * 16);
    for child in 0..5_u64 {
        input.extend((child * 4 + 1)..=(child * 4 + 4));
        input.extend((0..12_u64).map(|tail| 1_000 + child * 100 + tail));
    }
    let expected_inputs = [
        [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16],
        [17, 18, 19, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    ];
    let expected = expected_inputs
        .into_iter()
        .flat_map(|state| {
            poseidon2_hash_16(state.map(Felt::from_u64))
                .map(Felt::to_u64)
                .into_iter()
        })
        .collect::<Vec<_>>();

    let input_buffer =
        CudaDeviceBuffer::from_u64_words(&input).expect("input device buffer should allocate");
    let mut output_buffer =
        CudaDeviceBuffer::new(expected.len() * 8).expect("output device buffer should allocate");
    cuda_poseidon2_width16_merkle_parent_device(&input_buffer, &mut output_buffer)
        .expect("cuda device parent hash should run");
    let actual = output_buffer
        .to_u64_words()
        .expect("device words should copy back to host");

    assert_eq!(actual, expected);
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_poseidon2_width16_merkle_root_device_matches_cpu_reference() {
    let mut input = Vec::with_capacity(5 * 16);
    for child in 0..5_u64 {
        input.extend((child * 4 + 1)..=(child * 4 + 4));
        input.extend((0..12_u64).map(|tail| 1_000 + child * 100 + tail));
    }
    let expected = cpu_merkle_root_width16(&input);
    let input_buffer =
        CudaDeviceBuffer::from_u64_words(&input).expect("input device buffer should allocate");

    let actual = cuda_poseidon2_width16_merkle_root_device(&input_buffer)
        .expect("cuda device root hash should run");

    assert_eq!(actual, expected);
}

#[cfg(feature = "cuda")]
fn cpu_merkle_root_width8(states: &[u64]) -> [u64; 4] {
    assert!(states.len().is_multiple_of(8));
    let mut level = states
        .chunks_exact(8)
        .map(|chunk| [chunk[0], chunk[1], chunk[2], chunk[3]])
        .collect::<Vec<_>>();

    while level.len() > 1 {
        level = level
            .chunks(2)
            .map(|chunk| {
                let mut state = [0_u64; 8];
                for (slot, digest) in chunk.iter().enumerate() {
                    state[slot * 4..slot * 4 + 4].copy_from_slice(digest);
                }
                let hashed = poseidon2_hash_8(state.map(Felt::from_u64)).map(Felt::to_u64);
                [hashed[0], hashed[1], hashed[2], hashed[3]]
            })
            .collect();
    }

    level[0]
}

#[cfg(feature = "cuda")]
fn cpu_merkle_root_width16(states: &[u64]) -> [u64; 4] {
    assert!(states.len().is_multiple_of(16));
    let mut level = states
        .chunks_exact(16)
        .map(|chunk| [chunk[0], chunk[1], chunk[2], chunk[3]])
        .collect::<Vec<_>>();

    while level.len() > 1 {
        level = level
            .chunks(4)
            .map(|chunk| {
                let mut state = [0_u64; 16];
                for (slot, digest) in chunk.iter().enumerate() {
                    state[slot * 4..slot * 4 + 4].copy_from_slice(digest);
                }
                let hashed = poseidon2_hash_16(state.map(Felt::from_u64)).map(Felt::to_u64);
                [hashed[0], hashed[1], hashed[2], hashed[3]]
            })
            .collect();
    }

    level[0]
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_hashes_keccak256_fixed_messages() {
    let messages = [
        vec![b'a'; 200],
        (0_u8..200).collect::<Vec<_>>(),
        (0..200)
            .map(|index| ((index * 7 + 3) % 251) as u8)
            .collect::<Vec<_>>(),
    ];
    let input = messages
        .iter()
        .flat_map(|message| message.iter().copied())
        .collect::<Vec<_>>();
    let expected = messages
        .iter()
        .map(|message| keccak256(message))
        .collect::<Vec<_>>();

    let actual = cuda_keccak256_fixed(&input, 200).expect("cuda keccak should run");

    assert_eq!(actual, expected);
}
