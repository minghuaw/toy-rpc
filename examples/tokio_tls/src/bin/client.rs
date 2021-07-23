use std::{fs::File, io::BufReader, path::Path};
use std::io;
use rustls::ClientConfig;
use anyhow::Result;
use toy_rpc::Client;

use tokio_tls::{ADDR, rpc::*};

const SELF_SIGNED_CA_CERT_PATH: &str = "certs/ca.cert";

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let mut config = ClientConfig::new();
    let mut pem = BufReader::new(File::open(Path::new(SELF_SIGNED_CA_CERT_PATH))?);
    config
        .root_store
        .add_pem_file(&mut pem)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid cert"))?;

    let client = Client::dial_with_tls_config(ADDR, "localhost", config).await?;

    let result = client.echo().echo_i32(3).await?;
    println!("{:?}", &result);
    Ok(())
}