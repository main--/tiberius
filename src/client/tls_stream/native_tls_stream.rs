use crate::{
    client::{config::Config, TrustConfig},
    error::Error,
};
pub(crate) use async_native_tls::TlsStream;
use async_native_tls::{Certificate, TlsConnector};
use futures_util::io::{AsyncRead, AsyncWrite};
use std::fs;
use tracing::{event, Level};

use super::iter_certs::IterCertBundle;

pub(crate) async fn create_tls_stream<S: AsyncRead + AsyncWrite + Unpin + Send>(
    config: &Config,
    stream: S,
) -> crate::Result<TlsStream<S>> {
    let mut builder = TlsConnector::new();

    match &config.trust {
        TrustConfig::CaCertificateLocation(path) => {
            if let Ok(buf) = fs::read(path) {
                let cert = match path.extension() {
                        Some(ext)
                        if ext.to_ascii_lowercase() == "pem"
                            || ext.to_ascii_lowercase() == "crt" =>
                            {
                                Some(Certificate::from_pem(&buf)?)
                            }
                        Some(ext) if ext.to_ascii_lowercase() == "der" => {
                            Some(Certificate::from_der(&buf)?)
                        }
                        Some(_) | None => return Err(Error::Tls(
                            "Provided CA certificate with unsupported file-extension! Supported types are pem, crt and der.".to_string()
                        )),
                    };
                if let Some(c) = cert {
                    builder = builder.add_root_certificate(c);
                }
            } else {
                return Err(Error::Tls(
                    "Could not read provided CA certificate!".to_string()
                ));
            }
        }
        TrustConfig::CaCertificateBundle(bundle) => {
            for cert in IterCertBundle::new(bundle) {
                builder = builder.add_root_certificate(Certificate::from_pem(cert)?);
            }
        }
        TrustConfig::TrustAll => {
            event!(
                Level::WARN,
                "Trusting the server certificate without validation."
            );

            builder = builder.danger_accept_invalid_certs(true);
            builder = builder.danger_accept_invalid_hostnames(true);
            builder = builder.use_sni(false);
        }
        TrustConfig::Default => {
            event!(Level::INFO, "Using default trust configuration.");
        }
    }

    Ok(builder.connect(config.get_host(), stream).await?)
}
