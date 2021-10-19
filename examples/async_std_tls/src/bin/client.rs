use std::{fs::File, io::BufReader, path::Path};
use std::io;
use rustls::{Certificate, ClientConfig, RootCertStore};
use rustls_pemfile::certs;
use anyhow::Result;
use toy_rpc::Client;

use async_std_tls::{ADDR, rpc::*};

const SELF_SIGNED_CA_CERT_PATH: &str = "certs/ca.cert";

fn load_certs(path: &str) -> Result<Vec<Certificate>> {
    certs(&mut BufReader::new(File::open(Path::new(path))?))
        .map(|v| v.into_iter().map(|vv| Certificate(vv)).collect())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid cert").into())
}

#[async_std::main]
async fn main() -> Result<()> {
    env_logger::init();

    let certs = load_certs(SELF_SIGNED_CA_CERT_PATH).unwrap();
    let mut root_certs = RootCertStore::empty();
    root_certs.add(certs.first().unwrap()).unwrap();
    let config = ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(root_certs)
        .with_no_client_auth();

    let client = Client::dial_with_tls_config(ADDR, "localhost", config).await?;

    let result = client.echo().echo_i32(3).await?;
    println!("{:?}", &result);
    Ok(())
}