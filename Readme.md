# Quotes System – Multithreaded Server & Client in Rust

This workspace contains two Rust crates that together form a complete **quotes streaming system**.  
It demonstrates practical use of multithreading, channels (MPMC), and TCP/UDP networking in Rust.

- **`quote-server`** – A TCP server that streams generated stock quotes to clients over UDP, with a keep‑alive (ping/pong) mechanism.
- **`quote-client`** – A command‑line client that connects to the server, requests a filtered set of tickers, and displays received quotes.

## Features

### Server
- Artificial stock quote generator (random walk for `AAPL`, `GOOGL`, `TSLA`).
- TCP command interface to start a new stream.
- UDP streaming per client, filtered by requested tickers.
- Keep‑alive using UDP ping messages – stops streaming if a client goes silent.
- Multi‑threaded: single generator thread distributes data to client threads via a broadcast channel.

### Client
- Reads ticker filters from a text file (one per line).
- Connects to the server via TCP to request a UDP stream.
- Listens on a user‑defined UDP port for incoming quotes.
- Sends periodic UDP ping messages to the server to keep the stream alive.
- Displays received quotes in JSON format.

## [!] Before using server side of the application please make sure you created .env file in the root of the project with configuration as in .env.example file

## Command examples
### 1). Run server:

```shell
cargo server
```
#### - run release build:
```shell
cargo r-server
```

### 2). Run client:
```shell
cargo client --udp-port 127.0.0.1:45222 --tickers-file ./quote-client/client_tickers.txt
```
#### - run release build:
```shell
cargo r-client --udp-port 127.0.0.1:45222 --tickers-file ./quote-client/client_tickers.txt
```