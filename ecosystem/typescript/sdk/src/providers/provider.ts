import { DiemClient } from "./diem_client";
import { IndexerClient } from "./indexer";

import { CustomEndpoints, Network, NetworkToIndexerAPI, NetworkToNodeAPI } from "../utils";
import { ClientConfig } from "../client";

type NetworkWithCustom = Network | "CUSTOM";
/**
 * Builds a Provider class with an diem client configured to connect to an Diem node
 * and indexer client configured to connect to Diem Indexer.
 *
 * It creates DiemClient and IndexerClient instances based on the network or custom endpoints provided.
 *
 * This class holds both DiemClient and IndexerClient classes's methods and properties so we
 * can instantiate the Provider class and use it to query full node and/or Indexer.
 *
 * @example An example of how to use this class
 * ```
 * const provider = new Provider(Network.DEVNET)
 * const account = await provider.getAccount("0x123");
 * const accountNFTs = await provider.getAccountNFTs("0x123");
 * ```
 *
 * @param network enum of type Network - MAINNET | TESTNET | DEVENET or custom endpoints of type CustomEndpoints
 * @param config DiemClient config arg - additional configuration options for the generated Axios client.
 */
export class Provider {
  diemClient: DiemClient;

  indexerClient: IndexerClient;

  network: NetworkWithCustom;

  constructor(network: Network | CustomEndpoints, config?: ClientConfig, doNotFixNodeUrl: boolean = false) {
    let fullNodeUrl = null;
    let indexerUrl = null;

    if (typeof network === "object" && isCustomEndpoints(network)) {
      fullNodeUrl = network.fullnodeUrl;
      indexerUrl = network.indexerUrl;
      this.network = "CUSTOM";
    } else {
      fullNodeUrl = NetworkToNodeAPI[network];
      indexerUrl = NetworkToIndexerAPI[network];
      this.network = network;
    }

    if (!fullNodeUrl || !indexerUrl) {
      throw new Error("network is not provided");
    }

    this.diemClient = new DiemClient(fullNodeUrl, config, doNotFixNodeUrl);
    this.indexerClient = new IndexerClient(indexerUrl, config);
  }
}

export interface Provider extends DiemClient, IndexerClient {}

/**
In TypeScript, we can’t inherit or extend from more than one class,
Mixins helps us to get around that by creating a partial classes 
that we can combine to form a single class that contains all the methods and properties from the partial classes.
{@link https://www.typescriptlang.org/docs/handbook/mixins.html#alternative-pattern}

Here, we combine DiemClient and IndexerClient classes into one Provider class that holds all 
methods and properties from both classes.
*/
function applyMixin(targetClass: any, baseClass: any, baseClassProp: string) {
  // Mixin instance methods
  Object.getOwnPropertyNames(baseClass.prototype).forEach((propertyName) => {
    const propertyDescriptor = Object.getOwnPropertyDescriptor(baseClass.prototype, propertyName);
    if (!propertyDescriptor) return;
    // eslint-disable-next-line func-names
    propertyDescriptor.value = function (...args: any) {
      return (this as any)[baseClassProp][propertyName](...args);
    };
    Object.defineProperty(targetClass.prototype, propertyName, propertyDescriptor);
  });
  // Mixin static methods
  Object.getOwnPropertyNames(baseClass).forEach((propertyName) => {
    const propertyDescriptor = Object.getOwnPropertyDescriptor(baseClass, propertyName);
    if (!propertyDescriptor) return;
    // eslint-disable-next-line func-names
    propertyDescriptor.value = function (...args: any) {
      return (this as any)[baseClassProp][propertyName](...args);
    };
    if (targetClass.hasOwnProperty.call(targetClass, propertyName)) {
      // The mixin has already been applied, so skip applying it again
      return;
    }
    Object.defineProperty(targetClass, propertyName, propertyDescriptor);
  });
}

applyMixin(Provider, DiemClient, "diemClient");
applyMixin(Provider, IndexerClient, "indexerClient");

// use exhaustive type predicates
function isCustomEndpoints(network: CustomEndpoints): network is CustomEndpoints {
  return (
    network.fullnodeUrl !== undefined &&
    typeof network.fullnodeUrl === "string" &&
    network.indexerUrl !== undefined &&
    typeof network.indexerUrl === "string"
  );
}
