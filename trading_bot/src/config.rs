use std::env;
use url::Url;

#[derive(Debug, Clone)]
pub struct Config {
    pub websocket_url: Url,
    pub fix_target: String,
    pub market: String,
    pub account: String,
    pub dry_run: bool,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let websocket_url = env::var("WEBSOCKET_URL").unwrap_or_else(|_| {
            "wss://demo-exchange.kalshi.com/trade-api/v2/market-data".to_string()
        });
        let fix_target =
            env::var("FIX_TARGET").unwrap_or_else(|_| "demo-fix.kalshi.com:1234".to_string());
        let market = env::var("MARKET").unwrap_or_else(|_| "BTC-HOURLY".to_string());
        let account = env::var("ACCOUNT").unwrap_or_else(|_| "SIM-ACCOUNT".to_string());
        let dry_run = env::var("DRY_RUN")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(true);

        Ok(Self {
            websocket_url: Url::parse(&websocket_url)?,
            fix_target,
            market,
            account,
            dry_run,
        })
    }
}
