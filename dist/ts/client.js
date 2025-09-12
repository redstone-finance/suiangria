'use strict';
Object.defineProperty(exports, '__esModule', { value: true });
exports.SandboxClient = void 0;
const index_1 = require('../../index');
class SandboxClient {
  sandbox;
  constructor() {
    this.sandbox = new index_1.SuiSandbox();
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
  getCoins(address, coinType) {
    return JSON.parse(this.coinApi().getCoins(address, coinType));
  }
  executeTransactionBlock(input) {
    const txBytes =
      typeof input.transactionBlock === 'string'
        ? input.transactionBlock
        : Buffer.from(input.transactionBlock).toString('base64');
    const signatures = Array.isArray(input.signature) ? input.signature : [input.signature];
    const result = this.transactionApi().execute(txBytes, signatures);
    return JSON.parse(result);
  }
  dryRunTransaction(transactionBlock) {
    const txBytes =
      typeof transactionBlock === 'string' ? transactionBlock : Buffer.from(transactionBlock).toString('base64');
    const result = this.transactionApi().dryRun(txBytes);
    return JSON.parse(result);
  }
  getTransaction(digest) {
    const response = this.transactionApi().getResponse(digest);
    return JSON.parse(response);
  }
  getObject(input) {
    const result = this.objectApi().get(input.id);
    return JSON.parse(result);
  }
  advanceClockByMillis(millis) {
    this.clockApi().advanceByMillis(millis);
  }
  setClockTimestampMillis(timestamp_ms) {
    this.clockApi().setTimeMs(timestamp_ms);
  }
  rejectNextTransaction(reason) {
    this.behaviourApi().setRejectNextTransaction(reason);
  }
  mintSui(address, amount) {
    this.coinApi().mintSui(address, amount);
  }
  publishPackage(modules, dependencies, sender) {
    return JSON.parse(this.packageApi().publish(modules, dependencies, sender));
  }
  getSuiBalance(address) {
    return this.getBalance(address);
  }
  getBalance(address, coinType) {
    return this.coinApi().getBalance(address, coinType);
  }
  disableSigChecks() {
    this.behaviourApi().disableSignatureChecks();
  }
  enableSigChecks() {
    this.behaviourApi().enableSignatureChecks();
  }
  getNormalizedFunction(params) {
    return JSON.parse(this.packageApi().getNormalizedMoveFunction(params.package, params.module, params.function));
  }
  tryGetPastObject(input) {
    return JSON.parse(this.objectApi().getPast(JSON.stringify(input)));
  }
  getDynamicFields(params) {
    return JSON.parse(this.objectApi().getDynamicFields(JSON.stringify(params)));
  }
  getDynamicFieldObject(input) {
    return JSON.parse(this.objectApi().getDynamicFieldObject(JSON.stringify(input)));
  }
  queryTransactionBlocks(params) {
    return JSON.parse(this.transactionApi().queryBlocks(JSON.stringify(params)));
  }
  reset() {
    this.sandbox = new index_1.SuiSandbox();
  }
}
exports.SandboxClient = SandboxClient;
//# sourceMappingURL=client.js.map
