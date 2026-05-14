use std::io::Write;

use lzvm_artifacts::key_directory::read_key_directory_catalog;

pub fn run_cli(args: &[&str], stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    match args {
        ["setup", "validate", setup_dir] => validate_setup_directory(setup_dir, stdout, stderr),
        _ => write_usage(stderr),
    }
}

fn validate_setup_directory(
    setup_dir: &str,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    match read_key_directory_catalog(setup_dir) {
        Ok(catalog) => {
            let fixed_bytes = catalog
                .units
                .iter()
                .map(|unit| unit.actual_fixed_bytes)
                .sum::<u64>();
            let _ = writeln!(stdout, "status=ok");
            let _ = writeln!(stdout, "units={}", catalog.units.len());
            let _ = writeln!(
                stdout,
                "global_constraints={}",
                catalog.global_constraints.entries.len()
            );
            let _ = writeln!(stdout, "fixed_bytes={fixed_bytes}");
            0
        }
        Err(error) => {
            let _ = writeln!(stderr, "setup validation failed: {error}");
            1
        }
    }
}

fn write_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(stderr, "usage: lzvm setup validate <setup-dir>");
    2
}
