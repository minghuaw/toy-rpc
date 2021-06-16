# Define Service

There are two ways (but three attribute macros) to help you define the the RPC service.

- The attribute macro `#[export_impl]` can be used when the implementation and the service definition are 
located in the same file/project. The name of the `struct` will be used as the default service name with `#[export_impl]` in a case sensitive manner.
- The attribute macros `#[export_trait]` and `#[export_trait_impl]` should be used when an abstract service definition (a `trait`) will be shared among different projects. The name of the `trait` will be used as the default service name with `#[export_trait]` in a case sensitive manner.

Both `#[export_impl]` and `#[export_trait]` will generate the client stub traits/methods when the "client" feature flag is enabled and a runtime feature flag is enabled on `toy-rpc`.

Inside the `impl` block or `trait` definition block, you should then use the attribute 
`#[export_method]` to mark which method(s) should be "exported" as RPC method(s). 
The methods to export must meet the following criteria on the server side

- the method resides in an impl block marked with `#[export_impl]` or `#[export_trait]`
- the method is marked with `#[export_method]` attribute
- the method takes one argument other than `&self` and returns a `Result<T, E>`
    - the argument must implement trait `serde::Deserialize`
    - the `Ok` type `T` of the result must implement trait `serde::Serialize`
    - the `Err` type `E` of the result must implement trait `ToString`

The method is essentially in the form

```rust,noplaypen 
#[export_method]
async fn method_name(&self, args: Req) -> Result<Res, ErrorMsg>
where
    Req: serde::Deserialize,
    Res: serde::Serialize,
    ErrorMsg: ToString,
{
    // ...
}
```

## Example Usage

Use the following dependencies to work with the examples below

```toml
[dependencies]
async-trait = "0.1.50"
toy-rpc = "0.7.2"
```

### `#[export_impl]`

When you have both the service definition and the implementation in the same file, you can use `#[export_impl]` on the `impl` block. This will also use the name of the `struct` as the default service name.

File structure 

```
./src
├── /bin
│   ├── server.rs
│   ├── client.rs
└── lib.rs
```

Suppose that *both* the service definitions and implementations are placed in `src/lib.rs`.

```rust,noplaypen 
// src/lib.rs
use toy_rpc::macros::export_impl;

pub struct Foo { }

#[export_impl] // The default service name will be "Foo"
impl Foo {
    // use attribute `#[export_method]` to mark which method to "export"
    #[export_method]
    async fn exported_method(&self, args: ()) -> Result<String, String> {
        Ok("exported method".into())
    }

    async fn not_exported_method(&self, args: ()) -> Result<String, String> {
        Ok("not exported method".into())
    }
}
```

You may also define a separate trait in `src/lib.rs` which is implemented by some `struct` in the same file.

```rust,noplaypen 
// continuing in src/lib.rs

use async_trait::async_trait;

#[async_trait]
pub trait Arith {
    async fn add(&self, args: (i32, i32)) -> Result<i32, String>;
    async fn subtract(&self, args: (i32, i32)) -> Result<i32, String>;
}

pub struct Bar { }
// implement the Arith trait for `Bar { }`and then mark the implementation as "exported" for RPC

#[async_trait]
// Place `#[export_impl] ` after `#[async_trait]`
#[export_impl] // The default service name will be "Bar"
impl Arith for Bar {
    // Only mark `add(...)` as RPC method
    #[export_method]
    async fn add(&self, args: (i32, i32)) -> Result<i32, String> {
        Ok(args.0 + args.1)
    }

    // `subtract(...)` will not be accessible from RPC calls
    async fn subtract(&self, args: (i32, i32)) -> Result<i32, String> {
        Ok(args.0 - args.1)
    }
}
```

We will continue to use this example in the [Server](https://minghuaw.github.io/toy-rpc/04_server.html) and [Client](https://minghuaw.github.io/toy-rpc/06_client.html) chapters.

### `#[export_trait]` and `#[export_trait_impl]`

When you want the abstract service definition to be shared but without concreate implementations, you should use `#[export_trait]` on the trait definition and `#[export_trait_impl]` on the concrete trait implementation. Please note that the default service name hence will be the name of the `trait` **NOT** that of the `struct`.

Suppose we will have three separate crates

- `"example-service"` as the service definition,
- `"example-server"` acting as the server,
- and `"example-client"` acting as the client

which can also be found in the GitHub examples ([service](https://github.com/minghuaw/toy-rpc/tree/main/examples/example-service), [server](https://github.com/minghuaw/toy-rpc/tree/main/examples/example-server), [client](https://github.com/minghuaw/toy-rpc/tree/main/examples/example-client)).

We are going to define the RPC service just as a trait in `"example-service"`.

```rust,noplaypen
// example-service/src/lib.rs
use async_trait::async_trait;
use toy_rpc::macros::export_trait;

#[async_trait]
#[export_trait] // The default service name will be "Arith"
pub trait Arith {
    // let's mark both `add(...)` and `subtract(...)` as RPC method
    #[export_method]
    async fn add(&self, args: (i32, i32)) -> Result<i32, String>;

    #[export_method]
    async fn subtract(&self, args: (i32, i32)) -> Result<i32, String>;

    // some method that we don't want to export
    fn say_hi(&self);
}
```

We will continue to use this example in the [Server](https://minghuaw.github.io/toy-rpc/04_server.html) and [Client](https://minghuaw.github.io/toy-rpc/06_client.html) chapters.




