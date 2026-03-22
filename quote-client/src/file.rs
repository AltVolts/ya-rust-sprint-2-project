use anyhow::Result as AnyhowError;
use quote_core::ticker_list::get_tickers_from_txt;

pub(crate) fn read_tickers_file(tickers_file: String) -> AnyhowError<String> {
    let tickers: Vec<String> = get_tickers_from_txt(tickers_file.as_str())?
        .into_iter()
        .collect();

    let tickers_str = tickers.join(",");
    Ok(tickers_str)
}
