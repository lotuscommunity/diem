---
title: "Develop in Local Testnet from Source"
slug: "local-testnet-dev-flow"
---

This guide describes the end-to-end flow for developing with a local testnet from Diem source code.

:::caution CLI from source, not from GitHub
This guide is not correct if you're using the `diem` CLI from a GitHub release or from `cargo install`, only if you build it yourself from `diem-core` as described below.
:::

Please read this guide carefully. This guide specifically addresses the local testnet development flow. This flow will not work if you're building against devnet.

## Run a local testnet from `diem-core`

Pull and go into `diem-core`:
```
git clone git@github.com:aptos-labs/diem-core.git ~/diem-core && cd ~/diem-core
```

Run a local testnet with a faucet:
```
cargo run -p diem -- node run-local-testnet --with-faucet --faucet-port 8081 --force-restart --assume-yes
```
You may add the `--release` flag after `cargo run` if you want to build a release version of the CLI for running the local testnet.

You are now running a local testnet built from `diem-core` main.

## Typescript: Use the SDK from `diem-core`
**Important**: With this development flow, it is essential that you do not use the SDK from npmjs. Instead, you must use the same SDK as the `diem` CLI is built from, which we describe below.

This guide assumes you have done the previous local testnet step. We also assume you have `pnpm` installed.

First, go into `diem-core` and build the SDK:
```
cd ~/diem-core/ecosystem/typescript/sdk
 install
pnpm build
```

Make a new project if you don't have one already:
```
mkdir ~/project && cd ~/project
pnpm init
```

Make your project target the SDK from your local `diem-core`:
```
pnpm add ../diem-core/ecosystem/typescript/sdk
```
You could also use the absolute path, e.g. `/home/daniel/diem-core/ecosystem/typescript/sdk`.

Install everything:
```
pnpm install
```

Now you're set up! You should see in `package.json` that your project targets your local `diem-core`:
```json
{
  "name": "project",
  "version": "1.0.0",
  "main": "index.js",
  "license": "MIT",
  "dependencies": {
    "diem": "../a/core/ecosystem/typescript/sdk/"
  }
}
```

This way your local testnet and the SDK you're using match, meaning you won't see any compatibility issues.

Now you can use the Diem module like this in your code:
```ts
import { DiemClient, DiemAccount, FaucetClient } from "diem";

const NODE_URL = "http://127.0.0.1:8080/v1";
const FAUCET_URL = "http://127.0.0.1:8081";

(async () => {
  const client = new DiemClient(NODE_URL);
  const faucetClient = new FaucetClient(NODE_URL, FAUCET_URL);
})();
```

**Note**: See that this code builds clients that talk to your local testnet, not devnet.
