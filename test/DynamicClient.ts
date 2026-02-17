import { bcs } from '@mysten/sui/bcs';
import { Keypair } from '@mysten/sui/cryptography';
import { SuiGrpcClient } from '@mysten/sui/grpc';
import { Transaction } from '@mysten/sui/transactions';
import { MIST_PER_SUI } from '@mysten/sui/utils';
import z from 'zod';

const ValuesContent = z.object({
  id: z.string(),
});

export const DynamicContents = z.object({
  values: z.object({
    id: ValuesContent,
  }),
});

const DynamicValue = z.object({
  value: z.number(),
});

const DynamicValueBcs = bcs.struct('DynamicValue', {
  value: bcs.u8(),
});

export class DynamicClient {
  constructor(
    private readonly client: SuiGrpcClient,
    private readonly packageId: string,
    private readonly keypair: Keypair,
  ) {}

  async new() {
    const tx = new Transaction();
    tx.moveCall({
      target: `${this.packageId}::dynamic_fields::new`,
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
        change.inputState === 'DoesNotExist' && res.Transaction.objectTypes[change.objectId]?.includes('Dynamic'),
    );

    return created?.objectId
      ? created.objectId
      : (() => {
          throw new Error('Not found shared object');
        })();
  }

  async readStruct(objectId: string) {
    const { object } = await this.client.core.getObject({
      objectId,
      include: { json: true },
    });

    return DynamicContents.parse(object.json);
  }

  async readField(objectId: string, field: number) {
    const struct = await this.readStruct(objectId);

    const { dynamicField } = await this.client.core.getDynamicField({
      parentId: struct.values.id.id,
      name: { type: 'u8', bcs: bcs.u8().serialize(field).toBytes() },
    });

    const decoded = DynamicValueBcs.parse(dynamicField.value.bcs);

    return { value: { value: Number(decoded.value) } };
  }
}
