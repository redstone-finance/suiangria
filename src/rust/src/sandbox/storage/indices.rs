use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use sui_json_rpc_types::{ObjectChange, TransactionFilter};
use sui_types::{
    base_types::{ObjectID, SuiAddress},
    digests::TransactionDigest,
    object::Owner,
    transaction::{
        CallArg, Command, ObjectArg, ProgrammableTransaction, TransactionData, TransactionDataAPI,
        TransactionKind,
    },
};

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct MoveCallSignature {
    pub package: ObjectID,
    pub module: String,
    pub function: String,
}

#[derive(Default, Debug, Clone)]
pub struct TransactionMetadata {
    pub digest: TransactionDigest,
    pub sender: SuiAddress,
    pub recipients: HashSet<SuiAddress>,
    pub input_objects: HashSet<ObjectID>,
    pub created_objects: HashSet<ObjectID>,
    pub mutated_objects: HashSet<ObjectID>,
    pub deleted_objects: HashSet<ObjectID>,
    pub wrapped_objects: HashSet<ObjectID>,
    pub move_calls: Vec<MoveCallSignature>,
    pub transaction_kind: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct TransactionIndices {
    by_sender: HashMap<SuiAddress, HashSet<TransactionDigest>>,
    by_recipient: HashMap<SuiAddress, HashSet<TransactionDigest>>,
    by_sender_and_recipient: HashMap<(SuiAddress, SuiAddress), HashSet<TransactionDigest>>,

    by_input_object: HashMap<ObjectID, HashSet<TransactionDigest>>,
    by_created_object: HashMap<ObjectID, HashSet<TransactionDigest>>,
    by_mutated_object: HashMap<ObjectID, HashSet<TransactionDigest>>,
    by_deleted_object: HashMap<ObjectID, HashSet<TransactionDigest>>,
    by_wrapped_object: HashMap<ObjectID, HashSet<TransactionDigest>>,

    by_move_call: HashMap<MoveCallSignature, HashSet<TransactionDigest>>,
    by_package: HashMap<ObjectID, HashSet<TransactionDigest>>,
    by_module: HashMap<(ObjectID, String), HashSet<TransactionDigest>>,

    by_transaction_kind: HashMap<String, HashSet<TransactionDigest>>,
}

impl TransactionIndices {
    pub fn new() -> Self {
        Self {
            by_sender: HashMap::new(),
            by_recipient: HashMap::new(),
            by_sender_and_recipient: HashMap::new(),
            by_input_object: HashMap::new(),
            by_created_object: HashMap::new(),
            by_mutated_object: HashMap::new(),
            by_deleted_object: HashMap::new(),
            by_wrapped_object: HashMap::new(),
            by_move_call: HashMap::new(),
            by_package: HashMap::new(),
            by_module: HashMap::new(),
            by_transaction_kind: HashMap::new(),
        }
    }

    pub fn index_transaction(
        &mut self,
        transaction_data: &TransactionData,
        object_changes: &[ObjectChange],
    ) {
        let metadata =
            self.build_metadata(transaction_data.digest(), transaction_data, object_changes);

        self.index_sender(&metadata);
        self.index_recipients(&metadata);
        self.index_objects(&metadata);
        self.index_move_calls(&metadata);
        self.index_transaction_kind(&metadata);
    }
    pub fn query(&self, filter: &TransactionFilter) -> HashSet<TransactionDigest> {
        match filter {
            TransactionFilter::Checkpoint(_) => HashSet::new(),

            TransactionFilter::MoveFunction {
                package,
                module,
                function,
            } => self.query_by_move_function(*package, module.as_deref(), function.as_deref()),

            TransactionFilter::InputObject(object_id) => self.query_by_input_object(*object_id),

            TransactionFilter::ChangedObject(object_id) => self.query_by_changed_object(*object_id),

            TransactionFilter::AffectedObject(object_id) => {
                self.query_by_affected_object(*object_id)
            }

            TransactionFilter::FromAddress(address) => self.query_by_sender(*address),

            TransactionFilter::ToAddress(address) => self.query_by_recipient(*address),

            TransactionFilter::FromAndToAddress { from, to } => {
                self.query_by_sender_and_recipient(*from, *to)
            }

            TransactionFilter::FromOrToAddress { addr } => {
                let mut results = self.query_by_sender(*addr);
                results.extend(self.query_by_recipient(*addr));

                results
            }

            TransactionFilter::TransactionKind(kind) => self.query_by_transaction_kind(kind),

            TransactionFilter::TransactionKindIn(kinds) => self.query_by_transaction_kinds(kinds),
        }
    }

    fn build_metadata(
        &self,
        digest: TransactionDigest,
        transaction_data: &TransactionData,
        object_changes: &[ObjectChange],
    ) -> TransactionMetadata {
        let sender = transaction_data.sender();
        let mut recipients = HashSet::new();
        let mut input_objects = HashSet::new();
        let mut created_objects = HashSet::new();
        let mut mutated_objects = HashSet::new();
        let mut deleted_objects = HashSet::new();
        let mut wrapped_objects = HashSet::new();
        let mut move_calls = Vec::new();

        self.extract_from_object_changes(
            object_changes,
            &mut recipients,
            &mut created_objects,
            &mut mutated_objects,
            &mut deleted_objects,
            &mut wrapped_objects,
        );

        self.extract_from_transaction_data(transaction_data, &mut input_objects, &mut move_calls);

        let transaction_kind = self.classify_transaction_kind(transaction_data);

        TransactionMetadata {
            digest,
            sender,
            recipients,
            input_objects,
            created_objects,
            mutated_objects,
            deleted_objects,
            wrapped_objects,
            move_calls,
            transaction_kind,
        }
    }

    fn extract_from_object_changes(
        &self,
        object_changes: &[ObjectChange],
        recipients: &mut HashSet<SuiAddress>,
        created_objects: &mut HashSet<ObjectID>,
        mutated_objects: &mut HashSet<ObjectID>,
        deleted_objects: &mut HashSet<ObjectID>,
        wrapped_objects: &mut HashSet<ObjectID>,
    ) {
        for change in object_changes {
            match change {
                ObjectChange::Created {
                    object_id, owner, ..
                } => {
                    created_objects.insert(*object_id);
                    if let Owner::AddressOwner(address) = owner {
                        recipients.insert(*address);
                    }
                }
                ObjectChange::Mutated { object_id, .. } => {
                    mutated_objects.insert(*object_id);
                }
                ObjectChange::Deleted { object_id, .. } => {
                    deleted_objects.insert(*object_id);
                }
                ObjectChange::Wrapped { object_id, .. } => {
                    wrapped_objects.insert(*object_id);
                }
                ObjectChange::Transferred {
                    object_id,
                    recipient,
                    ..
                } => {
                    mutated_objects.insert(*object_id);
                    if let Owner::AddressOwner(address) = recipient {
                        recipients.insert(*address);
                    }
                }
                ObjectChange::Published { .. } => {}
            }
        }
    }

    fn extract_from_transaction_data(
        &self,
        transaction_data: &TransactionData,
        input_objects: &mut HashSet<ObjectID>,
        move_calls: &mut Vec<MoveCallSignature>,
    ) {
        if let TransactionKind::ProgrammableTransaction(pt) = transaction_data.kind() {
            self.extract_input_objects_from_programmable(pt, input_objects);
            self.extract_move_calls_from_programmable(pt, move_calls);
        }
    }

    fn extract_input_objects_from_programmable(
        &self,
        pt: &ProgrammableTransaction,
        input_objects: &mut HashSet<ObjectID>,
    ) {
        for input in &pt.inputs {
            if let CallArg::Object(object_arg) = input {
                match object_arg {
                    ObjectArg::ImmOrOwnedObject(oref) => {
                        input_objects.insert(oref.0);
                    }
                    ObjectArg::SharedObject { id, .. } => {
                        input_objects.insert(*id);
                    }
                    ObjectArg::Receiving(oref) => {
                        input_objects.insert(oref.0);
                    }
                }
            }
        }
    }

    fn extract_move_calls_from_programmable(
        &self,
        pt: &ProgrammableTransaction,
        move_calls: &mut Vec<MoveCallSignature>,
    ) {
        for command in &pt.commands {
            if let Command::MoveCall(call) = command {
                move_calls.push(MoveCallSignature {
                    package: call.package,
                    module: call.module.to_string(),
                    function: call.function.to_string(),
                });
            }
        }
    }

    fn classify_transaction_kind(&self, transaction_data: &TransactionData) -> String {
        transaction_data.kind().to_string()
    }

    fn index_sender(&mut self, metadata: &TransactionMetadata) {
        self.by_sender
            .entry(metadata.sender)
            .or_default()
            .insert(metadata.digest);
    }

    fn index_recipients(&mut self, metadata: &TransactionMetadata) {
        for recipient in &metadata.recipients {
            self.by_recipient
                .entry(*recipient)
                .or_default()
                .insert(metadata.digest);

            self.by_sender_and_recipient
                .entry((metadata.sender, *recipient))
                .or_default()
                .insert(metadata.digest);
        }
    }

    fn index_objects(&mut self, metadata: &TransactionMetadata) {
        for object_id in &metadata.input_objects {
            self.by_input_object
                .entry(*object_id)
                .or_default()
                .insert(metadata.digest);
        }

        for object_id in &metadata.created_objects {
            self.by_created_object
                .entry(*object_id)
                .or_default()
                .insert(metadata.digest);
        }

        for object_id in &metadata.mutated_objects {
            self.by_mutated_object
                .entry(*object_id)
                .or_default()
                .insert(metadata.digest);
        }

        for object_id in &metadata.deleted_objects {
            self.by_deleted_object
                .entry(*object_id)
                .or_default()
                .insert(metadata.digest);
        }

        for object_id in &metadata.wrapped_objects {
            self.by_wrapped_object
                .entry(*object_id)
                .or_default()
                .insert(metadata.digest);
        }
    }

    fn index_move_calls(&mut self, metadata: &TransactionMetadata) {
        for move_call in &metadata.move_calls {
            self.by_move_call
                .entry(move_call.clone())
                .or_default()
                .insert(metadata.digest);

            self.by_package
                .entry(move_call.package)
                .or_default()
                .insert(metadata.digest);

            self.by_module
                .entry((move_call.package, move_call.module.clone()))
                .or_default()
                .insert(metadata.digest);
        }
    }

    fn index_transaction_kind(&mut self, metadata: &TransactionMetadata) {
        self.by_transaction_kind
            .entry(metadata.transaction_kind.clone())
            .or_default()
            .insert(metadata.digest);
    }

    fn query_by_sender(&self, sender: SuiAddress) -> HashSet<TransactionDigest> {
        self.by_sender.get(&sender).cloned().unwrap_or_default()
    }

    fn query_by_recipient(&self, recipient: SuiAddress) -> HashSet<TransactionDigest> {
        self.by_recipient
            .get(&recipient)
            .cloned()
            .unwrap_or_default()
    }

    fn query_by_sender_and_recipient(
        &self,
        sender: SuiAddress,
        recipient: SuiAddress,
    ) -> HashSet<TransactionDigest> {
        self.by_sender_and_recipient
            .get(&(sender, recipient))
            .cloned()
            .unwrap_or_default()
    }

    fn query_by_input_object(&self, object_id: ObjectID) -> HashSet<TransactionDigest> {
        self.by_input_object
            .get(&object_id)
            .cloned()
            .unwrap_or_default()
    }

    fn query_by_changed_object(&self, object_id: ObjectID) -> HashSet<TransactionDigest> {
        let mut results = HashSet::new();

        if let Some(txs) = self.by_created_object.get(&object_id) {
            results.extend(txs);
        }
        if let Some(txs) = self.by_mutated_object.get(&object_id) {
            results.extend(txs);
        }
        if let Some(txs) = self.by_wrapped_object.get(&object_id) {
            results.extend(txs);
        }

        results
    }

    fn query_by_affected_object(&self, object_id: ObjectID) -> HashSet<TransactionDigest> {
        let mut results = HashSet::new();

        if let Some(txs) = self.by_input_object.get(&object_id) {
            results.extend(txs);
        }
        if let Some(txs) = self.by_created_object.get(&object_id) {
            results.extend(txs);
        }
        if let Some(txs) = self.by_mutated_object.get(&object_id) {
            results.extend(txs);
        }
        if let Some(txs) = self.by_deleted_object.get(&object_id) {
            results.extend(txs);
        }
        if let Some(txs) = self.by_wrapped_object.get(&object_id) {
            results.extend(txs);
        }

        results
    }

    fn query_by_move_function(
        &self,
        package: ObjectID,
        module: Option<&str>,
        function: Option<&str>,
    ) -> HashSet<TransactionDigest> {
        match (module, function) {
            (Some(mod_name), Some(func_name)) => {
                let signature = MoveCallSignature {
                    package,
                    module: mod_name.to_string(),
                    function: func_name.to_string(),
                };
                self.by_move_call
                    .get(&signature)
                    .cloned()
                    .unwrap_or_default()
            }
            (Some(mod_name), None) => self
                .by_module
                .get(&(package, mod_name.to_string()))
                .cloned()
                .unwrap_or_default(),
            (None, _) => self.by_package.get(&package).cloned().unwrap_or_default(),
        }
    }

    fn query_by_transaction_kind(&self, kind: &str) -> HashSet<TransactionDigest> {
        self.by_transaction_kind
            .get(kind)
            .cloned()
            .unwrap_or_default()
    }

    fn query_by_transaction_kinds(&self, kinds: &[String]) -> HashSet<TransactionDigest> {
        kinds
            .iter()
            .flat_map(|kind| self.query_by_transaction_kind(kind))
            .collect()
    }

    // pub fn intersect_queries(
    //     &self,
    //     sets: Vec<HashSet<TransactionDigest>>,
    // ) -> HashSet<TransactionDigest> {
    //     if sets.is_empty() {
    //         return HashSet::new();
    //     }

    //     sets.into_iter()
    //         .reduce(|acc, set| &acc & &set)
    //         .unwrap_or_default()
    // }

    // pub fn union_queries(
    //     &self,
    //     sets: Vec<HashSet<TransactionDigest>>,
    // ) -> HashSet<TransactionDigest> {
    //     sets.into_iter().flatten().collect()
    // }
}
