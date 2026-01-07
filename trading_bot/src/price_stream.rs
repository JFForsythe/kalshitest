use futures::{SinkExt, StreamExt};
use tokio::{net::TcpStream, sync::broadcast, time::Duration};
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};
use tracing::{debug, warn};
use url::Url;

#[derive(Debug, Clone)]
pub struct PriceUpdate {
    pub instrument: String,
    pub bid: f64,
    pub ask: f64,
    pub timestamp: i64,
}

pub struct PriceStream {
    ws_url: Url,
    tx: broadcast::Sender<PriceUpdate>,
}

impl PriceStream {
    pub fn new(ws_url: Url) -> Self {
        let (tx, _rx) = broadcast::channel(1024);
        Self { ws_url, tx }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<PriceUpdate> {
        self.tx.subscribe()
    }

    async fn connect(&self) -> anyhow::Result<WebSocketStream<MaybeTlsStream<TcpStream>>> {
        let (ws, _resp) = connect_async(self.ws_url.clone()).await?;
        Ok(ws)
    }

    pub async fn run(&self, market: String) -> anyhow::Result<()> {
        let mut ws = self.connect().await?;

        // Simplified Kalshi-like subscription request
        let subscribe_msg = serde_json::json!({
            "action": "subscribe",
            "channels": [{ "name": "markets", "market": market }]
        });

        ws.send(Message::Text(subscribe_msg.to_string())).await?;

        while let Some(msg) = ws.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if let Ok(update) = parse_update(&text) {
                        let _ = self.tx.send(update);
                    } else {
                        debug!("ignored message: {text}");
                    }
                }
                Ok(Message::Ping(p)) => {
                    ws.send(Message::Pong(p)).await?;
                }
                Ok(Message::Close(frame)) => {
                    warn!(?frame, "websocket closed by server");
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    ws = self.connect().await?;
                    ws.send(Message::Text(subscribe_msg.to_string())).await?;
                }
                Err(err) => {
                    warn!(%err, "websocket error, retrying");
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    ws = self.connect().await?;
                    ws.send(Message::Text(subscribe_msg.to_string())).await?;
                }
                _ => {}
            }
        }

        Ok(())
    }
}

fn parse_update(raw: &str) -> Result<PriceUpdate, serde_json::Error> {
    #[derive(serde::Deserialize)]
    struct Envelope {
        market: Option<String>,
        bid: Option<f64>,
        ask: Option<f64>,
        ts: Option<i64>,
    }

    let env: Envelope = serde_json::from_str(raw)?;
    Ok(PriceUpdate {
        instrument: env.market.unwrap_or_else(|| "UNKNOWN".to_string()),
        bid: env.bid.unwrap_or_default(),
        ask: env.ask.unwrap_or_default(),
        timestamp: env.ts.unwrap_or_default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_update_defaults_missing_fields() {
        let json = "{\"market\":\"BTC\"}";
        let update = parse_update(json).unwrap();
        assert_eq!(update.instrument, "BTC");
        assert_eq!(update.bid, 0.0);
        assert_eq!(update.ask, 0.0);
    }
}
