use sui_json_rpc_types::SuiTransactionBlockResponse;

use crate::sandbox::MoveVMSandbox;

pub mod stages;

#[derive(Debug)]
pub enum PipelineResult<T, E = SuiTransactionBlockResponse> {
    Continue(T),
    EarlyReturn(E),
}

pub trait TransactionStage {
    type Input;
    type Output;

    fn execute(
        &self,
        input: Self::Input,
        sandbox: &mut MoveVMSandbox,
    ) -> anyhow::Result<PipelineResult<Self::Output>>;
}

pub struct Pipeline<S, I, O>
where
    S: TransactionStage<Input = I, Output = O>,
{
    stage: S,
    _phantom: std::marker::PhantomData<(I, O)>,
}

impl<S, I, O> Pipeline<S, I, O>
where
    S: TransactionStage<Input = I, Output = O>,
{
    pub fn new(stage: S) -> Self {
        Self {
            stage,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn then<NextStage, NextOutput>(
        self,
        next: NextStage,
    ) -> Pipeline<ChainedStage<S, NextStage>, I, NextOutput>
    where
        NextStage: TransactionStage<Input = O, Output = NextOutput>,
    {
        Pipeline::new(ChainedStage {
            first: self.stage,
            second: next,
        })
    }
}

pub struct ChainedStage<S1, S2> {
    first: S1,
    second: S2,
}

impl<S1, S2, I, M, O> TransactionStage for ChainedStage<S1, S2>
where
    S1: TransactionStage<Input = I, Output = M>,
    S2: TransactionStage<Input = M, Output = O>,
{
    type Input = I;
    type Output = O;

    fn execute(
        &self,
        input: Self::Input,
        sandbox: &mut MoveVMSandbox,
    ) -> anyhow::Result<PipelineResult<Self::Output>> {
        match self.first.execute(input, sandbox)? {
            PipelineResult::Continue(intermediate) => self.second.execute(intermediate, sandbox),
            PipelineResult::EarlyReturn(response) => Ok(PipelineResult::EarlyReturn(response)),
        }
    }
}

impl<S, I, O> TransactionStage for Pipeline<S, I, O>
where
    S: TransactionStage<Input = I, Output = O>,
{
    type Input = I;
    type Output = O;

    fn execute(
        &self,
        input: Self::Input,
        sandbox: &mut MoveVMSandbox,
    ) -> anyhow::Result<PipelineResult<Self::Output>> {
        self.stage.execute(input, sandbox)
    }
}
