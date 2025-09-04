use serde::{Deserialize, Serialize};
use sui_json_rpc_types::{SuiTransactionBlockResponseOptions, TransactionFilter};
use sui_types::{
    base_types::{ObjectID, SequenceNumber},
    dynamic_field::DynamicFieldName,
};

// types I did not found in sui code.

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TryGetPastObjectParams {
    pub id: ObjectID,
    pub version: SequenceNumber,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetDynamicFieldsParams {
    pub parent_id: ObjectID,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetDynamicFieldObjectParams {
    pub parent_id: ObjectID,
    pub name: DynamicFieldName,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryTransactionBlocksParams {
    pub filter: TransactionFilter,
    pub options: SuiTransactionBlockResponseOptions,
}
