use std::fs;
use std::path::{Path, PathBuf};

use lzvm_artifacts::eth_block::parse_eth_block_rlp;
use lzvm_artifacts::eth_block_input::{
    build_eth_block_input, build_eth_block_input_with_receipts, encode_eth_block_input,
    eth_block_input_bytes_digest, parse_eth_block_input,
};
use lzvm_artifacts::eth_public_input::{
    eth_public_header_hash, parse_eth_public_block_prefix, parse_eth_public_header_prefix,
    parse_eth_public_transactions_prefix,
};
use lzvm_artifacts::eth_trie::receipt_trie_build;
use lzvm_artifacts::rlp::parse_rlp;
use lzvm_cli::run_cli;

fn temp_dir(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root should resolve")
        .join(format!(
            "temp/lzvm-eth-public-input-cli-{}-{name}",
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
fn summarizes_public_input_header() {
    let dir = temp_dir("summary");
    let _ = fs::remove_dir_all(&dir);
    let input_path = dir.join("public.bin");
    let mut input = sample_public_header_bytes();
    input.extend_from_slice(&1_u64.to_le_bytes());
    input.extend_from_slice(&eip1559_transaction_bytes());
    input.extend_from_slice(&0_u64.to_le_bytes());
    input.push(0);
    input.extend_from_slice(b"tail");
    write_bytes(&input_path, &input);
    let parsed = parse_eth_public_header_prefix(&input).expect("header should parse");
    let transactions =
        parse_eth_public_transactions_prefix(&input).expect("transactions should parse");
    let block = parse_eth_public_block_prefix(&input).expect("block should parse");
    let block_hash = eth_public_header_hash(&parsed.header);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "eth",
            "public-input-summary",
            input_path.to_str().expect("input path should be utf-8"),
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
            "status=ok\npublic_input={}\nbytes={}\nheader_bytes={}\ntransaction_prefix_bytes={}\nblock_prefix_bytes={}\nremaining_bytes=4\nblock_hash={}\nblock_number=42\ntimestamp=77\ngas_limit=100\ngas_used=90\nommers_hash={}\ncomputed_ommers_hash={}\nommers_hash_matches=false\nommer_count=0\ntransactions_root={}\ncomputed_transactions_root={}\ntransactions_root_matches=false\ntransaction_count=1\nlegacy_transactions=0\ntyped_transactions=1\nreceipts_root={}\nwithdrawals_root=present:{}\ncomputed_withdrawals_root=absent\nwithdrawals_root_matches=false\nwithdrawal_count=absent\nbase_fee_per_gas=123\nblob_gas_used=456\nexcess_blob_gas=789\nparent_beacon_block_root=present:{}\nrequests_hash=present:{}\nextra_data=616263\n",
            input_path.display(),
            input.len(),
            parsed.consumed,
            transactions.consumed,
            block.consumed,
            to_hex(&block_hash),
            to_hex(&[2; 32]),
            to_hex(&block.ommers_hash()),
            to_hex(&[5; 32]),
            to_hex(&block.transactions_root()),
            to_hex(&[6; 32]),
            to_hex(&[7; 32]),
            to_hex(&[12; 32]),
            to_hex(&[13; 32])
        )
    );
}

#[test]
fn reports_invalid_public_input_header() {
    let dir = temp_dir("invalid");
    let _ = fs::remove_dir_all(&dir);
    let input_path = dir.join("public.bin");
    write_bytes(&input_path, [31_u8, 0, 0, 0, 0, 0, 0, 0]);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "eth",
            "public-input-summary",
            input_path.to_str().expect("input path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "eth public input summary failed: unexpected end of ETH public input at 8, needed 31, available 8\n"
    );
}

#[test]
fn writes_public_block_rlp() {
    let dir = temp_dir("write-block-rlp");
    let _ = fs::remove_dir_all(&dir);
    let input_path = dir.join("public.bin");
    let output_path = dir.join("block.rlp");
    let input = sample_public_block_bytes_with_matching_roots();
    write_bytes(&input_path, &input);
    let parsed = parse_eth_public_block_prefix(&input).expect("block should parse");
    let expected_block_rlp = parsed.block_rlp();

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "eth",
            "write-public-block-rlp",
            input_path.to_str().expect("input path should be utf-8"),
            output_path.to_str().expect("output path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    let written = fs::read(&output_path).expect("block RLP should be written");
    let block = parse_eth_block_rlp(&written).expect("block RLP should parse");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(written, expected_block_rlp);
    assert_eq!(block.transactions.len(), 1);
    assert_eq!(block.ommers.len(), 0);
    assert_eq!(block.withdrawals.as_ref().map(Vec::len), Some(1));
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\npublic_input={}\nbytes={}\noutput={}\n",
            input_path.display(),
            expected_block_rlp.len(),
            output_path.display()
        )
    );
}

#[test]
fn refuses_public_block_rlp_with_trailing_bytes() {
    let dir = temp_dir("write-block-rlp-trailing");
    let _ = fs::remove_dir_all(&dir);
    let input_path = dir.join("public.bin");
    let output_path = dir.join("block.rlp");
    let mut input = sample_public_block_bytes_with_matching_roots();
    input.extend_from_slice(b"tail");
    write_bytes(&input_path, &input);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "eth",
            "write-public-block-rlp",
            input_path.to_str().expect("input path should be utf-8"),
            output_path.to_str().expect("output path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    let output_exists = output_path.exists();
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert!(!output_exists);
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "eth public block rlp write failed: unexpected trailing bytes in ETH public input: 4\n"
    );
}

#[test]
fn writes_public_block_rlp_with_allowed_trailing_bytes() {
    let dir = temp_dir("write-block-rlp-allow-trailing");
    let _ = fs::remove_dir_all(&dir);
    let input_path = dir.join("public.bin");
    let output_path = dir.join("block.rlp");
    let mut input = sample_public_block_bytes_with_matching_roots();
    input.extend_from_slice(b"tail");
    write_bytes(&input_path, &input);
    let parsed = parse_eth_public_block_prefix(&input).expect("block should parse");
    let expected_block_rlp = parsed.block_rlp();

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "eth",
            "write-public-block-rlp",
            "--allow-trailing",
            input_path.to_str().expect("input path should be utf-8"),
            output_path.to_str().expect("output path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    let written = fs::read(&output_path).expect("block RLP should be written");
    let block = parse_eth_block_rlp(&written).expect("block RLP should parse");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(written, expected_block_rlp);
    assert_eq!(block.transactions.len(), 1);
    assert_eq!(block.withdrawals.as_ref().map(Vec::len), Some(1));
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\npublic_input={}\nremaining_bytes=4\nbytes={}\noutput={}\n",
            input_path.display(),
            expected_block_rlp.len(),
            output_path.display()
        )
    );
}

#[test]
fn refuses_public_block_rlp_with_root_mismatch() {
    let dir = temp_dir("write-block-rlp-mismatch");
    let _ = fs::remove_dir_all(&dir);
    let input_path = dir.join("public.bin");
    let output_path = dir.join("block.rlp");
    let mut input = sample_public_header_bytes();
    input.extend_from_slice(&1_u64.to_le_bytes());
    input.extend_from_slice(&eip1559_transaction_bytes());
    input.extend_from_slice(&0_u64.to_le_bytes());
    input.push(0);
    write_bytes(&input_path, &input);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "eth",
            "write-public-block-rlp",
            input_path.to_str().expect("input path should be utf-8"),
            output_path.to_str().expect("output path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    let output_exists = output_path.exists();
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert!(!output_exists);
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "eth public block rlp write failed: transactions_root mismatch\n"
    );
}

#[test]
fn writes_public_block_input() {
    let dir = temp_dir("write-block-input");
    let _ = fs::remove_dir_all(&dir);
    let input_path = dir.join("public.bin");
    let output_path = dir.join("block.input");
    let input = sample_public_block_bytes_with_matching_roots();
    write_bytes(&input_path, &input);
    let parsed_public = parse_eth_public_block_prefix(&input).expect("block should parse");
    let expected_block_rlp = parsed_public.block_rlp();
    let expected_input =
        build_eth_block_input(&expected_block_rlp).expect("block input should build");
    let expected_encoded =
        encode_eth_block_input(&expected_input).expect("block input should encode");
    let expected_hash = eth_block_input_bytes_digest(&expected_encoded);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "eth",
            "write-public-block-input",
            input_path.to_str().expect("input path should be utf-8"),
            output_path.to_str().expect("output path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    let written = fs::read(&output_path).expect("block input should be written");
    let parsed = parse_eth_block_input(&written).expect("block input should parse");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(written, expected_encoded);
    assert_eq!(parsed.block_rlp, expected_block_rlp);
    assert_eq!(parsed.transactions.hash_preimages.len(), 1);
    assert_eq!(
        parsed
            .withdrawals
            .as_ref()
            .map(|withdrawals| withdrawals.hash_preimages.len()),
        Some(1)
    );
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\npublic_input={}\nblock_input={}\nbytes={}\nblock_input_hash={}\nblock_hash={}\ntransaction_count=1\nwithdrawal_count=1\n",
            input_path.display(),
            output_path.display(),
            expected_encoded.len(),
            to_hex(&expected_hash),
            to_hex(&expected_input.block_hash)
        )
    );
}

#[test]
fn writes_public_block_input_with_rpc_json_receipts() {
    let dir = temp_dir("write-block-input-rpc-receipts");
    let _ = fs::remove_dir_all(&dir);
    let input_path = dir.join("public.bin");
    let receipts_path = dir.join("receipts.json");
    let output_path = dir.join("block.input");
    let receipt_item = legacy_receipt_item();
    let receipt = parse_rlp(&receipt_item).expect("receipt should parse");
    let receipt_root = receipt_trie_build(&[receipt]).root;
    let input = sample_public_block_bytes_with_matching_roots_and_receipts(receipt_root, [0; 256]);
    let receipts_rlp = rlp_list(&[receipt_item]);
    write_bytes(&input_path, &input);
    write_bytes(
        &receipts_path,
        format!(
            r#"{{
  "result": [
    {{
      "status": "0x1",
      "cumulativeGasUsed": "0x5a",
      "logsBloom": "0x{}",
      "logs": []
    }}
  ]
}}"#,
            "00".repeat(256),
        ),
    );
    let parsed_public = parse_eth_public_block_prefix(&input).expect("block should parse");
    let expected_block_rlp = parsed_public.block_rlp();
    let expected_input = build_eth_block_input_with_receipts(&expected_block_rlp, &receipts_rlp)
        .expect("block input should build");
    let expected_encoded =
        encode_eth_block_input(&expected_input).expect("block input should encode");
    let expected_hash = eth_block_input_bytes_digest(&expected_encoded);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "eth",
            "write-public-block-input",
            "--receipts-rpc-json",
            receipts_path
                .to_str()
                .expect("receipts path should be utf-8"),
            input_path.to_str().expect("input path should be utf-8"),
            output_path.to_str().expect("output path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    let written = fs::read(&output_path).expect("block input should be written");
    let parsed = parse_eth_block_input(&written).expect("block input should parse");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(written, expected_encoded);
    assert_eq!(
        parsed.receipts_rlp.as_deref(),
        Some(receipts_rlp.as_slice())
    );
    assert_eq!(
        parsed
            .receipts
            .as_ref()
            .map(|receipts| receipts.hash_preimages.len()),
        Some(1)
    );
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\npublic_input={}\nblock_input={}\nbytes={}\nblock_input_hash={}\nblock_hash={}\ntransaction_count=1\nreceipts=present\nreceipts_rlp_bytes={}\nreceipt_trie_preimages=1\nreceipt_count=1\nlegacy_receipts=1\ntyped_receipts=0\nwithdrawal_count=1\n",
            input_path.display(),
            output_path.display(),
            expected_encoded.len(),
            to_hex(&expected_hash),
            to_hex(&expected_input.block_hash),
            receipts_rlp.len(),
        )
    );
}

#[test]
fn writes_public_block_input_with_allowed_trailing_bytes_and_rpc_json_receipts() {
    let dir = temp_dir("write-block-input-allow-trailing-rpc-receipts");
    let _ = fs::remove_dir_all(&dir);
    let input_path = dir.join("public.bin");
    let receipts_path = dir.join("receipts.json");
    let output_path = dir.join("block.input");
    let receipt_item = legacy_receipt_item();
    let receipt = parse_rlp(&receipt_item).expect("receipt should parse");
    let receipt_root = receipt_trie_build(&[receipt]).root;
    let mut input =
        sample_public_block_bytes_with_matching_roots_and_receipts(receipt_root, [0; 256]);
    input.extend_from_slice(b"tail");
    let receipts_rlp = rlp_list(&[receipt_item]);
    write_bytes(&input_path, &input);
    write_bytes(
        &receipts_path,
        format!(
            r#"{{
  "result": [
    {{
      "status": "0x1",
      "cumulativeGasUsed": "0x5a",
      "logsBloom": "0x{}",
      "logs": []
    }}
  ]
}}"#,
            "00".repeat(256),
        ),
    );
    let parsed_public = parse_eth_public_block_prefix(&input).expect("block should parse");
    let expected_block_rlp = parsed_public.block_rlp();
    let expected_input = build_eth_block_input_with_receipts(&expected_block_rlp, &receipts_rlp)
        .expect("block input should build");
    let expected_encoded =
        encode_eth_block_input(&expected_input).expect("block input should encode");
    let expected_hash = eth_block_input_bytes_digest(&expected_encoded);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "eth",
            "write-public-block-input",
            "--allow-trailing",
            "--receipts-rpc-json",
            receipts_path
                .to_str()
                .expect("receipts path should be utf-8"),
            input_path.to_str().expect("input path should be utf-8"),
            output_path.to_str().expect("output path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    let written = fs::read(&output_path).expect("block input should be written");
    let parsed = parse_eth_block_input(&written).expect("block input should parse");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(written, expected_encoded);
    assert_eq!(
        parsed.receipts_rlp.as_deref(),
        Some(receipts_rlp.as_slice())
    );
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\npublic_input={}\nremaining_bytes=4\nblock_input={}\nbytes={}\nblock_input_hash={}\nblock_hash={}\ntransaction_count=1\nreceipts=present\nreceipts_rlp_bytes={}\nreceipt_trie_preimages=1\nreceipt_count=1\nlegacy_receipts=1\ntyped_receipts=0\nwithdrawal_count=1\n",
            input_path.display(),
            output_path.display(),
            expected_encoded.len(),
            to_hex(&expected_hash),
            to_hex(&expected_input.block_hash),
            receipts_rlp.len(),
        )
    );
}

#[test]
fn reports_usage_when_receipts_rpc_json_path_is_missing_before_allow_trailing() {
    let dir = temp_dir("write-block-input-missing-rpc-receipts-path");
    let _ = fs::remove_dir_all(&dir);
    let input_path = dir.join("public.bin");
    let output_path = dir.join("block.input");
    write_bytes(&input_path, sample_public_block_bytes_with_matching_roots());

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "eth",
            "write-public-block-input",
            "--receipts-rpc-json",
            "--allow-trailing",
            input_path.to_str().expect("input path should be utf-8"),
            output_path.to_str().expect("output path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    let output_exists = output_path.exists();
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert!(!output_exists);
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "usage: lzvm eth write-public-block-input [--allow-trailing] [--receipts-rpc-json <receipts-json>] <input> <out>\n"
    );
}

#[test]
fn refuses_public_block_input_with_trailing_bytes() {
    let dir = temp_dir("write-block-input-trailing");
    let _ = fs::remove_dir_all(&dir);
    let input_path = dir.join("public.bin");
    let output_path = dir.join("block.input");
    let mut input = sample_public_block_bytes_with_matching_roots();
    input.extend_from_slice(b"tail");
    write_bytes(&input_path, &input);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "eth",
            "write-public-block-input",
            input_path.to_str().expect("input path should be utf-8"),
            output_path.to_str().expect("output path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    let output_exists = output_path.exists();
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert!(!output_exists);
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "eth public block input failed: unexpected trailing bytes in ETH public input: 4\n"
    );
}

#[test]
fn writes_public_block_input_with_allowed_trailing_bytes() {
    let dir = temp_dir("write-block-input-allow-trailing");
    let _ = fs::remove_dir_all(&dir);
    let input_path = dir.join("public.bin");
    let output_path = dir.join("block.input");
    let mut input = sample_public_block_bytes_with_matching_roots();
    input.extend_from_slice(b"tail");
    write_bytes(&input_path, &input);
    let parsed_public = parse_eth_public_block_prefix(&input).expect("block should parse");
    let expected_block_rlp = parsed_public.block_rlp();
    let expected_input =
        build_eth_block_input(&expected_block_rlp).expect("block input should build");
    let expected_encoded =
        encode_eth_block_input(&expected_input).expect("block input should encode");
    let expected_hash = eth_block_input_bytes_digest(&expected_encoded);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "eth",
            "write-public-block-input",
            "--allow-trailing",
            input_path.to_str().expect("input path should be utf-8"),
            output_path.to_str().expect("output path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    let written = fs::read(&output_path).expect("block input should be written");
    let parsed = parse_eth_block_input(&written).expect("block input should parse");
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(written, expected_encoded);
    assert_eq!(parsed.block_rlp, expected_block_rlp);
    assert_eq!(parsed.transactions.hash_preimages.len(), 1);
    assert_eq!(
        parsed
            .withdrawals
            .as_ref()
            .map(|withdrawals| withdrawals.hash_preimages.len()),
        Some(1)
    );
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\npublic_input={}\nremaining_bytes=4\nblock_input={}\nbytes={}\nblock_input_hash={}\nblock_hash={}\ntransaction_count=1\nwithdrawal_count=1\n",
            input_path.display(),
            output_path.display(),
            expected_encoded.len(),
            to_hex(&expected_hash),
            to_hex(&expected_input.block_hash)
        )
    );
}

#[test]
fn refuses_public_block_input_with_root_mismatch() {
    let dir = temp_dir("write-block-input-mismatch");
    let _ = fs::remove_dir_all(&dir);
    let input_path = dir.join("public.bin");
    let output_path = dir.join("block.input");
    let mut input = sample_public_header_bytes();
    input.extend_from_slice(&1_u64.to_le_bytes());
    input.extend_from_slice(&eip1559_transaction_bytes());
    input.extend_from_slice(&0_u64.to_le_bytes());
    input.push(0);
    write_bytes(&input_path, &input);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "eth",
            "write-public-block-input",
            input_path.to_str().expect("input path should be utf-8"),
            output_path.to_str().expect("output path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    let output_exists = output_path.exists();
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert!(!output_exists);
    assert_eq!(
        String::from_utf8(stderr).expect("stderr should be utf-8"),
        "eth public block input failed: ETH block transactions root mismatch\n"
    );
}

fn sample_public_block_bytes_with_matching_roots() -> Vec<u8> {
    sample_public_block_bytes_with_matching_roots_and_receipts([6; 32], [8; 256])
}

fn sample_public_block_bytes_with_matching_roots_and_receipts(
    receipts_root: [u8; 32],
    logs_bloom: [u8; 256],
) -> Vec<u8> {
    let mut input = sample_public_header_bytes();
    input.extend_from_slice(&1_u64.to_le_bytes());
    input.extend_from_slice(&eip1559_transaction_bytes());
    input.extend_from_slice(&0_u64.to_le_bytes());
    input.push(1);
    input.extend_from_slice(&1_u64.to_le_bytes());
    input.extend_from_slice(&withdrawal_bytes());

    let parsed = parse_eth_public_block_prefix(&input).expect("block should parse");
    let transaction_root = parsed.transactions_root();
    let ommers_hash = parsed.ommers_hash();
    let withdrawal_root = parsed
        .withdrawals_root()
        .expect("withdrawals root should be present");
    input[48..80].copy_from_slice(&ommers_hash);
    input[156..188].copy_from_slice(&transaction_root);
    input[196..228].copy_from_slice(&receipts_root);
    input[237..269].copy_from_slice(&withdrawal_root);
    input[277..533].copy_from_slice(&logs_bloom);
    input
}

fn sample_public_header_bytes() -> Vec<u8> {
    let mut input = Vec::new();
    push_bytes(&mut input, &[1; 32]);
    push_bytes(&mut input, &[2; 32]);
    push_bytes(&mut input, &[3; 20]);
    push_bytes(&mut input, &[4; 32]);
    push_bytes(&mut input, &[5; 32]);
    push_bytes(&mut input, &[6; 32]);
    push_option_bytes(&mut input, Some(&[7; 32]));
    push_bytes(&mut input, &[8; 256]);
    push_bytes(&mut input, &u256_bytes(9));
    input.extend_from_slice(&42_u64.to_le_bytes());
    input.extend_from_slice(&100_u64.to_le_bytes());
    input.extend_from_slice(&90_u64.to_le_bytes());
    input.extend_from_slice(&77_u64.to_le_bytes());
    push_bytes(&mut input, &[10; 32]);
    push_bytes(&mut input, &[11; 8]);
    push_option_u64(&mut input, Some(123));
    push_option_u64(&mut input, Some(456));
    push_option_u64(&mut input, Some(789));
    push_option_bytes(&mut input, Some(&[12; 32]));
    push_option_bytes(&mut input, Some(&[13; 32]));
    push_bytes(&mut input, b"abc");
    input
}

fn u256_bytes(value: u8) -> [u8; 32] {
    let mut bytes = [0; 32];
    bytes[31] = value;
    bytes
}

fn push_bytes(out: &mut Vec<u8>, bytes: &[u8]) {
    out.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
    out.extend_from_slice(bytes);
}

fn push_option_bytes(out: &mut Vec<u8>, bytes: Option<&[u8]>) {
    match bytes {
        Some(bytes) => {
            out.push(1);
            push_bytes(out, bytes);
        }
        None => out.push(0),
    }
}

fn push_option_u64(out: &mut Vec<u8>, value: Option<u64>) {
    match value {
        Some(value) => {
            out.push(1);
            out.extend_from_slice(&value.to_le_bytes());
        }
        None => out.push(0),
    }
}

fn eip1559_transaction_bytes() -> Vec<u8> {
    let mut bytes = Vec::new();
    push_u256(&mut bytes, 0x11);
    push_u256(&mut bytes, 0x22);
    push_uint_u64(&mut bytes, 1);
    bytes.extend_from_slice(&2_u32.to_le_bytes());
    bytes.extend_from_slice(&1_u64.to_le_bytes());
    bytes.extend_from_slice(&7_u64.to_le_bytes());
    bytes.extend_from_slice(&21_000_u64.to_le_bytes());
    bytes.extend_from_slice(&300_u128.to_le_bytes());
    bytes.extend_from_slice(&20_u128.to_le_bytes());
    push_option_bytes(&mut bytes, Some(&[9; 20]));
    push_u256(&mut bytes, 123);
    bytes.extend_from_slice(&0_u64.to_le_bytes());
    push_bytes(&mut bytes, b"call-data");
    bytes
}

fn withdrawal_bytes() -> Vec<u8> {
    let mut bytes = Vec::new();
    push_uint_u64(&mut bytes, 7);
    push_uint_u64(&mut bytes, 8);
    push_bytes(&mut bytes, &[6; 20]);
    push_uint_u64(&mut bytes, 9);
    bytes
}

fn legacy_receipt_item() -> Vec<u8> {
    rlp_list(&[
        rlp_bytes(&[1]),
        rlp_bytes(&[0x5a]),
        rlp_bytes(&[0; 256]),
        rlp_list(&[]),
    ])
}

fn rlp_list(items: &[Vec<u8>]) -> Vec<u8> {
    let payload_len: usize = items.iter().map(Vec::len).sum();
    let mut out = Vec::new();
    push_rlp_header(&mut out, 0xc0, payload_len);
    for item in items {
        out.extend_from_slice(item);
    }
    out
}

fn rlp_bytes(payload: &[u8]) -> Vec<u8> {
    if payload.len() == 1 && payload[0] <= 0x7f {
        return vec![payload[0]];
    }
    let mut out = Vec::new();
    push_rlp_header(&mut out, 0x80, payload.len());
    out.extend_from_slice(payload);
    out
}

fn push_rlp_header(out: &mut Vec<u8>, offset: u8, len: usize) {
    if len <= 55 {
        out.push(offset + len as u8);
        return;
    }
    let bytes = len.to_be_bytes();
    let first = bytes
        .iter()
        .position(|byte| *byte != 0)
        .unwrap_or(bytes.len() - 1);
    let len_bytes = &bytes[first..];
    out.push(offset + 55 + len_bytes.len() as u8);
    out.extend_from_slice(len_bytes);
}

fn push_u256(out: &mut Vec<u8>, value: u8) {
    let mut bytes = [0; 32];
    bytes[31] = value;
    push_bytes(out, &bytes);
}

fn push_uint_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&8_u64.to_le_bytes());
    out.extend_from_slice(&value.to_be_bytes());
}

fn to_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}
