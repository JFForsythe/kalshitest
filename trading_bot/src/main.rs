mod config;
mod fix;
mod latency;
mod price_stream;

use config::Config;
use fix::MockFixClient;
use latency::benchmark_order_paths;
use price_stream::PriceStream;
use tokio::time::Duration;
use tracing::{Level, info};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_max_level(Level::INFO)
        .init();

    let config = Config::from_env()?;
    info!(?config, "booting trading bot");

    let price_stream = PriceStream::new(config.websocket_url.clone());
    let stream_runner = tokio::spawn(price_stream.run(config.market.clone()));
    let mut price_rx = price_stream.subscribe();

    let price_task = tokio::spawn(async move {
        while let Ok(update) = price_rx.recv().await {
            info!(?update, "price update");
        }
    });

    // In production you would swap this mock for a real FIX session implementation.
    let fix_client = MockFixClient::new(Duration::from_millis(3), Duration::from_millis(8));
    let _bench = benchmark_order_paths(&fix_client, &config.market, 1.0, 30_000.0).await?;

    // Keep price stream alive for demonstration; in a daemon, you would await the runner instead.
    price_task.abort();
    stream_runner.abort();

    Ok(())
}
