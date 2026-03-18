use std::collections::HashMap;

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use quote_core::StockQuote;
use anyhow::{anyhow, Result};
use crossbeam_channel::Sender;
use log::{error};
use rand::rngs::{ ThreadRng};
use rand::{Rng};


const TICKERS_FILE: &str = "../../tickers.txt";
const GENERATOR_TICK_RATE: Duration = Duration::from_millis(100);

pub(crate) struct QuoteGenerator {
    tickers: Vec<String>,
    prices: HashMap<String, f64>,
    rng: ThreadRng,
}

impl QuoteGenerator {
    pub(crate) fn new(tickers: Vec<String>) -> Self {
        let mut rng = rand::thread_rng();
        let mut prices = HashMap::new();

        for ticker in &tickers {
            let price = rng.gen_range(50.0..=500.0);
            prices.insert(ticker.clone(), price);
        }

        Self {
            tickers,
            prices,
            rng
        }
    }

    pub fn generate_next_quote(&mut self) -> Result<StockQuote> {
        let ticker_idx = self.rng.gen_range(0..self.tickers.len());
        let ticker = self.tickers[ticker_idx].clone();

        let last_price = self.prices.get_mut(&ticker).ok_or_else(|| anyhow!("Error of borrowing tickers price"))?;
        let change = self.rng.gen_range(-1.0..=1.0);
        *last_price += change;
        if *last_price < 0.01 {
            *last_price = 0.01;
        }

        let volume = match ticker.as_str() {
            "AAPL" | "MSFT" | "TSLA" => 1000 + self.rng.gen_range(0..5000),
            _ => 100 + self.rng.gen_range(0..1000),
        };

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_millis() as u64;

        Ok(StockQuote {
            ticker,
            price: *last_price,
            volume,
            timestamp,
        })
    }

    pub fn generator_thread(&mut self, tx: Sender<StockQuote>) {
        loop {
            match self.generate_next_quote() {
                Ok(quote) => {
                    if let Err(e) = tx.send(quote) {
                        error!("Generator send error: {}, stopping", e);
                        break;
                    }
                }
                Err(e) => {
                    error!("Generator error: {}", e);
                    break;
                }
            }
            std::thread::sleep(GENERATOR_TICK_RATE);
        }
    }
}