use toy_rpc::client::Client;

use server_client::rpc::{
    FooRequest,
    FooResponse,
    BarRequest,
    BarResponse
};

#[async_std::main]
async fn main() {
    env_logger::init();

    let addr = "127.0.0.1:23333";
    let client = Client::dial(addr).unwrap();

    // first request, echo
    let args = FooRequest {a: 1, b: 3};
    let reply: FooResponse = client.call("foo_service.echo", &args).unwrap();
    println!("{:?}", reply);
    
    // second request, increment_a
    let args = FooRequest {a: reply.a, b: reply.b};
    let reply: FooResponse = client.call("foo_service.increment_a", &args).unwrap();
    println!("{:?}", reply);

    // second request, increment_b
    let args = FooRequest {a: reply.a, b: reply.b};
    let reply: FooResponse = client.call("foo_service.increment_b", &args).unwrap();
    println!("{:?}", reply);

    // third request, bar echo
    let args = BarRequest {content: "bar".to_string()};
    let reply: BarResponse = client.call("bar_service.echo", &args).unwrap();
    println!("{:?}", reply);

    // fourth request, bar exclaim
    let reply: BarResponse = client.call("bar_service.exclaim", &args).unwrap();
    println!("{:?}", reply);

    // third request, get_counter
    let args = ();
    let reply: u32 = client.call("foo_service.get_counter", &args).unwrap();
    println!("{:?}", reply);
}
    
