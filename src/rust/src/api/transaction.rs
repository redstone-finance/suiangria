use napi::bindgen_prelude::*;
use napi_derive::napi;
use sui_json_rpc_types::{Page, SuiTransactionBlockResponseQuery};
use sui_types::base_types::ObjectID;

use crate::{
    to_json,
    utils::{decode_base64, deserialize_bcs, parse_digest, parse_signature},
    SharedState,
};

#[napi]
pub struct TransactionApi {
    sandbox: SharedState,
}

#[napi]
impl TransactionApi {
    pub fn new(sandbox: SharedState) -> Self {
        Self { sandbox }
    }

    #[napi]
    pub fn dry_run(&self, transaction_data: String) -> Result<String> {
        let tx_bytes = decode_base64(&transaction_data)?;
        let tx_data = deserialize_bcs(&tx_bytes)?;
        let result = self
            .sandbox
            .borrow_mut()
            .transaction_mut()
            .dry_run_transaction(tx_data)
            .map_err(|e| Error::from_reason(format!("Dry run transaction failed: {}", e)))?;

        to_json!(result)
    }

    #[napi]
    pub fn execute(&self, transaction_data: String, signatures: Vec<String>) -> Result<String> {
        let tx_bytes = decode_base64(&transaction_data)?;
        let tx_data = deserialize_bcs(&tx_bytes)?;
        let parsed_signatures = signatures
            .iter()
            .map(|s| parse_signature(s))
            .collect::<Result<Vec<_>, _>>()?;

        let result = self
            .sandbox
            .borrow_mut()
            .transaction_mut()
            .execute_function(tx_data, parsed_signatures)
            .map_err(|e| Error::from_reason(format!("Transaction execution failed: {}", e)))?;

        to_json!(result)
    }

    #[napi]
    pub fn get_response(&self, digest: String) -> Result<String> {
        let transaction_digest = parse_digest(&digest)?;
        let response = self
            .sandbox
            .borrow()
            .storage()
            .get_transaction(&transaction_digest)
            .cloned();

        to_json!(response)
    }

    #[napi]
    pub fn query_blocks(&self, params: String) -> Result<String> {
        let query: SuiTransactionBlockResponseQuery = serde_json::from_str(&params)
            .map_err(|e| Error::from_reason(format!("Error parsing query: {}", e)))?;

        let data = self.sandbox.borrow().storage().query(query.filter);

        to_json!(Page::<_, ObjectID> {
            data,
            has_next_page: false,
            next_cursor: None,
        })
    }
}
