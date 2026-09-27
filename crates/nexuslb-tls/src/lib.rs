#![deny(unsafe_code)]

pub mod cert;
pub mod config;
pub mod mtls;
pub mod sni;

pub use cert::{generate_self_signed, load_pem_file, parse_certs_from_pem, parse_key_from_pem};
pub use config::TlsConfigBuilder;
pub use mtls::{build_client_verifier, ClientAuthMode, ClientIdentity};
pub use sni::DynamicSniResolver;
