# Configuration

Configuration for the Rust crate is file-based (`protocols.json`, optional `tokens.json`) and environment-based (`RPC_URL`, `THE_GRAPH_API_KEY`). Paths are relative to the **crate root** (`rust/`); override with `PROTOCOLS_JSON` and `TOKENS_JSON`.

## protocols.json

Required configuration file defining protocols and discovery settings. Same structure as the TypeScript scanner.

### Structure

```json
{
  "protocols": {
    "protocol-id": {
      "name": "Protocol Name",
      "factory": "0x...",
      "subgraphId": "subgraph-id",
      "enabled": true,
      "poolType": "UniswapV3"
    }
  },
  "discovery": {
    "minLiquidityUSD": 10000,
    "cacheRefreshMinutes": 60,
    "maxPoolsPerProtocol": 100
  }
}
```

### Protocol fields

- **name**: Display name.
- **factory**: Factory contract address (checksummed hex). Used for reference; subgraph URL is built from `subgraphId`.
- **subgraphId**: The Graph decentralized network subgraph ID. The crate builds the URL as `https://gateway.thegraph.com/api/<THE_GRAPH_API_KEY>/subgraphs/id/<subgraphId>`.
- **enabled**: If `false`, the protocol is skipped. Only enabled protocols with a non-empty subgraph URL are returned by `load_protocols_file`.
- **poolType**: Pool implementation type. Supported: `"UniswapV2"`, `"UniswapV3"` (default).

### Discovery settings

- **minLiquidityUSD**: Minimum liquidity (USD) for pools returned by the subgraph query.
- **cacheRefreshMinutes**: Reserved for future cache behavior; currently not used by the Rust discovery logic.
- **maxPoolsPerProtocol**: Maximum number of pools to request per protocol from the subgraph.

## tokens.json (optional)

Token whitelist: symbol → address. Used to **filter discovered pools**: only pools where **both** token0 and token1 are in this whitelist are tracked. Load with `config::load_tokens_file()`, build a `HashSet` of the addresses (values), then pass discovered pools through `discovery::filter_pools_by_token_whitelist(pools, &whitelist)` before starting the scanner. If the file is missing or the whitelist is empty, no filtering is applied (all discovered pools are tracked).

```json
{
  "tokens": {
    "WETH": "0x4200000000000000000000000000000000000006",
    "USDC": "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913"
  }
}
```

## Config module API

- **`load_protocols_file(path: &str) -> Result<(Vec<ProtocolConfig>, DiscoveryConfig)>`**  
  Reads `protocols.json` format from `path`. Builds subgraph URLs using `THE_GRAPH_API_KEY`. Returns only enabled protocols that have a non-empty subgraph URL. If the API key is unset, the returned protocol list is empty and a warning is logged.

- **`load_tokens_file(path: &str) -> Result<HashMap<String, Address>>`**  
  Reads `tokens.json` from `path`. Returns symbol → address. If the file is missing, returns an empty map. If the file exists but JSON is invalid, returns an error. Invalid address strings are skipped.

- **`load_protocol_config(path)`** / **`load_discovery_config(path)`**  
  Alternative loaders for raw `Vec<ProtocolConfig>` or `DiscoveryConfig` JSON (different format from `protocols.json`); useful if you maintain config in a different shape.

## Environment variables

| Variable | Required | Description |
|----------|----------|-------------|
| **RPC_URL** | Yes | WebSocket RPC URL for the chain (used by the scanner). |
| **THE_GRAPH_API_KEY** | Yes for discovery | API key for The Graph gateway. If unset, `load_protocols_file` returns no protocols and logs a warning. |
| **PROTOCOLS_JSON** | No | Path to `protocols.json`. Default: `protocols.json` at crate root (`rust/`). |
| **TOKENS_JSON** | No | Path to `tokens.json`. Default: `tokens.json` at crate root (`rust/`). |

## Resolving config paths

The **crate root** is the `rust/` directory. The `basic_discovery` example resolves paths as follows:

- **Default:** `protocols.json` and `tokens.json` at the crate root (when the binary is run from `rust/`, these are `rust/protocols.json` and `rust/tokens.json`).
- **Override:** set `PROTOCOLS_JSON` or `TOKENS_JSON` to an absolute path or a path relative to the current working directory.

Run from the crate root (`rust/`) so that the default paths find the config files unless you set the env vars.
