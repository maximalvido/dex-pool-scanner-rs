use crate::types::{CachedPool, Protocol, ProtocolConfig, DiscoveryConfig};
use reqwest::Client;
use serde_json::json;
use std::collections::HashSet;
use eyre::Result;
use tracing::{info, error};

pub struct SubgraphClient {
    client: Client,
}

impl SubgraphClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    pub async fn fetch_pools_from_protocol(&self, config: &ProtocolConfig, discovery_config: &DiscoveryConfig) -> Result<Vec<CachedPool>> {
        if !config.enabled {
            return Ok(vec![]);
        }

        info!("Fetching pools from {} subgraph...", config.name);

        let query = if config.pool_type == Protocol::UniswapV2 {
            r#"
            query GetV2Pairs($first: Int!, $minLiquidityUSD: BigDecimal!) {
                pairs(
                    first: $first
                    orderBy: reserveUSD
                    orderDirection: desc
                    where: { reserveUSD_gte: $minLiquidityUSD }
                ) {
                    id
                    token0 { id symbol decimals }
                    token1 { id symbol decimals }
                    reserveUSD
                    volumeUSD
                }
            }
            "#
        } else {
            r#"
            query GetV3Pools($first: Int!, $minLiquidityUSD: BigDecimal!) {
                pools(
                    first: $first
                    orderBy: totalValueLockedUSD
                    orderDirection: desc
                    where: { totalValueLockedUSD_gte: $minLiquidityUSD }
                ) {
                    id
                    token0 { id symbol decimals }
                    token1 { id symbol decimals }
                    feeTier
                    totalValueLockedUSD
                    volumeUSD
                }
            }
            "#
        };

        let response = self.client.post(&config.subgraph_url)
            .json(&json!({
                "query": query,
                "variables": {
                    "first": discovery_config.max_pools_per_protocol,
                    "minLiquidityUSD": discovery_config.min_liquidity_usd.to_string()
                }
            }))
            .send()
            .await?;

        let data: serde_json::Value = response.json().await?;

        if let Some(errors) = data.get("errors") {
            error!("GraphQL errors from {}: {:?}", config.name, errors);
            return Ok(vec![]);
        }

        let pools_json = if config.pool_type == Protocol::UniswapV2 {
            data.get("data").and_then(|d| d.get("pairs"))
        } else {
            data.get("data").and_then(|d| d.get("pools"))
        };

        let mut cached_pools = Vec::new();

        if let Some(pools) = pools_json.and_then(|p| p.as_array()) {
            for pool in pools {
                let address: alloy::primitives::Address = pool.get("id").and_then(|v| v.as_str()).unwrap_or_default().parse().unwrap_or_default();
                let token0 = pool.get("token0").unwrap();
                let token1 = pool.get("token1").unwrap();
                
                cached_pools.push(CachedPool {
                    address,
                    protocol: config.id.clone(),
                    token0: token0.get("id").and_then(|v| v.as_str()).unwrap_or_default().parse().unwrap_or_default(),
                    token0_symbol: token0.get("symbol").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                    token0_decimals: token0.get("decimals").and_then(|v| v.as_str()).and_then(|v| v.parse().ok()).unwrap_or(18),
                    token1: token1.get("id").and_then(|v| v.as_str()).unwrap_or_default().parse().unwrap_or_default(),
                    token1_symbol: token1.get("symbol").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                    token1_decimals: token1.get("decimals").and_then(|v| v.as_str()).and_then(|v| v.parse().ok()).unwrap_or(18),
                    fee: pool.get("feeTier").and_then(|v| v.as_str()).and_then(|v| v.parse().ok()).unwrap_or(0),
                    liquidity_usd: pool.get("totalValueLockedUSD").or(pool.get("reserveUSD")).and_then(|v| v.as_str()).and_then(|v| v.parse().ok()).unwrap_or(0.0),
                    volume_24h_usd: pool.get("volumeUSD").and_then(|v| v.as_str()).and_then(|v| v.parse().ok()).unwrap_or(0.0),
                    last_seen: chrono::Utc::now().to_rfc3339(),
                });
            }
        }
        
        Ok(cached_pools)
    }
}

pub struct PoolDiscovery {
    subgraph_client: SubgraphClient,
}

impl PoolDiscovery {
    pub fn new() -> Self {
        Self {
            subgraph_client: SubgraphClient::new(),
        }
    }

    pub async fn discover_pools(&self, protocols: &[ProtocolConfig], config: &DiscoveryConfig) -> Result<Vec<CachedPool>> {
        let mut all_pools = Vec::new();
        for protocol in protocols {
            let pools = self.subgraph_client.fetch_pools_from_protocol(protocol, config).await?;
            all_pools.extend(pools);
        }
        Ok(all_pools)
    }
}

/// Filter pools to only those whose token0 and token1 are both in the token whitelist.
/// If `whitelist` is empty, returns `pools` unchanged (no filtering).
pub fn filter_pools_by_token_whitelist(
    pools: Vec<CachedPool>,
    whitelist: &HashSet<alloy::primitives::Address>,
) -> Vec<CachedPool> {
    if whitelist.is_empty() {
        return pools;
    }
    pools
        .into_iter()
        .filter(|p| whitelist.contains(&p.token0) && whitelist.contains(&p.token1))
        .collect()
}
