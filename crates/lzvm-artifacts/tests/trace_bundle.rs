use std::fs;
use std::path::PathBuf;

use lzvm_artifacts::trace_bundle::{
    encode_trace_bundle, parse_trace_bundle, read_trace_bundle_file, TraceBundle, TraceBundleError,
    TraceBundleUnit,
};

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("lzvm-trace-bundle-{}-{name}", std::process::id()))
}

fn sample_bundle() -> TraceBundle {
    TraceBundle {
        units: vec![
            TraceBundleUnit {
                unit_index: 0,
                trace_bytes: vec![1, 2, 3, 4],
            },
            TraceBundleUnit {
                unit_index: 2,
                trace_bytes: vec![5, 6],
            },
        ],
    }
}

#[test]
fn encodes_and_parses_trace_bundles() {
    let encoded = encode_trace_bundle(&sample_bundle()).expect("bundle should encode");
    let parsed = parse_trace_bundle(&encoded).expect("bundle should parse");

    assert_eq!(&encoded[0..4], b"trb0");
    assert_eq!(parsed, sample_bundle());
    assert_eq!(parsed.unit_count(), 2);
    assert_eq!(parsed.trace_bytes_for_unit(2), Some(&[5_u8, 6][..]));
    assert_eq!(parsed.trace_bytes_for_unit(1), None);
}

#[test]
fn encodes_trace_bundle_units_in_index_order() {
    let ordered = sample_bundle();
    let reversed = TraceBundle {
        units: ordered.units.iter().cloned().rev().collect(),
    };

    let ordered_bytes = encode_trace_bundle(&ordered).expect("ordered bundle should encode");
    let reversed_bytes = encode_trace_bundle(&reversed).expect("reversed bundle should encode");
    let parsed_reversed = parse_trace_bundle(&reversed_bytes).expect("bundle should parse");

    assert_eq!(reversed_bytes, ordered_bytes);
    assert_eq!(parsed_reversed, ordered);
}

#[test]
fn reads_trace_bundle_files() {
    let dir = temp_dir("read-file");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture directory should be created");
    let path = dir.join("trace-bundle.bin");
    let encoded = encode_trace_bundle(&sample_bundle()).expect("bundle should encode");
    fs::write(&path, encoded).expect("bundle should be written");

    let parsed = read_trace_bundle_file(&path).expect("bundle should read");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(parsed, sample_bundle());
}

#[test]
fn rejects_empty_trace_bundles() {
    let bundle = TraceBundle { units: Vec::new() };

    assert!(matches!(
        encode_trace_bundle(&bundle),
        Err(TraceBundleError::EmptyUnits)
    ));
}

#[test]
fn rejects_trace_bundle_units_without_bytes() {
    let bundle = TraceBundle {
        units: vec![TraceBundleUnit {
            unit_index: 3,
            trace_bytes: Vec::new(),
        }],
    };

    assert!(matches!(
        encode_trace_bundle(&bundle),
        Err(TraceBundleError::EmptyTraceBytes { unit_index: 3 })
    ));
}

#[test]
fn rejects_duplicate_trace_bundle_units() {
    let bundle = TraceBundle {
        units: vec![
            TraceBundleUnit {
                unit_index: 1,
                trace_bytes: vec![1],
            },
            TraceBundleUnit {
                unit_index: 1,
                trace_bytes: vec![2],
            },
        ],
    };

    assert!(matches!(
        encode_trace_bundle(&bundle),
        Err(TraceBundleError::DuplicateUnitIndex { unit_index: 1 })
    ));
}
