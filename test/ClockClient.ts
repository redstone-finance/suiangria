import { Keypair } from '@mysten/sui/cryptography';
import { SuiGrpcClient } from '@mysten/sui/grpc';
import { Transaction } from '@mysten/sui/transactions';
import { MIST_PER_SUI, SUI_CLOCK_OBJECT_ID } from '@mysten/sui/utils';
import z from 'zod';

export const ClockContent = z.object({
  value: z.string().transform(Number),
});

export class ClockClient {
  constructor(
    private readonly client: SuiGrpcClient,
    private readonly packageId: string,
    private readonly keypair: Keypair,
  ) {}

  async new() {
    const tx = new Transaction();
    tx.moveCall({
      target: `${this.packageId}::clock::new`,
      arguments: [],
    });
    tx.setGasBudget(10 * Number(MIST_PER_SUI));

    const res = await this.client.signAndExecuteTransaction({
      transaction: tx,
      signer: this.keypair,
      include: { effects: true, objectChanges: true, objectTypes: true },
    });

    if (res.$kind === 'FailedTransaction') {
      throw new Error(`Transaction failed: ${res.FailedTransaction}`);
    }
    const created = res.Transaction.effects.changedObjects.find(
      (change) =>
        change.inputState === 'DoesNotExist' &&
        res.Transaction.objectTypes[change.objectId].includes('IncreasingTimestamp'),
    );

    if (!created?.objectId) {
      throw new Error('Not found clock object');
    }

    return created.objectId;
  }

  async update(clock: string) {
    const tx = new Transaction();
    tx.moveCall({
      target: `${this.packageId}::clock::update`,
      arguments: [tx.object(clock), tx.object(SUI_CLOCK_OBJECT_ID)],
    });
    tx.setGasBudget(10 * Number(MIST_PER_SUI));

    return await this.client.signAndExecuteTransaction({
      transaction: tx,
      signer: this.keypair,
      include: { effects: true },
    });
  }

  async readTimestamp(clock: string) {
    const { object } = await this.client.core.getObject({
      objectId: clock,
      include: { json: true },
    });

    return ClockContent.parse(object.json).value;
  }
}
