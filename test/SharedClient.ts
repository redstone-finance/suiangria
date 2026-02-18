import { bcs } from '@mysten/sui/bcs';
import { Keypair } from '@mysten/sui/cryptography';
import { SuiGrpcClient } from '@mysten/sui/grpc';
import { Transaction } from '@mysten/sui/transactions';
import { MIST_PER_SUI } from '@mysten/sui/utils';
import z from 'zod';

export const SharedContent = z.object({
  value: z.number(),
});

export class SharedClient {
  constructor(
    private readonly client: SuiGrpcClient,
    private readonly packageId: string,
    private readonly keypair: Keypair,
  ) {}

  async new() {
    const tx = new Transaction();

    tx.moveCall({
      target: `${this.packageId}::shared::new`,
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
        change.inputState === 'DoesNotExist' && res.Transaction.objectTypes[change.objectId]?.includes('Test'),
    );

    return created?.objectId
      ? created.objectId
      : (() => {
          throw new Error('Not found shared object');
        })();
  }

  async setValue(shared: string, value: number) {
    const tx = new Transaction();

    tx.moveCall({
      target: `${this.packageId}::shared::set_value`,
      arguments: [tx.object(shared), bcs.u8().serialize(value)],
    });

    tx.setGasBudget(10 * Number(MIST_PER_SUI));

    const res = await this.client.signAndExecuteTransaction({
      transaction: tx,
      signer: this.keypair,
      include: { effects: true },
    });

    if (res.$kind === 'FailedTransaction') {
      throw new Error(`Transaction failed: ${res.FailedTransaction}`);
    }

    return res;
  }

  async readValue(shared: string) {
    const { object } = await this.client.core.getObject({
      objectId: shared,
      include: { json: true },
    });

    return SharedContent.parse(object.json).value;
  }
}
