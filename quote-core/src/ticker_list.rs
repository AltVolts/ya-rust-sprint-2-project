use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader};
use anyhow::{Context, Result};


/// Получить множество тикеров
pub fn get_tickers_from_txt(path: &str) -> Result<HashSet<String>> {
    let file = File::open(path)
        .with_context(|| format!("No tickers file found in {}", path))?;

    let mut tickers =HashSet::new();
    let reader = BufReader::new(file);
    for line in reader.lines() {
        let ticker = line?.trim().to_string();
        if !ticker.is_empty() {
            tickers.insert(ticker);
        }
    }
    Ok(tickers)
}