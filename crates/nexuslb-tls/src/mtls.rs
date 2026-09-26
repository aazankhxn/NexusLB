use rustls::pki_types::CertificateDer;
use rustls::server::WebPkiClientVerifier;
use rustls::RootCertStore;
use std::io::Cursor;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientAuthMode {
    Disabled,
    Optional,
    Required,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientIdentity {
    pub fingerprint_sha256: String,
    pub subject: Option<String>,
}

impl ClientIdentity {
    pub fn from_peer_certs(certs: &[CertificateDer]) -> Option<Self> {
        let first = certs.first()?;
        let raw = first.as_ref();

        // Calculate SHA-256 fingerprint
        use ring::digest::{digest, SHA256};
        let hash = digest(&SHA256, raw);
        let fingerprint = hash
            .as_ref()
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(":");

        Some(Self {
            fingerprint_sha256: fingerprint,
            subject: None,
        })
    }
}

/// Build a WebPkiClientVerifier from PEM-encoded CA root certificates
pub fn build_client_verifier(
    ca_pem_bytes: &[u8],
    mode: ClientAuthMode,
) -> Result<Arc<dyn rustls::server::danger::ClientCertVerifier>, String> {
    let mut roots = RootCertStore::empty();
    let mut reader = Cursor::new(ca_pem_bytes);

    for cert in rustls_pemfile::certs(&mut reader) {
        let cert = cert.map_err(|e| format!("Failed to parse CA certificate: {}", e))?;
        roots
            .add(cert)
            .map_err(|e| format!("Failed to add CA cert to root store: {}", e))?;
    }

    if roots.is_empty() {
        return Err("No valid CA certificates found in provided PEM data".to_string());
    }

    let roots_arc = Arc::new(roots);
    let builder = match mode {
        ClientAuthMode::Required => WebPkiClientVerifier::builder(roots_arc),
        ClientAuthMode::Optional => {
            WebPkiClientVerifier::builder(roots_arc).allow_unauthenticated()
        }
        ClientAuthMode::Disabled => {
            return Err("Cannot build verifier when ClientAuthMode is Disabled".to_string());
        }
    };

    builder
        .build()
        .map_err(|e| format!("Failed to construct client verifier: {}", e))
}
