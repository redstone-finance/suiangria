use std::sync::Arc;

use sui_json_rpc_types::{
    BalanceChange, ObjectChange, SuiTransactionBlockEvents, SuiTransactionBlockResponse,
};
use sui_types::{
    crypto::Signature,
    effects::TransactionEffects,
    error::ExecutionError,
    gas::SuiGasStatus,
    inner_temporary_store::InnerTemporaryStore,
    metrics::LimitsMetrics,
    signature::GenericSignature,
    transaction::{
        CheckedInputObjects, SenderSignedData, Transaction, TransactionData, TransactionDataAPI,
    },
};

use crate::{
    sandbox::{
        extensions::changes::Changes,
        storage::StorageExtension,
        transaction_pipeline::{PipelineResult, TransactionStage},
        MoveVMSandbox,
    },
    utils::response_from_errors,
};

pub struct ValidationInput {
    pub tx_data: TransactionData,
    pub signatures: Vec<Signature>,
}

pub struct ExecutionInput {
    pub tx_data: TransactionData,
    pub transaction: Transaction,
    pub checked_objects: CheckedInputObjects,
    pub gas_status: SuiGasStatus,
}

pub struct EffectsInput {
    pub tx_data: TransactionData,
    pub transaction: Transaction,
    pub temporary_store: InnerTemporaryStore,
    pub effects: TransactionEffects,
    pub execution_result: Result<(), ExecutionError>,
}

pub struct StorageInput {
    pub tx_data: TransactionData,
    pub transaction: Transaction,
    pub temporary_store: InnerTemporaryStore,
    pub effects: TransactionEffects,
    pub events: SuiTransactionBlockEvents,
    pub object_changes: Vec<ObjectChange>,
    pub balance_changes: Vec<BalanceChange>,
    pub execution_result: Result<(), ExecutionError>,
}

pub struct TransactionOutput {
    pub tx_data: TransactionData,
    pub transaction: Transaction,
    pub effects: TransactionEffects,
    pub events: SuiTransactionBlockEvents,
    pub object_changes: Vec<ObjectChange>,
    pub balance_changes: Vec<BalanceChange>,
    pub execution_result: Result<(), ExecutionError>,
}

#[derive(Debug)]
pub struct DryRunOutput {
    pub tx_data: TransactionData,
    pub effects: TransactionEffects,
    pub events: SuiTransactionBlockEvents,
    pub object_changes: Vec<ObjectChange>,
    pub balance_changes: Vec<BalanceChange>,
    pub execution_result: Result<(), ExecutionError>,
}

pub struct ValidationStage;

impl ValidationStage {
    fn create_error_response(
        &self,
        tx_data: &TransactionData,
        transaction: Transaction,
        error: String,
        storage: &StorageExtension,
    ) -> anyhow::Result<SuiTransactionBlockResponse> {
        response_from_errors(
            tx_data,
            transaction.data().clone(),
            vec![error],
            storage.as_inner(),
        )
    }
}

impl TransactionStage for ValidationStage {
    type Input = ValidationInput;
    type Output = ExecutionInput;

    fn execute(
        &self,
        input: Self::Input,
        sandbox: &mut MoveVMSandbox,
    ) -> anyhow::Result<PipelineResult<Self::Output>> {
        let transaction = Transaction::new(SenderSignedData::new(
            input.tx_data.clone(),
            input
                .signatures
                .iter()
                .cloned()
                .map(GenericSignature::Signature)
                .collect(),
        ));

        if let Some(response) = sandbox.transaction_control.reject(|reason| {
            self.create_error_response(
                &input.tx_data,
                transaction.clone(),
                reason,
                &sandbox.storage,
            )
        }) {
            let response = response?;

            return Ok(PipelineResult::EarlyReturn(response));
        }

        if let Err(e) = sandbox
            .auth_extension
            .verify_transaction(&transaction, sandbox.epoch)
        {
            let response = self.create_error_response(
                &input.tx_data,
                transaction,
                e.to_string(),
                &sandbox.storage,
            )?;

            return Ok(PipelineResult::EarlyReturn(response));
        }

        match sandbox.storage.get_checked_objects(
            &input.tx_data,
            Some(&input.signatures),
            &sandbox.auth_extension,
        ) {
            Ok(checked) => {
                let gas_status = SuiGasStatus::new(
                    input.tx_data.gas_data().budget,
                    input.tx_data.gas_data().price,
                    sandbox.reference_price,
                    &sandbox.config,
                )?;

                Ok(PipelineResult::Continue(ExecutionInput {
                    tx_data: input.tx_data,
                    transaction,
                    checked_objects: checked,
                    gas_status,
                }))
            }
            Err(e) => {
                let response = self.create_error_response(
                    &input.tx_data,
                    transaction,
                    e.to_string(),
                    &sandbox.storage,
                )?;
                Ok(PipelineResult::EarlyReturn(response))
            }
        }
    }
}

pub struct ExecutionStage;

impl TransactionStage for ExecutionStage {
    type Input = ExecutionInput;
    type Output = EffectsInput;

    fn execute(
        &self,
        input: Self::Input,
        sandbox: &mut MoveVMSandbox,
    ) -> anyhow::Result<PipelineResult<Self::Output>> {
        let (temporary_store, _, effects, _, execution_result) =
            sandbox.executor.execute_transaction_to_effects(
                &sandbox.storage.as_inner(),
                &sandbox.config,
                Arc::new(LimitsMetrics::new(&Default::default())),
                false,
                Ok(()),
                &sandbox.epoch,
                sandbox.clock().get_time(),
                input.checked_objects,
                input.tx_data.gas_data().clone(),
                input.gas_status,
                input.tx_data.kind().clone(),
                input.tx_data.sender(),
                input.tx_data.digest(),
                &mut None,
            );

        Ok(PipelineResult::Continue(EffectsInput {
            tx_data: input.tx_data,
            transaction: input.transaction,
            temporary_store,
            effects,
            execution_result,
        }))
    }
}

pub struct EffectsStage;

impl TransactionStage for EffectsStage {
    type Input = EffectsInput;
    type Output = StorageInput;

    fn execute(
        &self,
        input: Self::Input,
        sandbox: &mut MoveVMSandbox,
    ) -> anyhow::Result<PipelineResult<Self::Output>> {
        let events = SuiTransactionBlockEvents::try_from_using_module_resolver(
            input.temporary_store.events.clone(),
            input.tx_data.digest(),
            Some(sandbox.clock().get_time()),
            sandbox.storage.as_inner(),
        )?;

        let object_changes = sandbox.storage.compute_object_changes(
            &input.temporary_store,
            &input.effects,
            input.tx_data.sender(),
        );

        let balance_changes = sandbox
            .storage()
            .extract_balance_changes(&input.effects, &input.temporary_store);

        Ok(PipelineResult::Continue(StorageInput {
            tx_data: input.tx_data,
            transaction: input.transaction,
            temporary_store: input.temporary_store,
            effects: input.effects,
            events,
            object_changes,
            balance_changes,
            execution_result: input.execution_result,
        }))
    }
}

pub struct StorageStage;

impl TransactionStage for StorageStage {
    type Input = StorageInput;
    type Output = TransactionOutput;

    fn execute(
        &self,
        input: Self::Input,
        sandbox: &mut MoveVMSandbox,
    ) -> anyhow::Result<PipelineResult<Self::Output>> {
        sandbox.storage.apply_transaction_effects(
            input.transaction.data().transaction_data(),
            &input.object_changes,
            &input.temporary_store,
        );

        Ok(PipelineResult::Continue(TransactionOutput {
            tx_data: input.tx_data,
            transaction: input.transaction,
            effects: input.effects,
            events: input.events,
            object_changes: input.object_changes,
            balance_changes: input.balance_changes,
            execution_result: input.execution_result,
        }))
    }
}

pub struct DryRunStage;

impl TransactionStage for DryRunStage {
    type Input = EffectsInput;
    type Output = DryRunOutput;

    fn execute(
        &self,
        input: Self::Input,
        sandbox: &mut MoveVMSandbox,
    ) -> anyhow::Result<PipelineResult<Self::Output>> {
        let events = SuiTransactionBlockEvents::try_from_using_module_resolver(
            input.temporary_store.events.clone(),
            input.tx_data.digest(),
            Some(sandbox.clock().get_time()),
            sandbox.storage.as_inner(),
        )?;

        let object_changes = sandbox.storage().compute_object_changes(
            &input.temporary_store,
            &input.effects,
            input.tx_data.sender(),
        );

        let balance_changes = sandbox
            .storage()
            .extract_balance_changes(&input.effects, &input.temporary_store);

        Ok(PipelineResult::Continue(DryRunOutput {
            tx_data: input.tx_data,
            effects: input.effects,
            events,
            object_changes,
            balance_changes,
            execution_result: input.execution_result,
        }))
    }
}
