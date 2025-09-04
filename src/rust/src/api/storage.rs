use napi::bindgen_prelude::*;
use napi_derive::napi;

use crate::SharedState;

#[napi]
pub struct StorageApi {
    sandbox: SharedState,
}

#[napi]
impl StorageApi {
    pub fn new(sandbox: SharedState) -> Self {
        Self { sandbox }
    }

    #[napi]
    pub fn take_snapshot(&self) -> Result<Vec<u8>> {
        self.sandbox
            .borrow()
            .storage()
            .to_bytes_compressed()
            .map_err(|e| Error::from_reason(format!("Error while creating snapshot: {e}.")))
    }

    #[napi]
    pub fn restore_from_snapshot(&self, snapshot: Vec<u8>) -> Result<()> {
        self.sandbox
            .borrow_mut()
            .storage_mut()
            .restone_from_bytes(&snapshot)
            .map_err(|e| Error::from_reason(format!("Error while restoring snapshot: {e}.")))
    }
}
