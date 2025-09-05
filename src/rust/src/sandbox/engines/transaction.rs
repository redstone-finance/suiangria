use std::ops::{Deref, DerefMut};

use sui_json_rpc_types::{
    BalanceChange, DryRunTransactionBlockResponse, ObjectChange, SuiTransactionBlock,
    SuiTransactionBlockData, SuiTransactionBlockEffects, SuiTransactionBlockEvents,
    SuiTransactionBlockResponse,
};
use sui_types::{
    base_types::ObjectID,
    crypto::Signature,
    effects::TransactionEffects,
    transaction::{Transaction, TransactionData, TransactionDataAPI},
};

use crate::{
    sandbox::{
        extensions::auth_extension::AuthMode,
        transaction_pipeline::{
            stages::{
                DryRunStage, EffectsStage, ExecutionStage, StorageStage, ValidationInput,
                ValidationStage,
            },
            Pipeline, PipelineResult, TransactionStage,
        },
        CoinExtension, MoveVMSandbox,
    },
    utils::serialize_bcs,
};

pub struct TransactionEngine<S> {
    sandbox: S,
}

impl<S> TransactionEngine<S> {
    pub fn new(storage: S) -> Self {
        Self { sandbox: storage }
    }
}

impl<S> TransactionEngine<S>
where
    S: Deref<Target = MoveVMSandbox>,
{
    fn convert_error_response_to_dry_run(
        &self,
        response: SuiTransactionBlockResponse,
        tx_data: &TransactionData,
    ) -> anyhow::Result<DryRunTransactionBlockResponse> {
        Ok(DryRunTransactionBlockResponse {
            effects: response
                .effects
                .unwrap_or_else(|| TransactionEffects::default().try_into().unwrap()),
            events: response.events.unwrap_or_default(),
            object_changes: response.object_changes.unwrap_or_default(),
            balance_changes: response.balance_changes.unwrap_or_default(),
            input: SuiTransactionBlockData::try_from_with_module_cache(
                tx_data.clone(),
                self.sandbox.storage.as_inner(),
            )?,
            execution_error_source: response.errors.first().cloned(),
            suggested_gas_price: Some(self.sandbox.reference_price),
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn create_transaction_response(
        &self,
        tx_data: &TransactionData,
        transaction: Option<Transaction>,
        effects: TransactionEffects,
        events: SuiTransactionBlockEvents,
        object_changes: Vec<ObjectChange>,
        balance_changes: Vec<BalanceChange>,
        errors: Vec<String>,
    ) -> anyhow::Result<SuiTransactionBlockResponse> {
        let response = SuiTransactionBlockResponse {
            digest: tx_data.digest(),
            transaction: transaction
                .map(|t| {
                    SuiTransactionBlock::try_from(
                        t.data().clone(),
                        self.sandbox.storage().as_inner(),
                    )
                })
                .transpose()?,
            raw_transaction: serialize_bcs(tx_data)?,
            effects: Some(effects.clone().try_into()?),
            events: Some(events),
            object_changes: Some(object_changes),
            balance_changes: Some(balance_changes),
            timestamp_ms: Some(self.sandbox.clock().get_time()),
            confirmed_local_execution: Some(true),
            checkpoint: Some(self.sandbox.storage().checkpoint()),
            errors,
            raw_effects: serialize_bcs(&effects)?,
        };

        Ok(response)
    }
}

impl<S> TransactionEngine<S>
where
    S: DerefMut<Target = MoveVMSandbox>,
{
    pub fn execute_function(
        &mut self,
        tx_data: TransactionData,
        signatures: Vec<Signature>,
    ) -> anyhow::Result<SuiTransactionBlockResponse> {
        let pipeline = Pipeline::new(ValidationStage)
            .then(ExecutionStage)
            .then(EffectsStage)
            .then(StorageStage);

        let input = ValidationInput {
            tx_data: tx_data.clone(),
            signatures,
        };

        let response = match pipeline.execute(input, &mut self.sandbox)? {
            PipelineResult::Continue(output) => self.create_transaction_response(
                &output.tx_data,
                Some(output.transaction),
                output.effects,
                output.events,
                output.object_changes,
                output.balance_changes,
                output
                    .execution_result
                    .err()
                    .map(|e| vec![e.to_string()])
                    .unwrap_or_default(),
            ),
            PipelineResult::EarlyReturn(response) => Ok(response),
        }?;

        self.sandbox
            .storage
            .insert_transaction(response.digest, response.clone());

        Ok(response)
    }

    pub fn dry_run_transaction(
        &mut self,
        mut tx_data: TransactionData,
    ) -> anyhow::Result<DryRunTransactionBlockResponse> {
        let pipeline = Pipeline::new(ValidationStage)
            .then(ExecutionStage)
            .then(DryRunStage);

        let coin_to_delete = if tx_data.gas_data().payment.is_empty() {
            let mut gas_coins = self
                .sandbox
                .storage()
                .get_default_gas_payment(tx_data.sender());

            let sender_balance = self
                .sandbox
                .storage
                .calculate_balance(tx_data.sender(), None);

            let coin_for_dry_run = self
                .sandbox
                .storage_mut()
                .mint_gas_coin(tx_data.sender(), tx_data.gas_budget() + sender_balance);

            let reference = self
                .sandbox
                .storage
                .get_object(&coin_for_dry_run)
                .unwrap()
                .compute_object_reference();

            gas_coins.push(reference);

            tx_data.gas_data_mut().payment = vec![reference];

            Some(coin_for_dry_run)
        } else {
            None
        };

        let input = ValidationInput {
            tx_data: tx_data.clone(),
            signatures: vec![],
        };

        let execution_result = self.sandbox.execute_with_auth_override(
            |sandbox| pipeline.execute(input, sandbox),
            AuthMode::Disabled,
        );

        if let Some(id) = coin_to_delete {
            self.sandbox.storage_mut().remove_object_without_trace(id);
        }

        let response = match execution_result? {
            PipelineResult::Continue(output) => DryRunTransactionBlockResponse {
                effects: output.effects.try_into()?,
                events: output.events,
                object_changes: output.object_changes,
                balance_changes: output.balance_changes,
                input: SuiTransactionBlockData::try_from_with_module_cache(
                    output.tx_data,
                    self.sandbox.storage.as_inner(),
                )?,
                execution_error_source: output
                    .execution_result
                    .err()
                    .and_then(|e| e.source().as_ref().map(|s| s.to_string())),
                suggested_gas_price: Some(self.sandbox.reference_price),
            },
            PipelineResult::EarlyReturn(response) => {
                self.convert_error_response_to_dry_run(response, &tx_data)?
            }
        };

        Ok(response)
    }
}

fn _filter_temp_object_from_response(
    mut response: DryRunTransactionBlockResponse,
    temp_object_id: Option<ObjectID>,
) -> DryRunTransactionBlockResponse {
    let temp_object_id = match temp_object_id {
        Some(id) => id,
        None => return response,
    };

    response.object_changes.retain(|change| {
        !matches!(change,
            ObjectChange::Created { object_id, .. } |
            ObjectChange::Mutated { object_id, .. } |
            ObjectChange::Deleted { object_id, .. }
            if *object_id == temp_object_id
        )
    });

    let SuiTransactionBlockEffects::V1(effects) = &mut response.effects;

    effects
        .created
        .retain(|o| o.reference.object_id != temp_object_id);
    effects
        .mutated
        .retain(|o| o.reference.object_id != temp_object_id);
    effects.deleted.retain(|o| o.object_id != temp_object_id);

    if effects.gas_object.reference.object_id == temp_object_id {
        effects.gas_object = effects
            .mutated
            .first()
            .cloned()
            .unwrap_or_else(|| effects.gas_object.clone());
    }

    response
}
