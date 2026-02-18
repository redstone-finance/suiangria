import { createSandboxGrpcClient, publishPackage, SandboxClient } from '../src/ts/index';
import { Secp256k1Keypair } from '@mysten/sui/keypairs/secp256k1';
import { Transaction } from '@mysten/sui/transactions';
import { MIST_PER_SUI, SUI_CLOCK_OBJECT_ID } from '@mysten/sui/utils';
import { AdminClient } from './AdminClient';
import { SharedClient } from './SharedClient';
import { ClockClient } from './ClockClient';
import { DynamicClient } from './DynamicClient';
import { SuiClientTypes } from '@mysten/sui/client';

const INITIAL_BALANCE = 1000000000000000n;
const GAS_BUDGET = 1000000000000000n;
const GAS_PRICE = 100;

describe('SuiSandboxClient', () => {
  it('Estimate gas for tx', async () => {
    const { client, sandbox } = createSandboxGrpcClient();
    const sender = Secp256k1Keypair.generate();
    const recipient = Secp256k1Keypair.generate();

    sandbox.coinApi().mintSui(sender.toSuiAddress(), Number(10n * MIST_PER_SUI));
    const tx = new Transaction();

    const [coin1, coin2] = tx.splitCoins(tx.gas, [tx.pure.u64(100_000_000), tx.pure.u64(100_000_000)]);

    tx.transferObjects([coin1, coin2], tx.pure.address(recipient.toSuiAddress()));

    const result = await client.signAndExecuteTransaction({
      signer: sender,
      transaction: tx,
    });

    expectTxSucceeded(result);
  });

  const publishClockPackage = () => {
    return publishTestPackage('./move-fixtures/clock');
  };

  const publishSharedPackage = () => {
    return publishTestPackage('./move-fixtures/shared');
  };

  const publishDynamicPackage = () => {
    return publishTestPackage('./move-fixtures/dynamic_fields');
  };

  const publishAdminPackage = () => {
    const { client, sandbox, packageId, sender, publishResult } = publishTestPackage('./move-fixtures/admin');

    const adminCap = publishResult.objectChanges!.find(
      (change) => change.type === 'created' && change.objectType.includes('AdminCap'),
    );

    const adminCapId = adminCap?.objectId ?? '';

    return { client, sandbox, packageId, adminCapId, sender, publishResult };
  };

  describe('Sandbox storage', () => {
    it('snapshot test', () => {
      const sandbox = new SandboxClient();
      const keypair = Secp256k1Keypair.generate();
      const address = keypair.toSuiAddress();

      expect(sandbox.getBalance(address)).toBe(0);
      sandbox.coinApi().mintSui(address, 10000);
      expect(sandbox.getBalance(address)).toBe(10000);

      const snapshot = sandbox.storageApi().takeSnapshot();
      const newSandbox = new SandboxClient();

      expect(newSandbox.getBalance(address)).toBe(0);
      newSandbox.storageApi().restoreFromSnapshot(snapshot);
      expect(newSandbox.getBalance(address)).toBe(10000);
    });
  });

  describe('balance operations', () => {
    it('mints SUI to address', () => {
      const sandbox = new SandboxClient();
      const keypair = Secp256k1Keypair.generate();
      const address = keypair.toSuiAddress();

      expect(sandbox.getBalance(address)).toBe(0);
      sandbox.coinApi().mintSui(address, 10000);
      expect(sandbox.getBalance(address)).toBe(10000);
    });
  });

  describe('transaction execution', () => {
    it('executes transfer with valid signature', async () => {
      const { client, sandbox, sender, recipient, coinIds } = setupTransferTest();

      expect(sandbox.getBalance(sender.toSuiAddress())).toBe(Number(INITIAL_BALANCE) * 3);
      expect(sandbox.getBalance(sender.toSuiAddress(), '0x2::sui::SUI')).toBe(Number(INITIAL_BALANCE) * 3);

      sandbox.clockApi().advanceByMillis(100);

      const tx = createTransferTransaction(sender, recipient, coinIds);
      const result = await client.signAndExecuteTransaction({
        signer: sender,
        transaction: tx,
      });

      const txRes = expectTxSucceeded(result);
      expect(txRes.digest).toBeDefined();
      expect(sandbox.getBalance(recipient.toSuiAddress())).toBe(Number(INITIAL_BALANCE) * 2);
    });

    it('executes transaction using signAndExecuteTransaction', async () => {
      const { client, sandbox, sender, recipient, coinIds } = setupTransferTest();

      sandbox.clockApi().advanceByMillis(100);

      const tx = createTransferTransaction(sender, recipient, coinIds);
      expectTxSucceeded(await client.signAndExecuteTransaction({ transaction: tx, signer: sender }));

      expect(sandbox.getBalance(recipient.toSuiAddress())).toBe(Number(INITIAL_BALANCE) * 2);
    });

    it('rejects transaction with invalid signer', async () => {
      const { client, sandbox, sender, recipient, coinIds } = setupTransferTest();

      const tx = createTransferTransaction(sender, recipient, coinIds);
      expectTxFailed(
        await client.signAndExecuteTransaction({
          transaction: tx,
          signer: recipient,
        }),
      );

      expect(sandbox.getBalance(recipient.toSuiAddress())).toBe(0);
    });

    it('accepts invalid signer when signature checks disabled', async () => {
      const { client, sandbox, sender, recipient, coinIds } = setupTransferTest();

      sandbox.disableSigChecks();

      const tx = createTransferTransaction(sender, recipient, coinIds);
      expectTxSucceeded(
        await client.signAndExecuteTransaction({
          transaction: tx,
          signer: recipient,
        }),
      );

      expect(sandbox.getBalance(recipient.toSuiAddress())).toBe(Number(INITIAL_BALANCE) * 2);
    });

    it('reject valid transaction when set to do this', async () => {
      const { client, sandbox, sender, recipient, coinIds } = setupTransferTest();

      sandbox.clockApi().advanceByMillis(100);

      const tx = createTransferTransaction(sender, recipient, coinIds);
      sandbox.behaviourApi().setRejectNextTransaction('i dont like you');
      const result = await client.signAndExecuteTransaction({ transaction: tx, signer: sender });

      expectTxFailed(result);
      expect(sandbox.getBalance(recipient.toSuiAddress())).toBe(0);
    });
  });

  describe('clock operations', () => {
    it('advances clock time', async () => {
      const { sandbox } = createSandboxGrpcClient();

      const initialClock = sandbox.getObject({ id: SUI_CLOCK_OBJECT_ID });
      expect(initialClock.data!.content!.fields.timestamp_ms).toBe('0');

      sandbox.clockApi().advanceByMillis(1000);

      const updatedClock = sandbox.getObject({ id: SUI_CLOCK_OBJECT_ID });
      expect(updatedClock.data!.content!.fields.timestamp_ms).toBe('1000');
    });
  });

  describe('transaction status', () => {
    it('retrieves status for successful and failed transactions', async () => {
      const { client, sender, recipient, coinIds } = setupTransferTest();

      const tx = createTransferTransaction(sender, recipient, coinIds);

      const successResult = await client.signAndExecuteTransaction({
        transaction: tx,
        signer: sender,
      });
      const success = expectTxSucceeded(successResult);

      const successLookup = await client.waitForTransaction({ digest: success.digest });
      expectTxSucceeded(successLookup);

      const failResult = await client.signAndExecuteTransaction({
        transaction: tx,
        signer: recipient,
      });
      const fail = expectTxFailed(failResult);
      const failLookup = await client.waitForTransaction({ digest: fail.digest });

      expectTxFailed(failResult);
      expectTxFailed(failLookup);
    });
  });

  describe('Admin package', () => {
    it('publishes package successfully', () => {
      const { publishResult } = publishAdminPackage();
      expect(publishResult.errors).toBeUndefined();
    });

    it('allows admin to call protected function', async () => {
      const { client, packageId, adminCapId, sender } = publishAdminPackage();
      const adminClient = new AdminClient(client, packageId, adminCapId, sender);

      expectTxSucceeded(await adminClient.callFunction());
    });

    it('prevents non-admin from calling protected function', async () => {
      const { client, sandbox, packageId, adminCapId } = publishAdminPackage();

      const unauthorizedSigner = Secp256k1Keypair.generate();
      sandbox.coinApi().mintSui(unauthorizedSigner.toSuiAddress(), Number(20n * MIST_PER_SUI));

      const adminClient = new AdminClient(client, packageId, adminCapId, unauthorizedSigner);

      expectTxFailed(await adminClient.callFunction());
    });

    it('bypasses admin check when signature verification disabled', async () => {
      const { client, sandbox, packageId, adminCapId } = publishAdminPackage();

      const unauthorizedSigner = Secp256k1Keypair.generate();
      sandbox.coinApi().mintSui(unauthorizedSigner.toSuiAddress(), Number(20n * MIST_PER_SUI));

      sandbox.disableSigChecks();

      const adminClient = new AdminClient(client, packageId, adminCapId, unauthorizedSigner);

      expectTxSucceeded(await adminClient.callFunction());
    });
  });

  describe('Shared package', () => {
    it('publishes package successfully', () => {
      const { publishResult } = publishSharedPackage();
      expect(publishResult.errors).toBeUndefined();
    });

    it('creates shared object', async () => {
      const { client, packageId, sender } = publishSharedPackage();

      const sharedClient = new SharedClient(client, packageId, sender);

      const shared = await sharedClient.new();

      expect(await sharedClient.readValue(shared)).toBe(0);
    });

    it('creates and updates shared object', async () => {
      const { client, packageId, sender } = publishSharedPackage();

      const sharedClient = new SharedClient(client, packageId, sender);

      const shared = await sharedClient.new();

      expect(await sharedClient.readValue(shared)).toBe(0);

      await sharedClient.setValue(shared, 213);

      expect(await sharedClient.readValue(shared)).toBe(213);
    });
  });

  describe('Clock package', () => {
    it('publishes package successfully', () => {
      const { publishResult } = publishClockPackage();

      expect(publishResult.errors).toBeUndefined();
    });

    it('creates clock object', async () => {
      const { client, packageId, sender } = publishClockPackage();

      const clockClient = new ClockClient(client, packageId, sender);

      const clock = await clockClient.new();

      expect(await clockClient.readTimestamp(clock)).toBe(0);
    });

    it('creates and updates clock object', async () => {
      const { client, packageId, sender, sandbox } = publishClockPackage();

      const clockClient = new ClockClient(client, packageId, sender);

      const clock = await clockClient.new();

      expect(await clockClient.readTimestamp(clock)).toBe(0);

      sandbox.clockApi().advanceByMillis(1000);

      expectTxSucceeded(await clockClient.update(clock));

      expect(await clockClient.readTimestamp(clock)).toBe(1000);
    });

    it('creates and updates clock object, fails due to sui-clock time not increased', async () => {
      const { client, packageId, sender, sandbox } = publishClockPackage();

      const clockClient = new ClockClient(client, packageId, sender);

      const clock = await clockClient.new();

      expect(await clockClient.readTimestamp(clock)).toBe(0);

      sandbox.clockApi().advanceByMillis(1000);

      expectTxSucceeded(await clockClient.update(clock));
      expect(await clockClient.readTimestamp(clock)).toBe(1000);

      expectTxFailed(await clockClient.update(clock));
    });
  });

  describe('Dynamic package', () => {
    it('publishes package successfully', () => {
      const { publishResult } = publishDynamicPackage();

      expect(publishResult.errors).toBeUndefined();
    });

    it('Reads main object', async () => {
      const { client, packageId, sender } = publishDynamicPackage();

      const dynamicClient = new DynamicClient(client, packageId, sender);

      const dynamic = await dynamicClient.new();

      const struct = await dynamicClient.readStruct(dynamic);

      expect(struct.values.id).toBeDefined();
    });

    it('Reads dynamic fields', async () => {
      const { client, packageId, sender } = publishDynamicPackage();

      const dynamicClient = new DynamicClient(client, packageId, sender);

      const dynamic = await dynamicClient.new();

      for (let i = 0; i < 4; i++) {
        expect((await dynamicClient.readField(dynamic, i)).value.value).toBe(i);
      }
    });
  });

  describe('Object queries', () => {
    it('tryGetPastObject', async () => {
      const { client, packageId, sender, sandbox } = publishClockPackage();

      const clockClient = new ClockClient(client, packageId, sender);

      const clock = await clockClient.new();

      expect(await clockClient.readTimestamp(clock)).toBe(0);

      sandbox.clockApi().advanceByMillis(1000);

      expectTxSucceeded(await clockClient.update(clock));

      const object = await client.core.getObject({ objectId: clock });
      expect(object.object).toBeDefined();

      const pastObject = sandbox.tryGetPastObject({ id: clock, version: 3 });
      expect(pastObject.status).toBe('VersionFound');

      const notExistingVersion = sandbox.tryGetPastObject({ id: clock, version: 1 });
      expect(notExistingVersion.status).toBe('VersionNotFound');

      const versionTooHigh = sandbox.tryGetPastObject({ id: clock, version: 11234 });
      expect(versionTooHigh.status).toBe('VersionTooHigh');

      const notExistingObject = sandbox.tryGetPastObject({
        id: Secp256k1Keypair.generate().toSuiAddress(),
        version: 2136,
      });
      expect(notExistingObject.status).toBe('ObjectNotExists');
    });

    it('transaction block queries', async () => {
      const { client, packageId, sender, sandbox } = publishClockPackage();

      const clockClient = new ClockClient(client, packageId, sender);

      const clock = await clockClient.new();
      sandbox.clockApi().advanceByMillis(1000);
      await clockClient.update(clock);

      const changedIn = sandbox.queryTransactionBlocks({
        filter: { ChangedObject: clock },
      }).data;
      expect(changedIn.length).toBe(2);

      const inputIn = sandbox.queryTransactionBlocks({
        filter: { InputObject: clock },
      }).data;
      expect(inputIn.length).toBe(1);

      const affectedIn = sandbox.queryTransactionBlocks({
        filter: { AffectedObject: clock },
      }).data;
      expect(affectedIn.length).toBe(2);

      const allTxs = sandbox.queryTransactionBlocks({}).data;
      expect(allTxs.length).toBe(3);
    });
  });
});

function setupTransferTest() {
  const { client, sandbox } = createSandboxGrpcClient();
  const sender = Secp256k1Keypair.generate();
  const recipient = Secp256k1Keypair.generate();

  const coinIds = Array.from({ length: 3 }, () =>
    sandbox.coinApi().mintSui(sender.toSuiAddress(), Number(INITIAL_BALANCE)),
  );

  return { client, sandbox, sender, recipient, coinIds };
}

function createTransferTransaction(sender: Secp256k1Keypair, recipient: Secp256k1Keypair, coinIds: string[]) {
  const tx = new Transaction();

  tx.transferObjects([coinIds[1], coinIds[2]], recipient.toSuiAddress());
  tx.setSender(sender.toSuiAddress());
  tx.setGasBudget(Number(GAS_BUDGET));
  tx.setGasPrice(GAS_PRICE);
  tx.setGasPayment([
    {
      objectId: coinIds[0],
      version: '1',
      digest: '11111111111111111111111111111111',
    },
  ]);

  return tx;
}

function publishTestPackage(packageDir: string) {
  const { client, sandbox } = createSandboxGrpcClient();
  const sender = Secp256k1Keypair.generate();

  sandbox.coinApi().mintSui(sender.toSuiAddress(), Number(20n * MIST_PER_SUI));
  const publishResult = publishPackage(sandbox, packageDir, sender.toSuiAddress());

  const packageId = publishResult.objectChanges!.find((change) => change.type === 'published')!.packageId;

  return { client, sandbox, packageId, sender, publishResult };
}

function expectTxSucceeded(res: SuiClientTypes.TransactionResult) {
  expect(res.$kind).not.toBe('FailedTransaction');

  return res.Transaction!;
}

function expectTxFailed(res: SuiClientTypes.TransactionResult) {
  expect(res.$kind).toBe('FailedTransaction');

  return res.FailedTransaction!;
}
