use std::error::Error;
use std::sync::Arc;
use tokio_rustls::rustls::pki_types::PrivatePkcs8KeyDer;
use tokio_rustls::rustls::ServerConfig;
use tokio_rustls::TlsAcceptor;

pub(super) fn acceptor() -> Result<TlsAcceptor, Box<dyn Error>> {
    let hosts = match std::env::var("TLS_HOSTS") {
        Ok(hosts) => hosts,
        Err(std::env::VarError::NotPresent) => "localhost,127.0.0.1".to_string(),
        Err(error) => return Err(error.into()),
    };
    let names: Vec<String> = hosts
        .split(',')
        .map(|host| host.trim().to_string())
        .collect();
    if names.iter().any(String::is_empty) {
        return Err("TLS_HOSTS must contain comma-separated DNS names or IP addresses".into());
    }

    let generated = rcgen::generate_simple_self_signed(names)?;
    let key = PrivatePkcs8KeyDer::from(generated.signing_key.serialize_der());
    let mut config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![generated.cert.der().clone()], key.into())?;
    config.alpn_protocols = vec![b"http/1.1".to_vec()];

    Ok(TlsAcceptor::from(Arc::new(config)))
}
