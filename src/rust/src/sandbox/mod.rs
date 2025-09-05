use move_core_types::ident_str;

use std::sync::Arc;
use sui_execution::Executor;

use sui_types::{
    base_types::ObjectID,
    committee::EpochId,
    digests::TransactionDigest,
    metrics::LimitsMetrics,
    object::{Object, ObjectRead},
    programmable_transaction_builder::ProgrammableTransactionBuilder,
    supported_protocol_versions::ProtocolConfig,
    transaction::CheckedInputObjects,
    SUI_FRAMEWORK_ADDRESS,
};

mod builder;
mod engines;
mod extensions;
mod storage;
mod transaction_pipeline;

pub use crate::sandbox::extensions::{
    auth_extension::{AuthExtension, AuthMode},
    coins::CoinExtension,
    transaction_control::TransactionControlExtension,
};
pub use builder::SandboxBuilder;
pub use storage::StorageExtension;

use crate::sandbox::engines::{
    clock::ClockEngine, package::PackageEngine, transaction::TransactionEngine,
};

pub struct MoveVMSandbox {
    config: ProtocolConfig,
    executor: Arc<dyn Executor + Send + Sync>,
    storage: StorageExtension,
    epoch: EpochId,
    auth_extension: AuthExtension,
    reference_price: u64,
    transaction_control: TransactionControlExtension,
}

impl MoveVMSandbox {
    pub fn init(&mut self) -> anyhow::Result<()> {
        let mut builder = ProgrammableTransactionBuilder::new();

        builder.programmable_move_call(
            SUI_FRAMEWORK_ADDRESS.into(),
            ident_str!("object").to_owned(),
            ident_str!("sui_system_state").to_owned(),
            vec![],
            vec![],
        );

        builder.move_call(
            SUI_FRAMEWORK_ADDRESS.into(),
            ident_str!("clock").to_owned(),
            ident_str!("create").to_owned(),
            vec![],
            vec![],
        )?;

        let pt = builder.finish();

        let storage = self.executor.update_genesis_state(
            self.storage.as_inner(),
            &self.config,
            Arc::new(LimitsMetrics::new(&Default::default())),
            self.epoch,
            0,
            &TransactionDigest::genesis_marker(),
            CheckedInputObjects::new_for_genesis(vec![]),
            pt,
        )?;
        self.storage.finish(storage.written);

        Ok(())
    }

    pub fn clock(&self) -> ClockEngine<&StorageExtension> {
        ClockEngine::new(&self.storage)
    }

    pub fn clock_mut(&mut self) -> ClockEngine<&mut StorageExtension> {
        ClockEngine::new(&mut self.storage)
    }

    pub fn transaction(&self) -> TransactionEngine<&MoveVMSandbox> {
        TransactionEngine::new(self)
    }

    pub fn transaction_mut(&mut self) -> TransactionEngine<&mut MoveVMSandbox> {
        TransactionEngine::new(self)
    }

    pub fn package(&self) -> PackageEngine<&MoveVMSandbox> {
        PackageEngine::new(self)
    }

    pub fn package_mut(&mut self) -> PackageEngine<&mut MoveVMSandbox> {
        PackageEngine::new(self)
    }

    pub fn storage(&self) -> &StorageExtension {
        &self.storage
    }

    pub fn storage_mut(&mut self) -> &mut StorageExtension {
        &mut self.storage
    }

    pub fn get_object(&self, id: ObjectID) -> ObjectRead {
        let object = self.storage.get_object(&id);

        match object {
            Some(obj) => ObjectRead::Exists(
                (
                    id,
                    obj.as_inner().compute_full_object_reference().1,
                    obj.digest(),
                ),
                obj.clone(),
                obj.get_layout(self.storage.as_inner()).unwrap(),
            ),
            None => ObjectRead::NotExists(id),
        }
    }

    pub fn create_object(&mut self, object: Object) {
        self.storage.insert_object(object);
    }

    pub fn reject_next_tx(&mut self, reason: String) {
        self.transaction_control.reject_with(reason);
    }

    pub fn disable_signature_checks(&mut self) {
        self.auth_extension.set_mode(AuthMode::Disabled);
    }

    pub fn enable_signature_checks(&mut self) {
        self.auth_extension.set_mode(AuthMode::Enabled);
    }

    fn execute_with_auth_override<T, F: FnOnce(&mut Self) -> T>(
        &mut self,
        action: F,
        mode: AuthMode,
    ) -> T {
        let prev = self.auth_extension.set_mode(mode);

        let out = action(self);

        self.auth_extension.set_mode(prev);

        out
    }

    pub fn gas_price(&self) -> u64 {
        self.reference_price
    }
}
