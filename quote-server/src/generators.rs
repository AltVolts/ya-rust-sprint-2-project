use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::time::{SystemTime, UNIX_EPOCH};

use quote_core::StockQuote;
use anyhow::Result;
use crossbeam_channel::Sender;
use log::{info, warn};
use rand::rngs::{StdRng, ThreadRng};
use rand::{Rng, SeedableRng};


const TICKERS_FILE: &str = "../../tickers.txt";

struct QuoteGenerator {
    tickers: Vec<String>,
    prices: HashMap<String, f64>,
    rng: rand::rngs::ThreadRng,
}

impl QuoteGenerator {
    fn new(tickers: Vec<String>) -> Self {
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

    pub fn generate_next_quote(&mut self) -> StockQuote {
        let ticker_idx = self.rng.gen_range(0..self.tickers.len());
        let ticker = self.tickers[ticker_idx].clone();

        let last_price = self.prices.get_mut(&ticker).unwrap();
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
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        StockQuote {
            ticker,
            price: *last_price,
            volume,
            timestamp,
        }
    }
}


/// Получение стартового списка тикеров из файла, участвующих в генерации
fn load_tickers_from_file(path: &str) -> Result<Vec<String>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut tickers = Vec::new();
    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            tickers.push(trimmed.to_string());
        }
    }
    Ok(tickers)
}

fn generator_thread(tx: Sender<StockQuote>) -> Result<()> {
    let tickers = if let Ok(tickers) = load_tickers_from_file(TICKERS_FILE) {
        tickers
    } else {
        warn!("Could not load tickers file, using default tickers");
        vec![
            "AAPL".to_string(),
            "GOOGL".to_string(),
            "TSLA".to_string(),
            "MSFT".to_string(),
            "AMZN".to_string(),
        ]
    };

    let mut generator = QuoteGenerator::new(tickers);
    loop {
        let quote = generator.generate_quote();
        if tx.send(quote).is_err() {
            // Все получатели отключились – завершаем поток
            break;
        }
        thread::sleep(QUOTE_GEN_INTERVAL);
    }
    Ok(())
}