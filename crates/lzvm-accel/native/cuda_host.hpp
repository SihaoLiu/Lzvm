#pragma once

#include <cstddef>

extern "C" int lzvm_cuda_alloc_bytes(void** out, std::size_t bytes);
extern "C" void lzvm_cuda_free_bytes(void* ptr);
extern "C" int lzvm_cuda_copy_h2d_bytes(void* dst, const void* src, std::size_t bytes);
extern "C" int lzvm_cuda_copy_d2h_bytes(void* dst, const void* src, std::size_t bytes);
extern "C" int lzvm_cuda_check_launch(void);
extern "C" int lzvm_cuda_synchronize(void);

template <typename T>
class DeviceBuffer {
public:
    DeviceBuffer() = default;
    DeviceBuffer(const DeviceBuffer&) = delete;
    DeviceBuffer& operator=(const DeviceBuffer&) = delete;

    ~DeviceBuffer() {
        release();
    }

    int reset(std::size_t count) {
        release();
        if (count == 0) {
            return 0;
        }
        const std::size_t bytes = count * sizeof(T);
        const int status = lzvm_cuda_alloc_bytes(reinterpret_cast<void**>(&ptr_), bytes);
        if (status == 0) {
            count_ = count;
        } else {
            ptr_ = nullptr;
            count_ = 0;
        }
        return status;
    }

    void release() {
        if (ptr_ != nullptr) {
            lzvm_cuda_free_bytes(ptr_);
            ptr_ = nullptr;
            count_ = 0;
        }
    }

    T* data() {
        return ptr_;
    }

    const T* data() const {
        return ptr_;
    }

    int copy_from_bytes(const void* src, std::size_t bytes) const {
        if (bytes == 0) {
            return 0;
        }
        return lzvm_cuda_copy_h2d_bytes(ptr_, src, bytes);
    }

    int copy_to_bytes(void* dst, std::size_t bytes) const {
        if (bytes == 0) {
            return 0;
        }
        return lzvm_cuda_copy_d2h_bytes(dst, ptr_, bytes);
    }

    std::size_t count() const {
        return count_;
    }

    std::size_t bytes() const {
        return count_ * sizeof(T);
    }

private:
    T* ptr_ = nullptr;
    std::size_t count_ = 0;
};
