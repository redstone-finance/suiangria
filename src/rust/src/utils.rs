use std::str::FromStr;

use base64::{engine::general_purpose, Engine};
use move_core_types::{account_address::AccountAddress, language_storage::StructTag};
use sui_json_rpc_types::{SuiTransactionBlock, SuiTransactionBlockResponse};
use sui_types::{
    base_types::{ObjectID, SuiAddress},
    crypto::{Signature, ToFromBytes},
    digests::TransactionDigest,
    in_memory_storage::InMemoryStorage,
    transaction::{SenderSignedData, TransactionData},
    Identifier, TypeTag,
};

use napi::bindgen_prelude::*;

#[macro_export]
macro_rules! to_json {
    ($value:expr) => {
        serde_json::to_string(&$value)
            .map_err(|e| Error::from_reason(format!("JSON serialization failed: {}", e)))
    };
}

pub fn parse_account_address(address: &str) -> Result<AccountAddress> {
    AccountAddress::from_str(address)
        .map_err(|e| Error::from_reason(format!("Invalid address: {} - {}", address, e)))
}
pub fn parse_identifier(id: &str) -> Result<Identifier> {
    Identifier::from_str(id)
        .map_err(|e| Error::from_reason(format!("Invalid identifier: {} - {}", id, e)))
}

pub fn parse_address(address: &str) -> Result<SuiAddress> {
    SuiAddress::from_str(address)
        .map_err(|e| Error::from_reason(format!("Invalid address: {} - {}", address, e)))
}

pub fn parse_object_id(id: &str) -> Result<ObjectID> {
    ObjectID::from_str(id)
        .map_err(|e| Error::from_reason(format!("Invalid object ID: {} - {}", id, e)))
}

pub fn parse_digest(digest: &str) -> Result<TransactionDigest> {
    TransactionDigest::from_str(digest)
        .map_err(|e| Error::from_reason(format!("Invalid transaction digest: {} - {}", digest, e)))
}

pub fn decode_base64(data: &str) -> Result<Vec<u8>> {
    general_purpose::STANDARD
        .decode(data)
        .map_err(|e| Error::from_reason(format!("Base64 decode failed: {}", e)))
}

pub fn deserialize_bcs<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    bcs::from_bytes(bytes)
        .map_err(|e| Error::from_reason(format!("BCS deserialization failed: {}", e)))
}

pub fn deserialize_json<T: serde::de::DeserializeOwned>(json: &str) -> Result<T> {
    serde_json::from_str(json)
        .map_err(|e| Error::from_reason(format!("JSON deserialization failed: {}", e)))
}

pub fn parse_signature(signature_base64: &str) -> Result<Signature> {
    let sig_bytes = decode_base64(signature_base64)?;
    Signature::from_bytes(&sig_bytes)
        .map_err(|e| Error::from_reason(format!("Signature parsing failed: {}", e)))
}

pub fn parse_optional_type_tag(type_str: Option<String>) -> Option<TypeTag> {
    type_str.map(|st| {
        TypeTag::Struct(Box::new(
            StructTag::from_str(&st).expect("Failed to parse struct tag"),
        ))
    })
}

pub fn serialize_bcs<T: serde::Serialize>(value: &T) -> Result<Vec<u8>> {
    bcs::to_bytes(value).map_err(|e| Error::from_reason(format!("BCS serialization failed: {}", e)))
}

pub fn response_from_errors(
    tx_data: &TransactionData,
    sender_signed: SenderSignedData,
    errors: Vec<String>,
    storage: &InMemoryStorage,
) -> anyhow::Result<SuiTransactionBlockResponse> {
    Ok(SuiTransactionBlockResponse {
        digest: tx_data.digest(),
        transaction: Some(SuiTransactionBlock::try_from(sender_signed, storage)?),
        raw_transaction: bcs::to_bytes(&tx_data)?,
        effects: None,
        events: None,
        object_changes: None,
        balance_changes: None,
        timestamp_ms: None,
        confirmed_local_execution: None,
        checkpoint: None,
        errors,
        raw_effects: vec![],
    })
}
