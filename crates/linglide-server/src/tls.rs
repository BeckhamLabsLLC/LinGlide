//! TLS support for LinGlide server
//!
//! Provides self-signed certificate generation, persistent storage,
//! and fingerprint verification for secure pairing.

use axum_server::tls_rustls::RustlsConfig;
use chrono::{DateTime, Duration, Utc};
use rcgen::{CertificateParams, DistinguishedName, DnType, KeyPair, SanType};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tracing::{debug, info};

/// Default certificate validity period (1 year)
const CERT_VALIDITY_DAYS: i64 = 365;

/// Regenerate cert if less than this many days remain
const CERT_RENEWAL_THRESHOLD_DAYS: i64 = 30;

/// Certificate manager for persistent storage and validation
pub struct CertificateManager {
    /// Directory for storing certificates
    config_dir: PathBuf,
}

impl CertificateManager {
    /// Create a new certificate manager using the default config directory
    pub fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let config_dir = dirs::config_dir()
            .ok_or("Could not determine config directory")?
            .join("linglide");

        std::fs::create_dir_all(&config_dir)?;

        Ok(Self { config_dir })
    }

    /// Create with a custom config directory
    pub fn with_dir(config_dir: PathBuf) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        std::fs::create_dir_all(&config_dir)?;
        Ok(Self { config_dir })
    }

    /// Get the certificate file path
    pub fn cert_path(&self) -> PathBuf {
        self.config_dir.join("server.crt")
    }

    /// Get the private key file path
    pub fn key_path(&self) -> PathBuf {
        self.config_dir.join("server.key")
    }

    /// Get the certificate metadata file path
    fn metadata_path(&self) -> PathBuf {
        self.config_dir.join("cert_meta.json")
    }

    /// Load or generate a certificate
    ///
    /// If a valid certificate exists, it will be loaded.
    /// If no certificate exists or it's expiring soon, a new one is generated.
    pub fn load_or_generate(
        &self,
        hostnames: &[String],
    ) -> Result<(String, String, String), Box<dyn std::error::Error + Send + Sync>> {
        let cert_path = self.cert_path();
        let key_path = self.key_path();

        // Check if we have existing valid certificates
        if cert_path.exists() && key_path.exists() {
            if let Some(meta) = self.load_metadata() {
                if self.is_certificate_valid(&meta, hostnames) {
                    info!("Loading existing certificate (expires {})", meta.expires_at);
                    let cert_pem = std::fs::read_to_string(&cert_path)?;
                    let key_pem = std::fs::read_to_string(&key_path)?;
                    return Ok((cert_pem, key_pem, meta.fingerprint));
                } else {
                    info!("Certificate needs regeneration");
                }
            }
        }

        // Generate new certificate
        info!("Generating new self-signed certificate...");
        let (cert_pem, key_pem, fingerprint) = self.generate_and_save(hostnames)?;

        Ok((cert_pem, key_pem, fingerprint))
    }

    /// Generate a new certificate and save it
    fn generate_and_save(
        &self,
        hostnames: &[String],
    ) -> Result<(String, String, String), Box<dyn std::error::Error + Send + Sync>> {
        let (cert_pem, key_pem) = generate_self_signed_cert(hostnames)?;

        // Calculate fingerprint
        let fingerprint = calculate_cert_fingerprint(&cert_pem);

        // Save certificate files
        std::fs::write(self.cert_path(), &cert_pem)?;
        std::fs::write(self.key_path(), &key_pem)?;

        // Save metadata
        let meta = CertMetadata {
            created_at: Utc::now(),
            expires_at: Utc::now() + Duration::days(CERT_VALIDITY_DAYS),
            fingerprint: fingerprint.clone(),
            hostnames: hostnames.to_vec(),
        };
        self.save_metadata(&meta)?;

        info!("Certificate saved to {:?}", self.cert_path());
        info!("Certificate fingerprint: {}", fingerprint);

        Ok((cert_pem, key_pem, fingerprint))
    }

    /// Check if the certificate is valid and doesn't need renewal
    fn is_certificate_valid(&self, meta: &CertMetadata, hostnames: &[String]) -> bool {
        let now = Utc::now();
        let renewal_threshold = Duration::days(CERT_RENEWAL_THRESHOLD_DAYS);

        // Check expiration
        if meta.expires_at - now < renewal_threshold {
            debug!("Certificate expiring soon");
            return false;
        }

        // Check if hostnames match
        let mut current: Vec<String> = hostnames.to_vec();
        let mut stored: Vec<String> = meta.hostnames.clone();
        current.sort();
        stored.sort();

        if current != stored {
            debug!("Hostnames changed, regenerating certificate");
            return false;
        }

        true
    }

    /// Load certificate metadata
    fn load_metadata(&self) -> Option<CertMetadata> {
        let path = self.metadata_path();
        if !path.exists() {
            return None;
        }

        match std::fs::read_to_string(&path) {
            Ok(contents) => serde_json::from_str(&contents).ok(),
            Err(_) => None,
        }
    }

    /// Save certificate metadata
    fn save_metadata(
        &self,
        meta: &CertMetadata,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let json = serde_json::to_string_pretty(meta)?;
        std::fs::write(self.metadata_path(), json)?;
        Ok(())
    }

    /// Get the fingerprint of the current certificate
    pub fn get_fingerprint(&self) -> Option<String> {
        self.load_metadata().map(|m| m.fingerprint)
    }
}

/// Certificate metadata for persistence
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct CertMetadata {
    created_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    fingerprint: String,
    hostnames: Vec<String>,
}

/// Generate a self-signed certificate for the given hostnames/IPs
pub fn generate_self_signed_cert(
    hostnames: &[String],
) -> Result<(String, String), Box<dyn std::error::Error + Send + Sync>> {
    let mut params = CertificateParams::default();

    // Set distinguished name
    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, "LinGlide");
    dn.push(DnType::OrganizationName, "LinGlide");
    params.distinguished_name = dn;

    // Add Subject Alternative Names for all hostnames and IPs
    let mut san_list = Vec::new();
    san_list.push(SanType::DnsName("localhost".try_into()?));

    for hostname in hostnames {
        if let Ok(ip) = hostname.parse::<std::net::IpAddr>() {
            san_list.push(SanType::IpAddress(ip));
        } else if let Ok(dns) = hostname.as_str().try_into() {
            san_list.push(SanType::DnsName(dns));
        }
    }

    // Always add common local IPs
    san_list.push(SanType::IpAddress(std::net::IpAddr::V4(
        std::net::Ipv4Addr::new(127, 0, 0, 1),
    )));

    params.subject_alt_names = san_list;

    // Generate key pair and certificate
    let key_pair = KeyPair::generate()?;
    let cert = params.self_signed(&key_pair)?;

    let cert_pem = cert.pem();
    let key_pem = key_pair.serialize_pem();

    Ok((cert_pem, key_pem))
}

/// Calculate SHA-256 fingerprint of a certificate in human-readable format
pub fn calculate_cert_fingerprint(cert_pem: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(cert_pem.as_bytes());
    let result = hasher.finalize();

    // Format as colon-separated hex pairs (like browsers display)
    result
        .iter()
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<_>>()
        .join(":")
}

/// Create RustlsConfig from PEM strings
pub async fn create_rustls_config(
    cert_pem: &str,
    key_pem: &str,
) -> Result<RustlsConfig, Box<dyn std::error::Error + Send + Sync>> {
    let config =
        RustlsConfig::from_pem(cert_pem.as_bytes().to_vec(), key_pem.as_bytes().to_vec()).await?;
    Ok(config)
}

/// Create RustlsConfig from certificate files
pub async fn create_rustls_config_from_files(
    cert_path: &Path,
    key_path: &Path,
) -> Result<RustlsConfig, Box<dyn std::error::Error + Send + Sync>> {
    let config = RustlsConfig::from_pem_file(cert_path, key_path).await?;
    Ok(config)
}

/// Generate and save self-signed certificate to files
pub fn generate_and_save_cert(
    cert_path: &Path,
    key_path: &Path,
    hostnames: &[String],
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("Generating self-signed certificate...");

    let (cert_pem, key_pem) = generate_self_signed_cert(hostnames)?;

    std::fs::write(cert_path, &cert_pem)?;
    std::fs::write(key_path, &key_pem)?;

    info!("Certificate saved to {:?}", cert_path);
    info!("Private key saved to {:?}", key_path);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_cert_generation() {
        let hostnames = vec!["192.168.1.100".to_string()];
        let (cert, key) = generate_self_signed_cert(&hostnames).unwrap();
        assert!(!cert.is_empty());
        assert!(!key.is_empty());
        assert!(cert.contains("BEGIN CERTIFICATE"));
        assert!(key.contains("BEGIN PRIVATE KEY"));
    }

    #[test]
    fn test_fingerprint_calculation() {
        let cert = "-----BEGIN CERTIFICATE-----\ntest\n-----END CERTIFICATE-----";
        let fp = calculate_cert_fingerprint(cert);
        assert!(fp.contains(":"));
        assert_eq!(fp.len(), 95); // 32 bytes * 2 hex + 31 colons
    }

    #[test]
    fn test_certificate_manager() {
        let dir = tempdir().unwrap();
        let manager = CertificateManager::with_dir(dir.path().to_path_buf()).unwrap();

        let hostnames = vec!["localhost".to_string(), "192.168.1.1".to_string()];

        // First call generates
        let (cert1, key1, fp1) = manager.load_or_generate(&hostnames).unwrap();
        assert!(!cert1.is_empty());
        assert!(!key1.is_empty());
        assert!(!fp1.is_empty());

        // Second call loads existing
        let (cert2, key2, fp2) = manager.load_or_generate(&hostnames).unwrap();
        assert_eq!(cert1, cert2);
        assert_eq!(key1, key2);
        assert_eq!(fp1, fp2);

        // Changed hostnames triggers regeneration
        let new_hostnames = vec!["localhost".to_string(), "10.0.0.1".to_string()];
        let (cert3, _, fp3) = manager.load_or_generate(&new_hostnames).unwrap();
        assert_ne!(cert1, cert3);
        assert_ne!(fp1, fp3);
    }
}
