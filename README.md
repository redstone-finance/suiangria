# 🏖️ suiangria

![SUIANGRIA Logo](assets/logo.png)

> In-memory Sui blockchain sandbox for testing sui d-apps connections with move contracts without running localnet.
> The sanbox simulates network & move execution environment (without consensus) and can be controlled programmatically.

⚠️ **Early Development Notice**: This project is under active development. Many features are incomplete or may not work as expected. Use at your own risk.

Above means, that if you will use this library for testing your sui d-apps, tests passing does not mean the app is perfect. Although you can use this to find failing cases in your app.

## Installation

```bash
yarn add suiangria
```

## Features (Desired Feature Set)

The following features represent the intended functionality. Not all features are fully implemented or work in every scenario:

- **Drop-in SuiClient Replacement** - Replace `@mysten/sui/client` in tests without modifying production code

- **In-Memory Execution Environment** - Run Sui Move VM sanbox in memory locally

- **Programmatic Network Control** - Control execution behavior via sandbox client:
  - Toggle signature verification
  - Manipulate clock time (set, advance)
  - Programmatically accept/reject transactions to test error handling
  - Configure gas requirements

- **Test Data Generation** - Create test state easily:
  - Mint SUI tokens
  - Create arbitrary objects
  - Generate funded test accounts

- **Full RPC Query Support** - Complete Sui RPC method implementation for state, transactions, and events

- **Move Package Support** - Build and publish Move packages directly from TypeScript tests

- **Time-Dependent Testing** - Advance blockchain clock for testing time-locked logic

## Quick Start

```typescript
import { createSandboxClient } from 'suiangria'
import { Transaction } from '@mysten/sui/transactions'

const { client, sandbox } = createSandboxClient()

// Fund an address
const coinId = sandbox.mintSui('0x...', 1000000000)
```

## Signature free testing

```typescript
  // Admin package which create one and only one instance of the object AdminCap.
  // Only owner of the object cann call its function `callFunction`.
  function publishAdminPackage(packageDir = "move-fixtures/admin") {
    const { client, sandbox } = createSandboxClient()
    const sender = Secp256k1Keypair.generate()

    sandbox.mintSui(sender.toSuiAddress(), Number(20n * MIST_PER_SUI))
    const publishResult = publishPackage(sandbox, packageDir, sender.toSuiAddress())

    const packageId = publishResult.objectChanges!.find((change) => change.type === 'published')!.packageId const adminCap = const adminCap = publishResult.objectChanges!.find(
      (change) => change.type === 'created' && change.objectType.includes('AdminCap'),
    )
    const adminCapId = adminCap?.type === 'created' ? adminCap.objectId : ''

    return { client, sandbox, packageId, adminCapId, sender, publishResult }

    return { client, sandbox, packageId, sender, publishResult }
  }

  describe('Admin package', () => {
    it('publishes package successfully', () => {
      const { publishResult } = publishAdminPackage()
      expect(publishResult.errors).toBeUndefined()
    })

    it('allows admin to call protected function', async () => {
      const { client, packageId, adminCapId, sender } = publishAdminPackage()
      const adminClient = new AdminClient(client, packageId, adminCapId, sender)

      checkTxSuccedded(await adminClient.callFunction())
    })

    it('prevents non-admin from calling protected function', async () => {
      const { client, sandbox, packageId, adminCapId } = publishAdminPackage()

      const unauthorizedSigner = Secp256k1Keypair.generate()
      sandbox.mintSui(unauthorizedSigner.toSuiAddress(), Number(20n * MIST_PER_SUI))

      const adminClient = new AdminClient(client, packageId, adminCapId, unauthorizedSigner)

      checkTxFailed(await adminClient.callFunction())
    })

    it('bypasses admin check when signature verification disabled', async () => {
      const { client, sandbox, packageId, adminCapId } = publishAdminPackage()

      const unauthorizedSigner = Secp256k1Keypair.generate()
      sandbox.mintSui(unauthorizedSigner.toSuiAddress(), Number(20n * MIST_PER_SUI))

      sandbox.disableSigChecks()

      const adminClient = new AdminClient(client, packageId, adminCapId, unauthorizedSigner)

      checkTxSuccedded(await adminClient.callFunction())
    })
  })
```

for more examples check out tests.

## Development

### Prerequisites

- Rust (latest stable)
- Node.js >= 16.0.0
- Yarn 4.x

### Building

```bash
# Install dependencies
yarn install

# Build both Rust and TypeScript
yarn build

# Build in watch mode
yarn build:ts:watch
```

### Testing

Sui cli should be installed (used for compilation of move packages) - [install sui](https://docs.sui.io/guides/developer/getting-started/sui-install)

```bash
# Run all tests, really slow
yarn test

# Run tests in debug mode, faster compilation prefer this
yarn test:debug
```

## API

### createSandboxClient

```typescript
function createSandboxClient(): {
  client: SuiClient // Drop-in replacement for @mysten/sui SuiClient
  sandbox: SandboxClient // Additional sandbox controls
}
```

## Platform Support

Pre-built binaries are available for:

- Windows x64
- macOS x64
- macOS ARM64 (Apple Silicon)
- Linux x64

## Release Process

```bash
npm version [major | minor | patch]
git push --follow-tags
```

GitHub Actions will automatically build and publish platform-specific packages.

## Credits

This project is built using [napi-rs](https://github.com/napi-rs/napi-rs), which provides excellent Rust bindings for Node.js. The project structure and build configuration are based on the [napi-rs package template](https://github.com/napi-rs/package-template).

## License

MIT
