use std::fs;
use std::path::{Path, PathBuf};

use lzvm_cli::run_cli;

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "lzvm-eth-block-summary-cli-{}-{name}",
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
fn summarizes_binary_block_rlp() {
    let dir = temp_dir("binary");
    let _ = fs::remove_dir_all(&dir);
    let block_path = dir.join("block.rlp");
    let block_rlp = sample_block_rlp();
    write_bytes(&block_path, &block_rlp);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "eth",
            "block-summary",
            block_path.to_str().expect("block path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\nbytes={}\nheader_fields=15\nblock_hash=fa44ca33c1ab75326fe997089b86b1414f53400af3a59b4d10c16e3f858bfb64\nblock_number=2\ntimestamp=101\ntransactions=1\ntransactions_root=5555555555555555555555555555555555555555555555555555555555555555\ncomputed_transactions_root=e52f61e61ebdce920205cfca55e00c70bf219b45ea432febbf96152313e61db5\ntransactions_root_matches=false\nlegacy_transactions=1\ntyped_transactions=0\nommers=0\nwithdrawals=absent\nextra_body_fields=0\nextra_header_fields=0\n",
            block_rlp.len()
        )
    );
}

#[test]
fn summarizes_typed_transaction_counts() {
    let dir = temp_dir("typed");
    let _ = fs::remove_dir_all(&dir);
    let block_path = dir.join("block.rlp");
    let block_rlp = sample_block_rlp_with_transactions(vec![rlp_bytes(&[2, 0xc0])]);
    write_bytes(&block_path, &block_rlp);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "eth",
            "block-summary",
            block_path.to_str().expect("block path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\nbytes={}\nheader_fields=15\nblock_hash=fa44ca33c1ab75326fe997089b86b1414f53400af3a59b4d10c16e3f858bfb64\nblock_number=2\ntimestamp=101\ntransactions=1\ntransactions_root=5555555555555555555555555555555555555555555555555555555555555555\ncomputed_transactions_root=8907bc17c6a0854990b43d9949e6288d78df286967e71499f41874ffd3376e2e\ntransactions_root_matches=false\nlegacy_transactions=0\ntyped_transactions=1\nommers=0\nwithdrawals=absent\nextra_body_fields=0\nextra_header_fields=0\n",
            block_rlp.len()
        )
    );
}

#[test]
fn summarizes_hex_block_rlp() {
    let dir = temp_dir("hex");
    let _ = fs::remove_dir_all(&dir);
    let block_path = dir.join("block.hex");
    let block_rlp = sample_block_rlp();
    write_bytes(&block_path, format!("0x{}\n", to_hex(&block_rlp)));

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "eth",
            "block-summary",
            "--hex",
            block_path.to_str().expect("block path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    assert!(String::from_utf8(stdout)
        .expect("stdout should be utf-8")
        .starts_with("status=ok\n"));
}

#[test]
fn summarizes_mainnet_genesis_block_hash() {
    let dir = temp_dir("genesis");
    let _ = fs::remove_dir_all(&dir);
    let block_path = dir.join("block.rlp");
    let block_rlp = sample_mainnet_genesis_block_rlp();
    write_bytes(&block_path, &block_rlp);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "eth",
            "block-summary",
            block_path.to_str().expect("block path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    let output = String::from_utf8(stdout).expect("stdout should be utf-8");
    assert!(output
        .contains("block_hash=d4e56740f876aef8c010b86a40d5f56745a118d0906a34e69aec8c0db1cb8fa3\n"));
    assert!(output.contains("transactions_root_matches=true\n"));
}

#[test]
fn reports_usage_for_missing_block_summary_input() {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(&["eth", "block-summary"], &mut stdout, &mut stderr);

    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "usage: lzvm eth block-summary [--hex] <block-rlp>\n"
    );
}

#[test]
fn rejects_invalid_hex_block_summary_input() {
    let dir = temp_dir("invalid-hex");
    let _ = fs::remove_dir_all(&dir);
    let block_path = dir.join("block.hex");
    write_bytes(&block_path, b"0xz1");

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "eth",
            "block-summary",
            "--hex",
            block_path.to_str().expect("block path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "eth block summary failed: invalid hex digit at offset 2: z\n"
    );
}

#[test]
fn rejects_invalid_transaction_envelopes() {
    let dir = temp_dir("invalid-transaction");
    let _ = fs::remove_dir_all(&dir);
    let block_path = dir.join("block.rlp");
    let block_rlp = sample_block_rlp_with_transactions(vec![rlp_bytes(&[0x80])]);
    write_bytes(&block_path, &block_rlp);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "eth",
            "block-summary",
            block_path.to_str().expect("block path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "eth block summary failed: invalid transaction type byte: 0x80\n"
    );
}

fn sample_block_rlp() -> Vec<u8> {
    sample_block_rlp_with_transactions(vec![rlp_list(&[rlp_bytes(&[0x01])])])
}

fn sample_block_rlp_with_transactions(transaction_items: Vec<Vec<u8>>) -> Vec<u8> {
    let header_rlp = rlp_list(&legacy_header_items());
    let transactions = rlp_list(&transaction_items);
    let empty_list = rlp_list(&[]);
    rlp_list(&[header_rlp, transactions, empty_list])
}

fn sample_mainnet_genesis_block_rlp() -> Vec<u8> {
    let header_rlp = rlp_list(&mainnet_genesis_header_items());
    let empty_list = rlp_list(&[]);
    rlp_list(&[header_rlp, empty_list.clone(), empty_list])
}

fn legacy_header_items() -> Vec<Vec<u8>> {
    vec![
        rlp_bytes(&[0x11; 32]),
        rlp_bytes(&[0x22; 32]),
        rlp_bytes(&[0x33; 20]),
        rlp_bytes(&[0x44; 32]),
        rlp_bytes(&[0x55; 32]),
        rlp_bytes(&[0x66; 32]),
        rlp_bytes(&[0x77; 256]),
        rlp_bytes(&[0x01]),
        rlp_bytes(&[0x02]),
        rlp_bytes(&[0x0f, 0x42, 0x40]),
        rlp_bytes(&[0x0d, 0xbb, 0xa0]),
        rlp_bytes(&[0x65]),
        rlp_bytes(b"lzvm"),
        rlp_bytes(&[0xaa; 32]),
        rlp_bytes(&[0xbb; 8]),
    ]
}

fn mainnet_genesis_header_items() -> Vec<Vec<u8>> {
    vec![
        rlp_bytes(&[0; 32]),
        rlp_bytes(&hex32(
            "1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
        )),
        rlp_bytes(&[0; 20]),
        rlp_bytes(&hex32(
            "d7f8974fb5ac78d9ac099b9ad5018bedc2ce0a72dad1827a1709da30580f0544",
        )),
        rlp_bytes(&hex32(
            "56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421",
        )),
        rlp_bytes(&hex32(
            "56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421",
        )),
        rlp_bytes(&[0; 256]),
        rlp_bytes(&[0x04, 0x00, 0x00, 0x00, 0x00]),
        rlp_bytes(&[]),
        rlp_bytes(&[0x13, 0x88]),
        rlp_bytes(&[]),
        rlp_bytes(&[]),
        rlp_bytes(&hex_bytes(
            "11bbe8db4e347b4e8c937c1c8370e4b5ed33adb3db69cbdb7a38e1e50b1b82fa",
        )),
        rlp_bytes(&[0; 32]),
        rlp_bytes(&[0, 0, 0, 0, 0, 0, 0, 0x42]),
    ]
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
