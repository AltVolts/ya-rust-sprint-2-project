use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::StockQuote;
use anyhow::Result;
use log::{info, warn};
use rand::rngs::{StdRng, ThreadRng};
use rand::{Rng, SeedableRng};

pub struct QuoteGenerator {
    prices: HashMap<String, f64>,
    rng: ThreadRng,
}

impl QuoteGenerator {
    pub fn new() -> Result<Self> {
        let mut tickers = get_tickers_from_txt()?;
        const SEED: u64 = 42;
        let mut rng = StdRng::seed_from_u64(SEED);

        for price in tickers.values_mut() {
            *price = rng.gen_range(100.0..999.0);
        }

        Ok(QuoteGenerator {
            prices: tickers,
            rng: rand::thread_rng(),
        })
    }

    pub fn generate_quote(&mut self, ticker: &str) -> Option<StockQuote> {
        let Some(last_price) = self.prices.get_mut(ticker) else {
            warn!("No ticker found for {}", ticker);
            return None;
        };
        let new_price = *last_price + self.rng.gen_range(-1.0..1.0);
        if new_price < 0.1 {
            *last_price = 1.0;
        } else {
            *last_price = new_price;
        }

        let volume = match ticker {
            "AAPL" | "MSFT" | "TSLA" => 1000 + (rand::random::<f64>() * 5000.0) as u32,
            _ => 100 + (rand::random::<f64>() * 1000.0) as u32,
        };

        Some(StockQuote {
            ticker: ticker.to_string(),
            price: *last_price,
            volume,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("System time is before Unix epoch")
                .as_millis() as u64,
        })
    }
}

fn get_tickers_from_txt() -> Result<HashMap<String, f64>> {
    let mut tickers = HashMap::new();
    let default_path = "../../tickers.txt";
    let file = match File::open(default_path) {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            info!("No tickers file found in {}", default_path);
            for &t in &["AAPL", "MSFT", "TSLA"] {
                tickers.insert(t.to_string(), 0.0);
            }
            return Ok(tickers);
        }
        Err(e) => return Err(e.into()),
    };
    let reader = BufReader::new(file);
    for line in reader.lines() {
        let ticker = line?.trim().to_string();
        if ticker.is_empty() {
            continue;
        }
        tickers.insert(ticker, 0.0);
    }
    Ok(tickers)
}
