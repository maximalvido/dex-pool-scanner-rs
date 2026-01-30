# dex-pool-scanner-rust Documentation

Rust crate for discovering DEX liquidity pools via The Graph subgraphs and tracking pool prices over WebSocket RPC. It mirrors the behavior of the TypeScript `dex-pool-scanner` package.

**Crate root:** The crate root is the `rust/` directory. All paths (e.g. `protocols.json`, `tokens.json`) in this doc set are relative to the crate root when running from `rust/`.

## Overview

- **Config**: Load `protocols.json` and `tokens.json`; subgraph URLs are built from `THE_GRAPH_API_KEY`.
- **Discovery**: Fetch pools from enabled protocols' subgraphs, filtered by liquidity and limits.
- **Scanner**: Hold discovered pools and (when implemented) subscribe to swap events to drive price callbacks.

## Documentation Index

| Document | Description |
|----------|-------------|
| [Configuration](configuration.md) | `protocols.json`, `tokens.json`, environment variables, and the `config` module API. |
| [Pool Discovery](pool-discovery.md) | How discovery works, `PoolDiscovery`, `SubgraphClient`, and configuration. |
| [Protocols](protocols.md) | Supported protocols (Uniswap V2/V3), `BaseLiquidityPool`, types, and the RPC scanner. |
| [Troubleshooting](troubleshooting.md) | Common issues and fixes for the Rust crate. |

## Quick Start

1. Add the crate to your `Cargo.toml` or run the example from the `rust/` directory.
2. Set environment variables (e.g. in `.env`):
   - `RPC_URL` – WebSocket RPC URL (required).
   - `THE_GRAPH_API_KEY` – Required for subgraph discovery.
3. Place `protocols.json` and optionally `tokens.json` at the crate root (`rust/`), or set `PROTOCOLS_JSON` / `TOKENS_JSON`.
4. From the crate root (`rust/`), run the example:

```bash
cd rust
cargo run --example basic_discovery
```

See [Configuration](configuration.md) for file formats and [Pool Discovery](pool-discovery.md) for the discovery flow.
