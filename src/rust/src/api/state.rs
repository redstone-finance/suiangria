use napi_derive::napi;

use crate::SharedState;

#[napi]
pub struct StateApi {
    sandbox: SharedState,
}

#[napi]
impl StateApi {
    pub fn new(sandbox: SharedState) -> Self {
        Self { sandbox }
    }

    #[napi]
    pub fn get_latest_checkpoint(&self) -> i64 {
        self.sandbox.borrow().storage().checkpoint() as i64
    }

    #[napi]
    pub fn get_reference_gas_price(&self) -> i64 {
        self.sandbox.borrow().gas_price() as i64
    }
}
