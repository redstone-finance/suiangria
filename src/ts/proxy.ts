import { SuiGrpcClient } from '@mysten/sui/grpc';
import { Signer } from '@mysten/sui/cryptography';
import { Transaction } from '@mysten/sui/transactions';
import { SandboxClient } from './client';

function translateSignatureBody(param: any): any {
  if (typeof param === 'string') {
    const primitives: Record<string, string> = {
      Bool: 'bool',
      U8: 'u8',
      U16: 'u16',
      U32: 'u32',
      U64: 'u64',
      U128: 'u128',
      U256: 'u256',
      Address: 'address',
    };

    return { $kind: primitives[param] ?? 'unknown' };
  }

  if (param.Vector) {
    return { $kind: 'vector', vector: translateSignatureBody(param.Vector) };
  }

  if (param.Struct) {
    const { address, module, name, typeArguments } = param.Struct;

    return {
      $kind: 'datatype',
      datatype: {
        typeName: `${address}::${module}::${name}`,
        typeParameters: (typeArguments ?? []).map(translateSignatureBody),
      },
    };
  }

  if (param.TypeParameter !== undefined) {
    return { $kind: 'typeParameter', index: param.TypeParameter };
  }

  return { $kind: 'unknown' };
}

function translateParameter(param: any): any {
  if (param.Reference) {
    return { reference: 'immutable', body: translateSignatureBody(param.Reference) };
  }

  if (param.MutableReference) {
    return { reference: 'mutable', body: translateSignatureBody(param.MutableReference) };
  }

  return { reference: null, body: translateSignatureBody(param) };
}

function translateMoveFunction(raw: any, packageId: string, moduleName: string, name: string) {
  return {
    function: {
      packageId,
      moduleName,
      name,
      visibility: raw.visibility?.toLowerCase() ?? 'private',
      isEntry: raw.isEntry ?? false,
      typeParameters: (raw.typeParameters ?? []).map((tp: any) => ({
        constraints: tp.abilities ?? [],
        isPhantom: tp.isPhantom ?? false,
      })),
      parameters: (raw.parameters ?? []).map(translateParameter),
      returns: (raw.return ?? []).map(translateParameter),
    },
  };
}

function flattenMoveJson(value: any): any {
  if (value === null || value === undefined || typeof value !== 'object') return value;

  if (Array.isArray(value)) return value.map(flattenMoveJson);

  if (typeof value.type === 'string' && value.fields && typeof value.fields === 'object') {
    return flattenMoveJson(value.fields);
  }

  return Object.fromEntries(Object.entries(value).map(([k, v]) => [k, flattenMoveJson(v)]));
}

function translateOwner(owner: any) {
  if (!owner) return { $kind: 'Unknown' };

  if (typeof owner === 'string' && owner === 'Immutable') {
    return { $kind: 'Immutable', Immutable: true };
  }

  if (owner.AddressOwner) {
    return { $kind: 'AddressOwner', AddressOwner: owner.AddressOwner };
  }

  if (owner.ObjectOwner) {
    return { $kind: 'ObjectOwner', ObjectOwner: owner.ObjectOwner };
  }

  if (owner.Shared) {
    return {
      $kind: 'Shared',
      Shared: { initialSharedVersion: String(owner.Shared.initial_shared_version) },
    };
  }

  return { $kind: 'Unknown' };
}

function translateObjectResponse(raw: any) {
  const data = raw?.data;
  if (!data) return { object: null };

  return {
    object: {
      objectId: data.objectId,
      version: data.version,
      digest: data.digest,
      type: data.content?.type,
      owner: translateOwner(data.owner),
      json: data.content?.fields ? flattenMoveJson(data.content.fields) : undefined,
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
      FailedTransaction: {
        digest: raw.digest,
        status: {
          success: false,
          error: { message: raw.effects?.status?.error ?? raw.errors?.join('; ') ?? 'Unknown error' },
        },
      },
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
      digest: raw.digest,
      effects: {
        changedObjects,
        status: { success: true, error: null },
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
    async getObjects({ objectIds }: { objectIds: string[]; include?: Record<string, boolean> }) {
      return {
        objects: objectIds.map((id) => {
          const raw = sandbox.getObject({ id });

          return translateObjectResponse(raw).object;
        }),
      };
    },

    async getObject({ objectId }: { objectId: string; include?: { json?: boolean; bcs?: boolean } }) {
      const raw = sandbox.getObject({ id: objectId });

      return translateObjectResponse(raw);
    },

    async listDynamicFields({ parentId, limit, cursor }: { parentId: string; limit?: number; cursor?: string }) {
      const raw = sandbox.getDynamicFields({ parentId, cursor: cursor ?? null, limit: limit ?? 50 });

      return { dynamicFields: raw.data, cursor: raw.nextCursor ?? undefined };
    },

    async getDynamicField({ parentId, name }: { parentId: string; name: { type: string; bcs: Uint8Array } }) {
      const valueBcs = sandbox.objectApi().getDynamicFieldValueBcs(parentId, Buffer.from(name.bcs));

      return {
        dynamicField: {
          value: {
            bcs: valueBcs,
          },
        },
      };
    },

    async getMoveFunction({ packageId, moduleName, name }: { packageId: string; moduleName: string; name: string }) {
      const raw = sandbox.getNormalizedFunction({ package: packageId, module: moduleName, function: name });

      return translateMoveFunction(raw, packageId, moduleName, name);
    },

    async getChainIdentifier() {
      return { chainIdentifier: '11111111111111111111111111111111' };
    },

    async listCoins({ owner, coinType }: { owner: string; coinType?: string }) {
      const raw = sandbox.getCoins(owner, coinType ?? '0x2::sui::SUI');
      const coins = Array.isArray(raw) ? raw : (raw.data ?? []);

      return {
        objects: coins.map((c: any) => ({
          objectId: c.coinObjectId,
          version: c.version,
          digest: c.digest,
          owner: { $kind: 'AddressOwner', AddressOwner: owner },
          type: `0x2::coin::Coin<${c.coinType}>`,
          balance: c.balance,
        })),
        hasNextPage: false,
        cursor: null,
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

    async simulateTransaction({ transaction }: { transaction: Uint8Array }) {
      const txBase64 = Buffer.from(transaction).toString('base64');
      const raw = sandbox.dryRunTransaction(txBase64);

      return {
        $kind: 'Transaction' as const,
        Transaction: {
          effects: {
            gasUsed: raw.effects?.gasUsed,
          },
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
