
## 0.9.0-alpha.1

Relaxed method return type requirements. 

Prior to this release, only `Result` type is accepted as the return type for
exported methods. Now, for methods that do not really need to be a `Result` (eg. a simple addition), the return type 
no long needs to be wrapped inside a `Result`. A more detailed example is provided below and can be found in 
this [example](https://github.com/minghuaw/toy-rpc/tree/0.9-devel/examples/tokio_tcp).

### Service definition

Please note that the codes that import the crates are intentionally omitted in the example below. Please refer to the 
[example](https://github.com/minghuaw/toy-rpc/tree/0.9-devel/examples/tokio_tcp) on github for more information.

#### Service definition with service struct impl

```rust 
pub struct Echo { }

#[export_impl]
impl Echo {
    /// This shows an exported method that returns a non-result type
    #[export_method]
    pub async fn echo_i32(&self, req: i32) -> i32 {
        req
    }

    /// This shows an exported method that returns a Result type
    #[export_method]
    pub async fn echo_if_equal_to_one(&self, req: i32) -> Result<i32, i32> {
        match req {
            1 => Ok(req),
            _ => Err(req), // This will present on the client call as `Error::ExecutionError`
        }
    }
}

```

#### Service definition with a trait 

Please note that if `impl_for_client` is enabled, all the methods in the service trait ***must*** return 
a `Result` type.

```rust
/// Here we define a trait `Arith` that comes with default implementations
/// for all of its methods
#[async_trait]
#[export_trait] 
pub trait Arith {
    /// Addition
    #[export_method]
    async fn add(&self, args: (i32, i32)) -> i32 {
        args.0 + args.1
    }

    /// Subtraction
    #[export_method]
    async fn subtract(&self, args: (i32, i32)) -> i32 {
        args.0 - args.1
    }

    /// Multiplication
    #[export_method]
    async fn multiply(&self, args: (i32, i32)) -> i32 {
        args.0 * args.1
    }

    /// Division. We cannot divide by zero
    #[export_method]
    async fn divide(&self, args: (i32, i32)) -> Result<i32, String> {
        let (numerator, denominator) = args;
        match denominator {
            0 => return Err("Divide by zero!".to_string()),
            _ => Ok( numerator / denominator )
        }
    }
}

/// Here we are going to define another trait (service) tha is almost 
/// identical to `Arith` shown above, but we are going to generate the trait
/// impl for the client using `impl_for_client` argument in our `#[export_trait]`
/// attribute. Plus, we will not going to supply a default implementation either.
#[async_trait]
/// All methods must be exported if client trait impl generation is enabled.
/// If `impl_for_client` is enabled, all methods in the trait must return 
/// a Result
#[export_trait(impl_for_client)]
pub trait Arith2 {
    #[export_method]
    async fn add(&self, args: (i32, i32)) -> anyhow::Result<i32>;

    #[export_method]
    async fn subtract(&self, args: (i32, i32)) -> anyhow::Result<i32>;

    #[export_method]
    async fn multiply(&self, args: (i32, i32)) -> anyhow::Result<i32>;

    #[export_method]
    async fn divide(&self, args: (i32, i32)) -> anyhow::Result<i32>;
}
```

### Server implementation

Nothing really changed in terms of usage for the server side. The server side example code is attached below for completeness.

```rust
struct Abacus { }

/// We will simply use the default implementation provided in the trait 
/// definition for all except for add
#[async_trait]
#[export_trait_impl]
impl Arith for Abacus { 
    /// We are overriding the default implementation just for 
    /// the sake of demo
    async fn add(&self, args: (i32, i32)) -> i32 {
        args.0 + args.1
    }
}

/// For now, you need a separate type for a new service
struct Abacus2 { }

#[async_trait]
#[export_trait_impl]
impl Arith2 for Abacus2 {
    async fn add(&self, args: (i32, i32)) -> anyhow::Result<i32> {
        Ok(args.0 + args.1)
    }

    async fn subtract(&self, args: (i32, i32)) -> anyhow::Result<i32> {
        Ok(args.0 - args.1)
    }

    async fn multiply(&self, args: (i32, i32)) -> anyhow::Result<i32> {
        Ok(args.0 * args.1)
    }

    async fn divide(&self, args: (i32, i32)) -> anyhow::Result<i32> {
        let (numerator, denominator) = args;
        match denominator {
            0 => return Err(anyhow::anyhow!("Divide by zero!")),
            _ => Ok( numerator / denominator )
        }
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let addr = "127.0.0.1:23333";
    let echo_service = Arc::new(
        Echo { }
    );
    let arith = Arc::new(Abacus { });
    let arith2 = Arc::new(Abacus2 { });

    let server = Server::builder()
        .register(echo_service)
        .register(arith)
        .register(arith2)
        .build();

    let listener = TcpListener::bind(addr).await.unwrap();

    log::info!("Starting server at {}", &addr);

    let handle = task::spawn(async move {
        server.accept(listener).await.unwrap();
    });
    handle.await.expect("Error");
}
```

### Client side implementation

Like the server side implementation, there isn't much change to the client side usage. The only thing
worth mentioning is that even if the method doesn't return a `Result` in the service definition, the client
will still get a `Result` because there could be errors with connection or serialization/deserialization.

```rust

#[tokio::main]
async fn main() {
    let _ = run().await;
}

async fn run() -> anyhow::Result<()> {
    env_logger::init();

    // Establish connection
    let addr = "127.0.0.1:23333";
    let client = Client::dial(addr).await.unwrap();

    // Perform RPC using `call()` method
    let call: Call<i32> = client.call("Echo.echo_i32", 13i32);
    let reply = call.await?;
    println!("{:?}", reply);

    let reply: i32 = client.call("Echo.echo_i32", 1313i32).await?;
    println!("{:?}", reply);

    let ok_result = client
        .echo() // refering to `Echo` service
        .echo_if_equal_to_one(1) // refering to `echo_if_equal_to_one` method
        .await; 
    let err_result = client
        .echo()
        .echo_if_equal_to_one(2)
        .await;
    println!("Ok result: {:?}", ok_result);
    println!("Err result: {:?}", err_result);

    // Demo usage with the `Arith` trait
    let addition = client
        .arith() // generated for `Arith` service
        .add((1, 3)).await; // call `add` method
    println!("{:?}", addition);

    // Although the return type of `divide` is a `Result<T, E>`,
    // the execution result will be mapped to `Result<T, toy_rpc::Error>`
    // where `E` is mapped to `toy_rpc::Error::ExecutionError` so that 
    //   (1) the Error type doesn't need to implement `Serialize` and
    //   (2) the users don't need to unwrap twice
    let division = client
        .arith()
        .divide((3, 1)).await;
    println!("{:?}", division);

    // let's try to get an execution error
    let divide_by_zero = client
        .arith()
        .divide((3, 0)).await;
    println!("{:?}", divide_by_zero);

    // Now let's take a look at using the generated trait impl for the client.
    let addition = Arith2::add(&client, (7, 8)).await;
    println!("{:?}", addition);

    let division = Arith2::divide(&client, (7, 2)).await;
    println!("{:?}", division);
    let divide_by_zero = Arith2::divide(&client, (7, 0)).await;
    println!("{:?}", divide_by_zero);

    client.close().await;
    Ok(())
}
```