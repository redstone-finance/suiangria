import { SuiGrpcClient } from '@mysten/sui/grpc';
import { Signer } from '@mysten/sui/cryptography';
import { Transaction } from '@mysten/sui/transactions';
import { SandboxClient } from './client';

function translateObjectResponse(raw: any) {
  const data = raw?.data;
  if (!data) return { object: null };

  return {
    object: {
      objectId: data.objectId,
      version: data.version,
      digest: data.digest,
      type: data.content?.type,
      json: data.content?.fields,
      ...(data.bcs?.bcsBytes && {
        bcs: Buffer.from(data.bcs.bcsBytes, 'base64'),
      }),
    },
  };
}

function translateTxResponse(raw: any) {
  const failed = raw.effects?.status?.status === 'failure' || (raw.errors?.length ?? 0) > 0;

  if (failed) {
    return {
      $kind: 'FailedTransaction' as const,
      digest: raw.digest,
      FailedTransaction: raw.effects?.status?.error ?? raw.errors?.join('; ') ?? 'Unknown error',
    };
  }

  const objectTypes: Record<string, string> = {};

  const changedObjects = (raw.objectChanges ?? [])
    .filter((c: any) => c.objectId)
    .map((change: any) => {
      if (change.objectType) {
        objectTypes[change.objectId] = change.objectType;
      }

      return {
        objectId: change.objectId,
        inputState: change.type === 'created' ? 'DoesNotExist' : 'Exists',
        outputState: change.type === 'deleted' ? 'DoesNotExist' : 'Exists',
      };
    });

  return {
    $kind: 'Transaction' as const,
    digest: raw.digest,
    Transaction: {
      effects: {
        changedObjects,
        status: raw.effects?.status,
        gasUsed: raw.effects?.gasUsed,
      },
      objectTypes,
      events: raw.events ?? [],
    },
  };
}


export function createSandboxGrpcClient(): { client: SuiGrpcClient; sandbox: SandboxClient } {
  const sandbox = new SandboxClient();

  const core = {
    async getObject({ objectId }: { objectId: string; include?: { json?: boolean; bcs?: boolean } }) {
      const raw = sandbox.getObject({ id: objectId });

      return translateObjectResponse(raw);
    },

    async listDynamicFields({ parentId, limit, cursor }: { parentId: string; limit?: number; cursor?: string }) {
      const raw = sandbox.getDynamicFields({ parentId, cursor: cursor ?? null, limit: limit ?? 50 });

      return { dynamicFields: raw.data, cursor: raw.nextCursor ?? undefined };
    },

    async getDynamicField({
      parentId,
      name,
    }: {
      parentId: string;
      name: { type: string; bcs: Uint8Array };
    }) {
      const valueBcs = sandbox.objectApi().getDynamicFieldValueBcs(parentId, Buffer.from(name.bcs));

      return {
        dynamicField: {
          value: {
            bcs: valueBcs,
          },
        },
      };
    },

    async getCurrentSystemState() {
      return {
        systemState: {
          epoch: '0',
          referenceGasPrice: String(sandbox.stateApi().getReferenceGasPrice()),
        },
      };
    },

    resolveTransactionPlugin() {
      return undefined;
    },

    async listOwnedObjects(_x: { owner: string; objectType?: string }) {
      throw new Error('listOwnedObjects not yet supported in sandbox');
    },
  };

  const executeTx = (txBase64: string, sigs: string[]) => {
    const raw = sandbox.executeTransactionBlock({ transactionBlock: txBase64, signature: sigs });

    return translateTxResponse(raw);
  };

  const topLevel = {
    core,

    async signAndExecuteTransaction({
      transaction,
      signer,
    }: {
      transaction: Uint8Array | Transaction;
      signer: Signer;
      include?: Record<string, boolean>;
    }) {
      let txBytes: Uint8Array;

      if (transaction instanceof Uint8Array) {
        txBytes = transaction;
      } else {
        transaction.setSenderIfNotSet(signer.toSuiAddress());
        txBytes = await transaction.build({ client: client as SuiGrpcClient });
      }

      const { signature, bytes } = await signer.signTransaction(txBytes);

      return executeTx(bytes, [signature]);
    },

    async waitForTransaction({ digest }: { digest: string }) {
      const raw = sandbox.getTransaction(digest);

      return translateTxResponse(raw);
    },

    async getCoins({ owner, coinType }: { owner: string; coinType?: string }) {
      return sandbox.getCoins(owner, coinType);
    },

    async getBalance({ owner, coinType }: { owner: string; coinType?: string }) {
      return {
        totalBalance: String(sandbox.getBalance(owner, coinType)),
        coinType: coinType ?? '0x2::sui::SUI',
        coinObjectCount: 1,
      };
    },

    async getReferenceGasPrice() {
      return BigInt(sandbox.stateApi().getReferenceGasPrice());
    },

    async dryRunTransaction({ transactionBlock }: { transactionBlock: Uint8Array | string }) {
      const txBase64 =
        typeof transactionBlock === 'string' ? transactionBlock : Buffer.from(transactionBlock).toString('base64');

      return sandbox.dryRunTransaction(txBase64);
    },

    async getNormalizedMoveFunction(params: { package: string; module: string; function: string }) {
      return sandbox.getNormalizedFunction(params);
    },
  };

  const client = new Proxy({} as SuiGrpcClient, {
    get(_, prop) {
      const key = prop as string;

      if (key in topLevel) {
        return topLevel[key as keyof typeof topLevel];
      }

      return (...args: unknown[]) => {
        throw new Error(`Method ${String(prop)}(${JSON.stringify(args)}) not yet supported in sandbox`);
      };
    },
  });

  return { client, sandbox };
}