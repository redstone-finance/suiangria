use sui_json_rpc_types::Coin;
use sui_types::{
    base_types::{MoveObjectType, ObjectID, ObjectRef, SuiAddress},
    object::{Object, Owner},
    TypeTag,
};

use crate::sandbox::storage::StorageExtension;

pub trait CoinExtension {
    fn mint_gas_coin(&mut self, owner: SuiAddress, amount: u64) -> ObjectID;

    fn calculate_balance(&self, owner: SuiAddress, coin_type: Option<TypeTag>) -> u64;

    fn get_coins(&self, owner: SuiAddress, coin_type: Option<TypeTag>) -> Vec<Coin>;

    fn get_default_gas_payment(&self, sender: SuiAddress) -> Vec<ObjectRef>;
}

impl CoinExtension for StorageExtension {
    fn mint_gas_coin(&mut self, owner: SuiAddress, amount: u64) -> ObjectID {
        let coin = Object::new_gas_with_balance_and_owner_for_testing(amount, owner);
        let id = coin.id();
        self.insert_object(coin);

        id
    }
    fn calculate_balance(&self, owner: SuiAddress, coin_type: Option<TypeTag>) -> u64 {
        let target_type =
            coin_type.unwrap_or_else(|| MoveObjectType::gas_coin().coin_type_maybe().unwrap());

        iter_coins_for_owner(self, owner)
            .filter(|obj| obj.coin_type_maybe().is_some_and(|tp| tp == target_type))
            .map(|obj| obj.get_coin_value_unsafe())
            .sum()
    }

    fn get_coins(&self, owner: SuiAddress, coin_type: Option<TypeTag>) -> Vec<Coin> {
        let target_type =
            coin_type.unwrap_or_else(|| MoveObjectType::gas_coin().coin_type_maybe().unwrap());

        iter_coins_for_owner(self, owner)
            .filter(|obj| obj.coin_type_maybe().is_some_and(|tp| tp == target_type))
            .map(|obj| object_to_coin(obj, &target_type))
            .collect()
    }

    fn get_default_gas_payment(&self, sender: SuiAddress) -> Vec<ObjectRef> {
        self.objects_for(&Owner::AddressOwner(sender))
            .filter(|obj| obj.is_gas_coin())
            .map(|obj| obj.compute_object_reference())
            .collect()
    }
}

fn iter_coins_for_owner(
    storage: &StorageExtension,
    owner: SuiAddress,
) -> impl Iterator<Item = &Object> {
    storage
        .objects_for(&Owner::AddressOwner(owner))
        .filter(|obj| obj.is_coin())
}

fn object_to_coin(object: &Object, coin_type: &TypeTag) -> Coin {
    Coin {
        coin_type: coin_type.to_canonical_string(true),
        coin_object_id: object.id(),
        version: object.version(),
        digest: object.digest(),
        balance: object.get_coin_value_unsafe(),
        previous_transaction: object.previous_transaction,
    }
}
