use std::fs;
use std::path::{Path, PathBuf};

use lzvm_artifacts::eth_block_input::{
    build_eth_block_input, encode_eth_block_input, eth_block_input_bytes_digest,
    parse_eth_block_input,
};
use lzvm_artifacts::eth_trie::{receipt_trie_build, transaction_trie_build};
use lzvm_artifacts::public_values::{parse_public_values, public_values_digest};
use lzvm_artifacts::rlp::parse_rlp;
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
    let input_hash = eth_block_input_bytes_digest(&encoded);
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
            "status=ok\nblock_input={}\nbytes={}\nblock_input_hash={}\nblock_hash={}\nparent_hash={}\nbeneficiary={}\nstate_root={}\nreceipts_root={}\ndifficulty=01\nblock_number=2\ntimestamp=101\nextra_data=6c7a766d\ngas_limit=1000000\ngas_used=900000\nbase_fee_per_gas=absent\nmix_hash={}\nnonce={}\ntransactions_root={}\ntransaction_trie_preimages=1\nlegacy_transactions=1\ntyped_transactions=0\nwithdrawals=absent\n",
            output_path.display(),
            encoded.len(),
            to_hex(&input_hash),
            to_hex(&parsed.block_hash),
            to_hex(&parsed.parent_hash),
            to_hex(&parsed.beneficiary),
            to_hex(&parsed.state_root),
            to_hex(&parsed.receipts_root),
            to_hex(&parsed.mix_hash),
            to_hex(&parsed.nonce),
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

#[test]
fn writes_binary_block_input_artifact_with_receipts() {
    let dir = temp_dir("binary-receipts");
    let _ = fs::remove_dir_all(&dir);
    let block_path = dir.join("block.rlp");
    let receipts_path = dir.join("receipts.rlp");
    let output_path = dir.join("block.input");
    let receipt_item = sample_receipt_item();
    let receipts = vec![parse_rlp(&receipt_item).expect("receipt should parse")];
    let receipt_build = receipt_trie_build(&receipts);
    let receipts_rlp = rlp_list(&[receipt_item]);
    let block_rlp = sample_block_rlp_with_receipts_root(receipt_build.root);
    write_bytes(&block_path, &block_rlp);
    write_bytes(&receipts_path, &receipts_rlp);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "eth",
            "write-block-input",
            "--receipts",
            receipts_path
                .to_str()
                .expect("receipts path should be utf-8"),
            block_path.to_str().expect("block path should be utf-8"),
            output_path.to_str().expect("output path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    let encoded = fs::read(&output_path).expect("block input should be written");
    let input_hash = eth_block_input_bytes_digest(&encoded);
    let parsed = parse_eth_block_input(&encoded).expect("block input should parse");
    let parsed_receipts = parsed.receipts.as_ref().expect("receipts should exist");
    assert_eq!(parsed.block_rlp, block_rlp);
    assert_eq!(parsed.receipts_root, receipt_build.root);
    assert_eq!(parsed_receipts.hash_preimages, receipt_build.hash_preimages);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\nblock_input={}\nbytes={}\nblock_input_hash={}\nblock_hash={}\nparent_hash={}\nbeneficiary={}\nstate_root={}\nreceipts_root={}\ndifficulty=01\nblock_number=2\ntimestamp=101\nextra_data=6c7a766d\ngas_limit=1000000\ngas_used=21000\nbase_fee_per_gas=absent\nmix_hash={}\nnonce={}\ntransactions_root={}\ntransaction_trie_preimages=1\nlegacy_transactions=1\ntyped_transactions=0\nreceipts=present\nreceipt_trie_preimages={}\nlegacy_receipts=1\ntyped_receipts=0\nwithdrawals=absent\n",
            output_path.display(),
            encoded.len(),
            to_hex(&input_hash),
            to_hex(&parsed.block_hash),
            to_hex(&parsed.parent_hash),
            to_hex(&parsed.beneficiary),
            to_hex(&parsed.state_root),
            to_hex(&parsed.receipts_root),
            to_hex(&parsed.mix_hash),
            to_hex(&parsed.nonce),
            to_hex(&parsed.transactions_root),
            parsed_receipts.hash_preimages.len()
        )
    );

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn writes_receipt_kind_counts_for_typed_block_input_artifacts() {
    let dir = temp_dir("typed-receipts");
    let _ = fs::remove_dir_all(&dir);
    let block_path = dir.join("block.rlp");
    let receipts_path = dir.join("receipts.rlp");
    let output_path = dir.join("block.input");
    let receipt_item = typed_receipt_item(2);
    let receipts = vec![parse_rlp(&receipt_item).expect("receipt should parse")];
    let receipt_build = receipt_trie_build(&receipts);
    let receipts_rlp = rlp_list(&[receipt_item]);
    let block_rlp = sample_block_rlp_with_receipts_root(receipt_build.root);
    write_bytes(&block_path, &block_rlp);
    write_bytes(&receipts_path, &receipts_rlp);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "eth",
            "write-block-input",
            "--receipts",
            receipts_path
                .to_str()
                .expect("receipts path should be utf-8"),
            block_path.to_str().expect("block path should be utf-8"),
            output_path.to_str().expect("output path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    let stdout_text = String::from_utf8(stdout).expect("stdout should be utf-8");
    assert!(stdout_text.contains("legacy_receipts=0\ntyped_receipts=1\n"));
}

#[test]
fn writes_transaction_kind_counts_for_typed_block_input_artifacts() {
    let dir = temp_dir("typed");
    let _ = fs::remove_dir_all(&dir);
    let block_path = dir.join("block.rlp");
    let output_path = dir.join("block.input");
    let transaction_item = typed_transaction_item();
    let transactions = vec![parse_rlp(&transaction_item).expect("transaction should parse")];
    let transaction_build =
        transaction_trie_build(&transactions).expect("transaction trie should build");
    let block_rlp =
        sample_block_rlp_with_transaction_items(transaction_build.root, vec![transaction_item]);
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
    fs::remove_dir_all(&dir).expect("fixture directory should be removed");

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    let stdout_text = String::from_utf8(stdout).expect("stdout should be utf-8");
    assert!(stdout_text.contains("legacy_transactions=0\ntyped_transactions=1\n"));
}

#[test]
fn summarizes_block_input_artifacts() {
    let dir = temp_dir("summary");
    let _ = fs::remove_dir_all(&dir);
    let input_path = dir.join("block.input");
    let block_rlp = sample_block_rlp();
    let input = build_eth_block_input(&block_rlp).expect("block input should build");
    let encoded = encode_eth_block_input(&input).expect("block input should encode");
    let input_hash = eth_block_input_bytes_digest(&encoded);
    write_bytes(&input_path, &encoded);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "eth",
            "block-input-summary",
            input_path.to_str().expect("input path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    let stdout_text = String::from_utf8(stdout).expect("stdout should be utf-8");
    assert_eq!(
        stdout_text,
        format!(
            "status=ok\nblock_input={}\nbytes={}\nblock_input_hash={}\nblock_rlp_bytes={}\nblock_hash={}\nparent_hash={}\nbeneficiary={}\nstate_root={}\nreceipts_root={}\ndifficulty=01\nblock_number=2\ntimestamp=101\nextra_data=6c7a766d\ngas_limit=1000000\ngas_used=900000\nbase_fee_per_gas=absent\nmix_hash={}\nnonce={}\ntransactions_root={}\ntransaction_trie_preimages=1\nlegacy_transactions=1\ntyped_transactions=0\nwithdrawals=absent\n",
            input_path.display(),
            encoded.len(),
            to_hex(&input_hash),
            block_rlp.len(),
            to_hex(&input.block_hash),
            to_hex(&input.parent_hash),
            to_hex(&input.beneficiary),
            to_hex(&input.state_root),
            to_hex(&input.receipts_root),
            to_hex(&input.mix_hash),
            to_hex(&input.nonce),
            to_hex(&input.transactions_root)
        )
    );

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn writes_block_public_values_from_block_input() {
    let dir = temp_dir("public-values");
    let _ = fs::remove_dir_all(&dir);
    let input_path = dir.join("block.input");
    let output_path = dir.join("public-values.bin");
    let setup_hash = [0x44_u8; 32];
    let setup_hash_hex = to_hex(&setup_hash);
    let block_rlp = sample_block_rlp();
    let input = build_eth_block_input(&block_rlp).expect("block input should build");
    let encoded_input = encode_eth_block_input(&input).expect("block input should encode");
    write_bytes(&input_path, &encoded_input);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "eth",
            "write-block-public-values",
            "--setup-hash",
            &setup_hash_hex,
            input_path.to_str().expect("input path should be utf-8"),
            output_path.to_str().expect("output path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    let encoded = fs::read(&output_path).expect("public values should be written");
    let parsed = parse_public_values(&encoded).expect("public values should parse");
    let public_values_hash =
        public_values_digest(&parsed).expect("public values digest should compute");
    assert_eq!(parsed.setup_hash, setup_hash);
    assert_eq!(parsed.schema_version, 1);
    assert_eq!(parsed.values.len(), 21);
    assert_eq!(parsed.values[0].name, "eth_block_hash_u32_be");
    assert_eq!(parsed.values[0].elements, hash_u32_be(&input.block_hash));
    assert_eq!(parsed.values[1].name, "eth_parent_hash_u32_be");
    assert_eq!(parsed.values[1].elements, hash_u32_be(&input.parent_hash));
    assert_eq!(parsed.values[2].name, "eth_beneficiary_u32_be");
    assert_eq!(parsed.values[2].elements, bytes_u32_be(&input.beneficiary));
    assert_eq!(parsed.values[3].name, "eth_state_root_u32_be");
    assert_eq!(parsed.values[3].elements, hash_u32_be(&input.state_root));
    assert_eq!(parsed.values[4].name, "eth_receipts_root_u32_be");
    assert_eq!(parsed.values[4].elements, hash_u32_be(&input.receipts_root));
    assert_eq!(parsed.values[5].name, "eth_logs_bloom_u32_be");
    assert_eq!(parsed.values[5].elements, vec![0x7777_7777; 64]);
    assert_eq!(parsed.values[6].name, "eth_difficulty_u32_be");
    assert_eq!(parsed.values[6].elements, vec![0, 0, 0, 0, 0, 0, 0, 1]);
    assert_eq!(parsed.values[7].name, "eth_block_number_u32_le");
    assert_eq!(parsed.values[7].elements, u64_u32_le(input.block_number));
    assert_eq!(parsed.values[8].name, "eth_block_timestamp_u32_le");
    assert_eq!(parsed.values[8].elements, u64_u32_le(input.timestamp));
    assert_eq!(parsed.values[9].name, "eth_extra_data_len");
    assert_eq!(parsed.values[9].elements, vec![4]);
    assert_eq!(parsed.values[10].name, "eth_extra_data_u32_be");
    assert_eq!(parsed.values[10].elements, padded_32_bytes_u32_be(b"lzvm"));
    assert_eq!(parsed.values[11].name, "eth_gas_limit_u32_le");
    assert_eq!(parsed.values[11].elements, u64_u32_le(1_000_000));
    assert_eq!(parsed.values[12].name, "eth_gas_used_u32_le");
    assert_eq!(parsed.values[12].elements, u64_u32_le(900_000));
    assert_eq!(parsed.values[13].name, "eth_base_fee_per_gas_present");
    assert_eq!(parsed.values[13].elements, vec![0]);
    assert_eq!(parsed.values[14].name, "eth_base_fee_per_gas_u32_be");
    assert_eq!(parsed.values[14].elements, vec![0; 8]);
    assert_eq!(parsed.values[15].name, "eth_mix_hash_u32_be");
    assert_eq!(parsed.values[15].elements, hash_u32_be(&input.mix_hash));
    assert_eq!(parsed.values[16].name, "eth_nonce_u32_be");
    assert_eq!(parsed.values[16].elements, bytes_u32_be(&input.nonce));
    assert_eq!(parsed.values[17].name, "eth_ommers_hash_u32_be");
    assert_eq!(parsed.values[17].elements, hash_u32_be(&input.ommers_hash));
    assert_eq!(parsed.values[18].name, "eth_transactions_root_u32_be");
    assert_eq!(
        parsed.values[18].elements,
        hash_u32_be(&input.transactions_root)
    );
    assert_eq!(parsed.values[19].name, "eth_withdrawals_root_present");
    assert_eq!(parsed.values[19].elements, vec![0]);
    assert_eq!(parsed.values[20].name, "eth_withdrawals_root_u32_be");
    assert_eq!(parsed.values[20].elements, vec![0; 8]);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\npublic_values={}\nbytes={}\nsetup_hash={}\npublic_values_hash={}\nvalues=21\npublic_value_fields=170\nblock_hash={}\nparent_hash={}\nbeneficiary={}\nstate_root={}\nreceipts_root={}\ndifficulty=01\nblock_number=2\ntimestamp=101\nextra_data=6c7a766d\ngas_limit=1000000\ngas_used=900000\nbase_fee_per_gas=absent\nmix_hash={}\nnonce={}\ntransactions_root={}\nwithdrawals=absent\n",
            output_path.display(),
            encoded.len(),
            setup_hash_hex,
            to_hex(&public_values_hash),
            to_hex(&input.block_hash),
            to_hex(&input.parent_hash),
            to_hex(&input.beneficiary),
            to_hex(&input.state_root),
            to_hex(&input.receipts_root),
            to_hex(&input.mix_hash),
            to_hex(&input.nonce),
            to_hex(&input.transactions_root)
        )
    );

    fs::remove_dir_all(&dir).expect("fixture directory should be removed");
}

#[test]
fn writes_block_public_values_from_base_fee_block_input() {
    let dir = temp_dir("public-values-base-fee");
    let _ = fs::remove_dir_all(&dir);
    let input_path = dir.join("block.input");
    let output_path = dir.join("public-values.bin");
    let setup_hash = [0x44_u8; 32];
    let setup_hash_hex = to_hex(&setup_hash);
    let block_rlp = sample_block_rlp_with_base_fee();
    let input = build_eth_block_input(&block_rlp).expect("block input should build");
    let encoded_input = encode_eth_block_input(&input).expect("block input should encode");
    write_bytes(&input_path, &encoded_input);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run_cli(
        &[
            "eth",
            "write-block-public-values",
            "--setup-hash",
            &setup_hash_hex,
            input_path.to_str().expect("input path should be utf-8"),
            output_path.to_str().expect("output path should be utf-8"),
        ],
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    let encoded = fs::read(&output_path).expect("public values should be written");
    let parsed = parse_public_values(&encoded).expect("public values should parse");
    let public_values_hash =
        public_values_digest(&parsed).expect("public values digest should compute");
    assert_eq!(parsed.values.len(), 21);
    assert_eq!(parsed.values[5].name, "eth_logs_bloom_u32_be");
    assert_eq!(parsed.values[5].elements, vec![0x7777_7777; 64]);
    assert_eq!(parsed.values[6].name, "eth_difficulty_u32_be");
    assert_eq!(parsed.values[6].elements, vec![0, 0, 0, 0, 0, 0, 0, 1]);
    assert_eq!(parsed.values[9].name, "eth_extra_data_len");
    assert_eq!(parsed.values[9].elements, vec![4]);
    assert_eq!(parsed.values[10].name, "eth_extra_data_u32_be");
    assert_eq!(parsed.values[10].elements, padded_32_bytes_u32_be(b"lzvm"));
    assert_eq!(parsed.values[13].name, "eth_base_fee_per_gas_present");
    assert_eq!(parsed.values[13].elements, vec![1]);
    assert_eq!(parsed.values[14].name, "eth_base_fee_per_gas_u32_be");
    assert_eq!(parsed.values[14].elements, vec![0, 0, 0, 0, 0, 0, 0, 100]);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout should be utf-8"),
        format!(
            "status=ok\npublic_values={}\nbytes={}\nsetup_hash={}\npublic_values_hash={}\nvalues=21\npublic_value_fields=170\nblock_hash={}\nparent_hash={}\nbeneficiary={}\nstate_root={}\nreceipts_root={}\ndifficulty=01\nblock_number=2\ntimestamp=101\nextra_data=6c7a766d\ngas_limit=1000000\ngas_used=900000\nbase_fee_per_gas=64\nmix_hash={}\nnonce={}\ntransactions_root={}\nwithdrawals=absent\n",
            output_path.display(),
            encoded.len(),
            setup_hash_hex,
            to_hex(&public_values_hash),
            to_hex(&input.block_hash),
            to_hex(&input.parent_hash),
            to_hex(&input.beneficiary),
            to_hex(&input.state_root),
            to_hex(&input.receipts_root),
            to_hex(&input.mix_hash),
            to_hex(&input.nonce),
            to_hex(&input.transactions_root)
        )
    );

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

fn sample_block_rlp_with_transaction_items(
    transactions_root: [u8; 32],
    transaction_items: Vec<Vec<u8>>,
) -> Vec<u8> {
    let header_rlp = rlp_list(&legacy_header_items(transactions_root, None));
    let transactions = rlp_list(&transaction_items);
    let empty_list = rlp_list(&[]);
    rlp_list(&[header_rlp, transactions, empty_list])
}

fn sample_block_rlp_with_base_fee() -> Vec<u8> {
    let mut header_items = legacy_header_items(
        hex32("e52f61e61ebdce920205cfca55e00c70bf219b45ea432febbf96152313e61db5"),
        None,
    );
    header_items.push(rlp_bytes(&[0x64]));
    let header_rlp = rlp_list(&header_items);
    let transactions = rlp_list(&[rlp_list(&[rlp_bytes(&[1])])]);
    let empty_list = rlp_list(&[]);
    rlp_list(&[header_rlp, transactions, empty_list])
}

fn sample_block_rlp_with_receipts_root(receipts_root: [u8; 32]) -> Vec<u8> {
    let header_rlp = rlp_list(&legacy_header_items_with_receipts_and_logs_bloom(
        hex32("e52f61e61ebdce920205cfca55e00c70bf219b45ea432febbf96152313e61db5"),
        receipts_root,
        [0; 256],
        &[0x52, 0x08],
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
    legacy_header_items_with_receipts_and_logs_bloom(
        transactions_root,
        [0x66; 32],
        [0x77; 256],
        &[0x0d, 0xbb, 0xa0],
        withdrawals_root,
    )
}

fn legacy_header_items_with_receipts_and_logs_bloom(
    transactions_root: [u8; 32],
    receipts_root: [u8; 32],
    logs_bloom: [u8; 256],
    gas_used: &[u8],
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
        rlp_bytes(&receipts_root),
        rlp_bytes(&logs_bloom),
        rlp_bytes(&[1]),
        rlp_bytes(&[2]),
        rlp_bytes(&[0x0f, 0x42, 0x40]),
        rlp_bytes(gas_used),
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

fn sample_receipt_item() -> Vec<u8> {
    rlp_list(&[
        rlp_bytes(&[1]),
        rlp_bytes(&[0x52, 0x08]),
        rlp_bytes(&[0; 256]),
        rlp_list(&[]),
    ])
}

fn typed_receipt_item(receipt_type: u8) -> Vec<u8> {
    let mut bytes = vec![receipt_type];
    bytes.extend_from_slice(&sample_receipt_item());
    rlp_bytes(&bytes)
}

fn typed_transaction_item() -> Vec<u8> {
    rlp_bytes(&[2, 0xc0])
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

fn hash_u32_be(bytes: &[u8; 32]) -> Vec<u64> {
    bytes_u32_be(bytes)
}

fn bytes_u32_be(bytes: &[u8]) -> Vec<u64> {
    bytes
        .chunks_exact(4)
        .map(|chunk| {
            u64::from(u32::from_be_bytes(
                chunk.try_into().expect("chunk has 4 bytes"),
            ))
        })
        .collect()
}

fn u64_u32_le(value: u64) -> Vec<u64> {
    vec![value & 0xffff_ffff, value >> 32]
}

fn padded_32_bytes_u32_be(bytes: &[u8]) -> Vec<u64> {
    assert!(bytes.len() <= 32);
    let mut padded = [0_u8; 32];
    padded[..bytes.len()].copy_from_slice(bytes);
    bytes_u32_be(&padded)
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
