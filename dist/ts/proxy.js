"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.createSandboxClient = createSandboxClient;
const client_1 = require("./client");
function createSandboxClient() {
    const sandbox = new client_1.SandboxClient();
    const client = new Proxy({}, {
        get(_, prop) {
            const overrides = {
                core: undefined,
                async getBalance(input) {
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
                async dryRunTransactionBlock(input) {
                    const txBase64 = typeof input.transactionBlock === 'string'
                        ? input.transactionBlock
                        : Buffer.from(input.transactionBlock).toString('base64');
                    const result = sandbox.dryRunTransaction(txBase64);
                    return result;
                },
                async getObject(input) {
                    return sandbox.getObject(input);
                },
                async multiGetObjects(input) {
                    const objects = Promise.all(input.ids.map((id) => this.getObject({ id })));
                    return objects;
                },
                async signAndExecuteTransaction({ transaction, signer, ...input }) {
                    let transactionBytes;
                    if (transaction instanceof Uint8Array) {
                        transactionBytes = transaction;
                    }
                    else {
                        transaction.setSenderIfNotSet(signer.toSuiAddress());
                        transactionBytes = await transaction.build({ client: this });
                    }
                    const { signature, bytes } = await signer.signTransaction(transactionBytes);
                    return this.executeTransactionBlock({
                        transactionBlock: bytes,
                        signature,
                        ...input,
                    });
                },
                async waitForTransaction({ ...input }) {
                    return sandbox.getTransaction(input.digest);
                },
                async getNormalizedMoveFunction(params) {
                    return sandbox.getNormalizedFunction(params);
                },
                async getReferenceGasPrice(_ = {}) {
                    return BigInt(sandbox.stateApi().getReferenceGasPrice());
                },
                async getCoins(params) {
                    const coins = sandbox.getCoins(params.owner, params.coinType);
                    return {
                        data: coins,
                        hasNextPage: false,
                    };
                },
                async tryGetPastObject(input) {
                    return sandbox.tryGetPastObject(input);
                },
                async getDynamicFields(input) {
                    return sandbox.getDynamicFields(input);
                },
                async getDynamicFieldObject(input) {
                    return sandbox.getDynamicFieldObject(input);
                },
                async queryTransactionBlocks(params) {
                    return sandbox.queryTransactionBlocks(params);
                },
                async getLatestCheckpointSequenceNumber(_ = {}) {
                    return '10';
                },
                async getTransactionBlock(input) {
                    return sandbox.getTransaction(input.digest);
                },
            };
            if (prop in overrides) {
                return overrides[prop];
            }
            return (...args) => {
                throw new Error(`Method ${String(prop)}(${JSON.stringify(args)}) not yet supported`);
            };
        },
    });
    return { client, sandbox };
}
//# sourceMappingURL=proxy.js.map