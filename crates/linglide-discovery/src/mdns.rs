//! mDNS/DNS-SD service advertisement for LinGlide
//!
//! Advertises the LinGlide service on the local network using mDNS (Bonjour/Avahi).
//! This allows mobile devices to automatically discover LinGlide servers.

use crate::error::{DiscoveryError, DiscoveryResult};
use mdns_sd::{ServiceDaemon, ServiceInfo};
use std::collections::HashMap;
use std::net::IpAddr;
use tracing::{debug, info, warn};

/// Service type for LinGlide mDNS advertisement
pub const SERVICE_TYPE: &str = "_linglide._tcp.local.";

/// Default service name prefix
pub const SERVICE_NAME_PREFIX: &str = "LinGlide";

/// mDNS service advertiser for LinGlide
pub struct ServiceAdvertiser {
    daemon: ServiceDaemon,
    service_fullname: Option<String>,
    port: u16,
    instance_name: String,
}

impl ServiceAdvertiser {
    /// Create a new service advertiser
    ///
    /// # Arguments
    /// * `port` - The port the LinGlide server is running on
    /// * `instance_name` - Optional custom instance name (defaults to hostname-based name)
    pub fn new(port: u16, instance_name: Option<String>) -> DiscoveryResult<Self> {
        let daemon = ServiceDaemon::new()
            .map_err(|e| DiscoveryError::Mdns(e.to_string()))?;

        let instance_name = instance_name.unwrap_or_else(|| {
            let hostname = hostname::get()
                .ok()
                .and_then(|h| h.into_string().ok())
                .unwrap_or_else(|| "unknown".to_string());
            format!("{}-{}", SERVICE_NAME_PREFIX, hostname)
        });

        Ok(Self {
            daemon,
            service_fullname: None,
            port,
            instance_name,
        })
    }

    /// Start advertising the service on the network
    ///
    /// # Arguments
    /// * `version` - Server version string
    /// * `fingerprint` - TLS certificate fingerprint (first 20 chars)
    /// * `addresses` - Optional list of IP addresses to advertise
    pub fn start(
        &mut self,
        version: &str,
        fingerprint: Option<&str>,
        addresses: Option<Vec<IpAddr>>,
    ) -> DiscoveryResult<()> {
        // Build TXT record properties
        let mut properties = HashMap::new();
        properties.insert("version".to_string(), version.to_string());
        properties.insert("port".to_string(), self.port.to_string());

        if let Some(fp) = fingerprint {
            // Store first 20 chars of fingerprint for identification
            let fp_short = if fp.len() > 20 { &fp[..20] } else { fp };
            properties.insert("fingerprint".to_string(), fp_short.to_string());
        }

        // Build the service info
        let service_info = if let Some(addrs) = addresses {
            ServiceInfo::new(
                SERVICE_TYPE,
                &self.instance_name,
                &format!("{}.local.", self.instance_name),
                addrs.iter().map(|a| a.to_string()).collect::<Vec<_>>().join(",").as_str(),
                self.port,
                properties,
            )
        } else {
            ServiceInfo::new(
                SERVICE_TYPE,
                &self.instance_name,
                &format!("{}.local.", self.instance_name),
                "",
                self.port,
                properties,
            )
        }
        .map_err(|e| DiscoveryError::Mdns(e.to_string()))?;

        let fullname = service_info.get_fullname().to_string();

        self.daemon
            .register(service_info)
            .map_err(|e| DiscoveryError::Mdns(e.to_string()))?;

        self.service_fullname = Some(fullname.clone());

        info!(
            "mDNS: Advertising service '{}' on port {}",
            self.instance_name, self.port
        );
        debug!("mDNS: Full service name: {}", fullname);

        Ok(())
    }

    /// Stop advertising the service
    pub fn stop(&mut self) -> DiscoveryResult<()> {
        if let Some(fullname) = self.service_fullname.take() {
            match self.daemon.unregister(&fullname) {
                Ok(_) => {
                    info!("mDNS: Stopped advertising service");
                }
                Err(e) => {
                    warn!("mDNS: Failed to unregister service: {}", e);
                }
            }
        }
        Ok(())
    }

    /// Get the advertised instance name
    pub fn instance_name(&self) -> &str {
        &self.instance_name
    }

    /// Get the service type
    pub fn service_type() -> &'static str {
        SERVICE_TYPE
    }
}

impl Drop for ServiceAdvertiser {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_advertiser_creation() {
        let advertiser = ServiceAdvertiser::new(8443, None);
        assert!(advertiser.is_ok());

        let advertiser = advertiser.unwrap();
        assert!(advertiser.instance_name().starts_with("LinGlide-"));
    }

    #[test]
    fn test_custom_instance_name() {
        let advertiser = ServiceAdvertiser::new(8443, Some("MyLinGlide".to_string()));
        assert!(advertiser.is_ok());

        let advertiser = advertiser.unwrap();
        assert_eq!(advertiser.instance_name(), "MyLinGlide");
    }
}
