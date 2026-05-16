#[cfg(feature = "cuda")]
#[test]
fn prepares_gpu_setup_for_extended_domain() {
    lzvm_prover::prepare_gpu_setup(4).expect("GPU setup should prepare domain roots");
}
