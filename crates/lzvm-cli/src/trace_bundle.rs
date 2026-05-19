use std::io::Write;
use std::path::Path;

use lzvm_artifacts::trace_bundle::{encode_trace_bundle, TraceBundle, TraceBundleUnit};

pub(crate) fn run(
    out_bundle: &str,
    unit_args: &[&str],
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    if unit_args.is_empty() || !unit_args.len().is_multiple_of(2) {
        return write_usage(stderr);
    }

    let mut units = Vec::with_capacity(unit_args.len() / 2);
    for pair in unit_args.chunks_exact(2) {
        let Some(unit_index) =
            parse_u32_arg(pair[0], "unit index", "prove trace bundle write", stderr)
        else {
            return 1;
        };
        let trace_bytes = match std::fs::read(pair[1]) {
            Ok(bytes) => bytes,
            Err(error) => {
                let _ = writeln!(
                    stderr,
                    "prove trace bundle write failed: read trace bytes failed: {}: {error}",
                    pair[1]
                );
                return 1;
            }
        };
        units.push(TraceBundleUnit {
            unit_index,
            trace_bytes,
        });
    }

    let bytes = match encode_trace_bundle(&TraceBundle { units }) {
        Ok(bytes) => bytes,
        Err(error) => {
            let _ = writeln!(stderr, "prove trace bundle write failed: {error}");
            return 1;
        }
    };
    let output_path = Path::new(out_bundle);
    if let Some(parent) = output_path.parent() {
        if !parent.as_os_str().is_empty() {
            if let Err(error) = std::fs::create_dir_all(parent) {
                let _ = writeln!(
                    stderr,
                    "prove trace bundle write failed: create output directory failed: {}: {error}",
                    parent.display()
                );
                return 1;
            }
        }
    }
    if let Err(error) = std::fs::write(output_path, &bytes) {
        let _ = writeln!(
            stderr,
            "prove trace bundle write failed: write output failed: {}: {error}",
            output_path.display()
        );
        return 1;
    }

    let _ = writeln!(stdout, "status=ok");
    let _ = writeln!(stdout, "units={}", unit_args.len() / 2);
    let _ = writeln!(stdout, "bytes_written={}", bytes.len());
    let _ = writeln!(stdout, "output={}", output_path.display());
    0
}

pub(crate) fn write_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm prove write-trace-bundle <out-bundle> <unit-index> <trace-bin>..."
    );
    2
}

fn parse_u32_arg(value: &str, name: &str, role: &str, stderr: &mut dyn Write) -> Option<u32> {
    value.parse().map_or_else(
        |_| {
            let _ = writeln!(stderr, "{role} failed: invalid {name}: {value}");
            None
        },
        Some,
    )
}
