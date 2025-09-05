use std::collections::{BTreeMap, HashMap, HashSet};

use crate::sandbox::storage::{indices::TransactionIndices, ObjectTimeline};
use serde::{Deserialize, Serialize};
use sui_json_rpc_types::SuiTransactionBlockResponse;
use sui_types::{
    base_types::ObjectID,
    digests::TransactionDigest,
    object::{Object, Owner},
};

#[derive(Serialize, Deserialize)]
pub struct StorageSnapshot {
    pub objects: BTreeMap<ObjectID, Object>,
    pub address_objects: HashMap<Owner, HashSet<ObjectID>>,
    pub object_addresses: HashMap<ObjectID, Owner>,
    pub timelines: HashMap<ObjectID, ObjectTimeline>,
    pub transactions: HashMap<TransactionDigest, SuiTransactionBlockResponse>,
    pub transaction_indices: TransactionIndices,
    pub checkpoint: u64,
}
