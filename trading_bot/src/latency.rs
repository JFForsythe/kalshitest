use crate::fix::{OrderEntry, OrderRequest, OrderType, Side};
use tokio::time::Instant;
use tracing::info;

#[derive(Debug, Clone)]
pub struct LatencySample {
    pub path: &'static str,
    pub elapsed_ms: f64,
}

#[derive(Debug, Clone)]
pub struct LatencyReport {
    pub market_buy_ms: f64,
    pub modify_cross_ms: f64,
}

pub async fn benchmark_order_paths<E: OrderEntry>(
    client: &E,
    instrument: &str,
    qty: f64,
    cross_price: f64,
) -> anyhow::Result<LatencyReport> {
    let market_start = Instant::now();
    let market_ack = client
        .send_order(OrderRequest {
            instrument: instrument.to_string(),
            side: Side::Buy,
            quantity: qty,
            order_type: OrderType::Market,
        })
        .await?;
    let market_elapsed = market_ack.acked_at.duration_since(market_start);

    let limit_order = client
        .send_order(OrderRequest {
            instrument: instrument.to_string(),
            side: Side::Buy,
            quantity: qty,
            order_type: OrderType::Limit(cross_price - 10.0),
        })
        .await?;
    let modify_start = Instant::now();
    let modify_ack = client
        .modify_order(&limit_order.order_id, cross_price)
        .await?;
    let modify_elapsed = modify_ack.acked_at.duration_since(modify_start);

    let report = LatencyReport {
        market_buy_ms: market_elapsed.as_secs_f64() * 1_000.0,
        modify_cross_ms: modify_elapsed.as_secs_f64() * 1_000.0,
    };

    info!(
        "Latency -- market: {:.2} ms | modify-cross: {:.2} ms",
        report.market_buy_ms, report.modify_cross_ms
    );

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fix::MockFixClient;
    use std::time::Duration;

    #[tokio::test]
    async fn benchmark_runs_with_mock() {
        let client = MockFixClient::new(Duration::from_millis(2), Duration::from_millis(4));
        let report = benchmark_order_paths(&client, "BTC-HOURLY", 1.0, 30000.0)
            .await
            .unwrap();
        assert!(report.market_buy_ms > 0.0);
        assert!(report.modify_cross_ms > 0.0);
    }
}
