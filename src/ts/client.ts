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
} from '@mysten/sui/client'
import { SuiSandbox } from '../../index'

export interface SandboxConfig {
  initialBalance?: bigint
  enableLogging?: boolean
}

export class SandboxClient {
  private sandbox: SuiSandbox

  constructor() {
    this.sandbox = new SuiSandbox()
  }

  coinApi() {
    return this.sandbox.coinApi()
  }

  transactionApi() {
    return this.sandbox.transactionApi()
  }

  objectApi() {
    return this.sandbox.objectApi()
  }

  clockApi() {
    return this.sandbox.clockApi()
  }

  behaviourApi() {
    return this.sandbox.behaviourApi()
  }

  packageApi() {
    return this.sandbox.packageApi()
  }

  stateApi() {
    return this.sandbox.stateApi()
  }

  storageApi() {
    return this.sandbox.storageApi()
  }

  getCoins(address: string, coinType?: string | null) {
    return JSON.parse(this.coinApi().getCoins(address, coinType))
  }

  executeTransactionBlock(input: {
    transactionBlock: Uint8Array | string
    signature: string | string[]
  }): SuiTransactionBlockResponse {
    const txBytes =
      typeof input.transactionBlock === 'string'
        ? input.transactionBlock
        : Buffer.from(input.transactionBlock).toString('base64')

    const signatures = Array.isArray(input.signature) ? input.signature : [input.signature]

    const result = this.transactionApi().execute(txBytes, signatures)

    return JSON.parse(result)
  }

  dryRunTransaction(transactionBlock: Uint8Array | string): DryRunTransactionBlockResponse {
    const txBytes =
      typeof transactionBlock === 'string' ? transactionBlock : Buffer.from(transactionBlock).toString('base64')

    const result = this.transactionApi().dryRun(txBytes)

    return JSON.parse(result)
  }

  getTransaction(digest: string): SuiTransactionBlockResponse {
    const response = this.transactionApi().getResponse(digest)

    return JSON.parse(response)
  }

  getObject(input: { id: string }) {
    const result = this.objectApi().get(input.id)

    return JSON.parse(result)
  }

  advanceClockByMillis(millis: number) {
    this.clockApi().advanceByMillis(millis)
  }

  setClockTimestampMillis(timestamp_ms: number) {
    this.clockApi().setTimeMs(timestamp_ms)
  }

  rejectNextTransaction(reason: string) {
    this.behaviourApi().setRejectNextTransaction(reason)
  }

  mintSui(address: string, amount: number) {
    this.coinApi().mintSui(address, amount)
  }

  publishPackage(modules: number[][], dependencies: string[], sender: string): SuiTransactionBlockResponse {
    return JSON.parse(this.packageApi().publish(modules, dependencies, sender))
  }

  getSuiBalance(address: string) {
    return this.getBalance(address)
  }

  getBalance(address: string, coinType?: string | null) {
    return this.coinApi().getBalance(address, coinType)
  }

  disableSigChecks() {
    this.behaviourApi().disableSignatureChecks()
  }

  enableSigChecks() {
    this.behaviourApi().enableSignatureChecks()
  }

  getNormalizedFunction(params: GetNormalizedMoveFunctionParams) {
    return JSON.parse(this.packageApi().getNormalizedMoveFunction(params.package, params.module, params.function))
  }

  tryGetPastObject(input: TryGetPastObjectParams): ObjectRead {
    return JSON.parse(this.objectApi().getPast(JSON.stringify(input)))
  }

  getDynamicFields(params: GetDynamicFieldsParams) {
    return JSON.parse(this.objectApi().getDynamicFields(JSON.stringify(params)))
  }

  getDynamicFieldObject(input: GetDynamicFieldObjectParams): Promise<SuiObjectResponse> {
    return JSON.parse(this.objectApi().getDynamicFieldObject(JSON.stringify(input)))
  }

  queryTransactionBlocks(params: QueryTransactionBlocksParams): Promise<PaginatedTransactionResponse> {
    return JSON.parse(this.transactionApi().queryBlocks(JSON.stringify(params)))
  }

  reset() {
    this.sandbox = new SuiSandbox()
  }
}
