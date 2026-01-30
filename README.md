# DEX Pool Scanner (Rust)

[![Crates.io](https://img.shields.io/crates/v/dex-pool-scanner-rust.svg)](https://crates.io/crates/dex-pool-scanner-rust)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)

Rust crate for multi-protocol DEX liquidity pool discovery and real-time price tracking on EVM networks. Discovers pools from multiple DEX protocols using [The Graph](https://thegraph.com/explorer) and is designed to monitor price changes via WebSocket. Mirrors the behavior of the TypeScript `dex-pool-scanner` package.

**Crate root:** All paths in this README and in [docs](docs/README.md) are relative to the **crate root** (the `rust/` directory). Run commands and config file lookups from `rust/` so that `protocols.json` and `tokens.json` resolve to `rust/protocols.json` and `rust/tokens.json` unless overridden with env vars.

## Supported Protocols

Full support for pool discovery (real-time price tracking via swap events is in progress):

- **Uniswap V3**
- **Uniswap V2**

Other protocols with V3-compatible subgraphs (e.g. Aerodrome CL, SushiSwap V3) work with `poolType: "UniswapV3"` in config.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
dex-pool-scanner-rust = "0.1"
```

Or clone the repo and use the path (from the repository root):

```toml
[dependencies]
dex-pool-scanner-rust = { path = "rust" }
```

## Quick Start

1. Create `protocols.json` at the crate root (`rust/protocols.json`) or set `PROTOCOLS_JSON`:

```json
{
  "protocols": {
    "uniswap-v3": {
      "name": "Uniswap V3",
      "factory": "0x33128a8fC17869897dcE68Ed026d694621f6FDfD",
      "subgraphId": "43Hwfi3dJSoGpyas9VwNoDAv55yjgGrPpNSmbQZArzMG",
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

2. Set environment variables (e.g. in `.env`):

```bash
THE_GRAPH_API_KEY=your_api_key
RPC_URL=wss://your-rpc-url
ENABLE_LOG=true
```

3. From the crate root (`rust/`), run the example:

```bash
cd rust
cargo run --example basic_discovery
```

## Build on top

```rust
use dex_pool_scanner_rust::{CachedPool, PoolPrice, Scanner};
use std::sync::Arc;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    dotenvy::dotenv().ok();

    let on_price_change = Arc::new(|pool: CachedPool, new_price: PoolPrice, old_price: Option<PoolPrice>| {
        // Log the parameters received in the callback
        tracing::info!("{} / {} [{}]: {:.6}", pool.token0_symbol, pool.token1_symbol, pool.protocol, new_price.token0_price);
        if let Some(old) = old_price {
            tracing::info!("   change: {:.4}%", (new_price.token0_price - old.token0_price) / old.token0_price * 100.0);
        }
    });

    let mut scanner = Scanner::new(on_price_change).await?;
    scanner.start().await?;

    tokio::signal::ctrl_c().await?;
    Ok(())
}
```

## Configuration

### protocols.json

Required configuration file:

- **protocols**: Map of protocol ID → config
  - `name`: Display name
  - `factory`: Factory contract address
  - `subgraphId`: The Graph subgraph ID
  - `enabled`: Enable/disable protocol
  - `poolType`: `"UniswapV3"` or `"UniswapV2"`
- **discovery**:
  - `minLiquidityUSD`: Minimum liquidity threshold (USD)
  - `cacheRefreshMinutes`: Reserved for future cache behavior
  - `maxPoolsPerProtocol`: Max pools to fetch per protocol

### tokens.json (Optional)

Token whitelist (symbol → address). Used for reference or future filtering.

```json
{
  "tokens": {
    "WETH": "0x4200000000000000000000000000000000000006",
    "USDC": "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913"
  }
}
```

Config paths default to `protocols.json` and `tokens.json` at the crate root (when run from `rust/`). Override with `PROTOCOLS_JSON` and `TOKENS_JSON`.

## API

### Config

- `config::load_protocols_file(path) -> Result<(Vec<ProtocolConfig>, DiscoveryConfig)>` – Load protocols and discovery from `protocols.json` (subgraph URLs built from `THE_GRAPH_API_KEY`).
- `config::load_tokens_file(path) -> Result<HashMap<String, Address>>` – Load token whitelist.

### Discovery

- `PoolDiscovery::new()` – Create discovery client.
- `discover_pools(&self, protocols, config) -> Result<Vec<CachedPool>>` – Fetch pools from all enabled protocols’ subgraphs.

### Scanner

- `Scanner::new(on_price_change) -> Result<Self>` – Create scanner with callback. Reads `RPC_URL` from the environment.
- `scanner.start() -> Result<()>` – Load config (`protocols.json`, `tokens.json`), discover pools, filter by token whitelist, and subscribe to price changes.
- `PriceChangeCallback`: `Arc<dyn Fn(CachedPool, PoolPrice, Option<PoolPrice>) + Send + Sync>`.

## Environment Variables

- `THE_GRAPH_API_KEY`: The Graph API key (required for discovery)
- `RPC_URL`: WebSocket RPC URL (required)
- `PROTOCOLS_JSON`: Path to `protocols.json` (default: `protocols.json` at crate root)
- `TOKENS_JSON`: Path to `tokens.json` (default: `tokens.json` at crate root)

## Documentation

All docs live under the crate root at `rust/docs/`:

- [Configuration](docs/configuration.md) – Config files and env vars
- [Pool Discovery](docs/pool-discovery.md) – How discovery works
- [Protocols](docs/protocols.md) – Supported protocols and types
- [Troubleshooting](docs/troubleshooting.md) – Common issues

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

MIT © [Maximiliano Malvido](https://github.com/maximalvido)
