use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=native/cuda_field.cu");
    println!("cargo:rerun-if-changed=native/cuda_field_constants.cuh");
    println!("cargo:rerun-if-changed=native/cuda_goldilocks_canonical.cuh");
    println!("cargo:rerun-if-changed=native/cuda_goldilocks_ntt.cuh");
    println!("cargo:rerun-if-changed=native/cuda_goldilocks_row_extend.cuh");
    println!("cargo:rerun-if-changed=native/cuda_row_major_fill.cuh");
    println!("cargo:rerun-if-changed=native/cuda_zisk_main_trace.cuh");
    println!("cargo:rerun-if-changed=native/cuda_main_trace_layout.cuh");
    println!("cargo:rerun-if-changed=native/cuda_poseidon2_merkle_exports.cuh");
    println!("cargo:rerun-if-changed=native/cuda_poseidon2_merkle_digest.cuh");
    println!("cargo:rerun-if-changed=native/cuda_poseidon2_merkle_opening.cuh");
    println!("cargo:rerun-if-changed=native/cuda_poseidon2_merkle_parent.cuh");
    println!("cargo:rerun-if-changed=native/cuda_poseidon2_merkle_root.cuh");
    println!("cargo:rerun-if-changed=native/cuda_poseidon2_permutation.cuh");
    println!("cargo:rerun-if-changed=native/cuda_poseidon2_row_major.cuh");
    println!("cargo:rerun-if-changed=native/cuda_poseidon2_row_major_exports.cuh");
    println!("cargo:rerun-if-changed=native/cuda_regular_constraints.cuh");
    println!("cargo:rerun-if-changed=native/cuda_host.cpp");
    println!("cargo:rerun-if-changed=native/cuda_host_state_prefix.cuh");
    println!("cargo:rerun-if-changed=native/cuda_host_runtime.cpp");
    println!("cargo:rerun-if-changed=native/cuda_host.hpp");
    println!("cargo:rerun-if-env-changed=LZVM_CUDA_ARCH");

    if std::env::var_os("CARGO_FEATURE_CUDA").is_none() {
        return;
    }

    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("out dir"));
    let source = manifest_dir.join("native/cuda_field.cu");
    let host_source = manifest_dir.join("native/cuda_host.cpp");
    let host_runtime_source = manifest_dir.join("native/cuda_host_runtime.cpp");
    let cuda_object = out_dir.join("cuda_field.o");
    let host_object = out_dir.join("cuda_host.o");
    let host_runtime_object = out_dir.join("cuda_host_runtime.o");
    let library = out_dir.join("liblzvm_cuda_field.a");
    let arch = std::env::var("LZVM_CUDA_ARCH").unwrap_or_else(|_| "sm_120".to_owned());
    let cuda_home = std::env::var("CUDA_HOME")
        .or_else(|_| std::env::var("CUDA_PATH"))
        .unwrap_or_else(|_| "/usr/local/cuda".to_owned());
    let native_include = manifest_dir.join("native");

    let status = Command::new("nvcc")
        .arg("-std=c++17")
        .arg(format!("-arch={arch}"))
        .arg("-c")
        .arg(format!("-I{}", native_include.display()))
        .arg("-Xcompiler")
        .arg("-fPIC")
        .arg(&source)
        .arg("-o")
        .arg(&cuda_object)
        .status()
        .expect("failed to run nvcc");
    if !status.success() {
        panic!("nvcc failed while building {}", source.display());
    }

    for (source, object) in [
        (&host_source, &host_object),
        (&host_runtime_source, &host_runtime_object),
    ] {
        let status = Command::new("c++")
            .arg("-std=c++17")
            .arg("-fPIC")
            .arg(format!("-I{}", native_include.display()))
            .arg(format!("-I{cuda_home}/include"))
            .arg("-c")
            .arg(source)
            .arg("-o")
            .arg(object)
            .status()
            .expect("failed to run c++");
        if !status.success() {
            panic!("c++ failed while building {}", source.display());
        }
    }

    let _ = std::fs::remove_file(&library);
    let status = Command::new("ar")
        .arg("crs")
        .arg(&library)
        .arg(&cuda_object)
        .arg(&host_object)
        .arg(&host_runtime_object)
        .status()
        .expect("failed to run ar");
    if !status.success() {
        panic!("ar failed while building {}", library.display());
    }

    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-search=native={cuda_home}/lib64");
    println!("cargo:rustc-link-lib=static=lzvm_cuda_field");
    println!("cargo:rustc-link-lib=dylib=cudart");
    println!("cargo:rustc-link-lib=dylib=stdc++");
}
