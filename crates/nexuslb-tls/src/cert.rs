use rcgen::generate_simple_self_signed;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use std::io::{self, Cursor};
use std::path::Path;

pub fn generate_self_signed(subject_alt_names: Vec<String>) -> io::Result<(Vec<u8>, Vec<u8>)> {
    let cert = generate_simple_self_signed(subject_alt_names)
        .map_err(|e| io::Error::other(e.to_string()))?;

    let cert_pem = cert.cert.pem();
    let key_pem = cert.key_pair.serialize_pem();

    Ok((cert_pem.into_bytes(), key_pem.into_bytes()))
}

pub fn parse_certs_from_pem(pem_bytes: &[u8]) -> io::Result<Vec<CertificateDer<'static>>> {
    let mut reader = Cursor::new(pem_bytes);
    let certs: Vec<CertificateDer<'static>> = rustls_pemfile::certs(&mut reader)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    if certs.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "No valid certificates found in PEM",
        ));
    }
    Ok(certs)
}

pub fn parse_key_from_pem(pem_bytes: &[u8]) -> io::Result<PrivateKeyDer<'static>> {
    let mut reader = Cursor::new(pem_bytes);
    loop {
        match rustls_pemfile::read_one(&mut reader)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?
        {
            Some(rustls_pemfile::Item::Pkcs1Key(k)) => return Ok(k.into()),
            Some(rustls_pemfile::Item::Pkcs8Key(k)) => return Ok(k.into()),
            Some(rustls_pemfile::Item::Sec1Key(k)) => return Ok(k.into()),
            None => break,
            _ => continue,
        }
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "No valid private key found in PEM",
    ))
}

pub fn load_pem_file(path: impl AsRef<Path>) -> io::Result<Vec<u8>> {
    std::fs::read(path)
}
