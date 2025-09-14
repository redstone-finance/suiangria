import { SuiClient } from '@mysten/sui/client';
import { Keypair } from '@mysten/sui/cryptography';
import { Transaction } from '@mysten/sui/transactions';
import { MIST_PER_SUI } from '@mysten/sui/utils';
import z from 'zod';

function flattenFields<T>(data: { fields: T }): T {
  return data.fields;
}

const ValuesContent = z
  .object({
    fields: z.object({
      id: z.object({
        id: z.string(),
      }),
    }),
  })
  .transform(flattenFields);

export const DynamicContents = z
  .object({
    fields: z.object({
      values: ValuesContent,
    }),
  })
  .transform(flattenFields);

const Value = z
  .object({
    fields: z.object({
      value: z.object({ fields: z.object({ value: z.number() }) }).transform(flattenFields),
    }),
  })
  .transform(flattenFields);

export class DynamicClient {
  constructor(
    private readonly client: SuiClient,
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
      options: { showEffects: true, showEvents: true },
      signer: this.keypair,
    });

    if (res.errors) {
      throw new AggregateError(res.errors);
    }

    const dynamic = res.objectChanges!.find(
      (change) => change.type === 'created' && change.objectType.includes('Dynamic'),
    );

    return dynamic?.type === 'created'
      ? dynamic.objectId
      : (() => {
          throw new Error('Not found shared object');
        })();
  }

  async readStruct(dynamic: string) {
    const dynamicData = await this.client.getObject({ id: dynamic, options: { showContent: true } });

    return DynamicContents.parse(dynamicData.data?.content);
  }

  async readField(dynamic: string, field: number) {
    const str = await this.readStruct(dynamic);
    const fieldName = {
      type: 'u8',
      value: field,
    };

    const response = await this.client.getDynamicFieldObject({ parentId: str.values.id.id, name: fieldName });

    return Value.parse(response.data?.content);
  }
}
