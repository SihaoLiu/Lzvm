use std::fmt;

use lzvm_artifacts::rlp::{encode_rlp, RlpItem};

pub(crate) fn block_rlp_from_rpc_json(input: &[u8]) -> Result<Vec<u8>, RpcBlockJsonError> {
    let root = JsonParser::new(input).parse()?;
    let block = rpc_block_object(&root)?;
    let mut body = vec![
        RlpItem::List(header_items(block)?),
        RlpItem::List(transaction_items(block)?),
        RlpItem::List(ommer_items(block)?),
    ];

    match optional_field(block, "withdrawals") {
        Some(JsonValue::Null) => {
            if has_field(block, "withdrawalsRoot") {
                return Err(invalid("RPC block has withdrawalsRoot without withdrawals"));
            }
        }
        Some(value) => body.push(RlpItem::List(withdrawal_items(value)?)),
        None => {
            if has_field(block, "withdrawalsRoot") {
                return Err(invalid("RPC block has withdrawalsRoot without withdrawals"));
            }
        }
    }

    Ok(encode_rlp(&RlpItem::List(body)))
}

pub(crate) fn receipts_rlp_from_rpc_json(input: &[u8]) -> Result<Vec<u8>, RpcBlockJsonError> {
    let root = JsonParser::new(input).parse()?;
    let receipts = rpc_receipts_array(&root)?;
    let items = receipts
        .iter()
        .map(receipt_item)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(encode_rlp(&RlpItem::List(items)))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RpcBlockJsonError {
    Json(JsonParseError),
    Invalid(String),
}

impl fmt::Display for RpcBlockJsonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(error) => write!(f, "{error}"),
            Self::Invalid(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for RpcBlockJsonError {}

impl From<JsonParseError> for RpcBlockJsonError {
    fn from(error: JsonParseError) -> Self {
        Self::Json(error)
    }
}

fn invalid(message: impl Into<String>) -> RpcBlockJsonError {
    RpcBlockJsonError::Invalid(message.into())
}

fn rpc_block_object(root: &JsonValue) -> Result<&[(String, JsonValue)], RpcBlockJsonError> {
    let root = as_object(root, "RPC root")?;
    if let Some(result) = optional_field(root, "result") {
        if matches!(result, JsonValue::Null) {
            return Err(invalid("RPC block result is null"));
        }
        return as_object(result, "RPC block result");
    }
    Ok(root)
}

fn header_items(block: &[(String, JsonValue)]) -> Result<Vec<RlpItem>, RpcBlockJsonError> {
    let mut header = vec![
        required_fixed_item::<32>(block, "parentHash")?,
        required_fixed_item::<32>(block, "sha3Uncles")?,
        required_fixed_item_with_alias::<20>(block, "miner", "beneficiary")?,
        required_fixed_item::<32>(block, "stateRoot")?,
        required_fixed_item::<32>(block, "transactionsRoot")?,
        required_fixed_item::<32>(block, "receiptsRoot")?,
        required_fixed_item::<256>(block, "logsBloom")?,
        required_quantity_item(block, "difficulty")?,
        required_quantity_item(block, "number")?,
        required_quantity_item(block, "gasLimit")?,
        required_quantity_item(block, "gasUsed")?,
        required_quantity_item(block, "timestamp")?,
        required_data_item(block, "extraData")?,
        required_fixed_item_with_alias::<32>(block, "mixHash", "prevRandao")?,
        required_fixed_item::<8>(block, "nonce")?,
    ];

    let has_base_fee = push_optional_quantity(&mut header, block, "baseFeePerGas")?;
    require_previous_field(has_base_fee, block, "baseFeePerGas", "withdrawalsRoot")?;
    let has_withdrawals_root = push_optional_fixed::<32>(&mut header, block, "withdrawalsRoot")?;
    let has_blob_gas_used = has_field(block, "blobGasUsed");
    let has_excess_blob_gas = has_field(block, "excessBlobGas");
    if has_blob_gas_used != has_excess_blob_gas {
        return Err(invalid(
            "RPC block must provide blobGasUsed and excessBlobGas together",
        ));
    }
    if has_blob_gas_used {
        if !has_withdrawals_root {
            return Err(invalid(
                "RPC block field blobGasUsed requires withdrawalsRoot",
            ));
        }
        header.push(required_quantity_item(block, "blobGasUsed")?);
        header.push(required_quantity_item(block, "excessBlobGas")?);
    }

    require_previous_field(
        has_blob_gas_used,
        block,
        "blobGasUsed",
        "parentBeaconBlockRoot",
    )?;
    let has_parent_beacon_block_root =
        push_optional_fixed::<32>(&mut header, block, "parentBeaconBlockRoot")?;
    if has_field(block, "requestsHash") && !has_parent_beacon_block_root {
        return Err(invalid(
            "RPC block field requestsHash requires parentBeaconBlockRoot",
        ));
    }
    push_optional_fixed::<32>(&mut header, block, "requestsHash")?;

    Ok(header)
}

fn transaction_items(block: &[(String, JsonValue)]) -> Result<Vec<RlpItem>, RpcBlockJsonError> {
    let transactions = required_array(block, "transactions")?;
    transactions.iter().map(transaction_item).collect()
}

fn rpc_receipts_array(root: &JsonValue) -> Result<&[JsonValue], RpcBlockJsonError> {
    match root {
        JsonValue::Array(items) => Ok(items),
        JsonValue::Object(entries) => {
            let result = required_field(entries, "result")?;
            if matches!(result, JsonValue::Null) {
                return Err(invalid("RPC receipts result is null"));
            }
            as_array(result, "RPC receipts result")
        }
        _ => Err(invalid(
            "expected RPC receipts root to be an array or result object",
        )),
    }
}

fn transaction_item(transaction: &JsonValue) -> Result<RlpItem, RpcBlockJsonError> {
    let object = as_object(transaction, "RPC transaction")?;
    let transaction_type = optional_transaction_type(object)?;
    match transaction_type {
        0 => legacy_transaction_item(object),
        1 => Ok(typed_envelope_item(
            1,
            access_list_transaction_fields(object)?,
        )),
        2 => Ok(typed_envelope_item(
            2,
            dynamic_fee_transaction_fields(object)?,
        )),
        3 => Ok(typed_envelope_item(3, blob_transaction_fields(object)?)),
        4 => Ok(typed_envelope_item(
            4,
            authorized_transaction_fields(object)?,
        )),
        value => Err(invalid(format!(
            "unsupported RPC transaction type: 0x{value:x}"
        ))),
    }
}

fn legacy_transaction_item(
    transaction: &[(String, JsonValue)],
) -> Result<RlpItem, RpcBlockJsonError> {
    Ok(RlpItem::List(vec![
        required_quantity_item(transaction, "nonce")?,
        required_quantity_item(transaction, "gasPrice")?,
        required_quantity_item(transaction, "gas")?,
        optional_address_item(transaction, "to")?,
        required_quantity_item(transaction, "value")?,
        required_data_item_with_alias(transaction, "input", "data")?,
        required_quantity_item(transaction, "v")?,
        required_quantity_item(transaction, "r")?,
        required_quantity_item(transaction, "s")?,
    ]))
}

fn access_list_transaction_fields(
    transaction: &[(String, JsonValue)],
) -> Result<Vec<RlpItem>, RpcBlockJsonError> {
    Ok(vec![
        required_quantity_item(transaction, "chainId")?,
        required_quantity_item(transaction, "nonce")?,
        required_quantity_item(transaction, "gasPrice")?,
        required_quantity_item(transaction, "gas")?,
        optional_address_item(transaction, "to")?,
        required_quantity_item(transaction, "value")?,
        required_data_item_with_alias(transaction, "input", "data")?,
        access_list_item(transaction)?,
        y_parity_item(transaction)?,
        required_quantity_item(transaction, "r")?,
        required_quantity_item(transaction, "s")?,
    ])
}

fn dynamic_fee_transaction_fields(
    transaction: &[(String, JsonValue)],
) -> Result<Vec<RlpItem>, RpcBlockJsonError> {
    dynamic_fee_transaction_fields_with_destination(
        transaction,
        optional_address_item(transaction, "to")?,
    )
}

fn dynamic_fee_transaction_fields_with_destination(
    transaction: &[(String, JsonValue)],
    destination: RlpItem,
) -> Result<Vec<RlpItem>, RpcBlockJsonError> {
    Ok(vec![
        required_quantity_item(transaction, "chainId")?,
        required_quantity_item(transaction, "nonce")?,
        required_quantity_item(transaction, "maxPriorityFeePerGas")?,
        required_quantity_item(transaction, "maxFeePerGas")?,
        required_quantity_item(transaction, "gas")?,
        destination,
        required_quantity_item(transaction, "value")?,
        required_data_item_with_alias(transaction, "input", "data")?,
        access_list_item(transaction)?,
        y_parity_item(transaction)?,
        required_quantity_item(transaction, "r")?,
        required_quantity_item(transaction, "s")?,
    ])
}

fn blob_transaction_fields(
    transaction: &[(String, JsonValue)],
) -> Result<Vec<RlpItem>, RpcBlockJsonError> {
    let mut fields = dynamic_fee_transaction_fields_with_destination(
        transaction,
        required_address_item(transaction, "to", 3)?,
    )?;
    fields.insert(9, required_quantity_item(transaction, "maxFeePerBlobGas")?);
    fields.insert(10, blob_versioned_hashes_item(transaction)?);
    Ok(fields)
}

fn authorized_transaction_fields(
    transaction: &[(String, JsonValue)],
) -> Result<Vec<RlpItem>, RpcBlockJsonError> {
    let mut fields = dynamic_fee_transaction_fields_with_destination(
        transaction,
        required_address_item(transaction, "to", 4)?,
    )?;
    fields.insert(9, authorization_list_item(transaction)?);
    Ok(fields)
}

fn typed_envelope_item(item_type: u8, fields: Vec<RlpItem>) -> RlpItem {
    let payload = encode_rlp(&RlpItem::List(fields));
    let mut envelope = Vec::with_capacity(1 + payload.len());
    envelope.push(item_type);
    envelope.extend_from_slice(&payload);
    RlpItem::Bytes(envelope)
}

fn optional_transaction_type(transaction: &[(String, JsonValue)]) -> Result<u8, RpcBlockJsonError> {
    optional_type(transaction, "RPC transaction field type")
}

fn optional_receipt_type(receipt: &[(String, JsonValue)]) -> Result<u8, RpcBlockJsonError> {
    optional_type(receipt, "RPC receipt field type")
}

fn optional_type(
    object: &[(String, JsonValue)],
    context: &'static str,
) -> Result<u8, RpcBlockJsonError> {
    let Some(value) = optional_field(object, "type") else {
        return Ok(0);
    };
    let string = as_string(value, context)?;
    let bytes = hex_quantity(string, context)?;
    if bytes.len() > 1 {
        return Err(invalid(format!("{context} exceeds one byte")));
    }
    Ok(bytes.first().copied().unwrap_or(0))
}

fn ommer_items(block: &[(String, JsonValue)]) -> Result<Vec<RlpItem>, RpcBlockJsonError> {
    let Some(uncles) = optional_field(block, "uncles") else {
        return Ok(Vec::new());
    };
    let uncles = as_array(uncles, "RPC block field uncles")?;
    if !uncles.is_empty() {
        return Err(invalid(
            "RPC block uncles must be empty because RPC ommer hashes do not contain ommer headers",
        ));
    }
    Ok(Vec::new())
}

fn receipt_item(receipt: &JsonValue) -> Result<RlpItem, RpcBlockJsonError> {
    let object = as_object(receipt, "RPC receipt")?;
    let receipt_type = optional_receipt_type(object)?;
    let body = receipt_body_items(object, receipt_type)?;
    match receipt_type {
        0 => Ok(RlpItem::List(body)),
        1..=4 => Ok(typed_envelope_item(receipt_type, body)),
        value => Err(invalid(format!(
            "unsupported RPC receipt type: 0x{value:x}"
        ))),
    }
}

fn receipt_body_items(
    receipt: &[(String, JsonValue)],
    receipt_type: u8,
) -> Result<Vec<RlpItem>, RpcBlockJsonError> {
    Ok(vec![
        receipt_status_or_root_item(receipt, receipt_type)?,
        required_quantity_item(receipt, "cumulativeGasUsed")?,
        required_fixed_item::<256>(receipt, "logsBloom")?,
        receipt_logs_item(receipt)?,
    ])
}

fn receipt_status_or_root_item(
    receipt: &[(String, JsonValue)],
    receipt_type: u8,
) -> Result<RlpItem, RpcBlockJsonError> {
    match (
        optional_field(receipt, "status"),
        optional_field(receipt, "root"),
    ) {
        (Some(_), Some(_)) => Err(invalid("RPC receipt cannot contain both status and root")),
        (Some(value), None) => receipt_status_item(value),
        (None, Some(value)) => {
            if receipt_type != 0 {
                return Err(invalid("RPC typed receipt requires status"));
            }
            Ok(RlpItem::Bytes(fixed_hex_string::<32>(value, "root")?))
        }
        (None, None) => Err(invalid("missing RPC receipt field: status or root")),
    }
}

fn receipt_status_item(value: &JsonValue) -> Result<RlpItem, RpcBlockJsonError> {
    let status = hex_quantity(as_string(value, "status")?, "status")?;
    if status.is_empty() || status == [1] {
        return Ok(RlpItem::Bytes(status));
    }
    Err(invalid(format!(
        "invalid RPC receipt status: {}",
        format_hex_quantity(&status)
    )))
}

fn receipt_logs_item(receipt: &[(String, JsonValue)]) -> Result<RlpItem, RpcBlockJsonError> {
    let logs = required_array(receipt, "logs")?
        .iter()
        .map(|log| {
            let log = as_object(log, "RPC receipt log")?;
            let topics = required_array(log, "topics")?
                .iter()
                .map(|topic| Ok(RlpItem::Bytes(fixed_hex_string::<32>(topic, "topic")?)))
                .collect::<Result<Vec<_>, RpcBlockJsonError>>()?;
            Ok(RlpItem::List(vec![
                required_fixed_item::<20>(log, "address")?,
                RlpItem::List(topics),
                required_data_item(log, "data")?,
            ]))
        })
        .collect::<Result<Vec<_>, RpcBlockJsonError>>()?;
    Ok(RlpItem::List(logs))
}

fn withdrawal_items(value: &JsonValue) -> Result<Vec<RlpItem>, RpcBlockJsonError> {
    let withdrawals = as_array(value, "RPC block field withdrawals")?;
    withdrawals
        .iter()
        .map(|withdrawal| {
            let withdrawal = as_object(withdrawal, "RPC withdrawal")?;
            Ok(RlpItem::List(vec![
                required_quantity_item(withdrawal, "index")?,
                required_quantity_item(withdrawal, "validatorIndex")?,
                required_fixed_item::<20>(withdrawal, "address")?,
                required_quantity_item(withdrawal, "amount")?,
            ]))
        })
        .collect()
}

fn access_list_item(transaction: &[(String, JsonValue)]) -> Result<RlpItem, RpcBlockJsonError> {
    let access_list = required_array(transaction, "accessList")?;
    let entries = access_list
        .iter()
        .map(|entry| {
            let entry = as_object(entry, "RPC access list entry")?;
            let storage_keys = required_array(entry, "storageKeys")?
                .iter()
                .map(|key| Ok(RlpItem::Bytes(fixed_hex_string::<32>(key, "storage key")?)))
                .collect::<Result<Vec<_>, RpcBlockJsonError>>()?;
            Ok(RlpItem::List(vec![
                required_fixed_item::<20>(entry, "address")?,
                RlpItem::List(storage_keys),
            ]))
        })
        .collect::<Result<Vec<_>, RpcBlockJsonError>>()?;
    Ok(RlpItem::List(entries))
}

fn blob_versioned_hashes_item(
    transaction: &[(String, JsonValue)],
) -> Result<RlpItem, RpcBlockJsonError> {
    let hashes = required_array(transaction, "blobVersionedHashes")?
        .iter()
        .map(|hash| Ok(RlpItem::Bytes(fixed_hex_string::<32>(hash, "blob hash")?)))
        .collect::<Result<Vec<_>, RpcBlockJsonError>>()?;
    Ok(RlpItem::List(hashes))
}

fn authorization_list_item(
    transaction: &[(String, JsonValue)],
) -> Result<RlpItem, RpcBlockJsonError> {
    let authorizations = required_array(transaction, "authorizationList")?
        .iter()
        .map(|authorization| {
            let authorization = as_object(authorization, "RPC authorization")?;
            Ok(RlpItem::List(vec![
                required_quantity_item(authorization, "chainId")?,
                required_fixed_item::<20>(authorization, "address")?,
                required_quantity_item(authorization, "nonce")?,
                y_parity_item(authorization)?,
                required_quantity_item(authorization, "r")?,
                required_quantity_item(authorization, "s")?,
            ]))
        })
        .collect::<Result<Vec<_>, RpcBlockJsonError>>()?;
    Ok(RlpItem::List(authorizations))
}

fn y_parity_item(object: &[(String, JsonValue)]) -> Result<RlpItem, RpcBlockJsonError> {
    required_quantity_item_with_alias(object, "yParity", "v")
}

fn required_quantity_item(
    object: &[(String, JsonValue)],
    field: &'static str,
) -> Result<RlpItem, RpcBlockJsonError> {
    Ok(RlpItem::Bytes(hex_quantity(
        required_string(object, field)?,
        field,
    )?))
}

fn required_quantity_item_with_alias(
    object: &[(String, JsonValue)],
    field: &'static str,
    alias: &'static str,
) -> Result<RlpItem, RpcBlockJsonError> {
    Ok(RlpItem::Bytes(hex_quantity(
        required_string_with_alias(object, field, alias)?,
        field,
    )?))
}

fn required_data_item(
    object: &[(String, JsonValue)],
    field: &'static str,
) -> Result<RlpItem, RpcBlockJsonError> {
    Ok(RlpItem::Bytes(hex_data(
        required_string(object, field)?,
        field,
    )?))
}

fn required_data_item_with_alias(
    object: &[(String, JsonValue)],
    field: &'static str,
    alias: &'static str,
) -> Result<RlpItem, RpcBlockJsonError> {
    let value = required_string_with_alias(object, field, alias)?;
    Ok(RlpItem::Bytes(hex_data(value, field)?))
}

fn required_fixed_item<const N: usize>(
    object: &[(String, JsonValue)],
    field: &'static str,
) -> Result<RlpItem, RpcBlockJsonError> {
    Ok(RlpItem::Bytes(required_fixed::<N>(object, field)?))
}

fn required_fixed_item_with_alias<const N: usize>(
    object: &[(String, JsonValue)],
    field: &'static str,
    alias: &'static str,
) -> Result<RlpItem, RpcBlockJsonError> {
    let value = required_string_with_alias(object, field, alias)?;
    let bytes = fixed_hex(value, N, field)?;
    Ok(RlpItem::Bytes(bytes))
}

fn optional_address_item(
    object: &[(String, JsonValue)],
    field: &'static str,
) -> Result<RlpItem, RpcBlockJsonError> {
    match required_field(object, field)? {
        JsonValue::Null => Ok(RlpItem::Bytes(Vec::new())),
        value => Ok(RlpItem::Bytes(fixed_hex_string::<20>(value, field)?)),
    }
}

fn required_address_item(
    object: &[(String, JsonValue)],
    field: &'static str,
    transaction_type: u8,
) -> Result<RlpItem, RpcBlockJsonError> {
    match required_field(object, field)? {
        JsonValue::Null => Err(invalid(format!(
            "RPC transaction type 0x{transaction_type:x} field {field} must not be null"
        ))),
        value => Ok(RlpItem::Bytes(fixed_hex_string::<20>(value, field)?)),
    }
}

fn required_fixed<const N: usize>(
    object: &[(String, JsonValue)],
    field: &'static str,
) -> Result<Vec<u8>, RpcBlockJsonError> {
    fixed_hex(required_string(object, field)?, N, field)
}

fn fixed_hex_string<const N: usize>(
    value: &JsonValue,
    context: &'static str,
) -> Result<Vec<u8>, RpcBlockJsonError> {
    fixed_hex(as_string(value, context)?, N, context)
}

fn fixed_hex(
    value: &str,
    expected_len: usize,
    context: &'static str,
) -> Result<Vec<u8>, RpcBlockJsonError> {
    let bytes = hex_data(value, context)?;
    if bytes.len() != expected_len {
        return Err(invalid(format!(
            "expected {context} to have {expected_len} bytes, found {}",
            bytes.len()
        )));
    }
    Ok(bytes)
}

fn push_optional_quantity(
    header: &mut Vec<RlpItem>,
    block: &[(String, JsonValue)],
    field: &'static str,
) -> Result<bool, RpcBlockJsonError> {
    if !has_field(block, field) {
        return Ok(false);
    }
    header.push(required_quantity_item(block, field)?);
    Ok(true)
}

fn push_optional_fixed<const N: usize>(
    header: &mut Vec<RlpItem>,
    block: &[(String, JsonValue)],
    field: &'static str,
) -> Result<bool, RpcBlockJsonError> {
    if !has_field(block, field) {
        return Ok(false);
    }
    header.push(required_fixed_item::<N>(block, field)?);
    Ok(true)
}

fn require_previous_field(
    present: bool,
    object: &[(String, JsonValue)],
    previous: &'static str,
    field: &'static str,
) -> Result<(), RpcBlockJsonError> {
    if !present && has_field(object, field) {
        return Err(invalid(format!(
            "RPC block field {field} requires {previous}"
        )));
    }
    Ok(())
}

fn required_array<'a>(
    object: &'a [(String, JsonValue)],
    field: &'static str,
) -> Result<&'a [JsonValue], RpcBlockJsonError> {
    as_array(required_field(object, field)?, field)
}

fn required_string<'a>(
    object: &'a [(String, JsonValue)],
    field: &'static str,
) -> Result<&'a str, RpcBlockJsonError> {
    as_string(required_field(object, field)?, field)
}

fn required_string_with_alias<'a>(
    object: &'a [(String, JsonValue)],
    field: &'static str,
    alias: &'static str,
) -> Result<&'a str, RpcBlockJsonError> {
    if let Some(value) = optional_field(object, field) {
        let value = as_string(value, field)?;
        if let Some(alias_value) = optional_field(object, alias) {
            let alias_value = as_string(alias_value, alias)?;
            if value != alias_value {
                return Err(invalid(format!(
                    "conflicting RPC fields: {field} and {alias}"
                )));
            }
        }
        return Ok(value);
    }
    as_string(required_field(object, alias)?, alias)
}

fn required_field<'a>(
    object: &'a [(String, JsonValue)],
    field: &'static str,
) -> Result<&'a JsonValue, RpcBlockJsonError> {
    optional_field(object, field).ok_or_else(|| invalid(format!("missing RPC field: {field}")))
}

fn optional_field<'a>(object: &'a [(String, JsonValue)], field: &str) -> Option<&'a JsonValue> {
    object
        .iter()
        .find_map(|(key, value)| (key == field).then_some(value))
}

fn has_field(object: &[(String, JsonValue)], field: &str) -> bool {
    optional_field(object, field).is_some()
}

fn as_object<'a>(
    value: &'a JsonValue,
    context: &'static str,
) -> Result<&'a [(String, JsonValue)], RpcBlockJsonError> {
    match value {
        JsonValue::Object(entries) => Ok(entries),
        _ => Err(invalid(format!("expected {context} to be an object"))),
    }
}

fn as_array<'a>(
    value: &'a JsonValue,
    context: &'static str,
) -> Result<&'a [JsonValue], RpcBlockJsonError> {
    match value {
        JsonValue::Array(items) => Ok(items),
        _ => Err(invalid(format!("expected {context} to be an array"))),
    }
}

fn as_string<'a>(
    value: &'a JsonValue,
    context: &'static str,
) -> Result<&'a str, RpcBlockJsonError> {
    match value {
        JsonValue::String(value) => Ok(value),
        _ => Err(invalid(format!("expected {context} to be a string"))),
    }
}

fn hex_data(value: &str, context: &'static str) -> Result<Vec<u8>, RpcBlockJsonError> {
    let digits = hex_digits(value, context)?;
    if !digits.len().is_multiple_of(2) {
        return Err(invalid(format!(
            "expected {context} hex data to have even digit count"
        )));
    }
    hex_digits_to_bytes(digits, context)
}

fn hex_quantity(value: &str, context: &'static str) -> Result<Vec<u8>, RpcBlockJsonError> {
    let digits = hex_digits(value, context)?;
    if digits.is_empty() {
        return Err(invalid(format!(
            "expected {context} quantity to be non-empty"
        )));
    }
    if digits.len() > 1 && digits[0] == b'0' {
        return Err(invalid(format!(
            "expected {context} quantity to be canonically encoded"
        )));
    }
    if digits == b"0" {
        return Ok(Vec::new());
    }
    let mut padded = Vec::with_capacity(digits.len() + digits.len() % 2);
    if !digits.len().is_multiple_of(2) {
        padded.push(b'0');
    }
    padded.extend_from_slice(digits);
    hex_digits_to_bytes(&padded, context)
}

fn hex_digits<'a>(value: &'a str, context: &'static str) -> Result<&'a [u8], RpcBlockJsonError> {
    let Some(digits) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    else {
        return Err(invalid(format!("expected {context} to be 0x-prefixed hex")));
    };
    Ok(digits.as_bytes())
}

fn hex_digits_to_bytes(digits: &[u8], context: &'static str) -> Result<Vec<u8>, RpcBlockJsonError> {
    let mut bytes = Vec::with_capacity(digits.len() / 2);
    for pair in digits.chunks_exact(2) {
        let high = hex_value(pair[0]).ok_or_else(|| {
            invalid(format!(
                "invalid hex digit in {context}: {}",
                printable_byte(pair[0])
            ))
        })?;
        let low = hex_value(pair[1]).ok_or_else(|| {
            invalid(format!(
                "invalid hex digit in {context}: {}",
                printable_byte(pair[1])
            ))
        })?;
        bytes.push((high << 4) | low);
    }
    Ok(bytes)
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn printable_byte(byte: u8) -> String {
    if byte.is_ascii_graphic() {
        char::from(byte).to_string()
    } else {
        format!("0x{byte:02x}")
    }
}

fn format_hex_quantity(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return "0x0".to_owned();
    }
    let mut output = String::with_capacity(2 + bytes.len() * 2);
    output.push_str("0x");
    for (index, byte) in bytes.iter().enumerate() {
        if index > 0 || *byte >= 16 {
            output.push(hex_char(byte >> 4));
        }
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

#[derive(Debug, Clone, PartialEq, Eq)]
enum JsonValue {
    Object(Vec<(String, JsonValue)>),
    Array(Vec<JsonValue>),
    String(String),
    Number,
    Bool,
    Null,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum JsonParseError {
    UnexpectedEof { offset: usize },
    UnexpectedByte { offset: usize, byte: u8 },
    UnexpectedTrailingBytes { offset: usize },
    DuplicateObjectKey { offset: usize, key: String },
    InvalidNumber { offset: usize },
    InvalidStringEscape { offset: usize },
    InvalidUnicodeEscape { offset: usize },
}

impl fmt::Display for JsonParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedEof { offset } => write!(f, "unexpected end of JSON at {offset}"),
            Self::UnexpectedByte { offset, byte } => {
                write!(
                    f,
                    "unexpected JSON byte at {offset}: {}",
                    printable_byte(*byte)
                )
            }
            Self::UnexpectedTrailingBytes { offset } => {
                write!(f, "unexpected trailing JSON bytes at {offset}")
            }
            Self::DuplicateObjectKey { offset, key } => {
                write!(f, "duplicate JSON object key at {offset}: {key}")
            }
            Self::InvalidNumber { offset } => write!(f, "invalid JSON number at {offset}"),
            Self::InvalidStringEscape { offset } => {
                write!(f, "invalid JSON string escape at {offset}")
            }
            Self::InvalidUnicodeEscape { offset } => {
                write!(f, "invalid JSON unicode escape at {offset}")
            }
        }
    }
}

impl std::error::Error for JsonParseError {}

struct JsonParser<'a> {
    input: &'a [u8],
    cursor: usize,
}

impl<'a> JsonParser<'a> {
    fn new(input: &'a [u8]) -> Self {
        Self { input, cursor: 0 }
    }

    fn parse(mut self) -> Result<JsonValue, JsonParseError> {
        self.skip_ws();
        let value = self.parse_value()?;
        self.skip_ws();
        if self.cursor != self.input.len() {
            return Err(JsonParseError::UnexpectedTrailingBytes {
                offset: self.cursor,
            });
        }
        Ok(value)
    }

    fn parse_value(&mut self) -> Result<JsonValue, JsonParseError> {
        self.skip_ws();
        match self.peek()? {
            b'{' => self.parse_object(),
            b'[' => self.parse_array(),
            b'"' => Ok(JsonValue::String(self.parse_string()?)),
            b't' => {
                self.expect_literal(b"true")?;
                Ok(JsonValue::Bool)
            }
            b'f' => {
                self.expect_literal(b"false")?;
                Ok(JsonValue::Bool)
            }
            b'n' => {
                self.expect_literal(b"null")?;
                Ok(JsonValue::Null)
            }
            b'-' | b'0'..=b'9' => self.parse_number(),
            byte => Err(JsonParseError::UnexpectedByte {
                offset: self.cursor,
                byte,
            }),
        }
    }

    fn parse_object(&mut self) -> Result<JsonValue, JsonParseError> {
        self.expect_byte(b'{')?;
        self.skip_ws();
        let mut entries = Vec::new();
        if self.consume_byte(b'}') {
            return Ok(JsonValue::Object(entries));
        }
        loop {
            self.skip_ws();
            let key_offset = self.cursor;
            let key = self.parse_string()?;
            if entries.iter().any(|(existing, _)| existing == &key) {
                return Err(JsonParseError::DuplicateObjectKey {
                    offset: key_offset,
                    key,
                });
            }
            self.skip_ws();
            self.expect_byte(b':')?;
            let value = self.parse_value()?;
            entries.push((key, value));
            self.skip_ws();
            if self.consume_byte(b'}') {
                return Ok(JsonValue::Object(entries));
            }
            self.expect_byte(b',')?;
        }
    }

    fn parse_array(&mut self) -> Result<JsonValue, JsonParseError> {
        self.expect_byte(b'[')?;
        self.skip_ws();
        let mut items = Vec::new();
        if self.consume_byte(b']') {
            return Ok(JsonValue::Array(items));
        }
        loop {
            items.push(self.parse_value()?);
            self.skip_ws();
            if self.consume_byte(b']') {
                return Ok(JsonValue::Array(items));
            }
            self.expect_byte(b',')?;
        }
    }

    fn parse_string(&mut self) -> Result<String, JsonParseError> {
        self.expect_byte(b'"')?;
        let mut output = String::new();
        loop {
            let byte = self.next()?;
            match byte {
                b'"' => return Ok(output),
                b'\\' => output.push(self.parse_escape()?),
                0x00..=0x1f => {
                    return Err(JsonParseError::InvalidStringEscape {
                        offset: self.cursor - 1,
                    });
                }
                _ => output.push(char::from(byte)),
            }
        }
    }

    fn parse_escape(&mut self) -> Result<char, JsonParseError> {
        let offset = self.cursor;
        match self.next()? {
            b'"' => Ok('"'),
            b'\\' => Ok('\\'),
            b'/' => Ok('/'),
            b'b' => Ok('\u{0008}'),
            b'f' => Ok('\u{000c}'),
            b'n' => Ok('\n'),
            b'r' => Ok('\r'),
            b't' => Ok('\t'),
            b'u' => self.parse_unicode_escape(offset),
            _ => Err(JsonParseError::InvalidStringEscape { offset }),
        }
    }

    fn parse_unicode_escape(&mut self, offset: usize) -> Result<char, JsonParseError> {
        let value = self.parse_hex_u16(offset)?;
        if (0xd800..=0xdfff).contains(&value) {
            return Err(JsonParseError::InvalidUnicodeEscape { offset });
        }
        char::from_u32(u32::from(value)).ok_or(JsonParseError::InvalidUnicodeEscape { offset })
    }

    fn parse_hex_u16(&mut self, offset: usize) -> Result<u16, JsonParseError> {
        let mut value = 0_u16;
        for _ in 0..4 {
            let byte = self.next()?;
            let digit = hex_value(byte).ok_or(JsonParseError::InvalidUnicodeEscape { offset })?;
            value = (value << 4) | u16::from(digit);
        }
        Ok(value)
    }

    fn parse_number(&mut self) -> Result<JsonValue, JsonParseError> {
        let start = self.cursor;
        self.consume_byte(b'-');
        match self.peek()? {
            b'0' => {
                self.cursor += 1;
            }
            b'1'..=b'9' => {
                self.cursor += 1;
                while self
                    .peek_optional()
                    .is_some_and(|byte| byte.is_ascii_digit())
                {
                    self.cursor += 1;
                }
            }
            _ => return Err(JsonParseError::InvalidNumber { offset: start }),
        }
        if self.consume_byte(b'.') {
            let fraction_start = self.cursor;
            while self
                .peek_optional()
                .is_some_and(|byte| byte.is_ascii_digit())
            {
                self.cursor += 1;
            }
            if self.cursor == fraction_start {
                return Err(JsonParseError::InvalidNumber { offset: start });
            }
        }
        if self
            .peek_optional()
            .is_some_and(|byte| matches!(byte, b'e' | b'E'))
        {
            self.cursor += 1;
            if self
                .peek_optional()
                .is_some_and(|byte| matches!(byte, b'+' | b'-'))
            {
                self.cursor += 1;
            }
            let exponent_start = self.cursor;
            while self
                .peek_optional()
                .is_some_and(|byte| byte.is_ascii_digit())
            {
                self.cursor += 1;
            }
            if self.cursor == exponent_start {
                return Err(JsonParseError::InvalidNumber { offset: start });
            }
        }
        Ok(JsonValue::Number)
    }

    fn expect_literal(&mut self, literal: &[u8]) -> Result<(), JsonParseError> {
        for expected in literal {
            self.expect_byte(*expected)?;
        }
        Ok(())
    }

    fn expect_byte(&mut self, expected: u8) -> Result<(), JsonParseError> {
        let byte = self.next()?;
        if byte != expected {
            return Err(JsonParseError::UnexpectedByte {
                offset: self.cursor - 1,
                byte,
            });
        }
        Ok(())
    }

    fn consume_byte(&mut self, expected: u8) -> bool {
        if self.peek_optional() == Some(expected) {
            self.cursor += 1;
            true
        } else {
            false
        }
    }

    fn skip_ws(&mut self) {
        while self
            .peek_optional()
            .is_some_and(|byte| byte.is_ascii_whitespace())
        {
            self.cursor += 1;
        }
    }

    fn peek(&self) -> Result<u8, JsonParseError> {
        self.peek_optional().ok_or(JsonParseError::UnexpectedEof {
            offset: self.cursor,
        })
    }

    fn peek_optional(&self) -> Option<u8> {
        self.input.get(self.cursor).copied()
    }

    fn next(&mut self) -> Result<u8, JsonParseError> {
        let byte = self.peek()?;
        self.cursor += 1;
        Ok(byte)
    }
}
