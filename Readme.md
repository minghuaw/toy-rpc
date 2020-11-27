# toy-rpc

## A toy RPC crate based on `async-std` that mimics the `golang`'s `net/rpc` package

This crate aims at providing an easy-to-use RPC that is similar to `golang`'s
`net/rpc`.

The usage is similar to that of `golang`'s `net/rpc` with functions sharing similar
names and functionalities. Certain function names are changed to be more rusty.
Because `rust` doesn't have reflection, attribute macros are used to make certain
method 'exported'.

### Change Log

#### From 0.3 to 0.3.1

- Added `serde_rmp` features flag
- Updated and corrected examples in the documentation

#### From 0.2.1 to 0.3

- Added `serde_cbor` feature flag
- Changed `bincode` feature flag to `serde_bincode`

### Crate features tags

This crate offers the following features flag

- `std`: enables `serde/std`
- `serde_bincode`: the default codec will use `bincode`
for serialization/deserialization
- `serde_json`: the default codec will use `serde_json`
for `json` serialization/deserialization
- `serde_cbor`: the default codec will use `serde_cbor`
for serialization/deserialization
- `serde_rmp`: the default codec will use `rmp-serde`
for serialization/deserialization
- `logging`: enables logging
- `tide`: enables `tide` integration on the server side
- `surf`: enables HTTP client on the client side

#### Default Features

```toml
[features]
default = [
    "std",
    "serde_bincode",
    "tide",
    "surf",
]
```

### Documentation

The following documentation is adapted based on `golang`'s documentation.

This crate provides access to the methods marked with `#[export_impl]`
and `#[export_method]` of an object across a network connection. A server
registers an object, making it visible as a service with a name provided by the user.
After the registration, the 'exported' methods will be accessible remotely.
A server may register multiple objects as multiple services, and multiple
objects of the same type or different types could be registered on the same
`Server` object.

To export a method, use `#[export_method]` attribute in an impl block marked with
`#[export_impl]` attribute. This crate currently `only` support using `#[export_impl]` attribute
on `one` impl block per type.

```rust
struct ExampleService { }

#[export_impl]
impl ExampleService {
    #[export_method]
    async fn exported_method(&self, args: ()) -> Result<String, String> {
        Ok("This is an exported method".to_string())
    }

    async fn not_exported_method(&self, args: ()) -> Result<String, String> {
        Ok("This method is NOT exported".to_string())
    }
}
```

The methods to export must meet the following criteria on the server side

- the method resides in an impl block marked with `#[export_impl]`
- the method is marked with `#[export_method]` attribute
- the method takes one argument other than `&self` and returns a `Result<T, E>`

  - the argument must implement trait `serde::Deserialize`
  - the `Ok` type `T` of the result must implement trait `serde::Serialize`
  - the `Err` type `E` of the result must implement trait `ToString`

- the method is essentially in the form

```rust
struct ServiceState { }

#[export_impl]
impl ServiceState {
    #[export_method]
    async fn method_name(&self, args: Req) -> Result<Res, Msg>
    where
        Req: serde::Deserialize,
        Res: serde::Serialize,
        Msg: ToString,
    {
        unimplemented!()
    }
}
```

`Req` and `Res` are marshaled/unmarshaled (serialized/deserialized) by `serde`.
Realistically the `Req` and `Res` type must also be marshaled/unmarshaled on
the client side, and thus `Req` and `Res` must both implement *both*
`serde::Serialize` and `serde::Deserialize`.

The method's argument reprements the argument provided by the client caller,
and the `Ok` type of result represents success parameters to be returned to
the client caller. The `Err` type of result is passed back to the client as
a `String`.

The server may handle requests on a single connection by calling `serve_conn`,
and it may handle multiple connections by creating a `async_std::net::TcpListener`
and call `accept`. Integration with HTTP currently only supports `tide` by calling
`into_endpoint`.

A client wishing to use the service establishes a `async_std::net::TcpStream` connection
and then creates `Client` over the connection. The convenience function `dial` performs
this step for raw TCP socket connection, and `dial_http` performs this for an HTTP
connection. A client with raw TCP connection has three methods, `call`, `async_call`,
and `spawn_task`. A client with HTTP connection has three equivalent methods,
`call_http`, `async_call_http`, and `spawn_task_http`. All six functions have the
same signature that specifies the service and method to call and the argument.

- the `call` and `call_http` methods are synchronous and waits for the remote call
to complete and then returns the result.
- the `async_call` and `async_call_http` are `async` versions of `call` and `call_http`,
respectively. Because they are `async` functions, they must be called with `.await` to
be executed.
- the `spawn_task` and `spawn_task_http` spawn an `async` task and return a `JoinHandle`.
The result can be obtained using the `JoinHandle`.

Unless an explicity codec is set up (with `serve_codec`, HTTP is *NOT* supported yet),
the default codec specified by one of the following features tags (`bincode`, `serde_json`)
will be used to transport data.

### Examples

A few simple examples are shown below. More examples can be found in the `examples`
directory in the repo.

#### RPC over socket

server.rs

```rust
use async_std::net::TcpListener;
use async_std::sync::{Arc, Mutex};
use async_std::task;
use serde::{Serialize, Deserialize};

use toy_rpc::macros::{export_impl, service};
use toy_rpc::Server;

pub struct ExampleService {
    counter: Mutex<i32>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExampleRequest {
    pub a: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExampleResponse {
    a: u32,
}

#[async_trait::async_trait]
trait Rpc {
    async fn echo(&self, req: ExampleRequest) -> Result<ExampleResponse, String>;
}

#[async_trait::async_trait]
#[export_impl]
impl Rpc for ExampleService {
    #[export_method]
    async fn echo(&self, req: ExampleRequest) -> Result<ExampleResponse, String> {
        let mut counter = self.counter.lock().await;
        *counter += 1;

        let res = ExampleResponse{ a: req.a };
        Ok(res)
    }
}

#[async_std::main]
async fn main() {
    let addr = "127.0.0.1:8888";
    let example_service = Arc::new(
        ExampleService {
            counter: Mutex::new(0),
        }
    );

    let server = Server::builder()
        .register("example", service!(example_service, ExampleService))
        .build();

    let listener = TcpListener::bind(addr).await.unwrap();
    println!("Starting listener at {}", &addr);

    let handle = task::spawn(async move {
        server.accept(listener).await.unwrap();
    });
    handle.await;
}

```

client.rs

```rust
use serde::{Serialize, Deserialize};
use toy_rpc::Client;
use toy_rpc::error::Error;

#[derive(Debug, Serialize, Deserialize)]
struct ExampleRequest {
    a: u32
}

#[derive(Debug, Serialize, Deserialize)]
struct ExampleResponse {
    a: u32
}

#[async_std::main]
async fn main() {
    let addr = "127.0.0.1:8888";
    let mut client = Client::dial(addr).await.unwrap();

    let args = ExampleRequest{a: 1};
    let reply: Result<ExampleResponse, Error> = client.call("example.echo", &args);
    println!("{:?}", reply);
}
```

#### RPC over HTTP with `tide`

server.rs

```rust
use async_std::sync::{Arc, Mutex};
use serde::{Serialize, Deserialize};

use toy_rpc::macros::{export_impl, service};
use toy_rpc::Server;


pub struct ExampleService {
    counter: Mutex<i32>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExampleRequest {
    pub a: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExampleResponse {
    a: u32,
}

#[async_trait::async_trait]
trait Rpc {
    async fn echo(&self, req: ExampleRequest) -> Result<ExampleResponse, String>;
}

#[async_trait::async_trait]
#[export_impl]
impl Rpc for ExampleService {
    #[export_method]
    async fn echo(&self, req: ExampleRequest) -> Result<ExampleResponse, String> {
        let mut counter = self.counter.lock().await;
        *counter += 1;

        let res = ExampleResponse{ a: req.a };
        Ok(res)
    }
}

#[async_std::main]
async fn main() -> tide::Result<()> {
    let addr = "127.0.0.1:8888";
    let example_service = Arc::new(
        ExampleService {
            counter: Mutex::new(0),
        }
    );

    let server = Server::builder()
        .register("example", service!(example_service, ExampleService))
        .build();

    let mut app = tide::new();
    app.at("/rpc/").nest(server.into_endpoint());

    app.listen(addr).await?;
    Ok(())
}

```

client.rs

```rust
use serde::{Serialize, Deserialize};
use toy_rpc::Client;
use toy_rpc::error::Error;

#[derive(Debug, Serialize, Deserialize)]
struct ExampleRequest {
    a: u32
}

#[derive(Debug, Serialize, Deserialize)]
struct ExampleResponse {
    a: u32
}

#[async_std::main]
async fn main() {
    // note that the endpoint path must be specified
    let path = "http://127.0.0.1:8888/rpc/";
    let mut client = Client::dial_http(path).await.unwrap();

    let args = ExampleRequest{a: 1};
    let reply: Result<ExampleResponse, Error> = client.call_http("example.echo", &args);
    println!("{:?}", reply);
}
```

### Future Plan

- [ ] `actix` integration
- [ ] `warp` integration
- [ ] other I/O connection
- [ ] unify `call`, `async_call`, and `spawn_task` for raw connection and HTTP connection

License: MIT/Apache-2.0
