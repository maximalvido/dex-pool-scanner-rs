use dex_pool_scanner_rust::{CachedPool, PoolPrice, Scanner};
use std::sync::Arc;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use dotenvy::dotenv;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    dotenv().ok();
    tracing::subscriber::set_global_default(
        FmtSubscriber::builder().with_max_level(Level::INFO).finish(),
    )?;

    let on_price_change = Arc::new(
        |pool: CachedPool, new_price: PoolPrice, old_price: Option<PoolPrice>| {
            let change = match &old_price {
                Some(old) => format!(
                    "{:.4}",
                    (new_price.token0_price - old.token0_price) / old.token0_price * 100.0
                ),
                None => "N/A".to_string(),
            };

            info!("Price Update: {}/{} [{}]", pool.token0_symbol, pool.token1_symbol, pool.protocol);
            info!("   Pool: {:?}", pool.address);
            info!(
                "   {} price: {:.6} {}",
                pool.token0_symbol, new_price.token0_price, pool.token1_symbol
            );
            info!(
                "   {} price: {:.6} {}",
                pool.token1_symbol, new_price.token1_price, pool.token0_symbol
            );
            if old_price.is_some() {
                info!("   Change: {}%", change);
            }
        },
    );

    let mut scanner = Scanner::new(on_price_change).await?;
    scanner.start().await?;
    info!("Scanner running. Press Ctrl+C to stop.");
    tokio::signal::ctrl_c().await?;

    Ok(())
}
