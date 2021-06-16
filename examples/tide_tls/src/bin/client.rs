use std::{fs::File, io::BufReader, path::Path};
use env_logger;
use toy_rpc::client::Client;
use toy_rpc::error::Error;
use rustls::ClientConfig;

use tide_tls::rpc::{BarRequest, BarResponse, FooRequest, FooResponse};

const SELF_SIGNED_CA_CERT_PATH: &str = "certs/ca.cert";

#[async_std::main]
async fn main() {
    env_logger::init();

    let mut config = ClientConfig::new();
    let mut pem = BufReader::new(
        File::open(
            Path::new(SELF_SIGNED_CA_CERT_PATH)
        ).expect("Failed to open certificate file")
    );
    config
        .root_store
        .add_pem_file(&mut pem)
        .expect("Failed to load self-signed root certificate");

    let addr = "ws://127.0.0.1:23333/rpc/";
    let client = Client::dial_http_with_tls_config(addr, "localhost", config).await.unwrap();

    let args = FooRequest { a: 1, b: 3 };
    let reply: Result<FooResponse, Error> = client.call_blocking("FooService.echo", args.clone());
    println!("{:?}", reply);

    let reply: Result<FooResponse, Error> = client.call("FooService.increment_a", args.clone()).await;
    println!("{:?}", reply);

    let call = client.call("FooService.increment_b", args);
    let reply: Result<FooResponse, Error> = call.await;
    println!("{:?}", reply);

    // third request, bar echo
    let args = BarRequest {
        content: "bar".to_string(),
    };
    let reply: BarResponse = client.call_blocking("BarService.echo", args.clone()).unwrap();
    println!("{:?}", reply);

    // fourth request, bar exclaim
    let reply: BarResponse = client.call("BarService.exclaim", args).await.unwrap();
    println!("{:?}", reply);

    // third request, get_counter
    let args = ();
    let call = client.call("FooService.get_counter", args);
    let reply: u32 = call.await.unwrap();
    println!("{:?}", reply);

    // client.close().await;
}
