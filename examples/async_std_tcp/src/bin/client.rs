use toy_rpc::{error::Error};
use toy_rpc::Client;

// use async_std_tcp::rpc::{BarRequest, BarResponse, FooRequest, FooResponse};
use async_std_tcp::rpc::*;

#[async_std::main]
async fn main() {
    env_logger::init();

    let addr = "127.0.0.1:23333";
    let client = Client::dial(addr).await.unwrap();

    // first request, echo
    let args = FooRequest { a: 1, b: 3 };
    let reply: FooResponse = client.call("Foo.echo", &args).unwrap();
    println!("{:?}", reply);

    // second request, increment_a
    let args = FooRequest {
        a: reply.a,
        b: reply.b,
    };
    let reply: Result<FooResponse, Error> = client.foo().increment_a(&args).await;
    println!("{:?}", reply);

    // second request, increment_b
    // let args = FooRequest {
    //     a: reply.a,
    //     b: reply.b,
    // };
    let handle = client.spawn_task("Foo.increment_b", args);
    let reply: Result<FooResponse, Error> = handle.await;
    println!("{:?}", reply);

    // third request, bar echo
    let args = BarRequest {
        content: "bar".to_string(),
    };
    let reply: BarResponse = client.call("Bar.echo", &args).unwrap();
    println!("{:?}", reply);

    // fourth request, bar exclaim
    // let reply: BarResponse = client.async_call("bar_service.exclaim", &args).await.unwrap();
    let reply = client.bar().exclaim(&args).await.unwrap();
    println!("{:?}", reply);

    // third request, get_counter
    let args = ();
    let handle = client.spawn_task("Foo.get_counter", args);
    let reply: u32 = handle.await.unwrap();
    println!("{:?}", reply);

    let args = ();

    // test service not found
    let res: Result<(), Error> = client.call("Service.method", &args);
    println!("{:?}", res);

    // method not found if no method is supplied
    let res: Result<(), Error> = client.call("Foo", &args);
    println!("{:?}", res);

    // method not found
    let res: Result<(), Error> = client.call("Foo.method", &args);
    println!("{:?}", res);

    client.close().await;
}
