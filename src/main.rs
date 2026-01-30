use dotenvy::dotenv;
use dex_pool_scanner_rust::rpc::Scanner;
use dex_pool_scanner_rust::types::{CachedPool, PoolPrice};
use std::sync::Arc;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    dotenv().ok();

    let rpc_url = std::env::var("RPC_URL").expect("RPC_URL must be set");

    info!("Initializing DEX Pool Scanner (Rust Port)...");

    let on_price_change = Arc::new(|pool: CachedPool, new_price: PoolPrice, _old_price: Option<PoolPrice>| {
        info!("Price change for {}/{}: {}", pool.token0_symbol, pool.token1_symbol, new_price.token0_price);
    });

    let mut scanner = Scanner::new(&rpc_url, on_price_change).await?;

    // Placeholder: Initialize config and discover pools
    let pools = vec![]; // This would come from Discovery
    
    scanner.start(pools).await?;

    // Keep the main thread alive
    tokio::signal::ctrl_c().await?;
    info!("Shutting down...");

    Ok(())
}
