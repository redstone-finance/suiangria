import { Keypair } from '@mysten/sui/cryptography';
import { SuiGrpcClient } from '@mysten/sui/grpc';
import { Transaction } from '@mysten/sui/transactions';
import { MIST_PER_SUI } from '@mysten/sui/utils';

export class AdminClient {
  constructor(
    private readonly client: SuiGrpcClient,
    private readonly packageId: string,
    private readonly adminCap: string,
    private readonly keypair: Keypair,
  ) {}

  async callFunction() {
    const tx = new Transaction();

    tx.moveCall({
      target: `${this.packageId}::admin::only_admin_can_call_it`,
      arguments: [tx.object(this.adminCap)],
    });

    tx.setGasBudget(10 * Number(MIST_PER_SUI));

    return await this.client.signAndExecuteTransaction({
      transaction: tx,
      signer: this.keypair,
    });
  }
}
