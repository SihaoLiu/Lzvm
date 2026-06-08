#[cfg(feature = "cuda")]
use lzvm_accel::{cuda_goldilocks_add, cuda_goldilocks_mul};
#[cfg(feature = "cuda")]
use lzvm_accel::{
    cuda_goldilocks_begin_validate_canonical_words_device, cuda_goldilocks_butterfly,
    cuda_goldilocks_coset_extend, cuda_goldilocks_coset_extend_device,
    cuda_goldilocks_coset_extend_row_major_columns,
    cuda_goldilocks_coset_extend_row_major_columns_device,
    cuda_goldilocks_coset_extend_row_major_columns_output_bytes,
    cuda_goldilocks_coset_extend_row_major_columns_row_device,
    cuda_goldilocks_coset_extend_row_major_columns_rows_device,
    cuda_goldilocks_coset_extend_row_major_columns_selected_rows_device,
    cuda_goldilocks_coset_extend_row_major_columns_shifted_row_device,
    cuda_goldilocks_coset_extend_row_major_columns_strided_device,
    cuda_goldilocks_coset_extend_row_major_columns_strided_row_device,
    cuda_goldilocks_coset_extend_row_major_columns_strided_rows_device,
    cuda_goldilocks_coset_extend_row_major_columns_strided_selected_rows_device,
    cuda_goldilocks_coset_extend_row_major_columns_strided_shifted_row_device,
    cuda_goldilocks_intt, cuda_goldilocks_ntt, cuda_keccak256_fixed, cuda_poseidon2_width16,
    cuda_poseidon2_width16_device, cuda_poseidon2_width16_linear_round_device,
    cuda_poseidon2_width16_linear_round_row_major_device,
    cuda_poseidon2_width16_linear_round_row_major_digest_device,
    cuda_poseidon2_width16_merkle_digest_opening_path_device,
    cuda_poseidon2_width16_merkle_digest_opening_prefix_batch_device,
    cuda_poseidon2_width16_merkle_digest_opening_prefix_device,
    cuda_poseidon2_width16_merkle_digest_parent_device,
    cuda_poseidon2_width16_merkle_digest_root_device,
    cuda_poseidon2_width16_merkle_digest_selected_parent_device,
    cuda_poseidon2_width16_merkle_opening_path_device, cuda_poseidon2_width16_merkle_parent_device,
    cuda_poseidon2_width16_merkle_root_device, cuda_poseidon2_width4, cuda_poseidon2_width4_device,
    cuda_poseidon2_width4_find_nonce, cuda_poseidon2_width8, cuda_poseidon2_width8_device,
    cuda_poseidon2_width8_linear_round_device, cuda_poseidon2_width8_linear_round_row_major_device,
    cuda_poseidon2_width8_linear_round_row_major_digest_device,
    cuda_poseidon2_width8_merkle_digest_opening_path_device,
    cuda_poseidon2_width8_merkle_digest_opening_prefix_batch_device,
    cuda_poseidon2_width8_merkle_digest_opening_prefix_device,
    cuda_poseidon2_width8_merkle_digest_parent_device,
    cuda_poseidon2_width8_merkle_digest_root_device,
    cuda_poseidon2_width8_merkle_digest_selected_parent_device,
    cuda_poseidon2_width8_merkle_opening_path_device, cuda_poseidon2_width8_merkle_parent_device,
    cuda_poseidon2_width8_merkle_root_device, cuda_setup_init, AccelError, CudaDeviceBuffer,
    CudaRowMajorColumnView,
};
#[cfg(feature = "cuda")]
use lzvm_crypto::keccak256;
#[cfg(feature = "cuda")]
use lzvm_field::{
    coset_extend_evaluations, intt_in_place, ntt_in_place, poseidon2_hash_16, poseidon2_hash_4,
    poseidon2_hash_8, Felt,
};

#[test]
fn native_host_header_declares_row_range_extension_exports() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let header = std::fs::read_to_string(manifest_dir.join("native/cuda_host.hpp"))
        .expect("native host header should be readable");
    let source =
        std::fs::read_to_string(manifest_dir.join("native/cuda_goldilocks_row_extend.cuh"))
            .expect("row extension source should be readable");

    for symbol in [
        "lzvm_cuda_goldilocks_coset_extend_row_major_columns_rows_device",
        "lzvm_cuda_goldilocks_coset_extend_row_major_columns_selected_rows_device",
        "lzvm_cuda_goldilocks_coset_extend_row_major_columns_strided_rows_device",
        "lzvm_cuda_goldilocks_coset_extend_row_major_columns_strided_selected_rows_device",
    ] {
        assert!(
            source.contains(symbol),
            "row extension source should export {symbol}"
        );
        assert!(
            header.contains(symbol),
            "native host header should declare {symbol}"
        );
    }
}

#[cfg(feature = "cuda")]
const MODULUS: u64 = 0xffff_ffff_0000_0001;

#[test]
#[cfg(feature = "cuda")]
fn cuda_pending_canonical_check_reports_valid_and_invalid_words() {
    let valid = CudaDeviceBuffer::from_u64_words(&[0, 1, MODULUS - 1, 12345])
        .expect("valid field words should upload");
    let valid_check = cuda_goldilocks_begin_validate_canonical_words_device(&valid, 4)
        .expect("pending canonical check should launch");
    assert!(
        valid_check
            .is_canonical()
            .expect("valid canonical check should finish"),
        "canonical field words should be accepted"
    );

    let invalid = CudaDeviceBuffer::from_u64_words(&[0, MODULUS, MODULUS + 1])
        .expect("invalid field words should upload");
    let invalid_check = cuda_goldilocks_begin_validate_canonical_words_device(&invalid, 3)
        .expect("pending canonical check should launch");
    assert!(
        !invalid_check
            .is_canonical()
            .expect("invalid canonical check should finish"),
        "non-canonical field words should be rejected"
    );
}

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

    assert_same_words(&actual, &expected);
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_expands_zisk_main_trace_descriptors() {
    const WORDS_PER_DESCRIPTOR: usize = 11;
    const KIND_MEMORY: u64 = 1;
    const KIND_IMMEDIATE: u64 = 2;
    const KIND_REGISTER: u64 = 3;
    const KIND_INDIRECT: u64 = 4;
    const STORE_REGISTER: u64 = 2;
    const STORE_INDIRECT: u64 = 3;
    const A_KIND_SHIFT: u64 = 32;
    const B_KIND_SHIFT: u64 = 35;
    const STORE_KIND_SHIFT: u64 = 38;

    fn signed_word(value: i64) -> u64 {
        value as u64
    }

    fn packed_u32_pair(lhs: u32, rhs: u32) -> u64 {
        u64::from(lhs) | (u64::from(rhs) << 32)
    }

    fn packed_i32_pair(lhs: i32, rhs: i32) -> u64 {
        u64::from(lhs as u32) | (u64::from(rhs as u32) << 32)
    }

    #[allow(clippy::too_many_arguments)]
    fn control(
        op: u64,
        flag: bool,
        store_pc: bool,
        set_pc: bool,
        m32: bool,
        is_external_op: bool,
        is_precompiled: bool,
        ind_width: u64,
        a_kind: u64,
        b_kind: u64,
        store_kind: u64,
    ) -> u64 {
        op | (u64::from(flag) << 8)
            | (u64::from(store_pc) << 9)
            | (u64::from(set_pc) << 10)
            | (u64::from(m32) << 11)
            | (u64::from(is_external_op) << 12)
            | (u64::from(is_precompiled) << 13)
            | (ind_width << 16)
            | (a_kind << A_KIND_SHIFT)
            | (b_kind << B_KIND_SHIFT)
            | (store_kind << STORE_KIND_SHIFT)
    }

    fn signed_field(value: i64) -> u64 {
        if value >= 0 {
            value as u64
        } else {
            MODULUS - value.unsigned_abs()
        }
    }

    let row0_a = (2_u64 << 32) | 1;
    let row0_b = (4_u64 << 32) | 3;
    let row0_c = (6_u64 << 32) | 5;
    let row0_a_source = (8_u64 << 32) | 7;
    let row0_store_prev = (12_u64 << 32) | 11;
    let row1_b_source = (9_u64 << 32) | 8;
    let row1_store_prev = (14_u64 << 32) | 13;
    let descriptors = [
        [
            row0_a,
            row0_b,
            row0_c,
            row0_a_source,
            9,
            10,
            control(
                0x0a,
                true,
                false,
                true,
                false,
                true,
                false,
                4,
                KIND_IMMEDIATE,
                KIND_REGISTER,
                STORE_REGISTER,
            ),
            packed_u32_pair(0x1000, 23),
            packed_i32_pair(2, 3),
            packed_u32_pair(21, 22),
            row0_store_prev,
        ],
        [
            100,
            200,
            300,
            signed_word(-3),
            row1_b_source,
            signed_word(-2),
            control(
                0x0b,
                false,
                false,
                false,
                true,
                true,
                true,
                8,
                KIND_INDIRECT,
                KIND_MEMORY,
                STORE_INDIRECT,
            ),
            packed_u32_pair(0x2000, 33),
            packed_i32_pair(-4, 6),
            packed_u32_pair(31, 32),
            row1_store_prev,
        ],
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();
    assert_eq!(descriptors.len(), WORDS_PER_DESCRIPTOR * 2);

    let buffer = CudaDeviceBuffer::from_zisk_main_trace_descriptors(
        descriptors.as_slice(),
        WORDS_PER_DESCRIPTOR,
        2,
        4,
        39,
        0x3000,
    )
    .expect("descriptor expansion should run");
    let actual = buffer
        .to_u64_words()
        .expect("expanded trace should download");
    let mut expected = vec![0_u64; 4 * 39];
    expected[0] = 1;
    expected[1] = 2;
    expected[2] = 3;
    expected[3] = 4;
    expected[4] = 5;
    expected[5] = 6;
    expected[6] = 1;
    expected[7] = 0x1000;
    expected[8] = 1;
    expected[10] = 7;
    expected[11] = 8;
    expected[15] = 9;
    expected[18] = 4;
    expected[19] = 1;
    expected[20] = 0x0a;
    expected[24] = 10;
    expected[25] = 1;
    expected[26] = 2;
    expected[27] = 3;
    expected[29] = 9;
    expected[30] = 21;
    expected[31] = 22;
    expected[32] = 23;
    expected[33] = 11;
    expected[34] = 12;
    expected[36] = 1;
    expected[37] = 1;
    expected[39] = 100;
    expected[41] = 200;
    expected[43] = 300;
    expected[46] = 0x2000;
    expected[49] = signed_field(-3);
    expected[51] = 1;
    expected[53] = 1;
    expected[54] = 8;
    expected[55] = 9;
    expected[57] = 8;
    expected[58] = 1;
    expected[59] = 0x0b;
    expected[62] = 1;
    expected[63] = signed_field(-2);
    expected[65] = signed_field(-4);
    expected[66] = 6;
    expected[67] = 1;
    expected[68] = 8;
    expected[69] = 31;
    expected[70] = 32;
    expected[71] = 33;
    expected[72] = 13;
    expected[73] = 14;
    for row in 2..4 {
        let base = row * 39;
        expected[base + 7] = 0x3000;
        expected[base + 8] = 1;
        expected[base + 13] = 1;
        expected[base + 20] = 1;
    }

    assert_same_words(&actual, &expected);
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

    assert_same_words(&actual, &expected);
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
fn cuda_device_buffer_round_trips_large_bytes() {
    let len = (1_usize << 20) + 4096;
    let input = (0..len)
        .map(|index| {
            let shifted = index ^ (index >> 7);
            shifted.wrapping_mul(131) as u8
        })
        .collect::<Vec<_>>();
    let mut buffer = CudaDeviceBuffer::new(input.len()).expect("device buffer should allocate");

    buffer
        .copy_from(&input)
        .expect("large host bytes should copy to device");
    let output = buffer.to_vec().expect("device bytes should copy to host");

    assert_eq!(output, input);
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_memory_info_reports_available_device_memory() {
    let info = lzvm_accel::cuda_memory_info().expect("cuda memory info should be available");

    assert!(info.total_bytes > 0);
    assert!(info.free_bytes > 0);
    assert!(info.free_bytes <= info.total_bytes);
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
fn cuda_device_buffer_fills_row_major_column_ranges() {
    let mut buffer = CudaDeviceBuffer::zeroed(4 * 5 * 8).expect("device buffer should allocate");

    buffer
        .fill_row_major_column_u64(4, 5, 1, 2, 0xfeed_beef)
        .expect("device column fill should run");

    let words = buffer
        .to_u64_words()
        .expect("device words should read back");
    let mut expected = vec![0_u64; 20];
    expected[7] = 0xfeed_beef;
    expected[12] = 0xfeed_beef;
    expected[17] = 0xfeed_beef;
    assert_eq!(words, expected);
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_device_buffer_copies_u64_word_prefixes() {
    let mut buffer = CudaDeviceBuffer::zeroed(5 * 8).expect("device buffer should allocate");

    buffer
        .copy_prefix_from_u64_words(&[11, 12, 13])
        .expect("prefix words should copy");

    let words = buffer
        .to_u64_words()
        .expect("device words should read back");
    assert_eq!(words, vec![11, 12, 13, 0, 0]);
    assert!(buffer
        .copy_prefix_from_u64_words(&[1, 2, 3, 4, 5, 6])
        .is_err());
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_device_buffer_copies_prefix_and_fills_suffix_rows() {
    let prefix = vec![1, 2, 3, 4, 5, 6];
    let terminal = vec![10, 11, 12];
    let buffer =
        CudaDeviceBuffer::from_row_major_u64_prefix_and_suffix_row(&prefix, &terminal, 4, 3, 2)
            .expect("device suffix fill should build a full row-major buffer");

    let actual = buffer
        .to_u64_words()
        .expect("device buffer should read back");

    assert_eq!(actual, vec![1, 2, 3, 4, 5, 6, 10, 11, 12, 10, 11, 12]);
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_device_buffer_rebuilds_sparse_u64_words() {
    let indices = vec![1, 4, 7, 9];
    let values = vec![11, 44, 77, MODULUS - 1];

    let buffer = CudaDeviceBuffer::from_sparse_u64_words(12, &indices, &values)
        .expect("sparse words should rebuild on device");

    let actual = buffer
        .to_u64_words()
        .expect("device buffer should read back");
    assert_eq!(
        actual,
        vec![0, 11, 0, 0, 44, 0, 0, 77, 0, MODULUS - 1, 0, 0]
    );
    assert!(CudaDeviceBuffer::from_sparse_u64_words(12, &[3], &[1, 2]).is_err());
    assert!(CudaDeviceBuffer::from_sparse_u64_words(12, &[12], &[1]).is_err());
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_device_buffer_uploads_row_major_column_slice() {
    let source = (0_u64..15).collect::<Vec<_>>();
    let buffer = CudaDeviceBuffer::from_row_major_u64_slice(&source, 3, 5, 1, 3)
        .expect("row-major column slice should upload");

    let actual = buffer
        .to_u64_words()
        .expect("uploaded slice should download");

    assert_eq!(actual, vec![1, 2, 3, 6, 7, 8, 11, 12, 13]);
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_device_buffer_copies_row_major_column_slice_from_device() {
    let source = CudaDeviceBuffer::from_u64_words(&(0_u64..15).collect::<Vec<_>>())
        .expect("source device buffer should upload");
    let buffer = CudaDeviceBuffer::from_device_row_major_u64_slice(&source, 3, 5, 1, 3)
        .expect("device row-major column slice should copy");

    let actual = buffer.to_u64_words().expect("device slice should download");

    assert_eq!(actual, vec![1, 2, 3, 6, 7, 8, 11, 12, 13]);
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
fn cuda_device_buffer_expands_state_prefix_words_from_device_source() {
    let source =
        CudaDeviceBuffer::from_u64_words(&[1, 2, 3, 4, 5, 6]).expect("source rows should upload");

    let buffer = CudaDeviceBuffer::from_device_state_prefix_u64_words(&source, 2, 8, 3)
        .expect("device prefix rows should expand into padded states");

    assert_eq!(
        buffer
            .to_u64_words()
            .expect("expanded state words should copy back"),
        vec![1, 2, 3, 0, 0, 0, 0, 0, 4, 5, 6, 0, 0, 0, 0, 0]
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
fn cuda_extends_row_major_columns_from_strided_device_memory() {
    let trace = vec![
        100, 5, 9, 2, 200, 101, 1, 9, 4, 201, 102, 3, 7, 6, 202, 103, 8, 11, 10, 203,
    ];
    let compact = vec![5, 9, 2, 1, 9, 4, 3, 7, 6, 8, 11, 10];
    let source_bits = 2;
    let target_bits = 4;
    let column_count = 3;
    let expected = cuda_goldilocks_coset_extend_row_major_columns(
        &compact,
        column_count,
        source_bits,
        target_bits,
    )
    .expect("compact row-major extension should run");
    let trace_buffer =
        CudaDeviceBuffer::from_u64_words(&trace).expect("trace device buffer should allocate");
    let mut output_buffer =
        CudaDeviceBuffer::new(expected.len() * 8).expect("output device buffer should allocate");

    cuda_goldilocks_coset_extend_row_major_columns_strided_device(
        &trace_buffer,
        &mut output_buffer,
        CudaRowMajorColumnView {
            source_rows: 4,
            source_row_stride: 5,
            column_offset: 1,
            column_count,
        },
        source_bits,
        target_bits,
    )
    .expect("strided device row-major extension should run");
    let actual = output_buffer
        .to_u64_words()
        .expect("device output should copy back");

    assert_eq!(actual, expected);
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_extends_many_row_major_columns_across_large_stages() {
    let source_bits = 10;
    let target_bits = 12;
    let column_count = 7;
    let source_rows = 1_usize << source_bits;
    let target_rows = 1_usize << target_bits;
    let input = (0..source_rows * column_count)
        .map(|index| {
            let mixed = index as u64 * 17 + (index / column_count) as u64 * 29 + 5;
            mixed % 1_000_003
        })
        .collect::<Vec<_>>();
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

    assert_same_words(&actual, &expected);
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_extends_one_row_major_coset_row_from_device_memory() {
    let input = vec![5, 9, 2, 1, 9, 4, 3, 7, 6, 8, 11, 10];
    let source_bits = 2;
    let target_bits = 4;
    let column_count = 3;
    let target_rows = 1_usize << target_bits;
    let extended = cuda_goldilocks_coset_extend_row_major_columns(
        &input,
        column_count,
        source_bits,
        target_bits,
    )
    .expect("host row-major extension should run");
    let input_buffer =
        CudaDeviceBuffer::from_u64_words(&input).expect("input device buffer should allocate");

    for target_row in 0..target_rows {
        let mut row_buffer =
            CudaDeviceBuffer::new(column_count * 8).expect("row output should allocate");

        cuda_goldilocks_coset_extend_row_major_columns_row_device(
            &input_buffer,
            &mut row_buffer,
            column_count,
            source_bits,
            target_bits,
            target_row,
        )
        .expect("device row-major row extension should run");
        let actual = row_buffer
            .to_u64_words()
            .expect("device row output should copy back");
        let expected =
            extended[target_row * column_count..(target_row + 1) * column_count].to_vec();

        assert_eq!(actual, expected, "target row {target_row} should match");
    }
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_extends_shifted_row_major_coset_row_from_device_memory() {
    let input = vec![5, 9, 2, 1, 9, 4, 3, 7, 6, 8, 11, 10];
    let source_bits = 2;
    let target_bits = 4;
    let column_count = 3;
    let target_row = 11;
    let extended = cuda_goldilocks_coset_extend_row_major_columns(
        &input,
        column_count,
        source_bits,
        target_bits,
    )
    .expect("host row-major extension should run");
    let input_buffer =
        CudaDeviceBuffer::from_u64_words(&input).expect("input device buffer should allocate");
    let mut output_buffer =
        CudaDeviceBuffer::new(column_count * 8).expect("row output should allocate");

    cuda_goldilocks_coset_extend_row_major_columns_shifted_row_device(
        &input_buffer,
        &mut output_buffer,
        column_count,
        source_bits,
        target_bits,
        target_row,
    )
    .expect("device row-major shifted row extension should run");
    let actual = output_buffer
        .to_u64_words()
        .expect("device row output should copy back");
    let expected = extended[target_row * column_count..(target_row + 1) * column_count].to_vec();

    assert_eq!(actual, expected);
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_extends_row_major_coset_row_range_from_device_memory() {
    let input = vec![5, 9, 2, 1, 9, 4, 3, 7, 6, 8, 11, 10];
    let source_bits = 2;
    let target_bits = 4;
    let column_count = 3;
    let target_row_start = 3;
    let target_row_count = 5;
    let extended = cuda_goldilocks_coset_extend_row_major_columns(
        &input,
        column_count,
        source_bits,
        target_bits,
    )
    .expect("host row-major extension should run");
    let input_buffer =
        CudaDeviceBuffer::from_u64_words(&input).expect("input device buffer should allocate");
    let mut output_buffer = CudaDeviceBuffer::new(target_row_count * column_count * 8)
        .expect("row output should allocate");

    cuda_goldilocks_coset_extend_row_major_columns_rows_device(
        &input_buffer,
        &mut output_buffer,
        column_count,
        source_bits,
        target_bits,
        target_row_start,
        target_row_count,
    )
    .expect("device row-major row range extension should run");
    let actual = output_buffer
        .to_u64_words()
        .expect("device row output should copy back");
    let expected = extended
        [target_row_start * column_count..(target_row_start + target_row_count) * column_count]
        .to_vec();

    assert_eq!(actual, expected);
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_extends_selected_row_major_coset_rows_from_device_memory() {
    let input = vec![5, 9, 2, 1, 9, 4, 3, 7, 6, 8, 11, 10];
    let source_bits = 2;
    let target_bits = 4;
    let column_count = 3;
    let target_rows = [11_usize, 2, 14, 2, 7];
    let extended = cuda_goldilocks_coset_extend_row_major_columns(
        &input,
        column_count,
        source_bits,
        target_bits,
    )
    .expect("host row-major extension should run");
    let input_buffer =
        CudaDeviceBuffer::from_u64_words(&input).expect("input device buffer should allocate");
    let mut output_buffer = CudaDeviceBuffer::new(target_rows.len() * column_count * 8)
        .expect("row output should allocate");

    cuda_goldilocks_coset_extend_row_major_columns_selected_rows_device(
        &input_buffer,
        &mut output_buffer,
        column_count,
        source_bits,
        target_bits,
        &target_rows,
    )
    .expect("device row-major selected-row extension should run");
    let actual = output_buffer
        .to_u64_words()
        .expect("device row output should copy back");
    let expected = target_rows
        .iter()
        .flat_map(|target_row| {
            extended[*target_row * column_count..(*target_row + 1) * column_count].to_vec()
        })
        .collect::<Vec<_>>();

    assert_eq!(actual, expected);
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_extends_one_strided_row_major_coset_row_from_device_memory() {
    let source_bits = 2;
    let target_bits = 4;
    let source_rows = 1_usize << source_bits;
    let target_rows = 1_usize << target_bits;
    let source_row_stride = 5;
    let column_offset = 1;
    let column_count = 3;
    let mut strided = Vec::with_capacity(source_rows * source_row_stride);
    let mut compact = Vec::with_capacity(source_rows * column_count);
    for row in 0..source_rows {
        strided.push(90 + row as u64);
        for column in 0..column_count {
            let value = (row * 10 + column + 1) as u64;
            strided.push(value);
            compact.push(value);
        }
        strided.push(190 + row as u64);
    }
    let expected = cuda_goldilocks_coset_extend_row_major_columns(
        &compact,
        column_count,
        source_bits,
        target_bits,
    )
    .expect("host row-major extension should run");
    let input_buffer =
        CudaDeviceBuffer::from_u64_words(&strided).expect("input device buffer should allocate");
    let view = CudaRowMajorColumnView {
        source_rows,
        source_row_stride,
        column_offset,
        column_count,
    };

    for target_row in 0..target_rows {
        let mut row_buffer =
            CudaDeviceBuffer::new(column_count * 8).expect("row output should allocate");

        cuda_goldilocks_coset_extend_row_major_columns_strided_row_device(
            &input_buffer,
            &mut row_buffer,
            view,
            source_bits,
            target_bits,
            target_row,
        )
        .expect("device strided row-major row extension should run");
        let actual = row_buffer
            .to_u64_words()
            .expect("device row output should copy back");
        let expected =
            expected[target_row * column_count..(target_row + 1) * column_count].to_vec();

        assert_eq!(actual, expected, "target row {target_row} should match");
    }
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_extends_shifted_strided_row_major_coset_row_from_device_memory() {
    let source_bits = 2;
    let target_bits = 4;
    let source_rows = 1_usize << source_bits;
    let source_row_stride = 6;
    let column_offset = 2;
    let column_count = 3;
    let target_row = 10;
    let mut strided = Vec::with_capacity(source_rows * source_row_stride);
    let mut compact = Vec::with_capacity(source_rows * column_count);
    for row in 0..source_rows {
        strided.push(80 + row as u64);
        strided.push(90 + row as u64);
        for column in 0..column_count {
            let value = (row * 10 + column + 1) as u64;
            strided.push(value);
            compact.push(value);
        }
        strided.push(190 + row as u64);
    }
    let expected = cuda_goldilocks_coset_extend_row_major_columns(
        &compact,
        column_count,
        source_bits,
        target_bits,
    )
    .expect("host row-major extension should run");
    let input_buffer =
        CudaDeviceBuffer::from_u64_words(&strided).expect("input device buffer should allocate");
    let view = CudaRowMajorColumnView {
        source_rows,
        source_row_stride,
        column_offset,
        column_count,
    };
    let mut row_buffer =
        CudaDeviceBuffer::new(column_count * 8).expect("row output should allocate");

    cuda_goldilocks_coset_extend_row_major_columns_strided_shifted_row_device(
        &input_buffer,
        &mut row_buffer,
        view,
        source_bits,
        target_bits,
        target_row,
    )
    .expect("device strided row-major shifted row extension should run");
    let actual = row_buffer
        .to_u64_words()
        .expect("device row output should copy back");
    let expected = expected[target_row * column_count..(target_row + 1) * column_count].to_vec();

    assert_eq!(actual, expected);
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_extends_strided_row_major_coset_row_range_from_device_memory() {
    let source_bits = 2;
    let target_bits = 4;
    let source_rows = 1_usize << source_bits;
    let source_row_stride = 5;
    let column_offset = 1;
    let column_count = 3;
    let target_row_start = 4;
    let target_row_count = 6;
    let mut strided = Vec::with_capacity(source_rows * source_row_stride);
    let mut compact = Vec::with_capacity(source_rows * column_count);
    for row in 0..source_rows {
        strided.push(90 + row as u64);
        for column in 0..column_count {
            let value = (row * 10 + column + 1) as u64;
            strided.push(value);
            compact.push(value);
        }
        strided.push(190 + row as u64);
    }
    let expected = cuda_goldilocks_coset_extend_row_major_columns(
        &compact,
        column_count,
        source_bits,
        target_bits,
    )
    .expect("host row-major extension should run");
    let input_buffer =
        CudaDeviceBuffer::from_u64_words(&strided).expect("input device buffer should allocate");
    let view = CudaRowMajorColumnView {
        source_rows,
        source_row_stride,
        column_offset,
        column_count,
    };
    let mut output_buffer = CudaDeviceBuffer::new(target_row_count * column_count * 8)
        .expect("row output should allocate");

    cuda_goldilocks_coset_extend_row_major_columns_strided_rows_device(
        &input_buffer,
        &mut output_buffer,
        view,
        source_bits,
        target_bits,
        target_row_start,
        target_row_count,
    )
    .expect("device strided row-major row range extension should run");
    let actual = output_buffer
        .to_u64_words()
        .expect("device row output should copy back");
    let expected = expected
        [target_row_start * column_count..(target_row_start + target_row_count) * column_count]
        .to_vec();

    assert_eq!(actual, expected);
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_extends_selected_strided_row_major_coset_rows_from_device_memory() {
    let source_bits = 2;
    let target_bits = 4;
    let source_rows = 1_usize << source_bits;
    let source_row_stride = 6;
    let column_offset = 2;
    let column_count = 3;
    let target_rows = [10_usize, 1, 15, 4, 10];
    let mut strided = Vec::with_capacity(source_rows * source_row_stride);
    let mut compact = Vec::with_capacity(source_rows * column_count);
    for row in 0..source_rows {
        strided.push(80 + row as u64);
        strided.push(90 + row as u64);
        for column in 0..column_count {
            let value = (row * 10 + column + 1) as u64;
            strided.push(value);
            compact.push(value);
        }
        strided.push(190 + row as u64);
    }
    let expected = cuda_goldilocks_coset_extend_row_major_columns(
        &compact,
        column_count,
        source_bits,
        target_bits,
    )
    .expect("host row-major extension should run");
    let input_buffer =
        CudaDeviceBuffer::from_u64_words(&strided).expect("input device buffer should allocate");
    let view = CudaRowMajorColumnView {
        source_rows,
        source_row_stride,
        column_offset,
        column_count,
    };
    let mut output_buffer = CudaDeviceBuffer::new(target_rows.len() * column_count * 8)
        .expect("row output should allocate");

    cuda_goldilocks_coset_extend_row_major_columns_strided_selected_rows_device(
        &input_buffer,
        &mut output_buffer,
        view,
        source_bits,
        target_bits,
        &target_rows,
    )
    .expect("device strided row-major selected-row extension should run");
    let actual = output_buffer
        .to_u64_words()
        .expect("device row output should copy back");
    let expected = target_rows
        .iter()
        .flat_map(|target_row| {
            expected[*target_row * column_count..(*target_row + 1) * column_count].to_vec()
        })
        .collect::<Vec<_>>();

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
fn cuda_linear_round_poseidon2_width_8_digest_row_major_matches_full_state_prefix() {
    let current_states = CudaDeviceBuffer::from_u64_words(&[
        101, 102, 103, 104, 0, 0, 0, 0, 201, 202, 203, 204, 0, 0, 0, 0,
    ])
    .expect("current state buffer should allocate");
    let row_values = CudaDeviceBuffer::from_u64_words(&[1, 2, 3, 4, 5, 6, 11, 12, 13, 14, 15, 16])
        .expect("row-major value buffer should allocate");
    let mut full_states =
        CudaDeviceBuffer::new(16 * 8).expect("full output state buffer should allocate");
    let mut digest_states =
        CudaDeviceBuffer::zeroed(16 * 8).expect("digest output state buffer should allocate");

    cuda_poseidon2_width8_linear_round_row_major_device(
        &current_states,
        &row_values,
        &mut full_states,
        6,
        2,
        3,
    )
    .expect("full row-major linear round should run");
    cuda_poseidon2_width8_linear_round_row_major_digest_device(
        &current_states,
        &row_values,
        &mut digest_states,
        6,
        2,
        3,
    )
    .expect("digest row-major linear round should run");

    let full = full_states
        .to_u64_words()
        .expect("full output should copy back");
    let digest = digest_states
        .to_u64_words()
        .expect("digest output should copy back");
    for row in 0..2 {
        let base = row * 8;
        assert_eq!(&digest[base..base + 4], &full[base..base + 4]);
        assert_eq!(&digest[base + 4..base + 8], &[0, 0, 0, 0]);
    }
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
fn cuda_poseidon2_width8_merkle_opening_path_device_matches_cpu_reference() {
    let input = vec![
        1, 2, 3, 4, 101, 102, 103, 104, 5, 6, 7, 8, 201, 202, 203, 204, 9, 10, 11, 12, 301, 302,
        303, 304, 13, 14, 15, 16, 401, 402, 403, 404, 17, 18, 19, 20, 501, 502, 503, 504,
    ];
    let input_buffer =
        CudaDeviceBuffer::from_u64_words(&input).expect("input device buffer should allocate");
    let expected = cpu_merkle_opening_width8(&input, 3);

    let actual = cuda_poseidon2_width8_merkle_opening_path_device(&input_buffer, 3)
        .expect("cuda device opening path should run");

    assert_eq!(actual.root, expected.root);
    assert_eq!(actual.siblings, expected.siblings);
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_poseidon2_width8_merkle_digest_root_device_matches_padded_layout() {
    let digests = (0..5_u64)
        .flat_map(|child| (1..=4).map(move |word| child * 10 + word))
        .collect::<Vec<_>>();
    let padded = padded_digest_states(&digests, 8);
    let input_buffer =
        CudaDeviceBuffer::from_u64_words(&digests).expect("digest buffer should allocate");
    let expected = cpu_merkle_root_width8(&padded);

    let actual = cuda_poseidon2_width8_merkle_digest_root_device(&input_buffer)
        .expect("cuda compact digest root should run");

    assert_eq!(actual, expected);

    let single_buffer =
        CudaDeviceBuffer::from_u64_words(&digests[..4]).expect("single digest should allocate");
    let single_root = cuda_poseidon2_width8_merkle_digest_root_device(&single_buffer)
        .expect("single compact digest root should run");
    assert_eq!(
        single_root,
        [digests[0], digests[1], digests[2], digests[3]]
    );
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_poseidon2_width8_merkle_digest_parent_device_matches_padded_layout() {
    let digests = (0..5_u64)
        .flat_map(|child| (1..=4).map(move |word| child * 10 + word))
        .collect::<Vec<_>>();
    let padded = padded_digest_states(&digests, 8);
    let expected = flatten_digests(&cpu_merkle_first_parent_level_width8(&padded));
    let input_buffer =
        CudaDeviceBuffer::from_u64_words(&digests).expect("digest buffer should allocate");
    let mut output_buffer =
        CudaDeviceBuffer::new(expected.len() * 8).expect("parent buffer should allocate");

    cuda_poseidon2_width8_merkle_digest_parent_device(&input_buffer, &mut output_buffer)
        .expect("cuda compact digest parent level should run");
    let actual = output_buffer
        .to_u64_words()
        .expect("parent words should copy back");

    assert_same_words(&actual, &expected);
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_poseidon2_width8_merkle_digest_selected_parent_device_matches_padded_layout() {
    let digests = (0..5_u64)
        .flat_map(|child| (1..=4).map(move |word| child * 10 + word))
        .collect::<Vec<_>>();
    let padded = padded_digest_states(&digests, 8);
    let expected = cpu_merkle_first_parent_level_width8(&padded)[2];
    let input_buffer =
        CudaDeviceBuffer::from_u64_words(&digests).expect("digest buffer should allocate");

    let actual = cuda_poseidon2_width8_merkle_digest_selected_parent_device(&input_buffer, 2)
        .expect("cuda compact digest selected parent should run");

    assert_eq!(actual, expected);
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_poseidon2_width8_merkle_digest_opening_path_device_matches_padded_layout() {
    let digests = (0..5_u64)
        .flat_map(|child| (1..=4).map(move |word| child * 10 + word))
        .collect::<Vec<_>>();
    let padded = padded_digest_states(&digests, 8);
    let input_buffer =
        CudaDeviceBuffer::from_u64_words(&digests).expect("digest buffer should allocate");
    let expected = cpu_merkle_opening_width8(&padded, 4);

    let actual = cuda_poseidon2_width8_merkle_digest_opening_path_device(&input_buffer, 4)
        .expect("cuda compact digest opening should run");

    assert_eq!(actual.root, expected.root);
    assert_eq!(actual.siblings, expected.siblings);
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_poseidon2_width8_merkle_digest_opening_prefix_device_matches_path_prefix() {
    let digests = (0..9_u64)
        .flat_map(|child| (1..=4).map(move |word| child * 10 + word))
        .collect::<Vec<_>>();
    let padded = padded_digest_states(&digests, 8);
    let input_buffer =
        CudaDeviceBuffer::from_u64_words(&digests).expect("digest buffer should allocate");
    let expected_path = cpu_merkle_opening_width8(&padded, 8);
    let folded_level_count = 2;
    let expected_prefix_words = folded_level_count * 4;

    let actual = cuda_poseidon2_width8_merkle_digest_opening_prefix_device(
        &input_buffer,
        8,
        folded_level_count,
    )
    .expect("cuda compact digest opening prefix should run");

    assert_eq!(
        actual,
        expected_path.siblings[..expected_prefix_words].to_vec()
    );
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_poseidon2_width8_merkle_digest_opening_prefix_batch_device_matches_path_prefixes() {
    let digests = (0..17_u64)
        .flat_map(|child| (1..=4).map(move |word| child * 10 + word))
        .collect::<Vec<_>>();
    let padded = padded_digest_states(&digests, 8);
    let input_buffer =
        CudaDeviceBuffer::from_u64_words(&digests).expect("digest buffer should allocate");
    let query_indices = [0_usize, 8, 16];
    let folded_level_count = 3;
    let row_prefix_words = folded_level_count * 4;
    let expected = query_indices
        .iter()
        .flat_map(|query_index| {
            let expected_path = cpu_merkle_opening_width8(&padded, *query_index);
            expected_path.siblings[..row_prefix_words].to_vec()
        })
        .collect::<Vec<_>>();

    let actual = cuda_poseidon2_width8_merkle_digest_opening_prefix_batch_device(
        &input_buffer,
        &query_indices,
        folded_level_count,
    )
    .expect("cuda compact digest opening prefix batch should run");

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
fn cuda_linear_round_poseidon2_width_16_digest_row_major_matches_full_state_prefix() {
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
    let mut full_states =
        CudaDeviceBuffer::new(32 * 8).expect("full output state buffer should allocate");
    let mut digest_states =
        CudaDeviceBuffer::zeroed(32 * 8).expect("digest output state buffer should allocate");

    cuda_poseidon2_width16_linear_round_row_major_device(
        &current_states,
        &row_values,
        &mut full_states,
        14,
        7,
        5,
    )
    .expect("full row-major linear round should run");
    cuda_poseidon2_width16_linear_round_row_major_digest_device(
        &current_states,
        &row_values,
        &mut digest_states,
        14,
        7,
        5,
    )
    .expect("digest row-major linear round should run");

    let full = full_states
        .to_u64_words()
        .expect("full output should copy back");
    let digest = digest_states
        .to_u64_words()
        .expect("digest output should copy back");
    for row in 0..2 {
        let base = row * 16;
        assert_eq!(&digest[base..base + 4], &full[base..base + 4]);
        assert_eq!(&digest[base + 4..base + 16], &[0; 12]);
    }
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

#[test]
#[cfg(feature = "cuda")]
fn cuda_poseidon2_width16_merkle_opening_path_device_matches_cpu_reference() {
    let mut input = Vec::with_capacity(7 * 16);
    for child in 0..7_u64 {
        input.extend((child * 4 + 1)..=(child * 4 + 4));
        input.extend((0..12_u64).map(|tail| 1_000 + child * 100 + tail));
    }
    let input_buffer =
        CudaDeviceBuffer::from_u64_words(&input).expect("input device buffer should allocate");
    let expected = cpu_merkle_opening_width16(&input, 5);

    let actual = cuda_poseidon2_width16_merkle_opening_path_device(&input_buffer, 5)
        .expect("cuda device opening path should run");

    assert_eq!(actual.root, expected.root);
    assert_eq!(actual.siblings, expected.siblings);
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_poseidon2_width16_merkle_digest_root_device_matches_padded_layout() {
    let digests = (0..7_u64)
        .flat_map(|child| (1..=4).map(move |word| child * 10 + word))
        .collect::<Vec<_>>();
    let padded = padded_digest_states(&digests, 16);
    let input_buffer =
        CudaDeviceBuffer::from_u64_words(&digests).expect("digest buffer should allocate");
    let expected = cpu_merkle_root_width16(&padded);

    let actual = cuda_poseidon2_width16_merkle_digest_root_device(&input_buffer)
        .expect("cuda compact digest root should run");

    assert_eq!(actual, expected);

    let single_buffer =
        CudaDeviceBuffer::from_u64_words(&digests[..4]).expect("single digest should allocate");
    let single_root = cuda_poseidon2_width16_merkle_digest_root_device(&single_buffer)
        .expect("single compact digest root should run");
    assert_eq!(
        single_root,
        [digests[0], digests[1], digests[2], digests[3]]
    );
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_poseidon2_width16_merkle_digest_parent_device_matches_padded_layout() {
    let digests = (0..7_u64)
        .flat_map(|child| (1..=4).map(move |word| child * 10 + word))
        .collect::<Vec<_>>();
    let padded = padded_digest_states(&digests, 16);
    let expected = flatten_digests(&cpu_merkle_first_parent_level_width16(&padded));
    let input_buffer =
        CudaDeviceBuffer::from_u64_words(&digests).expect("digest buffer should allocate");
    let mut output_buffer =
        CudaDeviceBuffer::new(expected.len() * 8).expect("parent buffer should allocate");

    cuda_poseidon2_width16_merkle_digest_parent_device(&input_buffer, &mut output_buffer)
        .expect("cuda compact digest parent level should run");
    let actual = output_buffer
        .to_u64_words()
        .expect("parent words should copy back");

    assert_same_words(&actual, &expected);
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_poseidon2_width16_merkle_digest_selected_parent_device_matches_padded_layout() {
    let digests = (0..7_u64)
        .flat_map(|child| (1..=4).map(move |word| child * 10 + word))
        .collect::<Vec<_>>();
    let padded = padded_digest_states(&digests, 16);
    let expected = cpu_merkle_first_parent_level_width16(&padded)[1];
    let input_buffer =
        CudaDeviceBuffer::from_u64_words(&digests).expect("digest buffer should allocate");

    let actual = cuda_poseidon2_width16_merkle_digest_selected_parent_device(&input_buffer, 1)
        .expect("cuda compact digest selected parent should run");

    assert_eq!(actual, expected);
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_poseidon2_width16_merkle_digest_opening_path_device_matches_padded_layout() {
    let digests = (0..7_u64)
        .flat_map(|child| (1..=4).map(move |word| child * 10 + word))
        .collect::<Vec<_>>();
    let padded = padded_digest_states(&digests, 16);
    let input_buffer =
        CudaDeviceBuffer::from_u64_words(&digests).expect("digest buffer should allocate");
    let expected = cpu_merkle_opening_width16(&padded, 6);

    let actual = cuda_poseidon2_width16_merkle_digest_opening_path_device(&input_buffer, 6)
        .expect("cuda compact digest opening should run");

    assert_eq!(actual.root, expected.root);
    assert_eq!(actual.siblings, expected.siblings);
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_poseidon2_width16_merkle_digest_opening_prefix_device_matches_path_prefix() {
    let digests = (0..70_u64)
        .flat_map(|child| (1..=4).map(move |word| child * 10 + word))
        .collect::<Vec<_>>();
    let padded = padded_digest_states(&digests, 16);
    let input_buffer =
        CudaDeviceBuffer::from_u64_words(&digests).expect("digest buffer should allocate");
    let expected_path = cpu_merkle_opening_width16(&padded, 69);
    let folded_level_count = 3;
    let expected_prefix_words = folded_level_count * (4 - 1) * 4;

    let actual = cuda_poseidon2_width16_merkle_digest_opening_prefix_device(
        &input_buffer,
        69,
        folded_level_count,
    )
    .expect("cuda compact digest opening prefix should run");

    assert_eq!(
        actual,
        expected_path.siblings[..expected_prefix_words].to_vec()
    );
}

#[test]
#[cfg(feature = "cuda")]
fn cuda_poseidon2_width16_merkle_digest_opening_prefix_batch_device_matches_path_prefixes() {
    let digests = (0..70_u64)
        .flat_map(|child| (1..=4).map(move |word| child * 10 + word))
        .collect::<Vec<_>>();
    let padded = padded_digest_states(&digests, 16);
    let input_buffer =
        CudaDeviceBuffer::from_u64_words(&digests).expect("digest buffer should allocate");
    let query_indices = [0_usize, 21, 69];
    let folded_level_count = 3;
    let row_prefix_words = folded_level_count * (4 - 1) * 4;
    let expected = query_indices
        .iter()
        .flat_map(|query_index| {
            let expected_path = cpu_merkle_opening_width16(&padded, *query_index);
            expected_path.siblings[..row_prefix_words].to_vec()
        })
        .collect::<Vec<_>>();

    let actual = cuda_poseidon2_width16_merkle_digest_opening_prefix_batch_device(
        &input_buffer,
        &query_indices,
        folded_level_count,
    )
    .expect("cuda compact digest opening prefix batch should run");

    assert_eq!(actual, expected);
}

#[cfg(feature = "cuda")]
#[derive(Debug, Clone, PartialEq, Eq)]
struct CpuMerkleOpening {
    root: [u64; 4],
    siblings: Vec<u64>,
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
fn cpu_merkle_first_parent_level_width8(states: &[u64]) -> Vec<[u64; 4]> {
    assert!(states.len().is_multiple_of(8));
    states
        .chunks(16)
        .map(|chunk| {
            let mut state = [0_u64; 8];
            for (slot, child) in chunk.chunks(8).enumerate() {
                state[slot * 4..slot * 4 + 4].copy_from_slice(&child[..4]);
            }
            poseidon2_hash_8(state.map(Felt::from_u64)).map(Felt::to_u64)
        })
        .map(|digest| [digest[0], digest[1], digest[2], digest[3]])
        .collect()
}

#[cfg(feature = "cuda")]
fn cpu_merkle_opening_width8(states: &[u64], query: usize) -> CpuMerkleOpening {
    assert!(states.len().is_multiple_of(8));
    let level = states
        .chunks_exact(8)
        .map(|chunk| [chunk[0], chunk[1], chunk[2], chunk[3]])
        .collect::<Vec<_>>();
    cpu_merkle_opening(level, query, 2)
}

#[cfg(feature = "cuda")]
fn assert_same_words(actual: &[u64], expected: &[u64]) {
    assert_eq!(actual.len(), expected.len());
    if let Some(index) = actual
        .iter()
        .zip(expected.iter())
        .position(|(actual, expected)| actual != expected)
    {
        panic!(
            "word mismatch at index {index}: actual={}, expected={}",
            actual[index], expected[index]
        );
    }
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

#[cfg(feature = "cuda")]
fn cpu_merkle_first_parent_level_width16(states: &[u64]) -> Vec<[u64; 4]> {
    assert!(states.len().is_multiple_of(16));
    states
        .chunks(64)
        .map(|chunk| {
            let mut state = [0_u64; 16];
            for (slot, child) in chunk.chunks(16).enumerate() {
                state[slot * 4..slot * 4 + 4].copy_from_slice(&child[..4]);
            }
            poseidon2_hash_16(state.map(Felt::from_u64)).map(Felt::to_u64)
        })
        .map(|digest| [digest[0], digest[1], digest[2], digest[3]])
        .collect()
}

#[cfg(feature = "cuda")]
fn cpu_merkle_opening_width16(states: &[u64], query: usize) -> CpuMerkleOpening {
    assert!(states.len().is_multiple_of(16));
    let level = states
        .chunks_exact(16)
        .map(|chunk| [chunk[0], chunk[1], chunk[2], chunk[3]])
        .collect::<Vec<_>>();
    cpu_merkle_opening(level, query, 4)
}

#[cfg(feature = "cuda")]
fn padded_digest_states(digests: &[u64], width: usize) -> Vec<u64> {
    assert!(digests.len().is_multiple_of(4));
    let mut padded = vec![0_u64; digests.len() / 4 * width];
    for (index, digest) in digests.chunks_exact(4).enumerate() {
        padded[index * width..index * width + 4].copy_from_slice(digest);
    }
    padded
}

#[cfg(feature = "cuda")]
fn flatten_digests(digests: &[[u64; 4]]) -> Vec<u64> {
    digests
        .iter()
        .flat_map(|digest| digest.iter())
        .copied()
        .collect()
}

#[cfg(feature = "cuda")]
fn cpu_merkle_opening(
    mut level: Vec<[u64; 4]>,
    mut query: usize,
    arity: usize,
) -> CpuMerkleOpening {
    assert!(query < level.len());
    let mut siblings = Vec::new();
    while level.len() > 1 {
        let child_slot = query % arity;
        let group_start = (query / arity) * arity;
        for slot in 0..arity {
            if slot == child_slot {
                continue;
            }
            let child_index = group_start + slot;
            if child_index < level.len() {
                siblings.extend(level[child_index]);
            } else {
                siblings.extend([0_u64; 4]);
            }
        }
        level = level
            .chunks(arity)
            .map(|chunk| {
                let mut state = vec![0_u64; arity * 4];
                for (slot, digest) in chunk.iter().enumerate() {
                    state[slot * 4..slot * 4 + 4].copy_from_slice(digest);
                }
                match arity {
                    2 => {
                        let state = std::array::from_fn(|index| Felt::from_u64(state[index]));
                        poseidon2_hash_8(state).map(Felt::to_u64)[0..4]
                            .try_into()
                            .expect("digest width should match")
                    }
                    4 => {
                        let state = std::array::from_fn(|index| Felt::from_u64(state[index]));
                        poseidon2_hash_16(state).map(Felt::to_u64)[0..4]
                            .try_into()
                            .expect("digest width should match")
                    }
                    _ => unreachable!("test arity should be supported"),
                }
            })
            .collect();
        query /= arity;
    }
    CpuMerkleOpening {
        root: level[0],
        siblings,
    }
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
