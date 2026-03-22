
## [!] Before using server side of the application please make sure you created .env file in the root of the project with
configuration as in .env.example file

1). Run server:
```shell
cargo server
```

2). Run client:
```shell
cargo client --udp-port 127.0.0.1:45222 --tickers-file ./quote-client/client_tickers.txt
```