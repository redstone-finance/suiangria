use std::collections::HashMap;

use anyhow::anyhow;

use move_core_types::language_storage::StructTag;
use sui_json_rpc_types::{BalanceChange, ObjectChange};
use sui_types::{
    base_types::SuiAddress,
    crypto::Signature,
    effects::{TransactionEffects, TransactionEffectsAPI},
    inner_temporary_store::InnerTemporaryStore,
    object::Object,
    transaction::{
        CheckedInputObjects, InputObjectKind, InputObjects, ObjectReadResult, ObjectReadResultKind,
        TransactionData, TransactionDataAPI,
    },
    TypeTag,
};

use crate::sandbox::{AuthExtension, StorageExtension};

pub trait Changes {
    fn get_checked_objects(
        &self,
        tx_data: &TransactionData,
        signatures: Option<&[Signature]>,
        auth_extension: &AuthExtension,
    ) -> anyhow::Result<CheckedInputObjects>;

    fn extract_balance_changes(
        &self,
        effects: &TransactionEffects,
        temporary_store: &InnerTemporaryStore,
    ) -> Vec<BalanceChange>;

    fn compute_object_changes(
        &self,
        temporary_store: &InnerTemporaryStore,
        effects: &TransactionEffects,
        sender: SuiAddress,
    ) -> Vec<ObjectChange>;
}

impl Changes for StorageExtension {
    fn get_checked_objects(
        &self,
        tx_data: &TransactionData,
        signatures: Option<&[Signature]>,
        auth_extension: &AuthExtension,
    ) -> anyhow::Result<CheckedInputObjects> {
        let mut inputs = vec![];

        for gas_ref in &tx_data.gas_data().payment {
            let gas_object = self
                .get_object(&gas_ref.0)
                .ok_or(anyhow!("Gas payment object {} not found", gas_ref.0))?
                .clone();
            if let Some(signatures) = signatures {
                if gas_object.owner().is_address_owned() {
                    let address = gas_object.owner.get_address_owner_address().unwrap();
                    auth_extension.verify_object_ownership(address, signatures)?;
                }
            }

            inputs.push(ObjectReadResult {
                input_object_kind: InputObjectKind::ImmOrOwnedMoveObject(*gas_ref),
                object: ObjectReadResultKind::Object(gas_object),
            });
        }

        for ik in tx_data.input_objects()? {
            let object = self
                .get_object(&ik.object_id())
                .ok_or(anyhow!("No object {}", ik.object_id()))?
                .clone();

            if let Some(signatures) = signatures {
                if object.owner().is_address_owned() {
                    let address = object.owner.get_address_owner_address().unwrap();
                    auth_extension.verify_object_ownership(address, signatures)?;
                }
            }

            inputs.push(ObjectReadResult {
                input_object_kind: ik,
                object: ObjectReadResultKind::from(object),
            })
        }

        Ok(CheckedInputObjects::new_with_checked_transaction_inputs(
            InputObjects::new(inputs),
        ))
    }

    fn extract_balance_changes(
        &self,
        effects: &TransactionEffects,
        temporary_store: &InnerTemporaryStore,
    ) -> Vec<BalanceChange> {
        let mut balance_changes = HashMap::new();

        for created_obj in temporary_store.written.values() {
            if !created_obj.is_coin() {
                continue;
            }
            if let Some(coin_type) = extract_coin_type(&created_obj.struct_tag().unwrap()) {
                let entry = balance_changes
                    .entry((created_obj.owner.clone(), coin_type.clone()))
                    .or_insert(BalanceChange {
                        coin_type,
                        amount: 0,
                        owner: created_obj.owner().clone(),
                    });
                entry.amount += extract_coin_value(created_obj) as i128;
            }
        }

        for (deleted_id, _) in effects.all_tombstones() {
            let deleted_obj = match self.get_object(&deleted_id) {
                Some(object) => object,
                None => continue,
            };
            if let Some(coin_type) = extract_coin_type(&deleted_obj.struct_tag().unwrap()) {
                let entry = balance_changes
                    .entry((deleted_obj.owner.clone(), coin_type.clone()))
                    .or_insert(BalanceChange {
                        owner: deleted_obj.owner().clone(),
                        coin_type,
                        amount: 0,
                    });
                entry.amount -= extract_coin_value(deleted_obj) as i128;
            }
        }

        balance_changes.into_values().collect()
    }

    fn compute_object_changes(
        &self,
        temporary_store: &InnerTemporaryStore,
        effects: &TransactionEffects,
        sender: SuiAddress,
    ) -> Vec<ObjectChange> {
        let mut changes = Vec::new();

        for (id, new_object) in &temporary_store.written {
            let change = match (self.get_object(id), new_object.struct_tag()) {
                (None, Some(tag)) => ObjectChange::Created {
                    sender,
                    owner: new_object.owner().clone(),
                    object_type: tag,
                    object_id: *id,
                    version: new_object.version(),
                    digest: new_object.digest(),
                },
                (Some(old), Some(tag)) if old.owner() == new_object.owner() => {
                    ObjectChange::Mutated {
                        sender,
                        owner: new_object.owner().clone(),
                        object_type: tag,
                        object_id: *id,
                        previous_version: old.version(),
                        version: new_object.version(),
                        digest: new_object.digest(),
                    }
                }
                (Some(_), Some(tag)) => ObjectChange::Transferred {
                    sender,
                    recipient: new_object.owner().clone(),
                    object_type: tag,
                    object_id: *id,
                    version: new_object.version(),
                    digest: new_object.digest(),
                },
                (_, None) => ObjectChange::Published {
                    package_id: new_object.id(),
                    version: new_object.version(),
                    digest: new_object.digest(),
                    modules: extract_module_names(new_object),
                },
            };
            changes.push(change);
        }

        for (id, version) in effects.all_tombstones() {
            if let Some(object) = self.get_object(&id) {
                if let Some(tag) = object.struct_tag() {
                    let change = if effects.wrapped().iter().any(|w| w.0 == id) {
                        ObjectChange::Wrapped {
                            sender,
                            object_type: tag,
                            object_id: id,
                            version,
                        }
                    } else {
                        ObjectChange::Deleted {
                            sender,
                            object_type: tag,
                            object_id: id,
                            version,
                        }
                    };
                    changes.push(change);
                }
            }
        }

        changes
    }
}

fn extract_coin_type(struct_tag: &StructTag) -> Option<TypeTag> {
    if struct_tag.module.as_str() == "coin" && struct_tag.name.as_str() == "Coin" {
        struct_tag.type_params.first().cloned()
    } else {
        None
    }
}

fn extract_coin_value(object: &Object) -> u64 {
    object
        .as_coin_maybe()
        .map(|coin| coin.balance.value())
        .unwrap_or_default()
}
fn extract_module_names(object: &Object) -> Vec<String> {
    object
        .data
        .try_as_package()
        .map(|pkg| pkg.serialized_module_map().keys().cloned().collect())
        .unwrap_or_default()
}
