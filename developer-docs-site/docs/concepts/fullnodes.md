---
title: "Fullnodes Overview"
slug: "fullnodes"
---
An Diem node is an entity of the Diem ecosystem that tracks the [state](../reference/glossary.md#state) of the Diem blockchain. Clients interact with the blockchain via Diem nodes. There are two types of nodes:
* [Validator nodes](./validator-nodes.md)
* Fullnodes

Each Diem node comprises several logical components:
* [REST service](../reference/glossary.md#rest-service)
* [Mempool](./validator-nodes.md#mempool)
* [Execution](./validator-nodes.md#execution)
* [Virtual Machine](./validator-nodes.md#virtual-machine)
* [Storage](./validator-nodes.md#storage)
* [State synchronizer](./validator-nodes.md#state-synchronizer)

The [Diem-core](../reference/glossary.md#diem-core) software can be configured to run as a validator node or as a fullnode.

## Overview

Fullnodes can be run by anyone. Fullnodes re-execute all transactions in the history of the Diem blockchain. Fullnodes replicate the entire state of the blockchain by synchronizing with upstream participants, e.g., other fullnodes or validator nodes. To verify blockchain state, fullnodes receive the set of transactions and the [accumulator hash root](../reference/glossary.md#accumulator-root-hash) of the ledger signed by the validators. In addition, fullnodes accept transactions submitted by Diem clients and forward them directly (or indirectly) to validator nodes. While fullnodes and validators share the same code, fullnodes do not participate in consensus.

Depending on the fullnode upstream, a fullnode can be called as a validator fullnode, or a public fullnode:
* **Validator fullnode** state sync from a validator node directly.
* **Public fullnode** state sync from other fullnodes.

There's no difference in their functionality, only whether their upstream node is a validator or another fullnode. Read more details about network topology [here](./node-networks-sync.md)

Third-party blockchain explorers, wallets, exchanges, and DApps may run a local fullnode to:
* Leverage the REST interface for blockchain interactions.
* Get a consistent view of the Diem ledger.
* Avoid rate limitations on read traffic.
* Run custom analytics on historical data.
* Get notifications about particular on-chain events.
