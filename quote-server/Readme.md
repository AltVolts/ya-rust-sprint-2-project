# Quote Server

A multi‑threaded TCP server that streams generated stock quotes to clients over UDP.  
Clients can request a filtered stream of quotes for specific tickers.  
The server implements a keep‑alive mechanism (Ping/Pong) to stop streaming when a client disconnects.

## Features

- **Artificial data generation** – Simulates stock prices using a random walk for a predefined set of tickers (e.g., `AAPL`, `GOOGL`, `TSLA`).
- **TCP command interface** – Accepts client commands to start a UDP stream.
- **Filtered streaming** – Clients specify which tickers they want to receive.
- **UDP streaming** – Each client gets its own dedicated thread that sends quotes via UDP.
- **Keep‑alive (Ping/Pong)** – The server monitors incoming UDP ping messages; if no ping is received within a configurable timeout, the stream is terminated.
- **Multi‑threaded architecture** – A single data generator thread distributes quotes to all client threads using a shared channel (e.g., from the `crossbeam` crate).


## Protocol

### 1. TCP Command
Clients connect to the TCP command server and send a command in the following format:
`STREAM <udp_address> <ticker1>[,ticker2,...]`

- `<udp_address>` – The UDP address and port where the client wants to receive quotes (e.g., `127.0.0.1:45222`).
- `<ticker1>[,ticker2,...]` – A comma‑separated list of ticker symbols to receive (e.g., `AAPL,TSLA`).

**Example:**
`STREAM 127.0.0.1:45222 AAPL,GOOGL`


If the command is valid, the server responds with `OK <udp_ping_port>` where `<udp_ping_port>` is the port on which the server expects UDP ping messages from this client.

If the command is invalid (wrong format, unknown tickers), the server responds with `ERROR <description>`

### 2. UDP Streaming
After a successful command, the server starts sending datagrams to the client’s UDP address.  



### 3. Usage Example
Start the server:

```shell
cargo run
```
