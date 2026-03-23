# Quote client binary application
## Connects to quotes server to get new generated list of stock quotes according to client filter

### To launch the app you need to define udp address and list of ticker in client_tickers.txt

### You must define `--udp-port` and `--tickers-file` cli-arguments 
### and optionally you can define `--server-addr` cli-argument (default value is `127.0.0.1:8080`)
### The also has short way to define

### Examples of commands to start the app:


# Quote Client

A binary application that connects to a quotes server and retrieves a list of stock quotes based on client‑defined filters.

## Features

- Connects to a quote server (default address: `127.0.0.1:8080`).
- Reads ticker symbols from a specified file (`client_tickers.txt` by convention).
- Receives quotes via UDP on a configurable local address and port.

## Command Line Arguments

| Argument            | Short | Required | Description                                                                  | Default           |
|---------------------|-------|----------|------------------------------------------------------------------------------|-------------------|
| `--server-addr`     | `-s`  | No       | Address (IP:port) of the quotes server.                                      | `127.0.0.1:8080`  |
| `--udp-port`        | `-u`  | **Yes**  | Local UDP address and port on which the client listens for quotes.           | —                 |
| `--tickers-file`    | `-t`  | **Yes**  | Path to a text file containing one ticker symbol per line.                   | —                 |

> **Note:** The `--udp-port` argument expects an address in the format `IP:port` (e.g., `127.0.0.1:45222`).

## Usage Examples

### Minimal start (server address default)
```shell
cargo run -- --udp-port 127.0.0.1:45222 --tickers-file ./client_tickers.txt
```

### Explicit server address
```shell
cargo run -- --server-addr 127.0.0.1:8080 --udp-port 127.0.0.1:45222 --tickers-file ./client_tickers.txt
```

### Using short options
```shell
cargo run -- -s 127.0.0.1:8080 -u 127.0.0.1:45222 -t ./client_tickers.txt
```