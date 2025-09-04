use napi_derive::napi;

use crate::SharedState;

#[napi]
pub struct BehaviourApi {
    sandbox: SharedState,
}

#[napi]
impl BehaviourApi {
    pub fn new(sandbox: SharedState) -> Self {
        Self { sandbox }
    }

    #[napi]
    pub fn disable_signature_checks(&self) {
        self.sandbox.borrow_mut().disable_signature_checks();
    }

    #[napi]
    pub fn enable_signature_checks(&self) {
        self.sandbox.borrow_mut().enable_signature_checks();
    }

    #[napi]
    pub fn set_reject_next_transaction(&self, reason: String) {
        self.sandbox.borrow_mut().reject_next_tx(reason);
    }

    #[napi]
    pub fn bump_checkpoint(&self) {
        self.sandbox.borrow_mut().storage_mut().bump_checkpoint();
    }
}
