use anyhow::Result as AnyhowResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockQuote {
    pub ticker: String,
    pub price: f64,
    pub volume: u32,
    pub timestamp: u64,
}

pub type TickerPrices = HashMap<String, StockQuote>;

pub trait TickerPricesExt {
    fn to_json(&self) -> serde_json::Result<String>;
    fn from_json(s: &str) -> serde_json::Result<HashMap<String, StockQuote>>;

    fn serialize(&self) -> bincode::Result<Vec<u8>>;
    fn deserialize(bytes: &[u8]) -> bincode::Result<HashMap<String, StockQuote>>;
}

impl TickerPricesExt for HashMap<String, StockQuote> {
    fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }

    fn from_json(s: &str) -> serde_json::Result<HashMap<String, StockQuote>> {
        serde_json::from_str(s)
    }

    fn serialize(&self) -> bincode::Result<Vec<u8>> {
        bincode::serialize(self)
    }

    fn deserialize(bytes: &[u8]) -> bincode::Result<HashMap<String, StockQuote>> {
        bincode::deserialize(bytes)
    }
}

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

    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }

    pub fn from_json(s: &str) -> serde_json::Result<Self> {
        serde_json::from_str(s)
    }
}
