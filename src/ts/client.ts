import { SuiSandbox } from '../../index.js';

export interface ObjectChange {
  type: 'created' | 'mutated' | 'deleted' | 'published' | 'transferred' | 'wrapped';
  objectId: string;
  objectType: string;
  packageId: string;
  version: string;
  digest: string;
  sender: string;
}

export interface TransactionEffects {
  status: { status: 'success' | 'failure'; error?: string };
  gasUsed: {
    computationCost: string;
    storageCost: string;
    storageRebate: string;
    nonRefundableStorageFee: string;
  };
}

export interface SandboxTransactionResponse {
  digest: string;
  effects: TransactionEffects;
  objectChanges?: ObjectChange[];
  events?: unknown[];
  errors?: string[];
}

export interface ObjectContent {
  dataType: 'moveObject';
  type: string;
  fields: Record<string, unknown>;
  hasPublicTransfer: boolean;
}

export interface ObjectBcs {
  dataType: 'moveObject';
  type: string;
  bcsBytes: string;
  version: number;
}

export interface SandboxObjectData {
  objectId: string;
  version: string;
  digest: string;
  type?: string;
  owner?:
    | { AddressOwner: string }
    | { ObjectOwner: string }
    | { Shared: { initial_shared_version: number } }
    | 'Immutable';
  content?: ObjectContent;
  bcs?: ObjectBcs;
}

export interface SandboxObjectResponse {
  data?: SandboxObjectData;
  error?: { code: string; object_id?: string };
}

export interface DynamicFieldInfo {
  name: { type: string; value: unknown };
  bcsName: string;
  type: string;
  objectType: string;
  objectId: string;
  version: number;
  digest: string;
}

export interface DynamicFieldPage {
  data: DynamicFieldInfo[];
  hasNextPage: boolean;
  nextCursor?: string | null;
}

export type ObjectReadStatus =
  | 'VersionFound'
  | 'ObjectNotExists'
  | 'VersionNotFound'
  | 'VersionTooHigh'
  | 'ObjectDeleted';

export interface ObjectRead {
  status: ObjectReadStatus;
  details?: SandboxObjectData;
}

export interface DryRunEffects {
  effects: TransactionEffects;
  events: unknown[];
  objectChanges: ObjectChange[];
  input: unknown;
}

export interface PaginatedTransactionResponse {
  data: SandboxTransactionResponse[];
  hasNextPage: boolean;
  nextCursor?: string | null;
}

export interface NormalizedMoveFunction {
  visibility: string;
  isEntry: boolean;
  typeParameters: unknown[];
  parameters: unknown[];
  return: unknown[];
}

export class SandboxClient {
  private sandbox: SuiSandbox;

  constructor() {
    this.sandbox = new SuiSandbox();
  }

  coinApi() {
    return this.sandbox.coinApi();
  }

  transactionApi() {
    return this.sandbox.transactionApi();
  }

  objectApi() {
    return this.sandbox.objectApi();
  }

  clockApi() {
    return this.sandbox.clockApi();
  }

  behaviourApi() {
    return this.sandbox.behaviourApi();
  }

  packageApi() {
    return this.sandbox.packageApi();
  }

  stateApi() {
    return this.sandbox.stateApi();
  }

  storageApi() {
    return this.sandbox.storageApi();
  }

  getCoins(address: string, coinType?: string | null) {
    return JSON.parse(this.coinApi().getCoins(address, coinType));
  }

  executeTransactionBlock(input: {
    transactionBlock: Uint8Array | string;
    signature: string | string[];
  }): SandboxTransactionResponse {
    const txBytes =
      typeof input.transactionBlock === 'string'
        ? input.transactionBlock
        : Buffer.from(input.transactionBlock).toString('base64');

    const signatures = Array.isArray(input.signature) ? input.signature : [input.signature];

    return JSON.parse(this.transactionApi().execute(txBytes, signatures));
  }

  dryRunTransaction(transactionBlock: Uint8Array | string): DryRunEffects {
    const txBytes =
      typeof transactionBlock === 'string' ? transactionBlock : Buffer.from(transactionBlock).toString('base64');

    return JSON.parse(this.transactionApi().dryRun(txBytes));
  }

  getTransaction(digest: string): SandboxTransactionResponse {
    return JSON.parse(this.transactionApi().getResponse(digest));
  }

  getObject(input: { id: string }): SandboxObjectResponse {
    return JSON.parse(this.objectApi().get(input.id));
  }

  advanceClockByMillis(millis: number) {
    this.clockApi().advanceByMillis(millis);
  }

  setClockTimestampMillis(timestamp_ms: number) {
    this.clockApi().setTimeMs(timestamp_ms);
  }

  rejectNextTransaction(reason: string) {
    this.behaviourApi().setRejectNextTransaction(reason);
  }

  mintSui(address: string, amount: number) {
    this.coinApi().mintSui(address, amount);
  }

  publishPackage(modules: number[][], dependencies: string[], sender: string): SandboxTransactionResponse {
    return JSON.parse(this.packageApi().publish(modules, dependencies, sender));
  }

  getSuiBalance(address: string) {
    return this.getBalance(address);
  }

  getBalance(address: string, coinType?: string | null) {
    return this.coinApi().getBalance(address, coinType);
  }

  disableSigChecks() {
    this.behaviourApi().disableSignatureChecks();
  }

  enableSigChecks() {
    this.behaviourApi().enableSignatureChecks();
  }

  getNormalizedFunction(params: { package: string; module: string; function: string }): NormalizedMoveFunction {
    return JSON.parse(this.packageApi().getNormalizedMoveFunction(params.package, params.module, params.function));
  }

  tryGetPastObject(input: { id: string; version: number }): ObjectRead {
    return JSON.parse(this.objectApi().getPast(JSON.stringify(input)));
  }

  getDynamicFields(params: { parentId: string; cursor?: string | null; limit?: number }): DynamicFieldPage {
    return JSON.parse(this.objectApi().getDynamicFields(JSON.stringify(params)));
  }

  getDynamicFieldObject(input: { parentId: string; name: { type: string; value: unknown } }): SandboxObjectResponse {
    return JSON.parse(this.objectApi().getDynamicFieldObject(JSON.stringify(input)));
  }

  queryTransactionBlocks(params: {
    filter?: Record<string, unknown>;
    cursor?: string | null;
    limit?: number;
  }): PaginatedTransactionResponse {
    return JSON.parse(this.transactionApi().queryBlocks(JSON.stringify(params)));
  }

  reset() {
    this.sandbox = new SuiSandbox();
  }
}
