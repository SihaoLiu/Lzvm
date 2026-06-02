#pragma once

constexpr size_t kRegularMaxTmp1 = 64;

__device__ uint64_t regular_apply_base_op(uint16_t kind, uint64_t lhs, uint64_t rhs) {
    if (kind == 0) {
        return add_mod(lhs, rhs);
    }
    if (kind == 1) {
        return sub_mod(lhs, rhs);
    }
    if (kind == 2) {
        return mul_mod(lhs, rhs);
    }
    return sub_mod(rhs, lhs);
}

__device__ size_t regular_row_with_offset(size_t row, int64_t offset, size_t domain_size) {
    const int64_t domain = static_cast<int64_t>(domain_size);
    int64_t normalized = offset % domain;
    if (normalized < 0) {
        normalized += domain;
    }
    const size_t row_offset = static_cast<size_t>(normalized);
    if (row_offset == 0) {
        return row;
    }
    const size_t wrap_at = domain_size - row_offset;
    return row < wrap_at ? row + row_offset : row - wrap_at;
}

__device__ const LzvmCudaRegularStage* regular_find_stage(
    const LzvmCudaRegularStage* stages,
    size_t stage_input_count,
    uint32_t stage_index) {
    for (size_t index = 0; index < stage_input_count; ++index) {
        if (stages[index].stage_index == stage_index) {
            return &stages[index];
        }
    }
    return nullptr;
}

__device__ bool regular_read_source(
    uint16_t buffer,
    size_t offset,
    size_t row_offset_index,
    size_t row,
    size_t domain_size,
    size_t stage_count,
    const uint64_t* fixed_values,
    size_t fixed_value_count,
    size_t fixed_column_count,
    const LzvmCudaRegularStage* stages,
    size_t stage_input_count,
    const int64_t* opening_point_offsets,
    size_t opening_point_offset_count,
    const uint64_t* numbers,
    size_t number_count,
    const uint64_t* unit_values,
    size_t unit_value_count,
    const uint64_t* tmp1,
    size_t tmp1_count,
    uint64_t* out) {
    const size_t source_buffer = static_cast<size_t>(buffer);
    if (source_buffer == 0) {
        if (row_offset_index >= opening_point_offset_count || offset >= fixed_column_count) {
            return false;
        }
        const size_t source_row =
            regular_row_with_offset(row, opening_point_offsets[row_offset_index], domain_size);
        const size_t index = source_row * fixed_column_count + offset;
        if (index >= fixed_value_count) {
            return false;
        }
        *out = fixed_values[index];
        return true;
    }
    if (source_buffer <= stage_count + 1) {
        if (row_offset_index >= opening_point_offset_count) {
            return false;
        }
        const LzvmCudaRegularStage* stage =
            regular_find_stage(stages, stage_input_count, static_cast<uint32_t>(source_buffer));
        if (stage == nullptr || offset >= stage->column_count) {
            return false;
        }
        const size_t source_row =
            regular_row_with_offset(row, opening_point_offsets[row_offset_index], domain_size);
        const size_t index = source_row * stage->column_count + offset;
        if (index >= stage->value_count) {
            return false;
        }
        *out = stage->values[index];
        return true;
    }

    const size_t base = 1 + stage_count + 3;
    if (source_buffer == base) {
        if (offset >= tmp1_count) {
            return false;
        }
        *out = tmp1[offset];
        return true;
    }
    if (source_buffer == base + 3) {
        if (offset >= number_count) {
            return false;
        }
        *out = numbers[offset];
        return true;
    }
    if (source_buffer == base + 4) {
        if (offset >= unit_value_count) {
            return false;
        }
        *out = unit_values[offset];
        return true;
    }
    return false;
}

__device__ bool regular_evaluate_entry_row(
    const LzvmCudaRegularConstraintEntry& entry,
    const uint8_t* ops,
    const uint16_t* args,
    const uint64_t* numbers,
    size_t number_count,
    const uint64_t* fixed_values,
    size_t fixed_value_count,
    size_t fixed_column_count,
    const LzvmCudaRegularStage* stages,
    size_t stage_input_count,
    size_t stage_count,
    const int64_t* opening_point_offsets,
    size_t opening_point_offset_count,
    const uint64_t* unit_values,
    size_t unit_value_count,
    size_t domain_size,
    size_t row,
    uint64_t* value) {
    uint64_t tmp1[kRegularMaxTmp1];
    for (size_t index = 0; index < entry.temp1_count; ++index) {
        tmp1[index] = 0;
    }

    for (size_t op_index = 0; op_index < entry.ops_count; ++op_index) {
        const size_t op_offset = static_cast<size_t>(entry.ops_offset) + op_index;
        const size_t cursor = static_cast<size_t>(entry.args_offset) + op_index * 8;
        if (ops[op_offset] != 0) {
            return false;
        }
        const uint16_t kind = args[cursor];
        const size_t destination = args[cursor + 1];
        uint64_t lhs = 0;
        uint64_t rhs = 0;
        const bool valid = destination < entry.temp1_count &&
            regular_read_source(
                args[cursor + 2], args[cursor + 3], args[cursor + 4], row, domain_size,
                stage_count, fixed_values, fixed_value_count, fixed_column_count, stages,
                stage_input_count, opening_point_offsets, opening_point_offset_count, numbers,
                number_count, unit_values, unit_value_count, tmp1, entry.temp1_count, &lhs) &&
            regular_read_source(
                args[cursor + 5], args[cursor + 6], args[cursor + 7], row, domain_size,
                stage_count, fixed_values, fixed_value_count, fixed_column_count, stages,
                stage_input_count, opening_point_offsets, opening_point_offset_count, numbers,
                number_count, unit_values, unit_value_count, tmp1, entry.temp1_count, &rhs);
        if (!valid) {
            return false;
        }
        tmp1[destination] = regular_apply_base_op(kind, lhs, rhs);
    }

    *value = tmp1[entry.destination_id];
    return true;
}

__device__ void regular_record_invalid_row(
    LzvmCudaRegularConstraintOutput* output,
    size_t row) {
    auto* row_ptr = reinterpret_cast<unsigned long long*>(&output->row);
    const auto candidate = static_cast<unsigned long long>(row);
    atomicMin(row_ptr, candidate);
}

__global__ void regular_constraints_base_entry_kernel(
    const LzvmCudaRegularConstraintEntry* entries,
    size_t entry_index,
    const uint8_t* ops,
    const uint16_t* args,
    const uint64_t* numbers,
    size_t number_count,
    const uint64_t* fixed_values,
    size_t fixed_value_count,
    size_t fixed_column_count,
    const LzvmCudaRegularStage* stages,
    size_t stage_input_count,
    size_t stage_count,
    const int64_t* opening_point_offsets,
    size_t opening_point_offset_count,
    const uint64_t* unit_values,
    size_t unit_value_count,
    size_t domain_size,
    LzvmCudaRegularConstraintOutput* out) {
    const LzvmCudaRegularConstraintEntry entry = entries[entry_index];
    const size_t first_row = min(static_cast<size_t>(entry.first_row), domain_size);
    const size_t last_row = min(static_cast<size_t>(entry.last_row), domain_size);
    const size_t stride = blockDim.x * gridDim.x;
    size_t row = first_row + blockIdx.x * blockDim.x + threadIdx.x;

    while (row < last_row) {
        uint64_t value = 0;
        if (regular_evaluate_entry_row(
                entry, ops, args, numbers, number_count, fixed_values, fixed_value_count,
                fixed_column_count, stages, stage_input_count, stage_count, opening_point_offsets,
                opening_point_offset_count, unit_values, unit_value_count, domain_size, row,
                &value) &&
            value != 0) {
            regular_record_invalid_row(&out[entry_index], row);
        }
        row += stride;
    }
}

__global__ void regular_constraints_base_value_kernel(
    const LzvmCudaRegularConstraintEntry* entries,
    size_t entry_count,
    const uint8_t* ops,
    const uint16_t* args,
    const uint64_t* numbers,
    size_t number_count,
    const uint64_t* fixed_values,
    size_t fixed_value_count,
    size_t fixed_column_count,
    const LzvmCudaRegularStage* stages,
    size_t stage_input_count,
    size_t stage_count,
    const int64_t* opening_point_offsets,
    size_t opening_point_offset_count,
    const uint64_t* unit_values,
    size_t unit_value_count,
    size_t domain_size,
    LzvmCudaRegularConstraintOutput* out) {
    const size_t entry_index = blockIdx.x * blockDim.x + threadIdx.x;
    if (entry_index >= entry_count) {
        return;
    }

    LzvmCudaRegularConstraintOutput* output = &out[entry_index];
    if (output->row == UINT64_MAX) {
        output->value = 0;
        output->found = 0;
        return;
    }

    uint64_t value = 0;
    if (regular_evaluate_entry_row(
            entries[entry_index], ops, args, numbers, number_count, fixed_values,
            fixed_value_count, fixed_column_count, stages, stage_input_count, stage_count,
            opening_point_offsets, opening_point_offset_count, unit_values, unit_value_count,
            domain_size, static_cast<size_t>(output->row), &value) &&
        value != 0) {
        output->value = value;
        output->found = 1;
    } else {
        output->value = 0;
        output->found = 0;
    }
}

template <typename T>
int copy_device_array(DeviceBuffer<T>& buffer, const T* values, size_t count) {
    if (count == 0) {
        return buffer.reset(0);
    }
    if (values == nullptr) {
        return -1;
    }
    LZVM_CUDA_RETURN_ON_ERROR(buffer.reset(count));
    return buffer.copy_from_bytes(values, count * sizeof(T));
}

const LzvmCudaRegularStage* regular_host_find_stage(
    const LzvmCudaRegularStage* stages,
    size_t stage_input_count,
    uint32_t stage_index) {
    for (size_t index = 0; index < stage_input_count; ++index) {
        if (stages[index].stage_index == stage_index) {
            return &stages[index];
        }
    }
    return nullptr;
}

int validate_regular_host_source(
    uint16_t buffer,
    size_t offset,
    size_t row_offset_index,
    size_t temp1_count,
    size_t stage_count,
    size_t fixed_column_count,
    const LzvmCudaRegularStage* stages,
    size_t stage_input_count,
    size_t opening_point_offset_count,
    size_t number_count,
    size_t unit_value_count) {
    const size_t source_buffer = static_cast<size_t>(buffer);
    if (source_buffer == 0) {
        return row_offset_index < opening_point_offset_count && offset < fixed_column_count ? 0 : -2;
    }
    if (source_buffer <= stage_count + 1) {
        const LzvmCudaRegularStage* stage =
            regular_host_find_stage(stages, stage_input_count, static_cast<uint32_t>(source_buffer));
        return stage != nullptr && row_offset_index < opening_point_offset_count &&
                offset < stage->column_count
            ? 0
            : -2;
    }
    const size_t base = 1 + stage_count + 3;
    if (source_buffer == base) {
        return offset < temp1_count ? 0 : -2;
    }
    if (source_buffer == base + 3) {
        return offset < number_count ? 0 : -2;
    }
    if (source_buffer == base + 4) {
        return offset < unit_value_count ? 0 : -2;
    }
    return -2;
}

int validate_regular_base_inputs(
    const LzvmCudaRegularConstraintEntry* entries,
    size_t entry_count,
    const uint8_t* ops,
    size_t ops_count,
    const uint16_t* args,
    size_t args_count,
    const uint64_t* numbers,
    size_t number_count,
    const uint64_t* fixed_values,
    size_t fixed_value_count,
    const uint64_t* fixed_values_device,
    size_t fixed_column_count,
    const LzvmCudaRegularStage* stages,
    size_t stage_input_count,
    size_t stage_count,
    const int64_t* opening_point_offsets,
    size_t opening_point_offset_count,
    const uint64_t* unit_values,
    size_t unit_value_count,
    size_t domain_size,
    const LzvmCudaRegularConstraintOutput* out) {
    if (domain_size == 0 || domain_size > static_cast<size_t>(std::numeric_limits<int64_t>::max())) {
        return -2;
    }
    if ((entry_count > 0 && (entries == nullptr || out == nullptr)) ||
        (ops_count > 0 && ops == nullptr) ||
        (args_count > 0 && args == nullptr) ||
        (number_count > 0 && numbers == nullptr) ||
        (fixed_value_count > 0 && fixed_values == nullptr && fixed_values_device == nullptr) ||
        (stage_input_count > 0 && stages == nullptr) ||
        (opening_point_offset_count > 0 && opening_point_offsets == nullptr) ||
        (unit_value_count > 0 && unit_values == nullptr)) {
        return -1;
    }
    if (fixed_column_count > 0 && fixed_value_count % fixed_column_count != 0) {
        return -2;
    }
    for (size_t stage_index = 0; stage_index < stage_input_count; ++stage_index) {
        const LzvmCudaRegularStage& stage = stages[stage_index];
        if (stage.column_count == 0 || stage.value_count % stage.column_count != 0 ||
            (stage.value_count > 0 && stage.values == nullptr)) {
            return -2;
        }
    }
    for (size_t entry_index = 0; entry_index < entry_count; ++entry_index) {
        const LzvmCudaRegularConstraintEntry& entry = entries[entry_index];
        if (entry.temp1_count == 0 || entry.temp1_count > kRegularMaxTmp1 ||
            entry.destination_id >= entry.temp1_count ||
            entry.args_count != entry.ops_count * 8) {
            return -2;
        }
        const size_t ops_end = static_cast<size_t>(entry.ops_offset) + entry.ops_count;
        const size_t args_end = static_cast<size_t>(entry.args_offset) + entry.args_count;
        if (ops_end > ops_count || args_end > args_count) {
            return -2;
        }
        for (size_t op_index = 0; op_index < entry.ops_count; ++op_index) {
            const size_t op_offset = static_cast<size_t>(entry.ops_offset) + op_index;
            const size_t cursor = static_cast<size_t>(entry.args_offset) + op_index * 8;
            if (ops[op_offset] != 0 || args[cursor] > 3 || args[cursor + 1] >= entry.temp1_count) {
                return -2;
            }
            LZVM_CUDA_RETURN_ON_ERROR(validate_regular_host_source(
                args[cursor + 2], args[cursor + 3], args[cursor + 4], entry.temp1_count,
                stage_count, fixed_column_count, stages, stage_input_count,
                opening_point_offset_count, number_count, unit_value_count));
            LZVM_CUDA_RETURN_ON_ERROR(validate_regular_host_source(
                args[cursor + 5], args[cursor + 6], args[cursor + 7], entry.temp1_count,
                stage_count, fixed_column_count, stages, stage_input_count,
                opening_point_offset_count, number_count, unit_value_count));
        }
    }
    return 0;
}

extern "C" int lzvm_cuda_regular_constraints_base(
    const LzvmCudaRegularConstraintEntry* entries,
    size_t entry_count,
    const uint8_t* ops,
    size_t ops_count,
    const uint16_t* args,
    size_t args_count,
    const uint64_t* numbers,
    size_t number_count,
    const uint64_t* fixed_values,
    size_t fixed_value_count,
    const uint64_t* fixed_values_device,
    size_t fixed_column_count,
    const LzvmCudaRegularStage* stages,
    size_t stage_input_count,
    size_t stage_count,
    const int64_t* opening_point_offsets,
    size_t opening_point_offset_count,
    const uint64_t* unit_values,
    size_t unit_value_count,
    size_t domain_size,
    LzvmCudaRegularConstraintOutput* out) {
    LZVM_CUDA_RETURN_ON_ERROR(validate_regular_base_inputs(
        entries, entry_count, ops, ops_count, args, args_count, numbers, number_count,
        fixed_values, fixed_value_count, fixed_values_device, fixed_column_count, stages, stage_input_count,
        stage_count, opening_point_offsets, opening_point_offset_count, unit_values,
        unit_value_count, domain_size, out));
    if (entry_count == 0) {
        return 0;
    }

    DeviceBuffer<LzvmCudaRegularConstraintEntry> device_entries;
    DeviceBuffer<uint8_t> device_ops;
    DeviceBuffer<uint16_t> device_args;
    DeviceBuffer<uint64_t> device_numbers;
    DeviceBuffer<uint64_t> device_fixed_values;
    DeviceBuffer<int64_t> device_opening_point_offsets;
    DeviceBuffer<uint64_t> device_unit_values;
    DeviceBuffer<LzvmCudaRegularStage> device_stages;
    DeviceBuffer<LzvmCudaRegularConstraintOutput> device_out;

    LZVM_CUDA_RETURN_ON_ERROR(copy_device_array(device_entries, entries, entry_count));
    LZVM_CUDA_RETURN_ON_ERROR(copy_device_array(device_ops, ops, ops_count));
    LZVM_CUDA_RETURN_ON_ERROR(copy_device_array(device_args, args, args_count));
    LZVM_CUDA_RETURN_ON_ERROR(copy_device_array(device_numbers, numbers, number_count));
    const uint64_t* regular_fixed_values = fixed_values_device;
    if (regular_fixed_values == nullptr) {
        LZVM_CUDA_RETURN_ON_ERROR(
            copy_device_array(device_fixed_values, fixed_values, fixed_value_count));
        regular_fixed_values = device_fixed_values.data();
    }
    LZVM_CUDA_RETURN_ON_ERROR(copy_device_array(
        device_opening_point_offsets, opening_point_offsets, opening_point_offset_count));
    LZVM_CUDA_RETURN_ON_ERROR(copy_device_array(device_unit_values, unit_values, unit_value_count));

    std::vector<DeviceBuffer<uint64_t>> stage_value_buffers(stage_input_count);
    std::vector<LzvmCudaRegularStage> device_stage_descriptors(stage_input_count);
    for (size_t index = 0; index < stage_input_count; ++index) {
        LZVM_CUDA_RETURN_ON_ERROR(
            copy_device_array(stage_value_buffers[index], stages[index].values, stages[index].value_count));
        device_stage_descriptors[index] = stages[index];
        device_stage_descriptors[index].values = stage_value_buffers[index].data();
    }
    LZVM_CUDA_RETURN_ON_ERROR(copy_device_array(
        device_stages, device_stage_descriptors.data(), device_stage_descriptors.size()));

    std::vector<LzvmCudaRegularConstraintOutput> initial_out(entry_count);
    for (size_t index = 0; index < entry_count; ++index) {
        initial_out[index].row = UINT64_MAX;
        initial_out[index].value = 0;
        initial_out[index].found = 0;
    }
    LZVM_CUDA_RETURN_ON_ERROR(copy_device_array(device_out, initial_out.data(), initial_out.size()));

    for (size_t entry_index = 0; entry_index < entry_count; ++entry_index) {
        const LzvmCudaRegularConstraintEntry& entry = entries[entry_index];
        const size_t first_row = std::min(static_cast<size_t>(entry.first_row), domain_size);
        const size_t last_row = std::min(static_cast<size_t>(entry.last_row), domain_size);
        if (first_row >= last_row) {
            continue;
        }
        const size_t row_count = last_row - first_row;
        const size_t blocks = (row_count + kThreads - 1) / kThreads;
        regular_constraints_base_entry_kernel<<<blocks, kThreads>>>(
            device_entries.data(),
            entry_index,
            device_ops.data(),
            device_args.data(),
            device_numbers.data(),
            number_count,
            regular_fixed_values,
            fixed_value_count,
            fixed_column_count,
            device_stages.data(),
            stage_input_count,
            stage_count,
            device_opening_point_offsets.data(),
            opening_point_offset_count,
            device_unit_values.data(),
            unit_value_count,
            domain_size,
            device_out.data());
        LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_check_launch());
    }
    const size_t result_blocks = (entry_count + kThreads - 1) / kThreads;
    regular_constraints_base_value_kernel<<<result_blocks, kThreads>>>(
        device_entries.data(),
        entry_count,
        device_ops.data(),
        device_args.data(),
        device_numbers.data(),
        number_count,
        regular_fixed_values,
        fixed_value_count,
        fixed_column_count,
        device_stages.data(),
        stage_input_count,
        stage_count,
        device_opening_point_offsets.data(),
        opening_point_offset_count,
        device_unit_values.data(),
        unit_value_count,
        domain_size,
        device_out.data());
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_check_launch());
    LZVM_CUDA_RETURN_ON_ERROR(lzvm_cuda_synchronize());
    LZVM_CUDA_RETURN_ON_ERROR(
        device_out.copy_to_bytes(out, entry_count * sizeof(LzvmCudaRegularConstraintOutput)));
    return 0;
}
