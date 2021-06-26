# Client

The client side usage should feel fairly close to that of the golang's `net/rpc` package as well except some changes that makes the client API more rusty and async. 

To connect a client to a "raw" TCP server, you should use the `dial` function; to connect to a HTTP server, you should then use the `dial_http` function. Once you have a connected client,
you can then use the `call_blocking` and `call` methods to access the RPC functions on the server. The `#[export_impl]` and `#[export_trait]` attribute macros also generates client stub functions that allows the client to conveniently access the RPC functions without worrying about typing the wrong service or method name. 

# Connecting to TCP server / HTTP server

For the examples below, we will assume running with the `tokio` runtime.

```toml
[dependencies]
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
toy-rpc = { version = "0.7.5", features = ["tokio_runtime", "client"] }
```

The example below shows how to connect to a TCP server

```rust,noplaypen 
use toy_rpc::Client;

#[tokio::main]
async fn main() {
    let client = Client::dial("127.0.0.1:23333").await
        .expect("Failed to connect to server");
}
```

Connecting to the HTTP server looks very similar with some minor changes. The example below assumes that we are trying to connect to one of the [HTTP servers](https://minghuaw.github.io/toy-rpc/05_integration.html) (all three HTTP integration examples have the RPC server serving at `"ws://127.0.0.1:23333/rpc/"`). Please note that there is a "_rpc_" appended to the end of the path by the server integration methods, but this is automatically handled by the `Client::dial_http` method so you don't need to worry about that. 

```rust,noplaypen 
use toy_rpc::Client;

#[tokio::main]
async fn main() {
    let client = Client::dial_http("ws://127.0.0.1:23333/rpc/").await
        .expect("Failed to connect to server");
}
```
# Accessing RPC services

## `call_blocking(...)` and `call(...)`

There are two methods available for accessing the RPC services and methods. The `call_blocking` method blocks the execution until the response is received, and the `call` method is the asynchronous version where the execution will yield to other tasks scheduled by the runtime while waiting for the response. Cancellation is also supported on the `call` method, which is discussed with more details in the [next chapter](https://minghuaw.github.io/toy-rpc/06a_cancellation.html).

(Support of timeout is still work-in-progress. The book will be updated once the feature is implemented.)


## Generated client stub functions

The generated client stub functions internally uses the `call(...)` method and are thus async. The client stub functions consist of two steps. The first step is to access your service, and the second step is to access the method defined in that particular service. The method in the first step is always the name of the service but in snake case. For example, if you have a service `struct FooBar { }`, then the client method you use to access the service will be `foo_bar()`. The client method you use to access the method is identical to the method definition in the RPC service. For example, if an RPC method is defined as `fn add(&self, args(i32, i32)) -> Result<i32, String>;`, then the client method you use would be `client.bar().add((3i32, 4i32)).await;`

## Examples

We will continue the [`#[export_impl]` example](https://minghuaw.github.io/toy-rpc/03_define_service.html#export_impl) and the [`#[export_trait]` and `#[export_trait_impl]`] example to demonstrate how to access RPC service on the server. The methods you will use to access the RPC service are the same for a TCP connection and a HTTP connection, and for simplicity, all the examples below will assume a TCP connection. For more examples on use with HTTP connections, please checkout the [GitHub examples](https://github.com/minghuaw/toy-rpc/tree/main/examples).
### `#[export_impl]`

Let's just remind ourselves that in this example the service definition and implementation are located in the same file/project, and the file structure is as follows

```
./src
├── /bin
│   ├── server.rs
│   ├── client.rs
└── lib.rs
```

Since the service is defined and implemented in `src/lib.rs` from the [previous chapter](https://minghuaw.github.io/toy-rpc/03_define_service.html#export_impl), we are going to include everything in the `src/lib.rs` file to allow us use the generated client stub functions in our `src/bin/client.rs`.

```rust,noplaypen,noplaypen
use toy_rpc::Client;
use toy_rpc::Error;

// include everything from the lib.rs file
// assuming the name of the crate is "example"
use example::*;

#[tokio::main]
async fn main() {
    let client = Client::dial("127.0.0.1:23333").await
        .expect("Failed to connect to the server");

    // Access the remote `exported_method` method of `Foo` service in a blocking manner
    let result: Result<String, Error> = client.call_blocking("Foo.exported_method", ());
    println!("{:?}", result);

    // Access the remote `add` method of `Bar` service in an asynchronous manner
    let result: Result<i32, Error> = client.call("Bar.add", (3i32, 4i32)).await;
    println!("{:?}", result);

    // You can also use the generated client stub functions
    // Access the remote `exported_method` method of `Foo` service 
    // using the generated client stub functions
    let result = client
        .foo() // access `Foo` service
        .exported_method(()) // access `exported_method` of the `Foo` service
        .await;
    println!("{:?}", result);

    // Access the remote `add` method of `Bar` service 
    // using the generated client stub functions
    let result = client
        .bar()
        .add((3, 4))
        .await;
    println!("{:?}", result);
}
```

### `#[export_trait]` and `#[export_trait_impl]`

Again, let's just remind ourselves that the service in this example is defined in a separate crate [`example-service`](https://minghuaw.github.io/toy-rpc/03_define_service.html#export_trait-and-export_trait_impl) and the service is implemented and served by [`example-server`](https://minghuaw.github.io/toy-rpc/04_server.html#example-with-export_trait-and-export_trait_impl) crate. What we will be doing below is to demonstrate the client, which, not surprisingly, look pretty much the same as the example above.

Because the service definition resides in a separate crate, we will need to import that crate as well.

```toml
[dependencies]
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
toy-rpc = { version = "0.7.5", features = ["tokio_runtime", "client"] }

# import our service definition, assuming we have this definition at "../example-service"
example-service = { version = "0.1.0", path = "../example-service" }
```

```rust,noplaypen 
use toy_rpc::{Client, Error};

// import everything to use the generated client stub functions
use example_service::*;

#[tokio::main]
async fn main() {
    let client = Client::dial("127.0.0.1:23333").await
        .expect("Failed to connect to the server");
    
    // Access the remote method `add` of service `Arith` in a blocking manner
    let result: Result<i32, Error> = client.call_blocking("Arith.add", (3i32, 4i32));
    println!("{:?}", result);

    // Access the remote method `subtract` of service `Arith` in an asynchronous manner
    let result: Result<i32, Error> = client.call("Arith.subtract", (9i32, 6i32)).await;
    println!("{:?}", result);

    // Let's use the generated client stub functions
    let result = client
        .arith() // access `Arith` service
        .add((3i32, 4i32)) // access `add` method
        .await;
    println!("{:?}", result);

    let result = client
        .arith() // access `Arith` service
        .subtract((9i32, 6i32)) // access `subtract` method
        .await;
    println!("{:?}", result);
}
```