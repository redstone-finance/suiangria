import { SuiGrpcClient } from '@mysten/sui/grpc';
import { Signer } from '@mysten/sui/cryptography';
import { Transaction } from '@mysten/sui/transactions';
import { SandboxClient } from './client';
import { bcs } from '@mysten/bcs';

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

  const keys = Object.keys(value);
  if (keys.length === 1 && keys[0] === 'id' && typeof value.id === 'string') {
    return value.id;
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
      type: data.content?.type ?? data.type,
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
    .filter((c: any) => c.objectId || c.packageId)
    .map((change: any) => {
      const id = change.objectId ?? change.packageId;

      if (change.objectType) {
        objectTypes[id] = change.objectType;
      }

      return {
        objectId: id,
        inputState: change.type === 'created' || change.type === 'published' ? 'DoesNotExist' : 'Exists',
        outputState: change.type === 'deleted' ? 'DoesNotExist' : 'ObjectWrite',
        outputVersion: change.version,
        outputDigest: change.digest,
      };
    });

  const gasChange = (raw.objectChanges ?? []).find(
    (c: any) => c.objectType === '0x2::coin::Coin<0x2::sui::SUI>' && c.type === 'mutated',
  );

  return {
    $kind: 'Transaction' as const,
    digest: raw.digest,
    Transaction: {
      digest: raw.digest,
      effects: {
        changedObjects,
        status: { success: true, error: null },
        gasUsed: raw.effects?.gasUsed,
        gasObject: gasChange
          ? {
              objectId: gasChange.objectId,
              outputVersion: gasChange.version,
              outputDigest: gasChange.digest,
            }
          : undefined,
      },
      objectTypes,
      events: (raw.events ?? []).map((e: any) => ({
        ...e,
        eventType: e.type,
      })),
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

    async getDynamicField({ parentId, name }: any) {
      let nameBytes: Buffer;

      if (name.bcs) {
        nameBytes = Buffer.from(name.bcs?.value ?? name.bcs);
      } else {
        nameBytes = Buffer.from(bcs.vector(bcs.u8()).serialize(name.value).toBytes());
      }

      const valueBcs = sandbox.objectApi().getDynamicFieldValueBcs(parentId, nameBytes);

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

    async executeTransaction({ transaction, signatures }: { transaction: Uint8Array; signatures: string[] }) {
      const txBase64 = Buffer.from(transaction).toString('base64');

      return executeTx(txBase64, signatures);
    },

    async waitForTransaction({ digest }: { digest: string }) {
      const raw = sandbox.getTransaction(digest);

      return translateTxResponse(raw);
    },

    async getTransaction({ digest }: { digest: string }) {
      const raw = sandbox.getTransaction(digest);

      return translateTxResponse(raw);
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

    async getTransaction({ digest }: { digest: string }) {
      const raw = sandbox.getTransaction(digest);

      return translateTxResponse(raw);
    },

    async getReferenceGasPrice() {
      return {
        referenceGasPrice: String(sandbox.stateApi().getReferenceGasPrice()),
      };
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
