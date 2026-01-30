use alloy::primitives::Address;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Protocol {
    #[serde(rename = "UniswapV2")]
    UniswapV2,
    #[serde(rename = "UniswapV3")]
    UniswapV3,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedPool {
    pub address: Address,
    pub protocol: String,
    pub token0: Address,
    pub token0_symbol: String,
    pub token0_decimals: u8,
    pub token1: Address,
    pub token1_symbol: String,
    pub token1_decimals: u8,
    pub fee: u32,
    pub liquidity_usd: f64,
    pub volume_24h_usd: f64,
    pub last_seen: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolPrice {
    pub pool_address: Address,
    pub token0_price: f64,
    pub token1_price: f64,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryConfig {
    pub min_liquidity_usd: f64,
    pub max_pools_per_protocol: u32,
    #[serde(default)]
    pub cache_enabled: bool,
    #[serde(default)]
    pub cache_file: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolConfig {
    pub id: String,
    pub name: String,
    pub subgraph_url: String,
    pub pool_type: Protocol,
    pub enabled: bool,
}
