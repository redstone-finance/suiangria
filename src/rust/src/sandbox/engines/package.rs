use anyhow::anyhow;
use std::ops::{Deref, DerefMut};
use sui_types::{
    base_types::{ObjectID, SuiAddress},
    gas_coin::MIST_PER_SUI,
    programmable_transaction_builder::ProgrammableTransactionBuilder,
    transaction::TransactionData,
};

use move_binary_format::{
    normalized::{Module, NoPool},
    CompiledModule,
};
use move_core_types::language_storage::ModuleId;

use sui_json_rpc_types::{SuiMoveNormalizedFunction, SuiTransactionBlockResponse};

use crate::{
    sandbox::{AuthMode, CoinExtension, MoveVMSandbox},
    utils::{parse_account_address, parse_identifier, parse_object_id},
};

const OVERLY_SAFE_SUI_BALANCE: u64 = 1_000 * MIST_PER_SUI;

pub struct PackageEngine<S> {
    sandbox: S,
}

impl<S> PackageEngine<S> {
    pub fn new(storage: S) -> Self {
        Self { sandbox: storage }
    }
}

impl<S> PackageEngine<S>
where
    S: Deref<Target = MoveVMSandbox>,
{
    pub fn get_normalized_move_function(
        &self,
        package_id: String,
        module_id: String,
        function_id: String,
    ) -> anyhow::Result<SuiMoveNormalizedFunction> {
        let object_id = parse_object_id(&package_id)?;
        let package = self
            .sandbox
            .storage()
            .get_object(&object_id)
            .ok_or(anyhow!("No object: {package_id}"))?;

        let package = package
            .as_inner()
            .data
            .try_as_package()
            .ok_or(anyhow!("Object {} is not a package", package.id()))?;

        let module = package
            .get_module(&ModuleId::new(
                parse_account_address(&package_id)?,
                parse_identifier(&module_id)?,
            ))
            .ok_or(anyhow!("Module {module_id} not found in package"))?;

        let module = CompiledModule::deserialize_with_defaults(module)?;
        let module = Module::new(&mut NoPool, &module, false);

        let function = module
            .functions
            .get(&parse_identifier(&function_id)?)
            .ok_or(anyhow!("Function {function_id} not found in module"))?;

        Ok(SuiMoveNormalizedFunction::from(&**function))
    }
}

impl<S> PackageEngine<S>
where
    S: DerefMut<Target = MoveVMSandbox>,
{
    pub fn publish_package(
        &mut self,
        sender: SuiAddress,
        modules: Vec<Vec<u8>>,
        dep_ids: Vec<ObjectID>,
    ) -> anyhow::Result<SuiTransactionBlockResponse> {
        let tx = {
            let mut builder = ProgrammableTransactionBuilder::new();

            let upgrade_cap = builder.publish_upgradeable(modules, dep_ids);
            builder.transfer_arg(sender, upgrade_cap);

            let pt = builder.finish();
            let balance = self.sandbox.storage().calculate_balance(sender, None);

            if balance < OVERLY_SAFE_SUI_BALANCE {
                self.sandbox
                    .storage_mut()
                    .mint_gas_coin(sender, OVERLY_SAFE_SUI_BALANCE);
            }

            let balance = self.sandbox.storage().calculate_balance(sender, None);
            let payment = self.sandbox.storage().get_default_gas_payment(sender);

            TransactionData::new_programmable(
                sender,
                payment,
                pt,
                balance,
                self.sandbox.reference_price,
            )
        };

        self.sandbox.execute_with_auth_override(
            |this| this.transaction_mut().execute_function(tx, vec![]),
            AuthMode::Disabled,
        )
    }
}
