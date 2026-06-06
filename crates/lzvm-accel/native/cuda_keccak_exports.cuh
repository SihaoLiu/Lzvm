extern "C" int lzvm_cuda_keccak256_fixed(
    const uint8_t* input,
    size_t message_len,
    uint8_t* out,
    size_t message_count) {
    if (message_count == 0) {
        return 0;
    }
    if (message_len == 0) {
        return -2;
    }
    if (input == nullptr || out == nullptr) {
        return -1;
    }

    const size_t input_bytes = message_count * message_len;
    const size_t output_bytes = message_count * kKeccakOutputBytes;
    DeviceBuffer<uint8_t> device_input;
    DeviceBuffer<uint8_t> device_out;

    LZVM_CUDA_RETURN_ON_ERROR(device_input.reset(input_bytes));
    LZVM_CUDA_RETURN_ON_ERROR(device_out.reset(output_bytes));

    LZVM_CUDA_RETURN_ON_ERROR(device_input.copy_from_bytes(input, input_bytes));

    const size_t blocks = (message_count + kThreads - 1) / kThreads;
    keccak256_fixed_kernel<<<blocks, kThreads>>>(
        device_input.data(), device_out.data(), message_len, message_count);
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_check_launch());
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_synchronize());
    LZVM_CUDA_RETURN_ON_ERROR(device_out.copy_to_bytes(out, output_bytes));
    return 0;
}
