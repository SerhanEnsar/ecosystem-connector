//! Finding devices on the local network with mDNS / DNS-SD.
//!
//! The announcement contains the device id and name. It is not authenticated, and does
//! not need to be: connecting still requires the TLS handshake with the pinned
//! certificate, so a fake announcement can at most point us to an address where the
//! handshake fails.

use std::collections::BTreeMap;
use std::net::{IpAddr, SocketAddr};
use std::time::{Duration, Instant};

use ec_core::{DeviceId, DeviceIdentity, sanitize_device_name};
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};

use crate::{Error, Result};

/// The DNS-SD service type devices announce.
pub const SERVICE_TYPE: &str = "_ecconnect._udp.local.";
const PROTOCOL_VERSION: &str = "1";
/// Most addresses kept per discovered device.
const MAX_ADDRS: usize = 8;

/// Announces this device on the local network while it is alive.
pub struct Advertiser {
    daemon: ServiceDaemon,
    fullname: String,
}

impl Advertiser {
    /// Starts announcing `identity` with the node's `port`.
    pub fn start(identity: &DeviceIdentity, port: u16) -> Result<Self> {
        let daemon = ServiceDaemon::new()?;
        let id = identity.id().to_string();
        let instance = format!("ec-{}", &id[..16]);
        let host = format!("{instance}.local.");
        let properties = [
            ("id", id.as_str()),
            ("name", identity.name()),
            ("v", PROTOCOL_VERSION),
        ];
        let info = ServiceInfo::new(SERVICE_TYPE, &instance, &host, "", port, &properties[..])?
            .enable_addr_auto();
        let fullname = info.get_fullname().to_owned();
        daemon.register(info)?;
        Ok(Self { daemon, fullname })
    }
}

impl Drop for Advertiser {
    fn drop(&mut self) {
        let _ = self.daemon.unregister(&self.fullname);
        let _ = self.daemon.shutdown();
    }
}

/// A device seen on the network.
#[derive(Clone, Debug)]
pub struct DiscoveredDevice {
    /// The id it announced.
    pub id: DeviceId,
    /// The name it announced (sanitized).
    pub name: String,
    /// Where its node listens.
    pub addrs: Vec<SocketAddr>,
}

/// Looks for devices for `duration` and returns everything found.
pub async fn browse(duration: Duration) -> Result<Vec<DiscoveredDevice>> {
    tokio::task::spawn_blocking(move || browse_blocking(duration))
        .await
        .map_err(|_| Error::Protocol("discovery task failed"))?
}

fn browse_blocking(duration: Duration) -> Result<Vec<DiscoveredDevice>> {
    let daemon = ServiceDaemon::new()?;
    let events = daemon.browse(SERVICE_TYPE)?;
    let deadline = Instant::now() + duration;
    let mut found: BTreeMap<DeviceId, DiscoveredDevice> = BTreeMap::new();

    while let Some(left) = deadline.checked_duration_since(Instant::now()) {
        let Ok(event) = events.recv_timeout(left) else {
            break;
        };
        let ServiceEvent::ServiceResolved(service) = event else {
            continue;
        };
        if service.get_property_val_str("v") != Some(PROTOCOL_VERSION) {
            continue;
        }
        let Some(id) = service
            .get_property_val_str("id")
            .and_then(|s| s.parse::<DeviceId>().ok())
        else {
            continue;
        };
        let name = sanitize_device_name(service.get_property_val_str("name").unwrap_or(""));
        let entry = found.entry(id).or_insert_with(|| DiscoveredDevice {
            id,
            name,
            addrs: Vec::new(),
        });
        for ip in service.get_addresses() {
            let addr = SocketAddr::new(ip.to_ip_addr(), service.get_port());
            if addr.is_ipv4() && !entry.addrs.contains(&addr) && entry.addrs.len() < MAX_ADDRS {
                entry.addrs.push(addr);
            }
        }
    }
    let _ = daemon.shutdown();
    Ok(found
        .into_values()
        .filter(|d| !d.addrs.is_empty())
        .collect())
}

/// This machine's IPv4 addresses that other devices may reach, loopback last.
pub fn local_addrs() -> Vec<IpAddr> {
    let mut addrs: Vec<IpAddr> = if_addrs::get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .map(|iface| iface.ip())
        .filter(|ip| ip.is_ipv4() && !ip.is_loopback())
        .collect();
    addrs.sort();
    addrs.dedup();
    addrs.push(IpAddr::from([127, 0, 0, 1]));
    addrs
}
