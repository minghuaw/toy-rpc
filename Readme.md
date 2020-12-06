# toy-rpc

## A toy RPC crate based on `async-std` that mimics the `golang`'s `net/rpc` package

This crate aims at providing an easy-to-use RPC that is similar to `golang`'s
`net/rpc`.

The usage is similar to that of `golang`'s `net/rpc` with functions sharing similar
names and functionalities. Certain function names are changed to be more rusty.
Because `rust` doesn't have reflection, attribute macros are used to make certain
method "exported".

### Content

- [Crate Feature Flags](#crate-feature-flags)
  - [Default Features](#default-features)
- [Documentation](#documentation)
- [Examples](#examples)
  - [RPC over socket](#rpc-over-socket)
  - [RPC over HTTP with `tide`](#rpc-over-http-with-tide)
  - [RPC over HTTP with `actix-web`](#rpc-over-http-with-actix-web)
- [Change Log](#change-log)
- [Future Plan](#future-plan)


### Crate Feature Flags

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
- `actix-web`: enables `actix-web` integration on the server side
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
After the registration, the "exported" methods will be accessible remotely.
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
connection. A `Client` with HTTP connection or socket connection has three methods, `call`, `async_call`,
and `spawn_task`, to specify the service and method to call and the argument.

Please note that `call_http`, `async_call_http` and `spawn_task_http` are becoming deprecated
as the same API now can be called for both a socket client and an HTTP client.

- `call` method is synchronous and waits for the remote call
to complete and then returns the result.
- `async_call` is the `async` versions of `call` and `call_http`,
respectively. Because they are `async` functions, they must be called with `.await` to
be executed.
- `spawn_task` method spawns an `async` task and returns a `JoinHandle`.
The result can be obtained using the `JoinHandle`.

Unless an explicity codec is set up (with `serve_codec` method, HTTP is *NOT* supported yet),
the default codec specified by one of the following features tags (`bincode`, `serde_json`)
will be used to transport data.

### Examples

A few simple examples are shown below. More examples can be found in the `examples`
directory in the repo.

#### RPC over socket

The default feature flags will work with the example below.

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
    let client = Client::dial(addr).await.unwrap();

    let args = ExampleRequest{a: 1};
    let reply: Result<ExampleResponse, Error> = client.call("example.echo", &args);
    println!("{:?}", reply);
}
```

#### RPC over HTTP with `tide`

The default feature flags will work with the example below.

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
    "handle_http" is a conenience function that calls "into_endpoint"
    // with the "tide" feature turned on and "actix-web" feature disabled
    //app.at("/rpc/").nest(server.handle_http());

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
    let client = Client::dial_http(path).await.unwrap();

    let args = ExampleRequest{a: 1};
    let reply: Result<ExampleResponse, Error> = client.call("example.echo", &args);
    println!("{:?}", reply);
}
```

#### RPC over HTTP with `actix-web`

```toml
toy-rpc = { version = "0.4.2", default-features = false, features = ["std", "serde_bincode", "actix-web", "surf"] }
```

server.rs

```rust
use async_std::sync::{Arc, Mutex};
use serde::{Serialize, Deserialize};
use actix_web::{App, HttpServer, web};

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

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let addr = "127.0.0.1:8888";
    let example_service = Arc::new(
        ExampleService {
            counter: Mutex::new(0),
        }
    );

    let server = Server::builder()
        .register("example", service!(example_service, ExampleService))
        .build();

    HttpServer::new(
        move || {
            App::new()
                .service(
                    web::scope("/rpc/")
                        .app_data(app_data.clone())
                        .configure(Server::scope_config)
                        // The line above may be replaced with line below if "actix-web"
                        // is enabled and "tide" is disabled
                        //.configure(Server::handle_http()) // use the convenience "handle_http"
                )
        }
    )
    .bind(addr)?
    .run()
    .await
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
    let client = Client::dial_http(path).await.unwrap();

    let args = ExampleRequest{a: 1};
    let reply: Result<ExampleResponse, Error> = client.call("example.echo", &args);
    println!("{:?}", reply);
}

```

### Change Log

#### 0.4.2

- Removed previously unused NoneError
- Unified `call`, `async_call` and `spawn_task` for socket client
and HTTP client. The `call_http`, `async_call_http`, and `spawn_task_http`
methods are kept for compatibility.

#### 0.4.2

- Temporary fix of `spawn_task()` and `spawn_task_http()` with `Arc<Mutex<_>>` until
lifetime with async task is figured out. As a result, `Client` no longer needs to be declared `mut`.

#### 0.4.1

- Updated documentation

#### 0.4.0

- Added `actix-web` feature flag to support integration with `actix-web`

#### 0.3.1

- Added `serde_rmp` features flag
- Updated and corrected examples in the documentation

#### 0.3.0

- Added `serde_cbor` feature flag
- Changed `bincode` feature flag to `serde_bincode`


### Future Plan

- `warp` integration
- support other I/O connection
- unify `call`, `async_call`, and `spawn_task` for raw connection and HTTP connection


License: MIT/Apache-2.0
