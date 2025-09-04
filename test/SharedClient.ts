import { bcs } from '@mysten/bcs'
import { SuiClient } from '@mysten/sui/client'
import { Keypair } from '@mysten/sui/cryptography'
import { Transaction } from '@mysten/sui/transactions'
import { MIST_PER_SUI } from '@mysten/sui/utils'
import z from 'zod'

function flattenFields<T>(data: { fields: T }): T {
  return data.fields
}

export const SharedContent = z
  .object({
    fields: z.object({
      value: z.number(),
    }),
  })
  .transform(flattenFields)

export class SharedClient {
  constructor(
    private readonly client: SuiClient,
    private readonly packageId: string,
    private readonly keypair: Keypair,
  ) {}

  async new() {
    const tx = new Transaction()

    tx.moveCall({
      target: `${this.packageId}::shared::new`,
      arguments: [],
    })

    tx.setGasBudget(10 * Number(MIST_PER_SUI))

    const res = await this.client.signAndExecuteTransaction({
      transaction: tx,
      options: { showEffects: true, showEvents: true },
      signer: this.keypair,
    })

    if (res.errors) {
      throw new AggregateError(res.errors)
    }

    const test = res.objectChanges!.find((change) => change.type === 'created' && change.objectType.includes('Test'))

    return test?.type === 'created'
      ? test.objectId
      : (() => {
          throw new Error('Not found shared object')
        })()
  }

  async setValue(shared: string, value: number) {
    const tx = new Transaction()

    tx.moveCall({
      target: `${this.packageId}::shared::set_value`,
      arguments: [tx.object(shared), bcs.u8().serialize(value)],
    })

    tx.setGasBudget(10 * Number(MIST_PER_SUI))

    return await this.client.signAndExecuteTransaction({
      transaction: tx,
      options: { showEffects: true, showEvents: true },
      signer: this.keypair,
    })
  }

  async readValue(shared: string) {
    let sharedData = await this.client.getObject({ id: shared, options: { showContent: true } })

    return SharedContent.parse(sharedData.data?.content).value
  }
}
