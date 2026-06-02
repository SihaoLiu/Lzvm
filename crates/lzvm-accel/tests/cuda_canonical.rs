#![cfg(feature = "cuda")]

use lzvm_accel::{cuda_goldilocks_validate_canonical_words_device, CudaDeviceBuffer};

const MODULUS: u64 = 0xffff_ffff_0000_0001;

#[test]
fn cuda_canonical_checker_accepts_canonical_words() {
    let buffer =
        CudaDeviceBuffer::from_u64_words(&[0, 1, MODULUS - 1]).expect("buffer should upload");

    assert!(
        cuda_goldilocks_validate_canonical_words_device(&buffer, 3).expect("checker should run")
    );
}

#[test]
fn cuda_canonical_checker_rejects_modulus_word() {
    let buffer = CudaDeviceBuffer::from_u64_words(&[0, MODULUS]).expect("buffer should upload");

    assert!(
        !cuda_goldilocks_validate_canonical_words_device(&buffer, 2).expect("checker should run")
    );
}
