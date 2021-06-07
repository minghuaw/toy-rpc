# Examples

There are a few examples available on the [GitHub repo](https://github.com/minghuaw/toy-rpc/tree/main/examples). Here are some descriptions of those examples.

 - ["Raw" TCP server and client with `async-std` runtime](https://github.com/minghuaw/toy-rpc/tree/main/examples/async_std_tcp)
 - ["Raw" TCP server and client with `tokio` runtime](https://github.com/minghuaw/toy-rpc/tree/main/examples/tokio_tcp)
 - [Cancellation and timeout of RPC call](https://github.com/minghuaw/toy-rpc/tree/main/examples/cancel_and_timeout)
 - Service definition, server implementation, and client implementation in three separate crates
    - [service](https://github.com/minghuaw/toy-rpc/tree/main/examples/example-service)
    - [server](https://github.com/minghuaw/toy-rpc/tree/main/examples/example-server)
    - [client](https://github.com/minghuaw/toy-rpc/tree/main/examples/example-client)
 - HTTP integrations
    - [`actix-web`](https://github.com/minghuaw/toy-rpc/tree/main/examples/actix_v3_integration)\
    - [`tide`](https://github.com/minghuaw/toy-rpc/tree/main/examples/tide_integration)
    - [`warp`](https://github.com/minghuaw/toy-rpc/tree/main/examples/warp_integration)