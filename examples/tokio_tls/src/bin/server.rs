use tokio::net::TcpListener;
use std::fs::File;
use std::io::{self, BufReader};
use std::path::{Path};
use std::sync::Arc;
use rustls_pemfile::{certs, rsa_private_keys};
use rustls::{Certificate, PrivateKey, ServerConfig};
use toy_rpc::Server;
use anyhow::Result;

use tokio_tls::{ADDR, rpc::Echo};

// This is for demonstration purpose only
const SERVER_CERT_PATH: &str = "certs/service.pem";
const SERVER_KEY_PATH: &str = "certs/service.key";

fn load_certs(path: &str) -> Result<Vec<Certificate>> {
    certs(&mut BufReader::new(File::open(Path::new(path))?))
        .map(|v| v.into_iter().map(|vv| Certificate(vv)).collect())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid cert").into())
}

fn load_keys(path: &str) -> Result<Vec<PrivateKey>> {
    rsa_private_keys(&mut BufReader::new(File::open(Path::new(path))?))
        .map(|v| v.into_iter().map(|vv| PrivateKey(vv)).collect())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid key").into())
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let certs = load_certs(SERVER_CERT_PATH).unwrap();
    let mut keys = load_keys(SERVER_KEY_PATH).unwrap();
    let config = ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(certs, keys.remove(0))?;

    let echo = Arc::new(Echo { });
    let server = Server::builder()
        .register(echo)
        .build();
    let listener = TcpListener::bind(ADDR).await.unwrap();

    server.accept_with_tls_config(listener, config).await.unwrap();
    
    Ok(())
}