use napi::bindgen_prelude::*;
use napi_derive::napi;
use sui_json_rpc_types::{DynamicFieldPage, SuiObjectDataOptions, SuiObjectResponse};
use sui_types::{
    base_types::ObjectID,
    object::{ObjectRead, Owner},
};

use crate::{
    dynamic_utils::dynamic_field_info,
    to_json,
    types::{GetDynamicFieldObjectParams, GetDynamicFieldsParams, TryGetPastObjectParams},
    utils::{deserialize_json, parse_object_id},
    SharedState,
};

#[napi]
pub struct ObjectApi {
    sandbox: SharedState,
}

#[napi]
impl ObjectApi {
    pub fn new(sandbox: SharedState) -> Self {
        Self { sandbox }
    }

    #[napi]
    pub fn create(&self, object: String) -> Result<()> {
        self.sandbox
            .borrow_mut()
            .create_object(deserialize_json(&object)?);

        Ok(())
    }

    #[napi]
    pub fn get(&self, object_id: String) -> Result<String> {
        let object = self
            .sandbox
            .borrow()
            .get_object(parse_object_id(&object_id)?);
        let response = SuiObjectResponse::try_from((object, SuiObjectDataOptions::full_content()))
            .map_err(|e| {
                Error::from_reason(format!("Failed to construct object response: {}", e))
            })?;

        to_json!(response)
    }

    #[napi]
    pub fn get_past(&self, input: String) -> Result<String> {
        let input: TryGetPastObjectParams = serde_json::from_str(&input)
            .map_err(|e| Error::from_reason(format!("Failed to parse input: {}", e)))?;

        let read = self
            .sandbox
            .borrow()
            .storage()
            .get_object_at_version(&input.id, input.version)
            .map_err(|e| Error::from_reason(format!("{}", e)))?;

        to_json!(read)
    }

    #[napi]
    pub fn get_dynamic_fields(&self, input: String) -> Result<String> {
        let input: GetDynamicFieldsParams = serde_json::from_str(&input)
            .map_err(|e| Error::from_reason(format!("Error parsing query: {}", e)))?;

        let sandbox = self.sandbox.borrow();
        let data = sandbox
            .storage()
            .objects_for(&Owner::ObjectOwner(input.parent_id.into()))
            .map(|object| {
                dynamic_field_info(object.clone(), sandbox.storage())
                    .map_err(|e| Error::from_reason(format!("Error getting dynamic fields: {}", e)))
            })
            .collect::<Result<_, _>>()?;

        to_json!(DynamicFieldPage {
            data,
            has_next_page: false,
            next_cursor: None,
        })
    }

    #[napi]
    pub fn get_dynamic_field_object(&self, input: String) -> Result<String> {
        let input: GetDynamicFieldObjectParams = serde_json::from_str(&input)
            .map_err(|e| Error::from_reason(format!("Error parsing query: {}", e)))?;

        let sandbox = self.sandbox.borrow();
        let data: Vec<_> = sandbox
            .storage()
            .objects_for(&Owner::ObjectOwner(input.parent_id.into()))
            .map(|object| {
                Ok((
                    object,
                    dynamic_field_info(object.clone(), sandbox.storage()).map_err(|e| {
                        Error::from_reason(format!("Error getting dynamic fields: {}", e))
                    })?,
                ))
            })
            .collect::<Result<_, _>>()?;

        let object = data
            .into_iter()
            .find(|(_, dfi)| {
                dfi.name.type_ == input.name.type_ && dfi.name.value == input.name.value
            })
            .map(|(object, _)| object);

        let read = match object {
            Some(obj) => ObjectRead::Exists(
                (
                    obj.id(),
                    obj.as_inner().compute_full_object_reference().1,
                    obj.digest(),
                ),
                obj.clone(),
                obj.get_layout(sandbox.storage().as_inner()).unwrap(),
            ),
            None => ObjectRead::NotExists(ObjectID::random()),
        };

        let response = SuiObjectResponse::try_from((read, SuiObjectDataOptions::full_content()))
            .map_err(|e| {
                Error::from_reason(format!("Failed to construct object response: {}", e))
            })?;

        to_json!(response)
    }
}
