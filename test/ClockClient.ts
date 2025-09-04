import { SuiClient } from '@mysten/sui/client'
import { Keypair } from '@mysten/sui/cryptography'
import { Transaction } from '@mysten/sui/transactions'
import { MIST_PER_SUI, SUI_CLOCK_OBJECT_ID } from '@mysten/sui/utils'
import z from 'zod'

function flattenFields<T>(data: { fields: T }): T {
  return data.fields
}

// ye i know i shouldnt have named this clock...

export const ClockContent = z
  .object({
    fields: z.object({
      value: z.string().transform(Number),
    }),
  })
  .transform(flattenFields)

export class ClockClient {
  constructor(
    private readonly client: SuiClient,
    private readonly packageId: string,
    private readonly keypair: Keypair,
  ) {}

  async new() {
    const tx = new Transaction()

    tx.moveCall({
      target: `${this.packageId}::clock::new`,
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

    const clock = res.objectChanges!.find(
      (change) => change.type === 'created' && change.objectType.includes('IncreasingTimestamp'),
    )

    return clock?.type === 'created'
      ? clock.objectId
      : (() => {
          throw new Error('Not found shared object')
        })()
  }

  async update(clock: string) {
    const tx = new Transaction()

    tx.moveCall({
      target: `${this.packageId}::clock::update`,
      arguments: [tx.object(clock), tx.object(SUI_CLOCK_OBJECT_ID)],
    })

    tx.setGasBudget(10 * Number(MIST_PER_SUI))

    return await this.client.signAndExecuteTransaction({
      transaction: tx,
      options: { showEffects: true, showEvents: true },
      signer: this.keypair,
    })
  }

  async readTimestamp(clock: string) {
    let clockData = await this.client.getObject({ id: clock, options: { showContent: true } })

    return ClockContent.parse(clockData.data?.content).value
  }
}
