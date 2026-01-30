use crate::config;
use crate::discovery::{filter_pools_by_token_whitelist, PoolDiscovery};
use crate::liquidity_pools::{BaseLiquidityPool, EthereumLog, UniswapV2, UniswapV3};
use crate::types::{CachedPool, PoolPrice};
use alloy::primitives::Address;
use alloy::providers::{Provider, ProviderBuilder, WsConnect};
use alloy::pubsub::PubSubFrontend;
use alloy::rpc::types::eth::{Filter, Log};
use eyre::Result;
use futures::StreamExt;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn};

pub type PriceChangeCallback = Arc<dyn Fn(CachedPool, PoolPrice, Option<PoolPrice>) + Send + Sync>;

struct ScannerState {
    pools: Vec<CachedPool>,
    liquidity_pools: HashMap<Address, Box<dyn BaseLiquidityPool>>,
    current_prices: HashMap<Address, PoolPrice>,
    on_price_change: PriceChangeCallback,
}

pub struct Scanner {
    provider: Arc<dyn Provider<PubSubFrontend>>,
    state: Arc<Mutex<ScannerState>>,
}

fn config_path(env_key: &str, default: &str) -> std::path::PathBuf {
    std::env::var(env_key)
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::env::current_dir().unwrap().join(default))
}

impl Scanner {
    /// Create a scanner with a price-change callback. Reads `RPC_URL` from the environment.
    pub async fn new(on_price_change: PriceChangeCallback) -> Result<Self> {
        let rpc_url = std::env::var("RPC_URL").map_err(|_| eyre::eyre!("RPC_URL must be set"))?;
        let ws = WsConnect::new(rpc_url);
        let provider = ProviderBuilder::new().on_ws(ws).await?;

        Ok(Self {
            provider: Arc::new(provider),
            state: Arc::new(Mutex::new(ScannerState {
                pools: vec![],
                liquidity_pools: HashMap::new(),
                current_prices: HashMap::new(),
                on_price_change,
            })),
        })
    }

    /// Load config from `protocols.json` and `tokens.json`, discover pools, filter by token whitelist, and subscribe to price changes.
    /// Config paths: `PROTOCOLS_JSON` (default `protocols.json`), `TOKENS_JSON` (default `tokens.json`), relative to current directory.
    pub async fn start(&mut self) -> Result<()> {
        let protocols_path = config_path("PROTOCOLS_JSON", "protocols.json");
        let tokens_path = config_path("TOKENS_JSON", "tokens.json");

        let (protocol_configs, discovery_config) =
            config::load_protocols_file(protocols_path.to_str().unwrap())?;
        let tokens = config::load_tokens_file(tokens_path.to_str().unwrap()).unwrap_or_default();
        let token_whitelist: HashSet<Address> = tokens.into_values().collect();

        if protocol_configs.is_empty() {
            warn!(
                "No enabled protocols (or THE_GRAPH_API_KEY unset). Set THE_GRAPH_API_KEY and enable protocols in protocols.json."
            );
        }

        let discovery = PoolDiscovery::new();
        let all_pools = discovery
            .discover_pools(&protocol_configs, &discovery_config)
            .await?;
        let pools = filter_pools_by_token_whitelist(all_pools, &token_whitelist);

        info!("Starting scanner for {} pools", pools.len());

        let addresses: Vec<Address> = pools.iter().map(|p| p.address).collect();

        let mut lp_map: HashMap<Address, Box<dyn BaseLiquidityPool>> = HashMap::new();
        for pool in &pools {
            let lp: Box<dyn BaseLiquidityPool> = if pool.protocol.to_lowercase().contains("v2") {
                Box::new(UniswapV2::new(
                    pool.address,
                    pool.token0_decimals,
                    pool.token1_decimals,
                ))
            } else {
                Box::new(UniswapV3::new(
                    pool.address,
                    pool.token0_decimals,
                    pool.token1_decimals,
                ))
            };
            lp_map.insert(pool.address, lp);
        }

        {
            let mut state = self.state.lock().await;
            state.pools = pools;
            state.liquidity_pools = lp_map;
        }

        let filter = Filter::new()
            .address(addresses)
            .events([
                "Swap(address,address,int256,int256,uint160,uint128,int24)".as_bytes(),   // V3
                "Swap(address,uint256,uint256,uint256,uint256,address)".as_bytes(),       // V2
                "Sync(uint112,uint112)".as_bytes(),                                        // V2
            ]);

        let provider = Arc::clone(&self.provider);
        let state = Arc::clone(&self.state);

        tokio::spawn(async move {
            if let Err(e) = run_log_subscription(provider, state, filter).await {
                warn!("Log subscription ended with error: {:?}", e);
            }
        });

        Ok(())
    }
}

async fn run_log_subscription(
    provider: Arc<dyn Provider<PubSubFrontend>>,
    state: Arc<Mutex<ScannerState>>,
    filter: Filter,
) -> Result<()> {
    let sub = provider.subscribe_logs(&filter).await?;
    let mut stream = sub.into_stream();

    while let Some(log) = stream.next().await {
        if let Err(e) = handle_log_event(&state, log).await {
            warn!("handle_log_event error: {:?}", e);
        }
    }

    Ok(())
}

async fn handle_log_event(state: &Arc<Mutex<ScannerState>>, log: Log) -> Result<()> {
    let pool_address = log.address();
    let eth_log = EthereumLog::from(log);

    let (swap_data, cached_pool) = {
        let mut guard = state.lock().await;
        let lp = guard.liquidity_pools.get_mut(&pool_address).ok_or_else(|| {
            eyre::eyre!("No liquidity pool for address {:?}", pool_address)
        })?;
        let swap_data = lp.parse_swap_event_data(&eth_log)?;
        let cached_pool = guard
            .pools
            .iter()
            .find(|p| p.address == pool_address)
            .cloned()
            .ok_or_else(|| eyre::eyre!("No cached pool for address {:?}", pool_address))?;
        (swap_data, cached_pool)
    };

    let new_price = PoolPrice {
        pool_address,
        token0_price: swap_data.price,
        token1_price: 1.0 / swap_data.price,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs(),
    };

    let old_price = {
        let mut guard = state.lock().await;
        guard.current_prices.insert(pool_address, new_price.clone())
    };

    (state.lock().await.on_price_change)(cached_pool, new_price, old_price);

    Ok(())
}
