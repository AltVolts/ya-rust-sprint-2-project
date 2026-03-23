use anyhow::Result as AnyhowResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockQuote {
    pub ticker: String,
    pub price: f64,
    pub volume: u32,
    pub timestamp: u64,
}

pub type TickerPrices = HashMap<String, StockQuote>;

impl StockQuote {
    pub fn new(ticker: String, price: f64, volume: u32) -> AnyhowResult<Self> {
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64;
        Ok(StockQuote {
            ticker,
            price,
            volume,
            timestamp,
        })
    }

    pub fn header() -> String {
        format!(
            "{:<6} | {:>10} | {:>8} | {:>13}",
            "Ticker", "Price", "Volume", "Timestamp",
        )
    }
    pub fn separator() -> String {
        "-".repeat(46)
    }
}

impl Display for StockQuote {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:<6} | {:>10.2} | {:>8} | {:>13}",
            self.ticker, self.price, self.volume, self.timestamp
        )
    }
}

pub fn serialize_quotes(quotes: Vec<StockQuote>) -> bincode::Result<Vec<u8>> {
    bincode::serialize(&quotes)
}

pub fn deserialize_quotes(bytes: &[u8]) -> bincode::Result<Vec<StockQuote>> {
    bincode::deserialize(bytes)
}
