use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    io::{Read, Write},
};
use sui_json_rpc_types::{ObjectChange, SuiTransactionBlockResponse, TransactionFilter};
use sui_types::{
    base_types::{ObjectID, SequenceNumber},
    digests::TransactionDigest,
    in_memory_storage::InMemoryStorage,
    inner_temporary_store::InnerTemporaryStore,
    object::{Object, Owner, PastObjectRead},
    transaction::TransactionData,
};

use crate::sandbox::storage::{indices::TransactionIndices, snapshot::StorageSnapshot};

mod indices;
mod snapshot;

#[derive(Serialize, Deserialize, Clone)]
pub struct ObjectTimeline {
    versions: BTreeMap<SequenceNumber, Object>,
    deleted_at: Option<SequenceNumber>,
}

pub struct StorageExtension {
    inner: InMemoryStorage,
    address_objects: HashMap<Owner, HashSet<ObjectID>>,
    object_addresses: HashMap<ObjectID, Owner>,
    timelines: HashMap<ObjectID, ObjectTimeline>,
    transactions: HashMap<TransactionDigest, SuiTransactionBlockResponse>,
    transaction_indices: TransactionIndices,
    checkpoint: u64,
}

impl StorageExtension {
    pub fn new(inner: InMemoryStorage) -> Self {
        Self {
            inner,
            address_objects: Default::default(),
            object_addresses: Default::default(),
            timelines: Default::default(),
            transactions: Default::default(),
            transaction_indices: TransactionIndices::new(),
            checkpoint: 0,
        }
    }

    pub fn checkpoint(&self) -> u64 {
        self.checkpoint
    }

    pub fn bump_checkpoint(&mut self) {
        self.checkpoint += 1;
    }

    pub fn as_inner(&self) -> &InMemoryStorage {
        &self.inner
    }

    pub fn insert_object(&mut self, object: Object) {
        let object_id = object.id();
        let version = object.version();

        let timeline = self
            .timelines
            .entry(object_id)
            .or_insert_with(|| ObjectTimeline {
                versions: BTreeMap::new(),
                deleted_at: None,
            });

        timeline.versions.insert(version, object.clone());
        timeline.deleted_at = None;

        self.update_ownership_tracking(object_id, &object);
        self.inner.insert_object(object);
    }

    pub fn remove_object_without_trace(&mut self, object_id: ObjectID) {
        self.remove_object(object_id);

        self.timelines.remove(&object_id);
    }

    pub fn remove_object(&mut self, object_id: ObjectID) {
        if let Some(timeline) = self.timelines.get_mut(&object_id) {
            if let Some((last_version, _)) = timeline.versions.last_key_value() {
                timeline.deleted_at = Some(*last_version);
            }
        }

        self.clear_ownership_tracking(object_id);
        self.inner.remove_object(object_id);
    }

    // ideally record_transaction and insert_transaction would be done at the same stage,
    // but not easy to do at the moment
    pub fn record_transaction(
        &mut self,
        transaction_data: &TransactionData,
        object_changes: &[ObjectChange],
    ) {
        self.transaction_indices
            .index_transaction(transaction_data, object_changes);
    }

    pub fn insert_transaction(
        &mut self,
        digest: TransactionDigest,
        response: SuiTransactionBlockResponse,
    ) {
        self.transactions.insert(digest, response);
    }

    fn update_ownership_tracking(&mut self, object_id: ObjectID, object: &Object) {
        if let Some(previous_address) = self.object_addresses.remove(&object_id) {
            if let Some(set) = self.address_objects.get_mut(&previous_address) {
                set.remove(&object_id);
            }
        }

        if object.is_address_owned() || object.is_child_object() {
            let owner = object.owner.clone();
            self.address_objects
                .entry(owner.clone())
                .or_default()
                .insert(object_id);
            self.object_addresses.insert(object_id, owner);
        }
    }

    fn clear_ownership_tracking(&mut self, object_id: ObjectID) {
        if let Some(address) = self.object_addresses.remove(&object_id) {
            if let Some(set) = self.address_objects.get_mut(&address) {
                set.remove(&object_id);
            }
        }
    }

    pub fn wrap_object(&mut self, object_id: ObjectID, wrapped_object: Object) {
        let version = wrapped_object.version();

        let timeline = self
            .timelines
            .entry(object_id)
            .or_insert_with(|| ObjectTimeline {
                versions: BTreeMap::new(),
                deleted_at: None,
            });

        timeline.versions.insert(version, wrapped_object.clone());

        self.clear_ownership_tracking(object_id);
        self.inner.insert_object(wrapped_object);
    }

    pub fn finish(&mut self, written: BTreeMap<ObjectID, Object>) {
        for (_, object) in written {
            self.insert_object(object);
        }
    }

    pub fn get_object(&self, id: &ObjectID) -> Option<&Object> {
        self.inner.get_object(id)
    }

    pub fn get_object_at_version(
        &self,
        id: &ObjectID,
        version: SequenceNumber,
    ) -> anyhow::Result<PastObjectRead> {
        let timeline = match self.timelines.get(id) {
            Some(timeline) => timeline,
            None => return Ok(PastObjectRead::ObjectNotExists(*id)),
        };

        if let Some(obj) = timeline.versions.get(&version) {
            return Ok(PastObjectRead::VersionFound(
                obj.compute_object_reference(),
                obj.clone(),
                obj.get_layout(&self.inner)?,
            ));
        }

        if let Some(deleted_at) = timeline.deleted_at {
            let past = timeline.versions.get(&deleted_at).unwrap();
            return Ok(PastObjectRead::ObjectDeleted((
                *id,
                deleted_at,
                past.digest(),
            )));
        }

        if let Some((max_version, _)) = timeline.versions.last_key_value() {
            if version > *max_version {
                return Ok(PastObjectRead::VersionTooHigh {
                    object_id: *id,
                    asked_version: version,
                    latest_version: *max_version,
                });
            }
        }

        Ok(PastObjectRead::VersionNotFound(*id, version))
    }

    pub fn objects_for(&self, owner: &Owner) -> impl Iterator<Item = &Object> {
        self.address_objects
            .get(owner)
            .into_iter()
            .flat_map(|set| set.iter())
            .filter_map(move |id| self.inner.get_object(id))
    }

    // probably not needed, but at the same time might be nice for simulations where a test process can
    // move from one state to another
    // todo: also remove data from TransactionIndices
    pub fn _rollback_to_version(
        &mut self,
        id: &ObjectID,
        version: SequenceNumber,
    ) -> Option<Object> {
        let object = self
            .timelines
            .get(id)
            .and_then(|timeline| timeline.versions.get(&version))
            .cloned()?;

        self.update_ownership_tracking(*id, &object);
        self.inner.insert_object(object.clone());

        if let Some(timeline) = self.timelines.get_mut(id) {
            timeline.deleted_at = None;
        }

        Some(object)
    }

    pub fn get_transaction(
        &self,
        digest: &TransactionDigest,
    ) -> Option<&SuiTransactionBlockResponse> {
        self.transactions.get(digest)
    }

    pub fn apply_transaction_effects(
        &mut self,
        transaction_data: &TransactionData,
        object_changes: &[ObjectChange],
        temporary_store: &InnerTemporaryStore,
    ) {
        self.apply_changes(object_changes, temporary_store);

        self.record_transaction(transaction_data, object_changes);
    }

    fn apply_changes(
        &mut self,
        object_changes: &[ObjectChange],
        temporary_store: &InnerTemporaryStore,
    ) {
        for change in object_changes {
            match change {
                ObjectChange::Created { object_id, .. }
                | ObjectChange::Mutated { object_id, .. }
                | ObjectChange::Transferred { object_id, .. }
                | ObjectChange::Published {
                    package_id: object_id,
                    ..
                } => {
                    if let Some(object) = temporary_store.written.get(object_id) {
                        self.insert_object(object.clone());
                    }
                }
                ObjectChange::Wrapped { object_id, .. } => {
                    if let Some(object) = temporary_store.written.get(object_id) {
                        self.wrap_object(*object_id, object.clone());
                    }
                }
                ObjectChange::Deleted { object_id, .. } => {
                    self.remove_object(*object_id);
                }
            }
        }
    }

    pub fn query(&self, filter: Option<TransactionFilter>) -> Vec<SuiTransactionBlockResponse> {
        match filter {
            Some(filter) => {
                let digests = self.transaction_indices.query(&filter);

                digests
                    .iter()
                    .filter_map(|key| self.transactions.get(key).cloned())
                    .collect()
            }
            None => self.transactions.values().cloned().collect(),
        }
    }

    pub fn to_snapshot(&self) -> StorageSnapshot {
        StorageSnapshot {
            objects: self.inner.objects().clone(),
            address_objects: self.address_objects.clone(),
            object_addresses: self.object_addresses.clone(),
            timelines: self.timelines.clone(),
            transactions: self.transactions.clone(),
            transaction_indices: self.transaction_indices.clone(),
            checkpoint: self.checkpoint,
        }
    }

    pub fn from_snapshot(snapshot: StorageSnapshot) -> Self {
        let mut inner = InMemoryStorage::new(Default::default());

        for (_, object) in snapshot.objects {
            inner.insert_object(object);
        }

        Self {
            inner,
            address_objects: snapshot.address_objects,
            object_addresses: snapshot.object_addresses,
            timelines: snapshot.timelines,
            transactions: snapshot.transactions,
            transaction_indices: snapshot.transaction_indices,
            checkpoint: snapshot.checkpoint,
        }
    }

    pub fn restore_from_snapshot(&mut self, snapshot: StorageSnapshot) {
        let mut new_inner = InMemoryStorage::new(Default::default());

        for (_, object) in snapshot.objects {
            new_inner.insert_object(object);
        }

        self.inner = new_inner;
        self.address_objects = snapshot.address_objects;
        self.object_addresses = snapshot.object_addresses;
        self.timelines = snapshot.timelines;
        self.transactions = snapshot.transactions;
        self.transaction_indices = snapshot.transaction_indices;
        self.checkpoint = snapshot.checkpoint;
    }

    pub fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        let snapshot = self.to_snapshot();
        let bytes = bcs::to_bytes(&snapshot)?;

        Ok(bytes)
    }

    pub fn restone_from_bytes(&mut self, bytes: &[u8]) -> anyhow::Result<()> {
        let mut decoder = GzDecoder::new(bytes);
        let mut decompressed = Vec::new();

        decoder.read_to_end(&mut decompressed)?;

        let snapshot: StorageSnapshot = bcs::from_bytes(&decompressed)?;

        self.restore_from_snapshot(snapshot);

        Ok(())
    }

    pub fn to_bytes_compressed(&self) -> anyhow::Result<Vec<u8>> {
        let bytes = self.to_bytes()?;
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());

        encoder.write_all(&bytes)?;

        Ok(encoder.finish()?)
    }
}
