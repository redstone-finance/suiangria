use sui_execution::executor;
use sui_framework::BuiltInFramework;
use sui_types::{
    in_memory_storage::InMemoryStorage, object::Object, supported_protocol_versions::ProtocolConfig,
};

use crate::sandbox::{
    extensions::auth_extension::AuthExtension, storage::StorageExtension, MoveVMSandbox,
};

pub struct SandboxBuilder {
    protocol_config: Option<ProtocolConfig>,
    gas_price: u64,
    enable_auth: bool,
    initial_time_ms: Option<u64>,
    genesis_objects: Vec<Object>,
}

impl Default for SandboxBuilder {
    fn default() -> Self {
        Self {
            protocol_config: None,
            gas_price: 10,
            enable_auth: true,
            initial_time_ms: None,
            genesis_objects: Vec::new(),
        }
    }
}

impl SandboxBuilder {
    pub fn build(self) -> anyhow::Result<MoveVMSandbox> {
        let config = self
            .protocol_config
            .unwrap_or_else(ProtocolConfig::get_for_max_version_UNSAFE);

        let all_genesis_objects = BuiltInFramework::genesis_objects().chain(self.genesis_objects);

        let storage = StorageExtension::new(InMemoryStorage::new(all_genesis_objects.collect()));

        let mut sandbox = MoveVMSandbox {
            executor: executor(&config, false, None)?,
            config,
            storage,
            epoch: 0,
            auth_extension: AuthExtension::new(),
            reference_price: self.gas_price,
            transaction_control: Default::default(),
        };

        if !self.enable_auth {
            sandbox.disable_signature_checks();
        }

        if let Some(time_ms) = self.initial_time_ms {
            sandbox.clock_mut().set_time(time_ms);
        }

        sandbox.init()?;

        Ok(sandbox)
    }
}
