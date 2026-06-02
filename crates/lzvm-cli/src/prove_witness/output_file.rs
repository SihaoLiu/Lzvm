use std::fs;
use std::path::Path;

pub(super) fn write_proof_output(output_dir: &Path, proof_bytes: &[u8]) -> Result<(), String> {
    fs::create_dir_all(output_dir).map_err(|error| {
        format!(
            "create output directory failed: {}: {error}",
            output_dir.display()
        )
    })?;
    write_output_file(&output_dir.join("proof.bin"), proof_bytes)
}

pub(super) fn write_output_file(path: &Path, value: &[u8]) -> Result<(), String> {
    fs::write(path, value)
        .map_err(|error| format!("write output file failed: {}: {error}", path.display()))
}
