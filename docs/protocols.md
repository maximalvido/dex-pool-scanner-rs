# Supported Protocols

The Rust crate supports Uniswap V2–style and Uniswap V3–style pools for discovery (via subgraphs) and for pool/price types (via `BaseLiquidityPool` implementations). Discovery is driven by `protocols.json` and `poolType`; price tracking is intended to use the same pool types once log subscription is wired.

## Protocol types (discovery)

- **UniswapV3** (`poolType: "UniswapV3"`): Queries subgraph `pools` with `totalValueLockedUSD`, `feeTier`, etc.
- **UniswapV2** (`poolType: "UniswapV2"`): Queries subgraph `pairs` with `reserveUSD`.

Other protocols (e.g. Aerodrome CL, SushiSwap V3) that expose a V3-compatible subgraph can use `poolType: "UniswapV3"` in `protocols.json`.

## Types (`types` module)

- **`Protocol`** – Enum: `UniswapV2`, `UniswapV3`.
- **`CachedPool`** – Discovered pool: address, protocol id, token0/token1 (address, symbol, decimals), fee, liquidity_usd, volume_24h_usd, last_seen.
- **`PoolPrice`** – Price snapshot: pool_address, token0_price, token1_price, timestamp.
- **`ProtocolConfig`** – id, name, subgraph_url, pool_type, enabled.
- **`DiscoveryConfig`** – min_liquidity_usd, max_pools_per_protocol, cache_enabled, cache_file.

## Liquidity pool trait (`liquidity_pools`)

**`BaseLiquidityPool`** (async trait, `Send + Sync`):

- **`parse_swap_event_data(&self, log: &EthereumLog) -> Result<SwapEventData>`** – Decode a swap/sync log into amounts, price, sender, recipient.
- **`get_contract_address(&self) -> Address`**
- **`get_event_signatures(&self) -> Vec<B256>`** – Event topic0 hashes to subscribe to.
- **`get_name(&self) -> &str`**
- **`get_current_price(&self) -> f64`**
- **`apply_initial_state(&mut self, result: Vec<u8>) -> Result<()>`** – Apply RPC response (e.g. slot0 or getReserves) to internal state.

**Implementations:**

- **UniswapV3** – Uses `sqrtPriceX96`; price = (sqrtPriceX96/2^96)^2 with decimal adjustment. Swap event parsing is still `todo!()`.
- **UniswapV2** – Uses reserves; price = reserve1/reserve0 with decimal adjustment. Swap/Sync parsing is still `todo!()`.

**Shared types:**

- **`EthereumLog`** – address, topics, data (alloy `Log` → this type).
- **`SwapEventData`** – amount0, amount1, price, sender, recipient.

## RPC scanner (`rpc` module)

- **`Scanner`** – Holds a WebSocket provider, pool list, liquidity pool map, current prices, and a price-change callback.
- **`Scanner::new(on_price_change) -> Result<Self>`** – Creates the scanner with a callback. Reads `RPC_URL` from the environment and connects via `WsConnect`.
- **`Scanner::start(&mut self) -> Result<()>`** – Loads config (`protocols.json`, `tokens.json`), discovers pools, filters by token whitelist, subscribes to swap/sync logs, and invokes the callback with `CachedPool`, new `PoolPrice`, and previous `PoolPrice` (if any).
- **`PriceChangeCallback`** – `Arc<dyn Fn(CachedPool, PoolPrice, Option<PoolPrice>) + Send + Sync>`.

## Adding a new protocol

1. **Discovery**: Add an entry in `protocols.json` with the correct `subgraphId` and `poolType` (`UniswapV2` or `UniswapV3`) so the existing subgraph client can query it.
2. **Price tracking**: Implement `BaseLiquidityPool` in `liquidity_pools/mod.rs` (or a submodule), then register the pool type in the scanner when building `liquidity_pools` from `CachedPool` (once that path is implemented). Optionally extend `Protocol` and config if you need a new pool type.
