use std::io::Write;
use std::path::Path;

use lzvm_artifacts::fixed::{read_fixed_columns_file, FixedColumn, FixedColumns};
use lzvm_artifacts::key_directory::read_key_directory_catalog;
use lzvm_artifacts::setup_info::read_unit_setup_info_file;
use lzvm_setup::write_base_fixed_columns;
use serde_json::Value;

pub fn run_cli(args: &[&str], stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    match args {
        ["setup", "validate", setup_dir] => validate_setup_directory(setup_dir, stdout, stderr),
        ["setup", "validate", ..] => write_validate_usage(stderr),
        ["setup", "write-fixed", setup_info, columns_json, out_const] => {
            write_fixed_columns(setup_info, columns_json, out_const, stdout, stderr)
        }
        ["setup", "write-fixed", ..] => write_fixed_usage(stderr),
        ["setup", "write-fixed-bin", setup_info, columns_bin, out_const] => {
            write_fixed_columns_bin(setup_info, columns_bin, out_const, stdout, stderr)
        }
        ["setup", "write-fixed-bin", ..] => write_fixed_bin_usage(stderr),
        _ => write_validate_usage(stderr),
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

fn write_fixed_columns(
    setup_info: &str,
    columns_json: &str,
    out_const: &str,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let setup = match read_unit_setup_info_file(setup_info) {
        Ok(setup) => setup,
        Err(error) => {
            let _ = writeln!(stderr, "setup fixed-column write failed: {error}");
            return 1;
        }
    };
    let columns = match read_fixed_columns_json(columns_json) {
        Ok(columns) => columns,
        Err(error) => {
            let _ = writeln!(stderr, "setup fixed-column write failed: {error}");
            return 1;
        }
    };

    publish_fixed_columns(out_const, &columns, &setup, stdout, stderr)
}

fn write_fixed_columns_bin(
    setup_info: &str,
    columns_bin: &str,
    out_const: &str,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let setup = match read_unit_setup_info_file(setup_info) {
        Ok(setup) => setup,
        Err(error) => {
            let _ = writeln!(stderr, "setup fixed-column write failed: {error}");
            return 1;
        }
    };
    let columns = match read_fixed_columns_file(columns_bin) {
        Ok(columns) => columns,
        Err(error) => {
            let _ = writeln!(stderr, "setup fixed-column write failed: {error}");
            return 1;
        }
    };

    publish_fixed_columns(out_const, &columns, &setup, stdout, stderr)
}

fn read_fixed_columns_json(path: impl AsRef<Path>) -> Result<FixedColumns, String> {
    let path = path.as_ref();
    let input = std::fs::read_to_string(path)
        .map_err(|error| format!("read {}: {error}", path.display()))?;
    parse_fixed_columns_json(&input)
}

fn parse_fixed_columns_json(input: &str) -> Result<FixedColumns, String> {
    let value = serde_json::from_str::<Value>(input).map_err(|error| error.to_string())?;
    let object = value
        .as_object()
        .ok_or_else(|| "fixed-column source must be a JSON object".to_owned())?;
    let group_name = read_string_field(object, "group_name")?;
    let unit_name = read_string_field(object, "unit_name")?;
    let row_count = read_u64_field(object, "row_count")?;
    let columns_value = object
        .get("columns")
        .and_then(Value::as_array)
        .ok_or_else(|| "fixed-column source must contain a columns array".to_owned())?;
    let mut columns = Vec::with_capacity(columns_value.len());
    for column_value in columns_value {
        let column = column_value
            .as_object()
            .ok_or_else(|| "fixed-column entry must be a JSON object".to_owned())?;
        columns.push(FixedColumn {
            name: read_string_field(column, "name")?,
            dimensions: read_u32_array(column, "dimensions")?,
            values: read_u64_array(column, "values")?,
        });
    }

    Ok(FixedColumns {
        group_name,
        unit_name,
        row_count,
        columns,
    })
}

fn publish_fixed_columns(
    out_const: &str,
    columns: &FixedColumns,
    setup: &lzvm_artifacts::setup_info::UnitSetupInfo,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    match write_base_fixed_columns(out_const, columns, setup) {
        Ok(report) => {
            let _ = writeln!(stdout, "status=ok");
            let _ = writeln!(stdout, "bytes_written={}", report.bytes_written);
            let _ = writeln!(stdout, "output={}", report.path.display());
            0
        }
        Err(error) => {
            let _ = writeln!(stderr, "setup fixed-column write failed: {error}");
            1
        }
    }
}

fn read_string_field(
    object: &serde_json::Map<String, Value>,
    field: &str,
) -> Result<String, String> {
    object
        .get(field)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("missing string field {field}"))
}

fn read_u64_field(object: &serde_json::Map<String, Value>, field: &str) -> Result<u64, String> {
    object
        .get(field)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("missing integer field {field}"))
}

fn read_u32_array(
    object: &serde_json::Map<String, Value>,
    field: &str,
) -> Result<Vec<u32>, String> {
    read_u64_array(object, field)?
        .into_iter()
        .map(|value| u32::try_from(value).map_err(|_| format!("{field} entry is too large")))
        .collect()
}

fn read_u64_array(
    object: &serde_json::Map<String, Value>,
    field: &str,
) -> Result<Vec<u64>, String> {
    object
        .get(field)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("missing integer array field {field}"))?
        .iter()
        .map(|value| {
            value
                .as_u64()
                .ok_or_else(|| format!("{field} entry must be an unsigned integer"))
        })
        .collect()
}

fn write_validate_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(stderr, "usage: lzvm setup validate <setup-dir>");
    2
}

fn write_fixed_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm setup write-fixed <setup-info-json> <columns-json> <out-const>"
    );
    2
}

fn write_fixed_bin_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm setup write-fixed-bin <setup-info-json> <columns-bin> <out-const>"
    );
    2
}
