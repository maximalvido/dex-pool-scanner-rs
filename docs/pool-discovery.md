# Pool Discovery

The Rust crate discovers liquidity pools by querying The Graph subgraphs for each enabled protocol configured in `protocols.json`. There is no local cache in the current implementation; each run fetches from the subgraphs.

## How it works

1. **Load config**: Use `config::load_protocols_file()` to get `Vec<ProtocolConfig>` and `DiscoveryConfig`. Only protocols with a valid subgraph URL (and `THE_GRAPH_API_KEY` set) are included.

2. **Fetch per protocol**: `PoolDiscovery::discover_pools(protocols, discovery_config)` calls the subgraph for each protocol with:
   - **V3-style subgraphs**: `pools` query, ordered by `totalValueLockedUSD`, with `totalValueLockedUSD_gte: minLiquidityUSD` and `first: maxPoolsPerProtocol`.
   - **V2-style subgraphs**: `pairs` query, ordered by `reserveUSD`, with `reserveUSD_gte: minLiquidityUSD` and `first: maxPoolsPerProtocol`.

3. **Aggregation**: Results are combined into a single `Vec<CachedPool>`. No deduplication by pool address is applied in the current code; you may get the same pool from multiple protocols.

4. **Scanner**: Call `Scanner::start()` with no arguments. The scanner loads config, discovers pools (step 2), filters by token whitelist (step 3), then subscribes to swap/sync logs and invokes the price-change callback on each update.

## Public API

### PoolDiscovery

- **`PoolDiscovery::new()`** – Creates a discovery instance with an internal `SubgraphClient`.
- **`discover_pools(&self, protocols: &[ProtocolConfig], config: &DiscoveryConfig) -> Result<Vec<CachedPool>>`** – Fetches pools from each protocol’s subgraph and returns the concatenated list.

### SubgraphClient

- **`SubgraphClient::new()`** – HTTP client for The Graph.
- **`fetch_pools_from_protocol(&self, config: &ProtocolConfig, discovery_config: &DiscoveryConfig) -> Result<Vec<CachedPool>>`** – Runs the appropriate GraphQL query (V2 or V3) against `config.subgraph_url` and maps the response to `CachedPool`. On GraphQL errors, logs and returns an empty vec.

## Configuration

Discovery behavior is fully driven by:

- **ProtocolConfig** (from `load_protocols_file`): `subgraph_url`, `pool_type` (UniswapV2 vs UniswapV3), `enabled`.
- **DiscoveryConfig**: `min_liquidity_usd`, `max_pools_per_protocol`.

The `discovery` section in `protocols.json` maps to `DiscoveryConfig`; `cacheRefreshMinutes` is read but not used for caching in the current implementation.

## Token whitelist filtering

After discovery, pools can be filtered so that **only pools whose token0 and token1 are both in the token whitelist** (from `tokens.json`) are kept. Use `discovery::filter_pools_by_token_whitelist(pools, &whitelist)` where `whitelist` is a `HashSet<Address>` of token addresses (e.g. the values from `config::load_tokens_file()`). If the whitelist is empty, no filtering is applied and all discovered pools are returned. The scanner should be started with the filtered list so that only whitelisted pairs are tracked for price changes.

## CachedPool shape

Each discovered pool is a `CachedPool` with: `address`, `protocol`, `token0`/`token1` (addresses, symbols, decimals), `fee`, `liquidity_usd`, `volume_24h_usd`, `last_seen`. See `types::CachedPool` in the crate.
