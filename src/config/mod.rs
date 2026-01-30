use crate::types::{DiscoveryConfig, Protocol, ProtocolConfig};
use std::collections::HashMap;
use std::fs;
use eyre::Result;

/// Format of each protocol entry in protocols.json (camelCase).
#[derive(serde::Deserialize)]
struct ProtocolEntry {
    name: String,
    #[allow(dead_code)]
    factory: String,
    #[serde(rename = "subgraphId")]
    subgraph_id: String,
    enabled: bool,
    #[serde(rename = "poolType")]
    pool_type: String,
}

/// Format of discovery section in protocols.json (camelCase).
#[derive(serde::Deserialize)]
struct DiscoveryEntry {
    #[serde(rename = "minLiquidityUSD")]
    min_liquidity_usd: f64,
    #[serde(rename = "cacheRefreshMinutes")]
    #[allow(dead_code)]
    cache_refresh_minutes: u32,
    #[serde(rename = "maxPoolsPerProtocol")]
    max_pools_per_protocol: u32,
}

/// Root format of protocols.json: { "protocols": { "id": {...} }, "discovery": {...} }
#[derive(serde::Deserialize)]
struct ProtocolsFile {
    protocols: HashMap<String, ProtocolEntry>,
    discovery: DiscoveryEntry,
}

/// Build subgraph URL from The Graph gateway and subgraph ID (env THE_GRAPH_API_KEY required).
fn subgraph_url_from_id(subgraph_id: &str, api_key: &str) -> String {
    format!(
        "https://gateway.thegraph.com/api/{}/subgraphs/id/{}",
        api_key, subgraph_id
    )
}

/// Load protocols.json and discovery config from the same file.
/// Expects format: { "protocols": { "id": { name, factory, subgraphId, enabled, poolType } }, "discovery": { minLiquidityUSD, cacheRefreshMinutes, maxPoolsPerProtocol } }.
/// Subgraph URL is built using THE_GRAPH_API_KEY. Returns only enabled protocols.
pub fn load_protocols_file(path: &str) -> Result<(Vec<ProtocolConfig>, DiscoveryConfig)> {
    let content = fs::read_to_string(path)?;
    let file: ProtocolsFile = serde_json::from_str(&content)?;

    let api_key = std::env::var("THE_GRAPH_API_KEY").unwrap_or_else(|_| {
        tracing::warn!("THE_GRAPH_API_KEY not set; subgraph URLs will be empty");
        String::new()
    });

    let mut protocols = Vec::new();
    for (id, entry) in file.protocols {
        if !entry.enabled {
            continue;
        }
        let subgraph_url = if api_key.is_empty() {
            String::new()
        } else {
            subgraph_url_from_id(&entry.subgraph_id, &api_key)
        };
        // Skip protocols we can't query (no subgraph URL)
        if subgraph_url.is_empty() {
            continue;
        }
        let pool_type = match entry.pool_type.as_str() {
            "UniswapV2" => Protocol::UniswapV2,
            _ => Protocol::UniswapV3,
        };
        protocols.push(ProtocolConfig {
            id,
            name: entry.name,
            subgraph_url,
            pool_type,
            enabled: entry.enabled,
        });
    }

    let discovery = DiscoveryConfig {
        min_liquidity_usd: file.discovery.min_liquidity_usd,
        max_pools_per_protocol: file.discovery.max_pools_per_protocol,
        cache_enabled: false,
        cache_file: String::new(),
    };

    Ok((protocols, discovery))
}

pub fn load_protocol_config(path: &str) -> Result<Vec<ProtocolConfig>> {
    let content = fs::read_to_string(path)?;
    let config: Vec<ProtocolConfig> = serde_json::from_str(&content)?;
    Ok(config)
}

pub fn load_discovery_config(path: &str) -> Result<DiscoveryConfig> {
    let content = fs::read_to_string(path)?;
    let config: DiscoveryConfig = serde_json::from_str(&content)?;
    Ok(config)
}

/// Root format of tokens.json: { "tokens": { "SYMBOL": "0x..." } }
#[derive(serde::Deserialize)]
struct TokensFile {
    tokens: HashMap<String, String>,
}

/// Load token whitelist from tokens.json. Returns symbol -> address map.
/// Optional: if file is missing or invalid, returns empty map.
pub fn load_tokens_file(path: &str) -> Result<HashMap<String, alloy::primitives::Address>> {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Ok(HashMap::new()),
    };
    let file: TokensFile = serde_json::from_str(&content)?;
    let mut out = HashMap::new();
    for (symbol, addr_str) in file.tokens {
        if let Ok(addr) = addr_str.parse() {
            out.insert(symbol, addr);
        }
    }
    Ok(out)
}
