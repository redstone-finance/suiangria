import {
  DryRunTransactionBlockParams,
  DryRunTransactionBlockResponse,
  DynamicFieldPage,
  ExecuteTransactionBlockParams,
  GetBalanceParams,
  GetCoinsParams,
  GetDynamicFieldObjectParams,
  GetDynamicFieldsParams,
  GetLatestCheckpointSequenceNumberParams,
  GetNormalizedMoveFunctionParams,
  GetReferenceGasPriceParams,
  GetTransactionBlockParams,
  MultiGetObjectsParams,
  ObjectRead,
  PaginatedCoins,
  PaginatedTransactionResponse,
  QueryTransactionBlocksParams,
  SuiClient,
  SuiMoveNormalizedFunction,
  SuiObjectResponse,
  SuiTransactionBlockResponse,
  TryGetPastObjectParams,
} from '@mysten/sui/client';
import { SandboxClient } from './client';
import { Signer } from '@mysten/sui/cryptography';
import { Transaction } from '@mysten/sui/transactions';

export function createSandboxClient(): { client: SuiClient; sandbox: SandboxClient } {
  const sandbox = new SandboxClient();

  const client = new Proxy({} as SuiClient, {
    get(_, prop) {
      const overrides: Partial<SuiClient> = {
        core: undefined,

        async getBalance(input: GetBalanceParams) {
          return {
            coinObjectCount: 1,
            owner: input.owner,
            totalBalance: String(sandbox.getBalance(input.owner, input?.coinType)),
            lockedBalance: {},
            coinType: input.coinType ?? '0x2::sui::SUI',
          };
        },

        async executeTransactionBlock(input) {
          const result = sandbox.executeTransactionBlock(input);

          return result;
        },

        async dryRunTransactionBlock(input: DryRunTransactionBlockParams): Promise<DryRunTransactionBlockResponse> {
          const txBase64 =
            typeof input.transactionBlock === 'string'
              ? input.transactionBlock
              : Buffer.from(input.transactionBlock).toString('base64');

          const result = sandbox.dryRunTransaction(txBase64);

          return result;
        },

        async getObject(input) {
          return sandbox.getObject(input);
        },

        async multiGetObjects(input: MultiGetObjectsParams): Promise<SuiObjectResponse[]> {
          const objects = Promise.all(input.ids.map((id) => this.getObject!({ id })));

          return objects;
        },

        async signAndExecuteTransaction({
          transaction,
          signer,
          ...input
        }: {
          transaction: Uint8Array | Transaction;
          signer: Signer;
        } & Omit<
          ExecuteTransactionBlockParams,
          'transactionBlock' | 'signature'
        >): Promise<SuiTransactionBlockResponse> {
          let transactionBytes;

          if (transaction instanceof Uint8Array) {
            transactionBytes = transaction;
          } else {
            transaction.setSenderIfNotSet(signer.toSuiAddress());
            transactionBytes = await transaction.build({ client: this as SuiClient });
          }

          const { signature, bytes } = await signer.signTransaction(transactionBytes);

          return this.executeTransactionBlock!({
            transactionBlock: bytes,
            signature,
            ...input,
          });
        },

        async waitForTransaction({
          ...input
        }: {
          signal?: AbortSignal;
          timeout?: number;
          pollInterval?: number;
        } & Parameters<SuiClient['getTransactionBlock']>[0]): Promise<SuiTransactionBlockResponse> {
          return sandbox.getTransaction(input.digest);
        },

        async getNormalizedMoveFunction(params: GetNormalizedMoveFunctionParams): Promise<SuiMoveNormalizedFunction> {
          return sandbox.getNormalizedFunction(params);
        },

        async getReferenceGasPrice(_: GetReferenceGasPriceParams = {}): Promise<bigint> {
          return BigInt(sandbox.stateApi().getReferenceGasPrice());
        },

        async getCoins(params: GetCoinsParams): Promise<PaginatedCoins> {
          const coins = sandbox.getCoins(params.owner, params.coinType);

          return {
            data: coins,
            hasNextPage: false,
          };
        },

        async tryGetPastObject(input: TryGetPastObjectParams): Promise<ObjectRead> {
          return sandbox.tryGetPastObject(input);
        },

        async getDynamicFields(input: GetDynamicFieldsParams): Promise<DynamicFieldPage> {
          return sandbox.getDynamicFields(input);
        },

        async getDynamicFieldObject(input: GetDynamicFieldObjectParams): Promise<SuiObjectResponse> {
          return sandbox.getDynamicFieldObject(input);
        },

        async queryTransactionBlocks(params: QueryTransactionBlocksParams): Promise<PaginatedTransactionResponse> {
          return sandbox.queryTransactionBlocks(params);
        },

        async getLatestCheckpointSequenceNumber(_: GetLatestCheckpointSequenceNumberParams = {}): Promise<string> {
          return '10';
        },

        async getTransactionBlock(input: GetTransactionBlockParams): Promise<SuiTransactionBlockResponse> {
          return sandbox.getTransaction(input.digest);
        },
      };

      if (prop in overrides) {
        return overrides[prop as keyof SuiClient];
      }

      return (...args: any[]) => {
        throw new Error(`Method ${String(prop)}(${JSON.stringify(args)}) not yet supported`);
      };
    },
  });

  return { client, sandbox };
}
