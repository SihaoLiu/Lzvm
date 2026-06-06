int run_poseidon2_width8_merkle_root_on_device(
    const uint64_t* device_values,
    uint64_t* device_out,
    size_t child_state_count) {
    if (child_state_count == 0) {
        return 0;
    }
    if (device_values == nullptr || device_out == nullptr) {
        return -1;
    }
    if (child_state_count == 1) {
        return cudaMemcpy(
            device_out,
            device_values,
            kPoseidon2Width8 * sizeof(uint64_t),
            cudaMemcpyDeviceToDevice);
    }

    const size_t first_parent_state_count = (child_state_count + 1) / 2;
    const size_t second_parent_state_count =
        first_parent_state_count > 1 ? (first_parent_state_count + 1) / 2 : 0;
    DeviceBuffer<uint64_t> scratch_a;
    DeviceBuffer<uint64_t> scratch_b;
    LZVM_CUDA_RETURN_ON_ERROR(scratch_a.reset(first_parent_state_count * kPoseidon2Width8));
    LZVM_CUDA_RETURN_ON_ERROR(scratch_b.reset(second_parent_state_count * kPoseidon2Width8));

    const uint64_t* current = device_values;
    uint64_t* next = scratch_a.data();
    size_t state_count = child_state_count;
    while (state_count > 1) {
        const size_t parent_state_count = (state_count + 1) / 2;
        const size_t blocks = (parent_state_count + kThreads - 1) / kThreads;
        poseidon2_width8_merkle_parent_kernel<<<blocks, kThreads>>>(
            current, next, state_count);
        LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_check_launch());

        current = next;
        state_count = parent_state_count;
        next = next == scratch_a.data() ? scratch_b.data() : scratch_a.data();
    }

    return cudaMemcpy(
        device_out,
        current,
        kPoseidon2Width8 * sizeof(uint64_t),
        cudaMemcpyDeviceToDevice);
}

int run_poseidon2_width16_merkle_root_on_device(
    const uint64_t* device_values,
    uint64_t* device_out,
    size_t child_state_count) {
    if (child_state_count == 0) {
        return 0;
    }
    if (device_values == nullptr || device_out == nullptr) {
        return -1;
    }
    if (child_state_count == 1) {
        return cudaMemcpy(
            device_out,
            device_values,
            kPoseidon2Width16 * sizeof(uint64_t),
            cudaMemcpyDeviceToDevice);
    }

    const size_t first_parent_state_count = (child_state_count + 3) / 4;
    const size_t second_parent_state_count =
        first_parent_state_count > 1 ? (first_parent_state_count + 3) / 4 : 0;
    DeviceBuffer<uint64_t> scratch_a;
    DeviceBuffer<uint64_t> scratch_b;
    LZVM_CUDA_RETURN_ON_ERROR(scratch_a.reset(first_parent_state_count * kPoseidon2Width16));
    LZVM_CUDA_RETURN_ON_ERROR(scratch_b.reset(second_parent_state_count * kPoseidon2Width16));

    const uint64_t* current = device_values;
    uint64_t* next = scratch_a.data();
    size_t state_count = child_state_count;
    while (state_count > 1) {
        const size_t parent_state_count = (state_count + 3) / 4;
        const size_t blocks = (parent_state_count + kThreads - 1) / kThreads;
        poseidon2_width16_merkle_parent_kernel<<<blocks, kThreads>>>(
            current, next, state_count);
        LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_check_launch());

        current = next;
        state_count = parent_state_count;
        next = next == scratch_a.data() ? scratch_b.data() : scratch_a.data();
    }

    return cudaMemcpy(
        device_out,
        current,
        kPoseidon2Width16 * sizeof(uint64_t),
        cudaMemcpyDeviceToDevice);
}
