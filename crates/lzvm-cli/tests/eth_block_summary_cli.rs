use std::fs;
use std::path::{Path, PathBuf};

use lzvm_artifacts::eth_trie::{transaction_trie_root, withdrawals_trie_root};
use lzvm_artifacts::rlp::parse_rlp;
use lzvm_cli::run_cli;

fn temp_dir(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root should resolve")
        .join("temp")
        .join(format!(
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
            "status=ok\nbytes={}\nheader_fields=15\nblock_hash=fa44ca33c1ab75326fe997089b86b1414f53400af3a59b4d10c16e3f858bfb64\nblock_number=2\ntimestamp=101\ntransactions=1\ntransactions_root=5555555555555555555555555555555555555555555555555555555555555555\ncomputed_transactions_root=e52f61e61ebdce920205cfca55e00c70bf219b45ea432febbf96152313e61db5\ntransactions_root_matches=false\ntransaction_trie_preimages=1\nlegacy_transactions=1\ntyped_transactions=0\nommers=0\nommers_hash=2222222222222222222222222222222222222222222222222222222222222222\ncomputed_ommers_hash=1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347\nommers_hash_matches=false\nwithdrawals=absent\nextra_body_fields=0\nextra_header_fields=0\n",
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
            "status=ok\nbytes={}\nheader_fields=15\nblock_hash=fa44ca33c1ab75326fe997089b86b1414f53400af3a59b4d10c16e3f858bfb64\nblock_number=2\ntimestamp=101\ntransactions=1\ntransactions_root=5555555555555555555555555555555555555555555555555555555555555555\ncomputed_transactions_root=8907bc17c6a0854990b43d9949e6288d78df286967e71499f41874ffd3376e2e\ntransactions_root_matches=false\ntransaction_trie_preimages=1\nlegacy_transactions=0\ntyped_transactions=1\nommers=0\nommers_hash=2222222222222222222222222222222222222222222222222222222222222222\ncomputed_ommers_hash=1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347\nommers_hash_matches=false\nwithdrawals=absent\nextra_body_fields=0\nextra_header_fields=0\n",
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
fn summarizes_rpc_json_block() {
    let dir = temp_dir("rpc-json");
    let _ = fs::remove_dir_all(&dir);
    let block_path = dir.join("block.json");
    let transaction_item = type2_transaction_item();
    let transaction = parse_rlp(&transaction_item).expect("transaction should parse as RLP item");
    let transactions_root =
        transaction_trie_root(&[transaction]).expect("transaction trie root should be built");
    let withdrawals_root = withdrawals_trie_root(&[]);
    write_bytes(
        &block_path,
        format!(
            r#"{{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {{
    "parentHash": "0x1111111111111111111111111111111111111111111111111111111111111111",
    "sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
    "miner": "0x3333333333333333333333333333333333333333",
    "stateRoot": "0x4444444444444444444444444444444444444444444444444444444444444444",
    "transactionsRoot": "0x{transactions_root}",
    "receiptsRoot": "0x6666666666666666666666666666666666666666666666666666666666666666",
    "logsBloom": "0x{logs_bloom}",
    "difficulty": "0x0",
    "number": "0x2",
    "gasLimit": "0xf4240",
    "gasUsed": "0x5208",
    "timestamp": "0x65",
    "extraData": "0x6c7a766d",
    "mixHash": "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "nonce": "0xbbbbbbbbbbbbbbbb",
    "baseFeePerGas": "0x1",
    "withdrawalsRoot": "0x{withdrawals_root}",
    "blobGasUsed": "0x0",
    "excessBlobGas": "0x0",
    "parentBeaconBlockRoot": "0x9999999999999999999999999999999999999999999999999999999999999999",
    "transactions": [
      {{
        "type": "0x2",
        "chainId": "0x1",
        "nonce": "0x0",
        "maxPriorityFeePerGas": "0x2",
        "maxFeePerGas": "0x3",
        "gas": "0x5208",
        "to": "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "value": "0x0",
        "input": "0x",
        "accessList": [],
        "yParity": "0x0",
        "r": "0x1",
        "s": "0x2"
      }}
    ],
    "uncles": [],
    "withdrawals": []
  }}
}}"#,
            transactions_root = to_hex(&transactions_root),
            withdrawals_root = to_hex(&withdrawals_root),
            logs_bloom = "00".repeat(256),
        ),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "eth",
            "block-summary",
            "--rpc-json",
            block_path.to_str().expect("block path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    let output = String::from_utf8(stdout).expect("stdout should be utf-8");
    assert!(output.starts_with("status=ok\n"));
    assert!(output.contains("header_fields=20\n"));
    assert!(output.contains("block_number=2\n"));
    assert!(output.contains("timestamp=101\n"));
    assert!(output.contains("transactions=1\n"));
    assert!(output.contains("transactions_root_matches=true\n"));
    assert!(output.contains("legacy_transactions=0\n"));
    assert!(output.contains("typed_transactions=1\n"));
    assert!(output.contains("ommers=0\n"));
    assert!(output.contains("ommers_hash_matches=true\n"));
    assert!(output.contains("withdrawals=present\n"));
    assert!(output.contains("withdrawals_root_matches=true\n"));
    assert!(output.contains("withdrawals_trie_preimages=1\n"));
    assert!(output.contains("extra_body_fields=0\n"));
    assert!(output.contains("extra_header_fields=3\n"));
}

#[test]
fn summarizes_rpc_json_modern_transaction_envelopes() {
    let dir = temp_dir("rpc-json-modern-transactions");
    let _ = fs::remove_dir_all(&dir);
    let block_path = dir.join("block.json");
    let transaction_items = [
        legacy_transaction_item(),
        type1_transaction_item(),
        type3_transaction_item(),
        type4_transaction_item(),
    ];
    let transactions = transaction_items
        .iter()
        .map(|item| parse_rlp(item).expect("transaction should parse as RLP item"))
        .collect::<Vec<_>>();
    let transactions_root =
        transaction_trie_root(&transactions).expect("transaction trie root should be built");
    write_bytes(
        &block_path,
        rpc_json_block(
            &to_hex(&transactions_root),
            &format!(
                "{}, {}, {}, {}",
                rpc_legacy_transaction(),
                rpc_type1_transaction(),
                rpc_type3_transaction_with_blob_hash(
                    "\"0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\""
                ),
                rpc_type4_transaction_with_authorization(
                    "\"0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\""
                ),
            ),
            "",
            "",
        ),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "eth",
            "block-summary",
            "--rpc-json",
            block_path.to_str().expect("block path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    let output = String::from_utf8(stdout).expect("stdout should be utf-8");
    assert!(output.contains("transactions=4\n"));
    assert!(output.contains("transactions_root_matches=true\n"));
    assert!(output.contains("legacy_transactions=1\n"));
    assert!(output.contains("typed_transactions=3\n"));
}

#[test]
fn rejects_rpc_json_header_gap_before_parent_beacon_root() {
    let dir = temp_dir("rpc-json-header-gap");
    let _ = fs::remove_dir_all(&dir);
    let block_path = dir.join("block.json");
    let empty_root = hex32("56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421");
    write_bytes(
        &block_path,
        rpc_json_block(
            &to_hex(&empty_root),
            "",
            &format!(
                r#",
    "baseFeePerGas": "0x1",
    "withdrawalsRoot": "0x{}",
    "parentBeaconBlockRoot": "0x9999999999999999999999999999999999999999999999999999999999999999""#,
                to_hex(&empty_root)
            ),
            r#",
    "withdrawals": []"#,
        ),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "eth",
            "block-summary",
            "--rpc-json",
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
        "eth block summary failed: RPC block field parentBeaconBlockRoot requires blobGasUsed\n"
    );
}

#[test]
fn rejects_rpc_json_withdrawals_root_without_withdrawals_body() {
    let dir = temp_dir("rpc-json-missing-withdrawals-body");
    let _ = fs::remove_dir_all(&dir);
    let block_path = dir.join("block.json");
    let empty_root = hex32("56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421");
    write_bytes(
        &block_path,
        rpc_json_block(
            &to_hex(&empty_root),
            "",
            &format!(
                r#",
    "baseFeePerGas": "0x1",
    "withdrawalsRoot": "0x{}""#,
                to_hex(&empty_root)
            ),
            "",
        ),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "eth",
            "block-summary",
            "--rpc-json",
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
        "eth block summary failed: RPC block has withdrawalsRoot without withdrawals\n"
    );
}

#[test]
fn rejects_rpc_json_non_empty_ommer_hashes() {
    let dir = temp_dir("rpc-json-ommer-hashes");
    let _ = fs::remove_dir_all(&dir);
    let block_path = dir.join("block.json");
    let empty_root = hex32("56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421");
    write_bytes(
        &block_path,
        rpc_json_block(&to_hex(&empty_root), "", "", "").replace(
            r#""uncles": []"#,
            r#""uncles": ["0x1111111111111111111111111111111111111111111111111111111111111111"]"#,
        ),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "eth",
            "block-summary",
            "--rpc-json",
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
        "eth block summary failed: RPC block uncles must be empty because RPC ommer hashes do not contain ommer headers\n"
    );
}

#[test]
fn rejects_rpc_json_blob_transaction_without_destination() {
    assert_rpc_json_transaction_rejected(
        "blob-null-to",
        &rpc_type3_transaction("null"),
        "eth block summary failed: RPC transaction type 0x3 field to must not be null\n",
    );
}

#[test]
fn rejects_rpc_json_authorized_transaction_without_destination() {
    assert_rpc_json_transaction_rejected(
        "authorized-null-to",
        &rpc_type4_transaction("null"),
        "eth block summary failed: RPC transaction type 0x4 field to must not be null\n",
    );
}

#[test]
fn rejects_rpc_json_conflicting_alias_fields() {
    assert_rpc_json_transaction_rejected(
        "alias-conflict",
        r#"{
        "type": "0x2",
        "chainId": "0x1",
        "nonce": "0x0",
        "maxPriorityFeePerGas": "0x2",
        "maxFeePerGas": "0x3",
        "gas": "0x5208",
        "to": "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "value": "0x0",
        "input": "0x",
        "data": "0x01",
        "accessList": [],
        "yParity": "0x0",
        "r": "0x1",
        "s": "0x2"
      }"#,
        "eth block summary failed: conflicting RPC fields: input and data\n",
    );
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
    assert!(output.contains("transaction_trie_preimages=1\n"));
    assert!(output.contains("ommers_hash_matches=true\n"));
}

#[test]
fn summarizes_withdrawals_root_check() {
    let dir = temp_dir("withdrawals");
    let _ = fs::remove_dir_all(&dir);
    let block_path = dir.join("block.rlp");
    let withdrawals_root =
        hex32("51c445cba96d0dfd446eec8b2b94f104608cf8443a92f7f87c76a383d6687300");
    let block_rlp =
        sample_block_rlp_with_withdrawals(vec![withdrawal_item()], withdrawals_root.to_vec());
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
    assert!(output.contains("withdrawals=present\n"));
    assert!(output.contains(
        "withdrawals_root=51c445cba96d0dfd446eec8b2b94f104608cf8443a92f7f87c76a383d6687300\n"
    ));
    assert!(output.contains(
        "computed_withdrawals_root=51c445cba96d0dfd446eec8b2b94f104608cf8443a92f7f87c76a383d6687300\n"
    ));
    assert!(output.contains("withdrawals_root_matches=true\n"));
    assert!(output.contains("withdrawals_trie_preimages=1\n"));
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
        "usage: lzvm eth block-summary [--hex|--rpc-json] <block-rlp>\n"
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

#[test]
fn rejects_invalid_withdrawals() {
    let dir = temp_dir("invalid-withdrawal");
    let _ = fs::remove_dir_all(&dir);
    let block_path = dir.join("block.rlp");
    let malformed_withdrawal = rlp_list(&[rlp_bytes(&[]), rlp_bytes(&[]), rlp_bytes(&[])]);
    let block_rlp = sample_block_rlp_with_withdrawals(vec![malformed_withdrawal], vec![0x55; 32]);
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
        "eth block summary failed: expected 4 withdrawal fields, found 3\n"
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

fn sample_block_rlp_with_withdrawals(
    withdrawal_items: Vec<Vec<u8>>,
    withdrawals_root: Vec<u8>,
) -> Vec<u8> {
    let mut header = legacy_header_items();
    header.push(rlp_bytes(&[1]));
    header.push(rlp_bytes(&withdrawals_root));
    let header_rlp = rlp_list(&header);
    let empty_list = rlp_list(&[]);
    let withdrawals = rlp_list(&withdrawal_items);
    rlp_list(&[header_rlp, empty_list.clone(), empty_list, withdrawals])
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

fn withdrawal_item() -> Vec<u8> {
    rlp_list(&[
        rlp_bytes(&[]),
        rlp_bytes(&[1]),
        rlp_bytes(&[0x22; 20]),
        rlp_bytes(&[0x40]),
    ])
}

fn type2_transaction_item() -> Vec<u8> {
    typed_transaction_item(
        2,
        &[
            rlp_bytes(&[1]),
            rlp_bytes(&[]),
            rlp_bytes(&[2]),
            rlp_bytes(&[3]),
            rlp_bytes(&[0x52, 0x08]),
            rlp_bytes(&[0xaa; 20]),
            rlp_bytes(&[]),
            rlp_bytes(&[]),
            rlp_list(&[]),
            rlp_bytes(&[]),
            rlp_bytes(&[1]),
            rlp_bytes(&[2]),
        ],
    )
}

fn legacy_transaction_item() -> Vec<u8> {
    rlp_list(&[
        rlp_bytes(&[]),
        rlp_bytes(&[1]),
        rlp_bytes(&[0x52, 0x08]),
        rlp_bytes(&[0xaa; 20]),
        rlp_bytes(&[]),
        rlp_bytes(&[]),
        rlp_bytes(&[0x1b]),
        rlp_bytes(&[1]),
        rlp_bytes(&[2]),
    ])
}

fn type1_transaction_item() -> Vec<u8> {
    typed_transaction_item(
        1,
        &[
            rlp_bytes(&[1]),
            rlp_bytes(&[]),
            rlp_bytes(&[1]),
            rlp_bytes(&[0x52, 0x08]),
            rlp_bytes(&[0xaa; 20]),
            rlp_bytes(&[]),
            rlp_bytes(&[]),
            rlp_list(&[]),
            rlp_bytes(&[]),
            rlp_bytes(&[1]),
            rlp_bytes(&[2]),
        ],
    )
}

fn type3_transaction_item() -> Vec<u8> {
    typed_transaction_item(
        3,
        &[
            rlp_bytes(&[1]),
            rlp_bytes(&[]),
            rlp_bytes(&[2]),
            rlp_bytes(&[3]),
            rlp_bytes(&[0x52, 0x08]),
            rlp_bytes(&[0xaa; 20]),
            rlp_bytes(&[]),
            rlp_bytes(&[]),
            rlp_list(&[]),
            rlp_bytes(&[4]),
            rlp_list(&[rlp_bytes(&[1; 32])]),
            rlp_bytes(&[]),
            rlp_bytes(&[1]),
            rlp_bytes(&[2]),
        ],
    )
}

fn type4_transaction_item() -> Vec<u8> {
    typed_transaction_item(
        4,
        &[
            rlp_bytes(&[1]),
            rlp_bytes(&[]),
            rlp_bytes(&[2]),
            rlp_bytes(&[3]),
            rlp_bytes(&[0x52, 0x08]),
            rlp_bytes(&[0xaa; 20]),
            rlp_bytes(&[]),
            rlp_bytes(&[]),
            rlp_list(&[]),
            rlp_list(&[authorization_item()]),
            rlp_bytes(&[]),
            rlp_bytes(&[1]),
            rlp_bytes(&[2]),
        ],
    )
}

fn typed_transaction_item(transaction_type: u8, fields: &[Vec<u8>]) -> Vec<u8> {
    let payload = rlp_list(fields);
    let mut envelope = vec![transaction_type];
    envelope.extend_from_slice(&payload);
    rlp_bytes(&envelope)
}

fn authorization_item() -> Vec<u8> {
    rlp_list(&[
        rlp_bytes(&[1]),
        rlp_bytes(&[0xbb; 20]),
        rlp_bytes(&[]),
        rlp_bytes(&[]),
        rlp_bytes(&[1]),
        rlp_bytes(&[2]),
    ])
}

fn assert_rpc_json_transaction_rejected(name: &str, transaction_json: &str, expected: &str) {
    let dir = temp_dir(name);
    let _ = fs::remove_dir_all(&dir);
    let block_path = dir.join("block.json");
    let empty_root = hex32("56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421");
    write_bytes(
        &block_path,
        rpc_json_block(&to_hex(&empty_root), transaction_json, "", ""),
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "eth",
            "block-summary",
            "--rpc-json",
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
        expected
    );
}

fn rpc_json_block(
    transactions_root: &str,
    transactions_json: &str,
    extra_header_fields: &str,
    extra_body_fields: &str,
) -> String {
    format!(
        r#"{{
  "result": {{
    "parentHash": "0x1111111111111111111111111111111111111111111111111111111111111111",
    "sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
    "miner": "0x3333333333333333333333333333333333333333",
    "stateRoot": "0x4444444444444444444444444444444444444444444444444444444444444444",
    "transactionsRoot": "0x{transactions_root}",
    "receiptsRoot": "0x6666666666666666666666666666666666666666666666666666666666666666",
    "logsBloom": "0x{logs_bloom}",
    "difficulty": "0x0",
    "number": "0x2",
    "gasLimit": "0xf4240",
    "gasUsed": "0x5208",
    "timestamp": "0x65",
    "extraData": "0x6c7a766d",
    "mixHash": "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "nonce": "0xbbbbbbbbbbbbbbbb"{extra_header_fields},
    "transactions": [{transactions_json}],
    "uncles": []{extra_body_fields}
  }}
}}"#,
        logs_bloom = "00".repeat(256),
    )
}

fn rpc_legacy_transaction() -> &'static str {
    r#"{
        "nonce": "0x0",
        "gasPrice": "0x1",
        "gas": "0x5208",
        "to": "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "value": "0x0",
        "input": "0x",
        "v": "0x1b",
        "r": "0x1",
        "s": "0x2"
      }"#
}

fn rpc_type1_transaction() -> &'static str {
    r#"{
        "type": "0x1",
        "chainId": "0x1",
        "nonce": "0x0",
        "gasPrice": "0x1",
        "gas": "0x5208",
        "to": "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "value": "0x0",
        "input": "0x",
        "accessList": [],
        "yParity": "0x0",
        "r": "0x1",
        "s": "0x2"
      }"#
}

fn rpc_type3_transaction(destination: &str) -> String {
    format!(
        r#"{{
        "type": "0x3",
        "chainId": "0x1",
        "nonce": "0x0",
        "maxPriorityFeePerGas": "0x2",
        "maxFeePerGas": "0x3",
        "gas": "0x5208",
        "to": {destination},
        "value": "0x0",
        "input": "0x",
        "accessList": [],
        "maxFeePerBlobGas": "0x4",
        "blobVersionedHashes": [],
        "yParity": "0x0",
        "r": "0x1",
        "s": "0x2"
      }}"#
    )
}

fn rpc_type3_transaction_with_blob_hash(destination: &str) -> String {
    format!(
        r#"{{
        "type": "0x3",
        "chainId": "0x1",
        "nonce": "0x0",
        "maxPriorityFeePerGas": "0x2",
        "maxFeePerGas": "0x3",
        "gas": "0x5208",
        "to": {destination},
        "value": "0x0",
        "input": "0x",
        "accessList": [],
        "maxFeePerBlobGas": "0x4",
        "blobVersionedHashes": ["0x0101010101010101010101010101010101010101010101010101010101010101"],
        "yParity": "0x0",
        "r": "0x1",
        "s": "0x2"
      }}"#
    )
}

fn rpc_type4_transaction(destination: &str) -> String {
    format!(
        r#"{{
        "type": "0x4",
        "chainId": "0x1",
        "nonce": "0x0",
        "maxPriorityFeePerGas": "0x2",
        "maxFeePerGas": "0x3",
        "gas": "0x5208",
        "to": {destination},
        "value": "0x0",
        "input": "0x",
        "accessList": [],
        "authorizationList": [],
        "yParity": "0x0",
        "r": "0x1",
        "s": "0x2"
      }}"#
    )
}

fn rpc_type4_transaction_with_authorization(destination: &str) -> String {
    format!(
        r#"{{
        "type": "0x4",
        "chainId": "0x1",
        "nonce": "0x0",
        "maxPriorityFeePerGas": "0x2",
        "maxFeePerGas": "0x3",
        "gas": "0x5208",
        "to": {destination},
        "value": "0x0",
        "input": "0x",
        "accessList": [],
        "authorizationList": [
          {{
            "chainId": "0x1",
            "address": "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "nonce": "0x0",
            "yParity": "0x0",
            "r": "0x1",
            "s": "0x2"
          }}
        ],
        "yParity": "0x0",
        "r": "0x1",
        "s": "0x2"
      }}"#
    )
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
