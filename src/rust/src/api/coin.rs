use napi::bindgen_prelude::*;
use napi_derive::napi;
use sui_types::{balance::Balance, gas_coin::GAS};

use crate::{
    sandbox::{AddressBalanceExtension, CoinExtension},
    to_json,
    utils::{parse_address, parse_optional_type_tag},
    SharedState,
};

#[napi]
pub struct CoinApi {
    sandbox: SharedState,
}

#[napi]
impl CoinApi {
    pub fn new(sandbox: SharedState) -> Self {
        Self { sandbox }
    }

    #[napi]
    pub fn mint_sui(&self, address: String, amount: i64) -> Result<String> {
        let id = self
            .sandbox
            .borrow_mut()
            .storage_mut()
            .mint_gas_coin(parse_address(&address)?, amount as u64);

        Ok(id.to_hex())
    }

    #[napi]
    pub fn mint_sui_to_address_balance(&self, address: String, amount: i64) -> Result<()> {
        self.sandbox
            .borrow_mut()
            .storage_mut()
            .change_address_balance(
                parse_address(&address)?,
                Balance::type_tag(GAS::type_tag()),
                amount as i128,
            );

        Ok(())
    }

    #[napi]
    pub fn get_address_balance(&self, address: String, struct_type: Option<String>) -> Result<i64> {
        let tag = parse_optional_type_tag(struct_type);
        let balance = self
            .sandbox
            .borrow()
            .storage()
            .address_balance(parse_address(&address)?, tag);

        Ok(balance as i64)
    }

    #[napi]
    pub fn get_balance(&self, address: String, struct_type: Option<String>) -> Result<i64> {
        let tag = parse_optional_type_tag(struct_type);
        let balance = self
            .sandbox
            .borrow_mut()
            .storage()
            .calculate_balance(parse_address(&address)?, tag);

        Ok(balance as i64)
    }

    #[napi]
    pub fn get_coins(&self, address: String, struct_type: Option<String>) -> Result<String> {
        let tag = parse_optional_type_tag(struct_type);
        let address = parse_address(&address)?;
        let coins = self.sandbox.borrow_mut().storage().get_coins(address, tag);

        to_json!(coins)
    }
}
