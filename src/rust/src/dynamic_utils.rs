use anyhow::anyhow;
use move_core_types::annotated_value::MoveTypeLayout;
use sui_json_rpc_types::{BcsName, DynamicFieldInfo as DynamicFieldInfoResponse, SuiMoveValue};
use sui_types::{
    dynamic_field::{visitor as DFV, DynamicFieldName},
    object::{bounded_visitor::BoundedVisitor, Object},
    TypeTag,
};

use crate::sandbox::StorageExtension;

// code adjusted from
// https://github.com/MystenLabs/sui/blob/b5eb13baff3d2e155467b030cd1a442b46f51b47/crates/sui-indexer-alt-jsonrpc/src/api/dynamic_fields/response.rs#L209

pub fn dynamic_field_info(
    object: Object,
    storage: &StorageExtension,
) -> anyhow::Result<DynamicFieldInfoResponse> {
    let move_object = object
        .data
        .try_as_move()
        .ok_or(anyhow!("Object not a move-object"))?;

    let layout = move_object.get_layout(storage.as_inner())?;
    let x = MoveTypeLayout::Struct(Box::new(layout));

    let field = DFV::FieldVisitor::deserialize(move_object.contents(), &x)?;

    let type_ = field.kind;
    let name_type: TypeTag = field.name_layout.into();
    let bcs_name = BcsName::Base64 {
        bcs_name: field.name_bytes.to_owned(),
    };

    let name_value = BoundedVisitor::deserialize_value(field.name_bytes, field.name_layout)?;

    let name = DynamicFieldName {
        type_: name_type,
        value: SuiMoveValue::from(name_value).to_json_value(),
    };

    let value_metadata = field.value_metadata()?;

    Ok(match value_metadata {
        DFV::ValueMetadata::DynamicField(object_type) => DynamicFieldInfoResponse {
            name,
            bcs_name,
            type_,
            object_type: object_type.to_canonical_string(true),
            object_id: object.id(),
            version: object.version(),
            digest: object.digest(),
        },

        DFV::ValueMetadata::DynamicObjectField(object_id) => {
            let object = storage.get_object(&object_id).unwrap();

            let object_type = object.data.type_().cloned().unwrap();

            DynamicFieldInfoResponse {
                name,
                bcs_name,
                type_,
                object_type: object_type.to_canonical_string(true),
                object_id: object.id(),
                version: object.version(),
                digest: object.digest(),
            }
        }
    })
}
