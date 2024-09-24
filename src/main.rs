use std::fs::File;
use std::io::{BufReader, Seek, SeekFrom, Error as IoError};
use std::sync::Arc;
use std::env::var;
use hyper::service::service_fn;
use hyper::{Body, Request, Response};
use rustls::{Certificate, PrivateKey, ServerConfig};
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;
use rustls_pemfile::{certs, pkcs8_private_keys, rsa_private_keys};
use std::convert::Infallible;

// Integrate logging
use env_logger::Env;
use log::{error, info};

/// Load certificates from a PEM file.
fn load_certs(path: &str) -> Result<Vec<Certificate>, IoError> {
    let cert_file = File::open(path).map_err(|e| {
        eprintln!("Failed to open certificate file at {}: {}", path, e);
        e
    })?;
    let mut reader = BufReader::new(cert_file);
    let certs = certs(&mut reader)?
        .into_iter()
        .map(Certificate)
        .collect();
    Ok(certs)
}

/// Load a private key from a PEM file, supporting both PKCS#8 and PKCS#1 formats.
fn load_private_key(path: &str) -> Result<PrivateKey, IoError> {
    let key_file = File::open(path).map_err(|e| {
        eprintln!("Failed to open private key file at {}: {}", path, e);
        e
    })?;
    let mut reader = BufReader::new(key_file);

    // Attempt to load PKCS#8 private keys first
    if let Ok(mut keys) = pkcs8_private_keys(&mut reader) {
        if !keys.is_empty() {
            return Ok(PrivateKey(keys.remove(0)));
        }
    }

    // Reset the reader to the beginning of the file
    reader.seek(SeekFrom::Start(0))?;

    // Attempt to load PKCS#1 (RSA) private keys
    if let Ok(mut keys) = rsa_private_keys(&mut reader) {
        if !keys.is_empty() {
            return Ok(PrivateKey(keys.remove(0)));
        }
    }

    // If no keys were found in either format
    Err(std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        "No private keys found or unsupported key format",
    ))
}

/// Asynchronous request handler
async fn handle_request(_req: Request<Body>) -> Result<Response<Body>, Infallible> {
    Ok(Response::new(Body::from("Hello, HTTPS world!")))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize the logger
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    // Paths to your certificate and private key
    let cert_path = var("CERT_PATH")?; // Replace with your actual path
    let key_path  = var("KEY_PATH")?; // Replace with your actual path

    // Load certificates
    let certs = load_certs(&cert_path).map_err(|e| {
        error!("Error loading certificates: {}", e);
        Box::new(e) as Box<dyn std::error::Error + Send + Sync>
    })?;

    // Load private key
    let key = load_private_key(&key_path).map_err(|e| {
        error!("Error loading private key: {}", e);
        Box::new(e) as Box<dyn std::error::Error + Send + Sync>
    })?;

    // Configure TLS
    let tls_config = ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|e| {
            error!("Failed to configure TLS: {}", e);
            Box::new(e) as Box<dyn std::error::Error + Send + Sync>
        })?;
    let tls_acceptor = TlsAcceptor::from(Arc::new(tls_config));

    // Assign the address directly as a string
    let addr = "0.0.0.0:443"; // Use 443 for production with appropriate permissions
    let listener = TcpListener::bind(addr).await.map_err(|e| {
        error!("Failed to bind to address {}: {}", addr, e);
        Box::new(e) as Box<dyn std::error::Error + Send + Sync>
    })?;
    info!("Listening on https://{}", addr);

    loop {
        let (stream, peer_addr) = match listener.accept().await {
            Ok((s, addr)) => (s, addr),
            Err(e) => {
                error!("Failed to accept connection: {}", e);
                continue;
            }
        };
        let acceptor = tls_acceptor.clone();

        tokio::spawn(async move {
            let tls_stream = match acceptor.accept(stream).await {
                Ok(s) => s,
                Err(e) => {
                    error!("TLS handshake failed for {}: {}", peer_addr, e);
                    return;
                }
            };

            // Create a Service instance directly using service_fn
            let service = service_fn(handle_request);

            // Serve the connection using serve_connection
            if let Err(err) = hyper::server::conn::Http::new()
                .serve_connection(tls_stream, service)
                .await
            {
                error!("Error serving connection for {}: {:?}", peer_addr, err);
            }
        });
    }
}

