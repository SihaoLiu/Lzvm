use std::io::Write;

pub(super) fn write_usage(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "usage: lzvm prove witness [options] <setup-dir> <output-dir> <witness-library> <guest-image> [public-inputs]\n       lzvm prove witness --trace-bytes <trace-bin> [options] <setup-dir> <output-dir> <guest-image> [public-inputs]\n       lzvm prove witness --trace-bundle <bundle-bin> [options] <setup-dir> <output-dir> <guest-image> [public-inputs]\n       lzvm prove witness --guest-pc-trace <instruction-limit> [options] <setup-dir> <output-dir> <guest-image> [public-inputs]\n  --eth-block-input <block-input>\n  --eth-public-input <public-input>\n  --eth-public-input-allow-trailing\n  --program-image-cache <cache-bin>\n  --trace-bytes <trace-bin>\n  --trace-bundle <bundle-bin>\n  --guest-pc-trace <instruction-limit>\n  --unit-index <index>\n  --timings"
    );
    2
}
