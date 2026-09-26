use crate::sni::DynamicSniResolver;
use rustls::ServerConfig;
use std::sync::Arc;
use tokio_rustls::TlsAcceptor;

pub struct TlsConfigBuilder;

impl TlsConfigBuilder {
    pub fn build_acceptor(sni_resolver: Arc<DynamicSniResolver>) -> Result<TlsAcceptor, String> {
        let mut server_config = ServerConfig::builder()
            .with_no_client_auth()
            .with_cert_resolver(sni_resolver);

        // Enable ALPN for HTTP/2 and HTTP/1.1
        server_config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

        Ok(TlsAcceptor::from(Arc::new(server_config)))
    }

    pub fn build_acceptor_with_client_auth(
        sni_resolver: Arc<DynamicSniResolver>,
        client_verifier: Arc<dyn rustls::server::danger::ClientCertVerifier>,
    ) -> Result<TlsAcceptor, String> {
        let mut server_config = ServerConfig::builder()
            .with_client_cert_verifier(client_verifier)
            .with_cert_resolver(sni_resolver);

        // Enable ALPN for HTTP/2 and HTTP/1.1
        server_config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

        Ok(TlsAcceptor::from(Arc::new(server_config)))
    }
}
