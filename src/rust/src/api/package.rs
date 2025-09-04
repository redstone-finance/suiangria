use napi::bindgen_prelude::*;
use napi_derive::napi;

use crate::{
    to_json,
    utils::{parse_address, parse_object_id},
    SharedState,
};
#[napi]
pub struct PackageApi {
    sandbox: SharedState,
}

#[napi]
impl PackageApi {
    pub fn new(sandbox: SharedState) -> Self {
        Self { sandbox }
    }

    #[napi]
    pub fn publish(
        &self,
        modules: Vec<Vec<u8>>,
        dependency_ids: Vec<String>,
        sender: String,
    ) -> Result<String> {
        let sender_address = parse_address(&sender)?;
        let dep_object_ids = dependency_ids
            .iter()
            .map(|id| parse_object_id(id))
            .collect::<Result<Vec<_>, _>>()?;

        let res = self
            .sandbox
            .borrow_mut()
            .package_mut()
            .publish_package(sender_address, modules, dep_object_ids)
            .map_err(|e| Error::from_reason(format!("Publishing package failed: {}", e)))?;

        to_json!(res)
    }

    #[napi]
    pub fn get_normalized_move_function(
        &self,
        package_id: String,
        module: String,
        fun: String,
    ) -> Result<String> {
        let normalized = self
            .sandbox
            .borrow()
            .package()
            .get_normalized_move_function(package_id, module, fun)
            .map_err(|e| Error::from_reason(format!("Failed to get normalized function: {}", e)))?;

        to_json!(normalized)
    }
}
