use std::collections::BTreeMap;

use sui_types::{
    accumulator_event::AccumulatorEvent,
    accumulator_root::{AccumulatorObjId, AccumulatorValue},
    balance::Balance,
    base_types::SuiAddress,
    effects::{AccumulatorOperation, AccumulatorValue as AccumulatorEventValue},
    gas_coin::GAS,
    TypeTag,
};

use crate::sandbox::storage::StorageExtension;

pub trait AddressBalanceExtension {
    fn address_balance(&self, owner: SuiAddress, coin_type: Option<TypeTag>) -> u64;

    fn change_address_balance(&mut self, owner: SuiAddress, balance_type: TypeTag, change: i128);

    fn has_sufficient_funds(&self, withdrawals: &BTreeMap<AccumulatorObjId, u64>) -> bool;

    fn settle_accumulator_events(&mut self, events: Vec<AccumulatorEvent>);
}

impl AddressBalanceExtension for StorageExtension {
    fn address_balance(&self, owner: SuiAddress, coin_type: Option<TypeTag>) -> u64 {
        let balance_type = Balance::type_tag(coin_type.unwrap_or_else(GAS::type_tag));

        AccumulatorValue::get_field_id(owner, &balance_type)
            .map(|id| self.accumulator_value(&id))
            .unwrap_or_default()
    }

    fn change_address_balance(&mut self, owner: SuiAddress, balance_type: TypeTag, change: i128) {
        let Ok(id) = AccumulatorValue::get_field_id(owner, &balance_type) else {
            return;
        };

        let balance = (self.accumulator_value(&id) as i128 + change) as u64;
        let mut object = AccumulatorValue::create_for_testing(owner, balance_type, balance);
        if let Some(existing) = self.get_object(id.inner()) {
            let next_version = existing.version().next();
            object
                .data
                .try_as_move_mut()
                .unwrap()
                .increment_version_to(next_version);
        }

        self.insert_object(object);
    }

    fn has_sufficient_funds(&self, withdrawals: &BTreeMap<AccumulatorObjId, u64>) -> bool {
        withdrawals
            .iter()
            .all(|(id, amount)| self.accumulator_value(id) >= *amount)
    }

    fn settle_accumulator_events(&mut self, events: Vec<AccumulatorEvent>) {
        for event in events {
            let AccumulatorEventValue::Integer(amount) = event.write.value else {
                continue;
            };
            let change = match event.write.operation {
                AccumulatorOperation::Merge => amount as i128,
                AccumulatorOperation::Split => -(amount as i128),
            };
            let address = event.write.address;

            self.change_address_balance(address.address, address.ty, change);
        }
    }
}

impl StorageExtension {
    fn accumulator_value(&self, id: &AccumulatorObjId) -> u64 {
        self.get_object(id.inner())
            .and_then(|object| object.data.try_as_move())
            .and_then(|object| AccumulatorValue::try_from(object).ok())
            .and_then(|value| value.as_u128())
            .map_or(0, |value| value as u64)
    }
}
