use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=native/cuda_field.cu");
    println!("cargo:rerun-if-env-changed=LZVM_CUDA_ARCH");

    if std::env::var_os("CARGO_FEATURE_CUDA").is_none() {
        return;
    }

    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("out dir"));
    let source = manifest_dir.join("native/cuda_field.cu");
    let library = out_dir.join("liblzvm_cuda_field.a");
    let arch = std::env::var("LZVM_CUDA_ARCH").unwrap_or_else(|_| "sm_120".to_owned());

    let status = Command::new("nvcc")
        .arg("-std=c++17")
        .arg(format!("-arch={arch}"))
        .arg("-lib")
        .arg("-Xcompiler")
        .arg("-fPIC")
        .arg(&source)
        .arg("-o")
        .arg(&library)
        .status()
        .expect("failed to run nvcc");
    if !status.success() {
        panic!("nvcc failed while building {}", source.display());
    }

    let cuda_home = std::env::var("CUDA_HOME")
        .or_else(|_| std::env::var("CUDA_PATH"))
        .unwrap_or_else(|_| "/usr/local/cuda".to_owned());
    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-search=native={cuda_home}/lib64");
    println!("cargo:rustc-link-lib=static=lzvm_cuda_field");
    println!("cargo:rustc-link-lib=dylib=cudart");
    println!("cargo:rustc-link-lib=dylib=stdc++");
}
