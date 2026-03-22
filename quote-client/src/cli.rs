use clap::Parser;
use log::info;
use std::net::SocketAddr;

#[derive(Parser)]
pub struct Cli {
    /// Адрес и порт сервера для TCP-подключения
    #[arg(short, long, default_value_t = SocketAddr::from(([127, 0, 0, 1], 8080)))]
    pub(crate) server_addr: SocketAddr,

    /// Адрес и порт клиента для получения данных по UDP
    #[arg(short, long, required = true)]
    pub(crate) udp_port: SocketAddr,

    /// Путь к файлу со списком тикеров
    #[arg(short, long, required = true)]
    pub(crate) tickers_file: String,
}

impl Cli {
    pub fn get_args() -> Self {
        let result = Self::parse();
        info!("Сервер (TCP): {}", result.server_addr);
        info!("Клиент (UDP): {}", result.udp_port);
        info!(
            "Файл с фильтром по интересующим тикерам: {:?}\n",
            result.tickers_file
        );

        result
    }
}
