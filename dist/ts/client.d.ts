import {
  DryRunTransactionBlockResponse,
  GetDynamicFieldObjectParams,
  GetDynamicFieldsParams,
  GetNormalizedMoveFunctionParams,
  ObjectRead,
  PaginatedTransactionResponse,
  QueryTransactionBlocksParams,
  SuiObjectResponse,
  SuiTransactionBlockResponse,
  TryGetPastObjectParams,
} from '@mysten/sui/client';
export interface SandboxConfig {
  initialBalance?: bigint;
  enableLogging?: boolean;
}
export declare class SandboxClient {
  private sandbox;
  constructor();
  coinApi(): import('../../index').CoinApi;
  transactionApi(): import('../../index').TransactionApi;
  objectApi(): import('../../index').ObjectApi;
  clockApi(): import('../../index').ClockApi;
  behaviourApi(): import('../../index').BehaviourApi;
  packageApi(): import('../../index').PackageApi;
  stateApi(): import('../../index').StateApi;
  storageApi(): import('../../index').StorageApi;
  getCoins(address: string, coinType?: string | null): any;
  executeTransactionBlock(input: {
    transactionBlock: Uint8Array | string;
    signature: string | string[];
  }): SuiTransactionBlockResponse;
  dryRunTransaction(transactionBlock: Uint8Array | string): DryRunTransactionBlockResponse;
  getTransaction(digest: string): SuiTransactionBlockResponse;
  getObject(input: { id: string }): any;
  advanceClockByMillis(millis: number): void;
  setClockTimestampMillis(timestamp_ms: number): void;
  rejectNextTransaction(reason: string): void;
  mintSui(address: string, amount: number): void;
  publishPackage(modules: number[][], dependencies: string[], sender: string): SuiTransactionBlockResponse;
  getSuiBalance(address: string): number;
  getBalance(address: string, coinType?: string | null): number;
  disableSigChecks(): void;
  enableSigChecks(): void;
  getNormalizedFunction(params: GetNormalizedMoveFunctionParams): any;
  tryGetPastObject(input: TryGetPastObjectParams): ObjectRead;
  getDynamicFields(params: GetDynamicFieldsParams): any;
  getDynamicFieldObject(input: GetDynamicFieldObjectParams): Promise<SuiObjectResponse>;
  queryTransactionBlocks(params: QueryTransactionBlocksParams): Promise<PaginatedTransactionResponse>;
  reset(): void;
}
//# sourceMappingURL=client.d.ts.map
