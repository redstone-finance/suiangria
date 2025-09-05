use std::cell::RefCell;
use std::rc::Rc;

use crate::{
    api::{
        behaviour::BehaviourApi, clock::ClockApi, coin::CoinApi, object::ObjectApi,
        package::PackageApi, state::StateApi, storage::StorageApi, transaction::TransactionApi,
    },
    sandbox::{MoveVMSandbox, SandboxBuilder},
};

use napi::bindgen_prelude::*;
use napi_derive::napi;

mod api;
mod dynamic_utils;
mod sandbox;
mod types;
mod utils;

// Using refcell is safe because js is single threaded,
// and we only have a sync code here.
type SharedState = Rc<RefCell<MoveVMSandbox>>;

#[napi]
pub struct SuiSandbox {
    sandbox: SharedState,
}

#[napi]
impl SuiSandbox {
    #[napi(constructor)]
    pub fn new() -> Result<Self> {
        Ok(Self {
            sandbox: Rc::new(RefCell::new(SandboxBuilder::default().build().map_err(
                |e| Error::from_reason(format!("Failed to build sandbox: {}", e)),
            )?)),
        })
    }

    #[napi]
    pub fn clock_api(&self) -> ClockApi {
        ClockApi::new(self.sandbox.clone())
    }

    #[napi]
    pub fn object_api(&self) -> ObjectApi {
        ObjectApi::new(self.sandbox.clone())
    }

    #[napi]
    pub fn transaction_api(&self) -> TransactionApi {
        TransactionApi::new(self.sandbox.clone())
    }

    #[napi]
    pub fn coin_api(&self) -> CoinApi {
        CoinApi::new(self.sandbox.clone())
    }

    #[napi]
    pub fn package_api(&self) -> PackageApi {
        PackageApi::new(self.sandbox.clone())
    }

    #[napi]
    pub fn behaviour_api(&self) -> BehaviourApi {
        BehaviourApi::new(self.sandbox.clone())
    }

    #[napi]
    pub fn state_api(&self) -> StateApi {
        StateApi::new(self.sandbox.clone())
    }

    #[napi]
    pub fn storage_api(&self) -> StorageApi {
        StorageApi::new(self.sandbox.clone())
    }
}
