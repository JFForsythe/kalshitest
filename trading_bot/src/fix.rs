use rand::Rng;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;
use tokio::time::Instant;

#[derive(Debug, Error)]
pub enum FixError {
    #[error("connection failure: {0}")]
    Connection(String),
    #[error("order rejected: {0}")]
    Rejected(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Side {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderType {
    Market,
    Limit(f64),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderRequest {
    pub instrument: String,
    pub side: Side,
    pub quantity: f64,
    pub order_type: OrderType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderAck {
    pub order_id: String,
    pub sent_at: Instant,
    pub acked_at: Instant,
}

#[async_trait::async_trait]
pub trait OrderEntry: Send + Sync {
    async fn send_order(&self, req: OrderRequest) -> Result<OrderAck, FixError>;
    async fn modify_order(&self, order_id: &str, new_price: f64) -> Result<OrderAck, FixError>;
}

/// A lightweight mock FIX client that simulates network latency and exchange handling time.
pub struct MockFixClient {
    pub min_latency: Duration,
    pub max_latency: Duration,
}

impl MockFixClient {
    pub fn new(min_latency: Duration, max_latency: Duration) -> Self {
        Self {
            min_latency,
            max_latency,
        }
    }

    async fn simulate_round_trip(&self) -> Duration {
        let mut rng = rand::thread_rng();
        let jitter_ms: u64 = rng
            .gen_range(self.min_latency.as_millis() as u64..=self.max_latency.as_millis() as u64);
        let delay = Duration::from_millis(jitter_ms);
        tokio::time::sleep(delay).await;
        delay
    }
}

#[async_trait::async_trait]
impl OrderEntry for MockFixClient {
    async fn send_order(&self, req: OrderRequest) -> Result<OrderAck, FixError> {
        let sent_at = Instant::now();
        let _latency = self.simulate_round_trip().await;
        let acked_at = Instant::now();
        if matches!(req.order_type, OrderType::Limit(price) if price <= 0.0) {
            return Err(FixError::Rejected("limit price must be positive".into()));
        }
        Ok(OrderAck {
            order_id: format!("SIM-{}", sent_at.elapsed().as_nanos()),
            sent_at,
            acked_at,
        })
    }

    async fn modify_order(&self, _order_id: &str, _new_price: f64) -> Result<OrderAck, FixError> {
        let sent_at = Instant::now();
        let _latency = self.simulate_round_trip().await;
        let acked_at = Instant::now();
        Ok(OrderAck {
            order_id: format!("MOD-{}", sent_at.elapsed().as_nanos()),
            sent_at,
            acked_at,
        })
    }
}
