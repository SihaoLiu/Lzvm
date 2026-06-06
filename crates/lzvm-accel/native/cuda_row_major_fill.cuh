#pragma once

__global__ void fill_row_major_column_u64_kernel(
    uint64_t* dst,
    size_t rows_to_fill,
    size_t row_width_words,
    size_t start_row,
    size_t column,
    uint64_t value) {
    const size_t index = blockIdx.x * blockDim.x + threadIdx.x;
    if (index >= rows_to_fill) {
        return;
    }
    dst[(start_row + index) * row_width_words + column] = value;
}

__global__ void fill_row_major_suffix_from_row_u64_kernel(
    uint64_t* dst,
    const uint64_t* row_values,
    size_t suffix_word_count,
    size_t row_width_words,
    size_t start_word) {
    const size_t word = blockIdx.x * blockDim.x + threadIdx.x;
    if (word < suffix_word_count) {
        dst[start_word + word] = row_values[word % row_width_words];
    }
}

__global__ void scatter_sparse_u64_words_kernel(
    uint64_t* dst,
    const uint64_t* indices,
    const uint64_t* values,
    size_t sparse_word_count) {
    const size_t index = blockIdx.x * blockDim.x + threadIdx.x;
    if (index < sparse_word_count) {
        dst[indices[index]] = values[index];
    }
}

__global__ void scatter_sparse_u32_indices_u64_words_kernel(
    uint64_t* dst,
    const uint32_t* indices,
    const uint64_t* values,
    size_t sparse_word_count) {
    const size_t index = blockIdx.x * blockDim.x + threadIdx.x;
    if (index < sparse_word_count) {
        dst[indices[index]] = values[index];
    }
}

extern "C" int lzvm_cuda_fill_row_major_column_u64(
    uint64_t* dst,
    size_t row_count,
    size_t row_width_words,
    size_t start_row,
    size_t column,
    uint64_t value) {
    if (start_row > row_count) {
        return -2;
    }
    if (start_row == row_count) {
        return 0;
    }
    if (dst == nullptr) {
        return -1;
    }
    if (row_width_words == 0 || column >= row_width_words) {
        return -2;
    }
    const size_t rows_to_fill = row_count - start_row;
    const size_t blocks = (rows_to_fill + kThreads - 1) / kThreads;
    if (blocks > static_cast<size_t>(std::numeric_limits<int>::max())) {
        return -2;
    }
    fill_row_major_column_u64_kernel<<<static_cast<int>(blocks), kThreads>>>(
        dst, rows_to_fill, row_width_words, start_row, column, value);
    return lzvm_cuda_check_launch();
}

extern "C" int lzvm_cuda_fill_row_major_suffix_from_row_u64(
    uint64_t* dst,
    const uint64_t* row_values,
    size_t row_count,
    size_t row_width_words,
    size_t start_row) {
    if (start_row > row_count) {
        return -2;
    }
    if (start_row == row_count) {
        return 0;
    }
    if (dst == nullptr || row_values == nullptr) {
        return -1;
    }
    if (row_width_words == 0) {
        return -2;
    }
    const size_t rows_to_fill = row_count - start_row;
    if (rows_to_fill > std::numeric_limits<size_t>::max() / row_width_words) {
        return -2;
    }
    const size_t suffix_word_count = rows_to_fill * row_width_words;
    const size_t blocks = (suffix_word_count + kThreads - 1) / kThreads;
    if (blocks > static_cast<size_t>(std::numeric_limits<int>::max())) {
        return -2;
    }
    const size_t start_word = start_row * row_width_words;
    fill_row_major_suffix_from_row_u64_kernel<<<static_cast<int>(blocks), kThreads>>>(
        dst, row_values, suffix_word_count, row_width_words, start_word);
    return lzvm_cuda_check_launch();
}

extern "C" int lzvm_cuda_scatter_sparse_u64_words(
    uint64_t* dst,
    const uint64_t* indices,
    const uint64_t* values,
    size_t sparse_word_count) {
    if (sparse_word_count == 0) {
        return 0;
    }
    if (dst == nullptr || indices == nullptr || values == nullptr) {
        return -1;
    }
    const size_t blocks = (sparse_word_count + kThreads - 1) / kThreads;
    if (blocks > static_cast<size_t>(std::numeric_limits<int>::max())) {
        return -2;
    }
    scatter_sparse_u64_words_kernel<<<static_cast<int>(blocks), kThreads>>>(
        dst, indices, values, sparse_word_count);
    return lzvm_cuda_check_launch();
}

extern "C" int lzvm_cuda_scatter_sparse_u32_indices_u64_words(
    uint64_t* dst,
    const uint32_t* indices,
    const uint64_t* values,
    size_t sparse_word_count) {
    if (sparse_word_count == 0) {
        return 0;
    }
    if (dst == nullptr || indices == nullptr || values == nullptr) {
        return -1;
    }
    const size_t blocks = (sparse_word_count + kThreads - 1) / kThreads;
    if (blocks > static_cast<size_t>(std::numeric_limits<int>::max())) {
        return -2;
    }
    scatter_sparse_u32_indices_u64_words_kernel<<<static_cast<int>(blocks), kThreads>>>(
        dst, indices, values, sparse_word_count);
    return lzvm_cuda_check_launch();
}
