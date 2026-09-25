use arc_swap::ArcSwap;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::server::{ClientHello, ResolvesServerCert};
use rustls::sign::CertifiedKey;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, warn};

pub struct DynamicSniResolver {
    certs: ArcSwap<HashMap<String, Arc<CertifiedKey>>>,
    default_cert: ArcSwap<Option<Arc<CertifiedKey>>>,
}

impl DynamicSniResolver {
    pub fn new() -> Self {
        Self {
            certs: ArcSwap::new(Arc::new(HashMap::new())),
            default_cert: ArcSwap::new(Arc::new(None)),
        }
    }

    pub fn set_default_certificate(
        &self,
        cert_chain: Vec<CertificateDer<'static>>,
        key: PrivateKeyDer<'static>,
    ) -> Result<(), String> {
        let certified_key = Self::build_certified_key(cert_chain, key)?;
        self.default_cert
            .store(Arc::new(Some(Arc::new(certified_key))));
        Ok(())
    }

    pub fn add_or_update_sni(
        &self,
        domain: impl Into<String>,
        cert_chain: Vec<CertificateDer<'static>>,
        key: PrivateKeyDer<'static>,
    ) -> Result<(), String> {
        let certified_key = Arc::new(Self::build_certified_key(cert_chain, key)?);
        let mut map = (**self.certs.load()).clone();
        map.insert(domain.into().to_ascii_lowercase(), certified_key);
        self.certs.store(Arc::new(map));
        debug!("Dynamic SNI certificate updated");
        Ok(())
    }

    pub fn reload_all(
        &self,
        new_map: HashMap<String, (Vec<CertificateDer<'static>>, PrivateKeyDer<'static>)>,
        default: Option<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>)>,
    ) -> Result<(), String> {
        let mut parsed_map = HashMap::with_capacity(new_map.len());
        for (domain, (chain, key)) in new_map {
            let certified_key = Arc::new(Self::build_certified_key(chain, key)?);
            parsed_map.insert(domain.to_ascii_lowercase(), certified_key);
        }

        let new_default = if let Some((chain, key)) = default {
            Some(Arc::new(Self::build_certified_key(chain, key)?))
        } else {
            None
        };

        self.certs.store(Arc::new(parsed_map));
        self.default_cert.store(Arc::new(new_default));
        debug!("Reloaded all TLS certificates dynamically without restarting load balancer");
        Ok(())
    }

    fn build_certified_key(
        cert_chain: Vec<CertificateDer<'static>>,
        key: PrivateKeyDer<'static>,
    ) -> Result<CertifiedKey, String> {
        let signing_key = rustls::crypto::ring::sign::any_supported_type(&key)
            .map_err(|e| format!("Unsupported private key type: {:?}", e))?;

        Ok(CertifiedKey::new(cert_chain, signing_key))
    }
}

impl Default for DynamicSniResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for DynamicSniResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DynamicSniResolver").finish()
    }
}

impl ResolvesServerCert for DynamicSniResolver {
    fn resolve(&self, client_hello: ClientHello) -> Option<Arc<CertifiedKey>> {
        let sni = client_hello.server_name();
        let certs = self.certs.load();

        if let Some(domain) = sni {
            let lower_domain = domain.to_ascii_lowercase();
            if let Some(key) = certs.get(&lower_domain) {
                return Some(key.clone());
            }

            // Check wildcard *.domain.com
            if let Some(idx) = lower_domain.find('.') {
                let wildcard = format!("*{}", &lower_domain[idx..]);
                if let Some(key) = certs.get(&wildcard) {
                    return Some(key.clone());
                }
            }
        }

        // Fallback to default certificate
        let default_cert = self.default_cert.load();
        if let Some(ref key) = **default_cert {
            return Some(key.clone());
        }

        warn!(sni = ?sni, "No matching TLS certificate found for client hello");
        None
    }
}
