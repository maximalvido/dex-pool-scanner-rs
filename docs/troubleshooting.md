# Troubleshooting (Rust crate)

## No pools discovered

- **THE_GRAPH_API_KEY**: Must be set. If unset, `load_protocols_file` skips all protocols (subgraph URL stays empty) and logs a warning. Set it in `.env` or the environment.
- **protocols.json**: Ensure the file is at the path used by the program (default: `protocols.json` at crate root `rust/`). Override with `PROTOCOLS_JSON` if needed.
- **Enabled protocols**: Only entries with `"enabled": true` are considered. At least one enabled protocol must have a valid subgraph URL.
- **Subgraph IDs**: Verify `subgraphId` values in `protocols.json` for your network. Wrong IDs can lead to empty or wrong results.
- **Liquidity threshold**: If `minLiquidityUSD` is too high, the subgraph may return few or no pools. Try lowering it in the `discovery` section.

## Config file not found

- **Working directory / crate root**: Config paths default to `protocols.json` and `tokens.json` at the crate root (`rust/`). Run from `rust/` so those files are found, or set `PROTOCOLS_JSON` and `TOKENS_JSON` to the desired paths.
- **Path override**: Use env vars `PROTOCOLS_JSON` and `TOKENS_JSON` to point to your config files.

## RPC / scanner issues

- **RPC_URL**: Must be a WebSocket URL (e.g. `wss://...`). The scanner uses `alloy` with `WsConnect`; HTTP-only URLs will not work for subscription-based logic once implemented.
- **Connection failures**: Check network, firewall, and RPC provider status. Ensure the provider supports the chain you target.

## Build / dependency issues

- **Edition**: Crate uses Rust 2024 edition; ensure your toolchain is up to date.
- **alloy / tokio / reqwest**: If you get version or feature errors, check `Cargo.toml` and that you use the same versions as the crate (alloy with `full` and `serde`, tokio with `full`, reqwest with `json`).

## Empty or invalid tokens.json

- **Optional file**: `load_tokens_file` is best-effort; missing file can return an empty map. Invalid JSON may still return an error.
- **Address format**: Token addresses must be valid hex (e.g. `0x...`). Invalid entries are skipped when parsing.

## GraphQL / subgraph errors

- **Errors in response**: If the subgraph returns GraphQL `errors`, the discovery module logs them and returns an empty list for that protocol. Check logs and subgraph status/ID.
- **Rate limiting**: The Graph gateway may rate-limit. If requests start failing, add backoff or reduce `maxPoolsPerProtocol`.

## Price updates not received

- **Current state**: The scanner does not yet subscribe to logs or call the price-change callback in the main flow. `handle_log_event` exists but is not invoked. Until log subscription is wired, you will not see price updates from the scanner.
