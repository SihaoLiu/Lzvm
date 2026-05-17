use std::fs;
use std::path::{Path, PathBuf};

use lzvm_artifacts::eth_block_input::parse_eth_block_input;
use lzvm_cli::run_cli;

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "lzvm-eth-block-input-cli-{}-{name}",
        std::process::id()
    ))
}

fn write_bytes(path: &Path, bytes: impl AsRef<[u8]>) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("parent directory should be created");
    }
    fs::write(path, bytes).expect("fixture bytes should be written");
}

#[test]
fn writes_binary_block_input_artifact() {
    let dir = temp_dir("binary");
    let _ = fs::remove_dir_all(&dir);
    let block_path = dir.join("block.rlp");
    let output_path = dir.join("block.input");
    let block_rlp = sample_block_rlp();
    write_bytes(&block_path, &block_rlp);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "eth",
            "write-block-input",
            block_path.to_str().expect("block path should be utf-8"),
            output_path.to_str().expect("output path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    let encoded = fs::read(&output_path).expect("block input should be written");
    assert_eq!(&encoded[..4], b"ethi");
    let parsed = parse_eth_block_input(&encoded).expect("block input should parse");
    assert_eq!(parsed.block_rlp, block_rlp);
    assert_eq!(parsed.block_number, 2);
    assert_eq!(parsed.timestamp, 101);
    assert_eq!(parsed.transactions.hash_preimages.len(), 1);
    assert_eq!(parsed.withdrawals, None);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\nblock_input={}\nbytes={}\nblock_hash={}\nblock_number=2\ntimestamp=101\ntransactions_root={}\ntransaction_trie_preimages=1\nwithdrawals=absent\n",
            output_path.display(),
            encoded.len(),
            to_hex(&parsed.block_hash),
            to_hex(&parsed.transactions_root)
        )
    );

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn writes_hex_block_input_artifact() {
    let dir = temp_dir("hex");
    let _ = fs::remove_dir_all(&dir);
    let block_path = dir.join("block.hex");
    let output_path = dir.join("block.input");
    let block_rlp = sample_block_rlp();
    write_bytes(&block_path, format!("0x{}\n", to_hex(&block_rlp)));

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "eth",
            "write-block-input",
            "--hex",
            block_path.to_str().expect("block path should be utf-8"),
            output_path.to_str().expect("output path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    let encoded = fs::read(&output_path).expect("block input should be written");
    let parsed = parse_eth_block_input(&encoded).expect("block input should parse");
    assert_eq!(parsed.block_rlp, block_rlp);
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .starts_with("status=ok\n"));

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

fn sample_block_rlp() -> Vec<u8> {
    let header_rlp = rlp_list(&legacy_header_items(
        hex32("e52f61e61ebdce920205cfca55e00c70bf219b45ea432febbf96152313e61db5"),
        None,
    ));
    let transactions = rlp_list(&[rlp_list(&[rlp_bytes(&[1])])]);
    let empty_list = rlp_list(&[]);
    rlp_list(&[header_rlp, transactions, empty_list])
}

fn legacy_header_items(
    transactions_root: [u8; 32],
    withdrawals_root: Option<[u8; 32]>,
) -> Vec<Vec<u8>> {
    let mut items = vec![
        rlp_bytes(&[0x11; 32]),
        rlp_bytes(&hex32(
            "1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
        )),
        rlp_bytes(&[0x33; 20]),
        rlp_bytes(&[0x44; 32]),
        rlp_bytes(&transactions_root),
        rlp_bytes(&[0x66; 32]),
        rlp_bytes(&[0x77; 256]),
        rlp_bytes(&[1]),
        rlp_bytes(&[2]),
        rlp_bytes(&[0x0f, 0x42, 0x40]),
        rlp_bytes(&[0x0d, 0xbb, 0xa0]),
        rlp_bytes(&[0x65]),
        rlp_bytes(b"lzvm"),
        rlp_bytes(&[0xaa; 32]),
        rlp_bytes(&[0xbb; 8]),
    ];
    if let Some(root) = withdrawals_root {
        items.push(rlp_bytes(&[1]));
        items.push(rlp_bytes(&root));
    }
    items
}

fn rlp_bytes(payload: &[u8]) -> Vec<u8> {
    if payload.len() == 1 && payload[0] <= 0x7f {
        return vec![payload[0]];
    }
    rlp_with_payload(0x80, 0xb7, payload)
}

fn rlp_list(items: &[Vec<u8>]) -> Vec<u8> {
    let payload = items.iter().flatten().copied().collect::<Vec<_>>();
    rlp_with_payload(0xc0, 0xf7, &payload)
}

fn rlp_with_payload(short_base: u8, long_base: u8, payload: &[u8]) -> Vec<u8> {
    if payload.len() <= 55 {
        let mut output = vec![short_base + payload.len() as u8];
        output.extend_from_slice(payload);
        return output;
    }

    let length = length_bytes(payload.len());
    let mut output = vec![long_base + length.len() as u8];
    output.extend_from_slice(&length);
    output.extend_from_slice(payload);
    output
}

fn length_bytes(mut value: usize) -> Vec<u8> {
    let mut bytes = Vec::new();
    while value > 0 {
        bytes.push((value & 0xff) as u8);
        value >>= 8;
    }
    bytes.reverse();
    bytes
}

fn hex32(value: &str) -> [u8; 32] {
    hex_bytes(value)
        .try_into()
        .expect("hex value should have 32 bytes")
}

fn hex_bytes(value: &str) -> Vec<u8> {
    assert!(value.len().is_multiple_of(2));
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| (hex_value(pair[0]) << 4) | hex_value(pair[1]))
        .collect()
}

fn hex_value(value: u8) -> u8 {
    match value {
        b'0'..=b'9' => value - b'0',
        b'a'..=b'f' => value - b'a' + 10,
        b'A'..=b'F' => value - b'A' + 10,
        _ => panic!("invalid hex digit"),
    }
}

fn to_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(hex_char(byte >> 4));
        output.push(hex_char(byte & 0x0f));
    }
    output
}

fn hex_char(value: u8) -> char {
    match value {
        0..=9 => char::from(b'0' + value),
        10..=15 => char::from(b'a' + value - 10),
        _ => unreachable!("hex nybble should be in range"),
    }
}
