use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::client_registry::ClientRegistry;
use anyhow::{Result, anyhow};
use quote_core::{StockQuote, TickerPrices};
use rand::Rng;
use rand::rngs::ThreadRng;

const GENERATOR_TICK_RATE: Duration = Duration::from_millis(2000);

pub(crate) struct QuoteGenerator {
    client_registry: Arc<Mutex<ClientRegistry>>,
    prices: HashMap<String, f64>,
    rng: ThreadRng,
}

impl QuoteGenerator {
    pub(crate) fn new(client_registry: Arc<Mutex<ClientRegistry>>, tickers: Vec<String>) -> Self {
        let mut rng = rand::thread_rng();
        let mut prices = HashMap::new();

        for ticker in &tickers {
            let price = rng.gen_range(50.0..=500.0);
            prices.insert(ticker.clone(), price);
        }

        Self {
            client_registry,
            prices,
            rng,
        }
    }

    pub fn generate_all_quotes(&mut self) -> Result<TickerPrices> {
        let mut result = TickerPrices::with_capacity(self.prices.len());
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64;

        for ticker in self.prices.clone().keys() {
            let last_price = self.prices.get_mut(ticker).unwrap();
            let change = self.rng.gen_range(-2.0..2.0);
            *last_price += change;
            if *last_price < 0.01 {
                *last_price = 0.01;
            }

            let volume = match ticker.as_str() {
                "AAPL" | "MSFT" | "TSLA" => 1000 + self.rng.gen_range(0..5000),
                _ => 100 + self.rng.gen_range(0..1000),
            };

            result.insert(
                ticker.to_string(),
                StockQuote {
                    ticker: ticker.clone(),
                    price: *last_price,
                    volume,
                    timestamp: now,
                },
            );
        }

        Ok(result)
    }

    pub fn start_generation(&mut self) -> Result<()> {
        loop {
            let prices = self.generate_all_quotes()?;
            let guard = self
                .client_registry
                .lock()
                .map_err(|e| anyhow!("Failed to lock client registry: {}", e))?;
            guard
                .broadcast(&prices)
                .map_err(|e| anyhow!("Broadcast failed: {}", e))?;

            thread::sleep(GENERATOR_TICK_RATE);
        }
    }
}
